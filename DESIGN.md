# Design

egui_markdown is designed for rendering large markdown documents (LLM chat threads,
documentation) in immediate-mode egui at 60fps.

## Custom link handling

**Problem:** Applications need links to do more than open URLs. A chat app might render
domain-specific links as interactive widgets. A documentation viewer might navigate
internally. The markdown renderer shouldn't hardcode these behaviors.

**Solution:** The `LinkHandler` trait lets consumers customize links at three levels,
checked in order during layout:

1. **`is_widget_link(href)` -> `render_link(ui, text, href)`** - Promote a link to a
   standalone widget. The link becomes a segment break: text before it is flushed as a
   galley, the handler renders any egui widget it wants (buttons, custom views, embedded
   previews), and text after starts a new galley. The handler gets full control via the
   `Ui` and returns a `Response`.

2. **`layout_link(text, href, job, font, color)`** - Customize inline styling. The
   handler appends sections to the LayoutJob directly (icons, colored segments,
   backgrounds). All appended sections are mapped to this link's token so hover and
   click still work. Returns true if handled.

3. **`link_style(href)`** - Simple color/underline override. Returns `None` for default
   hyperlink styling.

At interaction time:
- **Hover:** `link_style()` provides the underline color. Cursor changes to pointing hand.
- **Click:** `on_click(text, href, ui)` is called. Return true if handled, false to
  open the URL in the browser.
- **Cache:** `cache_key()` returns a u64 mixed into the layout hash. When the handler's
  behavior changes (e.g. different app context), change the key to invalidate cached layouts.

**Files:** `link.rs` (LinkHandler trait), `layout.rs` (layout-time dispatch), `label.rs` (hover/click dispatch)

---

## Performance optimizations

A naive approach (re-parse, re-layout, re-paint every frame) doesn't scale. Below are
the optimizations, what problem each solves, and how it works.

## Layout caching

**Problem:** Parsing markdown and building a LayoutJob (font metrics, text wrapping,
section styling) is expensive. Doing it every frame wastes CPU on unchanged content.
Scrolling code blocks add syntect highlighting on top of that.

**Solution:** Three cache layers stored in egui temp data:

1. **Full-document cache** (`CachedMarkdownLayout`): Caches the parse + layout result
   for the entire markdown string. Keyed by a hash of the text content, style, and
   link handler cache key. On cache hit, skips parsing and layout entirely. When the
   document needs the segmented path (tables, scrolling fences, blockquotes, …),
   `layout` is stored as `None` and only the tokens are kept.

2. **Per-segment cache** (`CachedFlushRange`): When the document has block elements,
   text between blocks is laid out independently. Each segment has its own cache keyed
   by a context hash (tokens, style, font, color, dark mode). Changed segments are
   re-laid out without affecting others. This covers prose around blocks, and also
   non-scrolling code fences that stay inside a document galley.

3. **Scrolling code-block cache** (`StreamingCodeCache`): Fences rendered with
   `scroll_code_blocks(true)` are standalone widgets, so they are not covered by
   `CachedFlushRange`. Each stores `Arc`s for source, frozen `LayoutJob`, and galley,
   plus syntect parse/highlight state frozen after every complete line (ending in `\n`).
   Identity is language, code font size, dark mode, pixels-per-point, and the theme
   actually used (per-call override pointer, else installed theme pointer via
   `set_code_themes`, else builtin selected by dark mode). The source is compared
   separately. Arcing the heavy fields keeps settled-hit `get_temp` clones cheap.

   - Exact source match: reuse the galley (no syntect).
   - Source still starts with the frozen complete-line prefix: commit any newly completed
     lines, tentatively highlight the incomplete tail, and reshape. Cost stays near the
     size of the tail, not the whole fence.
   - Otherwise: full rebuild.

   Callers that stream markdown into the same widget must keep a **stable widget id**
   across length changes. Putting `content.len()` in the id empties temp caches every
   token and defeats append-only highlighting.

Cache invalidation happens automatically: if the hash doesn't match, the cache is
rebuilt. Tokens are converted from borrowed to owned (`Token<'static>`) for cache
storage since the input string may not live across frames.

**Files:** `label.rs` (CachedMarkdownLayout, CachedFlushRange, hash_text,
hash_flush_context, hash_code_block_identity), `layout.rs` (StreamingCodeCache,
scrolling_code_galley)

## Viewport culling

**Problem:** A 10,000-line document in a scroll area would layout and paint all content
every frame, even though only ~50 lines are visible.

**Solution:** For each block element and text segment, cache its rendered size. Before
rendering, estimate the screen rect and check `ui.is_rect_visible()`. If off-screen,
call `ui.allocate_space()` to reserve the correct amount of space (so scrollbars work)
but skip all layout and painting.

This reduces per-frame work from O(document) to O(visible area).

**Files:** `label.rs` (flush_text_range size cache, render_token_range block size caches)

## Segmented rendering

**Problem:** Some markdown elements (tables, code blocks with scrolling, blockquotes,
images, widget links) can't be part of a single text galley; they need separate egui
widgets. A monolithic layout approach can't handle this.

**Solution:** During layout, identify "segment breaks" (token indices where the text
galley must be flushed and a block widget rendered). The render path then alternates
between flushing text ranges (as galleys) and rendering block widgets.

This also enables per-segment viewport culling and caching, since each segment is
independent.

**Files:** `layout.rs` (segment_breaks), `label.rs` (render_segmented, render_token_range, flush_text_range)

## Section-to-token mapping

**Problem:** When the user hovers or clicks on rendered text, we need to know which
markdown token is under the cursor. The galley only knows about layout sections (styled
text runs), not tokens.

**Solution:** During layout, build a parallel `Vec<usize>` mapping each section index
to its originating token index. On hover, find the section under the cursor (via glyph
position), then look up the token in O(1).

A companion function `section_for_char()` walks sections to find the section index for
a character offset, replacing a per-frame `Vec<u32>` allocation that would map every
character to its section.

**Files:** `layout.rs` (section_to_token, section_for_char)

## Streaming heal with zero-copy fast path

**Problem:** LLM output arrives incrementally. Unclosed code fences, bold markers, or
links cause pulldown-cmark to swallow subsequent text. We need to auto-close these
constructs before parsing.

**Solution:** `heal()` scans for unclosed constructs and appends closing markers. It
returns `Cow::Borrowed` when no healing is needed (the common case for complete
markdown), avoiding allocation entirely. Only incomplete input triggers `Cow::Owned`
with a new string.

**Files:** `parser.rs` (heal, heal_inline, heal_table)

## Token size constraint

**Problem:** Token vectors can be large (thousands of entries for big documents). If
each Token enum variant is bloated, memory usage and cache performance suffer.

**Solution:** A compile-time test asserts `Token` stays under 88 bytes. The current
size is driven by Link/Image variants with 3 CowStr fields (~80 bytes). The test
catches accidental growth from new fields or variants.

**Files:** `types.rs` (size_tests)

## Benchmarks

Criterion benchmarks cover parsing, text hashing, context hashing, and cache retrieval
(Arc clone). These validate that caching overhead is justified and catch performance
regressions.

**Files:** `benches/markdown.rs`

# Design

egui_markdown renders large markdown documents, such as LLM chat threads and
documentation, in immediate-mode egui at 60 frames per second.

## Custom link handling

**Problem:** Applications need links to do more than open URLs. A chat app can render
domain-specific links as interactive widgets. A documentation viewer can navigate
internally. The markdown renderer must not hardcode these behaviors.

**Solution:** The `LinkHandler` trait lets consumers customize links at four levels.
Layout applies them in this order, and the first one that applies wins:

1. **`is_block_widget(href)` -> `block_widget(ui, text, href)`** - Promote a link to a
   standalone widget. The link becomes a segment break. The renderer flushes the text
   before the link as a galley. The handler then draws any egui widget it wants, such as
   a card or an embedded preview. The text after the link starts a new galley. The
   handler receives the `Ui` and returns a `Response`.

2. **`inline_widget_size(href, font)` -> `paint_inline_widget(ui, text, href, font, rect)`** -
   Reserve space inside the text and paint a widget over it, such as a mention or a
   status badge. The size sets the row height. `layout_link` emits the placeholder text
   that sets the width, or the renderer emits the link text in a transparent color. After
   the galley is placed, the renderer calls `paint_inline_widget` once for each row the
   placeholder spans.

3. **`layout_link(ui, text, href, job, font, color)`** - Customize inline styling. The
   handler appends sections to the `LayoutJob` directly (icons, colored segments,
   backgrounds). The renderer maps every appended section to the token of this link, so
   that hover and click continue to work. Returns true if the handler styled the link.

4. **`link_style(href)`** - A color override, and whether the link shows an underline on
   hover. Returns `None` for default hyperlink styling.

A link takes the size, weight and slant of the text around it, so a link inside a heading
has the size of the heading.

At interaction time:

- **Hover:** the cursor changes to a pointing hand, and the link gets an underline in the
  `link_style()` color unless `link_style()` turns the underline off. The tooltip is
  `hover_text(href)`, then the link title, then the href when `url_in_tooltip` is set.
- **Click:** `click(text, href, ui)` is called. Return true if the handler used the
  click, false to open the URL in the browser. A link with an empty href, which heal
  produces until the URL arrives, opens nothing.
- **Cache:** `id()` returns a u64 that the layout hash includes. When the behavior
  of the handler changes, for example in a different app context, change the key to
  invalidate the cached layouts.

**Files:** `link.rs` (LinkHandler trait), `layout.rs` (layout-time dispatch), `label.rs` (hover/click dispatch)

---

## Performance optimizations

A naive approach, which parses, lays out, and paints again on every frame, does not
scale. The sections below give each optimization, the problem it solves, and how it
works.

## Layout caching

**Problem:** A markdown parse and a `LayoutJob` build (font metrics, text wrapping,
section styling) are both expensive. The same work on every frame wastes CPU time on
unchanged content. Scrolling code blocks also add syntect highlighting.

**Solution:** The widget keeps three cache layers in egui temp data:

1. **Full-document cache** (`CachedMarkdownLayout`): Holds the parse and layout result
   for the whole markdown string. The key is a hash of the text content, the style, and
   the cache key of the link handler. On a cache hit, the widget parses nothing and lays
   out nothing, and it borrows the cached tokens rather than copying them. The segmented path handles tables, scrolling code blocks, blockquotes,
   and similar elements. When the document needs that path, the entry holds `None` in
   its `layout` field and keeps only the tokens.

2. **Per-segment cache** (`CachedFlushRange`): When the document has block elements, the
   widget lays out the text between the blocks independently. Each segment has its own
   cache, keyed by a context hash of the tokens, style, font, color, and dark mode. The
   widget lays out a changed segment again and leaves the other segments alone. This
   covers the prose around blocks, and also the non-scrolling code blocks that stay
   inside a document galley.

3. **Scrolling code-block cache** (`StreamingCodeCache`): A code block that
   `scroll_code_blocks(true)` renders is a standalone widget, so `CachedFlushRange` does
   not cover it. Each entry holds an `Arc` for the source, one for the frozen
   `LayoutJob`, and one for the galley. It also holds the syntect parse and highlight
   state, frozen after each complete line, which is a line that ends in `\n`. The
   identity of an entry is the language, the code font size, the dark mode flag, the
   pixels-per-point value, and the theme in use. The theme in use is the per-call
   override pointer when there is one. If there is not, it is the installed theme pointer
   from `set_code_themes`, and then the built-in theme that the dark mode flag selects.
   The cache compares the source separately. Each large field sits behind an `Arc`, so a
   `get_temp` clone on a cache hit costs little.

   - Exact source match: reuse the galley and call no syntect code.
   - The source still starts with the frozen complete-line prefix: commit the lines that
     are now complete, highlight the incomplete last line, and shape the text again. The
     cost stays near the size of that last line, not the size of the whole code block.
   - No match: build the entry again.

   A caller that streams markdown into the same widget must keep a **stable widget id**
   as the length changes. An id that contains `content.len()` empties the temp caches on
   every token, which disables append-only highlighting.

The cache invalidates itself. If the hash does not match, the widget builds the entry
again. The widget converts the tokens from borrowed to owned (`Token<'static>`) before it
stores them, because the input string may not live across frames.

**The layout cache keys leave out the wrap settings, and this is intentional.** A
`LayoutJob` does not depend on the wrap width. `build_layout` uses `max_width` and
`break_anywhere` only to set the initial values on `job.wrap`. `render_galley` writes the
live values over both before it shapes the text. A key that includes the width
rebuilds every job on every frame of a resize. Without the width, a resize frame on the
benchmark document costs 426us rather than 1.008ms. The remaining cost is epaint, which
must shape the text again at each new width. A width added back to `hash_flush_context`
removes this saving and shows no other symptom.

`resolve_wrap` calculates the live values once per render. It reads the wrap mode from
`wrap_mode()` when the caller set one, and from `ui.wrap_mode()` when the caller did not.
`apply_live_wrap` then writes those values onto the cached job. `Extend` gives an infinite
width. `Wrap` uses `ui.available_width()`, and sets `break_anywhere` only for
`OverflowWrap::BreakAll`. `Truncate` always sets `break_anywhere`, so that the elision can
fall inside a token.

One value stays as it was. `build_layout` writes `max_rows` into the job at build time,
and `apply_live_wrap` does not replace it. A change to `truncate()` or `max_lines()` over
unchanged text, under the same widget id, keeps the previous row limit until something
else invalidates the entry. `max_rows` is also the reason for the `usize::MAX - 1` value.
A job with no limit still needs a finite row count, because that count disables the
paragraph-splitting optimization in egui (egui #5411).

**Files:** `label.rs` (CachedMarkdownLayout, CachedFlushRange, hash_text,
hash_flush_context, hash_code_block_identity, resolve_wrap, apply_live_wrap), `layout.rs`
(StreamingCodeCache, scrolling_code_galley)

## Viewport culling

**Problem:** A 10,000-line document in a scroll area lays out and paints all of its
content on every frame, even though only about 50 lines are visible.

**Solution:** The widget caches the rendered size of each block element and each text
segment. Before it renders one, it estimates the screen rect and calls
`ui.is_rect_visible()`. If the rect is off-screen, the widget calls `ui.allocate_space()`
to reserve the correct space, which keeps the scrollbars correct. It then lays out nothing
and paints nothing.

A measured size does depend on the width, but a layout job does not. The segment size
cache therefore stores the width it measured at together with the hash, and a hit needs
both values to match. This is the counterpart to the layout keys above, which leave the
width out.

This reduces the layout and paint work per frame from O(document) to O(visible area).

A culled segment or block must still know that its cached size is valid, and a hash of its
tokens on every frame would cost O(document). So each cache entry records the parse generation
it was last used in, from the incremental parse below. An entry is valid without a hash when no
parse ran since, or when the new parse kept the entry's tokens from the parse it was last used
in. A frame key, a hash of the style, the link handler, the font, the color and the theme,
guards against a change that is not in the text. When neither holds, the label hashes the
tokens and compares, as before.

A frame with unchanged text therefore hashes the text once and walks the token list. A frame
of a streamed reply also parses only the end of the text, and hashes only the segments in it.

**Files:** `label.rs` (flush_text_range size cache, Revision, frame_key, render_token_range block
size caches, BlockHeight, hash_block)

## Incremental parse

**Problem:** A streamed reply changes on every frame, and a parse of the whole document costs
O(document). On a 37 KB document, the parse took about 400us of an 800us frame.

**Solution:** A document can be cut at a resume point, where the parse can start again with no
state from the text before it. A resume point is a line that starts in the first column, after
a blank line, outside a code fence, and that starts neither a list item nor a quote. Every block
before it is closed there, because a list or a quote does not continue past a blank line into
an unindented line. And no text after it changes a token before it.

`ParseState` holds the owned tokens of a parse, the last resume point, the index of its first
token, and the text up to the end of its line. When the next text starts with that text, the
parse keeps the tokens before the index and parses only from the resume point. The last two
kept tokens seed the token list, because they decide the breaks before the next block. The
label takes the cache entry out of egui memory before the parse, so its tokens have one owner
and the parse truncates and extends them without a copy.

Two kinds of line can change text before them, and a text that holds either one is parsed
whole. A link reference definition, such as `[a]: url`, makes a `[a]` anywhere into a link. An
HTML block that starts with `<!--` or `<pre` runs past blank lines. The scan treats every line
that starts with `[` and holds `]:`, and every line that starts with `<`, as one of these.

Heal uses the same point. Heal changes only the text after the last blank line outside a fence,
so the text before a resume point heals to itself, and a frame heals only the end of the text.

`an_incremental_parse_matches_a_full_parse_at_every_prefix` streams the example documents, the
streaming fixtures, and a document of edge cases one character at a time, with heal on and off,
and checks every prefix against a full heal and parse.

**Files:** `parser.rs` (ParseState, parse_tail, last_resume_point, starts_a_resumable_block),
`label.rs` (render)

## Segmented rendering

**Problem:** Some markdown elements cannot be part of a single text galley, because they
need separate egui widgets. These elements are tables, scrolling code blocks,
blockquotes, images, and widget links. One monolithic layout cannot render them.

**Solution:** Layout identifies segment breaks, which are the token indices where the
widget must flush the text galley and render a block widget. The render path then
alternates between text ranges, which it flushes as galleys, and block widgets.

The widget chooses the path before it lays anything out. `needs_segmentation` answers the
same question as `build_layout`, but it reads the tokens only. A document that takes the
segmented path therefore never builds a whole-document layout and then discards it. That
discarded work doubled the cost of every keystroke. The two functions must agree.
`needs_segmentation` must account for every `segment_breaks.push` in `build_layout`, and a
`debug_assert!` on the non-segmented path fails when they do not. A document on the
segmented path holds `None` in the `layout` field of its cache entry and keeps only its
tokens.

This also permits viewport culling and caching per segment, because each segment is
independent.

**Files:** `layout.rs` (needs_segmentation, segment_breaks), `label.rs` (render_segmented, render_token_range, flush_text_range)

## Section-to-token mapping

**Problem:** When the user hovers or clicks the rendered text, the widget must know which
markdown token is under the cursor. The galley knows only about layout sections, which
are styled runs of text, and it knows nothing about tokens.

**Solution:** Layout builds a parallel `Vec<usize>` that maps each section index to the
index of its source token. On hover, the widget finds the section under the cursor
from the glyph position, and then finds the token in O(1).

The function `section_for_char()` reads the sections in order to find the section index
for a character offset. It replaces a per-frame `Vec<u32>` allocation that mapped every
character to its section.

**Files:** `layout.rs` (section_to_token, section_for_char)

## Streaming heal with zero-copy fast path

**Problem:** LLM output arrives one piece at a time. An unclosed code fence, bold marker,
or link makes pulldown-cmark treat all the text that follows as part of that construct.
The renderer must close these constructs before it parses the text.

**Solution:** `heal()` finds the unclosed constructs and appends the closing markers. It
reads them as CommonMark does where that matters. A fence closes only with a run of its own
character that is at least as long. An emphasis run opens only before a non-space character
and closes only after one, so a `* ` bullet opens nothing. A blank line closes every inline
construct. It
returns `Cow::Borrowed` when the text needs no repair, which is the common case for
complete markdown, and this avoids an allocation. Only incomplete input produces a
`Cow::Owned` with a new string.

A marker at the end of the text has no kind until more text arrives. `**` can open bold, or
heal to `****`, which is a rule. A `-` under a line can be a list item or a setext underline,
which makes the line above it a heading. So `heal()` removes a last line that holds only block
markers and a run of inline markers at the end. The next delta then adds text, and it does not
change what already shows. For the same reason, `heal()` removes the URL of an open link. Each
delta of a URL makes a different link, and a partial URL can parse as a valid link to the
wrong target.

A table line has the same problem. The parser makes a table only from a complete header and a
complete separator, and it makes a body row only after a cell starts. So `heal()` removes an
incomplete line that starts with `|` above the separator, and a body row that holds only its
first `|`. When the header is complete, `heal_table` adds the separator.

**Files:** `parser.rs` (heal, heal_inline, heal_table, without_marker_line, without_marker_run,
is_partial_table_line)

## Token size constraint

**Problem:** A token vector can be large, with thousands of entries for a big document. A
large `Token` variant therefore increases the memory use and reduces the cache
performance.

**Solution:** A compile-time test asserts that `Token` stays under 88 bytes. The `Link`
and `Image` variants set the current size, because each one holds three `CowStr` fields,
which is about 80 bytes. The test detects unintended growth from a new field or variant.

**Files:** `types.rs` (size_tests)

## Benchmarks

Criterion benchmarks cover the operations that the caches depend on: the parse
(`parse_100_sections`), the hashes (`hash_text_100_sections`,
`hash_token_slice_100_sections`), and the retrieval (`arc_clone_tokens`). These confirm
that the cost of a cache lookup stays below the cost of the work it prevents.

Four more benchmarks measure a whole frame. A regression in the cache layers therefore
appears as a frame cost, and not only as a slower hash:

- `render_steady_state`: repeated frames at a constant width. The cache layers together
  should make this case cost almost nothing.
- `render_resizing`: a different width on every frame. This is the case that the
  width-independent layout keys exist for.
- `render_scroll_code_steady_state` and `render_scroll_code_streaming_append`: a settled
  code block and a code block that grows. These two cover the append-only syntect path.

**Files:** `benches/markdown.rs`

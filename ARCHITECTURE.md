# Architecture

This file describes how `egui_markdown` renders a document, the caches that make a frame cheap, and the
rules that a change must keep. The rustdoc of the public API describes how to use the crate.

## Pipeline

`MarkdownLabel::show` (`src/label.rs`) does these steps on each frame:

1. **Heal** (`parser::heal`, only with `.heal(true)`). Close the open constructs of incomplete text, such as
   a code fence, emphasis, a link or a table, so that the parse does not read the rest of the text as part
   of them.
2. **Parse** (`parser::parse`). pulldown-cmark events become a flat list of `Token`s (`src/types.rs`).
3. **Choose the path** (`layout::needs_segmentation`). A table, an image, a blockquote, a block widget link
   and a scrolling code block cannot be text in a galley. A document without them takes the whole-document
   path. A document with them takes the segmented path.
4. **Lay out** (`layout::build_layout`). The tokens become one `LayoutJob`, with a map from each section to
   its token, the character spans of code blocks, rules, task list boxes and inline widgets.
5. **Shape and paint** (`MarkdownLabel::render_galley`). egui shapes the job into a galley. The label paints
   the code block backgrounds and the rules, then the galley, then the task list boxes, then the inline
   widgets of the `LinkHandler`. It handles hover, click and text selection on the galley.

A task list item holds a transparent `[x]` or `[ ]` in the middle of a slot that is as wide for both, so
that the text of a checked item and of an unchecked item starts at the same x. The default egui fonts have
no checkbox glyph. The label paints the box over the placeholder with the look of an `egui::Checkbox` that
is not interactive. The label paints the box after the galley. A text selection paints the placeholder in
the selection text color, and the fill of the box covers it. A copy of the item gives the placeholder.

On the segmented path, `render_token_range` walks the tokens. It flushes the text between two blocks as one
galley (`flush_text_range`), and renders each block as a separate egui widget. A blockquote renders its
tokens in a child `Ui`, so a quote can hold blocks too.

`needs_segmentation` reads the tokens only. It must agree with each `segment_breaks.push` in
`build_layout`. `needs_segmentation_matches_build_layout` (`src/layout.rs`) checks this. A `debug_assert!` in
`render` also fails when `needs_segmentation` chooses the whole-document path for a document that has a
segment break.

## Caches

The label keeps its caches in egui temp data. All caches except `FontsEpoch` have ids that the label
derives from the widget id. A streamed text must keep one widget id while it grows. An id that changes with
the text misses all caches on each frame.

| Cache | Id | Key | It skips |
|-------|----|-----|----------|
| `CachedMarkdownLayout` | `id.with("md_cache")` | `hash_text`: the text, the frame key, `max_rows` and `scroll_code_blocks` | The parse, and on the whole-document path the layout. On the segmented path it holds no layout. |
| `CachedFlushRange` | `id.with(("flush", start))` | `hash_flush_context`: the tokens of the range and the frame key | The layout of one range of text on the segmented path. |
| Segment size | `id.with(("flush_sz", start))` | The `CachedFlushRange` key, the wrap width and the break flag | Layout and paint of a range outside the view. |
| `BlockHeight` | `id.with(("block_sz", index))` | `hash_block` (the token, the style and the font), the frame key and the available width | Render and paint of a table, an image or a scrolling code block outside the view. |
| `StreamingCodeCache` | `id.with(("code_block", index)).with("code_galley")` | The language, the code font size, dark mode, `pixels_per_point`, the code theme and the fonts epoch, and then the source | Syntect highlighting of a scrolling code block. |
| `FontsEpoch` | `Id::unique` of the viewport id | none | Nothing. It holds the fonts epoch of the viewport, which the frame key and the code block keys hold. |

Without the `syntax_highlighting` feature, `CachedCodeBlock` replaces `StreamingCodeCache`, and its key holds
the source too.

The **frame key** (`frame_key`) holds every input of a layout that is not a token: the style, the code font
size, `LinkHandler::id`, the font, the text color, the egui visuals that `build_layout` reads (dark mode, the
hyperlink color, the strong text color, `Visuals::code`), the egui sizes that it reads (`Spacing::icon_width`
and the size of `TextStyle::Body`, for the slot of a task list box), the identity of the code theme, and the
fonts epoch. It costs the same on each frame, whatever the length of the document.

**Scrolling code blocks.** The source, the job and the galley of a `StreamingCodeCache` are each in an
`Arc`, so the clone that `get_temp` makes on each frame copies no text. When the identity is the same and the
new source starts with the complete lines of the cached source, the cache keeps the syntect state after the
last complete line. It highlights only the new complete lines and the incomplete last line. So the cost of a
frame of a growing code block follows the length of the last line, not of the block.

**Skipped hashes.** Each `ParseState` has a generation. An incremental parse records the generation it
started from and the number of tokens it kept (`kept_from`). A cache entry records the generation and the
frame key of the frame that last used it. When `Revision::unchanged` holds, the entry is valid without a
hash of its tokens. Otherwise the label hashes the tokens and compares. So a frame with unchanged text hashes
the text once and hashes no segment.

**Fonts epoch.** A cached layout holds widths that depend on the fonts, and a cached galley holds positions
in the font atlas. egui gives no signal when the fonts change, but it clears its galley cache. `fonts_epoch`
lays out a small probe job once per pass, and gives a new epoch when egui returns a new galley.

## Rules

- **The wrap width is not in a layout key.** `build_layout` writes `max_width` and `break_anywhere` into the
  job, and `render_galley` writes the live values over them before egui shapes the job. A layout key with
  the width rebuilds each job on each frame of a resize. The size caches must hold the width, because a size
  depends on it.
- **Each value that `build_layout` reads from `ui` is in the frame key.** A value that is not in it gives a
  stale layout when the value changes. The `a_label_follows_*` tests in `tests/cache.rs` check the font, the
  visuals, the fonts and the code theme, and `a_task_list_follows_the_icon_width_and_the_body_size` checks the
  egui sizes. They cannot check a value that a change adds.
- **`max_rows` is in `hash_text`.** A change of `max_lines` or of `truncate` lays out the document again.
  The segmented path ignores `max_rows`. `a_label_follows_a_change_of_max_lines` checks this.
- **`build_layout` sets `max_rows` to `usize::MAX - 1` when there is no row limit.** A finite row count
  turns off the paragraph split of egui (egui#5411).
- **`map_job` runs after the caches.** The cached job is the job before the map, so a map can change on
  each frame at no layout cost.
- **`Token` is at most 88 bytes.** A long document has thousands of tokens, so the size of a token sets
  the memory of the parse. `token_enum_size` in `src/types.rs` fails when a new field makes it larger.

## Incremental parse and heal

A **resume point** is a line where a parse can start again with no state from the text before it: a line in
the first column, after a blank line, outside a code fence, that starts neither a list item nor a quote.
`ParseState` keeps the owned tokens, the last resume point, the index of its first token, and the text up to
the end of its line. When the next text starts with that text, `ParseState::parse` keeps the tokens before
the index and parses only from the resume point. The last two kept tokens seed the parse, because they decide
the breaks before the next block.

Two kinds of line can change text before them, so a text that holds one is parsed whole. A link reference
definition (`[a]: url`) makes a `[a]` anywhere into a link. An HTML block that starts with `<!--` or `<pre`
continues past blank lines. The scan treats each line that starts with `[` and holds `]:`, and each line that
starts with `<`, as one of them.

Heal changes only the text after the last blank line outside a fence. So the text before a resume point
heals to itself, and `ParseState::heal` heals only the end of the text.

Heal holds back a marker at the end of the text until more text shows what it is: a last line of block
markers (`-` can be a list item or a setext underline), a run of inline markers (`**` can heal to the rule
`****`), the URL of an open link, and an incomplete table line.

`an_incremental_parse_matches_a_full_parse_at_every_prefix` (`src/parser.rs`) streams the example documents,
the streaming fixtures and a document of edge cases one character at a time, with heal on and off. It
compares each prefix with a full heal and parse.

## Benchmarks

`benches/markdown.rs` holds Criterion benchmarks:

- `parse_100_sections` and `hash_token_slice_100_sections`: the operations that a cache hit costs.
- `render_steady_state` and `render_steady_state_long`: frames with no change. The caches must make these
  cost almost nothing.
- `render_resizing`: a new width on each frame. It guards the rule that the wrap width is not in a layout
  key.
- `render_streaming_long` and `render_streaming_long_healed`: a reply that streams in, with heal off and on.
  They guard the incremental parse and the skipped hashes.
- `render_scroll_code_steady_state` and `render_scroll_code_streaming_append`: a scrolling code block that
  does not change, and one that grows.

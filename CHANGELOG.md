# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `OverflowWrap` enum (`Normal`, `BreakAll`) and `MarkdownLabel::overflow_wrap`, which break a
  run of text that is wider than the available width.
- `MarkdownLabel::wrap_mode`, `.wrap()`, `.truncate()`, and `.extend()`, which mirror the same
  methods on the egui `Label`. Truncate elides after `max_lines` rows, which defaults to 1, and
  sets `BreakAll`.
- `LinkHandler::hover_text`, which shows a tooltip while the pointer is on a link. A handler
  uses it to say where a link goes when the link text does not.
- `TableStyle::stroke_width` on `MarkdownStyle`. A non-zero width draws separator lines
  between table cells. Default is `0.0` (no stroke).
- `TableStyle::corner_radius` on the outer table stroke.
- `TableStyle::cell_padding` (`[left, top, right, bottom]`) for the inset inside each cell.
- Overflow chrome on tables: a matching stroke on the visible cut edge, and a
  light inner shadow when the table does not fit horizontally or vertically.
- A link shows its title, such as `"Docs"` in `[text](url "Docs")`, as a tooltip when the
  `LinkHandler` gives no hover text.
- `LinkStyle` derives `Clone`, `Copy`, `Debug`, `PartialEq` and `Eq`.
- `render_steady_state_long` and `render_streaming_long` benchmarks, on a document of 500 sections.

### Changed

- **Breaking:** `build_layout` now takes `max_width: f32` and `break_anywhere: bool`, and no
  longer reads `ui.wrap_mode()` itself. A caller that caches the resulting job must write the
  live wrap values over it before each shape, as `MarkdownLabel` already does.
- **Breaking:** `MarkdownLabel::show` now returns `MarkdownLabelOutput`. Its `response` is the
  union of the responses of the text, and `link_clicked` tells a click on a link from a click on
  other text. A parent widget that reacts to a click uses the two to ignore a link click. A
  caller that uses `show` as the last expression of a closure that returns `()` must add a `;`.
- **Breaking:** `Token::Link` has a `style: TokenStyle` field, the inline style around the link. A
  pattern that names every field of `Token::Link` must add `style` or `..`.
- Holding Shift no longer turns off text selection, so a shift-click extends the selection, as it
  does on `egui::Label`. The selection also stays painted while Shift is down. An app that used
  Shift for something else can pass `.selectable(!ui.input(|i| i.modifiers.shift))`.
- The default `MarkdownStyle::code_font_size` is `12.0`, the size of egui's default monospace text. It
  was `10.0`, which was small beside egui's 12.5 point body text. An app that wants the old size sets
  it.
- **Breaking:** the crates need Rust 1.95, which egui 0.36 needs. `rust-version` said 1.80.
- `heal()` removes a last line that holds only block markers, such as `-` or `1.`, and a run of
  inline markers at the end, such as `**`. It also removes the URL of an open link, so the link
  has an empty href until its `)` arrives. A click on a link with an empty href opens no URL.

### Fixed

- A streamed `**` at the end of the text healed to `****` and showed as a rule for one delta.
  A streamed `-` under a line made the line a heading for one delta.
- A `map_job` that hides text, such as a reveal, now also hides a rule or an inline widget after
  that text. The label previously painted them before the text above them showed.
- A streamed table header showed as raw text, then as a table, and then as raw text again.
  `heal()` now removes an incomplete table line above the separator, and a body row that holds
  only its first `|`. A complete header gets its separator without a blank line between them.
- `TextWrapMode::Truncate` on the surrounding `Ui`, and now on the widget builders, truncates
  the text. It previously behaved as wrap.
- The label draws the link underline half way into the descent of the font. It previously drew
  the underline at the bottom of the row, so a tall line height put a gap under the text.
- `heal()` treated every `*` as emphasis, so a streamed `* item` bullet, or `2 * 3`, showed a stray
  `*`. An emphasis run now opens only before a non-space character and closes only after one, and a
  blank line closes every inline construct.
- `heal()` closed a `~~~` fence with a backtick fence, which showed as a line of code. It now closes
  a fence with a run of the same character and length, and a backtick run inside a tilde fence stays
  code.
- `heal()` no longer cuts the `]` from a footnote reference at the end of the text, counts a table
  header with an empty cell as one column short, or adds a table separator after a blank line.
- The text of a link or an image kept only its last styled run, so `[**bold** and more](u)` showed
  ` and more`. It now keeps every run.
- A later paragraph of a loose list item ran into the first paragraph, and a code block inside an
  item started on the row of the item text. Both now start a row of their own.
- Inline code and links inside a heading now have the size of the heading, and inline code inside
  bold text is bold.
- `LinkStyle::underline` is now read. `false` turns off the hover underline.
- A blockquote now gets block spacing above it, as it already did below it.
- Inline code in a table cell had no space between its background and the next word. A cell that
  holds a link also dropped the inline code background, italics and strikethrough of its other text,
  and put item spacing between its words.
- A block widget link now gets block spacing above and below it, and the text after it no longer starts
  with a space.
- A nested list starts its marker at the x of the text of the item above it. It was indented by the
  width of two spaces, which left the nested text close to the parent's text.
- A horizontal rule had two blank rows above it and two below it, and its own row was a full line
  tall. It now has one blank row on each side, as between two paragraphs, and its row is
  `HorizontalRuleStyle::height` tall.
- The segmented render path no longer copies every token of the document on each cached frame.
- A frame with unchanged text no longer hashes the tokens of every text segment. On a document of 500
  sections, a steady frame costs about 89us, down from 162us.
- A block's cached height is now keyed by the block itself, so a block outside the view stays culled
  while text streams in elsewhere in the document.
- `MarkdownLabel::calculate_size` now heals the text when `heal` is on, and counts tables,
  blockquotes, images and scrolling code blocks. It measures by running the render of `show` in an
  invisible child `Ui`. Its width is still the width of the content.
- `MarkdownLabel::layout_in_ui` now heals the text when `heal` is on. Its docs say that the galley holds
  text only.
- The published package now includes the streaming test fixtures, and `egui_markdown_style` ships
  its license files.

## [0.1.0] - 2026-03-23

### Added

- CommonMark markdown parser via `pulldown-cmark` with extensions: tables, strikethrough, footnotes, task lists.
- `MarkdownLabel` widget with text selection, clickable links, and cached layout.
- Syntax-highlighted code blocks via `syntect` (feature: `syntax_highlighting`).
- Custom syntax theme support via `MarkdownLabel::code_theme()` - pass your own `syntect::highlighting::Theme` instead of the built-in default.
- Scrollable code blocks with horizontal scroll and copy button overlay.
- Code block background fill using `ui.visuals().code_bg_color`.
- Image rendering via `egui_extras` (feature: `images`, `svg`).
- Table rendering with column alignment and pre-measured column widths.
- `heal_table()`, which completes a partial table separator in streaming input.
- Blockquote rendering with configurable indent and vertical bar.
- Horizontal rules.
- Nested ordered and unordered lists.
- Task list checkboxes.
- Footnote references and definitions.
- `heal()` function to auto-close unclosed code fences, bold, italic, strikethrough, inline code, and links for streaming input.
- `MarkdownStyle` for customizable visual styling (inline code, code blocks, headings, horizontal rules, blockquotes, block spacing, code font size, default code language).
- `LinkHandler` trait for custom link styling (`link_style`), click handling (`click`), inline layout (`layout_link`), inline widgets (`inline_widget_size` / `paint_inline_widget`), and block-level widgets (`is_block_widget` / `block_widget`).
- `LinkHandler::id()` for cache invalidation when handler behavior changes.
- Differentiated heading sizes: H1=1.6x, H2=1.35x, H3=1.2x, H4=1.1x, H5=1.05x, H6=1.0x.
- `section_for_char()` on-demand lookup (replaces per-frame allocation).
- Language alias mapping (`ts`/`tsx`/`jsx` to `javascript`) for broader syntax highlighting coverage.
- `render_galley` wrap-width fix - text re-wraps correctly when the container resizes.
- Two examples: `simple` (editor + rendered output), `advanced` (style editor, custom link handlers, inline widgets, streaming simulation).

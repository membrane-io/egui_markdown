# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- The `egui_markdown_style` crate, which holds `MarkdownStyle` and the other style types, so that an
  application can store a style without a dependency on the renderer.
- `set_style` and `global_style`, which install and read one `MarkdownStyle` for an `egui::Context`. A
  label with no `.style()` uses it.
- `set_code_themes` and `code_theme`, which install and read a dark and a light syntect theme for an
  `egui::Context`. A label with no `.code_theme()` uses them.
- `ListStyle` on `MarkdownStyle`. A list marker is right-aligned in a slot, and a wrapped row of an item
  starts at the x of the item text.
- `MarkdownLabel::shrink_code_blocks` and `MarkdownLabel::code_block_min_width`, which size a scrolling
  code block to its content.
- `MarkdownLabel::hug_content`, which sizes the label to its content and not to the available width.
- `MarkdownLabel::map_job`, which rewrites the layout job before the label shapes it, for example to
  fade in new text.
- Inline code has rounded corners, a margin, and a border. It takes all three from
  `egui::Visuals::code`.
- A row that soft-wraps inside a list item or a code line starts at the indent of the line that it
  continues. The label sets `LayoutSection::wrap_indent` for this.
- The `extra_syntaxes` feature, which highlights TypeScript, TSX, JSX and the other syntaxes of `bat`
  through `two-face`.
- A code block shows the italic, bold and underline that the syntect theme gives a scope. Bold code
  uses a registered `"mono-bold"` font family.
- The label renders the HTML `<img>` and `<br>` tags. An `<img>` tag gives a `Token::Image` with its
  `width` and `height` in points. A table cell shows an image and starts a new row at a `<br>`.
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
- `render_steady_state_long`, `render_streaming_long` and `render_streaming_long_healed` benchmarks, on a
  document of 500 sections.

### Changed

- Inline code uses `FontFamily::Monospace`, as a code block does. It keeps the size of the text
  around it. Bold inline code uses the `"mono-bold"` family when the app registers one.
- **Breaking:** `MarkdownStyle::code_font_size` is an `Option<f32>`. `None`, the default, uses the
  size of `TextStyle::Monospace` in the egui style, as `ui.code()` does. `Some(size)` sets a fixed
  size. Version 0.1.0 used a fixed `10.0`.
- **Breaking:** the crates need egui 0.37 from the egui `main` branch, for
  `LayoutSection::wrap_indent` and `TextFormat::background_stroke`. Patch egui to `main` until the
  next egui release. Version 0.1.0 needed egui 0.33.
- **Breaking:** `CodeBlockStyle::corner_radius` is removed. A code block takes its fill and its corner
  radius from `egui::Visuals::code`.
- **Breaking:** `InlineCodeStyle::expand_bg_y`, `bg_corner_radius`, `stroke_dark`, `stroke_light` and
  `stroke_width` are removed. Set the margin, the corner radius and the border of inline code on
  `egui::Visuals::code`.
- **Breaking:** `apply_inline_code_bg` takes `&egui::Visuals` in place of `dark_mode: bool`.
- **Breaking:** `LinkHandler::layout_link` receives the `&Ui`, so that a handler can lay out against
  the live `Ui`.
- A scrolling code block keeps the syntect state of each complete line, and highlights only the new
  lines when the code grows.
- A code block in the text galley gets the code background fill, as a scrolling code block does.
- The scroll bar of a table that scrolls is inside the border of the table.
- **Breaking:** `build_layout` now takes `max_width: f32` and `break_anywhere: bool`, and no
  longer reads `ui.wrap_mode()` itself. A caller that caches the resulting job must write the
  live wrap values over it before each shape, as `MarkdownLabel` already does.
- **Breaking:** `MarkdownLabel::show` now returns `MarkdownLabelOutput`. Its `response` covers all of
  the label: the text, the tables, the images, the code blocks and the block widgets. It is hovered
  when the pointer is anywhere on the label, and it is clicked only when the click is on the text.
  `link_clicked` tells a click on a link from a click on other text. A parent widget that reacts to
  a click uses the two to ignore a link click. A caller that uses `show` as the last expression of a
  closure that returns `()` must add a `;`.
- **Breaking:** `Token::Link` has a `style: TokenStyle` field, the inline style around the link. A
  pattern that names every field of `Token::Link` must add `style` or `..`.
- Holding Shift no longer turns off text selection, so a shift-click extends the selection, as it
  does on `egui::Label`. The selection also stays painted while Shift is down. An app that used
  Shift for something else can pass `.selectable(!ui.input(|i| i.modifiers.shift))`.
- **Breaking:** the crates need Rust 1.95, which egui 0.36 needs. `rust-version` said 1.80.
- **Breaking:** `LinkHandler::paint_inline_widget` now runs once for each link, not once for each
  row. It receives the font of the link and an `InlineWidgetRow` for each row, with the rect and the
  characters of the row, so one widget can paint and react to hover over all of its rows.
- `heal()` removes a last line that holds only block markers, such as `-` or `1.`, and a run of
  inline markers at the end, such as `**`. It also removes the URL of an open link, so the link
  has an empty href until its `)` arrives. A click on a link with an empty href opens no URL.

### Fixed

- A table measured smaller in a sizing pass than it shows: each cell lost its right and bottom
  padding. A `Harness::fit_contents` or an auto-sized `Area` cut off the end of the label.
- A code block in a list item was drawn over the item text and the next item, at the left edge of the
  label. It now starts below the item text, at the x of the item text.
- Without the `syntax_highlighting` feature, a code block was gray and hard to read in light mode.
  It now takes the text color of `egui::Visuals::code`.
- The rendered text was invisible to screen readers and other accessibility tools. Each galley now
  has the widget info of a label, as `egui::Label` does.
- The text of a code block shown with `scroll_code_blocks` could not be selected. It now can, as other
  text can.
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
- A streamed reply is parsed and healed incrementally. When the new text starts with the text of the
  last parse up to its last resume point, the label keeps the tokens before that point and parses and
  heals only the text after it. Cached segments and blocks in the kept tokens skip their hash. On a
  document of 500 sections, a streamed frame costs about 220us, down from 810us, or 945us with heal on.
- `MarkdownLabel::calculate_size` now heals the text when `heal` is on, and counts tables,
  blockquotes, images and scrolling code blocks. It measures by running the render of `show` in an
  invisible child `Ui`. Its width is still the width of the content.
- `MarkdownLabel::layout_in_ui` now heals the text when `heal` is on. Its docs say that the galley holds
  text only.
- A table or an image outside the view keeps no height from an old width. The label measures it
  again when the available width changes.
- A `LinkHandler::layout_link` that reads the egui context inside a table cell no longer runs while
  the label holds the fonts lock.
- Two tables in one label no longer share a scroll id.
- The published package now includes the streaming test fixtures, and `egui_markdown_style` ships
  its license files.
- A label that keeps its id and its text now follows a change of its font, of the egui text style that
  it uses, of the egui visuals (dark or light mode, the hyperlink color, the strong text color and
  `Visuals::code`), of the code theme, of `max_lines` and of `scroll_code_blocks`. Before, it kept the
  layout of an earlier frame.
- A label now follows a change of the fonts (`Context::set_fonts`, `Context::add_font`, or a "bold" or
  "mono-bold" family that the app registers later) and a reset of the font atlas. Before, a list and an
  indented code line kept the widths of the old fonts, and a scrolling code block kept the advances of the
  old fonts and painted parts of other glyphs.
- After a change of `overflow_wrap` or of the wrap mode, `calculate_size` and the culling of text off the
  screen used a size from before the change.

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

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking

- The crates need egui `main` (after egui 0.37.0), for APIs that egui 0.37.0 does not have:
  `Visuals::code`, `LayoutSection::wrap_indent`, and the `background_margin`,
  `background_corner_radius` and `background_stroke` fields of `TextFormat`. Patch egui to `main`, as the
  README shows, until the next egui release. Version 0.1.0 needed egui 0.33. ([#13], [#20])
- The crates need Rust 1.95. Version 0.1.0 needed Rust 1.80. ([#14])
- `MarkdownLabel::show` returns a `MarkdownLabelOutput`. Its `response` covers the whole label, and
  `link_clicked` is true for a click on a link. A caller that used `show` as the last expression of a
  closure that returns `()` must add a `;`. ([#11], [#32])
- `MarkdownStyle::code_font_size` is an `Option<f32>`. `None`, the default, uses the size of
  `TextStyle::Monospace` in the egui style, as `ui.code()` does. Version 0.1.0 used a fixed `10.0`.
  ([#30])
- `CodeBlockStyle::corner_radius` is removed. A code block takes its corner radius from
  `egui::Visuals::code`. ([#20])
- `LinkHandler::layout_link` receives the `&Ui`.
- `LinkHandler::paint_inline_widget` runs once for each link, not once for each row. It receives the
  font of the link and an `InlineWidgetRow` for each row. ([#16])
- `Token::Link` has a `style: TokenStyle` field, and `Token::Image` has `width` and `height` fields. A
  pattern that names every field must add them or `..`. ([#14], [#18])
- `MarkdownStyle` has the new fields `list`, `table` and `line_height_ratio`, and `HorizontalRuleStyle`
  has the new field `height`. A struct literal that names every field must add them or
  `..Default::default()`. ([#2], [#3], [#10], [#34])
- The modules `label`, `layout`, `link`, `paint`, `parser`, `style`, `table`, `theme` and `types` are
  private. Import the public types from the crate root, for example `egui_markdown::InlineCodeStyle` in
  place of `egui_markdown::style::InlineCodeStyle`. ([#37])
- The layout, paint and table functions of those modules are not public, for example `build_layout`,
  `highlight_code`, `compute_code_block_rects` and `render_table`. ([#37])
- `cursor_from_pos`, `glyph_at_index`, `last_non_whitespace_glyph` and `CodeThemeArg` are not public.
  ([#37])
- `default_code_theme` is removed. Call `egui_extras::syntax_highlighting::CodeTheme::from_style`.
  ([#37])
- `MarkdownStyle::ui` does not show the Dark and Light buttons. They changed the visuals of the whole
  app. The `style` example shows them. ([#37])

### Added

- Without the `syntax_highlighting` feature, code blocks use the built-in highlighter of `egui_extras`
  instead of one plain color.
- The `egui_markdown_style` crate, which holds `MarkdownStyle` and the other style types. An
  application can store a style without a dependency on the renderer. ([#2])
- `set_style` and `global_style`, which install and read one `MarkdownStyle` for an `egui::Context`.
- `set_code_themes` and `code_theme`, which install and read a dark and a light syntect theme for an
  `egui::Context`.
- `MarkdownStyle::line_height_ratio`, the height of a row of body text divided by the font size.
  ([#34])
- `ListStyle`: a list marker is right-aligned in a slot, so all items of a level start their text at
  the same x. ([#3])
- `TableStyle` with `stroke_width`, `corner_radius` and `cell_padding`. A table that does not fit
  shows a stroke on the cut edge and an inner shadow. ([#10])
- `MarkdownLabel::wrap_mode`, `wrap`, `truncate` and `extend`, as on `egui::Label`, and
  `MarkdownLabel::overflow_wrap` with `OverflowWrap`. `.truncate().max_lines(3)` elides after three
  rows. ([#6])
- `MarkdownLabel::hug_content`, which makes the label as wide as its content. ([#9])
- `MarkdownLabel::shrink_code_blocks` and `code_block_min_width`, which size a scrolling code block to
  its content. ([#1])
- `MarkdownLabel::map_job`, which changes the layout job before the label shapes it, for example to
  fade in new text. ([#8])
- `LinkHandler::hover_text`, a tooltip for a link. A link with a title, such as `[text](url "Docs")`,
  shows the title when the handler gives no text. ([#11], [#14])
- The `extra_syntaxes` feature: TypeScript, TSX, JSX and the other syntaxes of `bat`, through
  `two-face`. ([#15])
- A code block shows the italic, bold and underline that the syntect theme gives a scope. Bold code
  uses a registered `"mono-bold"` font family. ([#15])
- The HTML tags `<img>`, with `width` and `height`, and `<br>`, also in a table cell. ([#18])
- `LinkStyle` derives `Clone`, `Copy`, `Debug`, `PartialEq` and `Eq`.

### Changed

- The row height follows the font size. A row of body text is `line_height_ratio` times the font size,
  and a heading or inline code row is at least as tall as its font. Headings at the default size get
  taller rows, so a label with a heading is taller than before. ([#34])
- Inline code uses the monospace font, at the size of the text around it. Bold inline code uses the
  `"mono-bold"` family when the app registers one. ([#30])
- A wrapped row of a list item or of a code line starts at the indent of the line. ([#3], [#20])
- A nested list starts its marker at the x of the text of its parent item.
- A horizontal rule has one blank row above it and one below it, and its row is
  `HorizontalRuleStyle::height` tall.
- Inline code takes its margin, corner radius and border from `egui::Visuals::code`. With the default
  egui visuals, inline code has rounded corners and 2 points of space on each side, and no border.
  ([#10], [#20])
- A shift-click extends the text selection, as on `egui::Label`. Shift no longer turns off selection.
  An app that uses Shift for something else can pass `.selectable(!ui.input(|i| i.modifiers.shift))`.
  ([#14])
- `heal()` holds back a marker at the end of the text, such as `-`, `1.` or `**`, and the URL of an
  open link, until more text shows what it is. A click on a link with an empty href opens nothing.
  ([#12], [#42])
- A link in a table cell uses the `LinkHandler`. Version 0.1.0 opened the URL of the link and did not
  call the handler.
- A link in a table cell has the same hover and click as a link in the text. It shows the tooltip of
  `LinkHandler::hover_text`, else the title, else the href when `url_in_tooltip` is set. A click on a
  link with an empty href in a cell does not call `LinkHandler::click`. ([#42])
- `MarkdownLabel::calculate_size` runs the render of `show` in an invisible child `Ui`, so it counts
  tables, blockquotes, images and scrolling code blocks. `calculate_size` and `layout_in_ui` heal the
  text when `heal` is on.
- The background of a code block starts at the left edge of the label and ends at its right edge. The
  label allocates the padding of a code block at its top and bottom. The code text is 4 points further
  right than before, and the code block buttons use the padding of `CodeBlockStyle`. ([#35])
- A code block in the text galley gets the background fill of `egui::Visuals::code`, as a scrolling
  code block does. ([#10])
- Seven small examples replace the examples `simple` and `advanced`: `hello_world`, `editor`, `streaming`,
  `links`, `code_blocks`, `style` and `images`. Each example shows one topic. ([#39])

### Fixed

- A code block in a list item covered the item text and the next item. ([#29])
- A table measured smaller in a sizing pass than it shows, so `Harness::fit_contents` or an auto-sized
  `Area` cut off the end of the label. ([#28])
- Without the `syntax_highlighting` feature, a code block was gray and hard to read in light mode.
  ([#23])
- Screen readers did not read the text of the label. The text of a scrolling code block could not be
  selected. ([#24])
- A label kept an old layout after a change of its font, of the egui fonts, of the visuals, of the code
  theme, of `max_lines` or of `scroll_code_blocks`. ([#33])
- `TextWrapMode::Truncate` on the surrounding `Ui` wrapped the text.
- `heal()` errors in streamed text: a `**` at the end showed as a rule, a `-` under a line made a
  heading, a `* item` bullet or `2 * 3` showed a stray `*`, a `~~~` fence closed with backticks, a
  table header showed as raw text and then as a table, and a footnote reference lost its `]`. ([#12],
  [#14])
- The text of a link or an image kept only its last styled run.
- Inline code and links in a heading did not have the size of the heading. Inline code in bold text was
  not bold.
- `LinkStyle::underline` had no effect.
- A blockquote had no block spacing above it. A block widget link had no block spacing, and the text
  after it started with a space. A later paragraph or a code block of a list item started on the row
  of the item text.
- Inline code in a table cell had no space after its background. A cell with a link dropped the inline
  code background, italics and strikethrough of its other text.
- The link underline was at the bottom of the row, not under the text.
- Two tables in one label shared a scroll id. ([#2])
- The scroll bar of a table was outside the border of the table. ([#17])
- A table or an image outside the view kept its height from an old width. ([#18])
- A link in a table cell showed no tooltip, opened an empty URL, and did not set
  `MarkdownLabelOutput::link_clicked`. ([#42])

### Performance

- The label parses and heals a streamed text only from the last point where the parse can start
  again. ([#14])
- A frame with unchanged text hashes no text segment, and a block outside the view stays culled while
  text streams in elsewhere. ([#14])
- The segmented render path copies no tokens on a cached frame. ([#14])
- A scrolling code block highlights only the new lines when the code grows.

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

[Unreleased]: https://github.com/membrane-io/egui_markdown/commits/main
[0.1.0]: https://crates.io/crates/egui_markdown/0.1.0
[#1]: https://github.com/membrane-io/egui_markdown/pull/1
[#2]: https://github.com/membrane-io/egui_markdown/pull/2
[#3]: https://github.com/membrane-io/egui_markdown/pull/3
[#6]: https://github.com/membrane-io/egui_markdown/pull/6
[#8]: https://github.com/membrane-io/egui_markdown/pull/8
[#9]: https://github.com/membrane-io/egui_markdown/pull/9
[#10]: https://github.com/membrane-io/egui_markdown/pull/10
[#11]: https://github.com/membrane-io/egui_markdown/pull/11
[#12]: https://github.com/membrane-io/egui_markdown/pull/12
[#13]: https://github.com/membrane-io/egui_markdown/pull/13
[#14]: https://github.com/membrane-io/egui_markdown/pull/14
[#15]: https://github.com/membrane-io/egui_markdown/pull/15
[#16]: https://github.com/membrane-io/egui_markdown/pull/16
[#17]: https://github.com/membrane-io/egui_markdown/pull/17
[#18]: https://github.com/membrane-io/egui_markdown/pull/18
[#20]: https://github.com/membrane-io/egui_markdown/pull/20
[#23]: https://github.com/membrane-io/egui_markdown/pull/23
[#24]: https://github.com/membrane-io/egui_markdown/pull/24
[#28]: https://github.com/membrane-io/egui_markdown/pull/28
[#29]: https://github.com/membrane-io/egui_markdown/pull/29
[#30]: https://github.com/membrane-io/egui_markdown/pull/30
[#32]: https://github.com/membrane-io/egui_markdown/pull/32
[#33]: https://github.com/membrane-io/egui_markdown/pull/33
[#34]: https://github.com/membrane-io/egui_markdown/pull/34
[#35]: https://github.com/membrane-io/egui_markdown/pull/35
[#37]: https://github.com/membrane-io/egui_markdown/pull/37
[#39]: https://github.com/membrane-io/egui_markdown/pull/39
[#42]: https://github.com/membrane-io/egui_markdown/pull/42

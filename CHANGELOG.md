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

### Changed

- **Breaking:** `build_layout` now takes `max_width: f32` and `break_anywhere: bool`, and no
  longer reads `ui.wrap_mode()` itself. A caller that caches the resulting job must write the
  live wrap values over it before each shape, as `MarkdownLabel` already does.
- **Breaking:** `MarkdownLabel::show` now returns `MarkdownLabelOutput`. Its `response` is the
  union of the responses of the text, and `link_clicked` tells a click on a link from a click on
  other text. A parent widget that reacts to a click uses the two to ignore a link click. A
  caller that uses `show` as the last expression of a closure that returns `()` must add a `;`.

### Fixed

- `TextWrapMode::Truncate` on the surrounding `Ui`, and now on the widget builders, truncates
  the text. It previously behaved as wrap.
- The label draws the link underline half way into the descent of the font. It previously drew
  the underline at the bottom of the row, so a tall line height put a gap under the text.

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

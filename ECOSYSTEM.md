# Markdown rendering in the Rust / egui ecosystem

A brief survey of related crates.

## egui_markdown and egui_commonmark

`egui_commonmark` adds one egui label for each styled run of text. A paragraph with bold, a link
and inline code is several labels in a wrapping row. It shows a code block in a read-only
`TextEdit`. It parses the whole document again on each frame, and it keeps no parsed tokens and no
layouts between frames.

`egui_markdown` lays out a document as galleys, which are styled text layouts. A document without
block elements is one galley. A table, a blockquote, an image, a block widget link, or a code block
with `scroll_code_blocks(true)` splits the document into segments. Each segment between two blocks is
one galley, and each block is a separate widget. The label keeps the tokens, the layout of each
segment and the measured size of each segment and block in egui memory. On the next frame it parses
only the end of a streamed text, and it lays out only the segments that changed.

Both crates let a user select text across paragraphs, because egui selects text across labels. Both
crates can skip the work for content outside the viewport. `egui_commonmark` does this only in
`show_scrollable` with `viewport_cache(true)`, and it skips whole top-level blocks. `egui_markdown`
does it in any `ScrollArea` with no option, from the cached size of each segment and block.

Features that `egui_markdown` has and `egui_commonmark` does not have:

- **Streaming heal.** `heal()` closes the unclosed constructs in incomplete text, such as a reply of
  an LLM that is not complete. `egui_commonmark` has no equivalent.
- **Custom links.** A `LinkHandler` can style a link, lay it out, and render it as an inline widget or
  as a block widget. A link hook of `egui_commonmark` only records a click on one exact href.
- **A style type.** `MarkdownStyle` holds the sizes, spacing, strokes and colors of the markdown
  elements, and `set_style` installs one style for a context. `egui_commonmark` has a few builder
  options, and it takes its other visuals from the egui `Style`.

Features that `egui_commonmark` has and `egui_markdown` does not have:

- GFM alerts, such as `> [!NOTE]`, with custom colors and icons.
- Definition lists.
- A callback that renders math, and a callback that renders HTML.
- Task list checkboxes that change the source text (`show_mut`).
- Links to headings, full-text search, and remote and embedded images.
- Macros that convert markdown to egui code at compile time (`commonmark!`, `commonmark_str!`).

## egui markdown widgets

| Crate | Description |
|-------|-------------|
| [`egui_commonmark`](https://crates.io/crates/egui_commonmark) | CommonMark viewer for egui with tables, task lists, strikethrough, footnotes, alerts and definition lists. Macro variant available. Widely used. |
| [`egui_markdown`](https://crates.io/crates/egui_markdown) | This crate. One widget call per document, with caches in egui memory, streaming heal, custom link handlers, and a style API. |

## Rust markdown parsers

These are the parsing backends. `egui_markdown` and `egui_commonmark` use `pulldown-cmark`.

| Crate | Description |
|-------|-------------|
| [`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) | Pull parser for CommonMark. Fast, no AST, event-based. Used by rustdoc and mdBook. |
| [`comrak`](https://crates.io/crates/comrak) | Full CommonMark + GFM parser/renderer with AST. Used by crates.io and docs.rs. |
| [`markdown-it`](https://crates.io/crates/markdown-it) | Rust port of markdown-it.js. Extensible syntax (mentions, emoji, custom tags). No release since 2024. |

## Syntax highlighting

| Crate | Description |
|-------|-------------|
| [`syntect`](https://crates.io/crates/syntect) | Sublime Text syntax definitions + themes. Used by `egui_markdown` for code blocks. |
| [`tree-sitter-highlight`](https://crates.io/crates/tree-sitter-highlight) | Highlighting based on an incremental parse. Larger dependency. |

## Standalone markdown viewers / renderers

| Crate | Description |
|-------|-------------|
| [`md-viewer`](https://crates.io/crates/md-viewer) | Lightweight markdown viewer built on egui, for Linux first, with Windows and macOS builds. Renders through a fork of `egui_commonmark`. |
| [`mdcat`](https://crates.io/crates/mdcat) | Terminal markdown renderer with image support. |
| [`termimad`](https://crates.io/crates/termimad) | Terminal markdown renderer using crossterm. |

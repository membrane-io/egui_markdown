# egui_markdown

[![crates.io](https://img.shields.io/crates/v/egui_markdown.svg)](https://crates.io/crates/egui_markdown)
[![docs.rs](https://docs.rs/egui_markdown/badge.svg)](https://docs.rs/egui_markdown)
[![license](https://img.shields.io/crates/l/egui_markdown.svg)](https://github.com/iamseeley/egui_markdown#license)
![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-brightgreen.svg)

A markdown parser and renderer for [egui](https://github.com/emilk/egui).

Parses [CommonMark](https://commonmark.org/) markdown into a token stream, then renders it as a single
interactive egui widget with selectable text, clickable links, syntax-highlighted
code blocks, tables, images, blockquotes, lists, and more.

## Quick Start

```rust
use eframe::egui;
use egui_markdown::MarkdownLabel;

fn show_markdown(ui: &mut egui::Ui) {
    let text = "# Hello\n\nThis is **bold** and *italic*.";
    MarkdownLabel::new(ui.id().with("md"), text).show(ui);
}
```

## Supported Markdown

**CommonMark** - headings (H1-H6), paragraphs, bold, italic, strikethrough,
inline code, fenced code blocks, links, images, blockquotes (nested), ordered
and unordered lists (nested), horizontal rules.

**GitHub Flavored Markdown** - tables (with column alignment), task lists,
footnotes.

## Features

- **Text selection** - rendered markdown is selectable just like normal egui text.
- **Scrollable code blocks** - long lines scroll horizontally instead of wrapping.
- **Code block overlays** - attach buttons (copy, language badge, etc.) to code blocks via a callback.
- **Syntax highlighting** - `syntect`-based highlighting with built-in base16-ocean dark/light themes and support for custom themes.
- **Custom link handlers** - the `LinkHandler` trait lets you style links, handle clicks, render links as inline widgets or full block-level widgets, and override layout.
- **Streaming / heal mode** - `.heal(true)` auto-closes unclosed code fences, bold, italic, links, and tables so partially-received LLM output renders correctly.
- **Configurable style** - `MarkdownStyle` controls inline code colors, code block padding/radius/stroke, code font size, heading scales, horizontal rule stroke, blockquote indent, and block spacing. Includes a built-in interactive editor via `MarkdownStyle::ui()`.
- **Bold font family** - uses a registered `"bold"` font family when available, falling back to strong text color.
- **Layout caching + viewport culling** - hash-based layout caching with per-segment viewport culling for smooth scrolling through large documents.

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `syntax_highlighting` | Yes | Syntax-highlighted code blocks via `syntect`. |
| `images` | No | Render images inline via `egui_extras` image support. |
| `svg` | No | SVG image support via `egui_extras`. |
| `membrane` | No | Rendering that needs [Membrane's egui fork](https://github.com/membrane-io/egui). Does not compile against published egui — see below. |

### The `membrane` feature

Everything above works against published egui. `membrane` is different: it calls
`epaint::text::LeadingSpace`, `TextFormat::bg_corner_radius`, and a two-dimensional
`TextFormat::expand_bg`, none of which exist upstream, so enabling it without patching egui
to [the fork](https://github.com/membrane-io/egui) is a compile error rather than a
fallback.

```toml
[patch.crates-io]
egui = { path = "path/to/egui/crates/egui" }
epaint = { path = "path/to/egui/crates/epaint" }
egui_extras = { path = "path/to/egui/crates/egui_extras" }
emath = { path = "path/to/egui/crates/emath" }
ecolor = { path = "path/to/egui/crates/ecolor" }
```

Patch `emath` and `ecolor` even though you may not depend on them directly, or you end up
with two incompatible copies of the types egui passes around. Add `eframe` too if you use it.

Three things render differently without the feature. Inline code backgrounds have square
corners, and their padding expands by the same amount horizontally and vertically instead of
separately. A row that soft-wraps inside a list or other indented block returns to the left
margin rather than keeping the indentation of the line it continues. And a long run of text
with no spaces breaks between two glyphs instead of overrunning the available width, which
[`OverflowWrap`](https://docs.rs/egui_markdown/latest/egui_markdown/enum.OverflowWrap.html)
lets you ask for explicitly either way.

Parsing, selection, links, tables, and syntax highlighting are identical either way.

## Customization

### `MarkdownStyle`

Control the visual appearance of all markdown elements via [`MarkdownStyle`](https://docs.rs/egui_markdown/latest/egui_markdown/style/struct.MarkdownStyle.html).

```rust
use egui_markdown::{MarkdownLabel, MarkdownStyle};

let mut style = MarkdownStyle::default();
style.heading.scales[0] = 2.0; // Bigger H1
style.code_font_size = 14.0;
MarkdownLabel::new(id, text).style(&style).show(ui);
```

`MarkdownStyle` also provides a `ui()` method that renders an interactive editor
for all style fields - see the `advanced` example.

### `LinkHandler`

Implement the [`LinkHandler`](https://docs.rs/egui_markdown/latest/egui_markdown/link/trait.LinkHandler.html) trait
to customize link styling, click handling, and rendering. Links can be rendered as
inline widgets (e.g. user mentions, status badges) or block-level widgets (e.g. embeds,
cards) by implementing `inline_widget()` / `is_block_widget()`.

### Code Themes

Pass a custom `syntect::highlighting::Theme` via `.code_theme()` to override the
built-in base16-ocean theme.

### Streaming

Enable `.heal(true)` on `MarkdownLabel` to auto-close unclosed code fences and inline
constructs before parsing. This prevents partial markdown from swallowing subsequent
text - useful for streaming LLM output.

## Examples

| Example | Description |
|---------|-------------|
| `simple` | Editor + rendered output. |
| `advanced` | Style editor, custom link handlers, inline widgets, code block buttons, streaming simulation. |

```sh
cargo run --example simple
cargo run --example advanced
```

## License

MIT or Apache-2.0

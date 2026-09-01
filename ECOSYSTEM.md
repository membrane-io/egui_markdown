# Markdown rendering in the Rust / egui ecosystem

A brief survey of related crates, as of early 2025.

`egui_markdown` renders markdown into one egui galley, which is a styled text layout. It
draws the block elements as separate widgets, and those elements are tables, code blocks,
blockquotes, and images. `egui_commonmark` differs, because it renders every element as
its own widget. One galley permits text selection across paragraphs, viewport culling,
and layout caching.

Three features are unique to `egui_markdown`. It heals streaming input, which means that
it closes the unclosed markdown constructs in incomplete LLM output. Its `LinkHandler`
trait promotes a link to any custom widget. An application can therefore render a custom
protocol link as an interactive widget rather than as plain text. Its `MarkdownStyle` type makes
every visual style configurable.

## egui markdown widgets

| Crate | Description |
|-------|-------------|
| [`egui_commonmark`](https://crates.io/crates/egui_commonmark) | CommonMark + GFM viewer for egui. Macro variant available. Widely used. |
| [`egui_markdown`](https://crates.io/crates/egui_markdown) | This crate. Stateless widget with streaming heal, custom link handlers, style API. |

## Rust markdown parsers

These are the parsing backends. `egui_markdown` uses `pulldown-cmark`.

| Crate | Description |
|-------|-------------|
| [`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) | Pull parser for CommonMark. Fast, no AST, event-based. Used by `cargo doc`. |
| [`comrak`](https://crates.io/crates/comrak) | Full CommonMark + GFM parser/renderer with AST. Used by crates.io and docs.rs. |
| [`markdown-it`](https://crates.io/crates/markdown-it) | Rust port of markdown-it.js. Extensible syntax (mentions, emoji, custom tags). |

## Syntax highlighting

| Crate | Description |
|-------|-------------|
| [`syntect`](https://crates.io/crates/syntect) | Sublime Text syntax definitions + themes. Used by `egui_markdown` for code blocks. |
| [`tree-sitter-highlight`](https://crates.io/crates/tree-sitter-highlight) | Highlighting based on an incremental parse. Larger dependency. |

## Standalone markdown viewers / renderers

| Crate | Description |
|-------|-------------|
| [`md-viewer`](https://crates.io/crates/md-viewer) | Lightweight Linux markdown viewer built on egui. |
| [`mdcat`](https://crates.io/crates/mdcat) | Terminal markdown renderer with image support. |
| [`termimad`](https://crates.io/crates/termimad) | Terminal markdown renderer using crossterm. |

#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! A markdown parser and renderer for [egui](https://github.com/emilk/egui).
//!
//! Parses CommonMark markdown into a token stream, then renders it as an interactive
//! egui widget with support for text formatting, links, code blocks, tables, images,
//! blockquotes, lists, and more.
//!
//! # Quick Start
//!
//! ```no_run
//! # use eframe::egui;
//! fn show_markdown(ui: &mut egui::Ui) {
//!     let text = "# Hello\n\nThis is **bold** and *italic*.";
//!     egui_markdown::MarkdownLabel::new(ui.id().with("md"), text).show(ui);
//! }
//! ```
//!
//! # Feature Flags
//!
//! - `syntax_highlighting` (default) - Syntax-highlighted code blocks via `syntect`.
//! - `images` - Render images inline using `egui_extras` image support.
//! - `svg` - SVG image support.
//!
// TODO: Math/LaTeX support. pulldown-cmark already supports `Options::ENABLE_MATH` which
// emits `InlineMath`/`DisplayMath` events for `$...$` and `$$...$$`. To add math rendering:
//   1. Add a `math` feature flag that enables ENABLE_MATH in the parser.
//   2. Add Token::InlineMath / Token::DisplayMath variants.
//   3. Create a separate `egui_math` crate that takes a LaTeX string and paints it
//      using egui's Painter (text for symbols/Greek letters, lines for fraction bars,
//      paths for roots, etc.). Use `pulldown-latex` to parse LaTeX into a layout tree.
//   4. Feature-gate the dependency: `math = ["dep:egui_math"]`, delegating to it
//      for inline/display math rendering (same pattern as egui_extras for tables).

/// The [`MarkdownLabel`] widget.
pub mod label;
/// Layout job construction from token streams.
pub mod layout;
/// Link handler trait and link styling.
pub mod link;
/// Code block and horizontal rule painting.
pub mod paint;
/// CommonMark markdown parser.
pub mod parser;
/// Customizable visual styling for markdown rendering.
pub mod style;
/// Table rendering.
pub mod table;
/// Syntax highlighting theme utilities.
#[cfg(feature = "syntax_highlighting")]
pub mod theme;
/// Parsed markdown token types.
pub mod types;

pub use egui_markdown_style::{global_style, set_style};
pub use label::MarkdownLabel;
pub use label::{cursor_from_pos, glyph_at_index, last_non_whitespace_glyph, OverflowWrap};
pub use layout::CodeThemeArg;
pub use link::{LinkHandler, LinkStyle};
pub use parser::{heal, parse};
pub use style::MarkdownStyle;
pub use types::{Alignment, Markdown, TableData, Token, TokenStyle};

#[cfg(feature = "syntax_highlighting")]
pub use theme::{code_theme, default_code_theme, set_code_themes};

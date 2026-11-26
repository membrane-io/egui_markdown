#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! A markdown parser and renderer for [egui](https://github.com/emilk/egui).
//!
//! The crate parses CommonMark markdown into a list of [`Token`]s. [`MarkdownLabel`] renders the tokens
//! as one interactive egui widget, with selectable text, links, code blocks, tables, images, blockquotes
//! and lists. Of the raw HTML in the text, it renders the `<img>` and `<br>` tags, and it shows each other
//! tag as written.
//!
//! # Quick Start
//!
//! ```no_run
//! # use eframe::egui;
//! fn show_markdown(ui: &mut egui::Ui) {
//!     let text = "# Hello\n\nThis is **bold** and *italic*.";
//!     egui_markdown::MarkdownLabel::new(ui.make_persistent_id("md"), text).show(ui);
//! }
//! ```
//!
//! # Feature Flags
//!
//! - `syntax_highlighting` (default) - Syntax-highlighted code blocks via `syntect`.
//! - `extra_syntaxes` - The syntaxes of `bat` through `two-face`, with TypeScript, TSX and JSX.
//! - `images` - Render images through the `egui_extras` image loaders. Call
//!   `egui_extras::install_image_loaders(ctx)` once at startup. The feature also turns on the HTTP
//!   loader, so an image URL can be remote.
//! - `svg` - SVG image support.

mod image;
mod label;
mod layout;
mod link;
mod paint;
mod parser;
mod style;
mod table;
#[cfg(feature = "syntax_highlighting")]
mod theme;
mod types;

pub use egui_markdown_style::{global_style, set_style};
pub use label::{MarkdownLabel, MarkdownLabelOutput, OverflowWrap};
pub use link::{InlineWidgetRow, LinkHandler, LinkStyle};
pub use parser::{heal, parse};
pub use style::{
  BlockquoteStyle, CodeBlockStyle, HeadingStyle, HorizontalRuleStyle, InlineCodeStyle, ListStyle, MarkdownStyle,
  TableStyle,
};
pub use types::{Alignment, Markdown, TableData, Token, TokenStyle};

#[cfg(feature = "syntax_highlighting")]
pub use theme::{code_theme, set_code_themes};

#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! A markdown parser and renderer for [egui](https://github.com/emilk/egui).
//!
//! The crate parses CommonMark markdown into a list of [`Token`]s. [`MarkdownLabel`] renders the tokens
//! as one interactive egui widget, with selectable text, links, code blocks, tables, images, blockquotes
//! and lists. Of the raw HTML in the text, it renders the `<img>` and `<br>` tags, and it shows each other
//! tag as written.
//!
//! # Quick start
//!
//! ```no_run
//! # use eframe::egui;
//! fn show_markdown(ui: &mut egui::Ui) {
//!     let text = "# Hello\n\nThis is **bold** and *italic*.";
//!     egui_markdown::MarkdownLabel::new(ui.make_persistent_id("md"), text).show(ui);
//! }
//! ```
//!
//! # Supported markdown
//!
//! - CommonMark: headings (H1 to H6), paragraphs, bold, italic, strikethrough, inline code, fenced code
//!   blocks, links, images, nested blockquotes, nested ordered and unordered lists, and horizontal rules.
//! - GitHub Flavored Markdown: tables with column alignment, task lists and footnotes.
//!
//! # Style
//!
//! [`MarkdownStyle`] controls the look of each markdown element. Give a style to one label with
//! [`MarkdownLabel::style`]:
//!
//! ```no_run
//! # use eframe::egui;
//! use egui_markdown::{MarkdownLabel, MarkdownStyle};
//!
//! fn ui(ui: &mut egui::Ui, text: &str, style: &MarkdownStyle) {
//!     MarkdownLabel::new(ui.make_persistent_id("md"), text).style(style).show(ui);
//! }
//! ```
//!
//! Or install one style for all labels of a context with [`set_style`]. Call it one time at
//! startup, for example in the constructor of the app, and not in each frame:
//!
//! ```no_run
//! # use eframe::egui;
//! use egui_markdown::MarkdownStyle;
//!
//! struct App;
//!
//! impl App {
//!     fn new(cc: &eframe::CreationContext<'_>) -> Self {
//!         let mut style = MarkdownStyle::default();
//!         style.heading.scales[0] = 2.0; // A bigger H1.
//!         style.code_font_size = Some(14.0); // `None` uses the egui monospace size.
//!         egui_markdown::set_style(&cc.egui_ctx, style);
//!         Self
//!     }
//! }
//! ```
//!
//! `MarkdownStyle::ui` shows an editor for each field of the style. The `style` example uses it.
//!
//! A label uses a registered `"bold"` font family for bold text. Without one, bold text has the strong
//! text color of the egui visuals. A code block shows the italic, bold and underline that the syntect theme
//! gives a scope. Bold code uses a registered `"mono-bold"` font family. Without one, bold code keeps the
//! regular monospace family.
//!
//! # Clicks
//!
//! [`MarkdownLabel::show`] returns a [`MarkdownLabelOutput`]. The label handles a click on a link itself,
//! so a parent that reacts to a click on the label checks both fields:
//!
//! ```no_run
//! # use eframe::egui;
//! # fn select_message() {}
//! fn ui(ui: &mut egui::Ui, text: &str) {
//!     let output = egui_markdown::MarkdownLabel::new(ui.make_persistent_id("md"), text).show(ui);
//!     if output.response.clicked() && !output.link_clicked {
//!         select_message();
//!     }
//! }
//! ```
//!
//! # Other options
//!
//! - Links: [`LinkHandler`] changes the look of a link, its tooltip and its click. It can also show a
//!   link as an inline widget or a block widget.
//! - Streaming: [`MarkdownLabel::heal`] closes open constructs in incomplete text, and
//!   [`MarkdownLabel::map_job`] changes the layout before egui shapes it, for example to fade in new text.
//! - Wrapping: [`MarkdownLabel::wrap`], [`MarkdownLabel::truncate`], [`MarkdownLabel::extend`],
//!   [`MarkdownLabel::max_lines`] and [`MarkdownLabel::overflow_wrap`].
//! - Code themes: `MarkdownLabel::code_theme` sets the syntect theme of one label, and `set_code_themes`
//!   installs a dark and a light theme for all labels of a context.
//! - Code blocks: [`MarkdownLabel::code_block_buttons`], [`MarkdownLabel::scroll_code_blocks`] and
//!   [`MarkdownLabel::shrink_code_blocks`].
//! - Size: [`MarkdownLabel::hug_content`], [`MarkdownLabel::calculate_size`] and
//!   [`MarkdownLabel::layout_in_ui`].
//!
//! # Feature flags
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

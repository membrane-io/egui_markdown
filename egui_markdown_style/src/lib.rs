#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! The style types of [`egui_markdown`](https://docs.rs/egui_markdown).
//!
//! [`MarkdownStyle`] holds the sizes, the spacing and the strokes of the markdown elements. [`set_style`]
//! installs one style for an [`egui::Context`], and [`global_style`] reads it. An application can store a
//! style with this crate and without a dependency on the renderer.

mod style;

pub use style::{
  BlockquoteStyle, CodeBlockStyle, HeadingStyle, HorizontalRuleStyle, InlineCodeStyle, ListStyle, MarkdownStyle,
  TableStyle,
};

use std::sync::Arc;

use egui::{Context, Id};

/// The style that [`set_style`] installs, in the temp data of the context.
#[derive(Clone)]
struct MarkdownStyleSlot(Arc<MarkdownStyle>);

/// Install `style` for all labels of `ctx`. A label without its own style uses it.
///
/// The context keeps an [`Arc`], so a call on each change of the app theme costs little.
pub fn set_style(ctx: &Context, style: impl Into<Arc<MarkdownStyle>>) {
  ctx.data_mut(|d| d.insert_temp(Id::NULL, MarkdownStyleSlot(style.into())));
}

/// The style that [`set_style`] installed for `ctx`, or [`MarkdownStyle::default`].
pub fn global_style(ctx: &Context) -> Arc<MarkdownStyle> {
  ctx
    .data(|d| d.get_temp::<MarkdownStyleSlot>(Id::NULL).map(|s| Arc::clone(&s.0)))
    .unwrap_or_else(|| Arc::new(MarkdownStyle::default()))
}

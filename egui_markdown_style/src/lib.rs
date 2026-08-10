#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Visual style types for [`egui_markdown`](https://docs.rs/egui_markdown), plus a
//! context-scoped default stored as [`std::sync::Arc`] (same idea as egui's
//! [`egui::Style`]).

mod style;

pub use style::{
  BlockquoteStyle, CodeBlockStyle, HeadingStyle, HorizontalRuleStyle, InlineCodeStyle, ListStyle, MarkdownStyle,
};

use std::sync::Arc;

use egui::{Context, Id};

/// Temp-data slot holding the context-wide markdown style.
#[derive(Clone)]
struct MarkdownStyleSlot(Arc<MarkdownStyle>);

/// Install a context-wide markdown style. Cheap to call on theme changes —
/// stores an [`Arc`] that widgets clone or deref.
pub fn set_style(ctx: &Context, style: impl Into<Arc<MarkdownStyle>>) {
  ctx.data_mut(|d| d.insert_temp(Id::NULL, MarkdownStyleSlot(style.into())));
}

/// Return the context-wide markdown style, or [`MarkdownStyle::default`] if none
/// has been installed yet.
pub fn global_style(ctx: &Context) -> Arc<MarkdownStyle> {
  ctx
    .data(|d| d.get_temp::<MarkdownStyleSlot>(Id::NULL).map(|s| Arc::clone(&s.0)))
    .unwrap_or_else(|| Arc::new(MarkdownStyle::default()))
}

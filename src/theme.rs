//! The syntect themes of code blocks.

use std::sync::Arc;

use egui::{Context, Id};

/// The syntect themes that [`set_code_themes`] installs, in the temp data of the context.
#[cfg(feature = "syntax_highlighting")]
#[derive(Clone)]
struct CodeThemesSlot {
  dark: Arc<syntect::highlighting::Theme>,
  light: Arc<syntect::highlighting::Theme>,
}

/// Install a dark and a light syntect theme for all labels of `ctx`. A label that has no
/// [`crate::MarkdownLabel::code_theme`] uses the theme for the current dark mode. Without either, a label
/// uses the built-in `base16-ocean` themes.
#[cfg(feature = "syntax_highlighting")]
pub fn set_code_themes(
  ctx: &Context,
  dark: impl Into<Arc<syntect::highlighting::Theme>>,
  light: impl Into<Arc<syntect::highlighting::Theme>>,
) {
  ctx.data_mut(|d| d.insert_temp(Id::NULL, CodeThemesSlot { dark: dark.into(), light: light.into() }));
}

/// The syntect theme that [`set_code_themes`] installed for `dark_mode`, or `None`.
#[cfg(feature = "syntax_highlighting")]
pub fn code_theme(ctx: &Context, dark_mode: bool) -> Option<Arc<syntect::highlighting::Theme>> {
  ctx.data(|d| {
    d.get_temp::<CodeThemesSlot>(Id::NULL).map(|s| if dark_mode { Arc::clone(&s.dark) } else { Arc::clone(&s.light) })
  })
}

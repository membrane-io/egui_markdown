//! Syntax highlighting theme utilities.

use std::sync::Arc;

use egui::{Context, Id, Style};

/// Return the default [`CodeTheme`](egui_extras::syntax_highlighting::CodeTheme) for the given egui style.
#[cfg(feature = "syntax_highlighting")]
pub fn default_code_theme(style: &Style) -> egui_extras::syntax_highlighting::CodeTheme {
  egui_extras::syntax_highlighting::CodeTheme::from_style(style)
}

/// Temp-data slot holding context-wide syntect themes for dark/light mode.
#[cfg(feature = "syntax_highlighting")]
#[derive(Clone)]
struct CodeThemesSlot {
  dark: Arc<syntect::highlighting::Theme>,
  light: Arc<syntect::highlighting::Theme>,
}

/// Install context-wide syntect themes used when a widget does not pass
/// [`crate::MarkdownLabel::code_theme`].
#[cfg(feature = "syntax_highlighting")]
pub fn set_code_themes(
  ctx: &Context,
  dark: impl Into<Arc<syntect::highlighting::Theme>>,
  light: impl Into<Arc<syntect::highlighting::Theme>>,
) {
  ctx.data_mut(|d| {
    d.insert_temp(
      Id::NULL,
      CodeThemesSlot { dark: dark.into(), light: light.into() },
    )
  });
}

/// Return the installed syntect theme for the given mode, if any.
#[cfg(feature = "syntax_highlighting")]
pub fn code_theme(ctx: &Context, dark_mode: bool) -> Option<Arc<syntect::highlighting::Theme>> {
  ctx.data(|d| {
    d.get_temp::<CodeThemesSlot>(Id::NULL).map(|s| {
      if dark_mode {
        Arc::clone(&s.dark)
      } else {
        Arc::clone(&s.light)
      }
    })
  })
}

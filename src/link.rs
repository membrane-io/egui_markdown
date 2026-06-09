//! Link handler trait and styling for customizing link behavior.

use egui::{text::LayoutJob, Color32, FontId, Rect, Response, Ui, Vec2};

/// Trait for customizing link behavior in rendered markdown.
///
/// Methods are checked in priority order during layout:
///
/// 1. [`Self::is_block_widget`] - standalone widget that breaks text flow ([`Self::block_widget`])
/// 2. [`Self::inline_widget_size`] - widget painted over reserved inline space ([`Self::paint_inline_widget`])
/// 3. [`Self::layout_link`] - custom styled text sections in the galley
/// 4. [`Self::link_style`] - color/underline override on default text
/// 5. default - hyperlink-colored text
pub trait LinkHandler {
  /// Customize how a link is displayed based on its href.
  /// Return `None` for default hyperlink styling.
  fn link_style(&self, _href: &str) -> Option<LinkStyle> {
    None
  }

  /// Handle a link click. Return true if handled, false for default (open URL in browser).
  fn click(&self, _text: &str, _href: &str, _ui: &mut Ui) -> bool {
    false
  }

  /// Append custom styled sections to the LayoutJob for this link.
  /// Called during layout. Return true if handled, false for default (colored text).
  ///
  /// The handler can append multiple sections with different fonts, colors,
  /// backgrounds (icons, colored path segments, etc). All sections will be
  /// mapped to this link token for hover/click.
  fn layout_link(
    &self,
    _ui: &Ui,
    _text: &str,
    _href: &str,
    _job: &mut LayoutJob,
    _font: &FontId,
    _color: Color32,
  ) -> bool {
    false
  }

  /// Opt this link into inline widget rendering by returning `Some(size)`.
  ///
  /// The height becomes `line_height` on placeholder sections so the row grows
  /// tall enough. Width is controlled by [`Self::layout_link`] (which emits the
  /// invisible placeholder text). Returns `None` by default (not an inline widget).
  fn inline_widget_size(&self, _href: &str, _font: &FontId) -> Option<Vec2> {
    None
  }

  /// Paint a widget over the placeholder space reserved by [`Self::inline_widget_size`].
  ///
  /// Called after galley positioning, once per row the placeholder spans.
  /// `rect` is in screen-space coordinates.
  fn paint_inline_widget(&self, _ui: &mut Ui, _text: &str, _href: &str, _rect: Rect) {}

  /// Should this link render as a standalone block-level widget?
  /// When true, the link becomes a segment break and [`Self::block_widget`]
  /// is called to render it outside the text galley.
  fn is_block_widget(&self, _href: &str) -> bool {
    false
  }

  /// Render a block-level widget for this link.
  /// Called when [`Self::is_block_widget`] returned true. The handler has full control:
  /// render any widget, handle drag/hover via the Response.
  /// Returns `None` if not handled.
  fn block_widget(&self, _ui: &mut Ui, _text: &str, _href: &str) -> Option<Response> {
    None
  }

  /// Identity key for cache invalidation. Change when handler behavior
  /// changes for the same markdown text.
  fn id(&self) -> u64 {
    0
  }
}

/// Visual style overrides for a specific link.
pub struct LinkStyle {
  /// Custom text color, or `None` for the default hyperlink color.
  pub color: Option<Color32>,
  /// Whether to draw an underline beneath the link text.
  pub underline: bool,
}

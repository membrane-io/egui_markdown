//! The [`LinkHandler`] trait, which changes how links look and what a click on a link does.

use egui::{text::LayoutJob, Color32, FontId, OpenUrl, Rect, Response, Ui, Vec2};
use std::ops::Range;

/// Changes how the links of a [`crate::MarkdownLabel`] look and what a click on a link does.
///
/// Give a handler to the label with [`crate::MarkdownLabel::link_handler`]. Each method has a default, so a
/// handler implements only the methods that it needs. A chat app can show a mention as a badge, and a
/// documentation viewer can open a link in the app instead of in the browser.
///
/// # Layout
///
/// The label asks the handler about each link in this order, and uses the first answer that applies:
///
/// 1. [`Self::is_block_widget`]: the link becomes a widget with rows of its own. The label ends the galley of
///    the text before the link, calls [`Self::block_widget`], and starts a new galley for the text after it.
/// 2. [`Self::inline_widget_size`]: the link becomes a widget inside the text. The label reserves space in the
///    text, and calls [`Self::paint_inline_widget`] after it places the galley.
/// 3. [`Self::layout_link`]: the handler writes the sections of the link into the layout job, for example
///    an icon and colored text.
/// 4. [`Self::link_style`]: the handler changes the color of the link and the hover underline.
/// 5. Else the link has the hyperlink color of the egui visuals.
///
/// A link takes the size, the weight and the slant of the text around it, so a link in a heading has the
/// size of the heading.
///
/// # Interaction
///
/// - **Hover**: the cursor is a pointing hand, and the link is underlined unless [`LinkStyle::underline`]
///   is `false`. The tooltip is [`Self::hover_text`], else the title of the link, else the href when
///   [`egui::Style::url_in_tooltip`] is set. An inline widget gets no underline and no tooltip.
/// - **Click**: the label calls [`Self::click`]. When it returns `false`, the label opens the href in a new
///   browser tab. A click on a link with an empty href, which [`crate::heal`] gives until the URL arrives,
///   does nothing.
/// - **Table cell**: a link in a table cell has the same hover and click as a link in the text.
/// - **Cache**: the label caches its layout. [`Self::id`] is part of the cache key.
pub trait LinkHandler {
  /// The color of the link and whether it shows an underline on hover. Return `None` for the default
  /// style.
  fn link_style(&self, _href: &str) -> Option<LinkStyle> {
    None
  }

  /// The text of the tooltip that shows while the pointer is on the link, for example where the link goes
  /// when its text does not say. Return `None` for the title of the link, such as `"Docs"` in
  /// `[text](url "Docs")`. A link with no title shows the href when [`egui::Style::url_in_tooltip`] is
  /// set, and no tooltip when it is not.
  fn hover_text(&self, _href: &str) -> Option<String> {
    None
  }

  /// React to a click on the link. Return `true` when the handler used the click, and `false` to let the label
  /// open the href, in a new tab when a modifier key is down, as [`egui::Hyperlink`] does.
  fn click(&self, _text: &str, _href: &str, _ui: &mut Ui) -> bool {
    false
  }

  /// Append the sections of the link to the [`LayoutJob`]. Return `true` when the handler appended them, and
  /// `false` for the default text.
  ///
  /// The handler can append many sections with different fonts, colors and backgrounds, such as an icon and
  /// colored text. The label maps each section that the handler appends to the link, so that hover and click
  /// work on all of them. `font` is the font of the text around the link, and `color` is the link color.
  ///
  /// For an inline widget, the label calls this method with [`Color32::TRANSPARENT`] as `color`. The
  /// sections that the handler appends set the width of the widget, and they must use that color so that
  /// the widget hides them.
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

  /// Return `Some(size)` to show the link as a widget inside the text. Return `None`, the default, for a
  /// link.
  ///
  /// The label uses only the height: it is the row height of the space that the label reserves. The text of
  /// the space sets the width. That text is the text from [`Self::layout_link`] when it returns `true`, else
  /// the link text in a transparent monospace font.
  fn inline_widget_size(&self, _href: &str, _font: &FontId) -> Option<Vec2> {
    None
  }

  /// Paint the widget of the link over the space that [`Self::inline_widget_size`] reserved.
  ///
  /// The label calls this method after it places the galley, once for each link. `rows` holds one
  /// [`InlineWidgetRow`] for each row that the space covers, from top to bottom. `font` is the font that
  /// [`Self::layout_link`] received for the link.
  fn paint_inline_widget(&self, _ui: &mut Ui, _text: &str, _href: &str, _font: &FontId, _rows: &[InlineWidgetRow]) {}

  /// Return `true` to show the link as a widget with rows of its own, which [`Self::block_widget`] renders.
  ///
  /// The widget takes rows of its own, with block spacing above and below it. A link in the middle of a
  /// paragraph splits the paragraph there, so a block widget reads best as a paragraph of its own.
  fn is_block_widget(&self, _href: &str) -> bool {
    false
  }

  /// Render the widget of a link for which [`Self::is_block_widget`] returned `true`. The handler can add
  /// any egui widget to `ui`. Return its response, or `None` when the handler added nothing.
  fn block_widget(&self, _ui: &mut Ui, _text: &str, _href: &str) -> Option<Response> {
    None
  }

  /// A number that the label adds to the keys of its layout caches. Return a new number when the handler
  /// lays out the same link in a different way, so that the label lays out the text again.
  fn id(&self) -> u64 {
    0
  }
}

/// The part of an inline widget placeholder in one row of the galley.
#[derive(Clone, Debug)]
pub struct InlineWidgetRow {
  /// The screen rect from the left edge of the first glyph to the right edge of the last glyph, at the
  /// full height of the row.
  pub rect: Rect,
  /// The characters of the row, counted from the first character that [`LinkHandler::layout_link`]
  /// appended for the link.
  pub chars: Range<usize>,
}

/// The style of one link, from [`LinkHandler::link_style`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkStyle {
  /// Custom text color, or `None` for the default hyperlink color.
  pub color: Option<Color32>,
  /// Whether to draw an underline under the link text while the pointer is on the link.
  pub underline: bool,
}

/// Open `url` the way a click on an [`egui::Hyperlink`] does: in a new tab only when a modifier key is down.
pub(crate) fn open_clicked_url(ui: &Ui, url: &str) {
  let new_tab = ui.input(|i| i.modifiers.any());
  ui.ctx().open_url(OpenUrl { url: url.to_owned(), new_tab });
}

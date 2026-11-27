use egui::{Ui, Vec2};

use crate::link::LinkHandler;
use crate::types::ImageLink;

/// The side of the square that an image takes while it loads, the same as in [`egui::Image`].
const LOADING_SIDE: f32 = 24.0;

/// The size of the image at `url` in a space that is `max_width` wide.
///
/// `width` and `height` come from the attributes of an `<img>` tag. When the tag gives only one of
/// them, the other one keeps the aspect ratio of the image. When it gives neither, the image has the
/// size of its texture. Until the image loads, a side that the tag does not give is
/// [`LOADING_SIDE`], or the side that the tag gives. A wider image shrinks to `max_width` and keeps its
/// aspect ratio.
pub(crate) fn image_size(ui: &Ui, url: &str, width: Option<u16>, height: Option<u16>, max_width: f32) -> Vec2 {
  let source = egui::Image::new(url)
    .load_for_size(ui.ctx(), Vec2::splat(max_width))
    .ok()
    .and_then(|poll| poll.size())
    .filter(|size| size.x > 0.0 && size.y > 0.0);
  let size = match (width.map(f32::from), height.map(f32::from), source) {
    (Some(width), Some(height), _) => Vec2::new(width, height),
    (Some(width), None, Some(source)) => Vec2::new(width, width * source.y / source.x),
    (None, Some(height), Some(source)) => Vec2::new(height * source.x / source.y, height),
    (None, None, Some(source)) => source,
    (Some(side), None, None) | (None, Some(side), None) => Vec2::splat(side),
    (None, None, None) => Vec2::splat(LOADING_SIDE),
  };
  if size.x > max_width && max_width > 0.0 {
    size * (max_width / size.x)
  } else {
    size
  }
}

/// Add `image` to `ui`. Return its response, and whether a click on it opened a link.
///
/// An image in a link is a link to the href of the link: it has the hover, the tooltip and the click of a
/// link, and the accessibility role of a link with the alt text as its name, else the href when the alt text
/// is empty. When the label is not `interactable`, an image in a link is a plain image. The tooltip text comes from
/// [`crate::layout::link_tooltip`] with the title of the image, else the title of the link. A browser does
/// the same for `<a title><img title></a>`: it shows the title of the element under the pointer, and the
/// title of the link when the image has none.
pub(crate) fn add_image(
  ui: &mut Ui,
  image: egui::Image<'_>,
  alt: &str,
  title: Option<&str>,
  link: Option<&ImageLink<'_>>,
  handler: Option<&dyn LinkHandler>,
  interactable: bool,
) -> (egui::Response, bool) {
  let Some(link) = link.filter(|_| interactable) else { return (ui.add(image), false) };
  let mut response = ui.add(image.sense(egui::Sense::click()));
  let enabled = ui.is_enabled();
  let name = if alt.is_empty() { link.href.as_ref() } else { alt };
  response.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::Link, enabled, name));
  response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
  if let Some(tooltip) = crate::layout::link_tooltip(ui, handler, &link.href, title.or(link.title.as_deref())) {
    response = response.on_hover_text_at_pointer(tooltip);
  }
  let clicked = crate::link::link_clicked(&response);
  if clicked {
    crate::layout::open_link(ui, &response, handler, alt, &link.href);
  }
  (response, clicked)
}

/// The href, the title and the link handler of the link that shows an image without the `images` feature.
///
/// An image in a link links to the href of the link, and uses `handler`, because the link is a markdown link.
/// The title is the title of the image, else the title of the link. An image that is not in a link links to
/// its `url`, with its `title` and no link handler.
pub(crate) fn image_link_target<'a>(
  url: &'a str,
  title: Option<&'a str>,
  link: Option<&'a ImageLink<'_>>,
  handler: Option<&'a dyn LinkHandler>,
) -> (&'a str, Option<&'a str>, Option<&'a dyn LinkHandler>) {
  match link {
    Some(link) => (&link.href, title.or(link.title.as_deref()), handler),
    None => (url, title, None),
  }
}

/// The image at `url` at exactly `size`, with a spinner while it loads. `size` comes from
/// [`image_size`], so it already has the aspect ratio that the image must show at.
pub(crate) fn sized_image(url: &str, size: Vec2) -> egui::Image<'_> {
  egui::Image::new(url).fit_to_exact_size(size).maintain_aspect_ratio(false).show_loading_spinner(true)
}

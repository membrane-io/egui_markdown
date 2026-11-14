use egui::{Ui, Vec2};

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

/// The image at `url` at exactly `size`, with a spinner while it loads. `size` comes from
/// [`image_size`], so it already has the aspect ratio that the image must show at.
pub(crate) fn sized_image(url: &str, size: Vec2) -> egui::Image<'_> {
  egui::Image::new(url).fit_to_exact_size(size).maintain_aspect_ratio(false).show_loading_spinner(true)
}

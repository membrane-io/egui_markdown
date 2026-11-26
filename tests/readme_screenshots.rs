//! The screenshots of the README.
//!
//! Regenerate them with `README_SCREENSHOTS=1 cargo test --test readme_screenshots`. Without
//! `README_SCREENSHOTS`, the tests do nothing. The README images stay in plain git, so the tests do not compare
//! them. `tests/elements.rs` holds the snapshot tests.
// The screenshots show the syntect colors of the code block, so the test needs syntect.
#![cfg(feature = "syntax_highlighting")]

use egui::{Id, Theme};
use egui_kittest::Harness;
use egui_markdown::MarkdownLabel;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::DynamicImage;

const MARKDOWN: &str = include_str!("../examples/markdown/readme.md");

/// Render the README document in `theme`, and write the image to `docs/screenshots/{name}.png`.
fn screenshot(theme: Theme, name: &str) {
  if std::env::var_os("README_SCREENSHOTS").is_none() {
    return;
  }
  let mut harness = Harness::builder()
    .with_size([520.0, 1000.0])
    .with_pixels_per_point(2.0)
    .with_theme(theme)
    .wgpu()
    .with_fit_contents()
    .build_ui(|ui| {
      // The harness leaves an 8 pt margin around the panel. Fill it with the panel color.
      let painter = ui.ctx().layer_painter(ui.layer_id());
      painter.rect_filled(ui.ctx().content_rect(), 0.0, ui.visuals().panel_fill);
      // The frame adds 8 pt to the 8 pt margin of the harness, so the text has a 16 pt margin.
      egui::Frame::NONE.inner_margin(8).show(ui, |ui| {
        MarkdownLabel::new(Id::NULL.with("readme"), MARKDOWN).show(ui);
      });
    });
  let image = DynamicImage::ImageRgba8(harness.render().expect("a wgpu render")).into_rgb8();

  // An RGB image with the best compression is about 35 percent smaller than the RGBA image of the harness.
  let path = format!("{}/docs/screenshots/{name}.png", env!("CARGO_MANIFEST_DIR"));
  let file = std::io::BufWriter::new(std::fs::File::create(&path).expect("a writable docs/screenshots"));
  let encoder = PngEncoder::new_with_quality(file, CompressionType::Best, FilterType::Adaptive);
  image.write_with_encoder(encoder).expect("a PNG encode");
  println!("Wrote {path}");
}

#[test]
fn readme_light() {
  screenshot(Theme::Light, "readme-light");
}

#[test]
fn readme_dark() {
  screenshot(Theme::Dark, "readme-dark");
}

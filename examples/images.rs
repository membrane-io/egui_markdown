//! Images from URLs. The `images` feature renders them through the `egui_extras` image loaders.
//!
//! ```sh
//! cargo run --example images --features images
//! ```

use eframe::egui;
use egui_markdown::MarkdownLabel;

const MARKDOWN: &str = include_str!("markdown/images.md");

fn main() -> eframe::Result {
  env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 700.0]),
    ..Default::default()
  };

  eframe::run_native(
    "egui_markdown: images",
    options,
    Box::new(|cc| {
      // Install the image loaders once, at startup.
      egui_extras::install_image_loaders(&cc.egui_ctx);
      Ok(Box::new(App))
    }),
  )
}

struct App;

impl eframe::App for App {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    egui::CentralPanel::default().show(ui, |ui| {
      egui::ScrollArea::vertical().show(ui, |ui| {
        MarkdownLabel::new(ui.make_persistent_id("md"), MARKDOWN).show(ui);
      });
    });
  }
}

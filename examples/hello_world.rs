//! The smallest app: one `MarkdownLabel` in a scroll area.
//!
//! ```sh
//! cargo run --example hello_world
//! ```

use eframe::egui;
use egui_markdown::MarkdownLabel;

const MARKDOWN: &str = include_str!("markdown/hello_world.md");

fn main() -> eframe::Result {
  env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 400.0]),
    ..Default::default()
  };

  eframe::run_ui_native("egui_markdown: hello world", options, |ui, _frame| {
    egui::CentralPanel::default().show(ui, |ui| {
      egui::ScrollArea::vertical().show(ui, |ui| {
        MarkdownLabel::new(ui.make_persistent_id("md"), MARKDOWN).show(ui);
      });
    });
  })
}

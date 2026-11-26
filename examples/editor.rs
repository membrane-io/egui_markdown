//! A text editor on the left and the rendered markdown on the right. The output changes as you type.
//!
//! ```sh
//! cargo run --example editor
//! ```

use eframe::egui;
use egui_markdown::MarkdownLabel;

fn main() -> eframe::Result {
  env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
    ..Default::default()
  };

  let mut text = include_str!("markdown/features.md").to_owned();

  eframe::run_ui_native("egui_markdown: editor", options, move |ui, _frame| {
    egui::Panel::left("source").default_size(450.0).show(ui, |ui| {
      egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add(egui::TextEdit::multiline(&mut text).code_editor().desired_width(f32::INFINITY));
      });
    });

    egui::CentralPanel::default().show(ui, |ui| {
      egui::ScrollArea::vertical().show(ui, |ui| {
        MarkdownLabel::new(ui.make_persistent_id("output"), &text).show(ui);
      });
    });
  })
}

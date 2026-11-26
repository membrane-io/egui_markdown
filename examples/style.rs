//! The editor of `MarkdownStyle` in a side panel, and the buttons of the egui theme.
//!
//! The style lives in the egui context. `global_style` reads it, and `set_style` installs the edited copy
//! for all labels of the app.
//!
//! ```sh
//! cargo run --example style
//! ```

use eframe::egui;
use egui_markdown::MarkdownLabel;

const MARKDOWN: &str = include_str!("markdown/features.md");

fn main() -> eframe::Result {
  env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]),
    ..Default::default()
  };

  eframe::run_ui_native("egui_markdown: style", options, |ui, _frame| {
    egui::Panel::right("style").default_size(300.0).show(ui, |ui| {
      egui::widgets::global_theme_preference_buttons(ui);
      ui.separator();
      egui::ScrollArea::vertical().show(ui, |ui| {
        let mut style = (*egui_markdown::global_style(ui.ctx())).clone();
        style.ui(ui);
        egui_markdown::set_style(ui.ctx(), style);
      });
    });

    egui::CentralPanel::default().show(ui, |ui| {
      egui::ScrollArea::vertical().show(ui, |ui| {
        // The label has no `.style()`, so it uses the style of the context.
        MarkdownLabel::new(ui.make_persistent_id("md"), MARKDOWN).show(ui);
      });
    });
  })
}

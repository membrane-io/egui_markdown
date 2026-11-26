//! Code blocks: a Copy button and the language on each block, horizontal scrolling, and a syntect theme
//! for dark mode and one for light mode.
//!
//! ```sh
//! cargo run --example code_blocks
//! ```

use eframe::egui;
use egui_markdown::MarkdownLabel;
use syntect::highlighting::ThemeSet;

const MARKDOWN: &str = include_str!("markdown/code_blocks.md");

fn main() -> eframe::Result {
  env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 600.0]),
    ..Default::default()
  };

  eframe::run_native(
    "egui_markdown: code blocks",
    options,
    Box::new(|cc| {
      // Install the themes once. Each label uses the theme for the current dark mode.
      let mut themes = ThemeSet::load_defaults().themes;
      let dark = themes.remove("Solarized (dark)").expect("syntect has this theme");
      let light = themes.remove("Solarized (light)").expect("syntect has this theme");
      egui_markdown::set_code_themes(&cc.egui_ctx, dark, light);
      Ok(Box::new(App { scroll: true }))
    }),
  )
}

struct App {
  scroll: bool,
}

impl eframe::App for App {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    egui::Panel::top("options").show(ui, |ui| {
      ui.horizontal(|ui| {
        egui::widgets::global_theme_preference_buttons(ui);
        ui.separator();
        ui.checkbox(&mut self.scroll, "Scroll code blocks");
      });
    });

    egui::CentralPanel::default().show(ui, |ui| {
      egui::ScrollArea::vertical().show(ui, |ui| {
        MarkdownLabel::new(ui.make_persistent_id("md"), MARKDOWN)
          .code_block_buttons(&code_block_buttons)
          .scroll_code_blocks(self.scroll)
          .show(ui);
      });
    });
  }
}

/// Add the buttons of one code block. The label lays them out from right to left, over the top right
/// corner of the block.
fn code_block_buttons(ui: &mut egui::Ui, code: &str, language: &str) {
  if ui.button("Copy").clicked() {
    ui.ctx().copy_text(code.to_owned());
  }
  ui.weak(language);
}

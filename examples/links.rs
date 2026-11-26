//! A `LinkHandler` that changes how links look and what a click on a link does.
//!
//! The handler gives app links a color, a tooltip and a click in the app. It lays out a mention as `@name`,
//! paints a status as a badge inside the text, and shows a card as a widget with rows of its own.
//!
//! ```sh
//! cargo run --example links
//! ```

use std::cell::RefCell;

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Response, Ui, Vec2};
use egui_markdown::{InlineWidgetRow, LinkHandler, LinkStyle, MarkdownLabel};

const MARKDOWN: &str = include_str!("markdown/links.md");

fn main() -> eframe::Result {
  env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([700.0, 600.0]),
    ..Default::default()
  };

  let handler = Handler::default();

  eframe::run_ui_native("egui_markdown: links", options, move |ui, _frame| {
    egui::Panel::bottom("last_click").show(ui, |ui| {
      ui.label(format!("Last click: {}", handler.last_click.borrow()));
    });

    egui::CentralPanel::default().show(ui, |ui| {
      // Show the href of a link that has no title and no hover text.
      ui.style_mut().url_in_tooltip = true;
      egui::ScrollArea::vertical().show(ui, |ui| {
        let output = MarkdownLabel::new(ui.make_persistent_id("md"), MARKDOWN).link_handler(&handler).show(ui);
        // The label handles a click on a link. React only to a click on the other text.
        if output.response.clicked() && !output.link_clicked {
          *handler.last_click.borrow_mut() = "the text".to_owned();
        }
      });
    });
  })
}

#[derive(Default)]
struct Handler {
  last_click: RefCell<String>,
}

impl LinkHandler for Handler {
  fn link_style(&self, href: &str) -> Option<LinkStyle> {
    href.starts_with("app://").then_some(LinkStyle { color: Some(Color32::from_rgb(230, 120, 40)), underline: true })
  }

  fn hover_text(&self, href: &str) -> Option<String> {
    href.strip_prefix("app://").map(|page| format!("Opens the {page} page of the app"))
  }

  fn click(&self, _text: &str, href: &str, _ui: &mut Ui) -> bool {
    *self.last_click.borrow_mut() = href.to_owned();
    // Return `true` for the links that the app handles. The label opens the other links in the browser.
    !href.starts_with("https://")
  }

  fn layout_link(&self, _ui: &Ui, text: &str, href: &str, job: &mut LayoutJob, font: &FontId, color: Color32) -> bool {
    if href.starts_with("user://") {
      let color = Color32::from_rgb(70, 130, 230);
      job.append(&format!("@{text}"), 0.0, TextFormat { font_id: font.clone(), color, ..Default::default() });
      true
    } else if href.starts_with("status://") {
      // This text sets the width of the badge. `color` is transparent, so the text does not show.
      job.append(&format!(" {text} "), 0.0, TextFormat { font_id: font.clone(), color, ..Default::default() });
      true
    } else {
      false
    }
  }

  fn inline_widget_size(&self, href: &str, font: &FontId) -> Option<Vec2> {
    href.starts_with("status://").then(|| Vec2::new(0.0, font.size + 4.0))
  }

  fn paint_inline_widget(&self, ui: &mut Ui, text: &str, href: &str, font: &FontId, rows: &[InlineWidgetRow]) {
    let fill = if href == "status://passing" { Color32::from_rgb(40, 150, 70) } else { Color32::from_rgb(200, 60, 60) };
    let badge_text: Vec<char> = format!(" {text} ").chars().collect();
    // A badge that wraps covers more than one row. Paint the part of the text that each row holds.
    for row in rows {
      let row_text: String = badge_text[row.chars.clone()].iter().collect();
      ui.painter().rect_filled(row.rect, row.rect.height() / 2.0, fill);
      ui.painter().text(row.rect.center(), egui::Align2::CENTER_CENTER, row_text, font.clone(), Color32::WHITE);
    }
  }

  fn is_block_widget(&self, href: &str) -> bool {
    href.starts_with("card://")
  }

  fn block_widget(&self, ui: &mut Ui, text: &str, href: &str) -> Option<Response> {
    let frame = egui::Frame::group(ui.style()).show(ui, |ui| {
      ui.horizontal(|ui| {
        ui.strong(text);
        if ui.button("Open").clicked() {
          *self.last_click.borrow_mut() = href.to_owned();
        }
      });
    });
    Some(frame.response)
  }
}

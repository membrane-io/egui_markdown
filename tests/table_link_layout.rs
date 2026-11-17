use egui::text::LayoutJob;
use egui::{pos2, vec2, Color32, Context, FontId, Id, RawInput, Rect, TextFormat, Ui, UiBuilder, Vec2};
use egui_markdown::{LinkHandler, MarkdownLabel};
use std::sync::mpsc;
use std::time::Duration;

/// A handler that reads the context in `layout_link`, as a handler that lays out a widget with `ui` does.
struct ContextReadingHandler;

impl LinkHandler for ContextReadingHandler {
  fn inline_widget_size(&self, _href: &str, _font: &FontId) -> Option<Vec2> {
    Some(vec2(40.0, 16.0))
  }

  fn layout_link(&self, ui: &Ui, text: &str, _href: &str, job: &mut LayoutJob, font: &FontId, color: Color32) -> bool {
    let pixels_per_point = ui.ctx().pixels_per_point();
    let width = ui.ctx().fonts_mut(|fonts| fonts.glyph_width(font, 'x')) * pixels_per_point;
    job.append(text, width.min(1.0), TextFormat { font_id: font.clone(), color, ..Default::default() });
    true
  }
}

/// A table cell with a link calls the link handler. The handler can read the context, so the table must not call
/// it while the table holds a lock of the context. On native the second lock blocks forever, so the frame runs on
/// another thread and the test fails after a timeout.
#[test]
fn a_link_in_a_table_cell_can_read_the_context_in_layout_link() {
  let (done_tx, done_rx) = mpsc::channel();
  std::thread::spawn(move || {
    let ctx = Context::default();
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
    let text = "| name | link |\n|---|---|\n| a | [gref](membrane://a) |";
    let handler = ContextReadingHandler;
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      MarkdownLabel::new(Id::NULL.with("table_link"), text).link_handler(&handler).show(&mut child);
    });
    output.textures_delta.clear();
    done_tx.send(()).ok();
  });
  assert!(
    done_rx.recv_timeout(Duration::from_secs(20)).is_ok(),
    "the frame did not finish: the table called the link handler while it held a lock of the context"
  );
}

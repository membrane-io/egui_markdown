use egui::{vec2, Context, Id, RawInput, Rect, UiBuilder};
use egui_markdown::MarkdownLabel;

fn layout(md: &str, width: f32, configure: impl FnOnce(MarkdownLabel<'_>) -> MarkdownLabel<'_>) -> (usize, bool, f32) {
  let ctx = Context::default();
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(width, 600.0));
  let mut rows = 0;
  let mut elided = false;
  let mut width_out = 0.0;
  let mut configure = Some(configure);
  let _ = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    let label = configure.take().unwrap()(MarkdownLabel::new(Id::new("truncate"), md));
    let (_pos, galley, _) = label.layout_in_ui(&mut child);
    rows = galley.rows.len();
    elided = galley.elided;
    width_out = galley.size().x;
  });
  (rows, elided, width_out)
}

const LONG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn truncate_yields_one_elided_row() {
  let (rows, elided, _) = layout(LONG, 80.0, |l| l.truncate());
  assert_eq!(rows, 1, "truncate should keep a single row");
  assert!(elided, "truncate should elide overflowing text");
}

#[test]
fn truncate_respects_max_lines() {
  let (rows, elided, _) = layout(LONG, 80.0, |l| l.truncate().max_lines(3));
  assert_eq!(rows, 3, "truncate().max_lines(3) should keep three rows");
  assert!(elided, "text should still be elided after three rows");
}

#[test]
fn extend_ignores_available_width() {
  let width = 80.0;
  let (_, _, galley_w) = layout(LONG, width, |l| l.extend());
  assert!(galley_w > width + 1.0, "extend should allow the galley to exceed available width, got {galley_w}");
}

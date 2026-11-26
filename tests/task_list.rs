//! A task list item shows a checkbox that the label paints. The default egui fonts have no glyph for a
//! checkbox, and the harness panics on a missing glyph.

use egui::epaint::{RectShape, Shape};
use egui::{Id, Pos2, Rect};
use egui_kittest::Harness;
use egui_markdown::MarkdownLabel;

/// A list, a nested list and an ordered list. The label renders this document as one galley.
const LISTS: &str =
  "- [x] alpha\n- [ ] beta\n  - [x] nested done\n  - [ ] nested todo\n\n1. [x] first\n2. [ ] second\n";

/// A task list in a quote. The label renders this document on the segmented path.
const QUOTE: &str = "Intro.\n\n> - [x] quoted done\n> - [ ] quoted todo\n";

/// The boxes and the check mark segments that the label paints for `markdown`, with the default fonts.
fn painted(markdown: &'static str) -> (Vec<RectShape>, Vec<[Pos2; 2]>) {
  let mut harness = Harness::builder().with_size([400.0, 400.0]).build_ui(|ui| {
    MarkdownLabel::new(Id::NULL.with("tasks"), markdown).show(ui);
  });
  harness.run();
  let side = harness.ctx.global_style().spacing.icon_width;
  let mut rects = Vec::new();
  let mut segments = Vec::new();
  for clipped in &harness.output().shapes {
    collect(&clipped.shape, &mut rects, &mut segments);
  }
  // A box has the size of the egui checkbox, because the label font is the body font.
  let boxes: Vec<RectShape> = rects
    .into_iter()
    .filter(|r| (r.rect.width() - side).abs() < 0.01 && (r.rect.height() - side).abs() < 0.01)
    .collect();
  (boxes, segments)
}

fn collect(shape: &Shape, rects: &mut Vec<RectShape>, segments: &mut Vec<[Pos2; 2]>) {
  match shape {
    Shape::Rect(rect) => rects.push(rect.clone()),
    Shape::LineSegment { points, .. } => segments.push(*points),
    Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, rects, segments)),
    _ => {}
  }
}

/// The number of segments that are inside `rect`.
fn segments_in(rect: Rect, segments: &[[Pos2; 2]]) -> usize {
  segments.iter().filter(|[a, b]| rect.contains(*a) && rect.contains(*b)).count()
}

/// Each task item gets one box, and only a checked item gets a check mark of two segments in its box.
fn assert_boxes(markdown: &'static str, checked: &[bool]) {
  let (mut boxes, segments) = painted(markdown);
  assert_eq!(boxes.len(), checked.len(), "one box for each item of {markdown:?}");
  boxes.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
  for (task_box, &checked) in boxes.iter().zip(checked) {
    let expected = if checked { 2 } else { 0 };
    assert_eq!(segments_in(task_box.rect, &segments), expected, "the check mark of the box at {:?}", task_box.rect);
  }
}

#[test]
fn a_task_list_paints_a_box_for_each_item() {
  assert_boxes(LISTS, &[true, false, true, false, true, false]);
}

#[test]
fn a_task_list_in_a_quote_paints_a_box_for_each_item() {
  assert_boxes(QUOTE, &[true, false]);
}

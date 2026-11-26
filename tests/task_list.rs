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

/// The boxes and the check marks that the label paints for `markdown`, with the default fonts, at
/// `pixels_per_point`. A check mark is the points of one line.
fn painted(markdown: &'static str, pixels_per_point: f32) -> (Vec<RectShape>, Vec<Vec<Pos2>>) {
  let mut harness =
    Harness::builder().with_size([400.0, 400.0]).with_pixels_per_point(pixels_per_point).build_ui(|ui| {
      MarkdownLabel::new(Id::NULL.with("tasks"), markdown).show(ui);
    });
  harness.run();
  let side = harness.ctx.global_style().spacing.icon_width;
  let mut rects = Vec::new();
  let mut lines = Vec::new();
  for clipped in &harness.output().shapes {
    collect(&clipped.shape, &mut rects, &mut lines);
  }
  // A box has the size of the egui checkbox, because the label font is the body font.
  let boxes: Vec<RectShape> = rects
    .into_iter()
    .filter(|r| (r.rect.width() - side).abs() < 0.01 && (r.rect.height() - side).abs() < 0.01)
    .collect();
  (boxes, lines)
}

fn collect(shape: &Shape, rects: &mut Vec<RectShape>, lines: &mut Vec<Vec<Pos2>>) {
  match shape {
    Shape::Rect(rect) => rects.push(rect.clone()),
    Shape::LineSegment { points, .. } => lines.push(points.to_vec()),
    Shape::Path(path) => lines.push(path.points.clone()),
    Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, rects, lines)),
    _ => {}
  }
}

/// The lines that are inside `rect`.
fn lines_in(rect: Rect, lines: &[Vec<Pos2>]) -> Vec<&Vec<Pos2>> {
  lines.iter().filter(|points| points.iter().all(|p| rect.contains(*p))).collect()
}

/// Each task item gets one box, and only a checked item gets a check mark in its box. The check mark is one
/// line of three points, as in [`egui::Checkbox`], so that egui joins the two strokes at the bottom point.
fn assert_boxes(markdown: &'static str, checked: &[bool]) {
  let (mut boxes, lines) = painted(markdown, 1.0);
  assert_eq!(boxes.len(), checked.len(), "one box for each item of {markdown:?}");
  boxes.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
  for (task_box, &checked) in boxes.iter().zip(checked) {
    let marks: Vec<usize> = lines_in(task_box.rect, &lines).iter().map(|points| points.len()).collect();
    let expected = if checked { vec![3] } else { vec![] };
    assert_eq!(marks, expected, "the check mark of the box at {:?}", task_box.rect);
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

/// The boxes of one list level have the same x at a fractional scale, where egui rounds `[x]` and `[ ]` to
/// different pixels.
#[test]
fn checked_and_unchecked_boxes_have_the_same_x_at_a_fractional_scale() {
  let (mut boxes, _) = painted(LISTS, 1.5);
  assert_eq!(boxes.len(), 6, "one box for each item");
  boxes.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
  let x: Vec<f32> = boxes.iter().map(|b| b.rect.min.x).collect();
  // The items are: two top level items, two nested items, and two ordered items.
  for (a, b) in [(0, 1), (2, 3), (4, 5), (0, 4)] {
    assert_eq!(x[a], x[b], "the x of box {a} and of box {b} in {x:?}");
  }
}

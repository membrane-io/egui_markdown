//! The label allocates the background of each code block, so the background stays inside the response rect.

use egui::{pos2, vec2, Color32, Context, Id, RawInput, Rect, Shape, UiBuilder};
use egui_markdown::MarkdownLabel;

const CODE: &str = "```rust\nfn main() {}\n```";

/// Add the rect of each shape in `shape` that has the fill `fill` to `out`.
fn rects_with_fill(shape: &Shape, fill: Color32, out: &mut Vec<Rect>) {
  match shape {
    Shape::Vec(shapes) => shapes.iter().for_each(|shape| rects_with_fill(shape, fill, out)),
    Shape::Rect(rect) if rect.fill == fill => out.push(rect.rect),
    _ => {}
  }
}

/// Run two frames of the label. Return its response rect and the rect of each code block background.
fn backgrounds(md: &str, scroll: bool, hug: bool) -> (Rect, Vec<Rect>) {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 400.0));
  let mut result = None;
  for _ in 0..2 {
    let mut label_rect = Rect::NOTHING;
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      let max_rect = Rect::from_min_size(pos2(20.0, 20.0), vec2(260.0, 360.0));
      let mut child = ui.new_child(UiBuilder::new().max_rect(max_rect));
      let label = MarkdownLabel::new(Id::NULL.with("md"), md).scroll_code_blocks(scroll).hug_content(hug);
      label_rect = label.show(&mut child).response.rect;
    });
    let fill = ctx.global_style().visuals.code.bg_color;
    let mut rects = Vec::new();
    for clipped in &output.shapes {
      rects_with_fill(&clipped.shape, fill, &mut rects);
    }
    output.textures_delta.clear();
    result = Some((label_rect, rects));
  }
  result.unwrap()
}

/// A code block at the start or at the end of a label, of a blockquote or of a list item, in both render paths.
fn cases() -> [(&'static str, String); 5] {
  [
    ("a code block only", CODE.to_owned()),
    ("a code block at the end", format!("Text before.\n\n{CODE}")),
    ("a code block at the start", format!("{CODE}\n\nText after.")),
    ("a code block at the end of a blockquote", "> A quote.\n>\n> ```rust\n> fn main() {}\n> ```".to_owned()),
    ("a code block at the end of a list item", "- An item.\n\n  ```rust\n  fn main() {}\n  ```".to_owned()),
  ]
}

#[test]
fn the_background_of_a_code_block_is_inside_the_label() {
  for (name, md) in &cases() {
    for scroll in [false, true] {
      for hug in [false, true] {
        let (label, rects) = backgrounds(md, scroll, hug);
        assert_eq!(rects.len(), 1, "{name}, scroll {scroll}, hug {hug}");
        let bg = rects[0];
        let inside = label.min.y <= bg.min.y && bg.max.y <= label.max.y;
        assert!(inside, "{name}, scroll {scroll}, hug {hug}: label {label:?}, background {bg:?}");
      }
    }
  }
}

/// The padding at the edges of the label does not move a code block that has text above and below it.
#[test]
fn a_code_block_between_paragraphs_keeps_its_place() {
  let (label, rects) = backgrounds(&format!("Text before.\n\n{CODE}\n\nText after."), false, false);
  assert_eq!(rects.len(), 1);
  let bg = rects[0];
  assert!(bg.min.y - label.min.y > 20.0, "label {label:?}, background {bg:?}");
  assert!(label.max.y - bg.max.y > 20.0, "label {label:?}, background {bg:?}");
}

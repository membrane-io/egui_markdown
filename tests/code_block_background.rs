//! The label allocates the background of each code block, so the background stays inside the response rect.

use std::sync::Arc;

use egui::{pos2, vec2, Color32, Context, Galley, Id, Pos2, RawInput, Rect, Shape, TextWrapMode, UiBuilder};
use egui_markdown::{MarkdownLabel, MarkdownStyle, OverflowWrap};

const CODE: &str = "```rust\nfn main() {}\n```";

/// The space that the label gets.
const MAX_RECT: Rect = Rect { min: pos2(20.0, 20.0), max: pos2(280.0, 380.0) };

/// What the label painted.
struct Shown {
  /// The response rect of the label.
  label: Rect,
  /// The rect of each code block background.
  backgrounds: Vec<Rect>,
  /// Each galley and the position where the label painted it.
  galleys: Vec<(Pos2, Arc<Galley>)>,
}

/// Add each shape in `shape` that is a rect with the fill `fill` or a text to `shown`.
fn collect(shape: &Shape, fill: Color32, shown: &mut Shown) {
  match shape {
    Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, fill, shown)),
    Shape::Rect(rect) if rect.fill == fill => shown.backgrounds.push(rect.rect),
    Shape::Text(text) => shown.galleys.push((text.pos, Arc::clone(&text.galley))),
    _ => {}
  }
}

/// Run two frames of the label that `label` makes from a new label, in [`MAX_RECT`].
fn show(md: &str, label: impl Fn(MarkdownLabel<'_>) -> MarkdownLabel<'_>) -> Shown {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 400.0));
  let mut result = None;
  for _ in 0..2 {
    let mut label_rect = Rect::NOTHING;
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(MAX_RECT));
      label_rect = label(MarkdownLabel::new(Id::NULL.with("md"), md)).show(&mut child).response.rect;
    });
    let fill = ctx.global_style().visuals.code.bg_color;
    let mut shown = Shown { label: label_rect, backgrounds: Vec::new(), galleys: Vec::new() };
    for clipped in &output.shapes {
      collect(&clipped.shape, fill, &mut shown);
    }
    output.textures_delta.clear();
    result = Some(shown);
  }
  result.unwrap()
}

/// Return the response rect of the label and the rect of each code block background.
fn backgrounds(md: &str, scroll: bool, hug: bool) -> (Rect, Vec<Rect>) {
  let shown = show(md, |label| label.scroll_code_blocks(scroll).hug_content(hug));
  (shown.label, shown.backgrounds)
}

/// Return the position of the galley that has a row with `text`, and the rect of that row in the galley.
fn row_with(shown: &Shown, text: &str) -> (Pos2, Rect) {
  for (pos, galley) in &shown.galleys {
    for row in &galley.rows {
      if row.glyphs.iter().map(|g| g.chr).collect::<String>().contains(text) {
        return (*pos, row.rect());
      }
    }
  }
  panic!("no row with {text:?}");
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
        assert!(label.contains_rect(bg), "{name}, scroll {scroll}, hug {hug}: label {label:?}, background {bg:?}");
      }
    }
  }
}

/// The padding at the edges of the label does not move a code block that has text above and below it. The label
/// starts at the top of the galley, and the background is the code row grown by the top and the bottom padding.
#[test]
fn a_code_block_between_paragraphs_keeps_its_place() {
  let shown = show(&format!("Text before.\n\n{CODE}\n\nText after."), |label| label);
  let [bg] = shown.backgrounds[..] else { panic!("{:?}", shown.backgrounds) };
  let (galley_pos, row) = row_with(&shown, "fn main");
  let padding = MarkdownStyle::default().code_block.padding;
  assert_eq!(galley_pos.y, shown.label.min.y);
  assert_eq!(bg.min.y - shown.label.min.y, row.min.y - padding[1]);
  assert_eq!(bg.max.y - shown.label.min.y, row.max.y + padding[3]);
}

/// A code line that wraps at any character fills the wrap width. The label that hugs its content stays inside the
/// available width.
#[test]
fn a_label_that_hugs_a_wrapped_code_block_stays_inside_the_available_width() {
  let md = format!("```\n{}\n```", "x".repeat(200));
  let shown = show(&md, |label| label.hug_content(true).overflow_wrap(OverflowWrap::BreakAll));
  assert!(shown.label.max.x <= MAX_RECT.max.x, "label {:?}", shown.label);
}

/// A label that does not wrap is wide enough for the right padding of a code line that is wider than the other
/// text.
#[test]
fn a_label_that_extends_has_the_right_padding_of_a_wide_code_block() {
  let md = format!("Text.\n\n```\n{}\n```", "x ".repeat(100));
  let shown = show(&md, |label| label.wrap_mode(TextWrapMode::Extend));
  let [bg] = shown.backgrounds[..] else { panic!("{:?}", shown.backgrounds) };
  let (galley_pos, row) = row_with(&shown, "x x");
  let padding = MarkdownStyle::default().code_block.padding;
  assert_eq!(bg.max.x - (galley_pos.x + row.max.x), padding[2], "background {bg:?}, row {row:?}");
}

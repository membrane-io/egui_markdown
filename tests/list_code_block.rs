//! A code block in a list item starts below the item text, at the x of the item text (issue #25).

use egui::{pos2, vec2, Context, Id, RawInput, Rect, Shape};
use egui_markdown::{MarkdownLabel, MarkdownStyle};

const IN_LIST: &str = "1. An item with a code block:\n\n   ```\n   let x = 42;\n   ```\n2. A second item\n";
const IN_QUOTE: &str = "> - A list in a quote\n>\n>   ```\n>   let x = 42;\n>   ```\n";

/// A row of text that the label painted, in screen coordinates.
struct PaintedRow {
  text: String,
  rect: Rect,
  /// X of each glyph.
  xs: Vec<f32>,
}

/// What the label paints for `md`: the code block background and every row of text.
fn paint(md: &str, scroll_code_blocks: bool) -> (Vec<Rect>, Vec<PaintedRow>) {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 400.0));
  let mut shapes = Vec::new();
  // The second frame uses the caches of the first.
  for _ in 0..2 {
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      MarkdownLabel::new(Id::NULL.with("list_code"), md).scroll_code_blocks(scroll_code_blocks).show(ui);
    });
    output.textures_delta.clear();
    shapes = output.shapes;
  }
  let code_bg = ctx.global_style().visuals.code.bg_color;
  let (mut backgrounds, mut rows) = (Vec::new(), Vec::new());
  let mut stack: Vec<Shape> = shapes.into_iter().map(|s| s.shape).collect();
  while let Some(shape) = stack.pop() {
    match shape {
      Shape::Vec(inner) => stack.extend(inner),
      Shape::Rect(r) if r.fill == code_bg => backgrounds.push(r.rect),
      Shape::Text(t) => {
        for row in &t.galley.rows {
          rows.push(PaintedRow {
            text: row.glyphs.iter().map(|g| g.chr).collect(),
            rect: row.rect().translate(t.pos.to_vec2()),
            xs: row.glyphs.iter().map(|g| t.pos.x + row.pos.x + g.pos.x).collect(),
          });
        }
      }
      _ => {}
    }
  }
  (backgrounds, rows)
}

/// The code block of `md` lies below the row that starts with `item`, starts no further left than the
/// item text, and ends above the row that starts with `next`, if there is one.
#[track_caller]
fn assert_code_block_in_item(md: &str, item: &str, next: Option<&str>) {
  let left_padding = MarkdownStyle::default().code_block.padding[0];
  for scroll in [false, true] {
    let (backgrounds, rows) = paint(md, scroll);
    let [code] = backgrounds[..] else { panic!("scroll={scroll}: expected one code block, got {backgrounds:?}") };
    let item_row = rows.iter().find(|r| r.text.contains(item)).expect("item row");
    let text_index = item_row.text.find(item).unwrap();
    let text_x = item_row.xs[item_row.text[..text_index].chars().count()];

    assert!(
      code.min.y >= item_row.rect.max.y,
      "scroll={scroll}: the code block (top {}) overlaps the item text (bottom {})",
      code.min.y,
      item_row.rect.max.y
    );
    assert!(
      code.min.x + left_padding >= text_x - 1.0,
      "scroll={scroll}: the code block starts at x {} with padding {left_padding}, left of the item text at x {text_x}",
      code.min.x
    );
    if let Some(next) = next {
      let next_row = rows.iter().find(|r| r.text.contains(next)).expect("next item row");
      assert!(
        code.max.y <= next_row.rect.min.y,
        "scroll={scroll}: the code block (bottom {}) overlaps the next item (top {})",
        code.max.y,
        next_row.rect.min.y
      );
    }
  }
}

#[test]
fn code_block_in_a_list_item() {
  assert_code_block_in_item(IN_LIST, "An item", Some("A second item"));
}

#[test]
fn code_block_in_a_list_in_a_quote() {
  assert_code_block_in_item(IN_QUOTE, "A list in a quote", None);
}

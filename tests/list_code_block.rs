//! A code block in a list item starts below the item text, at the x of the item text (issue #25).

use egui::{pos2, vec2, Context, Id, RawInput, Rect, Shape};
use egui_markdown::{MarkdownLabel, MarkdownStyle};

const IN_LIST: &str = "1. An item with a code block:\n\n   ```\n   let x = 42;\n   ```\n2. A second item\n";
const IN_QUOTE: &str = "> - A list in a quote\n>\n>   ```\n>   let x = 42;\n>   ```\n";
const FIRST_IN_ITEM: &str = "- An item\n- ```\n  let x = 42;\n  let y = 7;\n  ```\n- A second item\n";

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

/// The code block of `md` covers no text but its own, and it and each of its lines start no further left
/// than the text of the row that holds `item`. The code lines are the rows that hold `let `.
#[track_caller]
fn assert_code_block_in_item(md: &str, item: &str) {
  let left_padding = MarkdownStyle::default().code_block.padding[0];
  for scroll in [false, true] {
    let (backgrounds, rows) = paint(md, scroll);
    let [code] = backgrounds[..] else { panic!("scroll={scroll}: expected one code block, got {backgrounds:?}") };
    let item_row = rows.iter().find(|r| r.text.contains(item)).expect("item row");
    let text_index = item_row.text.find(item).unwrap();
    let text_x = item_row.xs[item_row.text[..text_index].chars().count()];

    assert!(
      code.min.x + left_padding >= text_x - 1.0,
      "scroll={scroll}: the code block starts at x {} with padding {left_padding}, left of the item text at x {text_x}",
      code.min.x
    );
    for row in &rows {
      if row.text.contains("let ") {
        assert!(
          row.xs[0] >= text_x - 1.0,
          "scroll={scroll}: the code line {:?} starts at x {}, left of the item text at x {text_x}",
          row.text,
          row.xs[0]
        );
      } else if !row.text.trim().is_empty() {
        assert!(!row.rect.intersects(code), "scroll={scroll}: the code block {code:?} covers {:?}", row.text);
      }
    }
  }
}

#[test]
fn code_block_in_a_list_item() {
  assert_code_block_in_item(IN_LIST, "An item");
}

#[test]
fn code_block_in_a_list_in_a_quote() {
  assert_code_block_in_item(IN_QUOTE, "A list in a quote");
}

#[test]
fn code_block_that_starts_a_list_item() {
  assert_code_block_in_item(FIRST_IN_ITEM, "An item");
}

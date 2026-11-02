//! A nested list starts its marker at the x of the text of the item above it.

use egui::{vec2, Context, Id, RawInput, Rect, UiBuilder};
use egui_markdown::MarkdownLabel;

/// The x of each glyph of each row of `md`, laid out at `width`, with the row's text.
fn rows(md: &str, width: f32) -> Vec<(String, Vec<f32>)> {
  let ctx = Context::default();
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(width, 600.0));
  let mut rows = Vec::new();
  let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    let (_pos, galley, _response) = MarkdownLabel::new(Id::NULL.with("nested"), md).layout_in_ui(&mut child);
    rows = galley
      .rows
      .iter()
      .map(|row| (row.glyphs.iter().map(|g| g.chr).collect(), row.glyphs.iter().map(|g| g.pos.x).collect()))
      .collect();
  });
  output.textures_delta.clear();
  rows
}

#[test]
fn a_nested_marker_starts_at_the_text_of_its_parent() {
  let rows = rows("- parent\n  - child\n    - grandchild\n", 600.0);
  let find = |word: &str| rows.iter().find(|(text, _)| text.contains(word)).expect(word);
  let (parent, child, grandchild) = (find("parent"), find("child"), find("grandchild"));
  // Each row is the marker "• " and then the text, so glyph 0 is the marker and glyph 2 starts the text.
  let parent_text = parent.1[2];
  let child_text = child.1[2];
  assert!(
    child.1[0] >= parent_text - 0.5,
    "the child marker at {} is left of the parent text at {parent_text}",
    child.1[0]
  );
  assert!(grandchild.1[0] >= child_text - 0.5, "the grandchild marker is left of the child text");
  assert!(
    (grandchild.1[2] - child_text - (child_text - parent_text)).abs() < 0.5,
    "each level indents by the same width"
  );
}

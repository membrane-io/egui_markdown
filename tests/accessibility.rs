use egui::{pos2, vec2, Context, Id, RawInput, Rect};
use egui_markdown::MarkdownLabel;

/// The text of the label nodes in the accessibility tree after one frame of `text`.
fn accessible_labels(text: &str) -> Vec<String> {
  let ctx = Context::default();
  ctx.enable_accesskit();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    MarkdownLabel::new(Id::NULL.with("a11y"), text).show(ui);
  });
  output.textures_delta.clear();
  let Some(update) = output.platform_output.accesskit_update else { return Vec::new() };
  update
    .nodes
    .iter()
    .filter(|(_, node)| node.role() == egui::Role::Label)
    .filter_map(|(_, node)| node.value().map(str::to_owned))
    .collect()
}

#[test]
fn prose_is_accessible() {
  let labels = accessible_labels("Some **bold** prose.");
  assert!(labels.iter().any(|label| label.contains("Some bold prose.")), "{labels:?}");
}

#[test]
fn prose_around_a_code_block_is_accessible() {
  let labels = accessible_labels("Before.\n\n```\nlet x = 1;\n```\n\nAfter.");
  for text in ["Before.", "let x = 1;", "After."] {
    assert!(labels.iter().any(|label| label.contains(text)), "{text:?} missing from {labels:?}");
  }
}

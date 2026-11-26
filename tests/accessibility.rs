use egui::{pos2, vec2, Context, Id, RawInput, Rect};
use egui_markdown::MarkdownLabel;

/// The text of the label nodes in the accessibility tree after one frame of `text`.
fn accessible_labels(text: &str) -> Vec<String> {
  accessible_names(text, egui::Role::Label, |node| node.value())
}

/// The labels of the image nodes in the accessibility tree after one frame of `text`.
#[cfg(feature = "images")]
fn accessible_images(text: &str) -> Vec<String> {
  accessible_names(text, egui::Role::Image, |node| node.label())
}

fn accessible_names(
  text: &str,
  role: egui::Role,
  name: impl Fn(&egui::accesskit::Node) -> Option<&str>,
) -> Vec<String> {
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
    .filter(|(_, node)| node.role() == role)
    .filter_map(|(_, node)| name(node).map(str::to_owned))
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

/// A screen reader reads the alt text of an image, and an image that fails to load shows it.
#[cfg(feature = "images")]
#[test]
fn an_image_has_its_alt_text() {
  for text in ["![A logo](does-not-exist.png)", "| a |\n|---|\n| ![A logo](does-not-exist.png) |"] {
    assert_eq!(accessible_images(text), ["A logo"], "{text:?}");
  }
}

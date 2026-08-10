use egui::{vec2, Color32, Context, FontId, Id, RawInput, Rect, UiBuilder};
use egui_markdown::{layout, MarkdownLabel, MarkdownStyle};

/// Render `text` into `ctx` and return every string painted as text.
fn painted_text(ctx: &Context, text: &str) -> Vec<String> {
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(500.0, 2000.0));
  let output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    MarkdownLabel::new(Id::new("test"), text).show(&mut child);
  });

  let mut out = Vec::new();
  for clipped in &output.shapes {
    collect(&clipped.shape, &mut out);
  }
  out
}

fn collect(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
  match shape {
    egui::epaint::Shape::Text(t) => out.push(t.galley.text().to_owned()),
    egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
    _ => {}
  }
}

/// Editing a word must be reflected even when the edit does not change the token count.
#[test]
fn edit_within_segmented_doc_is_reflected() {
  // A table forces the segmented render path, which caches each flush range separately.
  let doc = |word: &str| format!("Intro {word} paragraph.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nOutro.");

  let ctx = Context::default();
  let before = painted_text(&ctx, &doc("alpha"));
  assert!(before.iter().any(|t| t.contains("alpha")), "first render missing 'alpha': {before:?}");

  let after = painted_text(&ctx, &doc("bravo"));
  assert!(after.iter().any(|t| t.contains("bravo")), "edited text not re-rendered: {after:?}");
  assert!(!after.iter().any(|t| t.contains("alpha")), "stale galley still painted: {after:?}");
}

/// Same, for an edit after the block element (a later flush range).
#[test]
fn edit_after_block_is_reflected() {
  let doc = |word: &str| format!("Intro.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nOutro {word} here.");

  let ctx = Context::default();
  let before = painted_text(&ctx, &doc("alpha"));
  assert!(before.iter().any(|t| t.contains("alpha")), "first render missing 'alpha': {before:?}");

  let after = painted_text(&ctx, &doc("bravo"));
  assert!(after.iter().any(|t| t.contains("bravo")), "edited text not re-rendered: {after:?}");
  assert!(!after.iter().any(|t| t.contains("alpha")), "stale galley still painted: {after:?}");
}

/// `needs_segmentation` decides the render path without laying out; it must agree with the
/// segment breaks `build_layout` would have produced.
#[test]
fn needs_segmentation_matches_build_layout() {
  let docs = [
    "Just **plain** text with `code` and a [link](https://example.com).",
    "# Heading\n\nParagraph.\n\n---\n\nAfter the rule.",
    "- one\n- two\n  - nested\n\n1. first\n2. second",
    "Intro.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nOutro.",
    "> quoted **text**\n>\n> > nested",
    "Text\n\n```rust\nfn main() {}\n```\n\nMore.",
    "![alt](https://example.com/x.png)",
    "A footnote[^1].\n\n[^1]: body.",
    "- [x] done\n- [ ] todo",
  ];

  let ctx = Context::default();
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(500.0, 2000.0));
  let style = MarkdownStyle::default();

  ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    for doc in docs {
      for scroll_code_blocks in [false, true] {
        let md = egui_markdown::parse(doc);
        let predicted = layout::needs_segmentation(&md.tokens, scroll_code_blocks, None);
        let built = layout::build_layout(
          ui,
          &md.tokens,
          FontId::proportional(14.0),
          Color32::WHITE,
          None,
          None,
          scroll_code_blocks,
          &style,
          Default::default(),
        );
        assert_eq!(
          predicted,
          !built.segment_breaks.is_empty(),
          "scroll_code_blocks={scroll_code_blocks} disagreement on:\n{doc}"
        );
      }
    }
  });
}

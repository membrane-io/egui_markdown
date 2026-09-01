use egui::{vec2, Color32, Context, FontId, Id, RawInput, Rect, UiBuilder};
use egui_markdown::{layout, MarkdownLabel, MarkdownStyle};

/// Render `text` into `ctx` and return every string painted as text.
fn painted_text(ctx: &Context, text: &str) -> Vec<String> {
  painted_text_with(ctx, text, false)
}

fn painted_text_with(ctx: &Context, text: &str, scroll_code_blocks: bool) -> Vec<String> {
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(500.0, 2000.0));
  let output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    MarkdownLabel::new(Id::new("test"), text).scroll_code_blocks(scroll_code_blocks).show(&mut child);
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

/// Editing the body of a scrolling fence must invalidate the code-block galley cache.
#[test]
fn edit_within_scrolling_code_block_is_reflected() {
  let doc = |word: &str| format!("Intro.\n\n```rust\nfn main() {{\n    let x = \"{word}\";\n}}\n```\n\nOutro.");

  let ctx = Context::default();
  let before = painted_text_with(&ctx, &doc("alpha"), true);
  assert!(before.iter().any(|t| t.contains("alpha")), "first render missing 'alpha': {before:?}");

  let after = painted_text_with(&ctx, &doc("bravo"), true);
  assert!(after.iter().any(|t| t.contains("bravo")), "edited fence not re-rendered: {after:?}");
  assert!(!after.iter().any(|t| t.contains("alpha")), "stale code galley still painted: {after:?}");
}

/// Growing a scrolling fence mid-line then completing the line must paint the new tail.
#[test]
fn streaming_append_scrolling_code_block_is_reflected() {
  let ctx = Context::default();

  let partial = "Intro.\n\n```rust\nfn main() {\n    let x = \"alp\n```\n";
  // heal closes the fence; body is still the incomplete string inside.
  let mid = painted_text_with(&ctx, partial, true);
  assert!(mid.iter().any(|t| t.contains("alp")), "partial stream missing 'alp': {mid:?}");
  assert!(!mid.iter().any(|t| t.contains("alpha")), "partial stream should not yet contain 'alpha': {mid:?}");

  let grown = "Intro.\n\n```rust\nfn main() {\n    let x = \"alpha\";\n}\n```\n";
  let after = painted_text_with(&ctx, grown, true);
  assert!(after.iter().any(|t| t.contains("alpha")), "appended stream missing 'alpha': {after:?}");
}

/// A non-prefix edit of a scrolling fence must rebuild rather than keep a stale galley.
#[test]
fn non_prefix_edit_scrolling_code_block_is_reflected() {
  let ctx = Context::default();

  let first = "```rust\nfn alpha() {}\n```\n";
  let before = painted_text_with(&ctx, first, true);
  assert!(before.iter().any(|t| t.contains("alpha")), "first render missing 'alpha': {before:?}");

  let second = "```rust\nfn bravo() {}\n```\n";
  let after = painted_text_with(&ctx, second, true);
  assert!(after.iter().any(|t| t.contains("bravo")), "non-prefix edit missing 'bravo': {after:?}");
  assert!(!after.iter().any(|t| t.contains("alpha")), "stale fence still painted: {after:?}");
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
          ui.available_width(),
          false,
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

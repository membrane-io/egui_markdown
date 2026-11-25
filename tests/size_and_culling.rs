//! `calculate_size` measures what `show` renders, without painting.

use egui::{vec2, Context, Id, RawInput, Rect, UiBuilder, Vec2};
use egui_markdown::MarkdownLabel;

const WITH_BLOCKS: &str = "Intro paragraph.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n> A quote.\n\nOutro.";

/// Run one frame of `f` in a child Ui over a screen of `size`, and return every string painted as text.
fn frame(ctx: &Context, size: Vec2, mut f: impl FnMut(&mut egui::Ui)) -> Vec<String> {
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), size);
  let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    f(&mut child);
  });
  output.textures_delta.clear();
  let mut painted = Vec::new();
  for clipped in &output.shapes {
    collect(&clipped.shape, &mut painted);
  }
  painted
}

fn collect(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
  match shape {
    egui::epaint::Shape::Text(t) => out.push(t.galley.text().to_owned()),
    egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| collect(s, out)),
    _ => {}
  }
}

#[test]
fn calculate_size_matches_what_show_allocates() {
  let ctx = Context::default();
  let screen = vec2(500.0, 800.0);
  let label = || MarkdownLabel::new(Id::NULL.with("size"), WITH_BLOCKS).hug_content(true);
  let mut measured = Vec2::ZERO;
  let mut shown = Vec2::ZERO;
  for _ in 0..2 {
    frame(&ctx, screen, |ui| {
      measured = label().calculate_size(ui);
      let top = ui.min_rect().bottom();
      label().show(ui);
      shown = vec2(ui.min_rect().width(), ui.min_rect().bottom() - top);
    });
  }
  assert!((measured.y - shown.y).abs() < 0.5, "measured {measured:?}, shown {shown:?}");

  let mut text_only = Vec2::ZERO;
  frame(&ctx, screen, |ui| {
    text_only = MarkdownLabel::new(Id::NULL.with("text"), "Intro paragraph.").calculate_size(ui)
  });
  assert!(measured.y > text_only.y + 40.0, "the table and the quote must count: {measured:?} vs {text_only:?}");
}

/// A caller sizes a chip or a button to the text, so the measured width is the width of the text, as it
/// was before the measure ran the render of `show`.
#[test]
fn calculate_size_measures_the_width_of_the_text() {
  let ctx = Context::default();
  let mut short = Vec2::ZERO;
  let mut long = Vec2::ZERO;
  for _ in 0..2 {
    frame(&ctx, vec2(500.0, 800.0), |ui| {
      short = MarkdownLabel::new(Id::NULL.with("short"), "Fix it").calculate_size(ui);
      long = MarkdownLabel::new(Id::NULL.with("long"), "Fix it, and then fix the next thing").calculate_size(ui);
    });
  }
  assert!(short.x > 0.0 && short.x < 100.0, "a short label measured {short:?}");
  assert!(long.x > short.x && long.x < 500.0, "a longer label measured {long:?}");
}

#[test]
fn calculate_size_paints_nothing() {
  let ctx = Context::default();
  let painted = frame(&ctx, vec2(500.0, 800.0), |ui| {
    MarkdownLabel::new(Id::NULL.with("size"), WITH_BLOCKS).calculate_size(ui);
  });
  assert!(painted.is_empty(), "painted {painted:?}");
}

/// A caller can measure several labels under one id in a frame, as a row of chips that each measure
/// their text does. The measures must not share widget ids, which egui reports as a clash.
#[test]
fn two_measures_under_one_id_do_not_clash() {
  let ctx = Context::default();
  let mut painted = Vec::new();
  for _ in 0..2 {
    painted = frame(&ctx, vec2(500.0, 800.0), |ui| {
      MarkdownLabel::new(Id::NULL, WITH_BLOCKS).calculate_size(ui);
      MarkdownLabel::new(Id::NULL, "| x | y |\n|---|---|\n| a much longer cell | 2 |").calculate_size(ui);
    });
  }
  assert!(!painted.iter().any(|t| t.contains("🔥")), "egui reported an id clash: {painted:?}");
}

/// The bottom of a frame with a margin around the label, after one frame that is a sizing pass or a normal pass.
fn frame_bottom(ctx: &Context, md: &str, sizing_pass: bool) -> f32 {
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(300.0, 600.0));
  let mut bottom = 0.0;
  let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut builder = UiBuilder::new().max_rect(screen);
    if sizing_pass {
      builder = builder.sizing_pass();
    }
    let mut child = ui.new_child(builder);
    let framed = egui::Frame::new().inner_margin(8).show(&mut child, |ui| {
      MarkdownLabel::new(Id::NULL.with(md), md).show(ui);
    });
    bottom = framed.response.rect.bottom();
  });
  output.textures_delta.clear();
  bottom
}

#[test]
fn a_sizing_pass_measures_the_height_of_a_table() {
  let one_table = "Text before\n\n| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n";
  let two_tables = "| a | b |\n|---|---|\n| 1 | 2 |\n\nText between\n\n| c | d |\n|---|---|\n| 3 | 4 |\n";
  for md in [one_table, two_tables] {
    let ctx = Context::default();
    let sizing = frame_bottom(&ctx, md, true);
    let normal = frame_bottom(&ctx, md, false);
    assert_eq!(sizing, normal, "{md:?}");
    assert_eq!(frame_bottom(&ctx, md, true), normal, "a sizing pass after a normal pass: {md:?}");
  }
}

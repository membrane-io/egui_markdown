//! Soft-wrapped rows keep the indentation of the line they continue.
//!
//! Requires the `membrane` feature (egui's `LeadingSpace::Indent`).
#![cfg(feature = "membrane")]

use egui::{vec2, Context, Id, RawInput, Rect, UiBuilder};
use egui_markdown::{MarkdownLabel, MarkdownStyle};

struct Row {
  text: String,
  /// X position of each glyph in the row.
  xs: Vec<f32>,
}

impl Row {
  /// X of the first glyph that isn't whitespace.
  fn text_start(&self) -> f32 {
    self.text.chars().zip(&self.xs).find(|(c, _)| !c.is_whitespace()).map_or(0.0, |(_, x)| *x)
  }
}

/// Lay the markdown out at `width` with the default style.
fn rows(md: &str, width: f32) -> Vec<Row> {
  rows_styled(md, width, &MarkdownStyle::default())
}

/// Lay the markdown out at `width` and return one entry per rendered row.
fn rows_styled(md: &str, width: f32, style: &MarkdownStyle) -> Vec<Row> {
  let ctx = Context::default();
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(width, 600.0));
  let mut rows = Vec::new();
  let _ = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    let (_pos, galley, _response) = MarkdownLabel::new(Id::new("indent"), md).style(style).layout_in_ui(&mut child);
    rows = galley
      .rows
      .iter()
      .map(|row| Row {
        text: row.glyphs.iter().map(|g| g.chr).collect(),
        xs: row.glyphs.iter().map(|g| g.pos.x).collect(),
      })
      .collect();
  });
  rows
}

#[test]
fn code_block_wrapped_rows_keep_line_indentation() {
  let md = "```rust\nfn main() {\n    let x = some_function_with_a_long_name(argument_one, argument_two);\n}\n```\n";
  let rows = rows(md, 200.0);

  let unindented = rows.iter().find(|r| r.text.contains("fn main")).expect("first code line");
  assert!(unindented.text_start() < 10.0, "an unindented line should start at the left edge");

  let start = rows.iter().position(|r| r.text.contains("let x")).expect("indented code line");
  let indent = rows[start].text_start();
  assert!(indent > 10.0, "the indented line should start past the left edge, got {indent}");
  assert!(rows.len() > start + 1, "the long line should have wrapped");

  for row in &rows[start + 1..] {
    if row.text.contains('}') {
      break; // back out to the closing brace line
    }
    let x = row.text_start();
    assert!((x - indent).abs() < 1.0, "wrapped row {:?} starts at {x}, expected the line indent {indent}", row.text);
  }
}

/// Wrapped rows must start exactly where the item's text starts on the first row — the marker's
/// own width, not a guess, so `- ` and `1. ` each line up with their own item.
#[track_caller]
fn assert_list_wraps_under_its_text(md: &str, marker: &str) {
  let rows = rows(md, 200.0);
  assert!(rows.len() > 1, "{md:?}: the item should have wrapped");

  let first = &rows[0];
  assert!(first.text.starts_with(marker), "{md:?}: first row {:?} should start with {marker:?}", first.text);
  let text_x = first.xs[marker.chars().count()];
  assert!(first.xs[0] >= 0.0, "{md:?}: the marker must not be pushed outside the widget");

  for row in &rows[1..] {
    let x = row.text_start();
    assert!((x - text_x).abs() < 1.0, "{md:?}: wrapped row {:?} starts at {x}, expected {text_x}", row.text);
  }
}

#[test]
fn unordered_list_wrapped_rows_align_with_item_text() {
  assert_list_wraps_under_its_text("- a fairly long list item that will certainly wrap around a few times\n", "• ");
}

#[test]
fn ordered_list_wrapped_rows_align_with_item_text() {
  assert_list_wraps_under_its_text("1. a fairly long list item that will certainly wrap around a few times\n", "1. ");
}

/// The marker is right-aligned in a fixed slot, so items of the same level start their text at the
/// same x whether the marker is a bullet, a one-digit number, or a two-digit one.
#[test]
fn markers_are_right_aligned_in_a_shared_slot() {
  let text_x = |md: &str, marker_chars: usize| {
    let rows = rows(md, 240.0);
    (rows[0].xs[0], rows[0].xs[marker_chars])
  };

  let (bullet_x, bullet_text) = text_x("- alpha\n", 2);
  let (one_x, one_text) = text_x("1. alpha\n", 3);
  let (ninetynine_x, ninetynine_text) = text_x("99. alpha\n", 4);

  assert!((bullet_text - one_text).abs() < 1.0, "bullet text at {bullet_text}, `1.` text at {one_text}");
  assert!((bullet_text - ninetynine_text).abs() < 1.0, "bullet text at {bullet_text}, `99.` text at {ninetynine_text}");
  assert!(bullet_x > one_x, "the narrower bullet should be pushed further right than `1. `");
  assert!(one_x > ninetynine_x, "`1. ` should be pushed further right than `99. `");
  assert!(ninetynine_x >= 0.0, "the widest marker must still start inside the widget");
}

#[test]
fn nested_list_items_are_indented_on_every_row() {
  let md =
    "- top level item that is long enough to wrap around here\n  - nested item that is also long enough to wrap\n";
  let rows = rows(md, 200.0);

  let top = rows.iter().position(|r| r.text.contains("top level")).expect("top level item");
  let nested = rows.iter().position(|r| r.text.contains("nested item")).expect("nested item");

  assert!(rows[nested].text_start() > rows[top].text_start(), "the nested item's first row should be indented");
  let nested_text_x = rows[nested].xs[2]; // past the "• " marker
  for row in &rows[nested + 1..] {
    let x = row.text_start();
    assert!((x - nested_text_x).abs() < 1.0, "wrapped row {:?} starts at {x}, expected {nested_text_x}", row.text);
  }
}

/// `gap` pushes the text away from the marker column on every row of the item; `bullet_nudge` and
/// `number_nudge` move only the marker, so both kinds keep starting their text at the same x.
#[test]
fn list_style_separates_marker_from_text_without_moving_it() {
  let md = "- alpha bullet item that is long enough to wrap around here somewhere\n1. one\n";
  let plain = rows(md, 240.0);
  let plain_text_x = plain[0].xs[2];

  let mut style = MarkdownStyle::default();
  style.list.gap = 6.0;
  let gapped = rows_styled(md, 240.0, &style);

  assert!(gapped[0].xs[0] == plain[0].xs[0], "the gap must not move the marker");
  let text_x = gapped[0].xs[2];
  assert!(
    (text_x - plain_text_x - 6.0).abs() < 0.5,
    "the gap should move the text by 6, got {}",
    text_x - plain_text_x
  );
  assert!((gapped[1].text_start() - text_x).abs() < 1.0, "wrapped rows must follow the gap");

  style.list.bullet_nudge = 4.0;
  style.list.number_nudge = 2.0;
  let nudged = rows_styled(md, 240.0, &style);
  let bullet = &nudged[0];
  let number = nudged.iter().find(|r| r.text.starts_with("1. ")).expect("ordered item");

  assert!((bullet.xs[0] - (gapped[0].xs[0] - 4.0)).abs() < 0.5, "the bullet should move 4 to the left");
  assert!((bullet.xs[2] - text_x).abs() < 0.5, "nudging the bullet must not move its text");
  assert!((number.xs[3] - text_x).abs() < 0.5, "bullet and number text must share a column");
}

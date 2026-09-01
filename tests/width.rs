use egui::{vec2, Context, Id, RawInput, Rect, UiBuilder};
use egui_markdown::{MarkdownLabel, OverflowWrap};

const MARKDOWN: &str = "Some intro text.\n\n```rust\nfn main() {}\n```\n\nTrailing text.";

/// Render the label into a `width`-wide ui and return (allocated rect, painted shape bounds).
fn render(scroll_code_blocks: bool, width: f32) -> (Rect, Rect) {
  let ctx = Context::default();
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(width, 600.0));
  let mut allocated = Rect::NOTHING;
  let output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    MarkdownLabel::new(Id::new("test"), MARKDOWN).scroll_code_blocks(scroll_code_blocks).show(&mut child);
    allocated = child.min_rect();
  });

  let mut painted = Rect::NOTHING;
  for clipped in &output.shapes {
    let bounds = clipped.shape.visual_bounding_rect();
    if bounds.is_finite() && bounds.is_positive() {
      painted = painted.union(bounds.intersect(clipped.clip_rect));
    }
  }
  (allocated, painted)
}

#[test]
fn allocates_full_width_regardless_of_code_block_scrolling() {
  let width = 400.0;
  for scroll in [false, true] {
    let (allocated, _) = render(scroll, width);
    assert!(
      (allocated.width() - width).abs() < 1.0,
      "scroll_code_blocks={scroll}: allocated {} but {width} was available",
      allocated.width()
    );
  }
}

#[test]
fn paints_inside_allocated_rect() {
  let width = 400.0;
  for scroll in [false, true] {
    let (allocated, painted) = render(scroll, width);
    assert!(
      painted.max.x <= allocated.max.x + 0.5,
      "scroll_code_blocks={scroll}: painted right edge {} exceeds allocated {}",
      painted.max.x,
      allocated.max.x
    );
  }
}

const UNBROKEN: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn unbroken_layout(overflow: OverflowWrap, width: f32) -> (f32, f32) {
  let ctx = Context::default();
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(width, 600.0));
  let mut allocated_w = 0.0_f32;
  let mut painted_right = 0.0_f32;
  let output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    MarkdownLabel::new(Id::new("unbroken"), UNBROKEN).overflow_wrap(overflow).show(&mut child);
    allocated_w = child.min_rect().width();
  });
  for clipped in &output.shapes {
    let bounds = clipped.shape.visual_bounding_rect();
    if bounds.is_finite() && bounds.is_positive() {
      painted_right = painted_right.max(bounds.intersect(clipped.clip_rect).max.x);
    }
  }
  (allocated_w, painted_right)
}

#[test]
fn unbroken_string_breaks_with_break_all() {
  let width = 120.0;
  let (allocated_w, painted_right) = unbroken_layout(OverflowWrap::BreakAll, width);
  assert!(allocated_w <= width + 1.0, "BreakAll should allocate within width, got {allocated_w} for {width}");
  assert!(
    painted_right <= width + 1.0,
    "BreakAll should paint within width, got right edge {painted_right} for {width}"
  );
}

#[test]
fn unbroken_string_overruns_with_normal() {
  let width = 120.0;
  let (allocated_w, _) = unbroken_layout(OverflowWrap::Normal, width);
  assert!(allocated_w > width + 1.0, "Normal should overrun on an unbroken string, got {allocated_w} for {width}");
}

const SENTENCE: &str = "hello world this is a fairly long sentence that should wrap at spaces";

/// The whitespace-separated tokens of each laid-out row, in order.
fn sentence_words(overflow: OverflowWrap, width: f32) -> Vec<String> {
  let ctx = Context::default();
  let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(width, 600.0));
  let mut words = Vec::new();
  let _ = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    let (_pos, galley, _) =
      MarkdownLabel::new(Id::new("sentence"), SENTENCE).overflow_wrap(overflow).layout_in_ui(&mut child);
    assert!(galley.rows.len() > 1, "sentence should not fit on one {width}px row");
    words =
      galley.rows.iter().flat_map(|r| r.row.text().split_whitespace().map(str::to_owned).collect::<Vec<_>>()).collect();
  });
  words
}

/// The default must leave words whole: the Gaze call site only opts into `BreakAll` for
/// messages with no spaces, so ordinary prose has to keep wrapping at word boundaries.
#[test]
fn spaced_sentence_keeps_words_intact_by_default() {
  let words = sentence_words(OverflowWrap::Normal, 120.0);
  let expected: Vec<String> = SENTENCE.split_whitespace().map(str::to_owned).collect();
  assert_eq!(words, expected, "Normal split a word across rows");
}

#[test]
fn break_all_splits_words_in_a_spaced_sentence() {
  let words = sentence_words(OverflowWrap::BreakAll, 120.0);
  let expected: Vec<String> = SENTENCE.split_whitespace().map(str::to_owned).collect();
  assert_ne!(words, expected, "BreakAll should fill rows past word boundaries");
}

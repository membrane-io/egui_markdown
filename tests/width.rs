use egui::{vec2, Context, Id, RawInput, Rect, UiBuilder};
use egui_markdown::MarkdownLabel;

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

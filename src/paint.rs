//! The painting of code block backgrounds, horizontal rules and task list boxes.

use std::sync::Arc;

use egui::epaint::{pos2, vec2, Galley};
use egui::{Pos2, Rect, Shape, Stroke, StrokeKind, Ui, Vec2};

use crate::style::{CodeBlockStyle, HorizontalRuleStyle};

/// The background rect of each code block, from the character spans of the blocks in `galley`.
///
/// A rect covers the rows of the block, from the x of its first glyph to the right edge of its widest row.
/// [`crate::MarkdownLabel`] keeps its left edge, moves its right edge to the right edge of the label, and grows
/// it by the top and bottom padding of the style.
pub fn compute_code_block_rects(code_blocks: &[(usize, usize)], galley: &Arc<Galley>) -> Vec<Rect> {
  let mut code_block_rects = Vec::new();
  for &(start, end) in code_blocks {
    debug_assert!(start <= end);
    let mut start_row = None;
    let mut right = 0.0_f32;
    let mut offset = 0;
    for (row_index, row) in galley.rows.iter().enumerate() {
      let row_range = offset..=offset + row.char_count_including_newline().0;
      if row_range.contains(&start) {
        start_row = Some(row_index);
        right = 0.0;
      }
      if let Some(start_row) = start_row {
        right = right.max(row.rect().max.x);
        if row_range.contains(&end) || row_index == galley.rows.len() - 1 {
          let first = &galley.rows[start_row];
          let mut bg_rect = Rect::from_min_max(first.rect().min, row.rect().max);
          // The padding space that starts the block sits at its left edge, which is past 0 in a list item.
          if let Some(glyph) = first.glyphs.first() {
            bg_rect.min.x = first.pos.x + glyph.pos.x;
          }
          bg_rect.max.x = right;
          code_block_rects.push(bg_rect);
          break;
        }
      }
      offset += row.char_count_including_newline().0;
    }
  }
  code_block_rects
}

/// Paint the background and the border of a code block in `r`.
///
/// The fill and corner radius come from [`egui::Visuals::code`], as for the frame that
/// [`crate::MarkdownLabel`] gives a scrolling code block, so both render paths show one code surface.
pub(crate) fn paint_code_block_rect(ui: &Ui, r: Rect, code_style: &CodeBlockStyle) {
  let code = &ui.visuals().code;
  ui.painter().rect_filled(r, code.corner_radius, code.bg_color);
  let stroke = Stroke::new(code_style.stroke_width, ui.visuals().widgets.noninteractive.bg_stroke.color);
  ui.painter().rect_stroke(r, code.corner_radius, stroke, StrokeKind::Inside);
}

/// Paint a horizontal rule across `available_width` at the row of each character index in `hr_positions`.
pub fn paint_horizontal_rules(
  ui: &Ui,
  hr_positions: &[usize],
  galley: &Arc<Galley>,
  origin: Pos2,
  available_width: f32,
  hr_style: &HorizontalRuleStyle,
) {
  if hr_positions.is_empty() {
    return;
  }
  let stroke = Stroke::new(hr_style.stroke_width, ui.visuals().widgets.noninteractive.bg_stroke.color);
  for &hr_char in hr_positions {
    let mut offset = 0;
    for row in &galley.rows {
      let row_end = offset + row.char_count_including_newline().0;
      if hr_char >= offset && hr_char < row_end {
        let y = origin.y + (row.min_y() + row.max_y()) / 2.0;
        ui.painter().line_segment([pos2(origin.x, y), pos2(origin.x + available_width, y)], stroke);
        break;
      }
      offset = row_end;
    }
  }
}

/// Paint the task list box of each `(char_index, checked, scale)` in `task_boxes` over its placeholder in
/// `galley`, with `origin` as the position of the galley.
///
/// The box has the look of a [`egui::Checkbox`] that is not interactive, at `scale` times its size. The
/// function puts the box in the slot that ends at the ` ` after the placeholder, not on the glyphs of the
/// placeholder. egui rounds the x of each glyph to a pixel, and `[x]` and `[ ]` round differently. The label
/// does not paint a box whose placeholder or ` ` wraps onto a second row.
pub fn paint_task_boxes(ui: &Ui, task_boxes: &[(usize, bool, f32)], galley: &Galley, origin: Pos2) {
  if task_boxes.is_empty() {
    return;
  }
  let spacing = ui.spacing();
  let visuals = &ui.visuals().widgets.noninteractive;
  let mut boxes = task_boxes.iter().peekable();
  let mut row_start = 0;
  for row in &galley.rows {
    let row_chars = row_start..row_start + row.glyphs.len();
    row_start += row.char_count_including_newline().0;
    while let Some(&&(start, checked, scale)) = boxes.peek() {
      // The placeholder `[x]` or `[ ]` has three characters, and a ` ` follows it.
      let last = start + 3;
      if last >= row_chars.end && start < row_start {
        // The placeholder wraps, or it is on a row that holds no glyph for it.
        boxes.next();
        continue;
      }
      if start >= row_chars.end {
        break;
      }
      boxes.next();
      // The layout makes the slot as wide as the box, and the ` ` starts where the slot ends.
      let side = spacing.icon_width * scale;
      let slot_end = row.glyphs[last - row_chars.start].pos.x;
      let center = origin + vec2(slot_end - side / 2.0, (row.min_y() + row.max_y()) / 2.0);
      paint_task_box(ui, center, checked, scale, spacing, visuals);
    }
  }
}

/// Paint one task list box at `center`, as [`egui::Checkbox`] paints its box, at `scale` times its size.
fn paint_task_box(
  ui: &Ui,
  center: Pos2,
  checked: bool,
  scale: f32,
  spacing: &egui::style::Spacing,
  visuals: &egui::style::WidgetVisuals,
) {
  let side = spacing.icon_width * scale;
  let big = Rect::from_center_size(center, Vec2::splat(side));
  // The box is small, so egui caps the rounding to keep it from looking like a radio button.
  let corner_radius = visuals.corner_radius.at_most((0.3 * side).round() as u8);
  let painter = ui.painter();
  // The fill covers the placeholder, which a text selection paints in the selection text color.
  painter.rect(big, corner_radius, visuals.bg_fill, visuals.bg_stroke, StrokeKind::Inside);
  if checked {
    let small = Rect::from_center_size(center, Vec2::splat(spacing.icon_width_inner * scale));
    // At the body size, the check mark has the width of the egui check mark. The width increases with the font
    // size, so that the mark is not thin in large text.
    let stroke = Stroke::new(visuals.fg_stroke.width * scale, visuals.fg_stroke.color);
    let points = vec![pos2(small.left(), small.center().y), pos2(small.center().x, small.bottom()), small.right_top()];
    painter.add(Shape::line(points, stroke));
  }
}

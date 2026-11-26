//! Code block background and horizontal rule painting.

use std::sync::Arc;

use egui::{Pos2, Rect, Stroke, StrokeKind, Ui};
use epaint::{pos2, Galley};

use crate::style::{CodeBlockStyle, HorizontalRuleStyle};

/// Compute the background rectangles for code blocks from character offset spans.
///
/// A rect covers the rows of the block, from the x of its first glyph to the right edge of its widest row.
/// [`paint_code_block_bg`] grows it by the padding of the style. [`crate::MarkdownLabel`] keeps its left edge,
/// because the layout puts the left padding inside the rows, and moves its right edge to the right edge of the
/// label.
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

/// Paint code block background and border.
///
/// The fill and corner radius come from [`egui::Visuals::code`], as for the frame that
/// [`crate::MarkdownLabel`] gives a scrolling code block, so both render paths show one code surface.
pub fn paint_code_block_bg(ui: &Ui, bg_rect: Rect, origin: Pos2, code_style: &CodeBlockStyle) {
  let mut r = bg_rect.translate(origin.to_vec2());
  r.min.x -= code_style.padding[0];
  r.min.y -= code_style.padding[1];
  r.max.x += code_style.padding[2];
  r.max.y += code_style.padding[3];
  paint_code_block_rect(ui, r, code_style);
}

/// Paint the background and the border of a code block in `r`.
pub(crate) fn paint_code_block_rect(ui: &Ui, r: Rect, code_style: &CodeBlockStyle) {
  let code = &ui.visuals().code;
  ui.painter().rect_filled(r, code.corner_radius, code.bg_color);
  let stroke = Stroke::new(code_style.stroke_width, ui.visuals().widgets.noninteractive.bg_stroke.color);
  ui.painter().rect_stroke(r, code.corner_radius, stroke, StrokeKind::Inside);
}

/// Paint horizontal rules at the given character positions.
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

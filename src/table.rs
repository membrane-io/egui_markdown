//! Table rendering for markdown tables.

use egui::text::LayoutJob;
use egui::{
  Align, Color32, CornerRadius, FontFamily, FontId, Id, Layout, Margin, Rect, ScrollArea, Stroke, StrokeKind,
  TextFormat, Ui, UiBuilder, Vec2,
};
use egui_extras::{Column, TableBuilder};

use crate::layout::{append_link_to_job, render_link_in_ui};
use crate::link::LinkHandler;
use crate::style::{InlineCodeStyle, TableStyle};
use crate::types::{Alignment, TableData, Token, TokenStyle};

/// Render a markdown table using `egui_extras::TableBuilder`.
#[allow(clippy::too_many_arguments)]
pub fn render_table(
  ui: &mut Ui,
  id: Id,
  data: &TableData<'_>,
  font_id: &FontId,
  color: Color32,
  inline_code_style: &InlineCodeStyle,
  table_style: &TableStyle,
  link_handler: Option<&dyn LinkHandler>,
) {
  let num_cols = data.alignments.len();
  if num_cols == 0 {
    return;
  }

  let hyperlink_color = ui.visuals().hyperlink_color;
  let strong_color = ui.visuals().strong_text_color();
  let bold_family = FontFamily::Name("bold".into());
  let has_bold = ui.ctx().fonts(|f| f.families().contains(&bold_family));

  let pad = table_style.cell_padding;
  let cell_style = CellStyle {
    font_id,
    color,
    hyperlink_color,
    strong_color,
    has_bold,
    inline_code_style,
    link_handler,
    max_image_width: (ui.available_width() - pad[0] - pad[2]).max(0.0),
  };
  let measured = measure_table(ui, data, &cell_style, pad);

  let stroke = (table_style.stroke_width > 0.0)
    .then(|| Stroke::new(table_style.stroke_width, ui.visuals().widgets.noninteractive.bg_stroke.color));
  let last_body = data.rows.len().saturating_sub(1);
  let cell_margin = egui::Margin { left: pad[0] as i8, top: pad[1] as i8, right: pad[2] as i8, bottom: pad[3] as i8 };
  let col_widths = measured.col_widths;
  let header_h = measured.header_h;
  let row_heights = measured.row_heights;
  let table_size = Vec2::new(col_widths.iter().sum(), header_h + row_heights.iter().sum::<f32>());
  let radius = table_style.corner_radius.round() as u8;

  // The scroll bar of a table wider than the space floats over the bottom row, inside the border of the table. egui
  // puts a horizontal bar at the bottom of the scroll area and gives `scroll_bar_rect` only its x range, so that area
  // has no bottom margin, and the bar stops before the rounded corners. A table that fits has no bar, and keeps the
  // bottom margin of 4 pt.
  let scrolls = table_size.x > ui.available_width();
  let visible = Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width().min(table_size.x), table_size.y));
  let corner_inset = f32::from(radius) + stroke.map_or(0.0, |s| s.width);
  let output = ScrollArea::horizontal()
    .id_salt(id.with("scroll"))
    .content_margin(Margin { top: 0, right: 0, bottom: if scrolls { 0 } else { 4 }, left: 0 })
    .scroll_bar_rect(visible.shrink2(Vec2::new(corner_inset, 0.0)))
    .show(ui, |ui| {
      let paint_table = |ui: &mut Ui| {
        if stroke.is_some() {
          ui.spacing_mut().item_spacing = Vec2::ZERO;
        }
        let mut builder = TableBuilder::new(ui)
          .id_salt(id)
          .striped(false)
          .vscroll(false)
          .min_scrolled_height(0.0)
          .cell_layout(Layout::left_to_right(Align::Center));
        for &w in &col_widths {
          builder = builder.column(Column::exact(w));
        }
        builder
          .header(header_h, |mut header| {
            for (col_idx, cell_tokens) in data.headers.iter().enumerate() {
              let align = data.alignments.get(col_idx).copied().unwrap_or(Alignment::None);
              header.col(|ui| {
                padded_cell(ui, cell_margin, |ui| render_cell(ui, cell_tokens, &cell_style, true, align));
                if let Some(stroke) = stroke {
                  paint_cell_separators(ui, ui.max_rect(), stroke, col_idx + 1 == num_cols, data.rows.is_empty());
                }
              });
            }
          })
          .body(|body| {
            body.heterogeneous_rows(row_heights.iter().copied(), |mut row| {
              let row_idx = row.index();
              let row_data = &data.rows[row_idx];
              for (col_idx, cell_tokens) in row_data.iter().enumerate() {
                let align = data.alignments.get(col_idx).copied().unwrap_or(Alignment::None);
                row.col(|ui| {
                  padded_cell(ui, cell_margin, |ui| render_cell(ui, cell_tokens, &cell_style, false, align));
                  if let Some(stroke) = stroke {
                    paint_cell_separators(ui, ui.max_rect(), stroke, col_idx + 1 == num_cols, row_idx == last_body);
                  }
                });
              }
            });
          });
      };

      let top_left = ui.cursor().min;
      paint_table(ui);
      Rect::from_min_size(top_left, table_size)
    });

  paint_table_chrome(ui, output.inner, output.inner_rect, stroke, radius);
}

fn padded_cell(ui: &mut Ui, margin: Margin, add: impl FnOnce(&mut Ui)) {
  let inner = ui.max_rect() - margin;
  ui.scope_builder(UiBuilder::new().max_rect(inner), add);
}

/// Outer stroke on the visible table, plus a light inner shadow on cut edges.
/// `ScrollAreaOutput::inner_rect` is the pre-shrink available rect, so height
/// comes from the table and width from the viewport. The visible rect is
/// always rounded; a cut edge is the same rounded path, just shorter.
fn paint_table_chrome(ui: &Ui, table_rect: Rect, viewport: Rect, stroke: Option<Stroke>, radius: u8) {
  let mut vis = table_rect.intersect(ui.clip_rect());
  vis.min.x = vis.min.x.max(viewport.min.x);
  vis.max.x = vis.max.x.min(viewport.max.x);
  if !vis.is_positive() {
    return;
  }

  let slop = stroke.map(|s| s.width.max(1.0)).unwrap_or(1.0);
  let cut_left = vis.left() > table_rect.left() + slop;
  let cut_right = vis.right() < table_rect.right() - slop;
  let cut_top = vis.top() > table_rect.top() + slop;
  let cut_bottom = vis.bottom() < table_rect.bottom() - slop;

  let painter = ui.painter_at(vis.expand(1.0));
  if let Some(stroke) = stroke {
    painter.rect_stroke(vis, CornerRadius::same(radius), stroke, StrokeKind::Inside);
  }

  if !(cut_left || cut_right || cut_top || cut_bottom) {
    return;
  }

  let dark = ui.visuals().dark_mode;
  if cut_right {
    paint_edge_shadow(&painter, vis, OverflowSide::Right, dark);
  }
  if cut_left {
    paint_edge_shadow(&painter, vis, OverflowSide::Left, dark);
  }
  if cut_top {
    paint_edge_shadow(&painter, vis, OverflowSide::Top, dark);
  }
  if cut_bottom {
    paint_edge_shadow(&painter, vis, OverflowSide::Bottom, dark);
  }
}

enum OverflowSide {
  Left,
  Right,
  Top,
  Bottom,
}

fn paint_edge_shadow(painter: &egui::Painter, vis: Rect, side: OverflowSide, dark: bool) {
  const SIZE: f32 = 6.0;
  let color = Color32::BLACK.gamma_multiply(if dark { 0.18 } else { 0.08 });
  let strip = match side {
    OverflowSide::Right => Rect::from_x_y_ranges((vis.right() - SIZE)..=vis.right(), vis.y_range()),
    OverflowSide::Left => Rect::from_x_y_ranges(vis.left()..=vis.left() + SIZE, vis.y_range()),
    OverflowSide::Top => Rect::from_x_y_ranges(vis.x_range(), vis.top()..=vis.top() + SIZE),
    OverflowSide::Bottom => Rect::from_x_y_ranges(vis.x_range(), (vis.bottom() - SIZE)..=vis.bottom()),
  };
  painter.add(egui::epaint::RectShape::filled(strip, 0.0, color).with_blur_width(SIZE));
}

/// Internal separators only. The outer rectangle is painted by [`paint_table_chrome`].
fn paint_cell_separators(ui: &Ui, rect: Rect, stroke: Stroke, last_col: bool, last_row: bool) {
  // Exact columns clip their cells; pull the lines in so the stroke is not discarded.
  let rect = rect.shrink(stroke.width * 0.5);
  let painter = ui.painter();
  if !last_col {
    painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
  }
  if !last_row {
    painter.line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
  }
}

/// The style of the text in the cells of a table, and the widest image that a cell can show.
struct CellStyle<'a> {
  font_id: &'a FontId,
  color: Color32,
  hyperlink_color: Color32,
  strong_color: Color32,
  has_bold: bool,
  inline_code_style: &'a InlineCodeStyle,
  link_handler: Option<&'a dyn LinkHandler>,
  max_image_width: f32,
}

impl CellStyle<'_> {
  fn job(&self, ui: &Ui, tokens: &[Token<'_>], is_header: bool) -> LayoutJob {
    tokens_to_layout_job(
      ui,
      tokens,
      self.font_id,
      self.color,
      self.hyperlink_color,
      self.strong_color,
      self.has_bold,
      is_header,
      self.inline_code_style,
      self.link_handler,
    )
  }
}

/// A part of a row of a cell: text and links that lay out as one job, or an image.
enum CellPart<'a, 's> {
  Text(&'a [Token<'s>]),
  Image(&'a str),
}

/// A row of a cell, between two line breaks. Each part has its size.
struct CellLine<'a, 's> {
  parts: Vec<(CellPart<'a, 's>, Vec2)>,
  size: Vec2,
}

/// The rows of a cell. A [`Token::Newline`], such as from a `<br>` tag, starts a row. Without the `images`
/// feature, an image is a link in the text.
fn layout_cell<'a, 's>(ui: &Ui, tokens: &'a [Token<'s>], style: &CellStyle, is_header: bool) -> Vec<CellLine<'a, 's>> {
  let lines: Vec<&[Token<'s>]> = tokens.split(Token::is_newline).collect();
  let single_line = lines.len() == 1;
  let text_part = |run: &'a [Token<'s>]| {
    // `style.job` calls the link handler, which can read the context. `fonts_mut` holds a lock of the context, so
    // the job is complete before the call.
    let job = style.job(ui, run, is_header);
    let size = ui.ctx().fonts_mut(|f| f.layout_job(job)).size();
    (CellPart::Text(run), size)
  };
  lines
    .into_iter()
    .map(|line| {
      let mut parts = Vec::new();
      let mut text_start = 0;
      for (i, token) in line.iter().enumerate() {
        let Token::Image { url, width, height, .. } = token else { continue };
        if !cfg!(feature = "images") {
          continue;
        }
        if text_start < i {
          parts.push(text_part(&line[text_start..i]));
        }
        let size = crate::image::image_size(ui, url, *width, *height, style.max_image_width);
        parts.push((CellPart::Image(url), size));
        text_start = i + 1;
      }
      // An empty cell keeps the height of an empty text job.
      if text_start < line.len() || (single_line && line.is_empty()) {
        parts.push(text_part(&line[text_start..]));
      }
      let size = if parts.is_empty() {
        Vec2::new(0.0, ui.ctx().fonts_mut(|f| f.row_height(style.font_id)))
      } else {
        Vec2::new(parts.iter().map(|(_, size)| size.x).sum(), parts.iter().map(|(_, size)| size.y).fold(0.0, f32::max))
      };
      CellLine { parts, size }
    })
    .collect()
}

/// The size of a cell: its widest row, and the sum of the heights of its rows.
fn cell_size(lines: &[CellLine]) -> Vec2 {
  Vec2::new(lines.iter().map(|line| line.size.x).fold(0.0, f32::max), lines.iter().map(|line| line.size.y).sum())
}

/// Paint the rows of a cell, centered in the height of the cell, at the x that `align` gives.
fn render_cell(ui: &mut Ui, tokens: &[Token<'_>], style: &CellStyle, is_header: bool, align: Alignment) {
  let lines = layout_cell(ui, tokens, style, is_header);
  let rect = ui.max_rect();
  let mut y = rect.center().y - cell_size(&lines).y / 2.0;
  for line in &lines {
    let x = match align {
      Alignment::None | Alignment::Left => rect.left(),
      Alignment::Center => rect.center().x - line.size.x / 2.0,
      Alignment::Right => rect.right() - line.size.x,
    };
    let line_rect = Rect::from_min_size(egui::pos2(x, y), line.size);
    ui.scope_builder(UiBuilder::new().max_rect(line_rect).layout(Layout::left_to_right(Align::Center)), |ui| {
      ui.spacing_mut().item_spacing.x = 0.0;
      for (part, size) in &line.parts {
        match part {
          CellPart::Text(run) => render_text_run(ui, run, style, is_header),
          CellPart::Image(url) => {
            ui.add(crate::image::sized_image(url, *size));
          }
        }
      }
    });
    y += line.size.y;
  }
}

fn render_text_run(ui: &mut Ui, tokens: &[Token<'_>], style: &CellStyle, is_header: bool) {
  let has_links = tokens.iter().any(|t| matches!(t, Token::Link { .. } | Token::Image { .. }));
  if !has_links {
    ui.label(style.job(ui, tokens, is_header));
    return;
  }

  // Render tokens individually so links are clickable and dispatch through the LinkHandler. The tokens
  // hold their own spaces, so the labels sit side by side, as in the job that measured the cell.
  let (link_font, link_color) =
    link_font_and_color(style.font_id, style.hyperlink_color, style.strong_color, style.has_bold, is_header);
  for token in tokens {
    match token {
      Token::Link { text, href, .. } => {
        let base_format = TextFormat::default();
        render_link_in_ui(ui, text.as_ref(), href.as_ref(), &link_font, &base_format, link_color, style.link_handler);
      }
      Token::Image { alt, url, .. } => {
        let base_format = TextFormat::default();
        render_link_in_ui(ui, image_link_text(alt, url), url.as_ref(), &link_font, &base_format, link_color, None);
      }
      Token::Text { text, style: text_style } => {
        let format = token_style_to_format(
          text_style,
          style.font_id,
          style.color,
          style.strong_color,
          style.has_bold,
          is_header,
          ui.visuals(),
          style.inline_code_style,
        );
        let mut job = LayoutJob::default();
        job.append(text, 0.0, format);
        ui.label(job);
      }
      _ => {}
    }
  }
}

/// The text of the link that stands for an image without the `images` feature: the alt text, or the URL
/// when the image has no alt text.
fn image_link_text<'a>(alt: &'a str, url: &'a str) -> &'a str {
  if alt.is_empty() {
    url
  } else {
    alt
  }
}

/// Return the font and color a link should use in a table cell, accounting for
/// header bold styling and missing bold-font fallback.
#[inline]
fn link_font_and_color(
  font_id: &FontId,
  hyperlink_color: Color32,
  strong_color: Color32,
  has_bold: bool,
  is_header: bool,
) -> (FontId, Color32) {
  let mut font = font_id.clone();
  let mut color = hyperlink_color;
  if is_header {
    if has_bold {
      font.family = FontFamily::Name("bold".into());
    } else {
      color = strong_color;
    }
  }
  (font, color)
}

struct Measured {
  col_widths: Vec<f32>,
  header_h: f32,
  row_heights: Vec<f32>,
}

fn measure_table(ui: &Ui, data: &TableData<'_>, style: &CellStyle, pad: [f32; 4]) -> Measured {
  let num_cols = data.alignments.len();
  let mut widths = vec![40.0_f32; num_cols];
  let hpad = pad[0] + pad[2];
  let vpad = pad[1] + pad[3];

  let measure = |tokens: &[Token<'_>], is_header: bool| cell_size(&layout_cell(ui, tokens, style, is_header));

  let mut header_h = 0.0_f32;
  for (col, cell) in data.headers.iter().enumerate() {
    let size = measure(cell, true);
    widths[col] = widths[col].max(size.x + hpad);
    header_h = header_h.max(size.y);
  }

  let mut row_heights = Vec::with_capacity(data.rows.len());
  for row in &data.rows {
    let mut h = 0.0_f32;
    for (col, cell) in row.iter().enumerate() {
      if col < num_cols {
        let size = measure(cell, false);
        widths[col] = widths[col].max(size.x + hpad);
        h = h.max(size.y);
      }
    }
    row_heights.push(h + vpad);
  }

  Measured { col_widths: widths, header_h: header_h + vpad, row_heights }
}

#[allow(clippy::too_many_arguments)]
fn tokens_to_layout_job(
  ui: &Ui,
  tokens: &[Token<'_>],
  font_id: &FontId,
  color: Color32,
  hyperlink_color: Color32,
  strong_color: Color32,
  has_bold: bool,
  is_header: bool,
  inline_code_style: &InlineCodeStyle,
  link_handler: Option<&dyn LinkHandler>,
) -> LayoutJob {
  let mut job = LayoutJob::default();
  for token in tokens {
    match token {
      Token::Text { text, style } => {
        let format = token_style_to_format(
          style,
          font_id,
          color,
          strong_color,
          has_bold,
          is_header,
          ui.visuals(),
          inline_code_style,
        );
        job.append(text, 0.0, format);
      }
      Token::Link { text, href, .. } => {
        let (link_font, link_color) = link_font_and_color(font_id, hyperlink_color, strong_color, has_bold, is_header);
        let base_format = TextFormat::default();
        append_link_to_job(
          ui,
          &mut job,
          text.as_ref(),
          href.as_ref(),
          &link_font,
          &base_format,
          link_color,
          link_handler,
        );
      }
      Token::Image { alt, url, .. } => {
        let (link_font, link_color) = link_font_and_color(font_id, hyperlink_color, strong_color, has_bold, is_header);
        let base_format = TextFormat::default();
        append_link_to_job(ui, &mut job, image_link_text(alt, url), url, &link_font, &base_format, link_color, None);
      }
      _ => {}
    }
  }
  job
}

#[inline]
fn apply_bold_to_format(format: &mut TextFormat, strong_color: Color32, has_bold: bool) {
  if has_bold {
    format.font_id.family = FontFamily::Name("bold".into());
  } else {
    format.color = strong_color;
  }
}

#[allow(clippy::too_many_arguments)]
fn token_style_to_format(
  style: &TokenStyle,
  font_id: &FontId,
  color: Color32,
  strong_color: Color32,
  has_bold: bool,
  is_header: bool,
  visuals: &egui::Visuals,
  inline_code_style: &InlineCodeStyle,
) -> TextFormat {
  let mut format = TextFormat { font_id: font_id.clone(), color, ..Default::default() };
  if style.bold || is_header {
    apply_bold_to_format(&mut format, strong_color, has_bold);
  }
  if style.italic {
    format.italics = true;
  }
  if style.strikethrough {
    format.strikethrough = egui::Stroke::new(1.0_f32, color);
  }
  if style.inline_code {
    crate::layout::apply_inline_code_bg(&mut format, visuals, inline_code_style);
  }
  format
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::TokenStyle;
  use epaint::text::ByteRangeExt as _;
  use pulldown_cmark::CowStr;

  /// Inline code in a cell takes the margin and stroke of [`egui::Visuals::code`], so its background
  /// stays off the next word.
  #[test]
  fn inline_code_in_a_cell_keeps_its_background_off_the_next_word() {
    let code = TokenStyle { inline_code: true, ..Default::default() };
    let tokens = [
      Token::Text { text: CowStr::Borrowed("Behind the "), style: TokenStyle::default() },
      Token::Text { text: CowStr::Borrowed("images"), style: code },
      Token::Text { text: CowStr::Borrowed(" feature"), style: TokenStyle::default() },
    ];
    let mut checked = false;
    egui::__run_test_ui(|ui| {
      let font = FontId::proportional(14.0);
      let color = ui.visuals().text_color();
      let style = InlineCodeStyle::default();
      let job = tokens_to_layout_job(ui, &tokens, &font, color, color, color, false, false, &style, None);
      assert_eq!(job.text, "Behind the images feature");
      let code_section = job.sections.iter().find(|s| &job.text[s.byte_range.as_usize()] == "images");
      assert_eq!(code_section.map(|s| s.format.background_margin), Some(ui.visuals().code.margin));
      assert_eq!(code_section.map(|s| s.format.background_stroke), Some(ui.visuals().code.stroke));
      checked = true;
    });
    assert!(checked);
  }
}

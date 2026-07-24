//! Table rendering for markdown tables.

use egui::text::LayoutJob;
use egui::{Align, Color32, FontFamily, FontId, Id, Margin, ScrollArea, TextFormat, Ui};
use egui_extras::{Column, TableBuilder};

use crate::layout::{append_link_to_job, render_link_in_ui};
use crate::link::LinkHandler;
use crate::style::InlineCodeStyle;
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
  link_handler: Option<&dyn LinkHandler>,
) {
  let num_cols = data.alignments.len();
  if num_cols == 0 {
    return;
  }

  let hyperlink_color = ui.visuals().hyperlink_color;
  let strong_color = ui.visuals().strong_text_color();
  let dark_mode = ui.visuals().dark_mode;
  let bold_family = FontFamily::Name("bold".into());
  let has_bold = ui.ctx().fonts(|f| f.families().contains(&bold_family));

  let padding = 16.0;
  let col_widths = measure_column_widths(
    ui,
    data,
    font_id,
    color,
    hyperlink_color,
    strong_color,
    has_bold,
    dark_mode,
    inline_code_style,
    link_handler,
    padding,
  );

  ScrollArea::horizontal()
    .id_salt(id.with("scroll"))
    .content_margin(Margin { top: 0, right: 0, bottom: 4, left: 0 })
    .show(ui, |ui| {
      let mut builder = TableBuilder::new(ui).id_salt(id).striped(false).vscroll(false);
      for &w in &col_widths {
        builder = builder.column(Column::exact(w));
      }
      builder
        .header(20.0, |mut header| {
          for (col_idx, cell_tokens) in data.headers.iter().enumerate() {
            let align = col_alignment(data.alignments.get(col_idx).copied().unwrap_or(Alignment::None));
            header.col(|ui| {
              ui.with_layout(egui::Layout::left_to_right(align), |ui| {
                render_cell(
                  ui,
                  cell_tokens,
                  font_id,
                  color,
                  hyperlink_color,
                  strong_color,
                  has_bold,
                  true,
                  dark_mode,
                  inline_code_style,
                  link_handler,
                );
              });
            });
          }
        })
        .body(|body| {
          body.rows(18.0, data.rows.len(), |mut row| {
            let row_idx = row.index();
            let row_data = &data.rows[row_idx];
            for (col_idx, cell_tokens) in row_data.iter().enumerate() {
              let align = col_alignment(data.alignments.get(col_idx).copied().unwrap_or(Alignment::None));
              row.col(|ui| {
                ui.with_layout(egui::Layout::left_to_right(align), |ui| {
                  render_cell(
                    ui,
                    cell_tokens,
                    font_id,
                    color,
                    hyperlink_color,
                    strong_color,
                    has_bold,
                    false,
                    dark_mode,
                    inline_code_style,
                    link_handler,
                  );
                });
              });
            }
          });
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn render_cell(
  ui: &mut Ui,
  tokens: &[Token<'_>],
  font_id: &FontId,
  color: Color32,
  hyperlink_color: Color32,
  strong_color: Color32,
  has_bold: bool,
  is_header: bool,
  dark_mode: bool,
  inline_code_style: &InlineCodeStyle,
  link_handler: Option<&dyn LinkHandler>,
) {
  let has_links = tokens.iter().any(|t| matches!(t, Token::Link { .. }));
  if !has_links {
    let job = tokens_to_layout_job(
      ui,
      tokens,
      font_id,
      color,
      hyperlink_color,
      strong_color,
      has_bold,
      is_header,
      dark_mode,
      inline_code_style,
      link_handler,
    );
    ui.label(job);
    return;
  }

  // Render tokens individually so links are clickable and dispatch through the LinkHandler.
  for token in tokens {
    match token {
      Token::Link { text, href, .. } => {
        let (link_font, link_color) = link_font_and_color(font_id, hyperlink_color, strong_color, has_bold, is_header);
        let base_format = TextFormat::default();
        render_link_in_ui(ui, text.as_ref(), href.as_ref(), &link_font, &base_format, link_color, link_handler);
      }
      Token::Text { text, style } => {
        let format =
          token_style_to_format(style, font_id, color, strong_color, has_bold, is_header, dark_mode, inline_code_style);
        let job = LayoutJob::simple_singleline(text.to_string(), format.font_id, format.color);
        ui.label(job);
      }
      _ => {}
    }
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

#[allow(clippy::too_many_arguments)]
fn measure_column_widths(
  ui: &Ui,
  data: &TableData<'_>,
  font_id: &FontId,
  color: Color32,
  hyperlink_color: Color32,
  strong_color: Color32,
  has_bold: bool,
  dark_mode: bool,
  inline_code_style: &InlineCodeStyle,
  link_handler: Option<&dyn LinkHandler>,
  padding: f32,
) -> Vec<f32> {
  let num_cols = data.alignments.len();
  let mut widths = vec![40.0_f32; num_cols];

  let measure = |tokens: &[Token<'_>], is_header: bool| -> f32 {
    let job = tokens_to_layout_job(
      ui,
      tokens,
      font_id,
      color,
      hyperlink_color,
      strong_color,
      has_bold,
      is_header,
      dark_mode,
      inline_code_style,
      link_handler,
    );
    ui.ctx().fonts_mut(|f| f.layout_job(job)).size().x
  };

  for (col, cell) in data.headers.iter().enumerate() {
    widths[col] = widths[col].max(measure(cell, true) + padding);
  }
  for row in &data.rows {
    for (col, cell) in row.iter().enumerate() {
      if col < num_cols {
        widths[col] = widths[col].max(measure(cell, false) + padding);
      }
    }
  }
  widths
}

#[inline]
fn col_alignment(align: Alignment) -> Align {
  match align {
    Alignment::None | Alignment::Left => Align::Min,
    Alignment::Center => Align::Center,
    Alignment::Right => Align::Max,
  }
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
  dark_mode: bool,
  inline_code_style: &InlineCodeStyle,
  link_handler: Option<&dyn LinkHandler>,
) -> LayoutJob {
  let mut job = LayoutJob::default();
  for token in tokens {
    match token {
      Token::Text { text, style } => {
        let format =
          token_style_to_format(style, font_id, color, strong_color, has_bold, is_header, dark_mode, inline_code_style);
        job.append(text.as_ref(), 0.0, format);
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
      Token::Newline => {
        job.append(" ", 0.0, TextFormat { font_id: font_id.clone(), color, ..Default::default() });
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
  dark_mode: bool,
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
    format.strikethrough = egui::Stroke::new(1.0, color);
  }
  if style.inline_code {
    crate::layout::apply_inline_code_bg(&mut format, dark_mode, inline_code_style);
  }
  format
}

//! Layout job construction from parsed markdown token streams.

use egui::{text::LayoutJob, Align, Color32, FontFamily, FontId, Stroke, TextFormat, TextWrapMode, Ui};

use crate::link::LinkHandler;
use crate::style::{InlineCodeStyle, MarkdownStyle};
use crate::types::Token;

/// The result of building an egui [`LayoutJob`] from parsed markdown tokens.
#[derive(Clone)]
pub struct LayoutResult {
  /// The constructed layout job ready for galley creation.
  pub job: LayoutJob,
  /// Maps each layout section index to the originating token index.
  pub section_to_token: Vec<usize>,
  /// Byte-offset spans `(start_char, end_char)` of code blocks in the layout text.
  pub code_block_spans: Vec<(usize, usize)>,
  /// `(source_text, language)` pairs for each code block.
  pub code_block_info: Vec<(String, String)>,
  /// Token indices that require separate rendering (tables, images, blockquotes).
  pub segment_breaks: Vec<usize>,
  /// Character positions of horizontal rules (the middle space char).
  pub hr_positions: Vec<usize>,
  /// Current blockquote nesting depth at the end of layout.
  pub blockquote_depth: u32,
  /// Inline widget spans: `(start_char, end_char, token_index)`.
  /// Each entry marks a transparent placeholder region that a [`LinkHandler`]
  /// will paint over after galley positioning.
  pub inline_widget_spans: Vec<(usize, usize, usize)>,
}

/// Look up the section index for a given character offset by walking LayoutJob sections.
/// Replaces the per-frame `char_to_section: Vec<u32>` allocation.
#[inline]
pub fn section_for_char(job: &LayoutJob, char_index: u32) -> Option<u32> {
  let mut offset = 0u32;
  for (section_idx, section) in job.sections.iter().enumerate() {
    let section_text = &job.text[section.byte_range.clone()];
    let char_count = section_text.chars().count() as u32;
    if char_index < offset + char_count {
      return Some(section_idx as u32);
    }
    offset += char_count;
  }
  None
}

/// Apply inline code background and text color to a [`TextFormat`].
#[inline]
pub fn apply_inline_code_bg(format: &mut TextFormat, dark_mode: bool, inline_style: &InlineCodeStyle) {
  format.color = inline_style.color(dark_mode);
  format.background = inline_style.background(dark_mode);
  #[cfg(feature = "membrane")]
  {
    format.expand_bg = epaint::Vec2::new(inline_style.expand_bg, inline_style.expand_bg_y);
    format.bg_corner_radius = inline_style.bg_corner_radius;
  }
  #[cfg(not(feature = "membrane"))]
  {
    #[allow(clippy::useless_conversion)]
    { format.expand_bg = inline_style.expand_bg.into(); }
  }
}

#[inline]
fn text_format(font_id: FontId, color: Color32) -> TextFormat {
  TextFormat { font_id, color, valign: Align::BOTTOM, ..Default::default() }
}

/// Check if a "bold" font family is registered with egui.
fn has_bold_font(ui: &Ui) -> bool {
  let bold_family = FontFamily::Name("bold".into());
  ui.ctx().fonts(|f| f.families().contains(&bold_family))
}

/// Apply bold styling: use bold font family if registered, otherwise fall back to strong text color.
#[inline]
fn apply_bold(format: &mut TextFormat, ui: &Ui, has_bold: bool) {
  if has_bold {
    format.font_id.family = FontFamily::Name("bold".into());
  } else {
    format.color = ui.visuals().strong_text_color();
  }
}

/// Build an egui [`LayoutJob`] from a slice of markdown tokens.
///
/// Converts tokens into styled text sections suitable for galley layout.
/// Returns segment breaks for tokens that need separate rendering (tables, images, blockquotes).
#[allow(clippy::too_many_arguments)]
pub fn build_layout(
  ui: &mut Ui,
  tokens: &[Token<'_>],
  font_id: FontId,
  color: Color32,
  max_rows: Option<u32>,
  link_handler: Option<&dyn LinkHandler>,
  scroll_code_blocks: bool,
  style: &MarkdownStyle,
  code_theme: CodeThemeArg<'_>,
) -> LayoutResult {
  let code_font_size = style.code_font_size;
  let style_ref = style;
  let hyperlink_color = ui.visuals().hyperlink_color;
  let bold_available = has_bold_font(ui);

  let mut job = LayoutJob::default();
  job.wrap.max_width = if ui.wrap_mode() == TextWrapMode::Extend { f32::INFINITY } else { ui.available_width() };

  let mut section_to_token: Vec<usize> = Vec::new();
  let mut code_block_spans: Vec<(usize, usize)> = Vec::new();
  let mut code_block_info: Vec<(String, String)> = Vec::new();
  let mut segment_breaks: Vec<usize> = Vec::new();
  let mut blockquote_depth: u32 = 0;
  let mut hr_positions: Vec<usize> = Vec::new();
  let mut inline_widget_spans: Vec<(usize, usize, usize)> = Vec::new();

  // Pre-build common formats to avoid repeated font_id.clone().
  let base_format = text_format(font_id.clone(), color);
  let transparent_format = TextFormat { color: Color32::TRANSPARENT, ..base_format.clone() };

  // Disable egui's paragraph-splitting optimization (see egui #5411).
  if let Some(n) = max_rows {
    job.wrap.max_rows = n as usize;
  } else {
    job.wrap.max_rows = usize::MAX - 1;
  }

  for (token_index, token) in tokens.iter().enumerate() {
    match token {
      Token::Newline => {
        job.append("\n", 0.0, base_format.clone());
        section_to_token.push(token_index);
      }
      Token::Text { text, style } => {
        if text.is_empty() {
          continue;
        }
        let mut format = base_format.clone();

        if style.bold {
          apply_bold(&mut format, ui, bold_available);
        }
        if style.italic {
          format.italics = true;
        }
        if style.strikethrough {
          format.strikethrough = Stroke::new(1.0, color);
        }
        if style.inline_code {
          apply_inline_code_bg(&mut format, ui.visuals().dark_mode, &style_ref.inline_code);
          job.append(" ", 0.0, transparent_format.clone());
          section_to_token.push(token_index);
          job.append(text.as_ref(), 0.0, format);
          section_to_token.push(token_index);
          job.append(" ", 0.0, transparent_format.clone());
          section_to_token.push(token_index);
          continue;
        }
        if let Some(level) = style.heading {
          format.color = ui.visuals().strong_text_color();
          apply_bold(&mut format, ui, bold_available);
          let idx = (level as usize).saturating_sub(1).min(5);
          format.font_id.size *= style_ref.heading.scales[idx];
        }

        job.append(text.as_ref(), 0.0, format);
        section_to_token.push(token_index);
      }
      Token::CodeBlock { text, language } => {
        if scroll_code_blocks {
          segment_breaks.push(token_index);
        } else {
          let lang = language.as_deref().unwrap_or(style_ref.default_code_language.as_str());
          let mut padded_text = String::with_capacity(text.len() + text.lines().count());
          for (i, line) in text.lines().enumerate() {
            if i > 0 {
              padded_text.push('\n');
            }
            padded_text.push(' ');
            padded_text.push_str(line);
          }

          let highlighted_job = highlight_code(ui, &padded_text, lang, code_font_size, code_theme);

          let start_char = job.text.chars().count();
          for section in highlighted_job.sections {
            let section_text = &highlighted_job.text[section.byte_range.clone()];
            job.append(section_text, 0.0, section.format);
            section_to_token.push(token_index);
          }
          let end_char = job.text.chars().count();
          code_block_spans.push((start_char, end_char));
          code_block_info.push((text.to_string(), lang.to_string()));
        }
      }
      Token::ListMarker { marker, indent_level } => {
        let indent_width = ui.ctx().fonts_mut(|f| f.glyph_width(&font_id, ' ')) * 2.0;
        let total_indent =
          (*indent_level as f32 + 1.0) * indent_width + blockquote_depth as f32 * style_ref.blockquote.indent_per_depth;

        #[cfg(feature = "membrane")]
        {
          // Use LeadingSpace::Indent so wrapped lines stay indented past the bullet.
          job.push_with_leading_space("", epaint::text::LeadingSpace::Indent(total_indent), base_format.clone());
          section_to_token.push(token_index);
        }
        #[cfg(not(feature = "membrane"))]
        {
          // Upstream egui only supports first-row leading space; wrapped lines return to column 0.
          let indent_str = "  ".repeat(*indent_level);
          job.append(&indent_str, total_indent, base_format.clone());
          section_to_token.push(token_index);
        }

        job.append(marker.as_ref(), 0.0, base_format.clone());
        section_to_token.push(token_index);
      }
      Token::Link { text, href, .. } => {
        if link_handler.is_some_and(|h| h.is_block_widget(href)) {
          segment_breaks.push(token_index);
        } else if let Some(widget_size) = link_handler.and_then(|h| h.inline_widget_size(href, &font_id)) {
          // Inline widget: reserve transparent placeholder space.
          let start_char = job.text.chars().count();

          // Try layout_link first for custom placeholder width.
          let handler = link_handler.unwrap();
          let before = job.sections.len();
          let handled = handler.layout_link(text, href, &mut job, &font_id, Color32::TRANSPARENT);
          if handled {
            // Stamp line_height and force transparent on all added sections.
            let added = job.sections.len() - before;
            for i in (job.sections.len() - added)..job.sections.len() {
              job.sections[i].format.color = Color32::TRANSPARENT;
              job.sections[i].format.line_height = Some(widget_size.y);
            }
            for _ in 0..added {
              section_to_token.push(token_index);
            }
          } else {
            // Fallback: emit link text as transparent placeholder.
            let mut format =
              TextFormat { color: Color32::TRANSPARENT, line_height: Some(widget_size.y), ..base_format.clone() };
            // Use monospace font to get predictable width matching the widget.
            format.font_id = FontId::monospace(font_id.size);
            job.append(text.as_ref(), 0.0, format);
            section_to_token.push(token_index);
          }

          let end_char = job.text.chars().count();
          inline_widget_spans.push((start_char, end_char, token_index));
        } else if link_handler.is_some_and(|h| {
          let before = job.sections.len();
          let handled = h.layout_link(text, href, &mut job, &font_id, hyperlink_color);
          if handled {
            let added = job.sections.len() - before;
            for _ in 0..added {
              section_to_token.push(token_index);
            }
          }
          handled
        }) {
          // Already handled inside the closure.
        } else {
          let link_color = if let Some(handler) = link_handler {
            handler.link_style(href).and_then(|s| s.color).unwrap_or(hyperlink_color)
          } else {
            hyperlink_color
          };
          job.append(text.as_ref(), 0.0, text_format(font_id.clone(), link_color));
          section_to_token.push(token_index);
        }
      }
      Token::Image { .. } | Token::Table(_) => {
        segment_breaks.push(token_index);
      }
      Token::BlockquoteStart => {
        segment_breaks.push(token_index);
        blockquote_depth += 1;
      }
      Token::BlockquoteEnd => {
        segment_breaks.push(token_index);
        blockquote_depth = blockquote_depth.saturating_sub(1);
      }
      Token::HorizontalRule => {
        // Insert newline, a transparent space (to hold a row we paint the line on), then newline.
        job.append("\n", 0.0, base_format.clone());
        section_to_token.push(token_index);
        let hr_char_pos = job.text.chars().count();
        job.append(" ", 0.0, transparent_format.clone());
        section_to_token.push(token_index);
        hr_positions.push(hr_char_pos);
        job.append("\n", 0.0, base_format.clone());
        section_to_token.push(token_index);
      }
      Token::TaskListMarker { checked, .. } => {
        let marker_char = if *checked { "☑ " } else { "☐ " };
        job.append(marker_char, 0.0, base_format.clone());
        section_to_token.push(token_index);
      }
      Token::FootnoteRef { label } => {
        let mut format = TextFormat { color: hyperlink_color, valign: Align::TOP, ..base_format.clone() };
        format.font_id.size *= 0.75;
        let ref_text = label.to_string();
        job.append(&ref_text, 0.0, format);
        section_to_token.push(token_index);
      }
      Token::FootnoteDef { label } => {
        let mut format = base_format.clone();
        apply_bold(&mut format, ui, bold_available);
        let def_text = format!("{label}. ");
        job.append(&def_text, 0.0, format);
        section_to_token.push(token_index);
      }
    }
  }

  LayoutResult {
    job,
    section_to_token,
    code_block_spans,
    code_block_info,
    segment_breaks,
    hr_positions,
    blockquote_depth,
    inline_widget_spans,
  }
}

/// Optional custom syntax highlighting theme reference.
///
/// When the `syntax_highlighting` feature is enabled this is
/// `Option<&syntect::highlighting::Theme>`. When disabled it is `Option<&()>`
/// (always `None`).
#[cfg(feature = "syntax_highlighting")]
pub type CodeThemeArg<'a> = Option<&'a syntect::highlighting::Theme>;

/// See [`CodeThemeArg`] - stub type when syntax highlighting is disabled.
#[cfg(not(feature = "syntax_highlighting"))]
pub type CodeThemeArg<'a> = Option<&'a ()>;

/// Produce a syntax-highlighted [`LayoutJob`] for a code block body.
///
/// When `code_theme` is `Some`, that theme is used for highlighting.
/// When `None`, a built-in syntect theme is chosen based on dark/light mode.
#[cfg(feature = "syntax_highlighting")]
pub fn highlight_code(
  ui: &Ui,
  body: &str,
  language: &str,
  code_font_size: f32,
  code_theme: CodeThemeArg<'_>,
) -> LayoutJob {
  use egui::text::{LayoutSection, TextFormat as TF};
  use egui_extras::syntax_highlighting;
  use std::sync::LazyLock;

  static SYNTAX_SET: LazyLock<syntect::parsing::SyntaxSet> =
    LazyLock::new(syntect::parsing::SyntaxSet::load_defaults_newlines);
  static THEME_SET: LazyLock<syntect::highlighting::ThemeSet> =
    LazyLock::new(syntect::highlighting::ThemeSet::load_defaults);

  let style = &*ui.ctx().style();
  let ss = &*SYNTAX_SET;
  let syn_theme = code_theme.unwrap_or_else(|| {
    if style.visuals.dark_mode {
      &THEME_SET.themes["base16-ocean.dark"]
    } else {
      &THEME_SET.themes["base16-ocean.light"]
    }
  });

  let effective_language = match language {
    "typescript" | "ts" | "tsx" => "javascript",
    "jsx" => "javascript",
    other => other,
  };
  let syntax = ss.find_syntax_by_token(effective_language);

  if let Some(syntax) = syntax {
    let mut h = syntect::easy::HighlightLines::new(syntax, syn_theme);
    let mut job = LayoutJob { text: body.into(), ..Default::default() };
    let mut byte_offset = 0;
    for line in syntect::util::LinesWithEndings::from(body) {
      if let Ok(ranges) = h.highlight_line(line, ss) {
        for (syn_style, range) in ranges {
          let byte_start = byte_offset;
          let byte_end = byte_offset + range.len();
          let fg = syn_style.foreground;
          #[allow(clippy::useless_conversion)]
          job.sections.push(LayoutSection {
            leading_space: 0.0.into(),
            byte_range: byte_start..byte_end,
            format: TF {
              font_id: FontId::monospace(code_font_size),
              color: Color32::from_rgb(fg.r, fg.g, fg.b),
              ..Default::default()
            },
          });
          byte_offset = byte_end;
        }
      }
    }
    return job;
  }

  // Fallback: use egui_extras highlight (which may also fall back to plain text).
  let theme = syntax_highlighting::CodeTheme::from_style(style);
  let mut layout_job = syntax_highlighting::highlight(ui.ctx(), style, &theme, body, language);
  for section in &mut layout_job.sections {
    section.format.font_id = FontId::monospace(code_font_size);
  }
  layout_job
}

/// Produce a plain (unhighlighted) [`LayoutJob`] for a code block body.
#[cfg(not(feature = "syntax_highlighting"))]
pub fn highlight_code(
  _ui: &Ui,
  body: &str,
  _language: &str,
  code_font_size: f32,
  _code_theme: CodeThemeArg<'_>,
) -> LayoutJob {
  let mut job = LayoutJob::default();
  job.append(body, 0.0, TextFormat { font_id: FontId::monospace(code_font_size), ..Default::default() });
  job
}

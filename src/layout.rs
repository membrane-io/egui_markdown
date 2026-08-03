//! Layout job construction from parsed markdown token streams.

use egui::{
  text::LayoutJob, Align, Color32, CursorIcon, FontFamily, FontId, OpenUrl, Response, Sense, Stroke, TextFormat,
  TextWrapMode, Ui,
};

use crate::link::LinkHandler;
use crate::style::{InlineCodeStyle, MarkdownStyle};
use crate::types::Token;

/// Override for markdown body/table cell row height (`TextFormat::line_height`).
///
/// `None` uses egui’s default (font metrics).
pub(crate) const MARKDOWN_LINE_HEIGHT_POINTS: Option<f32> = Some(17.0);

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
    {
      format.expand_bg = inline_style.expand_bg.into();
    }
  }
}

#[inline]
fn text_format(font_id: FontId, color: Color32) -> TextFormat {
  TextFormat { font_id, color, valign: Align::BOTTOM, line_height: MARKDOWN_LINE_HEIGHT_POINTS, ..Default::default() }
}

/// Outcome of appending a Link token to a [`LayoutJob`].
#[derive(Default)]
pub struct LinkAppend {
  /// True when the [`LinkHandler`] indicated this link is a block widget.
  /// The caller should render the link separately via [`LinkHandler::block_widget`];
  /// no sections are appended in this case.
  pub is_block_widget: bool,
  /// Inline-widget placeholder span `(start_char, end_char)` in the job's text.
  /// `None` for non-inline-widget links.
  pub inline_widget_span: Option<(usize, usize)>,
  /// Number of sections appended to the [`LayoutJob`] for this link.
  pub sections_added: usize,
}

/// Append a `Link` token to a [`LayoutJob`], dispatching through an optional [`LinkHandler`].
///
/// Encapsulates the priority order: `is_block_widget` → `inline_widget_size` (with optional
/// `layout_link` placeholder) → `layout_link` → `link_style` → default hyperlink color.
///
/// The caller is responsible for mapping the appended sections to their token index
/// (see [`LinkAppend::sections_added`]).
pub fn append_link_to_job(
  ui: &Ui,
  job: &mut LayoutJob,
  text: &str,
  href: &str,
  font_id: &FontId,
  base_format: &TextFormat,
  link_color: Color32,
  link_handler: Option<&dyn LinkHandler>,
) -> LinkAppend {
  if let Some(handler) = link_handler {
    if handler.is_block_widget(href) {
      return LinkAppend { is_block_widget: true, ..LinkAppend::default() };
    }
    if let Some(widget_size) = handler.inline_widget_size(href, font_id) {
      let start_char = job.text.chars().count();
      let before = job.sections.len();
      let added = if handler.layout_link(ui, text, href, job, font_id, Color32::TRANSPARENT) {
        let added = job.sections.len() - before;
        for section in &mut job.sections[before..] {
          // section.format.color = Color32::TRANSPARENT;
          section.format.line_height = Some(widget_size.y);
        }
        added
      } else {
        let format = TextFormat {
          font_id: FontId::monospace(font_id.size),
          color: Color32::TRANSPARENT,
          line_height: Some(widget_size.y),
          ..base_format.clone()
        };
        job.append(text, 0.0, format);
        1
      };
      let end_char = job.text.chars().count();
      return LinkAppend {
        is_block_widget: false,
        inline_widget_span: Some((start_char, end_char)),
        sections_added: added,
      };
    }
    let before = job.sections.len();
    if handler.layout_link(ui, text, href, job, font_id, link_color) {
      return LinkAppend {
        is_block_widget: false,
        inline_widget_span: None,
        sections_added: job.sections.len() - before,
      };
    }
    let color = handler.link_style(href).and_then(|s| s.color).unwrap_or(link_color);
    let format = TextFormat { font_id: font_id.clone(), color, ..base_format.clone() };
    job.append(text, 0.0, format);
    return LinkAppend { is_block_widget: false, inline_widget_span: None, sections_added: 1 };
  }
  let format = TextFormat { font_id: font_id.clone(), color: link_color, ..base_format.clone() };
  job.append(text, 0.0, format);
  LinkAppend { is_block_widget: false, inline_widget_span: None, sections_added: 1 }
}

/// Render a single link inline in a [`Ui`], dispatching through an optional [`LinkHandler`].
///
/// Used by table cells where each link is rendered as its own widget. Handles
/// block widgets, inline widgets (placeholder + `paint_inline_widget`), custom
/// `layout_link` sections, `link_style`, and click delegation. Returns the
/// allocated [`Response`].
pub fn render_link_in_ui(
  ui: &mut Ui,
  text: &str,
  href: &str,
  font_id: &FontId,
  base_format: &TextFormat,
  link_color: Color32,
  link_handler: Option<&dyn LinkHandler>,
) -> Response {
  if let Some(handler) = link_handler {
    if handler.is_block_widget(href) {
      if let Some(resp) = handler.block_widget(ui, text, href) {
        return resp;
      }
    }
  }

  let mut job = LayoutJob::default();
  let info = append_link_to_job(ui, &mut job, text, href, font_id, base_format, link_color, link_handler);

  let galley = ui.fonts_mut(|f| f.layout_job(job));
  let size = galley.size();
  let (rect, response) = ui.allocate_exact_size(size, Sense::click());
  ui.painter().galley(rect.min, galley, link_color);

  let is_inline_widget = info.inline_widget_span.is_some();
  if is_inline_widget {
    if let Some(handler) = link_handler {
      ui.push_id(("link_widget", href), |ui| {
        handler.paint_inline_widget(ui, text, href, rect);
      });
    }
  }

  if response.hovered() {
    ui.output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);
    if !is_inline_widget {
      let underline_color = link_handler.and_then(|h| h.link_style(href).and_then(|s| s.color)).unwrap_or(link_color);
      ui.painter().line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0_f32, underline_color));
    }
  }

  if response.clicked() {
    let handled = link_handler.is_some_and(|h| h.click(text, href, ui));
    if !handled {
      ui.ctx().open_url(OpenUrl::new_tab(href.to_string()));
    }
  }

  response
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
          let mut format = base_format.clone();
          format.line_height = MARKDOWN_LINE_HEIGHT_POINTS.map(|h| h * 4.0);
          job.push_with_leading_space("", epaint::text::LeadingSpace::Indent(total_indent), format);
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
        let link_base = text_format(font_id.clone(), color);
        let info = append_link_to_job(ui, &mut job, text, href, &font_id, &link_base, hyperlink_color, link_handler);
        if info.is_block_widget {
          segment_breaks.push(token_index);
        } else {
          for _ in 0..info.sections_added {
            section_to_token.push(token_index);
          }
          if let Some((start_char, end_char)) = info.inline_widget_span {
            inline_widget_spans.push((start_char, end_char, token_index));
          }
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
        // Newline, short transparent spacer row for the painted rule, then newline.
        job.append("\n", 0.0, base_format.clone());
        section_to_token.push(token_index);
        let hr_char_pos = job.text.chars().count();
        let mut hr_format = transparent_format.clone();
        hr_format.line_height = Some(style_ref.horizontal_rule.height);
        job.append(" ", 0.0, hr_format);
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

  let style = &*ui.style();
  let ss = &*SYNTAX_SET;
  let installed = crate::theme::code_theme(ui.ctx(), style.visuals.dark_mode);
  let syn_theme = code_theme.unwrap_or_else(|| {
    if let Some(ref theme) = installed {
      theme.as_ref()
    } else if style.visuals.dark_mode {
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

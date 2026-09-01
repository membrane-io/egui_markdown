//! Layout job construction from parsed markdown token streams.

use egui::{
  text::LayoutJob, Align, Color32, CursorIcon, FontFamily, FontId, OpenUrl, Response, Sense, Stroke, TextFormat, Ui,
};

use crate::link::LinkHandler;
use crate::style::{InlineCodeStyle, MarkdownStyle};
use crate::types::Token;

/// Override for markdown body/table cell row height (`TextFormat::line_height`).
///
/// `None` uses egui’s default (font metrics).
pub(crate) const MARKDOWN_LINE_HEIGHT_POINTS: Option<f32> = Some(17.0);

/// List markers are right-aligned in a slot as wide as this string, measured in the body font,
/// so bullets and numbers of the same nesting level all start their text at the same x.
const MARKER_SLOT_REFERENCE: &str = "99. ";

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

/// Width of each line's leading whitespace, in points, using the code font.
#[cfg(feature = "membrane")]
fn code_line_indents(ui: &Ui, text: &str, code_font_size: f32) -> Vec<f32> {
  let font_id = FontId::monospace(code_font_size);
  ui.ctx().fonts_mut(|f| {
    text
      .split('\n')
      .map(|line| line.chars().take_while(|c| c.is_whitespace()).map(|c| f.glyph_width(&font_id, c)).sum())
      .collect()
  })
}

/// Whether a token stream contains anything that [`build_layout`] would report as a segment
/// break, i.e. whether it must go through the segmented render path.
///
/// Must stay in sync with every `segment_breaks.push` in [`build_layout`]. It exists so a
/// caller can make that decision without paying for a full layout it would then discard.
pub fn needs_segmentation(
  tokens: &[Token<'_>],
  scroll_code_blocks: bool,
  link_handler: Option<&dyn LinkHandler>,
) -> bool {
  tokens.iter().any(|token| match token {
    Token::CodeBlock { .. } => scroll_code_blocks,
    Token::Link { href, .. } => link_handler.is_some_and(|h| h.is_block_widget(href)),
    Token::Image { .. } | Token::Table(_) | Token::BlockquoteStart | Token::BlockquoteEnd => true,
    _ => false,
  })
}

/// Build an egui [`LayoutJob`] from a slice of markdown tokens.
///
/// Converts tokens into styled text sections suitable for galley layout.
/// Returns segment breaks for tokens that need separate rendering (tables, images, blockquotes).
///
/// `max_width` and `break_anywhere` seed [`LayoutJob::wrap`]. Callers that cache the
/// resulting job should re-apply the live wrap width (and break flag) before shaping,
/// since those values are typically excluded from layout cache keys.
#[allow(clippy::too_many_arguments)]
pub fn build_layout(
  ui: &mut Ui,
  tokens: &[Token<'_>],
  font_id: FontId,
  color: Color32,
  max_rows: Option<u32>,
  max_width: f32,
  break_anywhere: bool,
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
  job.wrap.max_width = max_width;
  job.wrap.break_anywhere = break_anywhere;

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

          #[cfg(feature = "membrane")]
          {
            // Each source line is its own paragraph, so a soft wrap inside a long line would
            // restart at column 0. Indent every line's wrapped rows by its own leading whitespace.
            let line_indents = code_line_indents(ui, &padded_text, code_font_size);
            let mut line_index = 0usize;
            let mut at_line_start = true;
            for section in highlighted_job.sections {
              let section_text = &highlighted_job.text[section.byte_range.clone()];
              for part in section_text.split_inclusive('\n') {
                if at_line_start {
                  let indent = line_indents.get(line_index).copied().unwrap_or(0.0);
                  if indent > 0.0 {
                    job.push_with_leading_space("", epaint::text::LeadingSpace::Indent(indent), section.format.clone());
                    section_to_token.push(token_index);
                  }
                }
                at_line_start = part.ends_with('\n');
                if at_line_start {
                  line_index += 1;
                }
                job.append(part, 0.0, section.format.clone());
                section_to_token.push(token_index);
              }
            }
          }
          #[cfg(not(feature = "membrane"))]
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
        let list = &style_ref.list;
        let is_ordered = marker.trim_start().starts_with(|c: char| c.is_ascii_digit());
        let nudge = if is_ordered { list.number_nudge } else { list.bullet_nudge };
        let mut marker_format = base_format.clone();
        if !is_ordered {
          marker_format.font_id.size *= list.bullet_scale;
        }

        let indent_width = ui.ctx().fonts_mut(|f| f.glyph_width(&font_id, ' ')) * 2.0;
        let (marker_width, slot_width) = ui.ctx().fonts_mut(|f| {
          let marker_width: f32 = marker.chars().map(|c| f.glyph_width(&marker_format.font_id, c)).sum();
          let slot_width: f32 = MARKER_SLOT_REFERENCE.chars().map(|c| f.glyph_width(&font_id, c)).sum();
          (marker_width, slot_width)
        });
        // `indent_level` is 1-based, so a top-level item gets no nesting indent.
        let nesting = (*indent_level as f32 - 1.0).max(0.0) * indent_width
          + blockquote_depth as f32 * style_ref.blockquote.indent_per_depth;
        // Right-align the marker inside a fixed slot so every item at this level starts its text
        // at the same x, whatever its marker. A marker wider than the slot keeps its own width
        // rather than reaching left of the item, which would paint outside the widget.
        let leading = (nesting + slot_width - nudge - marker_width).max(0.0);
        // The nudge and the gap are made up after the marker, so moving a marker never moves text.
        let trailing = nesting + slot_width + list.gap - leading - marker_width;
        let text_start = leading + marker_width + trailing.max(0.0);

        #[cfg(feature = "membrane")]
        {
          let mut format = base_format.clone();
          format.line_height = MARKDOWN_LINE_HEIGHT_POINTS.map(|h| h * 4.0);
          if leading > 0.0 {
            job.push_with_leading_space("", epaint::text::LeadingSpace::FirstRow(leading), format.clone());
            section_to_token.push(token_index);
          }
          // Wrapped rows resume where the item's text starts.
          job.push_with_leading_space("", epaint::text::LeadingSpace::Indent(text_start), format);
          section_to_token.push(token_index);
        }
        #[cfg(not(feature = "membrane"))]
        {
          // Upstream egui only supports first-row leading space; wrapped rows return to column 0.
          job.append("", leading, base_format.clone());
          section_to_token.push(token_index);
        }

        job.append(marker.as_ref(), 0.0, marker_format);
        section_to_token.push(token_index);

        if trailing > 0.0 {
          job.append("", trailing, base_format.clone());
          section_to_token.push(token_index);
        }
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

/// Pad each source line with a leading space (matches scrolling code-block rendering).
pub(crate) fn pad_code_body(text: &str) -> String {
  let mut padded = String::with_capacity(text.len() + text.lines().count());
  for (i, line) in text.lines().enumerate() {
    if i > 0 {
      padded.push('\n');
    }
    padded.push(' ');
    padded.push_str(line);
  }
  padded
}

#[cfg(feature = "syntax_highlighting")]
mod syntect_code {
  use std::sync::{Arc, LazyLock};

  use egui::{text::LayoutJob, Color32, FontId, Ui};
  use epaint::text::Galley;
  use syntect::easy::HighlightLines;
  use syntect::highlighting::{HighlightState, Highlighter, Style};
  use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

  use super::CodeThemeArg;

  fn effective_code_language(language: &str) -> &str {
    match language {
      "typescript" | "ts" | "tsx" => "javascript",
      "jsx" => "javascript",
      other => other,
    }
  }

  fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
    &SYNTAX_SET
  }

  fn theme_set() -> &'static syntect::highlighting::ThemeSet {
    static THEME_SET: LazyLock<syntect::highlighting::ThemeSet> =
      LazyLock::new(syntect::highlighting::ThemeSet::load_defaults);
    &THEME_SET
  }

  fn resolve_syntect_theme<'a>(
    dark_mode: bool,
    code_theme: CodeThemeArg<'a>,
    installed: &'a Option<Arc<syntect::highlighting::Theme>>,
  ) -> &'a syntect::highlighting::Theme {
    if let Some(theme) = code_theme {
      return theme;
    }
    if let Some(theme) = installed.as_ref() {
      return theme.as_ref();
    }
    if dark_mode {
      &theme_set().themes["base16-ocean.dark"]
    } else {
      &theme_set().themes["base16-ocean.light"]
    }
  }

  pub(crate) fn theme_identity_ptr(ui: &Ui, code_theme: CodeThemeArg<'_>) -> usize {
    if let Some(theme) = code_theme {
      return theme as *const _ as usize;
    }
    if let Some(installed) = crate::theme::code_theme(ui.ctx(), ui.visuals().dark_mode) {
      return Arc::as_ptr(&installed) as usize;
    }
    0
  }

  fn append_highlight_ranges_at(
    job: &mut LayoutJob,
    ranges: &[(Style, &str)],
    mut byte_offset: usize,
    code_font_size: f32,
  ) {
    use egui::text::{LayoutSection, TextFormat as TF};

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

  fn append_plain_padded_line(job: &mut LayoutJob, padded_line: &str, code_font_size: f32) {
    use egui::text::{LayoutSection, TextFormat as TF};

    let byte_start = job.text.len();
    job.text.push_str(padded_line);
    #[allow(clippy::useless_conversion)]
    job.sections.push(LayoutSection {
      leading_space: 0.0.into(),
      byte_range: byte_start..job.text.len(),
      format: TF { font_id: FontId::monospace(code_font_size), ..Default::default() },
    });
  }

  fn highlight_padded_line_into(
    theme: &syntect::highlighting::Theme,
    parse_state: &ParseState,
    highlight_state: &HighlightState,
    ss: &SyntaxSet,
    job: &mut LayoutJob,
    padded_line: &str,
    code_font_size: f32,
  ) -> (ParseState, HighlightState) {
    let mut h = HighlightLines::from_state(theme, highlight_state.clone(), parse_state.clone());
    match h.highlight_line(padded_line, ss) {
      Ok(ranges) => {
        let byte_offset = job.text.len();
        job.text.push_str(padded_line);
        append_highlight_ranges_at(job, &ranges, byte_offset, code_font_size);
        let (hs, ps) = h.state();
        (ps, hs)
      }
      Err(_) => {
        append_plain_padded_line(job, padded_line, code_font_size);
        (parse_state.clone(), highlight_state.clone())
      }
    }
  }

  fn pad_unpadded_line_with_ending(unpadded_line_with_opt_nl: &str) -> String {
    if let Some(content) = unpadded_line_with_opt_nl.strip_suffix('\n') {
      let mut s = String::with_capacity(content.len() + 2);
      s.push(' ');
      s.push_str(content);
      s.push('\n');
      s
    } else {
      let mut s = String::with_capacity(unpadded_line_with_opt_nl.len() + 1);
      s.push(' ');
      s.push_str(unpadded_line_with_opt_nl);
      s
    }
  }

  /// Streaming cache for a scrolling fence: syntect state is frozen after complete lines.
  #[derive(Clone)]
  pub(crate) struct StreamingCodeCache {
    pub identity_hash: u64,
    pub source: Arc<str>,
    /// Byte length of the complete-line prefix of `source` covered by state / `frozen_job`.
    pub frozen_len: usize,
    pub parse_state: ParseState,
    pub highlight_state: HighlightState,
    pub frozen_job: Arc<LayoutJob>,
    pub galley: Arc<Galley>,
  }

  /// Produce a syntax-highlighted [`LayoutJob`] for a code block body.
  pub fn highlight_code(
    ui: &Ui,
    body: &str,
    language: &str,
    code_font_size: f32,
    code_theme: CodeThemeArg<'_>,
  ) -> LayoutJob {
    use egui_extras::syntax_highlighting;

    let ss = syntax_set();
    let style = &*ui.style();
    let installed = crate::theme::code_theme(ui.ctx(), style.visuals.dark_mode);
    let syn_theme = resolve_syntect_theme(style.visuals.dark_mode, code_theme, &installed);

    if let Some(syntax) = ss.find_syntax_by_token(effective_code_language(language)) {
      let highlighter = Highlighter::new(syn_theme);
      let mut parse_state = ParseState::new(syntax);
      let mut highlight_state = HighlightState::new(&highlighter, ScopeStack::new());
      let mut job = LayoutJob { text: String::new(), ..Default::default() };
      for line in syntect::util::LinesWithEndings::from(body) {
        let (ps, hs) =
          highlight_padded_line_into(syn_theme, &parse_state, &highlight_state, ss, &mut job, line, code_font_size);
        parse_state = ps;
        highlight_state = hs;
      }
      return job;
    }

    let theme = syntax_highlighting::CodeTheme::from_style(style);
    let mut layout_job = syntax_highlighting::highlight(ui.ctx(), style, &theme, body, language);
    for section in &mut layout_job.sections {
      section.format.font_id = FontId::monospace(code_font_size);
    }
    layout_job
  }

  /// Build or update a scrolling code-block galley.
  pub(crate) fn scrolling_code_galley(
    ui: &mut Ui,
    text: &str,
    language: &str,
    code_font_size: f32,
    identity_hash: u64,
    code_theme: CodeThemeArg<'_>,
    cached: Option<StreamingCodeCache>,
  ) -> StreamingCodeCache {
    if let Some(cached) = &cached {
      if cached.identity_hash == identity_hash && cached.source.as_ref() == text {
        return cached.clone();
      }
    }

    let ss = syntax_set();
    let dark_mode = ui.visuals().dark_mode;
    let installed = crate::theme::code_theme(ui.ctx(), dark_mode);
    let syn_theme = resolve_syntect_theme(dark_mode, code_theme, &installed);

    let Some(syntax) = ss.find_syntax_by_token(effective_code_language(language)) else {
      return rebuild_unknown_language(ui, text, language, code_font_size, identity_hash, code_theme, syn_theme);
    };

    if let Some(mut cached) = cached.filter(|c| {
      c.identity_hash == identity_hash
        && text.len() >= c.frozen_len
        && text.as_bytes()[..c.frozen_len] == c.source.as_bytes()[..c.frozen_len]
    }) {
      let rest = &text[cached.frozen_len..];
      let mut consumed = 0usize;
      let mut frozen_job = Arc::unwrap_or_clone(cached.frozen_job);
      while let Some(rel_nl) = rest[consumed..].find('\n') {
        let end = consumed + rel_nl + 1;
        let padded_line = pad_unpadded_line_with_ending(&rest[consumed..end]);
        let (ps, hs) = highlight_padded_line_into(
          syn_theme,
          &cached.parse_state,
          &cached.highlight_state,
          ss,
          &mut frozen_job,
          &padded_line,
          code_font_size,
        );
        cached.parse_state = ps;
        cached.highlight_state = hs;
        consumed = end;
      }
      cached.frozen_len += consumed;
      cached.frozen_job = Arc::new(frozen_job);

      let mut paint_job = (*cached.frozen_job).clone();
      let incomplete = &text[cached.frozen_len..];
      if !incomplete.is_empty() {
        let padded_tail = pad_unpadded_line_with_ending(incomplete);
        let _ = highlight_padded_line_into(
          syn_theme,
          &cached.parse_state,
          &cached.highlight_state,
          ss,
          &mut paint_job,
          &padded_tail,
          code_font_size,
        );
      }
      paint_job.wrap.max_width = f32::INFINITY;
      cached.galley = ui.fonts_mut(|f| f.layout_job(paint_job));
      cached.source = Arc::from(text);
      return cached;
    }

    rebuild_streaming(ui, text, code_font_size, identity_hash, syn_theme, syntax, ss)
  }

  fn rebuild_unknown_language(
    ui: &mut Ui,
    text: &str,
    language: &str,
    code_font_size: f32,
    identity_hash: u64,
    code_theme: CodeThemeArg<'_>,
    syn_theme: &syntect::highlighting::Theme,
  ) -> StreamingCodeCache {
    let padded = super::pad_code_body(text);
    let mut job = highlight_code(ui, &padded, language, code_font_size, code_theme);
    job.wrap.max_width = f32::INFINITY;
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let highlighter = Highlighter::new(syn_theme);
    StreamingCodeCache {
      identity_hash,
      source: Arc::from(text),
      frozen_len: text.len(),
      parse_state: ParseState::new(syntax_set().find_syntax_plain_text()),
      highlight_state: HighlightState::new(&highlighter, ScopeStack::new()),
      frozen_job: Arc::new(LayoutJob::default()),
      galley,
    }
  }

  fn rebuild_streaming(
    ui: &mut Ui,
    text: &str,
    code_font_size: f32,
    identity_hash: u64,
    syn_theme: &syntect::highlighting::Theme,
    syntax: &syntect::parsing::SyntaxReference,
    ss: &SyntaxSet,
  ) -> StreamingCodeCache {
    let highlighter = Highlighter::new(syn_theme);
    let mut parse_state = ParseState::new(syntax);
    let mut highlight_state = HighlightState::new(&highlighter, ScopeStack::new());
    let mut frozen_job = LayoutJob { text: String::new(), ..Default::default() };
    let mut frozen_len = 0usize;
    let mut incomplete = "";

    for line in syntect::util::LinesWithEndings::from(text) {
      if line.ends_with('\n') {
        let padded_line = pad_unpadded_line_with_ending(line);
        let (ps, hs) = highlight_padded_line_into(
          syn_theme,
          &parse_state,
          &highlight_state,
          ss,
          &mut frozen_job,
          &padded_line,
          code_font_size,
        );
        parse_state = ps;
        highlight_state = hs;
        frozen_len += line.len();
      } else {
        incomplete = line;
      }
    }

    let mut paint_job = frozen_job.clone();
    if !incomplete.is_empty() {
      let padded_tail = pad_unpadded_line_with_ending(incomplete);
      let _ = highlight_padded_line_into(
        syn_theme,
        &parse_state,
        &highlight_state,
        ss,
        &mut paint_job,
        &padded_tail,
        code_font_size,
      );
    }
    paint_job.wrap.max_width = f32::INFINITY;
    let galley = ui.fonts_mut(|f| f.layout_job(paint_job));

    StreamingCodeCache {
      identity_hash,
      source: Arc::from(text),
      frozen_len,
      parse_state,
      highlight_state,
      frozen_job: Arc::new(frozen_job),
      galley,
    }
  }

  #[cfg(test)]
  mod tests {
    use super::*;
    use egui::{vec2, Context, RawInput, Rect, UiBuilder};
    use std::fmt::Write as _;

    fn growing_body(n: usize) -> String {
      let mut body = String::from("fn claim(log: &Log, seq: u64) -> Result<(), ClaimError> {\n");
      for idx in 0..n {
        let _ = writeln!(body, "    let step_{idx} = log.tail()?;");
      }
      body
    }

    #[test]
    fn frozen_len_advances_on_complete_lines() {
      let ctx = Context::default();
      let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(700.0, 400.0));
      let _ = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
        let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
        let c1 = scrolling_code_galley(&mut child, "fn main() {\n    let x", "rust", 12.0, 1, None, None);
        assert_eq!(c1.frozen_len, "fn main() {\n".len());
        assert!(c1.source.as_ref().ends_with("let x"));

        let c2 = scrolling_code_galley(
          &mut child,
          "fn main() {\n    let x = 1;\n    let y",
          "rust",
          12.0,
          1,
          None,
          Some(c1),
        );
        assert_eq!(c2.frozen_len, "fn main() {\n    let x = 1;\n".len());
        assert_eq!(c2.source.as_ref(), "fn main() {\n    let x = 1;\n    let y");
      });
    }

    #[test]
    fn append_reuses_frozen_prefix_bytes() {
      let ctx = Context::default();
      let screen = Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(700.0, 400.0));
      let _ = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
        let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
        let a = growing_body(20);
        let c1 = scrolling_code_galley(&mut child, &a, "rust", 12.0, 1, None, None);
        assert_eq!(c1.frozen_len, a.len());

        let b = growing_body(21);
        assert!(b.starts_with(&a));
        let frozen_before = c1.frozen_len;
        let c2 = scrolling_code_galley(&mut child, &b, "rust", 12.0, 1, None, Some(c1));
        assert!(c2.frozen_len > frozen_before);
        assert_eq!(c2.frozen_len, b.len());
        assert_eq!(c2.source.as_ref(), b);
      });
    }
  }
}
#[cfg(feature = "syntax_highlighting")]
pub use syntect_code::highlight_code;
#[cfg(feature = "syntax_highlighting")]
pub(crate) use syntect_code::{scrolling_code_galley, theme_identity_ptr, StreamingCodeCache};

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

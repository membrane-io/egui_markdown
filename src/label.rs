//! The [`MarkdownLabel`] widget for rendering interactive markdown.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::{
  text::LayoutJob, text_selection::LabelSelectionState, Align, Color32, CursorIcon, FontId, FontSelection, Id, Layout,
  OpenUrl, Pos2, Rect, Response, Sense, Stroke, Ui, UiBuilder, Vec2,
};
use epaint::{
  pos2,
  text::{Galley, Glyph, Row},
};

use crate::layout::{build_layout, highlight_code, section_for_char, CodeThemeArg, LayoutResult};
use crate::link::LinkHandler;
use crate::paint;
use crate::parser;
use crate::style::MarkdownStyle;
use crate::table;
use crate::types::{Markdown, Token};

#[cfg(feature = "syntax_highlighting")]
pub use crate::theme::default_code_theme;

/// Cached parse + layout result stored in egui temp memory.
#[derive(Clone)]
struct CachedMarkdownLayout {
  text_hash: u64,
  layout: Arc<LayoutResult>,
  /// Owned copy of the tokens for rendering (needed for hover/click after cache hit).
  tokens: Arc<Vec<Token<'static>>>,
}

/// Cached layout result for a single flush range (used by segmented render path).
#[derive(Clone)]
struct CachedFlushRange {
  ctx_hash: u64,
  layout: Arc<LayoutResult>,
  tokens: Arc<Vec<Token<'static>>>,
}

#[inline]
fn hash_text(text: &str, style: &MarkdownStyle, handler: Option<&dyn LinkHandler>) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  text.hash(&mut hasher);
  style.hash(&mut hasher);
  if let Some(h) = handler {
    h.id().hash(&mut hasher);
  }
  hasher.finish()
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn hash_flush_context(
  tokens: &[Token<'_>],
  style: &MarkdownStyle,
  font: &FontId,
  color: Color32,
  max_width: f32,
  dark_mode: bool,
  handler: Option<&dyn LinkHandler>,
) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  // Token content identity is already validated by the outer text_hash.
  // Only hash the slice length here to distinguish different flush ranges;
  // the remaining fields capture render-parameter changes (font, color, width, etc.).
  tokens.len().hash(&mut hasher);
  style.hash(&mut hasher);
  font.hash(&mut hasher);
  color.hash(&mut hasher);
  max_width.to_bits().hash(&mut hasher);
  dark_mode.hash(&mut hasher);
  if let Some(h) = handler {
    h.id().hash(&mut hasher);
  }
  hasher.finish()
}

/// Convert borrowed tokens to owned ('static) by converting CowStr::Borrowed to Boxed.
fn tokens_to_owned(tokens: &[Token<'_>]) -> Vec<Token<'static>> {
  tokens.iter().map(|t| token_to_owned(t)).collect()
}

fn cowstr_to_owned(s: &pulldown_cmark::CowStr<'_>) -> pulldown_cmark::CowStr<'static> {
  pulldown_cmark::CowStr::Boxed(s.to_string().into_boxed_str())
}

fn token_to_owned(t: &Token<'_>) -> Token<'static> {
  match t {
    Token::Newline => Token::Newline,
    Token::Text { text, style } => Token::Text { text: cowstr_to_owned(text), style: style.clone() },
    Token::CodeBlock { text, language } => {
      Token::CodeBlock { text: cowstr_to_owned(text), language: language.as_ref().map(|l| cowstr_to_owned(l)) }
    }
    Token::Link { text, href, title } => Token::Link {
      text: cowstr_to_owned(text),
      href: cowstr_to_owned(href),
      title: title.as_ref().map(|t| cowstr_to_owned(t)),
    },
    Token::ListMarker { marker, indent_level } => {
      Token::ListMarker { marker: cowstr_to_owned(marker), indent_level: *indent_level }
    }
    Token::Image { alt, url, title } => Token::Image {
      alt: cowstr_to_owned(alt),
      url: cowstr_to_owned(url),
      title: title.as_ref().map(|t| cowstr_to_owned(t)),
    },
    Token::Table(data) => Token::Table(crate::types::TableData {
      alignments: data.alignments.clone(),
      headers: data.headers.iter().map(|row| tokens_to_owned(row)).collect(),
      rows: data.rows.iter().map(|row| row.iter().map(|cell| tokens_to_owned(cell)).collect()).collect(),
    }),
    Token::HorizontalRule => Token::HorizontalRule,
    Token::BlockquoteStart => Token::BlockquoteStart,
    Token::BlockquoteEnd => Token::BlockquoteEnd,
    Token::TaskListMarker { checked, indent_level } => {
      Token::TaskListMarker { checked: *checked, indent_level: *indent_level }
    }
    Token::FootnoteRef { label } => Token::FootnoteRef { label: cowstr_to_owned(label) },
    Token::FootnoteDef { label } => Token::FootnoteDef { label: cowstr_to_owned(label) },
  }
}

/// An interactive markdown-rendered label widget for egui.
///
/// Parses markdown text and renders it with formatting, clickable links,
/// selectable text, code block overlays, and more.
///
/// # Example
///
/// ```no_run
/// # use eframe::egui;
/// fn ui(ui: &mut egui::Ui) {
///     egui_markdown::MarkdownLabel::new(ui.id().with("md"), "**Hello** world")
///         .show(ui);
/// }
/// ```
pub struct MarkdownLabel<'a> {
  id: Id,
  text: &'a str,
  font: Option<FontId>,
  max_lines: Option<u32>,
  selectable: bool,
  interactable: bool,
  link_handler: Option<&'a dyn LinkHandler>,
  #[allow(clippy::type_complexity)]
  code_block_buttons: Option<&'a dyn Fn(&mut Ui, &str, &str)>,
  scroll_code_blocks: bool,
  style: Option<&'a MarkdownStyle>,
  heal: bool,
  #[cfg(feature = "syntax_highlighting")]
  code_theme: Option<&'a syntect::highlighting::Theme>,
}

impl<'a> MarkdownLabel<'a> {
  /// Create a new markdown label with the given widget ID and markdown source text.
  pub fn new(id: Id, text: &'a str) -> Self {
    Self {
      id,
      text,
      font: None,
      max_lines: None,
      selectable: true,
      interactable: true,
      link_handler: None,
      code_block_buttons: None,
      scroll_code_blocks: false,
      style: None,
      heal: false,
      #[cfg(feature = "syntax_highlighting")]
      code_theme: None,
    }
  }

  /// Override the base font used for rendering.
  pub fn font(self, font: FontId) -> Self {
    Self { font: Some(font), ..self }
  }

  /// Limit the number of visible lines (rows).
  pub fn max_lines(self, n: u32) -> Self {
    Self { max_lines: Some(n), ..self }
  }

  /// Enable or disable text selection. Default: `true`.
  pub fn selectable(self, selectable: bool) -> Self {
    Self { selectable, ..self }
  }

  /// Enable or disable all interaction (links, hover, selection). Default: `true`.
  pub fn interactable(self, interactable: bool) -> Self {
    Self { interactable, ..self }
  }

  /// Set a custom link handler for styling and click behavior.
  pub fn link_handler(self, handler: &'a dyn LinkHandler) -> Self {
    Self { link_handler: Some(handler), ..self }
  }

  /// Add a button overlay to code blocks. The callback receives `(ui, code_text, language)`.
  pub fn code_block_buttons(self, f: &'a dyn Fn(&mut Ui, &str, &str)) -> Self {
    Self { code_block_buttons: Some(f), ..self }
  }

  /// Enable horizontal scrolling for code blocks. Default: `false`.
  ///
  /// When enabled, code blocks render as standalone scrollable widgets instead of
  /// inline galley text. Long lines scroll horizontally rather than wrapping.
  pub fn scroll_code_blocks(self, scroll: bool) -> Self {
    Self { scroll_code_blocks: scroll, ..self }
  }

  /// Auto-close unclosed code fences before parsing.
  ///
  /// Useful for streaming LLM output where text is incomplete. An unclosed
  /// triple-backtick fence causes all subsequent text to be swallowed as code;
  /// enabling heal appends a closing fence before parsing.
  pub fn heal(self, heal: bool) -> Self {
    Self { heal, ..self }
  }

  /// Set custom visual styling. Without this, sensible defaults are used.
  pub fn style(self, style: &'a MarkdownStyle) -> Self {
    Self { style: Some(style), ..self }
  }

  /// Use a custom syntect theme for code block syntax highlighting.
  ///
  /// When set, this theme is used instead of the built-in default.
  /// When `None` (the default), a built-in theme is chosen based on dark/light mode.
  #[cfg(feature = "syntax_highlighting")]
  pub fn code_theme(self, theme: &'a syntect::highlighting::Theme) -> Self {
    Self { code_theme: Some(theme), ..self }
  }

  /// Return the code theme argument for passing to layout functions.
  #[cfg(feature = "syntax_highlighting")]
  fn code_theme_arg(&self) -> CodeThemeArg<'_> {
    self.code_theme
  }

  /// Return the code theme argument (stub when syntax highlighting is disabled).
  #[cfg(not(feature = "syntax_highlighting"))]
  fn code_theme_arg(&self) -> CodeThemeArg<'_> {
    None
  }

  /// Render the markdown into the UI.
  pub fn show(self, ui: &mut Ui) {
    self.render(ui);
  }

  /// Calculate the rendered size without painting.
  pub fn calculate_size(&self, ui: &mut Ui) -> Vec2 {
    let default_style = MarkdownStyle::default();
    let style = self.style.unwrap_or(&default_style);
    let color = ui.visuals().text_color();
    let md = parser::parse(self.text);
    let font = self.font.clone().unwrap_or_else(|| FontSelection::Default.resolve(ui.style()));
    let code_theme = self.code_theme_arg();
    let result = build_layout(ui, &md.tokens, font, color, self.max_lines, self.link_handler, false, style, code_theme);
    let galley = ui.fonts_mut(|f| f.layout_job(result.job));
    galley.size()
  }

  /// Layout the markdown and return the galley position, galley, and response.
  pub fn layout_in_ui(self, ui: &mut Ui) -> (Pos2, Arc<Galley>, Response) {
    let default_style = MarkdownStyle::default();
    let style = self.style.unwrap_or(&default_style);
    let color = ui.visuals().text_color();
    let md = parser::parse(self.text);
    let font = self.font.clone().unwrap_or_else(|| FontSelection::Default.resolve(ui.style()));
    let code_theme = self.code_theme_arg();
    let result = build_layout(ui, &md.tokens, font, color, self.max_lines, self.link_handler, false, style, code_theme);

    let mut job = result.job;
    let available_width = ui.available_width();
    job.wrap.max_width = available_width;
    job.halign = ui.layout().horizontal_placement();
    job.justify = ui.layout().horizontal_justify();

    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let sense = if self.interactable { Sense::click_and_drag() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(galley.size(), sense);

    let galley_pos = match galley.job.halign {
      Align::LEFT => rect.left_top(),
      Align::Center => rect.center_top(),
      Align::RIGHT => rect.right_top(),
    };

    (galley_pos, galley, response)
  }

  fn render(self, ui: &mut Ui) {
    let default_style = MarkdownStyle::default();
    let style = self.style.unwrap_or(&default_style);
    let color = ui.visuals().text_color();
    let font = self.font.clone().unwrap_or_else(|| FontSelection::Default.resolve(ui.style()));

    let healed;
    let text = if self.heal {
      healed = parser::heal(self.text);
      &*healed
    } else {
      self.text
    };

    let text_hash = hash_text(text, style, self.link_handler);
    let cache_id = self.id.with("md_cache");

    // Check if we have a cached layout for this text.
    let cached: Option<CachedMarkdownLayout> = ui.data(|d| d.get_temp(cache_id));

    if let Some(ref cached) = cached {
      if cached.text_hash == text_hash {
        let layout = Arc::clone(&cached.layout);
        let tokens = Arc::clone(&cached.tokens);

        if !layout.segment_breaks.is_empty() {
          let md = Markdown { s: text, tokens: (*tokens).clone() };
          self.render_segmented(ui, &md, &font, color, style, text_hash);
          return;
        }

        self.render_galley(
          ui,
          &tokens,
          layout.job.clone(),
          &layout.section_to_token,
          &layout.code_block_spans,
          &layout.code_block_info,
          &layout.hr_positions,
          &layout.inline_widget_spans,
          color,
          style,
        );
        return;
      }
    }

    // Cache miss - parse and build layout.
    let md = parser::parse(text);
    let code_theme = self.code_theme_arg();
    let layout = build_layout(
      ui,
      &md.tokens,
      font.clone(),
      color,
      self.max_lines,
      self.link_handler,
      self.scroll_code_blocks,
      style,
      code_theme,
    );
    let owned_tokens = Arc::new(tokens_to_owned(&md.tokens));
    let layout = Arc::new(layout);

    // Store in cache.
    ui.data_mut(|d| {
      d.insert_temp(
        cache_id,
        CachedMarkdownLayout { text_hash, layout: Arc::clone(&layout), tokens: Arc::clone(&owned_tokens) },
      );
    });

    if !layout.segment_breaks.is_empty() {
      self.render_segmented(ui, &md, &font, color, style, text_hash);
      return;
    }

    self.render_galley(
      ui,
      &md.tokens,
      layout.job.clone(),
      &layout.section_to_token,
      &layout.code_block_spans,
      &layout.code_block_info,
      &layout.hr_positions,
      &layout.inline_widget_spans,
      color,
      style,
    );
  }

  fn render_segmented(
    &self,
    ui: &mut Ui,
    md: &Markdown<'_>,
    font: &FontId,
    color: Color32,
    style: &MarkdownStyle,
    text_hash: u64,
  ) {
    self.render_token_range(ui, md, font, color, 0, md.tokens.len(), style, text_hash);
  }

  /// Recursively render a range of tokens, handling blockquotes and tables as sub-regions.
  #[allow(clippy::too_many_arguments)]
  fn render_token_range(
    &self,
    ui: &mut Ui,
    md: &Markdown<'_>,
    font: &FontId,
    color: Color32,
    start: usize,
    end: usize,
    style: &MarkdownStyle,
    text_hash: u64,
  ) {
    let mut text_start = start;
    let mut i = start;

    let block_spacing = style.block_spacing;
    // Before rendering a block element, add spacing if there was preceding content.
    let before_block = |had_content: bool, ui: &mut Ui| {
      if had_content {
        ui.add_space(block_spacing);
      }
    };
    // After rendering a block element, skip trailing newlines and add uniform spacing.
    let after_block = |i: &mut usize, text_start: &mut usize, end: usize, ui: &mut Ui| {
      ui.add_space(block_spacing);
      while *text_start < end && matches!(md.tokens[*text_start], Token::Newline) {
        *text_start += 1;
        *i += 1;
      }
    };

    while i < end {
      match &md.tokens[i] {
        Token::Table(data) => {
          let had_content = text_start < i;
          self.flush_text_range(ui, md, font, color, text_start, i, style);
          before_block(had_content, ui);
          let block_sz_id = self.id.with(("block_sz", i));
          if let Some((cached_hash, cached_size)) = ui.data(|d| d.get_temp::<(u64, Vec2)>(block_sz_id)) {
            if cached_hash == text_hash {
              let est_rect = Rect::from_min_size(ui.available_rect_before_wrap().min, cached_size);
              if !ui.is_rect_visible(est_rect) {
                ui.allocate_space(cached_size);
                i += 1;
                text_start = i;
                after_block(&mut i, &mut text_start, end, ui);
                continue;
              }
            }
          }
          let before_y = ui.available_rect_before_wrap().min.y;
          table::render_table(ui, self.id.with(("table", i)), data, font, color, &style.inline_code);
          let after_y = ui.min_rect().bottom();
          let block_size = Vec2::new(ui.available_width(), after_y - before_y);
          ui.data_mut(|d| d.insert_temp(block_sz_id, (text_hash, block_size)));
          i += 1;
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        Token::CodeBlock { text, language } if self.scroll_code_blocks => {
          let had_content = text_start < i;
          self.flush_text_range(ui, md, font, color, text_start, i, style);
          before_block(had_content, ui);
          let block_sz_id = self.id.with(("block_sz", i));
          if let Some((cached_hash, cached_size)) = ui.data(|d| d.get_temp::<(u64, Vec2)>(block_sz_id)) {
            if cached_hash == text_hash {
              let est_rect = Rect::from_min_size(ui.available_rect_before_wrap().min, cached_size);
              if !ui.is_rect_visible(est_rect) {
                ui.allocate_space(cached_size);
                i += 1;
                text_start = i;
                after_block(&mut i, &mut text_start, end, ui);
                continue;
              }
            }
          }
          let before_y = ui.available_rect_before_wrap().min.y;
          render_code_block(
            ui,
            self.id.with(("code_block", i)),
            text,
            language.as_deref(),
            self.code_block_buttons,
            style,
            self.code_theme_arg(),
          );
          let after_y = ui.min_rect().bottom();
          let block_size = Vec2::new(ui.available_width(), after_y - before_y);
          ui.data_mut(|d| d.insert_temp(block_sz_id, (text_hash, block_size)));
          i += 1;
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        #[cfg(feature = "images")]
        Token::Image { url, .. } => {
          let had_content = text_start < i;
          self.flush_text_range(ui, md, font, color, text_start, i, style);
          before_block(had_content, ui);
          let block_sz_id = self.id.with(("block_sz", i));
          if let Some((cached_hash, cached_size)) = ui.data(|d| d.get_temp::<(u64, Vec2)>(block_sz_id)) {
            if cached_hash == text_hash {
              let est_rect = Rect::from_min_size(ui.available_rect_before_wrap().min, cached_size);
              if !ui.is_rect_visible(est_rect) {
                ui.allocate_space(cached_size);
                i += 1;
                text_start = i;
                after_block(&mut i, &mut text_start, end, ui);
                continue;
              }
            }
          }
          let before_y = ui.available_rect_before_wrap().min.y;
          let image = egui::Image::new(url.as_ref()).max_width(ui.available_width()).show_loading_spinner(true);
          ui.add(image);
          let after_y = ui.min_rect().bottom();
          let block_size = Vec2::new(ui.available_width(), after_y - before_y);
          ui.data_mut(|d| d.insert_temp(block_sz_id, (text_hash, block_size)));
          i += 1;
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        #[cfg(not(feature = "images"))]
        Token::Image { alt, url, .. } => {
          self.flush_text_range(ui, md, font, color, text_start, i, style);
          let block_sz_id = self.id.with(("block_sz", i));
          if let Some((cached_hash, cached_size)) = ui.data(|d| d.get_temp::<(u64, Vec2)>(block_sz_id)) {
            if cached_hash == text_hash {
              let est_rect = Rect::from_min_size(ui.available_rect_before_wrap().min, cached_size);
              if !ui.is_rect_visible(est_rect) {
                ui.allocate_space(cached_size);
                i += 1;
                text_start = i;
                after_block(&mut i, &mut text_start, end, ui);
                continue;
              }
            }
          }
          let before_y = ui.available_rect_before_wrap().min.y;
          let text = if alt.is_empty() { url.as_ref() } else { alt.as_ref() };
          if ui.link(text).clicked() {
            ui.ctx().open_url(OpenUrl::new_tab(url.to_string()));
          }
          let after_y = ui.min_rect().bottom();
          let block_size = Vec2::new(ui.available_width(), after_y - before_y);
          ui.data_mut(|d| d.insert_temp(block_sz_id, (text_hash, block_size)));
          i += 1;
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        Token::Link { text, href, .. } if self.link_handler.is_some_and(|h| h.is_block_widget(href)) => {
          self.flush_text_range(ui, md, font, color, text_start, i, style);
          let handler = self.link_handler.unwrap();
          handler.block_widget(ui, text, href);
          i += 1;
          text_start = i;
        }
        Token::BlockquoteStart => {
          self.flush_text_range(ui, md, font, color, text_start, i, style);

          // Find the matching BlockquoteEnd.
          let bq_start = i + 1;
          let mut depth = 1u32;
          let mut j = bq_start;
          while j < end && depth > 0 {
            match &md.tokens[j] {
              Token::BlockquoteStart => depth += 1,
              Token::BlockquoteEnd => depth -= 1,
              _ => {}
            }
            if depth > 0 {
              j += 1;
            }
          }
          let bq_end = j; // index of matching BlockquoteEnd

          // Skip leading newlines inside the blockquote.
          let mut bq_content_start = bq_start;
          while bq_content_start < bq_end && matches!(md.tokens[bq_content_start], Token::Newline) {
            bq_content_start += 1;
          }

          let indent = style.blockquote.indent_per_depth;
          let mut child_rect = ui.available_rect_before_wrap();
          child_rect.min.x += indent;

          let mut child_ui = ui.new_child(UiBuilder::new().id_salt(self.id.with(("bq", i))).max_rect(child_rect));
          self.render_token_range(&mut child_ui, md, font, color, bq_content_start, bq_end, style, text_hash);

          let bq_stroke =
            Stroke::new(style.blockquote.stroke_width, ui.visuals().widgets.noninteractive.bg_stroke.color);
          let line_x = child_rect.min.x - indent * 0.5;
          let top = child_rect.min.y;
          let bottom = child_ui.min_rect().bottom();
          ui.painter().line_segment([pos2(line_x, top), pos2(line_x, bottom)], bq_stroke);

          // Advance parent UI past the child content.
          let child_min_rect = child_ui.min_rect();
          ui.allocate_rect(child_min_rect, Sense::hover());

          i = bq_end + 1; // skip past BlockquoteEnd
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        Token::BlockquoteEnd => {
          self.flush_text_range(ui, md, font, color, text_start, i, style);
          i += 1;
          text_start = i;
        }
        _ => {
          i += 1;
        }
      }
    }

    // Flush remaining text.
    self.flush_text_range(ui, md, font, color, text_start, end, style);
  }

  /// Render a range of non-break tokens as an interactive galley.
  #[allow(clippy::too_many_arguments)]
  fn flush_text_range(
    &self,
    ui: &mut Ui,
    md: &Markdown<'_>,
    font: &FontId,
    color: Color32,
    start: usize,
    end: usize,
    style: &MarkdownStyle,
  ) {
    if start >= end {
      return;
    }
    // Skip if only newlines.
    if md.tokens[start..end].iter().all(|t| matches!(t, Token::Newline | Token::BlockquoteStart | Token::BlockquoteEnd))
    {
      return;
    }

    // Trim trailing newlines; spacing between blocks is handled by render_token_range.
    let mut trimmed_end = end;
    while trimmed_end > start && matches!(md.tokens[trimmed_end - 1], Token::Newline) {
      trimmed_end -= 1;
    }
    if trimmed_end <= start {
      return;
    }

    let token_slice = &md.tokens[start..trimmed_end];
    let max_width = if ui.wrap_mode() == egui::TextWrapMode::Extend { f32::INFINITY } else { ui.available_width() };
    let dark_mode = ui.visuals().dark_mode;
    let ctx_hash = hash_flush_context(token_slice, style, font, color, max_width, dark_mode, self.link_handler);
    let cache_id = self.id.with(("flush", start));
    let size_cache_id = self.id.with(("flush_sz", start));

    // Viewport culling: skip layout+paint for off-screen segments.
    if let Some((cached_hash, cached_size)) = ui.data(|d| d.get_temp::<(u64, Vec2)>(size_cache_id)) {
      if cached_hash == ctx_hash {
        let est_rect = Rect::from_min_size(ui.available_rect_before_wrap().min, cached_size);
        if !ui.is_rect_visible(est_rect) {
          ui.allocate_space(cached_size);
          return;
        }
      }
    }

    if let Some(cached) = ui.data(|d| d.get_temp::<CachedFlushRange>(cache_id)) {
      if cached.ctx_hash == ctx_hash {
        let size = self.render_galley(
          ui,
          &cached.tokens,
          cached.layout.job.clone(),
          &cached.layout.section_to_token,
          &cached.layout.code_block_spans,
          &cached.layout.code_block_info,
          &cached.layout.hr_positions,
          &cached.layout.inline_widget_spans,
          color,
          style,
        );
        ui.data_mut(|d| d.insert_temp(size_cache_id, (ctx_hash, size)));
        return;
      }
    }

    let code_theme = self.code_theme_arg();
    let layout = build_layout(ui, token_slice, font.clone(), color, None, self.link_handler, false, style, code_theme);
    debug_assert!(layout.segment_breaks.is_empty());
    let owned = Arc::new(tokens_to_owned(token_slice));
    let layout = Arc::new(layout);
    ui.data_mut(|d| {
      d.insert_temp(cache_id, CachedFlushRange { ctx_hash, layout: Arc::clone(&layout), tokens: Arc::clone(&owned) })
    });
    let size = self.render_galley(
      ui,
      token_slice,
      layout.job.clone(),
      &layout.section_to_token,
      &layout.code_block_spans,
      &layout.code_block_info,
      &layout.hr_positions,
      &layout.inline_widget_spans,
      color,
      style,
    );
    ui.data_mut(|d| d.insert_temp(size_cache_id, (ctx_hash, size)));
  }

  #[allow(clippy::too_many_arguments)]
  fn render_galley(
    &self,
    ui: &mut Ui,
    tokens: &[Token<'_>],
    job: LayoutJob,
    section_to_token: &[usize],
    code_block_spans: &[(usize, usize)],
    code_block_info: &[(String, String)],
    hr_positions: &[usize],
    inline_widget_spans: &[(usize, usize, usize)],
    color: Color32,
    style: &MarkdownStyle,
  ) -> Vec2 {
    let mut job = job;
    job.wrap.max_width =
      if ui.wrap_mode() == egui::TextWrapMode::Extend { f32::INFINITY } else { ui.available_width() };
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let size = galley.size();
    let code_block_rects = paint::compute_code_block_rects(ui, code_block_spans, &galley);
    let available_width = ui.available_width();

    if !self.interactable {
      let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
      paint_decorations(ui, hr_positions, &galley, &code_block_rects, rect.min, available_width, style);
      ui.painter().galley(rect.min, galley.clone(), color);
      if let Some(handler) = self.link_handler {
        paint_inline_widgets(ui, handler, tokens, inline_widget_spans, &galley, rect.min);
      }
      return size;
    }

    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    paint_decorations(ui, hr_positions, &galley, &code_block_rects, response.rect.min, available_width, style);

    let disable_text_selection = !self.selectable || ui.input(|input| input.modifiers.shift);
    if !disable_text_selection {
      LabelSelectionState::label_text_selection(ui, &response, rect.min, galley.clone(), color, Stroke::NONE);
    } else {
      ui.painter().galley(rect.min, galley.clone(), color);
    }

    if let Some(button_fn) = self.code_block_buttons {
      render_code_block_buttons(ui, button_fn, &code_block_rects, code_block_info, &response);
    }

    if let Some(handler) = self.link_handler {
      paint_inline_widgets(ui, handler, tokens, inline_widget_spans, &galley, response.rect.min);
    }

    let hovered_section =
      self.handle_hover(ui, tokens, &galley, section_to_token, inline_widget_spans, &response, rect, color);

    if self.interactable && response.clicked() {
      self.handle_click(ui, tokens, section_to_token, hovered_section);
    }
    size
  }

  #[allow(clippy::too_many_arguments)]
  fn handle_hover(
    &self,
    ui: &mut Ui,
    tokens: &[Token<'_>],
    galley: &Arc<Galley>,
    section_to_token: &[usize],
    inline_widget_spans: &[(usize, usize, usize)],
    response: &Response,
    rect: Rect,
    _color: Color32,
  ) -> Option<u32> {
    if !self.interactable || !response.hovered() {
      return None;
    }

    let current_transform = ui.ctx().layer_transform_to_global(ui.layer_id());
    let transformed_pointer_pos = ui.input(|i| {
      i.pointer.latest_pos().map(|raw_pos| {
        if let Some(transform) = current_transform {
          transform.inverse() * raw_pos
        } else {
          raw_pos
        }
      })
    });

    let pos_in_galley = transformed_pointer_pos.map(|pos| pos - rect.min.to_vec2())?;
    let glyph_index = cursor_from_pos(galley, pos_in_galley)?;
    let index_to_section = |index: u32| -> Option<u32> { section_for_char(&galley.job, index) };

    let section_index = index_to_section(glyph_index)?;
    let token_index = section_to_token.get(section_index as usize).copied();
    let token = token_index.and_then(|idx| tokens.get(idx));

    // Check if this token is an inline widget - show pointing hand but suppress underline/tooltip.
    let is_inline_widget =
      token_index.is_some_and(|ti| inline_widget_spans.iter().any(|&(_, _, span_ti)| span_ti == ti));

    if is_inline_widget {
      ui.output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);
      return Some(section_index);
    }

    let stroke = match token {
      Some(Token::Link { href, .. }) => {
        let link_color = if let Some(handler) = self.link_handler {
          handler.link_style(href).and_then(|s| s.color).unwrap_or(ui.visuals().hyperlink_color)
        } else {
          ui.visuals().hyperlink_color
        };
        Some(Stroke::new(1.0, link_color))
      }
      _ => None,
    };

    if let Some(stroke) = stroke {
      // Find all sections belonging to the same token (for multi-section layout_link).
      let target_token = token_index.unwrap();
      for (sec_idx, &tok_idx) in section_to_token.iter().enumerate() {
        if tok_idx != target_token {
          continue;
        }

        // Find the char range for this section.
        let mut sec_start_char = 0u32;
        for s in &galley.job.sections[..sec_idx] {
          sec_start_char += galley.job.text[s.byte_range.clone()].chars().count() as u32;
        }
        let sec_char_count = galley.job.sections[sec_idx]
          .byte_range
          .clone()
          .len()
          .min(galley.job.text[galley.job.sections[sec_idx].byte_range.clone()].chars().count());
        let sec_end_char = sec_start_char + sec_char_count as u32;
        if sec_end_char <= sec_start_char {
          continue;
        }

        let Some((sg, sr)) = glyph_at_index(galley, sec_start_char) else { continue };
        let Some((eg, er)) = glyph_at_index(galley, sec_end_char.saturating_sub(1)) else { continue };

        for row_index in sr..=er {
          let row = &galley.rows[row_index as usize];
          let row_sg = if row_index == sr { Some(sg) } else { row.glyphs.first() };
          let row_eg = if row_index == er { Some(eg) } else { last_non_whitespace_glyph(row) };
          if let Some((sg, eg)) = row_sg.zip(row_eg) {
            let row_rect =
              Rect::from_min_max(pos2(sg.pos.x, row.min_y()), pos2(eg.pos.x + eg.advance_width, row.max_y()))
                .translate(rect.min.to_vec2());
            ui.painter().line_segment([row_rect.left_bottom(), row_rect.right_bottom()], stroke);
          }
        }
      }
      ui.output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);

      // Show URL in tooltip when url_in_tooltip is enabled (matches egui's Hyperlink behavior)
      if ui.style().url_in_tooltip {
        if let Some(Token::Link { href, .. }) = token {
          response.clone().on_hover_text_at_pointer(href.as_ref());
        }
      }
    }

    Some(section_index)
  }

  fn handle_click(&self, ui: &mut Ui, tokens: &[Token<'_>], section_to_token: &[usize], hovered_section: Option<u32>) {
    let section_index = match hovered_section {
      Some(s) => s,
      None => return,
    };
    let token_index = section_to_token.get(section_index as usize).copied();
    let token = token_index.and_then(|idx| tokens.get(idx));

    if let Some(Token::Link { text, href, .. }) = token {
      let handled = if let Some(handler) = self.link_handler { handler.click(text, href, ui) } else { false };
      if !handled {
        ui.ctx().open_url(OpenUrl::new_tab(href.to_string()));
      }
    }
  }
}

fn paint_decorations(
  ui: &Ui,
  hr_positions: &[usize],
  galley: &Arc<Galley>,
  code_block_rects: &[Rect],
  origin: Pos2,
  available_width: f32,
  style: &MarkdownStyle,
) {
  for bg_rect in code_block_rects {
    paint::paint_code_block_bg(ui, *bg_rect, origin, &style.code_block);
  }
  paint::paint_horizontal_rules(ui, hr_positions, galley, origin, available_width, &style.horizontal_rule);
}

fn render_code_block_buttons(
  ui: &mut Ui,
  button_fn: &dyn Fn(&mut Ui, &str, &str),
  code_block_rects: &[Rect],
  code_block_info: &[(String, String)],
  response: &Response,
) {
  for (bg_rect, (code_text, lang)) in code_block_rects.iter().zip(code_block_info.iter()) {
    let mut bg_rect = bg_rect.translate(response.rect.min.to_vec2());
    bg_rect.min.x -= 4.0;
    bg_rect.min.y -= 6.0;
    bg_rect.max.x += 12.0;
    bg_rect.max.y += 6.0;

    let header_rect = Rect::from_min_max(pos2(bg_rect.min.x, bg_rect.min.y), pos2(bg_rect.max.x, bg_rect.min.y + 24.0));
    let mut child_ui =
      ui.new_child(UiBuilder::new().max_rect(header_rect).layout(Layout::right_to_left(Align::Center)));
    button_fn(&mut child_ui, code_text, lang);
  }
}

/// Paint inline widgets over their transparent placeholder regions.
fn paint_inline_widgets(
  ui: &mut Ui,
  handler: &dyn LinkHandler,
  tokens: &[Token<'_>],
  spans: &[(usize, usize, usize)],
  galley: &Arc<Galley>,
  origin: Pos2,
) {
  for &(start_char, end_char, token_index) in spans {
    let (text, href) = match tokens.get(token_index) {
      Some(Token::Link { text, href, .. }) => (text.as_ref(), href.as_ref()),
      _ => continue,
    };

    let Some((start_glyph, start_row)) = glyph_at_index(galley, start_char as u32) else { continue };
    let Some((end_glyph, end_row)) = glyph_at_index(galley, (end_char as u32).saturating_sub(1)) else { continue };

    for row_index in start_row..=end_row {
      let row = &galley.rows[row_index as usize];
      let row_sg = if row_index == start_row { Some(start_glyph) } else { row.glyphs.first() };
      let row_eg = if row_index == end_row { Some(end_glyph) } else { row.glyphs.last() };
      if let (Some(sg), Some(eg)) = (row_sg, row_eg) {
        let row_rect = Rect::from_min_max(pos2(sg.pos.x, row.min_y()), pos2(eg.pos.x + eg.advance_width, row.max_y()))
          .translate(origin.to_vec2());
        handler.paint_inline_widget(ui, text, href, row_rect);
      }
    }
  }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn render_code_block(
  ui: &mut Ui,
  id: Id,
  text: &pulldown_cmark::CowStr<'_>,
  language: Option<&str>,
  code_block_buttons: Option<&dyn Fn(&mut Ui, &str, &str)>,
  style: &MarkdownStyle,
  code_theme: CodeThemeArg<'_>,
) {
  let lang = language.unwrap_or(style.default_code_language.as_str());
  let mut padded_text = String::with_capacity(text.len() + text.lines().count());
  for (i, line) in text.lines().enumerate() {
    if i > 0 {
      padded_text.push('\n');
    }
    padded_text.push(' ');
    padded_text.push_str(line);
  }

  let mut job = highlight_code(ui, &padded_text, lang, style.code_font_size, code_theme);
  job.wrap.max_width = f32::INFINITY;

  let galley = ui.fonts_mut(|f| f.layout_job(job));
  let galley_width = galley.size().x;
  let galley_height = galley.size().y;
  let stroke = Stroke::new(style.code_block.stroke_width, ui.visuals().widgets.noninteractive.bg_stroke.color);

  let p = &style.code_block.padding;
  let frame = egui::Frame::NONE
    .fill(ui.visuals().code_bg_color)
    .stroke(stroke)
    .corner_radius(style.code_block.corner_radius)
    .inner_margin(egui::Margin { left: p[0] as i8, top: p[1] as i8, right: p[2] as i8, bottom: p[3] as i8 });

  let frame_response = frame.show(ui, |ui| {
    ui.set_min_width(ui.available_width());
    egui::ScrollArea::horizontal().id_salt(id).show(ui, |ui| {
      let (rect, _) = ui.allocate_exact_size(epaint::vec2(galley_width, galley_height), Sense::hover());
      ui.painter().galley(rect.min, galley, ui.visuals().text_color());
    });
  });

  // Overlay buttons in the top-right, absolutely positioned over the frame.
  if let Some(button_fn) = code_block_buttons {
    let border = frame_response.response.rect;
    let header_rect = Rect::from_min_max(pos2(border.min.x, border.min.y), pos2(border.max.x, border.min.y + 24.0));
    let mut child_ui =
      ui.new_child(UiBuilder::new().max_rect(header_rect).layout(Layout::right_to_left(Align::Center)));
    child_ui.add_space(4.0);
    button_fn(&mut child_ui, text, lang);
  }
}

/// Custom cursor-from-position that avoids egui's Galley::cursor_from_pos bug (#5796).
pub fn cursor_from_pos(galley: &Galley, pos: Pos2) -> Option<u32> {
  if !galley.rect.contains(pos) {
    return None;
  }
  let mut index = 0;
  for row in &galley.rows {
    let is_pos_within_row = row.min_y() <= pos.y && pos.y <= row.max_y();
    if is_pos_within_row {
      if let Some(column) =
        row.glyphs.iter().position(|glyph| glyph.pos.x <= pos.x && glyph.pos.x + glyph.advance_width >= pos.x)
      {
        return Some(index + column as u32);
      }
    }
    index += row.char_count_including_newline() as u32;
  }
  None
}

/// Finds the glyph and row_index from the given galley index.
pub fn glyph_at_index(galley: &Galley, index: u32) -> Option<(&Glyph, u32)> {
  let mut offset = 0;
  for (row_index, row) in galley.rows.iter().enumerate() {
    if index < offset + row.char_count_including_newline() as u32 {
      return row.glyphs.get((index - offset) as usize).map(|glyph| (glyph, row_index as u32));
    }
    offset += row.char_count_including_newline() as u32;
  }
  None
}

/// Helpful to avoid painting underlines for trailing whitespace.
pub fn last_non_whitespace_glyph(row: &Row) -> Option<&Glyph> {
  row.glyphs.iter().rfind(|glyph| !glyph.chr.is_whitespace())
}

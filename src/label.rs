//! The [`MarkdownLabel`] widget for rendering interactive markdown.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::{
  text::LayoutJob, text_selection::LabelSelectionState, Align, Color32, Context, CursorIcon, FontId, FontSelection, Id,
  Layout, OpenUrl, Pos2, Rect, Response, Sense, Stroke, TextWrapMode, Ui, UiBuilder, Vec2,
};
use epaint::{
  pos2,
  text::{cursor::CCursor, CharIndex, Galley, Glyph, Row},
};

use epaint::text::ByteRangeExt as _;

#[cfg(not(feature = "syntax_highlighting"))]
use crate::layout::highlight_code;
use crate::layout::{build_layout, needs_segmentation, section_for_char, CodeThemeArg, LayoutResult};
#[cfg(feature = "syntax_highlighting")]
use crate::layout::{scrolling_code_galley, StreamingCodeCache};
use crate::link::{InlineWidgetRow, LinkHandler};
use crate::paint;
use crate::parser;
use crate::style::MarkdownStyle;
use crate::table;
use crate::types::{tokens_to_owned, Token};

#[cfg(feature = "syntax_highlighting")]
pub use crate::theme::default_code_theme;

/// Cached parse + layout result stored in egui temp memory.
#[derive(Clone, Default)]
struct CachedMarkdownLayout {
  text_hash: u64,
  /// `None` when the document takes the segmented render path, which lays out each flush
  /// range separately and so has no use for a whole-document layout.
  layout: Option<Arc<LayoutResult>>,
  /// The owned tokens for rendering (needed for hover/click after cache hit), and where the parse of
  /// the next text, such as the next delta of a streamed reply, can start.
  parse: parser::ParseState,
}

/// Cached layout result for a single flush range (used by segmented render path).
#[derive(Clone)]
struct CachedFlushRange {
  /// The parse generation and the frame key of the frame that last used this entry, and the end of the
  /// range then. While [`Revision::unchanged`] holds, the range holds the same tokens, so the label skips
  /// hashing them.
  generation: u64,
  frame_key: u64,
  end: usize,
  ctx_hash: u64,
  layout: Arc<LayoutResult>,
  tokens: Arc<Vec<Token<'static>>>,
}

#[cfg(not(feature = "syntax_highlighting"))]
#[derive(Clone)]
struct CachedCodeBlock {
  ctx_hash: u64,
  galley: Arc<Galley>,
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

/// Identity of everything a [`LayoutResult`] depends on.
///
/// Deliberately excludes the wrap width and break flag: `build_layout` only seeds
/// those on `job.wrap`, and `render_galley` overwrites them with the live values
/// before shaping. Hashing them here would rebuild every job on every frame of a resize.
#[inline]
fn hash_flush_context(
  tokens: &[Token<'_>],
  style: &MarkdownStyle,
  font: &FontId,
  color: Color32,
  dark_mode: bool,
  handler: Option<&dyn LinkHandler>,
) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  // Token content must be hashed: a flush range is keyed by its start index, and an edit
  // can change a range's content while leaving its start index and length untouched.
  tokens.hash(&mut hasher);
  style.hash(&mut hasher);
  font.hash(&mut hasher);
  color.hash(&mut hasher);
  dark_mode.hash(&mut hasher);
  if let Some(h) = handler {
    h.id().hash(&mut hasher);
  }
  hasher.finish()
}

/// What a cached entry for a range of tokens needs to know to skip hashing them.
#[derive(Clone, Copy)]
struct Revision {
  /// [`parser::ParseState::generation`] of the tokens in this frame.
  generation: u64,
  /// [`parser::ParseState::kept_from`] of the tokens in this frame.
  kept_from: Option<(u64, usize)>,
  /// [`frame_key`] of this frame.
  frame_key: u64,
}

impl Revision {
  /// Whether an entry for the tokens before `end`, last used in `generation` with `frame_key`, still
  /// holds the same tokens and inputs. It does when nothing was parsed since, or when this parse kept
  /// those tokens from the parse that the entry was last used in.
  fn unchanged(&self, generation: u64, frame_key: u64, end: usize) -> bool {
    frame_key == self.frame_key
      && (generation == self.generation
        || self.kept_from.is_some_and(|(kept_generation, kept)| generation == kept_generation && end <= kept))
  }
}

/// The inputs other than the tokens that a cached layout or height depends on: the style, the link
/// handler, the font, the color and the theme. It costs the same on every frame, whatever the length of
/// the document.
fn frame_key(
  style: &MarkdownStyle,
  handler: Option<&dyn LinkHandler>,
  font: &FontId,
  color: Color32,
  dark_mode: bool,
) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  (style, handler.map(|h| h.id()), font, color, dark_mode).hash(&mut hasher);
  hasher.finish()
}

/// Identity of everything a block's height depends on, except the width: the block token, the style
/// and the font. It does not change when text elsewhere in the document does.
#[inline]
fn hash_block(token: &Token<'_>, style: &MarkdownStyle, font: &FontId) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  token.hash(&mut hasher);
  style.hash(&mut hasher);
  font.hash(&mut hasher);
  hasher.finish()
}

/// Identity of everything a scrolling code-block galley depends on except the source text.
#[cfg(feature = "syntax_highlighting")]
#[inline]
fn hash_code_block_identity(
  ui: &Ui,
  language: &str,
  code_font_size: f32,
  dark_mode: bool,
  pixels_per_point: f32,
  code_theme: CodeThemeArg<'_>,
) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  language.hash(&mut hasher);
  code_font_size.to_bits().hash(&mut hasher);
  dark_mode.hash(&mut hasher);
  pixels_per_point.to_bits().hash(&mut hasher);
  crate::layout::theme_identity_ptr(ui, code_theme).hash(&mut hasher);
  hasher.finish()
}

#[cfg(not(feature = "syntax_highlighting"))]
#[inline]
fn hash_code_block_context(
  text: &str,
  language: &str,
  code_font_size: f32,
  dark_mode: bool,
  pixels_per_point: f32,
  code_theme: CodeThemeArg<'_>,
) -> u64 {
  let mut hasher = std::collections::hash_map::DefaultHasher::new();
  text.hash(&mut hasher);
  language.hash(&mut hasher);
  code_font_size.to_bits().hash(&mut hasher);
  dark_mode.hash(&mut hasher);
  pixels_per_point.to_bits().hash(&mut hasher);
  let _ = code_theme;
  hasher.finish()
}

/// The measured height of a block, and the keys it is valid for.
#[derive(Clone, Copy)]
struct BlockHeight {
  /// The parse generation and the frame key when the block was last measured or culled. While
  /// [`Revision::unchanged`] holds, the block is the same, and the label skips `block_hash`.
  generation: u64,
  frame_key: u64,
  /// [`hash_block`] of the block, for a frame where the revision cannot vouch for it.
  block_hash: u64,
  height: f32,
}

/// The cached height of the block at token `index`, when it is still valid.
fn cached_block_height(
  ui: &Ui,
  block_sz_id: Id,
  revision: Revision,
  index: usize,
  block_hash: impl FnOnce() -> u64,
) -> Option<f32> {
  let cached = ui.data(|d| d.get_temp::<BlockHeight>(block_sz_id))?;
  let valid = revision.unchanged(cached.generation, cached.frame_key, index + 1)
    || (cached.frame_key == revision.frame_key && cached.block_hash == block_hash());
  if !valid {
    return None;
  }
  if cached.generation != revision.generation {
    ui.data_mut(|d| d.insert_temp(block_sz_id, BlockHeight { generation: revision.generation, ..cached }));
  }
  Some(cached.height)
}

/// Reserve space for an off-screen block using cached height and current available width.
/// Width is not cached so a panel shrink cannot leave a stale wider min-size.
fn try_cull_block(
  ui: &mut Ui,
  block_sz_id: Id,
  revision: Revision,
  index: usize,
  block_hash: impl FnOnce() -> u64,
) -> bool {
  let Some(cached_height) = cached_block_height(ui, block_sz_id, revision, index, block_hash) else {
    return false;
  };
  let size = Vec2::new(ui.available_width(), cached_height);
  let est_rect = Rect::from_min_size(ui.available_rect_before_wrap().min, size);
  if ui.is_rect_visible(est_rect) {
    return false;
  }
  ui.allocate_space(size);
  true
}

fn cache_block_height(
  ui: &mut Ui,
  block_sz_id: Id,
  revision: Revision,
  index: usize,
  block_hash: impl FnOnce() -> u64,
  before_y: f32,
) {
  let height = ui.min_rect().bottom() - before_y;
  let cached = ui.data(|d| d.get_temp::<BlockHeight>(block_sz_id));
  let block_hash = match cached {
    Some(cached) if revision.unchanged(cached.generation, cached.frame_key, index + 1) => cached.block_hash,
    _ => block_hash(),
  };
  let frame_key = revision.frame_key;
  ui.data_mut(|d| {
    d.insert_temp(block_sz_id, BlockHeight { generation: revision.generation, frame_key, block_hash, height })
  });
}

/// What the layout does with a token that is wider than the available width.
///
/// This setting is independent of [`MarkdownLabel::wrap_mode`]. The wrap mode selects
/// whether the box wraps, truncates, or extends. This enum selects what happens when one
/// run of characters still does not fit.
///
/// The variants are close to the CSS `overflow-wrap` and `word-break` properties.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverflowWrap {
  /// Break at a word boundary, a dash or a punctuation mark. A row that has none of these
  /// breaks between two characters, so the text does not overrun the box.
  ///
  /// This is close to CSS `overflow-wrap: break-word`.
  #[default]
  Normal,
  /// Break between any two characters when the row reaches `max_width`.
  ///
  /// This matches CSS `word-break: break-all`. It can split an ordinary sentence inside
  /// a word.
  BreakAll,
}

/// Resolved wrap settings for one layout/paint pass.
struct ResolvedWrap {
  mode: TextWrapMode,
  max_width: f32,
  break_anywhere: bool,
  max_rows: Option<u32>,
}

/// What [`MarkdownLabel::show`] returns.
pub struct MarkdownLabelOutput {
  /// The union of the responses of the text in the label. A table, a scrolled code block and an image
  /// have their own responses, which this response does not include.
  pub response: Response,

  /// True when the user clicked a link in this frame. The label handles that click itself, so a parent
  /// that reacts to `response.clicked()` must ignore it.
  pub link_clicked: bool,
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
  wrap_mode: Option<TextWrapMode>,
  overflow_wrap: OverflowWrap,
  selectable: bool,
  interactable: bool,
  link_handler: Option<&'a dyn LinkHandler>,
  #[allow(clippy::type_complexity)]
  code_block_buttons: Option<&'a dyn Fn(&mut Ui, &str, &str)>,
  scroll_code_blocks: bool,
  shrink_code_blocks: bool,
  hug_content: bool,
  code_block_min_width: Option<f32>,
  style: Option<&'a MarkdownStyle>,
  heal: bool,
  #[allow(clippy::type_complexity)]
  map_job: Option<Box<dyn Fn(&Context, LayoutJob) -> LayoutJob + 'a>>,
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
      wrap_mode: None,
      overflow_wrap: OverflowWrap::Normal,
      selectable: true,
      interactable: true,
      link_handler: None,
      code_block_buttons: None,
      scroll_code_blocks: false,
      shrink_code_blocks: false,
      hug_content: false,
      code_block_min_width: None,
      style: None,
      heal: false,
      map_job: None,
      #[cfg(feature = "syntax_highlighting")]
      code_theme: None,
    }
  }

  /// Override the base font used for rendering.
  pub fn font(self, font: FontId) -> Self {
    Self { font: Some(font), ..self }
  }

  /// Limit the number of visible lines (rows).
  ///
  /// Under [`Self::truncate`], this is the elision row budget (default `1` when unset),
  /// so `.truncate().max_lines(3)` acts like a CSS line-clamp.
  ///
  /// Row limits apply only to the whole-document render path. Documents that need
  /// segmentation (tables, images, blockquotes, scrolling code blocks) ignore this.
  pub fn max_lines(self, n: u32) -> Self {
    Self { max_lines: Some(n), ..self }
  }

  /// Override the wrap mode for this widget.
  ///
  /// When unset, [`Ui::wrap_mode`] is used (same as egui's [`egui::Label`]).
  pub fn wrap_mode(self, wrap_mode: TextWrapMode) -> Self {
    Self { wrap_mode: Some(wrap_mode), ..self }
  }

  /// Set wrap mode to [`TextWrapMode::Wrap`].
  pub fn wrap(self) -> Self {
    self.wrap_mode(TextWrapMode::Wrap)
  }

  /// Set wrap mode to [`TextWrapMode::Truncate`].
  ///
  /// Forces [`OverflowWrap::BreakAll`] at layout time. An explicit
  /// [`OverflowWrap::Normal`] from [`Self::overflow_wrap`] does not survive truncate,
  /// matching egui/`epaint` guidance for `max_rows = 1`.
  ///
  /// Elides after [`Self::max_lines`] rows, or 1 when `max_lines` is unset.
  /// Same segmentation limitation as [`Self::max_lines`].
  pub fn truncate(self) -> Self {
    self.wrap_mode(TextWrapMode::Truncate)
  }

  /// Set wrap mode to [`TextWrapMode::Extend`], disabling wrapping and truncation.
  pub fn extend(self) -> Self {
    self.wrap_mode(TextWrapMode::Extend)
  }

  /// How to break tokens that are wider than the available width.
  ///
  /// Default: [`OverflowWrap::Normal`]. Ignored under [`Self::extend`] (nothing to
  /// break against) and overridden to [`OverflowWrap::BreakAll`] under [`Self::truncate`].
  pub fn overflow_wrap(self, overflow_wrap: OverflowWrap) -> Self {
    Self { overflow_wrap, ..self }
  }

  /// Enable or disable text selection. Default: `true`.
  ///
  /// As on [`egui::Label`], a shift-click extends the current selection. An app that gives Shift
  /// another meaning can pass `!ui.input(|i| i.modifiers.shift)` here.
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

  /// Size code blocks to their content instead of filling the available width.
  /// Default: `false` (fill). Only applies with [`Self::scroll_code_blocks`].
  ///
  /// Useful for tooltips and popovers that should hug their content.
  pub fn shrink_code_blocks(self, shrink: bool) -> Self {
    Self { shrink_code_blocks: shrink, ..self }
  }

  /// Size the whole widget to the laid-out galley instead of filling available width.
  /// Default: `false` (fill). Wrapping still uses the available width as a cap.
  ///
  /// Decorations (code-block backgrounds, horizontal rules) span the allocated
  /// width, not the leftover space in the parent.
  pub fn hug_content(self, hug: bool) -> Self {
    Self { hug_content: hug, ..self }
  }

  /// Minimum width for shrunk code blocks (capped at the available width).
  ///
  /// Lets a container that sizes to content stretch blocks to match its widest
  /// sibling, typically using a width measured on the previous frame.
  pub fn code_block_min_width(self, width: Option<f32>) -> Self {
    Self { code_block_min_width: width, ..self }
  }

  /// Run [`crate::heal`] on the text before the parse. Default: `false`.
  ///
  /// Turn this on while the text streams in, for example from an LLM. Heal closes an open code
  /// fence, emphasis, inline code, link or table, and it holds back a marker at the end of the text
  /// until the text after it shows what the marker is. Turn it off when the text is complete: a
  /// complete document that ends in `[x]` heals into a link, because more text could make it one.
  pub fn heal(self, heal: bool) -> Self {
    Self { heal, ..self }
  }

  /// Rewrite the laid-out [`LayoutJob`] just before it is shaped into a galley.
  ///
  /// The parse/layout cache is unchanged; only the painted job is rewritten. Pass `None` to
  /// leave the job as built.
  ///
  /// A map that hides the end of the text (alpha 0, as a reveal does) makes the label allocate only
  /// the rows that show. The hidden rows stay in the layout below the allocation, so the caller must
  /// draw nothing below the label until all of its text shows.
  pub fn map_job<F>(self, f: Option<F>) -> Self
  where
    F: for<'c> Fn(&'c Context, LayoutJob) -> LayoutJob + 'a,
  {
    Self { map_job: f.map(|f| Box::new(f) as _), ..self }
  }

  /// Override visual styling for this widget.
  ///
  /// Without this, the widget uses the context-wide style from
  /// [`egui_markdown_style::global_style`], or [`MarkdownStyle::default`] when no context
  /// style is installed.
  pub fn style(self, style: &'a MarkdownStyle) -> Self {
    Self { style: Some(style), ..self }
  }

  /// Use a custom syntect theme for code block syntax highlighting.
  ///
  /// When set, this theme is used instead of the context-wide themes from
  /// [`crate::set_code_themes`]. When neither is set, a built-in theme is
  /// chosen based on dark/light mode.
  #[cfg(feature = "syntax_highlighting")]
  pub fn code_theme(self, theme: &'a syntect::highlighting::Theme) -> Self {
    Self { code_theme: Some(theme), ..self }
  }

  /// Resolve wrap mode, overflow, width, and row budget for this frame.
  fn resolve_wrap(&self, ui: &Ui) -> ResolvedWrap {
    let mode = self.wrap_mode.unwrap_or_else(|| ui.wrap_mode());
    match mode {
      TextWrapMode::Extend => {
        ResolvedWrap { mode, max_width: f32::INFINITY, break_anywhere: false, max_rows: self.max_lines }
      }
      TextWrapMode::Wrap => ResolvedWrap {
        mode,
        max_width: ui.available_width(),
        break_anywhere: matches!(self.overflow_wrap, OverflowWrap::BreakAll),
        max_rows: self.max_lines,
      },
      TextWrapMode::Truncate => ResolvedWrap {
        mode,
        max_width: ui.available_width(),
        break_anywhere: true,
        max_rows: Some(self.max_lines.unwrap_or(1)),
      },
    }
  }

  /// Re-apply live wrap fields that are deliberately excluded from layout cache keys.
  fn apply_live_wrap(wrap: &ResolvedWrap, job: &mut LayoutJob) {
    job.wrap.max_width = wrap.max_width;
    job.wrap.break_anywhere = wrap.break_anywhere;
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
  pub fn show(self, ui: &mut Ui) -> MarkdownLabelOutput {
    let mut out =
      MarkdownLabelOutput { response: ui.interact(Rect::NOTHING, self.id, Sense::hover()), link_clicked: false };
    self.render(ui, &mut out);
    out
  }

  /// The size of the rendered markdown, measured without painting or interaction.
  ///
  /// This runs the same render as [`Self::show`] in an invisible child [`Ui`], so it counts tables,
  /// blockquotes, images and scrolling code blocks, and it heals the text when [`Self::heal`] is on.
  /// The two calls share their caches, so a `show` after a `calculate_size` in the same frame costs
  /// little. The width is the width of the content, as with [`Self::hug_content`], so a caller can size a
  /// button or a chip to the text. A scrolling code block fills the available width, so a document with
  /// one measures that wide.
  pub fn calculate_size(&self, ui: &mut Ui) -> Vec2 {
    let measure_id = self.id.with("calculate_size");
    let mut out =
      MarkdownLabelOutput { response: ui.interact(Rect::NOTHING, measure_id, Sense::hover()), link_clicked: false };
    let builder = UiBuilder::new().id_salt(measure_id).max_rect(ui.available_rect_before_wrap()).invisible();
    let mut child = ui.new_child(builder);
    self.render(&mut child, &mut out);
    child.min_rect().size()
  }

  /// Lay out the text of the markdown as one galley, allocate its size, and return the galley position,
  /// the galley, and the response. The caller paints the galley.
  ///
  /// The galley holds text only. A table, an image, a block widget and a scrolling code block have no
  /// text to lay out, so they are left out, and a blockquote shows its text without its bar. Use
  /// [`Self::show`] to render those. The text is healed when [`Self::heal`] is on.
  pub fn layout_in_ui(self, ui: &mut Ui) -> (Pos2, Arc<Galley>, Response) {
    let context_style = egui_markdown_style::global_style(ui.ctx());
    let style = self.style.unwrap_or(context_style.as_ref());
    let color = ui.visuals().text_color();
    let text = if self.heal { parser::heal(self.text) } else { std::borrow::Cow::Borrowed(self.text) };
    let md = parser::parse(&text);
    let font = self.font.clone().unwrap_or_else(|| FontSelection::Default.resolve(ui.style()));
    let code_theme = self.code_theme_arg();
    let wrap = self.resolve_wrap(ui);
    let result = build_layout(
      ui,
      &md.tokens,
      font,
      color,
      wrap.max_rows,
      wrap.max_width,
      wrap.break_anywhere,
      self.link_handler,
      false,
      style,
      code_theme,
    );

    let mut job = result.job;
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

  fn render(&self, ui: &mut Ui, out: &mut MarkdownLabelOutput) {
    let context_style = egui_markdown_style::global_style(ui.ctx());
    let style = self.style.unwrap_or(context_style.as_ref());
    let color = ui.visuals().text_color();
    let font = self.font.clone().unwrap_or_else(|| FontSelection::Default.resolve(ui.style()));

    let cache_id = self.id.with("md_cache");
    // Check if we have a cached layout for this text.
    let cached: Option<CachedMarkdownLayout> = ui.data(|d| d.get_temp(cache_id));

    // The text before the last resume point of the previous parse heals to itself, so a streamed reply
    // heals only its end.
    let healed;
    let text = if self.heal {
      healed = match &cached {
        Some(cached) => cached.parse.heal(self.text),
        None => parser::heal(self.text),
      };
      &*healed
    } else {
      self.text
    };

    let text_hash = hash_text(text, style, self.link_handler);
    let frame_key = frame_key(style, self.link_handler, &font, color, ui.visuals().dark_mode);
    let revision =
      |parse: &parser::ParseState| Revision { generation: parse.generation, kept_from: parse.kept_from, frame_key };

    if let Some(ref cached) = cached {
      if cached.text_hash == text_hash {
        let tokens = Arc::clone(&cached.parse.tokens);
        let revision = revision(&cached.parse);

        let Some(layout) = cached.layout.clone() else {
          self.render_segmented(ui, &tokens, &font, color, style, revision, out);
          return;
        };

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
          true,
          out,
        );
        return;
      }
    }

    // Cache miss. Take the cached entry out of the map, so that its tokens have no other owner and the
    // parse can keep them up to its resume point without a copy.
    drop(cached);
    let previous = ui.data_mut(|d| d.remove_temp::<CachedMarkdownLayout>(cache_id)).map(|cached| cached.parse);
    let parse = parser::ParseState::parse(text, previous);
    let tokens = Arc::clone(&parse.tokens);
    let revision = revision(&parse);

    // Decide the render path before laying anything out: a whole-document layout is useless
    // to the segmented path, and building one per keystroke doubles the cost of every edit.
    if needs_segmentation(&tokens, self.scroll_code_blocks, self.link_handler) {
      ui.data_mut(|d| d.insert_temp(cache_id, CachedMarkdownLayout { text_hash, layout: None, parse }));
      self.render_segmented(ui, &tokens, &font, color, style, revision, out);
      return;
    }

    let code_theme = self.code_theme_arg();
    let wrap = self.resolve_wrap(ui);
    let layout = build_layout(
      ui,
      &tokens,
      font.clone(),
      color,
      wrap.max_rows,
      wrap.max_width,
      wrap.break_anywhere,
      self.link_handler,
      self.scroll_code_blocks,
      style,
      code_theme,
    );
    debug_assert!(layout.segment_breaks.is_empty(), "needs_segmentation disagrees with build_layout");
    let layout = Arc::new(layout);

    ui.data_mut(|d| {
      d.insert_temp(cache_id, CachedMarkdownLayout { text_hash, layout: Some(Arc::clone(&layout)), parse });
    });

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
      true,
      out,
    );
  }

  #[allow(clippy::too_many_arguments)]
  fn render_segmented(
    &self,
    ui: &mut Ui,
    tokens: &[Token<'_>],
    font: &FontId,
    color: Color32,
    style: &MarkdownStyle,
    revision: Revision,
    out: &mut MarkdownLabelOutput,
  ) {
    self.render_token_range(ui, tokens, font, color, 0, tokens.len(), style, revision, out);
  }

  /// Recursively render a range of tokens, handling blockquotes and tables as sub-regions.
  #[allow(clippy::too_many_arguments)]
  fn render_token_range(
    &self,
    ui: &mut Ui,
    tokens: &[Token<'_>],
    font: &FontId,
    color: Color32,
    start: usize,
    end: usize,
    style: &MarkdownStyle,
    revision: Revision,
    out: &mut MarkdownLabelOutput,
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
    // After rendering a block element, skip trailing newlines and add uniform spacing —
    // but only when more content follows, mirroring `before_block`, so a trailing block
    // doesn't leave a spacing gap at the bottom.
    let after_block = |i: &mut usize, text_start: &mut usize, end: usize, ui: &mut Ui| {
      while *text_start < end && matches!(tokens[*text_start], Token::Newline) {
        *text_start += 1;
        *i += 1;
      }
      if *text_start < end {
        ui.add_space(block_spacing);
      }
    };

    while i < end {
      match &tokens[i] {
        Token::Table(data) => {
          let had_content = text_start < i;
          self.flush_text_range(ui, tokens, font, color, text_start, i, style, revision, out);
          before_block(had_content, ui);
          let block_sz_id = self.id.with(("block_sz", i));
          if try_cull_block(ui, block_sz_id, revision, i, || hash_block(&tokens[i], style, font)) {
            i += 1;
            text_start = i;
            after_block(&mut i, &mut text_start, end, ui);
            continue;
          }
          let before_y = ui.available_rect_before_wrap().min.y;
          table::render_table(
            ui,
            self.id.with(("table", i)),
            data,
            font,
            color,
            &style.inline_code,
            &style.table,
            self.link_handler,
          );
          cache_block_height(ui, block_sz_id, revision, i, || hash_block(&tokens[i], style, font), before_y);
          i += 1;
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        Token::CodeBlock { text, language } if self.scroll_code_blocks => {
          let had_content = text_start < i;
          self.flush_text_range(ui, tokens, font, color, text_start, i, style, revision, out);
          before_block(had_content, ui);
          let block_sz_id = self.id.with(("block_sz", i));
          if try_cull_block(ui, block_sz_id, revision, i, || hash_block(&tokens[i], style, font)) {
            i += 1;
            text_start = i;
            after_block(&mut i, &mut text_start, end, ui);
            continue;
          }
          let before_y = ui.available_rect_before_wrap().min.y;
          render_code_block(
            ui,
            self.id.with(("code_block", i)),
            text,
            language.as_deref(),
            self.code_block_buttons,
            self.shrink_code_blocks,
            self.code_block_min_width,
            style,
            self.code_theme_arg(),
          );
          cache_block_height(ui, block_sz_id, revision, i, || hash_block(&tokens[i], style, font), before_y);
          i += 1;
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        #[cfg(feature = "images")]
        Token::Image { url, .. } => {
          let had_content = text_start < i;
          self.flush_text_range(ui, tokens, font, color, text_start, i, style, revision, out);
          before_block(had_content, ui);
          let block_sz_id = self.id.with(("block_sz", i));
          if try_cull_block(ui, block_sz_id, revision, i, || hash_block(&tokens[i], style, font)) {
            i += 1;
            text_start = i;
            after_block(&mut i, &mut text_start, end, ui);
            continue;
          }
          let before_y = ui.available_rect_before_wrap().min.y;
          let image = egui::Image::new(url.as_ref()).max_width(ui.available_width()).show_loading_spinner(true);
          ui.add(image);
          cache_block_height(ui, block_sz_id, revision, i, || hash_block(&tokens[i], style, font), before_y);
          i += 1;
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        #[cfg(not(feature = "images"))]
        Token::Image { alt, url, .. } => {
          let had_content = text_start < i;
          self.flush_text_range(ui, tokens, font, color, text_start, i, style, revision, out);
          before_block(had_content, ui);
          let block_sz_id = self.id.with(("block_sz", i));
          if try_cull_block(ui, block_sz_id, revision, i, || hash_block(&tokens[i], style, font)) {
            i += 1;
            text_start = i;
            after_block(&mut i, &mut text_start, end, ui);
            continue;
          }
          let before_y = ui.available_rect_before_wrap().min.y;
          let text = if alt.is_empty() { url.as_ref() } else { alt.as_ref() };
          if ui.link(text).clicked() {
            ui.ctx().open_url(OpenUrl::new_tab(url.to_string()));
          }
          cache_block_height(ui, block_sz_id, revision, i, || hash_block(&tokens[i], style, font), before_y);
          i += 1;
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        Token::Link { text, href, .. } if self.link_handler.is_some_and(|h| h.is_block_widget(href)) => {
          let had_content = text_start < i;
          self.flush_text_range(ui, tokens, font, color, text_start, i, style, revision, out);
          before_block(had_content, ui);
          let handler = self.link_handler.unwrap();
          handler.block_widget(ui, text, href);
          i += 1;
          text_start = i;
          after_block(&mut i, &mut text_start, end, ui);
        }
        Token::BlockquoteStart => {
          let had_content = text_start < i;
          self.flush_text_range(ui, tokens, font, color, text_start, i, style, revision, out);
          before_block(had_content, ui);

          // Find the matching BlockquoteEnd.
          let bq_start = i + 1;
          let mut depth = 1u32;
          let mut j = bq_start;
          while j < end && depth > 0 {
            match &tokens[j] {
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
          while bq_content_start < bq_end && matches!(tokens[bq_content_start], Token::Newline) {
            bq_content_start += 1;
          }

          let indent = style.blockquote.indent_per_depth;
          let mut child_rect = ui.available_rect_before_wrap();
          child_rect.min.x += indent;

          let mut child_ui = ui.new_child(UiBuilder::new().id_salt(self.id.with(("bq", i))).max_rect(child_rect));
          self.render_token_range(&mut child_ui, tokens, font, color, bq_content_start, bq_end, style, revision, out);

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
          self.flush_text_range(ui, tokens, font, color, text_start, i, style, revision, out);
          i += 1;
          text_start = i;
        }
        _ => {
          i += 1;
        }
      }
    }

    // Flush remaining text.
    self.flush_text_range(ui, tokens, font, color, text_start, end, style, revision, out);
  }

  /// Render a range of non-break tokens as an interactive galley.
  #[allow(clippy::too_many_arguments)]
  fn flush_text_range(
    &self,
    ui: &mut Ui,
    tokens: &[Token<'_>],
    font: &FontId,
    color: Color32,
    start: usize,
    end: usize,
    style: &MarkdownStyle,
    revision: Revision,
    out: &mut MarkdownLabelOutput,
  ) {
    if start >= end {
      return;
    }
    // Skip if only newlines.
    if tokens[start..end].iter().all(|t| matches!(t, Token::Newline | Token::BlockquoteStart | Token::BlockquoteEnd)) {
      return;
    }

    // Trim trailing newlines; spacing between blocks is handled by render_token_range.
    let mut trimmed_end = end;
    while trimmed_end > start && matches!(tokens[trimmed_end - 1], Token::Newline) {
      trimmed_end -= 1;
    }
    if trimmed_end <= start {
      return;
    }

    let token_slice = &tokens[start..trimmed_end];
    let apply_map_job = end == tokens.len();
    let wrap = self.resolve_wrap(ui);
    let max_width = wrap.max_width;
    let dark_mode = ui.visuals().dark_mode;
    let cache_id = self.id.with(("flush", start));
    let size_cache_id = self.id.with(("flush_sz", start));
    // While the revision vouches for the tokens of the range, the cached entry knows their hash, and
    // hashing them again on every frame would make each frame cost O(document).
    let cached = ui.data(|d| d.get_temp::<CachedFlushRange>(cache_id));
    let ctx_hash = match &cached {
      Some(cached) if cached.end == trimmed_end && revision.unchanged(cached.generation, cached.frame_key, end) => {
        cached.ctx_hash
      }
      _ => hash_flush_context(token_slice, style, font, color, dark_mode, self.link_handler),
    };
    let (generation, frame_key) = (revision.generation, revision.frame_key);

    // Viewport culling: skip layout+paint for off-screen segments. Unlike the layout, a
    // measured size is only valid at the width it was measured at.
    if let Some((cached_hash, cached_width, cached_size)) = ui.data(|d| d.get_temp::<(u64, f32, Vec2)>(size_cache_id)) {
      if cached_hash == ctx_hash && cached_width == max_width {
        let est_rect = Rect::from_min_size(ui.available_rect_before_wrap().min, cached_size);
        if !ui.is_rect_visible(est_rect) {
          ui.allocate_space(cached_size);
          return;
        }
      }
    }

    if let Some(cached) = cached {
      if cached.ctx_hash == ctx_hash {
        if (cached.generation, cached.frame_key, cached.end) != (generation, frame_key, trimmed_end) {
          let updated = CachedFlushRange { generation, frame_key, end: trimmed_end, ..cached.clone() };
          ui.data_mut(|d| d.insert_temp(cache_id, updated));
        }
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
          apply_map_job,
          out,
        );
        ui.data_mut(|d| d.insert_temp(size_cache_id, (ctx_hash, max_width, size)));
        return;
      }
    }

    let code_theme = self.code_theme_arg();
    // Segmented flush ranges ignore row limits (tables/images/etc. already split the doc).
    let layout = build_layout(
      ui,
      token_slice,
      font.clone(),
      color,
      None,
      wrap.max_width,
      wrap.break_anywhere,
      self.link_handler,
      false,
      style,
      code_theme,
    );
    debug_assert!(layout.segment_breaks.is_empty());
    let owned = Arc::new(tokens_to_owned(token_slice));
    let layout = Arc::new(layout);
    ui.data_mut(|d| {
      d.insert_temp(
        cache_id,
        CachedFlushRange {
          generation,
          frame_key,
          end: trimmed_end,
          ctx_hash,
          layout: Arc::clone(&layout),
          tokens: Arc::clone(&owned),
        },
      )
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
      apply_map_job,
      out,
    );
    ui.data_mut(|d| d.insert_temp(size_cache_id, (ctx_hash, max_width, size)));
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
    inline_widget_spans: &[(usize, usize, usize, FontId)],
    color: Color32,
    style: &MarkdownStyle,
    apply_map_job: bool,
    out: &mut MarkdownLabelOutput,
  ) -> Vec2 {
    let wrap = self.resolve_wrap(ui);
    let mut job = job;
    Self::apply_live_wrap(&wrap, &mut job);
    let mut mapped_sections = None;
    let mut built = Vec::new();
    if apply_map_job {
      if let Some(map_job) = &self.map_job {
        built = job.sections.iter().map(|section| (section.byte_range.as_usize(), section.format.color)).collect();
        job = map_job(ui.ctx(), job);
        mapped_sections = Some(tokens_of_sections(&built, section_to_token, &job.sections));
      }
    }
    let section_to_token = mapped_sections.as_deref().unwrap_or(section_to_token);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    // A rule or an inline widget paints over the galley, so it shows only after the text before it shows.
    let shown = |char_index: usize| built.is_empty() || shown_after_map(&galley.job, &built, char_index);
    let hr_positions: Vec<usize> = hr_positions.iter().copied().filter(|&at| shown(at)).collect();
    let hr_positions = hr_positions.as_slice();
    let painted_widget_spans: Vec<_> = inline_widget_spans.iter().filter(|&&(at, ..)| shown(at)).cloned().collect();
    let code_block_rects = paint::compute_code_block_rects(ui, code_block_spans, &galley);
    let available_width = ui.available_width();
    // Decorations (code block backgrounds, horizontal rules) span the allocated width, so a
    // filling widget must claim the leftover space — otherwise they paint outside the rect.
    // Popovers that hug content skip that expand and paint decorations at the galley width.
    let mut size = galley.size();
    // The hidden text stays in the layout, so a line does not reflow when the next glyph shows. The allocation
    // stops at the last row that shows. A scroll that follows the bottom then moves only for text that shows.
    if let Some(height) = shown_height(&galley, &built) {
      size.y = height;
    }
    // An invisible Ui, such as the one calculate_size measures in, sizes the text to its content, so the
    // measured width is the width of the text and not the width the label would claim.
    let hug_content = self.hug_content || !ui.is_visible();
    if !hug_content && wrap.mode != TextWrapMode::Extend {
      size.x = size.x.max(available_width);
    }
    let decoration_width = if hug_content { size.x } else { available_width };

    // An invisible Ui, such as the one calculate_size measures in, gets no hover, click or selection.
    if !self.interactable || !ui.is_visible() {
      let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
      out.response = out.response.union(response);
      paint_decorations(ui, hr_positions, &galley, &code_block_rects, rect.min, decoration_width, style);
      ui.painter().galley(rect.min, galley.clone(), color);
      if let Some(handler) = self.link_handler {
        paint_inline_widgets(ui, handler, tokens, &painted_widget_spans, &galley, rect.min);
      }
      return size;
    }

    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    paint_decorations(ui, hr_positions, &galley, &code_block_rects, response.rect.min, decoration_width, style);

    if self.selectable {
      LabelSelectionState::label_text_selection(ui, &response, rect.min, galley.clone(), color, Stroke::NONE);
    } else {
      ui.painter().galley(rect.min, galley.clone(), color);
    }

    if let Some(button_fn) = self.code_block_buttons {
      render_code_block_buttons(ui, button_fn, &code_block_rects, code_block_info, &response);
    }

    if let Some(handler) = self.link_handler {
      paint_inline_widgets(ui, handler, tokens, &painted_widget_spans, &galley, response.rect.min);
    }

    let hovered_section =
      self.handle_hover(ui, tokens, &galley, section_to_token, inline_widget_spans, &response, rect, color);

    if self.interactable && response.clicked() {
      out.link_clicked |= self.handle_click(ui, tokens, section_to_token, hovered_section);
    }
    out.response = out.response.union(response);
    size
  }

  #[allow(clippy::too_many_arguments)]
  fn handle_hover(
    &self,
    ui: &mut Ui,
    tokens: &[Token<'_>],
    galley: &Arc<Galley>,
    section_to_token: &[usize],
    inline_widget_spans: &[(usize, usize, usize, FontId)],
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
      token_index.is_some_and(|ti| inline_widget_spans.iter().any(|&(_, _, span_ti, _)| span_ti == ti));

    if is_inline_widget {
      ui.output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);
      return Some(section_index);
    }

    let Some(Token::Link { href, title, .. }) = token else { return Some(section_index) };
    let link_style = self.link_handler.and_then(|handler| handler.link_style(href));
    let underline = link_style.as_ref().is_none_or(|style| style.underline);
    if underline {
      let link_color = link_style.and_then(|style| style.color).unwrap_or(ui.visuals().hyperlink_color);
      let stroke = Stroke::new(1.0_f32, link_color);
      // Find all sections belonging to the same token (for multi-section layout_link).
      let target_token = token_index.unwrap();
      let mut sec_start_char = 0u32;
      for (sec_idx, &tok_idx) in section_to_token.iter().enumerate() {
        let section = &galley.job.sections[sec_idx];
        let sec_char_count = galley.job.text[section.byte_range.as_usize()].chars().count() as u32;
        let sec_end_char = sec_start_char + sec_char_count;
        let start_char = std::mem::replace(&mut sec_start_char, sec_end_char);
        if tok_idx != target_token || sec_end_char <= start_char {
          continue;
        }

        let Some((sg, sr)) = glyph_at_index(galley, start_char) else { continue };
        let Some((eg, er)) = glyph_at_index(galley, sec_end_char.saturating_sub(1)) else { continue };

        for row_index in sr..=er {
          let row = &galley.rows[row_index as usize];
          let row_sg = if row_index == sr { Some(sg) } else { row.glyphs.first() };
          let row_eg = if row_index == er { Some(eg) } else { last_non_whitespace_glyph(row) };
          if let Some((sg, eg)) = row_sg.zip(row_eg) {
            // The row is as tall as the markdown line height, which can be taller than the font. So the
            // underline goes half way into the descent of the font, and not at the bottom of the row.
            let descent = sg.font_height - sg.font_ascent;
            let y = ui.painter().round_to_pixel_center(rect.min.y + row.pos.y + sg.pos.y + descent * 0.5);
            let x = rect.min.x;
            ui.painter().line_segment([pos2(x + sg.pos.x, y), pos2(x + eg.pos.x + eg.advance_width, y)], stroke);
          }
        }
      }
    }
    ui.output_mut(|out| out.cursor_icon = CursorIcon::PointingHand);

    // The hover text of the handler comes first, then the title of the link. The href shows when
    // url_in_tooltip is set, as egui's Hyperlink does.
    if let Some(text) = self.link_handler.and_then(|handler| handler.hover_text(href)) {
      response.clone().on_hover_text_at_pointer(text);
    } else if let Some(title) = title {
      response.clone().on_hover_text_at_pointer(title.as_ref());
    } else if ui.style().url_in_tooltip {
      response.clone().on_hover_text_at_pointer(href.as_ref());
    }

    Some(section_index)
  }

  /// Open the link under a click. Returns true when the click is on a link.
  ///
  /// A healed link has an empty href until its URL arrives, and a click on it opens nothing.
  fn handle_click(
    &self,
    ui: &mut Ui,
    tokens: &[Token<'_>],
    section_to_token: &[usize],
    hovered_section: Option<u32>,
  ) -> bool {
    let section_index = match hovered_section {
      Some(s) => s,
      None => return false,
    };
    let token_index = section_to_token.get(section_index as usize).copied();
    let token = token_index.and_then(|idx| tokens.get(idx));

    let Some(Token::Link { text, href, .. }) = token else { return false };
    if href.is_empty() {
      return true;
    }
    let handled = if let Some(handler) = self.link_handler { handler.click(text, href, ui) } else { false };
    if !handled {
      ui.ctx().open_url(OpenUrl::new_tab(href.to_string()));
    }
    true
  }
}

/// The token of each section in `sections`, after a `map_job` rewrote the sections that the layout built.
///
/// `section_to_token` holds one token for each section in `built`. A map can split a section, so an index into
/// `sections` is not an index into `section_to_token`. Each section takes the token of the built section that
/// holds its first byte. A section that starts after the built text takes `usize::MAX`, which names no token.
fn tokens_of_sections(
  built: &[(std::ops::Range<usize>, Color32)],
  section_to_token: &[usize],
  sections: &[egui::text::LayoutSection],
) -> Vec<usize> {
  sections
    .iter()
    .map(|section| {
      let start = section.byte_range.start.0;
      let index = built.partition_point(|(range, _)| range.end <= start);
      built
        .get(index)
        .filter(|(range, _)| range.start <= start)
        .and_then(|_| section_to_token.get(index).copied())
        .unwrap_or(usize::MAX)
    })
    .collect()
}

/// Whether the character before `char_index` shows after a `map_job`. The map hides a character when it sets
/// the color alpha to 0 and the built job gave that character a color that shows. A placeholder at character 0
/// has no character before it, so it shows.
fn shown_after_map(job: &LayoutJob, built: &[(std::ops::Range<usize>, Color32)], char_index: usize) -> bool {
  let Some(before) = char_index.checked_sub(1) else { return true };
  let Some((byte, _)) = job.text.char_indices().nth(before) else { return true };
  let mapped_alpha = job.sections.iter().find(|s| s.byte_range.as_usize().contains(&byte)).map(|s| s.format.color.a());
  let built_alpha = built.iter().find(|(range, _)| range.contains(&byte)).map(|(_, color)| color.a());
  !(mapped_alpha == Some(0) && built_alpha != Some(0))
}

/// The height down to the bottom of the row that holds the last character that shows, when a `map_job` hides
/// the end of the text. `None` when every character shows.
///
/// A reveal hides a suffix, so the first hidden character ends what shows. A map that hides text in the middle
/// would get a height that is too small.
fn shown_height(galley: &Galley, built: &[(std::ops::Range<usize>, Color32)]) -> Option<f32> {
  let job = &galley.job;
  let hidden = job.sections.iter().find(|section| {
    let start = section.byte_range.start.0;
    let built_alpha = built.iter().find(|(range, _)| range.contains(&start)).map(|(_, color)| color.a());
    !section.byte_range.is_empty() && section.format.color.a() == 0 && built_alpha.is_some_and(|alpha| alpha != 0)
  })?;
  let first_hidden = job.text[..hidden.byte_range.start.0].chars().count();
  let Some(last_shown) = first_hidden.checked_sub(1) else { return Some(0.0) };
  let cursor = CCursor { index: CharIndex(last_shown), prefer_next_row: true };
  Some(galley.pos_from_cursor(cursor).max.y.min(galley.size().y))
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
  spans: &[(usize, usize, usize, FontId)],
  galley: &Arc<Galley>,
  origin: Pos2,
) {
  for (start_char, end_char, token_index, font) in spans {
    let (text, href) = match tokens.get(*token_index) {
      Some(Token::Link { text, href, .. }) => (text.as_ref(), href.as_ref()),
      _ => continue,
    };

    let rows = inline_widget_rows(galley, origin, *start_char..*end_char);
    if rows.is_empty() {
      continue;
    }
    ui.push_id(token_index, |ui| {
      handler.paint_inline_widget(ui, text, href, font, &rows);
    });
  }
}

/// The rows that the characters `chars` of `galley` occupy, with `origin` as the position of the galley.
pub(crate) fn inline_widget_rows(galley: &Galley, origin: Pos2, chars: std::ops::Range<usize>) -> Vec<InlineWidgetRow> {
  let mut rows = Vec::new();
  let mut row_start = 0;
  for row in &galley.rows {
    let row_chars = row_start..row_start + row.glyphs.len();
    row_start += row.char_count_including_newline().0;
    let start = chars.start.max(row_chars.start);
    let end = chars.end.min(row_chars.end);
    if start >= end {
      continue;
    }
    let first = &row.glyphs[start - row_chars.start];
    let last = &row.glyphs[end - 1 - row_chars.start];
    let rect = Rect::from_min_max(pos2(first.pos.x, row.min_y()), pos2(last.max_x(), row.max_y()));
    rows.push(InlineWidgetRow { rect: rect.translate(origin.to_vec2()), chars: start - chars.start..end - chars.start });
  }
  rows
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn render_code_block(
  ui: &mut Ui,
  id: Id,
  text: &pulldown_cmark::CowStr<'_>,
  language: Option<&str>,
  code_block_buttons: Option<&dyn Fn(&mut Ui, &str, &str)>,
  shrink_to_content: bool,
  min_width: Option<f32>,
  style: &MarkdownStyle,
  code_theme: CodeThemeArg<'_>,
) {
  let lang = language.unwrap_or(style.default_code_language.as_str());
  let cache_id = id.with("code_galley");

  #[cfg(feature = "syntax_highlighting")]
  let galley = {
    let identity_hash = hash_code_block_identity(
      ui,
      lang,
      style.code_font_size,
      ui.visuals().dark_mode,
      ui.ctx().pixels_per_point(),
      code_theme,
    );
    let cached = ui.data(|d| d.get_temp::<StreamingCodeCache>(cache_id));
    let updated =
      scrolling_code_galley(ui, text.as_ref(), lang, style.code_font_size, identity_hash, code_theme, cached);
    let galley = Arc::clone(&updated.galley);
    ui.data_mut(|d| d.insert_temp(cache_id, updated));
    galley
  };

  #[cfg(not(feature = "syntax_highlighting"))]
  let galley = {
    let ctx_hash = hash_code_block_context(
      text.as_ref(),
      lang,
      style.code_font_size,
      ui.visuals().dark_mode,
      ui.ctx().pixels_per_point(),
      code_theme,
    );
    match ui.data(|d| d.get_temp::<CachedCodeBlock>(cache_id)) {
      Some(cached) if cached.ctx_hash == ctx_hash => cached.galley,
      _ => {
        let galley = build_code_block_galley(ui, text.as_ref(), lang, style.code_font_size, code_theme);
        ui.data_mut(|d| {
          d.insert_temp(cache_id, CachedCodeBlock { ctx_hash, galley: Arc::clone(&galley) });
        });
        galley
      }
    }
  };

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
    if !shrink_to_content {
      ui.set_min_width(ui.available_width());
    } else if let Some(width) = min_width {
      // `width` is the desired outer width; we're inside the frame's margins.
      ui.set_min_width((width - p[0] - p[2]).min(ui.available_width()));
    }
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

#[cfg(not(feature = "syntax_highlighting"))]
fn build_code_block_galley(
  ui: &mut Ui,
  text: &str,
  lang: &str,
  code_font_size: f32,
  code_theme: CodeThemeArg<'_>,
) -> Arc<Galley> {
  let padded_text = crate::layout::pad_code_body(text);
  let mut job = highlight_code(ui, &padded_text, lang, code_font_size, code_theme);
  job.wrap.max_width = f32::INFINITY;
  ui.fonts_mut(|f| f.layout_job(job))
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
    index += row.char_count_including_newline().0 as u32;
  }
  None
}

/// Finds the glyph and row_index from the given galley index.
pub fn glyph_at_index(galley: &Galley, index: u32) -> Option<(&Glyph, u32)> {
  let mut offset = 0;
  for (row_index, row) in galley.rows.iter().enumerate() {
    if index < offset + row.char_count_including_newline().0 as u32 {
      return row.glyphs.get((index - offset) as usize).map(|glyph| (glyph, row_index as u32));
    }
    offset += row.char_count_including_newline().0 as u32;
  }
  None
}

/// Helpful to avoid painting underlines for trailing whitespace.
pub fn last_non_whitespace_glyph(row: &Row) -> Option<&Glyph> {
  row.glyphs.iter().rfind(|glyph| !glyph.chr.is_whitespace())
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A block's cached height stays valid while text elsewhere in the document changes, as it does on
  /// every delta of a streamed reply, so the block stays culled. A change to the block itself drops it.
  #[test]
  fn a_block_height_outlives_changes_elsewhere_in_the_document() {
    let mut checked = false;
    egui::__run_test_ui(|ui| {
      let id = Id::NULL.with("block");
      let revision = |generation, kept_from| Revision { generation, kept_from, frame_key: 7 };
      ui.data_mut(|d| d.insert_temp(id, BlockHeight { generation: 1, frame_key: 7, block_hash: 10, height: 42.0 }));
      let no_hash = || panic!("the revision vouches for the block");

      assert_eq!(cached_block_height(ui, id, revision(1, None), 5, no_hash), Some(42.0), "nothing was parsed");
      // Parse 2 kept the first 20 tokens of parse 1, and the block is token 5.
      assert_eq!(cached_block_height(ui, id, revision(2, Some((1, 20))), 5, no_hash), Some(42.0));
      // Parse 3 kept only 3 tokens, so the label hashes the block, which did not change.
      assert_eq!(cached_block_height(ui, id, revision(3, Some((2, 3))), 5, || 10), Some(42.0));
      assert_eq!(cached_block_height(ui, id, revision(4, None), 5, || 11), None, "the block changed");
      let other_style = Revision { frame_key: 8, ..revision(3, None) };
      assert_eq!(cached_block_height(ui, id, other_style, 5, || 10), None, "the style changed");
      checked = true;
    });
    assert!(checked);
  }
}

//! Customizable visual styling for all markdown elements.

use std::hash::{Hash, Hasher};

use egui::{self, Color32, DragValue, Grid, Ui};

/// Visual styling for markdown rendering.
///
/// All fields have sensible defaults matching the previously hardcoded values.
/// Dark/light theme adaptation is automatic via `InlineCodeStyle`'s per-theme color
/// fields and egui's `Visuals::dark_mode`.
///
/// # Example
///
/// ```no_run
/// # use eframe::egui;
/// use egui_markdown::{MarkdownLabel, MarkdownStyle};
///
/// fn show(ui: &mut egui::Ui) {
///     let mut style = MarkdownStyle::default();
///     style.heading.scales[0] = 2.0; // Bigger H1
///     MarkdownLabel::new(ui.id().with("md"), "**Hello**")
///         .style(&style)
///         .show(ui);
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownStyle {
  /// Styling for inline code spans.
  pub inline_code: InlineCodeStyle,
  /// Styling for fenced code blocks.
  pub code_block: CodeBlockStyle,
  /// Styling for heading levels 1–6.
  pub heading: HeadingStyle,
  /// Styling for horizontal rules.
  pub horizontal_rule: HorizontalRuleStyle,
  /// Styling for blockquotes.
  pub blockquote: BlockquoteStyle,
  /// Vertical spacing between block elements in pixels.
  pub block_spacing: f32,
  /// Font size for code blocks. Default: `10.0`.
  pub code_font_size: f32,
  /// Language used for syntax highlighting when no language is specified.
  pub default_code_language: String,
}

impl Default for MarkdownStyle {
  fn default() -> Self {
    Self {
      inline_code: InlineCodeStyle::default(),
      code_block: CodeBlockStyle::default(),
      heading: HeadingStyle::default(),
      horizontal_rule: HorizontalRuleStyle::default(),
      blockquote: BlockquoteStyle::default(),
      block_spacing: 8.0,
      code_font_size: 10.0,
      default_code_language: String::new(),
    }
  }
}

impl Hash for MarkdownStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.inline_code.hash(state);
    self.code_block.hash(state);
    self.heading.hash(state);
    self.horizontal_rule.hash(state);
    self.blockquote.hash(state);
    self.block_spacing.to_bits().hash(state);
    self.code_font_size.to_bits().hash(state);
    self.default_code_language.hash(state);
  }
}

impl MarkdownStyle {
  /// Show an interactive editor for all style fields.
  pub fn ui(&mut self, ui: &mut Ui) {
    ui.horizontal(|ui| {
      let dark_mode = ui.visuals().dark_mode;
      if ui.selectable_label(dark_mode, "Dark").clicked() {
        ui.ctx().set_visuals(egui::Visuals::dark());
      }
      if ui.selectable_label(!dark_mode, "Light").clicked() {
        ui.ctx().set_visuals(egui::Visuals::light());
      }
      ui.separator();
      if ui.button("Reset").clicked() {
        *self = Self::default();
      }
    });

    ui.separator();

    ui.label("Block spacing:");
    ui.add(DragValue::new(&mut self.block_spacing).range(0.0..=40.0).speed(0.5));

    ui.separator();

    egui::CollapsingHeader::new("Inline Code").default_open(true).show(ui, |ui| {
      self.inline_code.ui(ui);
    });

    egui::CollapsingHeader::new("Code Blocks").default_open(true).show(ui, |ui| {
      self.code_block.ui(ui);
      ui.separator();
      ui.horizontal(|ui| {
        ui.label("Font size:");
        ui.add(DragValue::new(&mut self.code_font_size).range(6.0..=30.0).speed(0.5));
      });
    });

    egui::CollapsingHeader::new("Headings").default_open(true).show(ui, |ui| {
      self.heading.ui(ui);
    });

    egui::CollapsingHeader::new("Horizontal Rules").default_open(false).show(ui, |ui| {
      self.horizontal_rule.ui(ui);
    });

    egui::CollapsingHeader::new("Blockquotes").default_open(false).show(ui, |ui| {
      self.blockquote.ui(ui);
    });
  }
}

/// Styling for inline code spans (backtick-delimited).
#[derive(Clone, Debug, PartialEq)]
pub struct InlineCodeStyle {
  /// Text color in dark mode.
  pub color_dark: Color32,
  /// Text color in light mode.
  pub color_light: Color32,
  /// Background color in dark mode.
  pub background_dark: Color32,
  /// Background color in light mode.
  pub background_light: Color32,
  /// How much to expand the background rectangle horizontally beyond the text bounds (in pixels).
  pub expand_bg: f32,
  /// How much to expand the background rectangle vertically (requires `membrane` feature).
  /// When not set or without `membrane`, falls back to `expand_bg` for both axes.
  #[cfg(feature = "membrane")]
  pub expand_bg_y: f32,
  /// Corner radius for inline code backgrounds (requires `membrane` feature).
  #[cfg(feature = "membrane")]
  pub bg_corner_radius: u8,
}

impl Default for InlineCodeStyle {
  fn default() -> Self {
    Self {
      color_dark: Color32::from_rgb(255, 152, 0),
      color_light: Color32::from_rgb(204, 102, 0),
      background_dark: Color32::from_gray(50),
      background_light: Color32::from_gray(225),
      expand_bg: 3.0,
      #[cfg(feature = "membrane")]
      expand_bg_y: 1.0,
      #[cfg(feature = "membrane")]
      bg_corner_radius: 3,
    }
  }
}

impl Hash for InlineCodeStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.color_dark.hash(state);
    self.color_light.hash(state);
    self.background_dark.hash(state);
    self.background_light.hash(state);
    self.expand_bg.to_bits().hash(state);
  }
}

impl InlineCodeStyle {
  /// Resolve color for the current theme.
  pub fn color(&self, dark_mode: bool) -> Color32 {
    if dark_mode {
      self.color_dark
    } else {
      self.color_light
    }
  }

  /// Resolve background for the current theme.
  pub fn background(&self, dark_mode: bool) -> Color32 {
    if dark_mode {
      self.background_dark
    } else {
      self.background_light
    }
  }

  fn ui(&mut self, ui: &mut Ui) {
    Grid::new("inline_code_style").num_columns(2).striped(true).show(ui, |ui| {
      ui.label("Color (dark):");
      ui.color_edit_button_srgba(&mut self.color_dark);
      ui.end_row();

      ui.label("Color (light):");
      ui.color_edit_button_srgba(&mut self.color_light);
      ui.end_row();

      ui.label("Background (dark):");
      ui.color_edit_button_srgba(&mut self.background_dark);
      ui.end_row();

      ui.label("Background (light):");
      ui.color_edit_button_srgba(&mut self.background_light);
      ui.end_row();

      ui.label("Expand bg:");
      ui.add(DragValue::new(&mut self.expand_bg).range(0.0..=10.0).speed(0.1));
      ui.end_row();
    });
  }
}

/// Styling for fenced code blocks.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeBlockStyle {
  /// Padding `[left, top, right, bottom]`.
  pub padding: [f32; 4],
  /// Corner radius for the code block border.
  pub corner_radius: f32,
  /// Stroke width for the code block border.
  pub stroke_width: f32,
}

impl Default for CodeBlockStyle {
  fn default() -> Self {
    Self { padding: [4.0, 6.0, 12.0, 6.0], corner_radius: 3.0, stroke_width: 1.0 }
  }
}

impl Hash for CodeBlockStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for v in &self.padding {
      v.to_bits().hash(state);
    }
    self.corner_radius.to_bits().hash(state);
    self.stroke_width.to_bits().hash(state);
  }
}

impl CodeBlockStyle {
  fn ui(&mut self, ui: &mut Ui) {
    Grid::new("code_block_style").num_columns(2).striped(true).show(ui, |ui| {
      ui.label("Padding left:");
      ui.add(DragValue::new(&mut self.padding[0]).range(0.0..=30.0).speed(0.5));
      ui.end_row();

      ui.label("Padding top:");
      ui.add(DragValue::new(&mut self.padding[1]).range(0.0..=30.0).speed(0.5));
      ui.end_row();

      ui.label("Padding right:");
      ui.add(DragValue::new(&mut self.padding[2]).range(0.0..=30.0).speed(0.5));
      ui.end_row();

      ui.label("Padding bottom:");
      ui.add(DragValue::new(&mut self.padding[3]).range(0.0..=30.0).speed(0.5));
      ui.end_row();

      ui.label("Corner radius:");
      ui.add(DragValue::new(&mut self.corner_radius).range(0.0..=20.0).speed(0.5));
      ui.end_row();

      ui.label("Stroke width:");
      ui.add(DragValue::new(&mut self.stroke_width).range(0.0..=5.0).speed(0.1));
      ui.end_row();
    });
  }
}

/// Styling for heading levels 1–6.
#[derive(Clone, Debug, PartialEq)]
pub struct HeadingStyle {
  /// Font size multipliers for H1–H6.
  pub scales: [f32; 6],
}

impl Default for HeadingStyle {
  fn default() -> Self {
    Self { scales: [1.6, 1.35, 1.2, 1.1, 1.05, 1.0] }
  }
}

impl Hash for HeadingStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for v in &self.scales {
      v.to_bits().hash(state);
    }
  }
}

impl HeadingStyle {
  fn ui(&mut self, ui: &mut Ui) {
    Grid::new("heading_style").num_columns(2).striped(true).show(ui, |ui| {
      for (i, scale) in self.scales.iter_mut().enumerate() {
        ui.label(format!("H{}:", i + 1));
        ui.add(DragValue::new(scale).range(0.5..=4.0).speed(0.01));
        ui.end_row();
      }
    });
  }
}

/// Styling for horizontal rules (`---`).
#[derive(Clone, Debug, PartialEq)]
pub struct HorizontalRuleStyle {
  /// Stroke width for horizontal rule lines.
  pub stroke_width: f32,
  /// Vertical space reserved for the rule row (points).
  pub height: f32,
}

impl Default for HorizontalRuleStyle {
  fn default() -> Self {
    Self {
      stroke_width: 1.0,
      height: 8.0,
    }
  }
}

impl Hash for HorizontalRuleStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.stroke_width.to_bits().hash(state);
    self.height.to_bits().hash(state);
  }
}

impl HorizontalRuleStyle {
  fn ui(&mut self, ui: &mut Ui) {
    Grid::new("hr_style").num_columns(2).striped(true).show(ui, |ui| {
      ui.label("Stroke width:");
      ui.add(DragValue::new(&mut self.stroke_width).range(0.0..=5.0).speed(0.1));
      ui.end_row();
      ui.label("Height:");
      ui.add(DragValue::new(&mut self.height).range(1.0..=40.0).speed(0.5));
      ui.end_row();
    });
  }
}

/// Styling for blockquotes.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockquoteStyle {
  /// Horizontal indent per nesting depth in pixels.
  pub indent_per_depth: f32,
  /// Width of the vertical bar drawn at the left edge of a blockquote.
  pub stroke_width: f32,
}

impl Default for BlockquoteStyle {
  fn default() -> Self {
    Self { indent_per_depth: 12.0, stroke_width: 1.0 }
  }
}

impl Hash for BlockquoteStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.indent_per_depth.to_bits().hash(state);
    self.stroke_width.to_bits().hash(state);
  }
}

impl BlockquoteStyle {
  fn ui(&mut self, ui: &mut Ui) {
    Grid::new("blockquote_style").num_columns(2).striped(true).show(ui, |ui| {
      ui.label("Indent per depth:");
      ui.add(DragValue::new(&mut self.indent_per_depth).range(0.0..=40.0).speed(0.5));
      ui.end_row();

      ui.label("Stroke width:");
      ui.add(DragValue::new(&mut self.stroke_width).range(0.0..=5.0).speed(0.1));
      ui.end_row();
    });
  }
}

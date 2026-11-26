//! The style of each markdown element.

use std::hash::{Hash, Hasher};

use egui::{self, Color32, DragValue, Grid, Ui};

/// The style of the markdown elements of a label.
///
/// Most colors come from the egui [`egui::Visuals`] of the `Ui`, so they follow dark and light mode. Inline
/// code has a color and a background for each mode in [`InlineCodeStyle`].
///
/// [`crate::set_style`] installs a style for all labels of a context. A label without its own style uses
/// it.
///
/// # Example
///
/// ```
/// use egui_markdown_style::MarkdownStyle;
///
/// let mut style = MarkdownStyle::default();
/// style.heading.scales[0] = 2.0; // Bigger H1
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MarkdownStyle {
  /// Styling for inline code spans.
  pub inline_code: InlineCodeStyle,
  /// Styling for fenced code blocks.
  pub code_block: CodeBlockStyle,
  /// Styling for heading levels 1 to 6.
  pub heading: HeadingStyle,
  /// Styling for horizontal rules.
  pub horizontal_rule: HorizontalRuleStyle,
  /// Styling for blockquotes.
  pub blockquote: BlockquoteStyle,
  /// Styling for list markers (bullets and numbers).
  pub list: ListStyle,
  /// Styling for tables.
  #[cfg_attr(feature = "serde", serde(default))]
  pub table: TableStyle,
  /// Vertical spacing between block elements in pixels.
  pub block_spacing: f32,
  /// The height of a row of body text divided by the label font size. The default is 17/13, which gives
  /// 17 point rows for the 13 point Body font of the default egui style. A row is never shorter than the
  /// row height of its tallest font, such as the font of a heading.
  #[cfg_attr(feature = "serde", serde(default = "default_line_height_ratio"))]
  pub line_height_ratio: f32,
  /// Font size for code blocks. `None`, the default, uses the size of [`egui::TextStyle::Monospace`]
  /// in the egui style, as `ui.code()` does. `Some(size)` sets a fixed size.
  pub code_font_size: Option<f32>,
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
      list: ListStyle::default(),
      table: TableStyle::default(),
      block_spacing: 8.0,
      line_height_ratio: default_line_height_ratio(),
      code_font_size: None,
      default_code_language: String::new(),
    }
  }
}

fn default_line_height_ratio() -> f32 {
  17.0 / 13.0
}

// `f32` has no `Hash`, so each style type hashes the bits of its floats. A label uses the hash in its cache
// keys.
impl Hash for MarkdownStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.inline_code.hash(state);
    self.code_block.hash(state);
    self.heading.hash(state);
    self.horizontal_rule.hash(state);
    self.blockquote.hash(state);
    self.list.hash(state);
    self.table.hash(state);
    self.block_spacing.to_bits().hash(state);
    self.line_height_ratio.to_bits().hash(state);
    self.code_font_size.map(f32::to_bits).hash(state);
    self.default_code_language.hash(state);
  }
}

impl MarkdownStyle {
  /// The font size of code blocks: [`Self::code_font_size`], or the size of
  /// [`egui::TextStyle::Monospace`] in `style` when it is `None`.
  pub fn code_font_size(&self, style: &egui::Style) -> f32 {
    self.code_font_size.unwrap_or_else(|| egui::TextStyle::Monospace.resolve(style).size)
  }

  /// Show an editor for each field of the style, and a button that resets the style.
  pub fn ui(&mut self, ui: &mut Ui) {
    if ui.button("Reset").clicked() {
      *self = Self::default();
    }

    ui.separator();
    ui.label("Block spacing:");
    ui.add(DragValue::new(&mut self.block_spacing).range(0.0..=40.0).speed(0.5));
    ui.label("Line height ratio:");
    ui.add(DragValue::new(&mut self.line_height_ratio).range(1.0..=3.0).speed(0.01));

    ui.separator();

    egui::CollapsingHeader::new("Inline Code").default_open(true).show(ui, |ui| {
      self.inline_code.ui(ui);
    });

    egui::CollapsingHeader::new("Code Blocks").default_open(true).show(ui, |ui| {
      self.code_block.ui(ui);
      ui.separator();
      let mut use_egui_size = self.code_font_size.is_none();
      if ui.checkbox(&mut use_egui_size, "Use egui's monospace size").changed() {
        self.code_font_size = (!use_egui_size).then(|| self.code_font_size(ui.style()));
      }
      if let Some(size) = &mut self.code_font_size {
        ui.horizontal(|ui| {
          ui.label("Font size:");
          ui.add(DragValue::new(size).range(6.0..=30.0).speed(0.5));
        });
      }
      ui.horizontal(|ui| {
        ui.label("Default language:");
        ui.add(egui::TextEdit::singleline(&mut self.default_code_language).desired_width(80.0));
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

    egui::CollapsingHeader::new("Lists").default_open(false).show(ui, |ui| {
      self.list.ui(ui);
    });

    egui::CollapsingHeader::new("Tables").default_open(false).show(ui, |ui| {
      self.table.ui(ui);
    });
  }
}

/// The style of inline code, the text between backticks.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InlineCodeStyle {
  /// Text color in dark mode.
  pub color_dark: Color32,
  /// Text color in light mode.
  pub color_light: Color32,
  /// Background color in dark mode.
  pub background_dark: Color32,
  /// Background color in light mode.
  pub background_light: Color32,
  /// How far the background extends past the text on each side, in points. It does not change the layout,
  /// so the background can cover the text next to it.
  pub expand_bg: f32,
}

impl Default for InlineCodeStyle {
  fn default() -> Self {
    Self {
      color_dark: Color32::from_rgb(255, 152, 0),
      color_light: Color32::from_rgb(204, 102, 0),
      background_dark: Color32::from_gray(50),
      background_light: Color32::from_gray(225),
      expand_bg: 3.0,
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
  /// The text color for dark mode or for light mode.
  pub fn color(&self, dark_mode: bool) -> Color32 {
    if dark_mode {
      self.color_dark
    } else {
      self.color_light
    }
  }

  /// The background color for dark mode or for light mode.
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

/// The style of code blocks. The fill and the corner radius come from [`egui::Visuals::code`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CodeBlockStyle {
  /// The space between the code and the border, `[left, top, right, bottom]`, in points.
  pub padding: [f32; 4],
  /// The width of the border, in the color of `Visuals::widgets.noninteractive.bg_stroke`.
  pub stroke_width: f32,
}

impl Default for CodeBlockStyle {
  fn default() -> Self {
    Self { padding: [4.0, 6.0, 12.0, 6.0], stroke_width: 1.0 }
  }
}

impl Hash for CodeBlockStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    for v in &self.padding {
      v.to_bits().hash(state);
    }
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

      ui.label("Stroke width:");
      ui.add(DragValue::new(&mut self.stroke_width).range(0.0..=5.0).speed(0.1));
      ui.end_row();
    });
  }
}

/// The style of headings.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HeadingStyle {
  /// The font size of each heading level, from H1 to H6, as a multiple of the label font size.
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

/// The style of horizontal rules (`---`).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HorizontalRuleStyle {
  /// The width of the line.
  pub stroke_width: f32,
  /// The height of the row of the rule, in points.
  pub height: f32,
}

impl Default for HorizontalRuleStyle {
  fn default() -> Self {
    Self { stroke_width: 1.0, height: 8.0 }
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

/// The style of blockquotes.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockquoteStyle {
  /// The indent of each nesting level, in points. The bar is in the middle of the indent.
  pub indent_per_depth: f32,
  /// The width of the bar at the left of a blockquote.
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

/// The style of list markers.
///
/// A marker is right-aligned in a slot with a width from the body font, so each item of a nesting level
/// starts its text at the same x. `gap` moves the text, and its wrapped rows, away from the marker. The two
/// `nudge` fields move a marker to the left, and do not move the text.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListStyle {
  /// Space between the marker column and the item text, in points.
  pub gap: f32,
  /// How far left of the marker column to draw a bullet, in points.
  pub bullet_nudge: f32,
  /// How far left of the marker column to draw a number, in points.
  pub number_nudge: f32,
  /// The size of a bullet, as a multiple of the font size. It does not change the row height.
  pub bullet_scale: f32,
}

impl Default for ListStyle {
  fn default() -> Self {
    Self { gap: 0.0, bullet_nudge: 0.0, number_nudge: 0.0, bullet_scale: 1.0 }
  }
}

impl Hash for ListStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.gap.to_bits().hash(state);
    self.bullet_nudge.to_bits().hash(state);
    self.number_nudge.to_bits().hash(state);
    self.bullet_scale.to_bits().hash(state);
  }
}

fn default_cell_padding() -> [f32; 4] {
  [10.0, 6.0, 10.0, 6.0]
}

/// The style of tables.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableStyle {
  /// The width of the lines between the cells and around the table. `0.0` draws no line.
  pub stroke_width: f32,
  /// The corner radius of the line around the table. It shows only when [`Self::stroke_width`] is not zero.
  pub corner_radius: f32,
  /// The space inside each cell, `[left, top, right, bottom]`, in points.
  #[cfg_attr(feature = "serde", serde(default = "default_cell_padding"))]
  pub cell_padding: [f32; 4],
}

impl Default for TableStyle {
  fn default() -> Self {
    Self { stroke_width: 0.0, corner_radius: 0.0, cell_padding: default_cell_padding() }
  }
}

impl Hash for TableStyle {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.stroke_width.to_bits().hash(state);
    self.corner_radius.to_bits().hash(state);
    for v in &self.cell_padding {
      v.to_bits().hash(state);
    }
  }
}

impl TableStyle {
  fn ui(&mut self, ui: &mut Ui) {
    Grid::new("table_style").num_columns(2).striped(true).show(ui, |ui| {
      ui.label("Stroke width:");
      ui.add(DragValue::new(&mut self.stroke_width).range(0.0..=5.0).speed(0.1));
      ui.end_row();

      ui.label("Corner radius:");
      ui.add(DragValue::new(&mut self.corner_radius).range(0.0..=20.0).speed(0.5));
      ui.end_row();

      ui.label("Cell padding left:");
      ui.add(DragValue::new(&mut self.cell_padding[0]).range(0.0..=30.0).speed(0.5));
      ui.end_row();

      ui.label("Cell padding top:");
      ui.add(DragValue::new(&mut self.cell_padding[1]).range(0.0..=30.0).speed(0.5));
      ui.end_row();

      ui.label("Cell padding right:");
      ui.add(DragValue::new(&mut self.cell_padding[2]).range(0.0..=30.0).speed(0.5));
      ui.end_row();

      ui.label("Cell padding bottom:");
      ui.add(DragValue::new(&mut self.cell_padding[3]).range(0.0..=30.0).speed(0.5));
      ui.end_row();
    });
  }
}

impl ListStyle {
  fn ui(&mut self, ui: &mut Ui) {
    Grid::new("list_style").num_columns(2).striped(true).show(ui, |ui| {
      ui.label("Marker gap:");
      ui.add(DragValue::new(&mut self.gap).range(0.0..=40.0).speed(0.25));
      ui.end_row();

      ui.label("Bullet nudge:");
      ui.add(DragValue::new(&mut self.bullet_nudge).range(0.0..=40.0).speed(0.25));
      ui.end_row();

      ui.label("Number nudge:");
      ui.add(DragValue::new(&mut self.number_nudge).range(0.0..=40.0).speed(0.25));
      ui.end_row();

      ui.label("Bullet scale:");
      ui.add(DragValue::new(&mut self.bullet_scale).range(0.5..=4.0).speed(0.05));
      ui.end_row();
    });
  }
}

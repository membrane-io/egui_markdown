//! The font size of inline code and code blocks: the code block size follows the label font and the
//! ratio of `TextStyle::Monospace` to `TextStyle::Body`, or `MarkdownStyle::code_font_size`, and inline
//! code is to the text around it as the code block is to the label font.

use egui::epaint::Shape;
use egui::{pos2, vec2, Context, FontId, Id, RawInput, Rect, TextStyle, UiBuilder};
use egui_markdown::{MarkdownLabel, MarkdownStyle};

/// The ways the label can render a code block: in the galley of the text, in the galley of a range of
/// text after a blockquote, and as a scrolling code block.
const PATHS: [(&str, bool); 3] = [("", false), ("> Quote\n\n", false), ("", true)];

/// Set the size of `TextStyle::Body` and `TextStyle::Monospace` in every style of `ctx`.
fn set_text_sizes(ctx: &Context, body: f32, monospace: f32) {
  ctx.all_styles_mut(|style| {
    style.text_styles.insert(TextStyle::Body, FontId::proportional(body));
    style.text_styles.insert(TextStyle::Monospace, FontId::monospace(monospace));
  });
}

/// Render one frame of a label and return the text and the font size of each painted text section.
fn render(
  ctx: &Context,
  markdown: &str,
  font: Option<FontId>,
  style: &MarkdownStyle,
  scroll_code_blocks: bool,
) -> Vec<(String, f32)> {
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 800.0));
  let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    let mut label = MarkdownLabel::new(Id::NULL.with("code_size"), markdown).style(style);
    if let Some(font) = font.clone() {
      label = label.font(font);
    }
    label.scroll_code_blocks(scroll_code_blocks).show(&mut child);
  });
  output.textures_delta.clear();

  fn collect(shape: &Shape, sections: &mut Vec<(String, f32)>) {
    match shape {
      Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, sections)),
      Shape::Text(text) => {
        let job = &text.galley.job;
        for section in &job.sections {
          let range = section.byte_range.start.0..section.byte_range.end.0;
          sections.push((job.text[range].to_owned(), section.format.font_id.size));
        }
      }
      _ => {}
    }
  }
  let mut sections = Vec::new();
  output.shapes.iter().for_each(|clipped| collect(&clipped.shape, &mut sections));
  sections
}

/// The font size of the first painted section that contains `text`.
fn size_of(sections: &[(String, f32)], text: &str) -> f32 {
  let found = sections.iter().find(|(section, _)| section.contains(text));
  found.unwrap_or_else(|| panic!("no painted section contains {text:?}: {sections:?}")).1
}

fn assert_size(sections: &[(String, f32)], text: &str, expected: f32) {
  let size = size_of(sections, text);
  assert!((size - expected).abs() < 1e-3, "{text:?} has the size {size}, not {expected}");
}

const DOCUMENT: &str = "# Title `heading`\n\nText `inline`\n\n```\nfn main() {}\n```\n";

/// With `Monospace` smaller than `Body` in the egui style, inline code and code blocks are smaller than
/// the text by the same ratio, also in a heading.
#[test]
fn inline_code_follows_the_monospace_ratio() {
  let ctx = Context::default();
  set_text_sizes(&ctx, 13.0, 11.0);
  let style = MarkdownStyle::default();
  let heading = 13.0 * style.heading.scales[0];
  for (prefix, scroll) in PATHS {
    let sections = render(&ctx, &format!("{prefix}{DOCUMENT}"), None, &style, scroll);
    assert_size(&sections, "Title", heading);
    assert_size(&sections, "heading", heading * 11.0 / 13.0);
    assert_size(&sections, "inline", 11.0);
    assert_size(&sections, "main", 11.0);
  }
}

/// `Some(size)` sets the size of code blocks, and inline code in body text has the same size. Inline code
/// in a heading is larger by the heading scale.
#[test]
fn inline_code_follows_a_fixed_code_size() {
  let ctx = Context::default();
  let style = MarkdownStyle { code_font_size: Some(11.0), ..Default::default() };
  let heading_scale = style.heading.scales[0];
  for font in [None, Some(FontId::proportional(20.0))] {
    for (prefix, scroll) in PATHS {
      let sections = render(&ctx, &format!("{prefix}{DOCUMENT}"), font.clone(), &style, scroll);
      assert_size(&sections, "heading", 11.0 * heading_scale);
      assert_size(&sections, "inline", 11.0);
      assert_size(&sections, "main", 11.0);
    }
  }
}

/// With `None`, a code block in a label with its own font has the size of that font, as inline code
/// has.
#[test]
fn code_block_follows_the_label_font() {
  let ctx = Context::default();
  let style = MarkdownStyle::default();
  for (prefix, scroll) in PATHS {
    let sections = render(&ctx, &format!("{prefix}{DOCUMENT}"), Some(FontId::proportional(20.0)), &style, scroll);
    assert_size(&sections, "inline", 20.0);
    assert_size(&sections, "main", 20.0);
  }
}

/// A change of `TextStyle::Body` changes the code size in a label with its own font, also after the label
/// caches its layout.
#[test]
fn a_change_of_the_body_size_changes_the_code_size() {
  let style = MarkdownStyle::default();
  for (prefix, scroll) in PATHS {
    let ctx = Context::default();
    let markdown = format!("{prefix}{DOCUMENT}");
    let font = Some(FontId::proportional(20.0));
    let sections = render(&ctx, &markdown, font.clone(), &style, scroll);
    assert_size(&sections, "main", 20.0);

    set_text_sizes(&ctx, 10.0, 13.0);
    let sections = render(&ctx, &markdown, font, &style, scroll);
    assert_size(&sections, "inline", 26.0);
    assert_size(&sections, "main", 26.0);
  }
}

/// Inline code in a table cell and in a header cell has the size of a code block.
#[test]
fn inline_code_in_a_table_cell_follows_the_code_size() {
  let ctx = Context::default();
  set_text_sizes(&ctx, 13.0, 11.0);
  let sections =
    render(&ctx, "| `head` | Name |\n|---|---|\n| `cell` | text |\n", None, &MarkdownStyle::default(), false);
  assert_size(&sections, "Name", 13.0);
  assert_size(&sections, "head", 11.0);
  assert_size(&sections, "cell", 11.0);
}

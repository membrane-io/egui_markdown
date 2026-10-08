use egui::epaint::Shape;
use egui::text::{LayoutJob, LayoutSection};
use egui::{pos2, vec2, Color32, Context, Id, RawInput, Rect, UiBuilder};
use egui_markdown::MarkdownLabel;

const RULE: &str = "Intro\n\n---\n\nAfter";

const PARAGRAPH: &str = "One two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen.";

/// One render of `text` at `width`, with the shapes it paints and the height it allocates. When `hide_after` is
/// set, a `map_job` sets alpha 0 on each character after that byte, as a reveal does.
fn render(text: &str, width: f32, hide_after: Option<usize>) -> (Vec<egui::epaint::ClippedShape>, f32) {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 400.0));
  let mut height = 0.0;
  let output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    let label = MarkdownLabel::new(Id::new("reveal"), text)
      .map_job(hide_after.map(|at| {
        move |_: &Context, mut job: LayoutJob| {
          let mut sections = Vec::new();
          for section in std::mem::take(&mut job.sections) {
            let range = section.byte_range.clone();
            let mut hidden = section.clone();
            hidden.format.color = Color32::TRANSPARENT;
            if range.end <= at {
              sections.push(section);
            } else if range.start >= at {
              sections.push(hidden);
            } else {
              sections.push(LayoutSection { byte_range: range.start..at, ..section });
              sections.push(LayoutSection { byte_range: at..range.end, ..hidden });
            }
          }
          job.sections = sections;
          job
        }
      }))
      .show(&mut child);
    height = label.response.rect.height();
  });
  (output.shapes, height)
}

/// The number of line segments that one render of `text` paints.
fn rules_painted(text: &str, hide_after: Option<usize>) -> usize {
  let (shapes, _) = render(text, 600.0, hide_after);
  shapes.iter().filter(|clipped| matches!(clipped.shape, Shape::LineSegment { .. })).count()
}

/// The height that one render of `text` allocates in a narrow column, so that the text wraps to several rows.
fn height(text: &str, hide_after: Option<usize>) -> f32 {
  render(text, 120.0, hide_after).1
}

/// A scroll that follows the bottom of the content moves for the allocated height. So a label that hides the
/// end of its text must not allocate the rows that hold only hidden text.
#[test]
fn a_label_that_hides_its_end_allocates_only_the_rows_that_show() {
  let whole = height(PARAGRAPH, None);
  let first_word = height(PARAGRAPH, Some("One".len()));
  assert!(whole > 3.0 * first_word, "the paragraph wraps to several rows: {whole} against one row {first_word}");
  assert!(first_word > 0.0, "the row that shows the first word is allocated");
  assert_eq!(height(PARAGRAPH, Some(0)), 0.0, "a label that shows no character allocates no row");
}

#[test]
fn a_label_that_shows_all_of_its_text_allocates_all_of_it() {
  assert_eq!(height(PARAGRAPH, Some(PARAGRAPH.len())), height(PARAGRAPH, None));
}

#[test]
fn a_rule_paints_when_no_map_hides_text() {
  assert_eq!(rules_painted(RULE, None), 1);
}

#[test]
fn a_rule_does_not_paint_before_the_reveal_shows_the_text_above_it() {
  assert_eq!(rules_painted(RULE, Some("Intro".len())), 0);
}

#[test]
fn a_rule_paints_after_the_reveal_shows_the_text_above_it() {
  assert_eq!(rules_painted(RULE, Some(RULE.len())), 1);
}

use egui::epaint::Shape;
use egui::text::{LayoutJob, LayoutSection};
use egui::{pos2, vec2, Color32, Context, Id, RawInput, Rect, UiBuilder};
use egui_markdown::MarkdownLabel;

const RULE: &str = "Intro\n\n---\n\nAfter";

/// The number of line segments that one render of `text` paints. When `hide_after` is set, a `map_job` sets
/// alpha 0 on each character after that byte, as a reveal does.
fn rules_painted(text: &str, hide_after: Option<usize>) -> usize {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
    MarkdownLabel::new(Id::new("reveal"), text)
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
  });
  output.shapes.iter().filter(|clipped| matches!(clipped.shape, Shape::LineSegment { .. })).count()
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

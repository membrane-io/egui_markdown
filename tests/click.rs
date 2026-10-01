use egui::text::{LayoutJob, LayoutSection};
use egui::{pos2, vec2, Context, Event, Id, Modifiers, OutputCommand, PointerButton, Pos2, RawInput, Rect, UiBuilder};
use egui_markdown::MarkdownLabel;

const PLAIN_THEN_LINK: &str = "plain words [link](https://example.com)";
const WITH_TABLE: &str = "plain words [link](https://example.com)\n\n| a |\n|---|\n| b |";

/// Render `text`, then click at the point that `at` picks from the label rect. Returns (clicked, link_clicked).
fn click(text: &str, at: impl Fn(Rect) -> Pos2) -> (bool, bool) {
  let (clicked, link_clicked, _) = click_label(text, None, false, at);
  (clicked, link_clicked)
}

/// As `click`, but a `map_job` splits the section that holds byte `split`, when there is one.
fn click_split(text: &str, split: Option<usize>, at: impl Fn(Rect) -> Pos2) -> (bool, bool) {
  let (clicked, link_clicked, _) = click_label(text, split, false, at);
  (clicked, link_clicked)
}

/// As `click_split`, with `heal` on the label. Also returns the URL of each `OpenUrl` command of the click.
fn click_label(text: &str, split: Option<usize>, heal: bool, at: impl Fn(Rect) -> Pos2) -> (bool, bool, Vec<String>) {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let frame = |events: Vec<Event>| {
    let mut result = None;
    let output = ctx.run_ui(RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      let out = MarkdownLabel::new(Id::new("click"), text)
        .hug_content(true)
        .selectable(false)
        .heal(heal)
        .map_job(split.map(|at| {
          move |_: &Context, mut job: LayoutJob| {
            let mut sections = Vec::new();
            for section in std::mem::take(&mut job.sections) {
              let range = section.byte_range.clone();
              if range.start < at && at < range.end {
                sections.push(LayoutSection { byte_range: range.start..at, ..section.clone() });
                sections.push(LayoutSection { byte_range: at..range.end, ..section });
              } else {
                sections.push(section);
              }
            }
            job.sections = sections;
            job
          }
        }))
        .show(&mut child);
      result = Some((out.response.rect, out.response.clicked(), out.link_clicked));
    });
    let urls = output
      .platform_output
      .commands
      .into_iter()
      .filter_map(|command| match command {
        OutputCommand::OpenUrl(open) => Some(open.url),
        _ => None,
      })
      .collect::<Vec<_>>();
    let (rect, clicked, link_clicked) = result.unwrap();
    (rect, clicked, link_clicked, urls)
  };

  let (rect, ..) = frame(Vec::new());
  let pos = at(rect);
  let button =
    |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
  frame(vec![Event::PointerMoved(pos)]);
  frame(vec![button(true)]);
  let (_, clicked, link_clicked, urls) = frame(vec![button(false)]);
  (clicked, link_clicked, urls)
}

#[test]
fn a_click_off_a_link_is_a_click_on_the_label() {
  for text in [PLAIN_THEN_LINK, WITH_TABLE] {
    assert_eq!(click(text, |rect| pos2(rect.left() + 4.0, rect.center().y)), (true, false), "{text:?}");
  }
}

#[test]
fn a_click_on_a_link_reports_the_link() {
  for text in [PLAIN_THEN_LINK, WITH_TABLE] {
    assert_eq!(click(text, |rect| pos2(rect.right() - 4.0, rect.top() + 6.0)), (true, true), "{text:?}");
  }
}

/// A `map_job` that splits a section before the link must not move the link to another section.
#[test]
fn a_click_on_a_link_reports_the_link_after_a_map_job_splits_a_section() {
  let at = |rect: Rect| pos2(rect.right() - 4.0, rect.top() + 6.0);
  assert_eq!(click_split(PLAIN_THEN_LINK, Some("plain".len()), at), (true, true));
  assert_eq!(
    click_split(PLAIN_THEN_LINK, Some("plain".len()), |rect| pos2(rect.left() + 4.0, rect.center().y)),
    (true, false)
  );
}

#[test]
fn a_click_on_a_link_opens_its_url() {
  let (_, link_clicked, urls) =
    click_label(PLAIN_THEN_LINK, None, false, |rect| pos2(rect.right() - 4.0, rect.top() + 6.0));
  assert!(link_clicked);
  assert_eq!(urls, ["https://example.com"]);
}

/// Heal gives a link an empty href until its URL arrives.
#[test]
fn a_click_on_a_link_with_a_partial_url_opens_nothing() {
  let at = |rect: Rect| pos2(rect.right() - 4.0, rect.top() + 6.0);
  let (clicked, link_clicked, urls) = click_label("See [app](program:/my", None, true, at);
  assert_eq!((clicked, link_clicked), (true, true));
  assert!(urls.is_empty(), "{urls:?}");
}

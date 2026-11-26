use egui::text::{ByteIndex, LayoutJob, LayoutSection};
use egui::{
  pos2, vec2, Context, CursorIcon, Event, Id, Modifiers, Order, OutputCommand, PointerButton, Pos2, RawInput, Rect,
  UiBuilder,
};
use egui_markdown::{LinkHandler, MarkdownLabel};
use std::cell::RefCell;

const PLAIN_THEN_LINK: &str = "plain words [link](https://example.com)";
const WITH_TABLE: &str = "plain words [link](https://example.com)\n\n| a |\n|---|\n| b |";

/// Render `text`, then click at the point that `at` picks from the label rect. Returns (clicked, link_clicked).
fn click(text: &str, at: impl Fn(Rect) -> Pos2) -> (bool, bool) {
  let (clicked, link_clicked, _) = click_label(text, None, false, PointerButton::Primary, Modifiers::NONE, at);
  (clicked, link_clicked)
}

/// As `click`, but a `map_job` splits the section that holds byte `split`, when there is one.
fn click_split(text: &str, split: Option<usize>, at: impl Fn(Rect) -> Pos2) -> (bool, bool) {
  let (clicked, link_clicked, _) = click_label(text, split, false, PointerButton::Primary, Modifiers::NONE, at);
  (clicked, link_clicked)
}

/// As `click_split`, with `heal` on the label, and a click of `button` with `modifiers` held. Also returns the
/// URL and `new_tab` of each `OpenUrl` command of the click.
fn click_label(
  text: &str,
  split: Option<usize>,
  heal: bool,
  button: PointerButton,
  modifiers: Modifiers,
  at: impl Fn(Rect) -> Pos2,
) -> (bool, bool, Vec<(String, bool)>) {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let frame = |events: Vec<Event>| {
    let mut result = None;
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      let out = MarkdownLabel::new(Id::NULL.with("click"), text)
        .hug_content(true)
        .selectable(false)
        .heal(heal)
        .map_job(split.map(|at| {
          move |_: &Context, mut job: LayoutJob| {
            let at = ByteIndex(at);
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
    output.textures_delta.clear();
    let urls = output
      .platform_output
      .commands
      .into_iter()
      .filter_map(|command| match command {
        OutputCommand::OpenUrl(open) => Some((open.url, open.new_tab)),
        _ => None,
      })
      .collect::<Vec<_>>();
    let (rect, clicked, link_clicked) = result.unwrap();
    (rect, clicked, link_clicked, urls)
  };

  let (rect, ..) = frame(Vec::new());
  let pos = at(rect);
  let press = |pressed| Event::PointerButton { pos, button, pressed, modifiers };
  frame(vec![Event::ModifiersChanged(modifiers), Event::PointerMoved(pos)]);
  frame(vec![press(true)]);
  let (_, clicked, link_clicked, urls) = frame(vec![press(false)]);
  (clicked, link_clicked, urls)
}

#[test]
fn a_click_off_a_link_is_a_click_on_the_label() {
  for text in [PLAIN_THEN_LINK, WITH_TABLE] {
    assert_eq!(click(text, |rect| pos2(rect.left() + 4.0, rect.top() + 6.0)), (true, false), "{text:?}");
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
  let at = |rect: Rect| pos2(rect.right() - 4.0, rect.top() + 6.0);
  let (_, link_clicked, urls) = click_label(PLAIN_THEN_LINK, None, false, PointerButton::Primary, Modifiers::NONE, at);
  assert!(link_clicked);
  assert_eq!(urls, [("https://example.com".to_owned(), false)]);
}

/// As with `egui::Hyperlink`, a modifier key or the middle button opens the link in a new tab.
#[test]
fn a_click_on_a_link_with_a_modifier_or_the_middle_button_opens_a_new_tab() {
  let at = |rect: Rect| pos2(rect.right() - 4.0, rect.top() + 6.0);
  for text in [PLAIN_THEN_LINK, WITH_TABLE] {
    for (button, modifiers) in [(PointerButton::Primary, Modifiers::COMMAND), (PointerButton::Middle, Modifiers::NONE)]
    {
      let (_, _, urls) = click_label(text, None, false, button, modifiers, at);
      assert_eq!(urls, [("https://example.com".to_owned(), true)], "{text:?} {button:?}");
    }
  }
}

/// Heal gives a link an empty href until its URL arrives.
#[test]
fn a_click_on_a_link_with_a_partial_url_opens_nothing() {
  let at = |rect: Rect| pos2(rect.right() - 4.0, rect.top() + 6.0);
  let (clicked, link_clicked, urls) =
    click_label("See [app](program:/my", None, true, PointerButton::Primary, Modifiers::NONE, at);
  assert_eq!((clicked, link_clicked), (true, true));
  assert!(urls.is_empty(), "{urls:?}");
}

/// What a hover and then a click on a point of a label give.
struct Interaction {
  cursor: CursorIcon,
  tooltip: bool,
  link_clicked: bool,
  urls: Vec<String>,
}

/// Render `text` with `handler`, move the pointer to the point that `at` picks from the label rect, and
/// then click there. The tooltip shows with no delay. `url_in_tooltip` sets the egui style flag.
fn hover_and_click(
  text: &str,
  handler: Option<&dyn LinkHandler>,
  url_in_tooltip: bool,
  at: impl Fn(Rect) -> Pos2,
) -> Interaction {
  let ctx = Context::default();
  ctx.global_style_mut(|style| {
    style.url_in_tooltip = url_in_tooltip;
    style.interaction.tooltip_delay = 0.0;
    style.interaction.show_tooltips_only_when_still = false;
  });
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let frame = |events: Vec<Event>| {
    let mut result = None;
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      let mut label = MarkdownLabel::new(Id::NULL.with("hover_and_click"), text).hug_content(true).selectable(false);
      if let Some(handler) = handler {
        label = label.link_handler(handler);
      }
      let out = label.show(&mut child);
      result = Some((out.response.rect, out.link_clicked));
    });
    output.textures_delta.clear();
    let urls = output
      .platform_output
      .commands
      .iter()
      .filter_map(|command| match command {
        OutputCommand::OpenUrl(open) => Some(open.url.clone()),
        _ => None,
      })
      .collect::<Vec<_>>();
    let (rect, link_clicked) = result.unwrap();
    (rect, link_clicked, urls, output.platform_output.cursor_icon)
  };

  let (rect, ..) = frame(Vec::new());
  let pos = at(rect);
  // A new tooltip area is invisible in its first frame, so the pointer stays on the link for a few frames.
  let (_, _, _, cursor) = frame(vec![Event::PointerMoved(pos)]);
  frame(Vec::new());
  frame(Vec::new());
  let tooltip = ctx.memory(|mem| mem.areas().visible_layer_ids().iter().any(|layer| layer.order == Order::Tooltip));
  let button =
    |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
  frame(vec![button(true)]);
  let (_, link_clicked, urls, _) = frame(vec![button(false)]);
  Interaction { cursor, tooltip, link_clicked, urls }
}

/// A table with one header cell and one body cell that holds `link`.
fn table_with_link(link: &str) -> String {
  format!("| name |\n|---|\n| {link} |")
}

/// A point on the text of the link in the body cell of [`table_with_link`]. The two rows have the same
/// height, and the table has 4 points of space under it.
fn on_cell_link(rect: Rect) -> Pos2 {
  pos2(rect.left() + 12.0, rect.top() + (rect.height() - 4.0) * 0.75)
}

#[test]
fn a_click_on_a_link_in_a_table_cell_opens_its_url_and_reports_the_link() {
  let text = table_with_link("[cell link](https://cell.example)");
  let result = hover_and_click(&text, None, false, on_cell_link);
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert_eq!(result.urls, ["https://cell.example"]);
  assert!(result.link_clicked);
}

/// Heal gives a link an empty href until its URL arrives.
#[test]
fn a_click_on_a_link_with_an_empty_href_in_a_table_cell_opens_nothing() {
  let text = table_with_link("[cell link]()");
  let result = hover_and_click(&text, None, false, on_cell_link);
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert!(result.link_clicked);
  assert!(result.urls.is_empty(), "{:?}", result.urls);
}

/// The tooltip of a paragraph link, to compare with a link in a table cell.
#[test]
fn a_link_with_a_title_in_a_paragraph_shows_a_tooltip() {
  let text = "[paragraph link](https://para.example \"Para title\")";
  let result = hover_and_click(text, None, false, |rect| pos2(rect.left() + 12.0, rect.center().y));
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert!(result.tooltip);
}

#[test]
fn a_link_with_a_title_in_a_table_cell_shows_a_tooltip() {
  let text = table_with_link("[cell link](https://cell.example \"Cell title\")");
  let result = hover_and_click(&text, None, false, on_cell_link);
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert!(result.tooltip);
}

#[test]
fn a_link_in_a_table_cell_shows_its_url_in_a_tooltip_when_url_in_tooltip_is_set() {
  let text = table_with_link("[cell link](https://cell.example)");
  assert!(!hover_and_click(&text, None, false, on_cell_link).tooltip);
  let result = hover_and_click(&text, None, true, on_cell_link);
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert!(result.tooltip);
}

/// A handler that records the href of each call to `hover_text`.
#[derive(Default)]
struct HoverTextRecorder {
  hrefs: RefCell<Vec<String>>,
}

impl LinkHandler for HoverTextRecorder {
  fn hover_text(&self, href: &str) -> Option<String> {
    self.hrefs.borrow_mut().push(href.to_owned());
    Some(format!("Open {href}"))
  }
}

#[test]
fn a_link_in_a_table_cell_shows_the_hover_text_of_the_link_handler() {
  let text = table_with_link("[cell link](https://cell.example)");
  let handler = HoverTextRecorder::default();
  let result = hover_and_click(&text, Some(&handler), false, on_cell_link);
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert!(result.tooltip);
  assert!(handler.hrefs.borrow().iter().any(|href| href == "https://cell.example"), "{:?}", handler.hrefs.borrow());
}

/// A handler that records the href of each call to `click`, and lets the label open the href.
#[derive(Default)]
struct ClickRecorder {
  hrefs: RefCell<Vec<String>>,
}

impl LinkHandler for ClickRecorder {
  fn click(&self, _text: &str, href: &str, _ui: &mut egui::Ui) -> bool {
    self.hrefs.borrow_mut().push(href.to_owned());
    false
  }
}

#[test]
fn a_click_on_a_link_with_an_empty_href_in_a_table_cell_does_not_call_the_link_handler() {
  let text = table_with_link("[cell link]()");
  let handler = ClickRecorder::default();
  let result = hover_and_click(&text, Some(&handler), false, on_cell_link);
  assert!(handler.hrefs.borrow().is_empty(), "{:?}", handler.hrefs.borrow());
  assert!(result.urls.is_empty(), "{:?}", result.urls);
  assert!(result.link_clicked);
}

/// A point on the first row of a label, near its left edge.
fn on_first_row(rect: Rect) -> Pos2 {
  pos2(rect.left() + 12.0, rect.top() + 6.0)
}

/// Without the `images` feature, an image shows as a link. With `url_in_tooltip`, its tooltip shows the URL,
/// also for an image in a paragraph.
#[cfg(not(feature = "images"))]
#[test]
fn an_image_link_shows_its_url_in_a_tooltip_when_url_in_tooltip_is_set() {
  for text in ["![image](https://img.example/a.png)", "![image](https://img.example/a.png) after the image"] {
    assert!(!hover_and_click(text, None, false, on_first_row).tooltip, "{text:?}");
    let result = hover_and_click(text, None, true, on_first_row);
    assert_eq!(result.cursor, CursorIcon::PointingHand, "{text:?}");
    assert!(result.tooltip, "{text:?}");
  }
}

/// Without the `images` feature, an image link shows the title of the image as its tooltip.
#[cfg(not(feature = "images"))]
#[test]
fn an_image_link_with_a_title_shows_a_tooltip() {
  let text = "![image](https://img.example/a.png \"Image title\")";
  let result = hover_and_click(text, None, false, on_first_row);
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert!(result.tooltip);
}

/// Without the `images` feature, an image link in a table cell shows the title of the image as its tooltip.
#[cfg(not(feature = "images"))]
#[test]
fn an_image_link_with_a_title_in_a_table_cell_shows_a_tooltip() {
  let text = table_with_link("![image](https://img.example/a.png \"Image title\")");
  let result = hover_and_click(&text, None, false, on_cell_link);
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert!(result.tooltip);
}

/// Without the `images` feature, a click on an image link opens the URL and sets `link_clicked`.
#[cfg(not(feature = "images"))]
#[test]
fn a_click_on_an_image_link_opens_its_url_and_reports_the_link() {
  let result = hover_and_click("![image](https://img.example/a.png)", None, false, on_first_row);
  assert_eq!(result.urls, ["https://img.example/a.png"]);
  assert!(result.link_clicked);
}

/// Heal gives an image an empty URL until its URL arrives.
#[cfg(not(feature = "images"))]
#[test]
fn a_click_on_an_image_link_with_an_empty_url_opens_nothing() {
  let result = hover_and_click("![image]()", None, false, on_first_row);
  assert!(result.urls.is_empty(), "{:?}", result.urls);
  assert!(result.link_clicked);
}

/// Heal gives a link an empty href until its URL arrives. `url_in_tooltip` gives no tooltip for it.
#[test]
fn a_link_with_an_empty_href_shows_no_tooltip_when_url_in_tooltip_is_set() {
  let result = hover_and_click("[paragraph link]()", None, true, on_first_row);
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert!(!result.tooltip);
  let text = table_with_link("[cell link]()");
  let result = hover_and_click(&text, None, true, on_cell_link);
  assert_eq!(result.cursor, CursorIcon::PointingHand);
  assert!(!result.tooltip);
}

/// Without the `images` feature, an image link uses the font of the label.
#[cfg(not(feature = "images"))]
#[test]
fn an_image_link_uses_the_font_of_the_label() {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let mut height = 0.0;
  for _ in 0..2 {
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      let out = MarkdownLabel::new(Id::NULL.with("image_font"), "![image](https://img.example/a.png)")
        .font(egui::FontId::proportional(40.0))
        .hug_content(true)
        .show(&mut child);
      height = out.response.rect.height();
    });
    output.textures_delta.clear();
  }
  assert!(height >= 40.0, "{height}");
}

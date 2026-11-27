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
  /// The `new_tab` of each `OpenUrl` command of the click.
  new_tabs: Vec<bool>,
}

/// Render `text` with `handler`, move the pointer to the point that `at` picks from the label rect, and
/// then click there. The tooltip shows with no delay. `url_in_tooltip` sets the egui style flag.
fn hover_and_click(
  text: &str,
  handler: Option<&dyn LinkHandler>,
  url_in_tooltip: bool,
  at: impl Fn(Rect) -> Pos2,
) -> Interaction {
  hover_and_click_in(&Context::default(), text, handler, url_in_tooltip, PointerButton::Primary, at)
}

/// As `hover_and_click`, in `ctx`, with a click of `button`.
fn hover_and_click_in(
  ctx: &Context,
  text: &str,
  handler: Option<&dyn LinkHandler>,
  url_in_tooltip: bool,
  button: PointerButton,
  at: impl Fn(Rect) -> Pos2,
) -> Interaction {
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
    let opened = output
      .platform_output
      .commands
      .iter()
      .filter_map(|command| match command {
        OutputCommand::OpenUrl(open) => Some((open.url.clone(), open.new_tab)),
        _ => None,
      })
      .collect::<Vec<_>>();
    let (rect, link_clicked) = result.unwrap();
    (rect, link_clicked, opened, output.platform_output.cursor_icon)
  };

  let (rect, ..) = frame(Vec::new());
  let pos = at(rect);
  // A new tooltip area is invisible in its first frame, so the pointer stays on the link for a few frames.
  let (_, _, _, cursor) = frame(vec![Event::PointerMoved(pos)]);
  frame(Vec::new());
  frame(Vec::new());
  let tooltip = ctx.memory(|mem| mem.areas().visible_layer_ids().iter().any(|layer| layer.order == Order::Tooltip));
  let button = |pressed| Event::PointerButton { pos, button, pressed, modifiers: Modifiers::NONE };
  frame(vec![button(true)]);
  let (_, link_clicked, opened, _) = frame(vec![button(false)]);
  let (urls, new_tabs) = opened.into_iter().unzip();
  Interaction { cursor, tooltip, link_clicked, urls, new_tabs }
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

/// A link around an image. Without the `images` feature, the image shows as a link with its alt text.
#[cfg(not(feature = "images"))]
const LINKED_IMAGE: &str = "[![logo](https://img.example/a.png)](https://site.example)";

/// Without the `images` feature, a linked image shows as a link to the link around it, not to the image.
#[cfg(not(feature = "images"))]
#[test]
fn a_click_on_a_linked_image_link_opens_the_link() {
  for (text, at) in
    [(LINKED_IMAGE.to_owned(), on_first_row as fn(Rect) -> Pos2), (table_with_link(LINKED_IMAGE), on_cell_link)]
  {
    let result = hover_and_click(&text, None, false, at);
    assert_eq!(result.cursor, CursorIcon::PointingHand, "{text:?}");
    assert_eq!(result.urls, ["https://site.example"], "{text:?}");
    assert_eq!(result.new_tabs, [false], "{text:?}");
    assert!(result.link_clicked, "{text:?}");
  }
}

/// Without the `images` feature, a linked image link is a markdown link, so it uses the link handler.
#[cfg(not(feature = "images"))]
#[test]
fn a_linked_image_link_uses_the_link_handler() {
  for (text, at) in
    [(LINKED_IMAGE.to_owned(), on_first_row as fn(Rect) -> Pos2), (table_with_link(LINKED_IMAGE), on_cell_link)]
  {
    let hover = HoverTextRecorder::default();
    assert!(hover_and_click(&text, Some(&hover), false, at).tooltip, "{text:?}");
    assert!(
      hover.hrefs.borrow().iter().any(|href| href == "https://site.example"),
      "{text:?}: {:?}",
      hover.hrefs.borrow()
    );
    let click = ClickRecorder::default();
    let result = hover_and_click(&text, Some(&click), false, at);
    assert_eq!(*click.hrefs.borrow(), ["https://site.example"], "{text:?}");
    assert_eq!(result.urls, ["https://site.example"], "{text:?}");
  }
}

/// A PNG image of `width` by `height` pixels.
#[cfg(feature = "images")]
fn png(width: u32, height: u32) -> Vec<u8> {
  let image = image::RgbaImage::from_pixel(width, height, image::Rgba([200, 30, 30, 255]));
  let mut bytes = std::io::Cursor::new(Vec::new());
  image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
  bytes.into_inner()
}

/// A context with the image loaders and a 60 by 60 image at `bytes://logo.png`. The loader decodes the image
/// on a thread, so the function waits until the image is ready.
#[cfg(feature = "images")]
fn image_context() -> Context {
  let ctx = Context::default();
  egui_extras::install_image_loaders(&ctx);
  ctx.include_bytes("bytes://logo.png", png(60, 60));
  for _ in 0..1000 {
    if matches!(ctx.try_load_image("bytes://logo.png", Default::default()), Ok(egui::load::ImagePoll::Ready { .. })) {
      return ctx;
    }
    std::thread::sleep(std::time::Duration::from_millis(5));
  }
  panic!("the image did not load");
}

/// A link around the image of [`image_context`].
#[cfg(feature = "images")]
const LINKED_LOGO: &str = "[![logo](bytes://logo.png)](https://site.example)";

/// A point on a label that holds only the image of [`image_context`].
#[cfg(feature = "images")]
fn on_image(rect: Rect) -> Pos2 {
  rect.center()
}

/// A point on the image in the body cell of [`table_with_link`]. The image is 60 points tall, at the vertical
/// center of its row, and the table has 4 points of space under it.
#[cfg(feature = "images")]
fn on_cell_image(rect: Rect) -> Pos2 {
  pos2(rect.left() + 30.0, rect.bottom() - 4.0 - 30.0)
}

/// With the `images` feature, a linked image is a link: the pointer is a hand, and a click opens the link
/// and calls the link handler.
#[cfg(feature = "images")]
#[test]
fn a_click_on_a_linked_image_opens_the_link() {
  for (text, at) in
    [(LINKED_LOGO.to_owned(), on_image as fn(Rect) -> Pos2), (table_with_link(LINKED_LOGO), on_cell_image)]
  {
    let handler = ClickRecorder::default();
    let result = hover_and_click_in(&image_context(), &text, Some(&handler), false, PointerButton::Primary, at);
    assert_eq!(result.cursor, CursorIcon::PointingHand, "{text:?}");
    assert_eq!(result.urls, ["https://site.example"], "{text:?}");
    assert_eq!(result.new_tabs, [false], "{text:?}");
    assert!(result.link_clicked, "{text:?}");
    assert_eq!(*handler.hrefs.borrow(), ["https://site.example"], "{text:?}");
  }
}

/// As with `egui::Hyperlink`, the middle button opens a linked image in a new tab.
#[cfg(feature = "images")]
#[test]
fn a_middle_click_on_a_linked_image_opens_a_new_tab() {
  for (text, at) in
    [(LINKED_LOGO.to_owned(), on_image as fn(Rect) -> Pos2), (table_with_link(LINKED_LOGO), on_cell_image)]
  {
    let result = hover_and_click_in(&image_context(), &text, None, false, PointerButton::Middle, at);
    assert_eq!(result.urls, ["https://site.example"], "{text:?}");
    assert_eq!(result.new_tabs, [true], "{text:?}");
  }
}

/// The tooltip of a linked image shows the title of the image, else the title of the link.
#[cfg(feature = "images")]
#[test]
fn a_linked_image_with_a_title_shows_a_tooltip() {
  for text in [
    "[![logo](bytes://logo.png \"Logo\")](https://site.example)",
    "[![logo](bytes://logo.png)](https://site.example \"Site\")",
  ] {
    let result = hover_and_click_in(&image_context(), text, None, false, PointerButton::Primary, on_image);
    assert!(result.tooltip, "{text:?}");
  }
}

/// With the `images` feature, an image that is not in a link is not a link.
#[cfg(feature = "images")]
#[test]
fn an_image_without_a_link_is_not_a_link() {
  for (text, at) in [
    ("![logo](bytes://logo.png)".to_owned(), on_image as fn(Rect) -> Pos2),
    (table_with_link("![logo](bytes://logo.png)"), on_cell_image),
  ] {
    let result = hover_and_click_in(&image_context(), &text, None, true, PointerButton::Primary, at);
    assert_ne!(result.cursor, CursorIcon::PointingHand, "{text:?}");
    assert!(result.urls.is_empty(), "{text:?}: {:?}", result.urls);
    assert!(!result.link_clicked, "{text:?}");
  }
}

/// A screen reader reads a linked image as a link with the alt text as its name.
#[cfg(feature = "images")]
#[test]
fn a_linked_image_is_an_accessible_link() {
  for text in [LINKED_LOGO.to_owned(), table_with_link(LINKED_LOGO)] {
    let ctx = image_context();
    ctx.enable_accesskit();
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      MarkdownLabel::new(Id::NULL.with("a11y"), &text).show(ui);
    });
    output.textures_delta.clear();
    let update = output.platform_output.accesskit_update.expect("no accessibility update");
    let links: Vec<_> =
      update.nodes.iter().filter(|(_, node)| node.role() == egui::Role::Link).map(|(_, node)| node.label()).collect();
    assert_eq!(links, [Some("logo")], "{text:?}");
  }
}

/// The height of the label of `text` in `ctx`.
fn label_height(ctx: &Context, text: &str) -> f32 {
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let mut height = 0.0;
  for _ in 0..2 {
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      height =
        MarkdownLabel::new(Id::NULL.with("height"), text).hug_content(true).show(&mut child).response.rect.height();
    });
    output.textures_delta.clear();
  }
  height
}

/// A link around an image adds no row to the image.
#[test]
fn a_linked_image_is_as_tall_as_the_image() {
  #[cfg(feature = "images")]
  let (ctx, url) = (image_context(), "bytes://logo.png");
  #[cfg(not(feature = "images"))]
  let (ctx, url) = (Context::default(), "https://img.example/a.png");
  for (image, linked) in [
    (format!("![logo]({url})"), format!("[![logo]({url})](https://site.example)")),
    (
      format!("Before.\n\n![logo]({url})\n\nAfter."),
      format!("Before.\n\n[![logo]({url})](https://site.example)\n\nAfter."),
    ),
  ] {
    assert_eq!(label_height(&ctx, &linked), label_height(&ctx, &image), "{linked:?}");
  }
}

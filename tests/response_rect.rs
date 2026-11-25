//! The response of `show` covers the whole label: the text and every block.

use egui::{pos2, vec2, Context, Event, Id, Modifiers, PointerButton, Pos2, RawInput, Rect, Response, Ui, UiBuilder};
use egui_markdown::{LinkHandler, MarkdownLabel};

const TABLE: &str = "Text before\n\n| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n";

/// A name for the assert message, and a function that makes the label.
type Case = (&'static str, fn() -> MarkdownLabel<'static>);

struct BlockWidgets;

impl LinkHandler for BlockWidgets {
  fn is_block_widget(&self, href: &str) -> bool {
    href.starts_with("widget://")
  }

  fn block_widget(&self, ui: &mut Ui, text: &str, _href: &str) -> Option<Response> {
    Some(egui::Frame::NONE.inner_margin(4).show(ui, |ui| ui.label(text)).response)
  }
}

/// Run one frame of `label` in a child Ui at `origin`. Returns the response and the rect that the label allocated.
fn frame(
  ctx: &Context,
  origin: Pos2,
  events: Vec<Event>,
  label: impl Fn() -> MarkdownLabel<'static>,
) -> (Response, Rect) {
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 400.0));
  let mut result = None;
  let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| {
    let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(origin, vec2(300.0, 800.0))));
    let response = label().show(&mut child).response;
    result = Some((response, child.min_rect()));
  });
  output.textures_delta.clear();
  result.unwrap()
}

#[test]
fn the_response_covers_every_block() {
  let cases: [Case; 6] = [
    ("a table after text", || MarkdownLabel::new(Id::NULL.with("table"), TABLE)),
    ("a table only", || MarkdownLabel::new(Id::NULL.with("table only"), "| a |\n|---|\n| 1 |")),
    ("an image", || MarkdownLabel::new(Id::NULL.with("image"), "Text\n\n![alt](https://example.com/i.png)")),
    ("a block widget", || {
      MarkdownLabel::new(Id::NULL.with("widget"), "Text\n\n[w](widget://w)").link_handler(&BlockWidgets)
    }),
    ("a scrolling code block", || {
      MarkdownLabel::new(Id::NULL.with("code"), "Text\n\n```\nlet x = 1;\n```").scroll_code_blocks(true)
    }),
    ("a blockquote", || MarkdownLabel::new(Id::NULL.with("quote"), "> A quote.")),
  ];
  for (name, label) in cases {
    let ctx = Context::default();
    let mut last = None;
    for _ in 0..2 {
      last = Some(frame(&ctx, Pos2::ZERO, Vec::new(), label));
    }
    let (response, allocated) = last.unwrap();
    assert_eq!(response.rect, allocated, "{name}");
  }
}

/// The label culls a segment that is off the screen. The response still covers the space the segment takes.
#[test]
fn the_response_covers_a_label_below_the_screen() {
  let ctx = Context::default();
  for pass in 0..3 {
    let (response, allocated) =
      frame(&ctx, pos2(0.0, 1000.0), Vec::new(), || MarkdownLabel::new(Id::NULL.with("culled"), TABLE));
    assert_eq!(response.rect, allocated, "pass {pass}");
  }
}

/// The pointer on a table hovers the label. A click on a table is not a click on the label, because only the text
/// senses a click.
#[test]
fn the_pointer_on_a_table_hovers_the_label() {
  let ctx = Context::default();
  let label = || MarkdownLabel::new(Id::NULL.with("hover"), TABLE).selectable(false);
  let (_, allocated) = frame(&ctx, Pos2::ZERO, Vec::new(), label);
  let pos = pos2(allocated.left() + 10.0, allocated.bottom() - 10.0);
  let button =
    |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
  frame(&ctx, Pos2::ZERO, vec![Event::PointerMoved(pos)], label);
  let (hovered, _) = frame(&ctx, Pos2::ZERO, Vec::new(), label);
  assert!(hovered.hovered());
  frame(&ctx, Pos2::ZERO, vec![button(true)], label);
  let (released, _) = frame(&ctx, Pos2::ZERO, vec![button(false)], label);
  assert!(!released.clicked());
}

/// In a wrapping layout, a label that does not fit after a widget goes to the next row, and takes the full width.
#[test]
fn a_label_in_a_wrapping_layout_goes_to_the_next_row() {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 400.0));
  let mut rects = None;
  for _ in 0..2 {
    let mut output = ctx.run_ui(RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
      ui.horizontal_wrapped(|ui| {
        let before = ui.button("A button before the label").rect;
        let response = MarkdownLabel::new(Id::NULL.with("wrapped"), "Hello world").show(ui).response;
        let after = ui.button("After").rect;
        rects = Some((before, response.rect, after));
      });
    });
    output.textures_delta.clear();
  }
  let (before, label, after) = rects.unwrap();
  assert_eq!((label.left(), label.width()), (0.0, 300.0), "{label:?}");
  assert!(label.top() >= before.bottom(), "{label:?} is on the row of {before:?}");
  assert!(after.top() >= label.bottom(), "{after:?} is on the row of {label:?}");
}

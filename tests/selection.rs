//! Text selection, as on `egui::Label`.

use egui::{pos2, vec2, Context, Event, Id, Modifiers, OutputCommand, PointerButton, Pos2, RawInput, Rect, UiBuilder};
use egui_markdown::MarkdownLabel;

const TEXT: &str = "alpha beta gamma delta";

/// Click at the left end of the label, shift-click near its right end, and copy. Returns the copied text.
fn shift_click_and_copy() -> Option<String> {
  let ends = |rect: Rect| (pos2(rect.left() + 1.0, rect.center().y), pos2(rect.right() - 1.0, rect.center().y));
  shift_click_and_copy_text(TEXT, ends)
}

/// Show the label of `text`, click at the first point of `ends`, shift-click at the second point, and copy.
/// `ends` receives the rect of the label. Returns the copied text.
fn shift_click_and_copy_text(text: &str, ends: fn(Rect) -> (Pos2, Pos2)) -> Option<String> {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let frame = |events: Vec<Event>, modifiers: Modifiers| {
    let mut rect = Rect::NOTHING;
    let events = std::iter::once(Event::ModifiersChanged(modifiers)).chain(events).collect();
    let input = RawInput { screen_rect: Some(screen), events, ..Default::default() };
    let mut output = ctx.run_ui(input, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      rect = MarkdownLabel::new(Id::NULL.with("selection"), text).hug_content(true).show(&mut child).response.rect;
    });
    output.textures_delta.clear();
    let copied = output.platform_output.commands.into_iter().find_map(|command| match command {
      OutputCommand::CopyText(text) => Some(text),
      _ => None,
    });
    (rect, copied)
  };
  let click = |pos: Pos2, modifiers: Modifiers| {
    let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers };
    frame(vec![Event::PointerMoved(pos)], modifiers);
    frame(vec![button(true)], modifiers);
    frame(vec![button(false)], modifiers);
  };

  let (rect, _) = frame(Vec::new(), Modifiers::NONE);
  let (start, end) = ends(rect);
  click(start, Modifiers::NONE);
  click(end, Modifiers::SHIFT);
  frame(vec![Event::Copy], Modifiers::NONE).1
}

#[test]
fn a_shift_click_extends_the_selection() {
  let copied = shift_click_and_copy().expect("a selection to copy");
  assert!(copied.starts_with("alpha beta gamma delt"), "copied {copied:?}");
}

/// A copy of a task list gives the markdown of each box, so that a paste is a task list again.
#[test]
fn a_copy_of_a_task_list_gives_the_markdown_of_each_box() {
  let ends = |rect: Rect| (pos2(rect.left() + 1.0, rect.top() + 4.0), pos2(rect.right() - 1.0, rect.bottom() - 4.0));
  let copied = shift_click_and_copy_text("- [x] alpha\n- [ ] beta", ends).expect("a selection to copy");
  assert!(copied.starts_with("• [x] alpha\n• [ ] bet"), "copied {copied:?}");
}

/// Drag across a code block shown with `scroll_code_blocks`, and copy. Returns the copied text.
fn drag_across_scrolling_code_block_and_copy() -> Option<String> {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let frame = |events: Vec<Event>| {
    let mut rect = Rect::NOTHING;
    let input = RawInput { screen_rect: Some(screen), events, ..Default::default() };
    let mut output = ctx.run_ui(input, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      MarkdownLabel::new(Id::NULL.with("scrolling_code"), "```\nlet answer = 42;\n```")
        .scroll_code_blocks(true)
        .show(&mut child);
      rect = child.min_rect();
    });
    output.textures_delta.clear();
    let copied = output.platform_output.commands.into_iter().find_map(|command| match command {
      OutputCommand::CopyText(text) => Some(text),
      _ => None,
    });
    (rect, copied)
  };
  let button =
    |pos, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };

  let (rect, _) = frame(Vec::new());
  let start = pos2(rect.left() + 12.0, rect.center().y);
  let end = pos2(rect.left() + 200.0, rect.center().y);
  frame(vec![Event::PointerMoved(start)]);
  frame(vec![button(start, true)]);
  frame(vec![Event::PointerMoved(end)]);
  frame(vec![button(end, false)]);
  frame(vec![Event::Copy]).1
}

#[test]
fn a_scrolling_code_block_is_selectable() {
  let copied = drag_across_scrolling_code_block_and_copy().expect("a selection to copy");
  assert!(copied.contains("answer = 42"), "copied {copied:?}");
}

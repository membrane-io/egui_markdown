//! A shift-click extends the selection of a label, as it does on `egui::Label`.

use egui::{pos2, vec2, Context, Event, Id, Modifiers, OutputCommand, PointerButton, Pos2, RawInput, Rect, UiBuilder};
use egui_markdown::MarkdownLabel;

const TEXT: &str = "alpha beta gamma delta";

/// Click at the left end of the label, shift-click near its right end, and copy. Returns the copied text.
fn shift_click_and_copy() -> Option<String> {
  let ctx = Context::default();
  let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
  let frame = |events: Vec<Event>, modifiers: Modifiers| {
    let mut rect = Rect::NOTHING;
    let events = std::iter::once(Event::ModifiersChanged(modifiers)).chain(events).collect();
    let input = RawInput { screen_rect: Some(screen), events, ..Default::default() };
    let mut output = ctx.run_ui(input, |ui| {
      let mut child = ui.new_child(UiBuilder::new().max_rect(screen));
      rect = MarkdownLabel::new(Id::NULL.with("selection"), TEXT).hug_content(true).show(&mut child).response.rect;
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
  click(pos2(rect.left() + 1.0, rect.center().y), Modifiers::NONE);
  click(pos2(rect.right() - 1.0, rect.center().y), Modifiers::SHIFT);
  frame(vec![Event::Copy], Modifiers::NONE).1
}

#[test]
fn a_shift_click_extends_the_selection() {
  let copied = shift_click_and_copy().expect("a selection to copy");
  assert!(copied.starts_with("alpha beta gamma delt"), "copied {copied:?}");
}

//! Text that arrives a few characters at a time, as a reply from a language model does.
//!
//! The label keeps one id while the text grows, so that it reuses its caches. `.heal(true)` closes the open
//! markdown of the incomplete text, and `.map_job()` fades in the newest characters.
//!
//! ```sh
//! cargo run --example streaming
//! ```

use eframe::egui;
use egui::text::{ByteIndex, LayoutJob, LayoutSection};
use egui_markdown::MarkdownLabel;

const REPLY: &str = include_str!("markdown/streaming.md");
const CHARS_PER_SECOND: f64 = 200.0;
const FADE_CHARS: usize = 40;

fn main() -> eframe::Result {
  env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([700.0, 700.0]),
    ..Default::default()
  };

  let mut start_time = 0.0;

  eframe::run_ui_native("egui_markdown: streaming", options, move |ui, _frame| {
    let now = ui.input(|i| i.time);
    let chars = ((now - start_time) * CHARS_PER_SECOND) as usize;
    let end = REPLY.char_indices().nth(chars).map_or(REPLY.len(), |(i, _)| i);
    let streaming = end < REPLY.len();
    if streaming {
      ui.request_repaint();
    }

    egui::Panel::top("controls").show(ui, |ui| {
      ui.horizontal(|ui| {
        if ui.button("Restart").clicked() {
          start_time = now;
        }
        ui.label(format!("{end} of {} bytes", REPLY.len()));
      });
    });

    egui::CentralPanel::default().show(ui, |ui| {
      egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
        MarkdownLabel::new(ui.make_persistent_id("reply"), &REPLY[..end])
          .heal(streaming)
          .map_job(streaming.then_some(fade_in_newest))
          .show(ui);
      });
    });
  })
}

/// Fade in the last `FADE_CHARS` characters of the job: the newest character is almost transparent.
fn fade_in_newest(_ctx: &egui::Context, mut job: LayoutJob) -> LayoutJob {
  let len = job.text.len();
  let fade_start = job.text.char_indices().rev().nth(FADE_CHARS).map_or(0, |(i, _)| i);
  let mut sections = Vec::new();
  for section in std::mem::take(&mut job.sections) {
    let (start, end) = (section.byte_range.start.0, section.byte_range.end.0);
    if end <= fade_start {
      sections.push(section);
      continue;
    }
    if start < fade_start {
      sections.push(LayoutSection { byte_range: ByteIndex(start)..ByteIndex(fade_start), ..section.clone() });
    }
    // Give each character of the fade its own section and its own alpha.
    let from = start.max(fade_start);
    for (offset, c) in job.text[from..end].char_indices() {
      let at = from + offset;
      let mut format = section.format.clone();
      format.color = format.color.gamma_multiply(1.0 - (at - fade_start) as f32 / (len - fade_start) as f32);
      let leading_space = if at == start { section.leading_space } else { 0.0 };
      let byte_range = ByteIndex(at)..ByteIndex(at + c.len_utf8());
      sections.push(LayoutSection { leading_space, byte_range, format, ..section.clone() });
    }
  }
  job.sections = sections;
  job
}

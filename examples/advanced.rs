// The fork deprecates `Panel::show_inside` in favour of `show`, and published egui
// deprecates `show` in favour of `show_inside`. Both spellings compile against both,
// so this picks one rather than cfg-ing between them.
#![allow(deprecated)]

use std::collections::VecDeque;
use std::time::Instant;

use eframe::egui::{
  self, text::LayoutJob, Color32, FontDefinitions, FontFamily, FontId, Rect, Response, RichText, TextFormat, Ui, Vec2,
};
use egui_markdown::{LinkHandler, LinkStyle, MarkdownLabel, MarkdownStyle};

fn main() -> eframe::Result {
  env_logger::init();

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 700.0]).with_title("egui_markdown - advanced"),
    ..Default::default()
  };

  eframe::run_native("egui_markdown advanced", options, Box::new(|cc| Ok(Box::new(AdvancedApp::new(cc)))))
}

struct AdvancedApp {
  markdown_input: String,
  show_editor: bool,
  show_style_editor: bool,
  selectable: bool,
  interactable: bool,
  render_times: VecDeque<f64>,
  markdown_style: MarkdownStyle,
  simulate_stream: bool,
  stream_pos: usize,
  stream_source: String,
  last_stream_tick: Instant,
}

impl AdvancedApp {
  fn new(cc: &eframe::CreationContext<'_>) -> Self {
    install_bold_font(&cc.egui_ctx);

    Self {
      markdown_input: DEFAULT_MARKDOWN.to_string(),
      show_editor: true,
      show_style_editor: false,
      selectable: true,
      interactable: true,
      render_times: VecDeque::with_capacity(120),
      markdown_style: {
        let mut s = MarkdownStyle::default();
        s.code_font_size = 12.0;
        s
      },
      simulate_stream: false,
      stream_pos: 0,
      stream_source: DEFAULT_MARKDOWN.to_string(),
      last_stream_tick: Instant::now(),
    }
  }
}

fn install_bold_font(ctx: &egui::Context) {
  let mut fonts = FontDefinitions::default();
  // Use the built-in proportional font data as "bold" (real apps would load an actual bold font)
  if let Some(font_data) = fonts.font_data.get("Ubuntu-Light").cloned() {
    fonts.font_data.insert("Bold".to_owned(), font_data);
    fonts.families.insert(FontFamily::Name("bold".into()), vec!["Bold".to_owned()]);
  }
  ctx.set_fonts(fonts);
}

struct DemoLinkHandler;

impl LinkHandler for DemoLinkHandler {
  fn link_style(&self, href: &str) -> Option<LinkStyle> {
    if href.starts_with("custom://") {
      Some(LinkStyle { color: Some(Color32::from_rgb(255, 100, 100)), underline: true })
    } else if href.starts_with("styled://") {
      Some(LinkStyle { color: Some(Color32::from_rgb(255, 150, 50)), underline: true })
    } else {
      None
    }
  }

  fn click(&self, _text: &str, href: &str, _ui: &mut Ui) -> bool {
    if href.starts_with("custom://") || href.starts_with("badge://") {
      eprintln!("Custom link clicked: {href}");
      true // handled
    } else {
      false // fall through to default (open in browser)
    }
  }

  fn inline_widget_size(&self, href: &str, font: &FontId) -> Option<Vec2> {
    if href.starts_with("badge://") {
      Some(Vec2::new(0.0, font.size + 6.0))
    } else {
      None
    }
  }

  fn layout_link(&self, _ui: &Ui, text: &str, href: &str, job: &mut LayoutJob, font: &FontId, _color: Color32) -> bool {
    if href.starts_with("styled://") {
      job.append("\u{26A1} ", 0.0, TextFormat { font_id: font.clone(), color: Color32::YELLOW, ..Default::default() });
      job.append(
        text,
        0.0,
        TextFormat { font_id: font.clone(), color: Color32::from_rgb(255, 150, 50), ..Default::default() },
      );
      true
    } else if href.starts_with("badge://") {
      let placeholder = "\u{00A0}".repeat(text.len() + 1);
      let format = TextFormat { font_id: FontId::monospace(font.size), color: _color, ..Default::default() };
      job.append(&placeholder, 0.0, format);
      true
    } else {
      false
    }
  }

  fn paint_inline_widget(&self, ui: &mut Ui, text: &str, href: &str, rect: Rect) {
    if !href.starts_with("badge://") {
      return;
    }
    let fill = Color32::from_rgb(80, 160, 80);
    let rounding = rect.height() * 0.5;
    ui.painter().rect_filled(rect, rounding, fill);
    ui.painter().text(
      rect.center(),
      egui::Align2::CENTER_CENTER,
      text,
      FontId::proportional(rect.height() - 6.0),
      Color32::WHITE,
    );
  }

  fn is_block_widget(&self, href: &str) -> bool {
    href.starts_with("widget://")
  }

  fn block_widget(&self, ui: &mut Ui, text: &str, href: &str) -> Option<Response> {
    let response = egui::Frame::NONE
      .fill(ui.visuals().widgets.inactive.bg_fill)
      .corner_radius(4.0)
      .inner_margin(egui::Margin::symmetric(6, 2))
      .show(ui, |ui| {
        ui.horizontal(|ui| {
          ui.label(RichText::new("\u{1F517}").small());
          ui.label(text);
        });
      })
      .response;
    if response.clicked() {
      eprintln!("Widget link clicked: {href}");
    }
    Some(response)
  }
}

fn code_block_header(ui: &mut Ui, code: &str, lang: &str) {
  let bg = if ui.visuals().dark_mode { Color32::from_gray(90) } else { Color32::from_gray(210) };
  let button = egui::Button::new("Copy").fill(bg);
  if ui.add(button).clicked() {
    ui.ctx().copy_text(code.to_string());
  }
  if !lang.is_empty() {
    ui.label(egui::RichText::new(lang).small().color(bg));
  }
}

impl eframe::App for AdvancedApp {
  fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
    let link_handler = DemoLinkHandler;

    egui::Panel::top("top").show(ui, |ui| {
      ui.horizontal(|ui| {
        ui.heading("egui_markdown advanced");
        ui.separator();
        ui.toggle_value(&mut self.show_editor, "Editor");
        ui.toggle_value(&mut self.show_style_editor, "Style");
        ui.toggle_value(&mut self.selectable, "Selectable");
        ui.toggle_value(&mut self.interactable, "Interactable");
        ui.separator();
        if ui.button("Reset").clicked() {
          self.markdown_input = DEFAULT_MARKDOWN.to_string();
          self.simulate_stream = false;
          self.stream_pos = 0;
        }
        if ui.toggle_value(&mut self.simulate_stream, "Simulate Stream").changed() {
          if self.simulate_stream {
            self.stream_source = self.markdown_input.clone();
            self.stream_pos = 0;
            self.markdown_input.clear();
            self.last_stream_tick = Instant::now();
          } else {
            self.markdown_input = self.stream_source.clone();
          }
        }
        ui.separator();
        let avg_render = if self.render_times.is_empty() {
          0.0
        } else {
          self.render_times.iter().sum::<f64>() / self.render_times.len() as f64
        };
        ui.label(format!("render: {avg_render:.2}ms"));
      });
    });

    if self.show_editor {
      egui::Panel::left("editor").default_size(400.0).show(ui, |ui| {
        ui.heading("Markdown Source");
        egui::ScrollArea::vertical().show(ui, |ui| {
          ui.add(egui::TextEdit::multiline(&mut self.markdown_input).desired_width(f32::INFINITY).code_editor());
        });
      });
    }

    if self.show_style_editor {
      egui::Panel::right("style_editor").default_size(280.0).show(ui, |ui| {
        ui.heading("Style");
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
          self.markdown_style.ui(ui);
        });
      });
    }

    if self.simulate_stream && self.stream_pos < self.stream_source.len() {
      let now = Instant::now();
      let elapsed_ms = now.duration_since(self.last_stream_tick).as_millis();
      if elapsed_ms >= 8 {
        let chars_to_add = 2;
        let mut end = self.stream_pos;
        for _ in 0..chars_to_add {
          if end >= self.stream_source.len() {
            break;
          }
          let next = end + 1;
          // Find next char boundary
          let mut boundary = next;
          while boundary < self.stream_source.len() && !self.stream_source.is_char_boundary(boundary) {
            boundary += 1;
          }
          end = boundary;
        }
        self.stream_pos = end;
        self.markdown_input = self.stream_source[..self.stream_pos].to_string();
        self.last_stream_tick = now;
      }
      ui.request_repaint();
    }

    egui::CentralPanel::default().show(ui, |ui| {
      ui.heading("Rendered Output");
      ui.separator();
      ui.style_mut().url_in_tooltip = true;
      egui::ScrollArea::vertical().show(ui, |ui| {
        let render_start = Instant::now();
        egui::Frame::new().inner_margin(egui::Margin::symmetric(24, 8)).fill(ui.visuals().faint_bg_color).show(
          ui,
          |ui| {
            MarkdownLabel::new(ui.id().with("md"), &self.markdown_input)
              .font(FontId::proportional(14.0))
              .selectable(self.selectable)
              .interactable(self.interactable)
              .link_handler(&link_handler)
              .code_block_buttons(&code_block_header)
              .scroll_code_blocks(true)
              .style(&self.markdown_style)
              .heal(self.simulate_stream)
              .show(ui);
          },
        );
        let render_elapsed = render_start.elapsed();

        if self.render_times.len() >= 120 {
          self.render_times.pop_front();
        }
        self.render_times.push_back(render_elapsed.as_secs_f64() * 1000.0);
      });
    });
  }
}

const DEFAULT_MARKDOWN: &str = r#"# egui_markdown - Advanced Demo

This demo shows **all features** including custom link handlers, code block buttons, and configurable options.

## Custom Links

A [normal link](https://github.com/emilk/egui) opens in the browser.

A [custom protocol link](custom://action/do-something) is handled by the `LinkHandler` trait.

## Custom Link Rendering

A [styled link](styled://path/to/thing) with custom colors inline.

An inline widget: [passing](badge://ci/passing) rendered as a painted badge over placeholder text.

A [widget link](widget://gref/myprogram/state) rendered as a block-level widget.

Regular [normal link](https://example.com) with default styling.

## Link Titles

Hover over this: [egui](https://github.com/emilk/egui "The egui UI library").

## Text Formatting

Regular, **bold**, *italic*, ***bold italic***, ~~strikethrough~~, and `inline code`.

## Headings

# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

## Lists

- Unordered item
  - Nested item
    - Deep nested
- Another item

1. First
2. Second
3. Third

## Long Lists
 - Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
 - Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.
 - Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.
 - Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

 1. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
 2. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.
 3. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.
 4. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

## Task Lists

- [x] Implement parser
- [x] Add caching
- [ ] Publish to crates.io

## Code Blocks

```rust
use egui_markdown::MarkdownLabel;

fn render(ui: &mut egui::Ui) {
    MarkdownLabel::new(ui.id().with("md"), "**Hello**")
        .selectable(true)
        .show(ui);
}
```

```javascript
// Try clicking the Copy button
const msg = "Hello from egui_markdown!";
console.log(msg);
```

```rust
// This line is intentionally very long to demonstrate horizontal scrolling in code blocks - it should scroll rather than wrap when scroll_code_blocks is enabled on the MarkdownLabel widget
fn example() { println!("scroll me!"); }
```

## Blockquotes

> Simple blockquote with **bold** and `code`.
>
> > Nested blockquote.
> >
> > > Triple nested.

## Tables

| Feature | Status | Description |
|:--------|:------:|------------:|
| Parsing | Done | pulldown-cmark based |
| Rendering | Done | Full egui integration |
| Caching | Done | FrameCache pattern |
| Links | Done | Custom handlers |
| Tables | Done | With alignment |
| [Link in table](https://example.com) | Done | Clickable |

## Horizontal Rules

Content above.

---

Content below.

## Footnotes

This sentence has a footnote[^1] and another[^note].

[^1]: First footnote content.
[^note]: Named footnote content.

## Mixed Content

> A blockquote with a list and formatting:
> - **Bold item** with `code`
> - *Italic item* with ~~strikethrough~~
> - A [link](https://example.com) in a list in a blockquote
"#;

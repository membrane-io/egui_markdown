// The fork deprecates `Panel::show_inside` in favour of `show`, and published egui
// deprecates `show` in favour of `show_inside`. Both spellings compile against both,
// so this picks one rather than cfg-ing between them.
#![allow(deprecated)]

use eframe::egui;
use egui_markdown::MarkdownLabel;

fn main() -> eframe::Result {
  env_logger::init();

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 700.0]).with_title("egui_markdown simple"),
    ..Default::default()
  };

  eframe::run_native("egui_markdown simple", options, Box::new(|cc| Ok(Box::new(DemoApp::new(cc)))))
}

struct DemoApp {
  markdown_input: String,
  show_editor: bool,
}

impl DemoApp {
  fn new(_cc: &eframe::CreationContext<'_>) -> Self {
    Self { markdown_input: DEFAULT_MARKDOWN.to_string(), show_editor: true }
  }
}

impl eframe::App for DemoApp {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    egui::Panel::top("top").show(ui, |ui| {
      ui.horizontal(|ui| {
        ui.heading("egui_markdown");
        ui.separator();
        ui.toggle_value(&mut self.show_editor, "Show Editor");
        if ui.button("Reset").clicked() {
          self.markdown_input = DEFAULT_MARKDOWN.to_string();
        }
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

    egui::CentralPanel::default().show(ui, |ui| {
      ui.heading("Rendered Output");
      ui.separator();
      egui::ScrollArea::vertical().show(ui, |ui| {
        MarkdownLabel::new(ui.id().with("md"), &self.markdown_input).show(ui);
      });
    });
  }
}

const DEFAULT_MARKDOWN: &str = r#"# egui_markdown

A markdown parser and renderer for [egui](https://github.com/emilk/egui).

## Features

### Text Formatting

Regular text, **bold text**, *italic text*, and ***bold italic text***.

You can also use `inline code` for short snippets.

~~Strikethrough~~ is supported too.

### Links

Visit [the egui repository](https://github.com/emilk/egui) for more info.

### Lists

Unordered list:
- First item
- Second item
  - Nested item
  - Another nested item
- Third item

Ordered list:
1. Step one
2. Step two
3. Step three

### Task Lists

- [x] Parser extraction
- [x] Renderer extraction
- [x] Strikethrough support
- [ ] Table rendering
- [ ] Image support

### Code Blocks

```rust
fn main() {
    println!("Hello from egui_markdown!");
}
```

```javascript
const greeting = "Hello, world!";
console.log(greeting);
```

### Blockquotes

> This is a blockquote. It can contain **formatted text** and `code`.
>
> > Nested blockquotes work too.

### Headings

# Heading 1
## Heading 2
### Heading 3
#### Heading 4

### Horizontal Rules

Above the rule.

---

Below the rule.

### Tables

| Feature | Status | Notes |
|:--------|:------:|------:|
| Bold | Done | Works well |
| Italic | Done | Works well |
| Code blocks | Done | With syntax highlighting |
| Tables | WIP | Basic support |
| Images | Planned | Behind feature flag |

### Footnotes

Here is a sentence with a footnote[^1].

[^1]: This is the footnote content.

### Mixed Content

Here's a paragraph with **bold**, *italic*, `code`, and a [link](https://example.com) all in one line. This demonstrates how inline formatting works seamlessly together.

> A blockquote containing a list:
> - Item one
> - Item two with **bold**
> - Item three with `code`
"#;

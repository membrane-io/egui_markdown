# egui_markdown

Render **markdown** in an *egui* app with one widget. The text is selectable, and a [link](https://github.com/emilk/egui) opens on a click.

- Parse CommonMark with `pulldown-cmark`
  - Tables and footnotes
  - Syntax highlighting with `syntect`
- Stream the text of an LLM reply

```rust
fn ui(ui: &mut egui::Ui, text: &str) {
    let id = ui.make_persistent_id("reply");
    MarkdownLabel::new(id, text).heal(true).show(ui);
}
```

| Feature               | Default | Size  |
|:----------------------|:-------:|------:|
| `syntax_highlighting` |   Yes   | Small |
| `extra_syntaxes`      |   No    | Large |

> Keep the widget id the same while the text grows. The label keeps its caches under that id.

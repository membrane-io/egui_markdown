# egui_markdown_style

The visual style types for [`egui_markdown`](https://crates.io/crates/egui_markdown), in a crate of
their own so that an application can build and store a style without a dependency on the renderer.

`MarkdownStyle` holds the styles of inline code, code blocks, headings, rules, blockquotes, lists and
tables. `set_style` installs one style for a whole `egui::Context`, and `global_style` reads it back.

## Feature flags

| Feature | Description |
|---------|-------------|
| `serde` | `Serialize` and `Deserialize` on every style type. |
| `membrane` | The inline code fields that need [the Membrane egui fork](https://github.com/membrane-io/egui). |

## License

MIT or Apache-2.0

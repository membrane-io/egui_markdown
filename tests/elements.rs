//! Snapshot tests for each markdown element, in the light theme and in the dark theme.
//!
//! Update the snapshots with `UPDATE_SNAPSHOTS=1 cargo test --test elements`. CONTRIBUTING.md has the
//! procedure.
// The code block snapshot shows the syntect colors, so the test needs syntect.
#![cfg(feature = "syntax_highlighting")]

use egui::{Id, Theme};
use egui_kittest::{Harness, SnapshotResults};
use egui_markdown::MarkdownLabel;

const HEADINGS: &str = "# Heading 1\n## Heading 2\n### Heading 3\n#### Heading 4\n##### Heading 5\n###### Heading 6\n";

const INLINE: &str = "Text with **bold**, *italic*, ~~strikethrough~~, `inline code` and a \
[link](https://github.com/membrane-io/egui_markdown).\n";

const LISTS: &str = "\
- Fruit
  - Apple
  - Pear
- Vegetables

1. Clone the repository
2. Run `cargo test`
3. Open a pull request

- [x] Write the parser
- [ ] Write the renderer
";

const CODE_BLOCK: &str = "\
```rust
fn main() {
  let name = \"egui\";
  println!(\"Hello, {name}!\");
}
```
";

const TABLE: &str = "\
| Element | Count | Notes |
|:--------|------:|:-----:|
| Heading | 6 | Levels |
| List | 3 | Nested |
";

const BLOCKQUOTE: &str = "\
> A quote holds blocks.
>
> > A nested quote.
";

const RULE: &str = "Text above the rule.\n\n---\n\nText below the rule.\n";

const WRAP: &str = "egui_markdown parses the markdown text with pulldown-cmark and lays out the text in one \
galley, so a long paragraph wraps at the width of the panel.\n";

#[test]
fn elements() {
  let mut results = SnapshotResults::new();

  for (name, markdown) in [
    ("headings", HEADINGS),
    ("inline", INLINE),
    ("lists", LISTS),
    ("code_block", CODE_BLOCK),
    ("table", TABLE),
    ("blockquote", BLOCKQUOTE),
    ("rule", RULE),
    ("wrap", WRAP),
  ] {
    for (theme_name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
      let mut harness = Harness::builder().with_size([320.0, 400.0]).with_theme(theme).build_ui(|ui| {
        MarkdownLabel::new(Id::NULL.with(name), markdown).show(ui);
      });
      harness.fit_contents();
      results.add(harness.try_snapshot(format!("elements/{name}_{theme_name}")));
    }
  }
}

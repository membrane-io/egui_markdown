# egui_markdown - Advanced Demo

This demo shows **all features** including custom link handlers, code block buttons, and configurable options.

## Custom Links

A [normal link](https://github.com/emilk/egui) opens in the browser.

A [custom protocol link](custom://action/do-something) is handled by the `LinkHandler` trait.

## Custom Link Rendering

A [styled link](styled://path/to/thing) with custom colors inline.

An inline widget: [passing](badge://ci/passing) rendered as a painted badge over placeholder text.

A block-level widget takes rows of its own, so its link reads best as a paragraph of its own:

[widget link](widget://card/release-notes)

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
- [ ] Render math

## Code Blocks

```rust
use egui_markdown::MarkdownLabel;

fn render(ui: &mut egui::Ui) {
    MarkdownLabel::new(ui.make_persistent_id("md"), "**Hello**")
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
| Caching | Done | Egui temp data |
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

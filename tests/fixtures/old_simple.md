# egui_markdown

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

- [x] Parse CommonMark and GFM
- [x] Render tables and code blocks
- [x] Heal streamed text
- [ ] Render math

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
| Tables | Done | With alignment |
| Images | Done | Behind the `images` feature |

### Footnotes

Here is a sentence with a footnote[^1].

[^1]: This is the footnote content.

### Mixed Content

Here's a paragraph with **bold**, *italic*, `code`, and a [link](https://example.com) all in one line. This demonstrates how inline formatting works seamlessly together.

> A blockquote containing a list:
> - Item one
> - Item two with **bold**
> - Item three with `code`

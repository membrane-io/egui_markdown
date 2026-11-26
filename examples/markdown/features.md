# egui_markdown

A markdown parser and renderer for [egui](https://github.com/emilk/egui "The egui repository").

## Text

Regular text, **bold**, *italic*, ***bold italic***, ~~strikethrough~~ and `inline code`.

## Lists

- An item
  - A nested item
  - Another nested item
- A long item that wraps to more than one row when the panel is narrow, and starts each row at the same x.

1. Step one
2. Step two
3. Step three

- [x] Parse CommonMark and GitHub Flavored Markdown
- [x] Highlight code blocks
- [ ] Render math

## Code

```rust
fn main() {
    let name = "world";
    println!("Hello, {name}!");
}
```

## Quotes

> A blockquote can hold **formatted text**, `code` and lists:
> - One
> - Two
>
> > And a nested blockquote.
> >
> > > And one more.

## Tables

| Element     | Status | Notes                 |
|:------------|:------:|----------------------:|
| Headings    |  Done  | Six levels            |
| Code blocks |  Done  | Syntax highlighting   |
| Tables      |  Done  | With column alignment |
| Images      |  Done  | With the `images` feature |

---

#### Heading 4
##### Heading 5
###### Heading 6

A sentence with a footnote[^1] and a named footnote[^note].

[^1]: The text of the footnote.
[^note]: The text of the named footnote.

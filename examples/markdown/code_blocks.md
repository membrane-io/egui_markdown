# Code blocks

Each code block has a **Copy** button and the name of its language.

```rust
fn main() {
    let numbers = [1, 2, 3, 4, 5];
    let sum: i32 = numbers.iter().sum();
    println!("The sum is {sum}");
}
```

```toml
[dependencies]
egui_markdown = "0.1"
```

A long line scrolls when the code blocks scroll, and wraps when they do not:

```python
print("This line is long, so that it is wider than the window. Turn scrolling off to see the line wrap.")
```


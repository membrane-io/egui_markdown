## A streamed reply

This text arrives a few characters at a time, as a reply from a language model does. While the
text is incomplete, `heal` closes the open **bold text**, `inline code` and [links](https://github.com/emilk/egui).

A code block shows as a code block before its closing fence arrives:

```rust
fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => n,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

A table shows as a table as soon as its header row is complete:

| n  | fibonacci(n) |
|---:|-------------:|
| 10 | 55           |
| 20 | 6765         |
| 30 | 832040       |

1. The label keeps its caches under one id.
2. So each new character costs only a little work.
3. The newest characters fade in.

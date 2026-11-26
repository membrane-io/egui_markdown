# Contributing to egui_markdown

## Get started

1. Clone the repository.
2. Install Rust 1.95 or later, which egui `main` requires (`rustup update stable`).
3. Run `cargo check` to verify your setup.
4. Run `cargo run --example simple` to see the library render a document.

## Code style

Read [ARCHITECTURE.md](ARCHITECTURE.md) before you change the layout or the caches.

- All inline comments end with a period.
- Doc comments (`///`) come before `#[allow(...)]` attributes.
- No comment banners or section headers.
- No `.to_string()` inside a formatting macro such as `format!`, `println!`, or `tracing::info!`.
- Keep `use` imports sorted.

## Before you submit

Run the local CI script:

```sh
./check.sh
```

This runs `cargo fmt --check`, `cargo clippy` for three feature sets, `cargo test`, `cargo doc`, and a
wasm check. CI runs `cargo fmt --check`, `cargo clippy` on the library only, `cargo test`, `cargo doc` and
the wasm check, with the default features only.

## Pull request guidelines

- Keep each pull request to a single change.
- Add tests for new parser or layout behavior.
- Update `CHANGELOG.md` under an `[Unreleased]` section.
- Make sure that `./check.sh` passes before you request a review.

# Contributing to egui_markdown

## Get started

1. Clone the repository.
2. Install Rust 1.95 or later, which egui `main` requires (`rustup update stable`).
3. Run `cargo check` to verify your setup.
4. Run `cargo run --example editor` to see the library render a document.

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

This runs `cargo fmt --check`, `cargo clippy` for three feature sets, `cargo test` with and without the
default features, `cargo doc`, and a wasm check. CI runs `cargo fmt --check`, `cargo clippy` on the library
only, `cargo test` with and without the default features, `cargo doc` and the wasm check.

## README screenshots

A test renders the README screenshots in `docs/screenshots/` with wgpu. Run it again after a change to the
look of the label or to `examples/markdown/readme.md`:

```sh
README_SCREENSHOTS=1 cargo test --test readme_screenshots
```

Without `README_SCREENSHOTS`, the test does nothing, so CI does not need a GPU.

## Pull request guidelines

- Keep each pull request to a single change.
- Add tests for new parser or layout behavior.
- Update `CHANGELOG.md` under an `[Unreleased]` section.
- Make sure that `./check.sh` passes before you request a review.

# Contributing to egui_markdown

## Get started

1. Clone the repository.
2. Install Rust 1.95 or later, which egui 0.36 requires (`rustup update stable`).
3. Run `cargo check` to verify your setup.
4. Run `cargo run --example simple` to see the library render a document.

## Code Style

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

This runs `cargo fmt --check`, `cargo clippy` across the feature configurations, `cargo test`,
`cargo doc`, and a wasm check. It covers every feature except `membrane`, which compiles only against
[the Membrane egui fork](https://github.com/membrane-io/egui). CI checks `membrane` in a separate job.

## Pull Request Guidelines

- Keep each pull request to a single change.
- Add tests for new parser or layout behavior.
- Update `CHANGELOG.md` under an `[Unreleased]` section.
- Make sure that `./check.sh` passes before you request a review.

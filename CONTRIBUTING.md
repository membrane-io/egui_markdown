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
only, `cargo doc` and the wasm check on Linux. It runs `cargo test` with and without the default features on
macOS, because the snapshot tests need a GPU.

## Snapshot tests

The tests in `tests/elements.rs` render each markdown element with
[egui_kittest](https://github.com/emilk/egui/tree/main/crates/egui_kittest) and compare the image with a snapshot in
`tests/snapshots/`. CI runs the snapshot tests on macOS, so the macOS images are the source of truth. On macOS, the
test fails when the difference is more than the `[mac]` threshold in `kittest.toml`. On other systems, the threshold
is larger.

A change to the look of the label changes the snapshots. To update them locally, run:

```sh
UPDATE_SNAPSHOTS=1 cargo test
```

If you do not use macOS, CI can fail on a snapshot that passes locally. Then update your snapshots from the last
CI run of your branch. The script downloads the `test-results` artifact with the `gh` CLI:

```sh
./scripts/update_snapshots_from_ci.sh
```

A comment on each pull request links to [kitdiff](https://rerun-io.github.io/kitdiff/), which shows the snapshot
changes of the pull request.

Git stores the snapshots with [git-lfs](https://git-lfs.com/). Install git-lfs, and then run this command once in
the repository:

```sh
git lfs install
```

`.gitattributes` puts each `.png` file in git-lfs, except the README screenshots. A CI check fails when a `.png`
file is not in git-lfs. If you see an `InvalidSignature` error from a snapshot test,
make sure that git-lfs downloaded the images (`git lfs pull`).

## README screenshots

A test renders the README screenshots in `docs/screenshots/` with wgpu. Run it again after a change to the
look of the label or to `examples/markdown/readme.md`:

```sh
README_SCREENSHOTS=1 cargo test --test readme_screenshots
```

Without `README_SCREENSHOTS`, the test does nothing.

The README screenshots stay in plain git, not in git-lfs, because raw.githubusercontent.com and crates.io do not
serve git-lfs content. `.gitattributes` excludes `docs/screenshots/` from git-lfs.

## Pull request guidelines

- Keep each pull request to a single change.
- Add tests for new parser or layout behavior.
- Update `CHANGELOG.md` under an `[Unreleased]` section.
- Make sure that `./check.sh` passes before you request a review.

#!/usr/bin/env bash
# Local CI script, run before pushing.
#
# Every feature but `membrane` is checked here. `membrane` compiles only against the
# Membrane egui fork, so CI checks it in a separate job that patches egui to the fork.
set -euo pipefail

FEATURES="syntax_highlighting,images,svg"

echo "--- cargo fmt --check ---"
cargo fmt --all --check

echo "--- clippy (default features) ---"
cargo clippy --all-targets -- -D warnings

echo "--- clippy (no default features) ---"
cargo clippy --all-targets --no-default-features -- -D warnings

echo "--- clippy (every feature but membrane) ---"
cargo clippy --all-targets --features "$FEATURES" -- -D warnings

echo "--- cargo test ---"
cargo test --features "$FEATURES"

echo "--- cargo doc ---"
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features "$FEATURES"

echo "--- wasm check ---"
cargo check --target wasm32-unknown-unknown --no-default-features

echo "All checks passed."

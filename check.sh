#!/usr/bin/env bash
# Local CI script, run before pushing.
set -euo pipefail

FEATURES="syntax_highlighting,images,svg"

echo "--- cargo fmt --check ---"
cargo fmt --all --check

echo "--- clippy (default features) ---"
cargo clippy --all-targets -- -D warnings

echo "--- clippy (no default features) ---"
cargo clippy --all-targets --no-default-features -- -D warnings

echo "--- clippy (every feature but extra_syntaxes) ---"
cargo clippy --all-targets --features "$FEATURES" -- -D warnings

echo "--- cargo test ---"
cargo test --features "$FEATURES"

echo "--- cargo test (no default features) ---"
cargo test --no-default-features

echo "--- cargo doc ---"
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features "$FEATURES"

echo "--- wasm check ---"
cargo check --target wasm32-unknown-unknown --no-default-features

echo "All checks passed."

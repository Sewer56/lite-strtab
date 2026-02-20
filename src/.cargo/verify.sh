#!/usr/bin/env bash
# Post-change verification script
# All steps must pass without warnings
# Keep in sync with verify.ps1

set -e

run_cmd() {
  echo "$*"
  "$@"
}

echo "Building..."
run_cmd cargo build --workspace --all-targets --features std --quiet

echo "Testing..."
run_cmd cargo test --workspace --features std --quiet

echo "Clippy..."
run_cmd cargo clippy --workspace --features std --quiet -- -D warnings

echo "Docs..."
run_cmd env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --features std --no-deps --document-private-items --quiet

echo "Formatting..."
run_cmd cargo fmt --all --quiet

echo "Publish dry-run..."
run_cmd cargo publish --dry-run --allow-dirty --quiet --workspace

echo "All checks passed!"

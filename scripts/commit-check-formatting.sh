#!/bin/bash
set -e

echo "Checking formatting..."
cargo fmt -- --check

echo "Cargo clippy..."
cargo clippy --all-targets -- -D warnings

exit 0

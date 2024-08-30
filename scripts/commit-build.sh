#!/bin/bash
set -e

echo "Building debug..."
cargo build

echo "Building release..."
cargo build --release

echo "Building wasi..."
cargo wasi build --release

exit 0

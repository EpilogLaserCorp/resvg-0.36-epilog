#!/bin/bash
set -e

echo "Building debug..."
cargo build

echo "Building release..."
cargo build --release

echo "Building wasi..."
cargo build --release --target wasm32-wasip2

exit 0

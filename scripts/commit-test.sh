#!/bin/bash
set -e

# Test in release mode to ensure final product will pass tests.
echo "Running tests..."
cargo test --release

exit 0

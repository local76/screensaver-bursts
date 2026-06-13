#!/bin/bash
set -e

echo "=== Running Cargo Check ==="
cargo check

echo "=== Running Cargo Clippy ==="
cargo clippy

echo "=== Running Cargo Test ==="
cargo test

echo "=== All checks and tests passed successfully! ==="

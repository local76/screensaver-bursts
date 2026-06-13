#!/bin/bash
set -e
echo "Building in release mode..."
cargo build --release
echo "Copying binary to dist/binaries..."
mkdir -p dist/binaries
cp target/release/bursts dist/binaries/
echo "Build complete."

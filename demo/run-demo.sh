#!/bin/bash
# Gity Performance Demo
# Demonstrates gity's speedup for git status on large repositories

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DEMO_BIN="$SCRIPT_DIR/target/release/gity-demo"
GITY_BIN="$PROJECT_DIR/target/release/gity"

# Build gity and demo in release mode for accurate benchmarks
echo "Building gity and demo (release mode)..."
cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml" --bin gity
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

# Run the demo with 10,000 files and full cleanup
echo ""
echo "Running gity performance demo..."
echo ""

"$DEMO_BIN" --files 100000 --stop-daemon --gity-bin "$GITY_BIN" "$@"

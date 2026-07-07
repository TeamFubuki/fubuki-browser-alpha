#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_TYPE="${BUILD_TYPE:-release}"

echo "Building FrostEngine (Rust)..."
cd "$ROOT_DIR"

if [[ "$BUILD_TYPE" == "debug" ]]; then
  cargo build --workspace
else
  cargo build --workspace --release
fi

echo "FrostEngine build complete."

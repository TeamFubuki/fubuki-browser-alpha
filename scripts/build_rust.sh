#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_TYPE="${BUILD_TYPE:-release}"

if [[ "$(uname -s)" == "Darwin" ]]; then
  # Keep Rust host proc-macros loadable by rustc; the native app itself still
  # targets macOS 12 in native/CMakeLists.txt.
  export MACOSX_DEPLOYMENT_TARGET="${FUBUKI_RUST_OSX_DEPLOYMENT_TARGET:-11.0}"
fi

echo "Building FrostEngine (Rust)..."
cd "$ROOT_DIR"

if [[ "$BUILD_TYPE" == "debug" ]]; then
  cargo build --workspace
else
  cargo build --workspace --release
fi

echo "FrostEngine build complete."

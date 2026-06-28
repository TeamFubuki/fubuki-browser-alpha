#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NATIVE_BUILD_DIR="${NATIVE_BUILD_DIR:-"$ROOT_DIR/native/build"}"
BUILD_TYPE="${BUILD_TYPE:-Release}"

if [[ ! -f "$NATIVE_BUILD_DIR/CMakeCache.txt" ]]; then
  "$ROOT_DIR/scripts/configure_native.sh"
fi

cmake --build "$NATIVE_BUILD_DIR" --config "$BUILD_TYPE"

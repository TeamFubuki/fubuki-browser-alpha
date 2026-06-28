#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CEF_ROOT="${CEF_ROOT:-"$ROOT_DIR/third_party/cef"}"
NATIVE_BUILD_DIR="${NATIVE_BUILD_DIR:-"$ROOT_DIR/native/build"}"
BUILD_TYPE="${BUILD_TYPE:-Release}"

if [[ ! -f "$CEF_ROOT/cmake/cef_variables.cmake" ]]; then
  echo "CEF is missing at $CEF_ROOT" >&2
  echo "Run: make cef" >&2
  exit 1
fi

cmake -S "$ROOT_DIR/native" -B "$NATIVE_BUILD_DIR" \
  -DCEF_ROOT="$CEF_ROOT" \
  -DCMAKE_BUILD_TYPE="$BUILD_TYPE"

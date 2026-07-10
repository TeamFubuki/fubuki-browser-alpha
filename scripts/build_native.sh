#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NATIVE_BUILD_DIR="${NATIVE_BUILD_DIR:-"$ROOT_DIR/native/build"}"
BUILD_TYPE="${BUILD_TYPE:-release}"

# Normalize build type to title case for CMake (e.g., "release" -> "Release")
CMAKE_BUILD_TYPE="$(echo "${BUILD_TYPE}" | sed 's/.*/\u&/')"

if [[ ! -f "$NATIVE_BUILD_DIR/CMakeCache.txt" ]]; then
  "$ROOT_DIR/scripts/configure_native.sh"
fi

cmake --build "$NATIVE_BUILD_DIR" --config "$CMAKE_BUILD_TYPE"

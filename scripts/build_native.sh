#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_CEF_ROOT="$ROOT_DIR/third_party/cef"
CEF_ROOT="${CEF_ROOT:-$DEFAULT_CEF_ROOT}"
CEF_ROOT="$(python3 -c 'import os, sys; print(os.path.abspath(sys.argv[1]))' "$CEF_ROOT")"
NATIVE_BUILD_DIR="${NATIVE_BUILD_DIR:-"$ROOT_DIR/native/build"}"
BUILD_TYPE="${BUILD_TYPE:-release}"

if [[ "$CEF_ROOT" == "$DEFAULT_CEF_ROOT" ]]; then
  "$ROOT_DIR/scripts/fetch_cef.sh"
elif [[ ! -f "$CEF_ROOT/cmake/cef_variables.cmake" ]]; then
  echo "CEF_ROOT is not a CEF distribution: $CEF_ROOT" >&2
  exit 1
fi

# Normalize build type to title case for CMake (e.g., "release" -> "Release")
CMAKE_BUILD_TYPE="$(echo "${BUILD_TYPE}" | sed 's/.*/\u&/')"

if [[ ! -f "$NATIVE_BUILD_DIR/CMakeCache.txt" ]]; then
  "$ROOT_DIR/scripts/configure_native.sh"
fi

cmake --build "$NATIVE_BUILD_DIR" --config "$CMAKE_BUILD_TYPE"

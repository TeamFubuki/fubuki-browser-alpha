#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NATIVE_BUILD_DIR="${NATIVE_BUILD_DIR:-"$ROOT_DIR/native/build"}"
BUILD_TYPE="${BUILD_TYPE:-release}"

# The native target packages ui/dist into the application bundle. Keep this
# script self-contained so `make native` and direct invocations cannot produce
# an app that starts without its UI assets.
"$ROOT_DIR/scripts/build_ui.sh"

if [[ ! -f "$ROOT_DIR/ui/dist/index.html" ]]; then
  echo "UI build completed without ui/dist/index.html" >&2
  exit 1
fi

# Normalize build type to title case for CMake (e.g., "release" -> "Release")
CMAKE_BUILD_TYPE="$(echo "${BUILD_TYPE}" | sed 's/.*/\u&/')"

if [[ ! -f "$NATIVE_BUILD_DIR/CMakeCache.txt" ]]; then
  "$ROOT_DIR/scripts/configure_native.sh"
fi

cmake --build "$NATIVE_BUILD_DIR" --config "$CMAKE_BUILD_TYPE"

# Ensure UI dist is copied to the app bundle.
# CMake POST_BUILD only runs when the target is rebuilt. If the native binary
# is up-to-date but ui/dist changed, we must copy manually.
APP_BUNDLE="$NATIVE_BUILD_DIR/Fubuki Browser Alpha.app"
if [[ ! -d "$APP_BUNDLE" ]]; then
  APP_BUNDLE="$NATIVE_BUILD_DIR/Release/Fubuki Browser Alpha.app"
fi
if [[ -d "$APP_BUNDLE/Contents" && -d "$ROOT_DIR/ui/dist" ]]; then
  mkdir -p "$APP_BUNDLE/Contents/Resources/ui"
  # Remove old UI files to ensure a clean copy
  rm -rf "$APP_BUNDLE/Contents/Resources/ui"
  cp -R "$ROOT_DIR/ui/dist" "$APP_BUNDLE/Contents/Resources/ui"
fi

if [[ ! -f "$APP_BUNDLE/Contents/Resources/ui/index.html" ]]; then
  echo "Native build completed without bundled UI assets: $APP_BUNDLE" >&2
  exit 1
fi

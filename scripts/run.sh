#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_TYPE="${BUILD_TYPE:-release}"
NATIVE_BUILD_DIR="${NATIVE_BUILD_DIR:-"$ROOT_DIR/native/build"}"

"$ROOT_DIR/scripts/build.sh"

if [[ "$BUILD_TYPE" == "debug" ]]; then
  app="$NATIVE_BUILD_DIR/Fubuki Browser Alpha.app"
else
  app="$NATIVE_BUILD_DIR/Fubuki Browser Alpha.app"
  if [[ ! -d "$app" ]]; then
    app="$NATIVE_BUILD_DIR/Release/Fubuki Browser Alpha.app"
  fi
fi

if [[ ! -d "$app" ]]; then
  echo "App bundle not found under $NATIVE_BUILD_DIR" >&2
  exit 1
fi

open "$app"

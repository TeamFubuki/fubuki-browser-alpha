#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NATIVE_BUILD_DIR="${NATIVE_BUILD_DIR:-"$ROOT_DIR/native/build"}"
BUILD_TYPE="${BUILD_TYPE:-Release}"

"$ROOT_DIR/scripts/build.sh"

app="$NATIVE_BUILD_DIR/Fubuki Browser Alpha.app"
if [[ ! -d "$app" ]]; then
  app="$NATIVE_BUILD_DIR/$BUILD_TYPE/Fubuki Browser Alpha.app"
fi
if [[ ! -d "$app" ]]; then
  echo "App bundle not found under $NATIVE_BUILD_DIR" >&2
  exit 1
fi

open "$app"

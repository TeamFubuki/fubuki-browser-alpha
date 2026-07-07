#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Cleaning build artifacts..."

rm -rf "$ROOT_DIR/native/build"
rm -rf "$ROOT_DIR/ui/dist"
rm -rf "$ROOT_DIR/target"
rm -rf "$ROOT_DIR/.cache"

echo "Removed: native/build, ui/dist, target, .cache"

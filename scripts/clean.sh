#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

rm -rf "$ROOT_DIR/native/build" "$ROOT_DIR/ui/dist"
echo "Removed native/build and ui/dist"

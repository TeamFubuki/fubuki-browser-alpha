#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_TYPE="${BUILD_TYPE:-release}"

echo "=== Fubuki Browser Alpha - Full Build ==="
echo ""

# 1. UI
echo "[1/3] Building UI..."
"$ROOT_DIR/scripts/build_ui.sh"
echo ""

# 2. Rust (FrostEngine)
echo "[2/3] Building FrostEngine..."
"$ROOT_DIR/scripts/build_rust.sh"
echo ""

# 3. Native (CEF)
echo "[3/3] Building native app..."
if [[ ! -f "${CEF_ROOT:-"$ROOT_DIR/third_party/cef"}/cmake/cef_variables.cmake" ]]; then
  "$ROOT_DIR/scripts/fetch_cef.sh"
fi
"$ROOT_DIR/scripts/build_native.sh"
echo ""

echo "=== Build complete ==="

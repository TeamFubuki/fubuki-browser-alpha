#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_TYPE="${BUILD_TYPE:-release}"

echo "=== Fubuki Browser Alpha - Full Build ==="
echo ""

# 1. UI
echo "[1/4] Building UI..."
"$ROOT_DIR/scripts/build_ui.sh"
echo ""

# 2. Internal pages
echo "[2/4] Building internal pages..."
"$ROOT_DIR/scripts/build_internal_pages.sh"
echo ""

# 3. Rust (FrostEngine)
echo "[3/4] Building FrostEngine..."
"$ROOT_DIR/scripts/build_rust.sh"
echo ""

# 4. Native (CEF)
echo "[4/4] Building native app..."
if [[ ! -f "${CEF_ROOT:-"$ROOT_DIR/third_party/cef"}/cmake/cef_variables.cmake" ]]; then
  "$ROOT_DIR/scripts/fetch_cef.sh"
fi
"$ROOT_DIR/scripts/build_native.sh"
echo ""

echo "=== Build complete ==="

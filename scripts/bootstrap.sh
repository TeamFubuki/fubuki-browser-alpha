#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "=== Fubuki Browser Alpha - Bootstrap ==="
echo ""

# 1. CEF
echo "[1/4] Fetching CEF..."
"$ROOT_DIR/scripts/fetch_cef.sh"
echo ""

# 2. UI dependencies
echo "[2/4] Installing UI dependencies..."
cd "$ROOT_DIR/ui"
pnpm install
echo ""

# 3. Build UI
echo "[3/4] Building UI..."
pnpm run build
echo ""

# 4. Configure native
echo "[4/4] Configuring native build..."
"$ROOT_DIR/scripts/configure_native.sh"
echo ""

echo "=== Bootstrap complete ==="
echo "Run 'make build' to build everything, or 'make run' to build and launch."

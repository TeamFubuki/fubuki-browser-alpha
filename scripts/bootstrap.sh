#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$ROOT_DIR/scripts/fetch_cef.sh"
cd "$ROOT_DIR/ui"
pnpm install
pnpm run build
"$ROOT_DIR/scripts/configure_native.sh"

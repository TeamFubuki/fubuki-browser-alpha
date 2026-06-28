#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$ROOT_DIR/scripts/build_ui.sh"
if [[ ! -f "${CEF_ROOT:-"$ROOT_DIR/third_party/cef"}/cmake/cef_variables.cmake" ]]; then
  "$ROOT_DIR/scripts/fetch_cef.sh"
fi
"$ROOT_DIR/scripts/build_native.sh"

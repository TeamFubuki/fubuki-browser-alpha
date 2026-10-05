#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
args=(fetch --lock-file "$ROOT_DIR/cef.lock" --cef-root "${CEF_ROOT:-$ROOT_DIR/third_party/cef}" --cache-dir "${CACHE_DIR:-$ROOT_DIR/.cache/cef}")

if [[ -n "${CEF_PLATFORM:-}" ]]; then
  args+=(--platform "$CEF_PLATFORM")
fi
if [[ "${FORCE:-0}" == "1" ]]; then
  args+=(--force)
fi

exec python3 "$ROOT_DIR/scripts/cef_lock.py" "${args[@]}"

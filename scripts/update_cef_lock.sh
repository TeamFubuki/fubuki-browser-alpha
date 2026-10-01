#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 "$ROOT_DIR/scripts/cef_lock.py" update \
  --lock-file "$ROOT_DIR/cef.lock" \
  --index-url "${CEF_INDEX_URL:-https://cef-builds.spotifycdn.com/index.json}"

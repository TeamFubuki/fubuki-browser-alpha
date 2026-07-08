#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CEF_ROOT="${CEF_ROOT:-"$ROOT_DIR/third_party/cef"}"
CEF_CHANNEL="${CEF_CHANNEL:-stable}"
CEF_PLATFORM="${CEF_PLATFORM:-}"
FORCE="${FORCE:-0}"
CACHE_DIR="${CACHE_DIR:-"$ROOT_DIR/.cache/cef"}"
INDEX_URL="${CEF_INDEX_URL:-https://cef-builds.spotifycdn.com/index.json}"
BASE_URL="${CEF_BASE_URL:-https://cef-builds.spotifycdn.com}"

if [[ -z "$CEF_PLATFORM" ]]; then
  case "$(uname -m)" in
    arm64) CEF_PLATFORM="macosarm64" ;;
    x86_64) CEF_PLATFORM="macosx64" ;;
    *) echo "Unsupported macOS architecture: $(uname -m)" >&2; exit 1 ;;
  esac
fi

if [[ "$FORCE" != "1" && -f "$CEF_ROOT/cmake/cef_variables.cmake" ]]; then
  echo "CEF already exists at $CEF_ROOT"
  echo "Set FORCE=1 to download and replace it."
  exit 0
fi

command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }
command -v curl >/dev/null || { echo "curl is required" >&2; exit 1; }
command -v tar >/dev/null || { echo "tar is required" >&2; exit 1; }

mkdir -p "$CACHE_DIR"

selection_file="$(mktemp "$CACHE_DIR/selection.XXXXXX.json")"

CEF_PLATFORM="$CEF_PLATFORM" CEF_CHANNEL="$CEF_CHANNEL" INDEX_URL="$INDEX_URL" python3 - <<'PY' > "$selection_file"
import json
import os
import re
import sys
import urllib.request

platform = os.environ["CEF_PLATFORM"]
channel = os.environ["CEF_CHANNEL"]
index_url = os.environ["INDEX_URL"]
archive_pattern = re.compile(rf"cef_binary_[A-Za-z0-9.+_-]+_{re.escape(platform)}\.tar\.bz2")

with urllib.request.urlopen(index_url, timeout=60) as response:
    index = json.load(response)

if platform not in index:
    raise SystemExit(f"CEF platform not found in index: {platform}")

def version_key(version):
    chromium = version.get("chromium_version", "0")
    return tuple(int(part) for part in re.findall(r"\d+", chromium))

def standard_archive(files):
    suffix = f"_{platform}.tar.bz2"
    for item in files:
        name = item.get("name", "")
        if (
            archive_pattern.fullmatch(name)
            and name.endswith(suffix)
            and "_minimal" not in name
            and "_client" not in name
            and "_symbols" not in name
            and "_debug" not in name
        ):
            return name
    return None

candidates = []
for version in index[platform].get("versions", []):
    if version.get("channel") != channel:
        continue
    name = standard_archive(version.get("files", []))
    if name:
        candidates.append((version_key(version), version, name))

if not candidates:
    raise SystemExit(f"No standard CEF archive found for {platform} channel={channel}")

_, version, name = sorted(candidates, key=lambda item: item[0], reverse=True)[0]
# Resolve the archive entry to read its checksum from the index.
archive_entry = next(
    (f for f in version.get("files", []) if f.get("name") == name),
    {},
)
json.dump({
    "cef_file": name,
    "cef_version": version.get("cef_version", ""),
    "chromium_version": version.get("chromium_version", ""),
    "cef_sha1": archive_entry.get("sha1", ""),
}, sys.stdout)
PY

json_get() {
  python3 -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))[sys.argv[2]])' "$selection_file" "$1"
}

CEF_FILE="$(json_get cef_file)"
CEF_URL="${BASE_URL%/}/$CEF_FILE"
CEF_VERSION="$(json_get cef_version)"
CHROMIUM_VERSION="$(json_get chromium_version)"
CEF_SHA1="$(json_get cef_sha1)"
rm -f "$selection_file"

archive="$CACHE_DIR/$CEF_FILE"
echo "Selected CEF: $CEF_VERSION / Chromium $CHROMIUM_VERSION"
echo "Archive: $CEF_FILE"

compute_sha1() {
  local file="$1"
  shasum -a 1 -b "$file" 2>/dev/null | cut -d' ' -f1
}

verify_archive() {
  local file="$1" expected="$2"
  if [[ -z "$expected" ]]; then
    echo "Warning: no checksum published for $file; skipping verification" >&2
    return 0
  fi
  local actual
  actual="$(compute_sha1 "$file")"
  if [[ "$actual" != "$expected" ]]; then
    echo "Checksum mismatch for $file" >&2
    echo "  expected: $expected" >&2
    echo "  actual:   $actual" >&2
    return 1
  fi
  echo "Checksum verified: $actual"
  return 0
}

if [[ ! -f "$archive" ]]; then
  echo "Downloading $CEF_URL"
  curl -fL --retry 3 --retry-delay 2 -o "$archive.tmp" "$CEF_URL"
  mv "$archive.tmp" "$archive"
  if ! verify_archive "$archive" "$CEF_SHA1"; then
    rm -f "$archive"
    exit 1
  fi
else
  echo "Using cached archive $archive"
  verify_archive "$archive" "$CEF_SHA1" || exit 1
fi

tmp_dir="$(mktemp -d "$CACHE_DIR/extract.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT

echo "Extracting CEF"
tar -xjf "$archive" -C "$tmp_dir"
extracted="$(find "$tmp_dir" -maxdepth 1 -type d -name 'cef_binary_*' | head -1)"
if [[ -z "$extracted" || ! -f "$extracted/cmake/cef_variables.cmake" ]]; then
  echo "Downloaded archive did not contain a valid CEF binary distribution" >&2
  exit 1
fi

mkdir -p "$(dirname "$CEF_ROOT")"
rm -rf "$CEF_ROOT"
mv "$extracted" "$CEF_ROOT"

echo "CEF installed to $CEF_ROOT"

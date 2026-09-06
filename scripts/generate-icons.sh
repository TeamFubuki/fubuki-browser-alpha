#!/bin/bash
# Script to generate .icns from PNG icons
# Usage: ./scripts/generate-icons.sh

set -e

ICONS_DIR="$(dirname "$0")/../icons"
RESOURCE_DIR="$(dirname "$0")/../native/resources"

echo "Generating app icons..."

# Create iconset directory
ICONSET_DIR="$ICONS_DIR/fubuki.iconset"
rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

# Use light icon as primary (prioritized per user request)
SOURCE="$ICONS_DIR/icon_light.png"

if [ ! -f "$SOURCE" ]; then
    echo "Error: Source icon not found at $SOURCE"
    exit 1
fi

echo "Using source: $SOURCE"

# Generate all required sizes for macOS
sips -z 16 16 "$SOURCE" --out "$ICONSET_DIR/icon_16x16.png" >/dev/null
sips -z 32 32 "$SOURCE" --out "$ICONSET_DIR/icon_16x16@2x.png" >/dev/null
sips -z 32 32 "$SOURCE" --out "$ICONSET_DIR/icon_32x32.png" >/dev/null
sips -z 64 64 "$SOURCE" --out "$ICONSET_DIR/icon_32x32@2x.png" >/dev/null
sips -z 128 128 "$SOURCE" --out "$ICONSET_DIR/icon_128x128.png" >/dev/null
sips -z 256 256 "$SOURCE" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null
sips -z 256 256 "$SOURCE" --out "$ICONSET_DIR/icon_256x256.png" >/dev/null
sips -z 512 512 "$SOURCE" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null
sips -z 512 512 "$SOURCE" --out "$ICONSET_DIR/icon_512x512.png" >/dev/null
sips -z 1024 1024 "$SOURCE" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null

echo "Generated iconset sizes:"
ls -la "$ICONSET_DIR/"

# Convert to .icns
iconutil -c icns "$ICONSET_DIR" -o "$ICONS_DIR/fubuki.icns"

# Copy to resources
cp "$ICONS_DIR/fubuki.icns" "$RESOURCE_DIR/"

echo ""
echo "✓ Icon generated successfully!"
echo "  - Iconset: $ICONSET_DIR"
echo "  - ICNS: $ICONS_DIR/fubuki.icns"
echo "  - Copied to: $RESOURCE_DIR/fubuki.icns"
echo ""
echo "Rebuild the app with 'make native' to apply changes."

#!/bin/bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ASSETS_DIR="$SCRIPT_DIR/assets/download"
TMP_DIR="/tmp"
ARCHIVE_NAME="logo-assets.tar.gz"
BASE64_NAME="logo-assets.tar.gz.b64"

echo "=== Upload Logo Assets to GitHub Gist ==="
echo ""

# Check if assets/download exists
if [ ! -d "$ASSETS_DIR" ]; then
    echo "ERROR: $ASSETS_DIR directory not found!"
    exit 1
fi

# Show directory info
echo "Directory: $ASSETS_DIR"
echo "Total files: $(find "$ASSETS_DIR" -type f | wc -l | tr -d ' ')"
echo "Total size: $(du -sh "$ASSETS_DIR" | cut -f1)"
echo ""

# Create compressed archive
echo "Creating compressed archive..."
tar czf "$TMP_DIR/$ARCHIVE_NAME" -C "$SCRIPT_DIR/assets" download
echo "Archive size: $(ls -lh "$TMP_DIR/$ARCHIVE_NAME" | awk '{print $5}')"
echo ""

# Encode to base64
echo "Encoding to base64..."
base64 -i "$TMP_DIR/$ARCHIVE_NAME" -o "$TMP_DIR/$BASE64_NAME"
echo "Base64 size: $(ls -lh "$TMP_DIR/$BASE64_NAME" | awk '{print $5}')"
echo ""

# Create private gist
echo "Creating private GitHub Gist..."
GIST_URL=$(gh gist create "$TMP_DIR/$BASE64_NAME" -d "Logo assets for chama-optics CI cache (base64 encoded tar.gz)")

if [ -z "$GIST_URL" ]; then
    echo "ERROR: Failed to create gist"
    exit 1
fi

echo ""
echo "✓ Gist created successfully!"
echo ""
echo "Gist URL: $GIST_URL"
echo ""

# Extract gist ID from URL
GIST_ID=$(basename "$GIST_URL")
RAW_URL="https://gist.githubusercontent.com/$(gh api user --jq .login)/$GIST_ID/raw/$BASE64_NAME"

echo "Raw URL for GitHub Actions:"
echo "$RAW_URL"
echo ""
echo "=== Next Steps ==="
echo "1. Go to your repository's Actions tab"
echo "2. Select 'Manual Upload Logo Assets' workflow"
echo "3. Click 'Run workflow'"
echo "4. Paste the Raw URL above into the 'download_url' field"
echo "5. Click 'Run workflow'"
echo ""

# Cleanup
rm -f "$TMP_DIR/$ARCHIVE_NAME" "$TMP_DIR/$BASE64_NAME"
echo "Temporary files cleaned up."

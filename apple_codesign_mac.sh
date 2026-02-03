#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: MIT
#
# Apple Developer Certificate Code Signing Script
# For CI/CD with proper notarization support
#
# Required Environment Variables:
#   APPLE_CERT_B64            - Base64 encoded .p12 certificate file
#   APPLE_CERT_PASSWORD       - Password for the .p12 file
#   APPLE_TEAM_ID             - Apple Developer Team ID
#
# Optional (for notarization):
#   APPLE_ID_EMAIL            - Apple ID email address
#   APPLE_ID_APP_PASSWORD     - App-specific password for notarization
#
# Usage:
#   export APPLE_CERT_B64="..."
#   export APPLE_CERT_PASSWORD="..."
#   export APPLE_TEAM_ID="..."
#   ./apple_codesign_mac.sh

set -e

# ---- Config ----
APP_PATH="target/release/bundle/osx/Chama Optics.app"
APP_NAME="Chama Optics"

# Use different keychain name to avoid conflicts with self-signed script
KEYCHAIN_NAME="${APPLE_KEYCHAIN_NAME:-apple.signing.keychain}"
KEYCHAIN_PASSWORD="${APPLE_KEYCHAIN_PASSWORD:-$(openssl rand -base64 32)}"

BACKGROUND_IMG="./assets/background.jpg"
MOUNT_DIR="/tmp/${APP_NAME}_dmg_mount"
STAGING_DIR="/tmp/${APP_NAME}_dmg_staging"
DMG_RW="/tmp/${APP_NAME}-temp.dmg"
FINAL_DMG="ChamaOptics-arm64.dmg"

# Base window size for DMG
BASE_WINDOW_WIDTH=800
BASE_WINDOW_HEIGHT=500

# ---- Helpers ----
info()  { echo ">> $*"; }
warn()  { echo "!! $*" >&2; }
fatal() { echo "!! $*" >&2; exit 1; }

cleanup() {
    info "Cleaning up..."
    rm -f apple_cert.p12
    security delete-keychain "$KEYCHAIN_NAME" 2>/dev/null || true
}
trap cleanup EXIT

# ---- 0. Validate required environment variables ----
if [ -z "${APPLE_CERT_B64:-}" ]; then
    fatal "APPLE_CERT_B64 environment variable is required"
fi
if [ -z "${APPLE_CERT_PASSWORD:-}" ]; then
    fatal "APPLE_CERT_PASSWORD environment variable is required"
fi
if [ -z "${APPLE_TEAM_ID:-}" ]; then
    fatal "APPLE_TEAM_ID environment variable is required"
fi

# Check for notarization capability
ENABLE_NOTARIZATION=false
if [ -n "${APPLE_ID_EMAIL:-}" ] && [ -n "${APPLE_ID_APP_PASSWORD:-}" ]; then
    ENABLE_NOTARIZATION=true
    info "Notarization: ENABLED"
else
    warn "Notarization: DISABLED (set APPLE_ID_EMAIL and APPLE_ID_APP_PASSWORD to enable)"
fi

# ---- 1. Check app exists ----
if [ ! -d "$APP_PATH" ]; then
    fatal "App not found at $APP_PATH — Build first with 'cargo bundle --release'"
fi

# ---- 2. Ensure retina display support in Info.plist ----
info "Updating Info.plist for retina display support..."
INFO_PLIST="$APP_PATH/Contents/Info.plist"

if ! /usr/libexec/PlistBuddy -c "Print NSHighResolutionCapable" "$INFO_PLIST" >/dev/null 2>&1; then
    /usr/libexec/PlistBuddy -c "Add :NSHighResolutionCapable bool true" "$INFO_PLIST"
else
    /usr/libexec/PlistBuddy -c "Set :NSHighResolutionCapable true" "$INFO_PLIST"
fi

if ! /usr/libexec/PlistBuddy -c "Print NSSupportsAutomaticGraphicsSwitching" "$INFO_PLIST" >/dev/null 2>&1; then
    /usr/libexec/PlistBuddy -c "Add :NSSupportsAutomaticGraphicsSwitching bool true" "$INFO_PLIST"
else
    /usr/libexec/PlistBuddy -c "Set :NSSupportsAutomaticGraphicsSwitching true" "$INFO_PLIST"
fi

if ! /usr/libexec/PlistBuddy -c "Print NSRequiresAquaSystemAppearance" "$INFO_PLIST" >/dev/null 2>&1; then
    /usr/libexec/PlistBuddy -c "Add :NSRequiresAquaSystemAppearance bool false" "$INFO_PLIST"
fi

# ---- 3. Decode and import Apple certificate ----
info "Decoding Apple Developer certificate..."
echo "$APPLE_CERT_B64" | base64 --decode > apple_cert.p12

info "Creating temporary keychain: $KEYCHAIN_NAME"
security delete-keychain "$KEYCHAIN_NAME" 2>/dev/null || true
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security set-keychain-settings -lut 21600 "$KEYCHAIN_NAME"

info "Importing certificate into keychain..."
security import apple_cert.p12 -P "$APPLE_CERT_PASSWORD" \
    -A -t cert -f pkcs12 -k "$KEYCHAIN_NAME"

security set-key-partition-list -S apple-tool:,apple: \
    -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME" || warn "partition-list failed (may still work)"

# Add to keychain search list
security list-keychain -d user -s "$KEYCHAIN_NAME" $(security list-keychains -d user | tr -d '"')

rm -f apple_cert.p12

# ---- 4. Find signing identity ----
info "Finding signing identity..."
IDENTITY=$(security find-identity -v -p codesigning "$KEYCHAIN_NAME" | grep "Developer ID Application" | head -n1 | awk -F'"' '{print $2}')

if [ -z "$IDENTITY" ]; then
    # Try finding any valid identity
    IDENTITY=$(security find-identity -v -p codesigning "$KEYCHAIN_NAME" | grep -v "CSSMERR" | head -n1 | awk -F'"' '{print $2}')
fi

if [ -z "$IDENTITY" ]; then
    security find-identity -v -p codesigning "$KEYCHAIN_NAME"
    fatal "No valid signing identity found in the certificate"
fi

info "Using identity: $IDENTITY"

# ---- 5. Sign frameworks/dylibs (with hardened runtime for notarization) ----
CODESIGN_OPTS="--force --options runtime --timestamp"

if [ -d "$APP_PATH/Contents/Frameworks" ]; then
    info "Signing frameworks and dylibs..."
    find "$APP_PATH/Contents/Frameworks" -type f \( -name "*.dylib" -o -name "*.framework" \) -print0 | while IFS= read -r -d '' lib; do
        info "  Signing: $lib"
        basename_lib="$(basename "$lib")"
        if [[ "$lib" == *.dylib ]]; then
            install_name_tool -id "@executable_path/../Frameworks/$basename_lib" "$lib" 2>/dev/null || true
        fi
        codesign $CODESIGN_OPTS --sign "$IDENTITY" --keychain "$KEYCHAIN_NAME" "$lib" || warn "codesign failed for $lib"
    done
fi

# ---- 6. Sign main binary and app bundle ----
info "Signing main executable..."
MAIN_BIN="$APP_PATH/Contents/MacOS/$(ls "$APP_PATH/Contents/MacOS" | head -n1)"
codesign $CODESIGN_OPTS --sign "$IDENTITY" --keychain "$KEYCHAIN_NAME" "$MAIN_BIN"

info "Signing app bundle (deep)..."
codesign $CODESIGN_OPTS --deep --sign "$IDENTITY" --keychain "$KEYCHAIN_NAME" "$APP_PATH"

# Verify signature
info "Verifying signature..."
codesign --verify --deep --strict --verbose=2 "$APP_PATH" || warn "Verification reported issues"

# ---- 7. Notarization submission (async - does not wait) ----
SUBMISSION_ID=""
if [ "$ENABLE_NOTARIZATION" = true ]; then
    info "Creating ZIP for notarization..."
    NOTARIZE_ZIP="/tmp/${APP_NAME}-notarize.zip"
    ditto -c -k --keepParent "$APP_PATH" "$NOTARIZE_ZIP"

    info "Submitting for notarization (async mode - will not wait)..."
    SUBMIT_OUTPUT=$(xcrun notarytool submit "$NOTARIZE_ZIP" \
        --apple-id "$APPLE_ID_EMAIL" \
        --password "$APPLE_ID_APP_PASSWORD" \
        --team-id "$APPLE_TEAM_ID" 2>&1)

    echo "$SUBMIT_OUTPUT"

    # Extract submission ID from output
    SUBMISSION_ID=$(echo "$SUBMIT_OUTPUT" | grep -i "id:" | head -n1 | awk '{print $NF}')

    if [ -n "$SUBMISSION_ID" ]; then
        info "Notarization submitted successfully!"
        info "Submission ID: $SUBMISSION_ID"

        # Save submission ID to file for CI/CD pickup
        echo "$SUBMISSION_ID" > notarization_submission_id.txt
        info "Submission ID saved to: notarization_submission_id.txt"

        # Also output in GitHub Actions format
        if [ -n "${GITHUB_OUTPUT:-}" ]; then
            echo "submission_id=$SUBMISSION_ID" >> "$GITHUB_OUTPUT"
        fi
    else
        warn "Could not extract submission ID from output"
        warn "You may need to check manually with: xcrun notarytool history"
    fi

    rm -f "$NOTARIZE_ZIP"

    info ""
    info "NOTE: Notarization is processing asynchronously."
    info "Check status later with:"
    info "  xcrun notarytool info $SUBMISSION_ID --apple-id \$APPLE_ID_EMAIL --password \$APPLE_ID_APP_PASSWORD --team-id \$APPLE_TEAM_ID"
    info ""
    info "After notarization is accepted, staple with:"
    info "  xcrun stapler staple \"$APP_PATH\""
else
    warn "Skipping notarization (credentials not provided)"
fi

# ---- 8. Create DMG ----
info "Creating DMG..."
rm -rf "$MOUNT_DIR" "$STAGING_DIR" "$DMG_RW" "$FINAL_DMG"
mkdir -p "$STAGING_DIR"

cp -R "$APP_PATH" "$STAGING_DIR/"
ln -s /Applications "$STAGING_DIR/Applications"

hdiutil create -volname "$APP_NAME" \
    -srcfolder "$STAGING_DIR" \
    -ov -format UDRW "$DMG_RW"

DEVICE=$(hdiutil attach -readwrite -noverify -noautoopen "$DMG_RW" | grep '^/dev/' | head -n1 | awk '{print $1}')

mkdir -p "/Volumes/$APP_NAME/.background"

# Handle background image
WINDOW_WIDTH=$BASE_WINDOW_WIDTH
WINDOW_HEIGHT=$BASE_WINDOW_HEIGHT
BACKGROUND_IMG_2X="${BACKGROUND_IMG%.*}@2x.${BACKGROUND_IMG##*.}"
TMP_BG_STD="/tmp/dmg_bg_std.jpg"
TMP_BG_2X="/tmp/dmg_bg_2x.jpg"
TMP_TIFF="/tmp/dmg_bg.tiff"

sips -z "$WINDOW_HEIGHT" "$WINDOW_WIDTH" "$BACKGROUND_IMG" --out "$TMP_BG_STD" 2>/dev/null || cp "$BACKGROUND_IMG" "$TMP_BG_STD"

if [ -f "$BACKGROUND_IMG_2X" ]; then
    BG_2X_WIDTH=$((WINDOW_WIDTH * 2))
    BG_2X_HEIGHT=$((WINDOW_HEIGHT * 2))
    sips -z "$BG_2X_HEIGHT" "$BG_2X_WIDTH" "$BACKGROUND_IMG_2X" --out "$TMP_BG_2X" 2>/dev/null || cp "$BACKGROUND_IMG_2X" "$TMP_BG_2X"
    tiffutil -cathidpicheck "$TMP_BG_STD" "$TMP_BG_2X" -out "$TMP_TIFF"
    cp "$TMP_TIFF" "/Volumes/$APP_NAME/.background/background.tiff"
else
    cp "$TMP_BG_STD" "/Volumes/$APP_NAME/.background/background.tiff"
fi

rm -f "$TMP_BG_STD" "$TMP_BG_2X" "$TMP_TIFF"

# Configure Finder layout
osascript <<EOF
tell application "Finder"
  tell disk "$APP_NAME"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set the bounds of container window to {100, 100, $(($WINDOW_WIDTH + 100)), $(($WINDOW_HEIGHT + 100))}
    set viewOptions to the icon view options of container window
    set arrangement of viewOptions to not arranged
    set icon size of viewOptions to 128
    set background picture of viewOptions to file ".background:background.tiff"
    set label position of viewOptions to bottom
    set shows icon preview of viewOptions to false
    set position of item "$APP_NAME.app" of container window to {275, 260}
    set position of item "Applications" of container window to {525, 260}
    update without registering applications
    delay 2
    close
    open
    delay 2
    eject
  end tell
end tell
EOF

hdiutil detach "$DEVICE"
hdiutil convert "$DMG_RW" -format UDZO -imagekey zlib-level=9 -o "$FINAL_DMG"

rm -rf "$MOUNT_DIR" "$STAGING_DIR" "$DMG_RW"

echo ""
echo "============================================================"
echo "[SUCCESS] Apple-signed DMG created: $FINAL_DMG"
if [ "$ENABLE_NOTARIZATION" = true ] && [ -n "$SUBMISSION_ID" ]; then
    echo "          Notarization: SUBMITTED (async)"
    echo "          Submission ID: $SUBMISSION_ID"
    echo ""
    echo "          DMG is NOT stapled yet."
    echo "          After Apple approves, run Phase 2 workflow to staple and publish."
elif [ "$ENABLE_NOTARIZATION" = true ]; then
    echo "          Notarization: SUBMISSION FAILED"
else
    echo "          Notarization: SKIPPED"
fi
echo "============================================================"

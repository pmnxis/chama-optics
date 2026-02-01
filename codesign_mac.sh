#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: MIT
# set -e

KEYCHAIN_NAME="${KEYCHAIN_NAME:-build.keychain.haatonworld}"
KEYCHAIN_PASSWORD="${KEYCHAIN_PASSWORD:-temporaryPassWord1234}"

# ---- Config ----
APP_PATH="target/release/bundle/osx/Chama Optics.app"
APP_NAME="Chama Optics"

KEY_FILE="cert.key"     # private (PEM)
CRT_FILE="cert.crt"     # cenrtification (PEM)
P12_FILE="cert.p12"     # generated PKCS#12 temporary
P12_PASS="${P12_PASS:-p12pass}"      # p12 password, from env or default
DMG_NAME="${APP_NAME}.dmg"
ZIP_NAME="${APP_NAME}.zip"

IDENTITY_CN="ChamaOptics Self Sign"

BACKGROUND_IMG="./assets/background.jpg"
MOUNT_DIR="/tmp/${APP_NAME}_dmg_mount"
STAGING_DIR="/tmp/${APP_NAME}_dmg_staging"
DMG_RW="/tmp/${APP_NAME}-temp.dmg"
FINAL_DMG="ChamaOptics-arm64.dmg"

# Base window size for DMG (for standard displays)
# Will be doubled for retina displays
BASE_WINDOW_WIDTH=800
BASE_WINDOW_HEIGHT=500

# ---- Helpers ----
info()  { echo ">> $*"; }
warn()  { echo "!! $*" >&2; }
fatal() { echo "!! $*" >&2; exit 1; }

# ---- 0. quick checks ----
if [ ! -d "$APP_PATH" ]; then
  fatal "App not found at $APP_PATH — Build first and make .app"
fi

# ---- 0.5. Ensure retina display support in Info.plist ----
info "Checking and updating Info.plist for retina display support..."
INFO_PLIST="$APP_PATH/Contents/Info.plist"

# Ensure NSHighResolutionCapable is set
if ! /usr/libexec/PlistBuddy -c "Print NSHighResolutionCapable" "$INFO_PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c "Add :NSHighResolutionCapable bool true" "$INFO_PLIST"
  info "Added NSHighResolutionCapable = true"
else
  /usr/libexec/PlistBuddy -c "Set :NSHighResolutionCapable true" "$INFO_PLIST"
  info "Ensured NSHighResolutionCapable = true"
fi

# Ensure NSSupportsAutomaticGraphicsSwitching is set (better battery life on MacBook Pro)
if ! /usr/libexec/PlistBuddy -c "Print NSSupportsAutomaticGraphicsSwitching" "$INFO_PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c "Add :NSSupportsAutomaticGraphicsSwitching bool true" "$INFO_PLIST"
  info "Added NSSupportsAutomaticGraphicsSwitching = true"
else
  /usr/libexec/PlistBuddy -c "Set :NSSupportsAutomaticGraphicsSwitching true" "$INFO_PLIST"
  info "Ensured NSSupportsAutomaticGraphicsSwitching = true"
fi

# Ensure NSRequiresAquaSystemAppearance is set (proper rendering on light/dark mode)
if ! /usr/libexec/PlistBuddy -c "Print NSRequiresAquaSystemAppearance" "$INFO_PLIST" >/dev/null 2>&1; then
  /usr/libexec/PlistBuddy -c "Add :NSRequiresAquaSystemAppearance bool false" "$INFO_PLIST"
  info "Added NSRequiresAquaSystemAppearance = false"
fi

# ---- 1. Create key & cert if not present (but respect existing files) ----
if [ -n "${CERT_KEY_B64:-}" ] && [ -n "${CERT_CRT_B64:-}" ]; then
  info "Using certificate from environment variables (CERT_KEY_B64/CERT_CRT_B64)..."
  echo "$CERT_KEY_B64" | base64 --decode > "$KEY_FILE"
  echo "$CERT_CRT_B64" | base64 --decode > "$CRT_FILE"
elif [ -f "$KEY_FILE" ] && [ -f "$CRT_FILE" ]; then
  info "Using local self-signed certificate (found $KEY_FILE and $CRT_FILE)."
else
  info "No local cert/key found — generating a self-signed code-signing certificate..."
  # Create openssl config with code signing EKU
  TMP_OPENSSL_CONF="$(mktemp)"
  cat > "$TMP_OPENSSL_CONF" <<EOF
[req]
distinguished_name = req_distinguished_name
x509_extensions = v3_req
prompt = no

[req_distinguished_name]
CN = ${IDENTITY_CN}

[v3_req]
basicConstraints = CA:FALSE
keyUsage = digitalSignature, keyEncipherment
extendedKeyUsage = codeSigning
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid,issuer
EOF

  # Generate key and cert (10 year)
  openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$KEY_FILE"
  openssl req -new -x509 -days 3650 -key "$KEY_FILE" -out "$CRT_FILE" -config "$TMP_OPENSSL_CONF" -extensions v3_req

  rm -f "$TMP_OPENSSL_CONF"
  info "Generated $KEY_FILE and $CRT_FILE"
fi

# ---- 2. Create PKCS#12 bundle for import (security prefers p12 for private key import) ----
info "Creating PKCS#12 bundle ($P12_FILE)..."
# Use -legacy for OpenSSL 3.x compatibility
if openssl version | grep -q "OpenSSL 3"; then
  openssl pkcs12 -export -legacy -inkey "$KEY_FILE" -in "$CRT_FILE" -out "$P12_FILE" -passout pass:"$P12_PASS" -name "$IDENTITY_CN"
else
  openssl pkcs12 -export -inkey "$KEY_FILE" -in "$CRT_FILE" -out "$P12_FILE" -passout pass:"$P12_PASS" -name "$IDENTITY_CN"
fi

# ---- 3. Create temporary keychain and import cert/key ----
info "Creating temporary keychain: $KEYCHAIN_NAME"
security delete-keychain "$KEYCHAIN_NAME" >/dev/null 2>&1 || true
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security set-keychain-settings -t 3600 -u "$KEYCHAIN_NAME"
# make it default for this session
security list-keychains -s "$KEYCHAIN_NAME"

info "Importing PKCS#12 into keychain..."
security import "$P12_FILE" -k "$KEYCHAIN_NAME" -P "$P12_PASS" -T /usr/bin/codesign -T /usr/bin/productbuild || warn "security import p12 returned non-zero"

# Also import cert (so it's visible)
security import "$CRT_FILE" -k "$KEYCHAIN_NAME" -T /usr/bin/codesign -A || warn "security import crt returned non-zero"

# ensure partition list so codesign can use key
security set-key-partition-list -S apple-tool:,apple: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME" || warn "partition-list failed (may still work)"

# unlock again
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"

# ---- 4. Find identity (CN) ----
IDENTITY_HEX=$(security find-identity -p codesigning "$KEYCHAIN_NAME" | grep "$IDENTITY_CN" | head -n1 | awk '{print $2}' || true)
if [ -z "$IDENTITY_HEX" ]; then
  warn "No identity fingerprint found for CN='$IDENTITY_CN'. We'll try using the CN directly."
  IDENTITY="$IDENTITY_CN"
else
  IDENTITY="$IDENTITY_HEX"
fi
info "Using identity: $IDENTITY"

# ---- 5. Sign frameworks / dylibs first (no hardened runtime) ----
if [ -d "$APP_PATH/Contents/Frameworks" ]; then
  info "Signing frameworks and dylibs..."
  find "$APP_PATH/Contents/Frameworks" -type f \( -name "*.dylib" -o -name "*.framework" \) -print0 | while IFS= read -r -d '' lib; do
    info "  Signing: $lib"
    # adjust install name to be relative inside bundle (best-effort)
    basename_lib="$(basename "$lib")"
    # try to set id only for dylib (ignore failures)
    if [[ "$lib" == *.dylib ]]; then
      install_name_tool -id "@executable_path/../Frameworks/$basename_lib" "$lib" >/dev/null 2>&1 || true
    fi
    # sign (do not use --options runtime)
    if ! codesign --force --verbose --sign "$IDENTITY" --keychain "$KEYCHAIN_NAME" "$lib"; then
      warn "codesign failed for $lib (continuing)"
    fi
  done
fi

# ---- 6. Sign main binary and then app bundle (no --options runtime) ----
info "Signing main executable and app bundle (no hardened runtime)..."
MAIN_BIN="$APP_PATH/Contents/MacOS/$(ls "$APP_PATH/Contents/MacOS" | head -n1)"
info "Main binary: $MAIN_BIN"

# sign the executable
if ! codesign --force --verbose --sign "$IDENTITY" --keychain "$KEYCHAIN_NAME" "$MAIN_BIN"; then
  warn "Failed to sign main binary (continuing)"
fi

# deep sign the whole bundle
if ! codesign --force --deep --verbose --sign "$IDENTITY" --keychain "$KEYCHAIN_NAME" "$APP_PATH"; then
  warn "Failed to deep-sign app bundle (continuing)"
fi

# verify (non-fatal)
if ! codesign --verify --deep --strict --verbose=2 "$APP_PATH"; then
  warn "codesign verification reported issues — app may still behave as 'unidentified developer' (users can allow via Security settings)."
fi

# ---- 7. Package: create DMG and ZIP for distribution ----
# ========== Cleanup ==========
echo "Cleaning up previous build..."
rm -rf "$MOUNT_DIR" "$STAGING_DIR" "$DMG_RW" "$FINAL_DMG"
mkdir -p "$STAGING_DIR"

# ========== Prepare DMG content ==========
echo "Copying app bundle..."
cp -R "$APP_PATH" "$STAGING_DIR/"
ln -s /Applications "$STAGING_DIR/Applications"

# ========== Create temporary DMG ==========
echo "Creating temporary DMG..."
hdiutil create -volname "$APP_NAME" \
  -srcfolder "$STAGING_DIR" \
  -ov -format UDRW "$DMG_RW"

# ========== Mount DMG ==========
echo "Mounting temporary DMG..."
DEVICE=$(hdiutil attach -readwrite -noverify -noautoopen "$DMG_RW" | grep '^/dev/' | head -n1 | awk '{print $1}')

echo "Copying background image..."
mkdir -p "/Volumes/$APP_NAME/.background"

# Window: 800x500 points (AppleScript)
# Background will be created as multi-resolution TIFF
WINDOW_WIDTH=$BASE_WINDOW_WIDTH
WINDOW_HEIGHT=$BASE_WINDOW_HEIGHT
BG_WIDTH=$WINDOW_WIDTH
BG_HEIGHT=$WINDOW_HEIGHT

echo "Configuring window for ${WINDOW_WIDTH}x${WINDOW_HEIGHT} points"
echo "Creating multi-resolution TIFF for retina support"

# Check for @2x version for retina support
BACKGROUND_IMG_2X="${BACKGROUND_IMG%.*}@2x.${BACKGROUND_IMG##*.}"
TMP_BG_STD="/tmp/dmg_bg_std.jpg"
TMP_BG_2X="/tmp/dmg_bg_2x.jpg"
TMP_TIFF="/tmp/dmg_bg.tiff"

# Ensure standard background is exactly 800x500
ORIG_WIDTH=$(sips -g pixelWidth "$BACKGROUND_IMG" | grep pixelWidth | awk '{print $2}')
ORIG_HEIGHT=$(sips -g pixelHeight "$BACKGROUND_IMG" | grep pixelHeight | awk '{print $2}')
echo "Standard background: ${ORIG_WIDTH}x${ORIG_HEIGHT}"

if [ "$ORIG_WIDTH" -eq "$BG_WIDTH" ] && [ "$ORIG_HEIGHT" -eq "$BG_HEIGHT" ]; then
    cp "$BACKGROUND_IMG" "$TMP_BG_STD"
else
    echo "Scaling standard background to ${BG_WIDTH}x${BG_HEIGHT}"
    sips -z "$BG_HEIGHT" "$BG_WIDTH" "$BACKGROUND_IMG" --out "$TMP_BG_STD"
fi

# Check for @2x version
if [ -f "$BACKGROUND_IMG_2X" ]; then
    echo "Found @2x retina version"
    ORIG_2X_WIDTH=$(sips -g pixelWidth "$BACKGROUND_IMG_2X" | grep pixelWidth | awk '{print $2}')
    ORIG_2X_HEIGHT=$(sips -g pixelHeight "$BACKGROUND_IMG_2X" | grep pixelHeight | awk '{print $2}')
    BG_2X_WIDTH=$((WINDOW_WIDTH * 2))
    BG_2X_HEIGHT=$((WINDOW_HEIGHT * 2))
    echo "Retina background: ${ORIG_2X_WIDTH}x${ORIG_2X_HEIGHT} -> ${BG_2X_WIDTH}x${BG_2X_HEIGHT}"
    
    if [ "$ORIG_2X_WIDTH" -eq "$BG_2X_WIDTH" ] && [ "$ORIG_2X_HEIGHT" -eq "$BG_2X_HEIGHT" ]; then
        cp "$BACKGROUND_IMG_2X" "$TMP_BG_2X"
    else
        sips -z "$BG_2X_HEIGHT" "$BG_2X_WIDTH" "$BACKGROUND_IMG_2X" --out "$TMP_BG_2X"
    fi
    
    # Create multi-resolution TIFF using tiffutil
    echo "Creating multi-resolution TIFF..."
    tiffutil -cathidpicheck "$TMP_BG_STD" "$TMP_BG_2X" -out "$TMP_TIFF"
    cp "$TMP_TIFF" "/Volumes/$APP_NAME/.background/background.tiff"
    echo "Multi-resolution TIFF created successfully"
    
    # Print TIFF file size
    TIFF_SIZE=$(du -h "/Volumes/$APP_NAME/.background/background.tiff" | awk '{print $1}')
    echo "TIFF file size: $TIFF_SIZE"
else
    # No @2x version, just use standard
    echo "No @2x version found, using standard resolution"
    cp "$TMP_BG_STD" "/Volumes/$APP_NAME/.background/background.tiff"
fi

# Cleanup temp files
rm -f "$TMP_BG_STD" "$TMP_BG_2X" "$TMP_TIFF"

# ========== Set Finder layout ==========
echo "Configuring Finder window layout..."
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

    -- icon positions (centered in window with shorter spacing)
    set position of item "$APP_NAME.app" of container window to {275, 260}
    set position of item "Applications" of container window to {525, 260}

    -- Set text color to white by setting window's label position
    set theViewOptions to the icon view options of container window
    set the position of theViewOptions's label position to bottom
    set theViewOptions's label position to bottom

    update without registering applications
    delay 2
    close
    open
    delay 2
    eject
  end tell
end tell
EOF

# ========== Detach DMG ==========
echo "Detaching DMG..."
hdiutil detach "$DEVICE"

# ========== Convert to final compressed DMG ==========
echo "Compressing final DMG..."
hdiutil convert "$DMG_RW" -format UDZO -imagekey zlib-level=9 -o "$FINAL_DMG"

# ========== Cleanup ==========
echo "Cleaning up temporary files..."
rm -rf "$MOUNT_DIR" "$STAGING_DIR" "$DMG_RW"

echo "[SUCCESS] DMG created successfully: $FINAL_DMG"

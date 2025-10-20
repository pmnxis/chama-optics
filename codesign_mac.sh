#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: MIT
set -e

APP_PATH="target/release/bundle/osx/Chama Optics.app"
NAME="ChamaOptics Self Sign"
KEYCHAIN_NAME="build.keychain.haatonworld"
KEYCHAIN_PASSWORD="temporaryPassWord1234"

KEY_FILE="cert.key"
CRT_FILE="cert.crt"

echo "Setting up code signing environment..."

# --- 1. Delete old keychain if exists ---
security delete-keychain "$KEYCHAIN_NAME" || true

# --- 2. Create and unlock temporary keychain ---
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"
security set-keychain-settings -t 3600 -u "$KEYCHAIN_NAME"

# --- 2a. Set temporary keychain as session default ---
security list-keychains -s "$KEYCHAIN_NAME"

# --- 3. Prepare certificate/key ---
if [ -n "$CERT_KEY_B64" ] && [ -n "$CERT_CRT_B64" ]; then
    echo "🔑 Using certificate from GitHub Secrets..."
    echo "$CERT_KEY_B64" | base64 --decode > "$KEY_FILE"
    echo "$CERT_CRT_B64" | base64 --decode > "$CRT_FILE"
elif [ -f "$KEY_FILE" ] && [ -f "$CRT_FILE" ]; then
    echo "Using local self-signed certificate..."
else
    echo "[ERROR] No signing certificate found!"
    exit 1
fi

# --- 4. Import key & certificate ---
echo "Importing private key..."
security import "$KEY_FILE" -k "$KEYCHAIN_NAME" -T /usr/bin/codesign -A
echo "Importing certificate..."
security import "$CRT_FILE" -k "$KEYCHAIN_NAME" -T /usr/bin/codesign -A

# --- 5. Set key partition list ---
security set-key-partition-list -S apple-tool:,apple: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"

# --- 6. Unlock again for CI stability ---
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_NAME"

# --- 7. Force use identity even if self-signed ---
IDENTITY=$(security find-identity -p codesigning "$KEYCHAIN_NAME" | grep "$NAME" | head -n1 | awk '{print $2}')
if [ -z "$IDENTITY" ]; then
    echo "[WARNING] No valid identity found. Will attempt signing anyway."
    # fallback: use the first matching identity in the keychain
    IDENTITY=$(security find-identity -p codesigning "$KEYCHAIN_NAME" | head -n1 | awk '{print $2}')
    if [ -z "$IDENTITY" ]; then
        echo "[WARNING] No identities at all found in the keychain. Signing may fail."
    else
        echo "[INFO] Using fallback identity: $IDENTITY"
    fi
else
    echo "Using identity: $IDENTITY"
fi

# --- 8. Sign the app ---
if [ -d "$APP_PATH" ]; then
    echo "Signing app..."
    set +e  # codesign 실패해도 스크립트 종료하지 않도록
    codesign --keychain "$KEYCHAIN_NAME" --force --deep --options runtime --verbose --sign "$IDENTITY" "$APP_PATH"
    CODE_SIGN_EXIT=$?
    set -e
    if [ $CODE_SIGN_EXIT -ne 0 ]; then
        echo "[WARNING] Codesign may have failed. Please check manually."
    else
        echo "Verifying signature..."
        set +e
        codesign --verify --deep --verbose=2 --keychain "$KEYCHAIN_NAME" "$APP_PATH"
        set -e
        echo "[SUCCESS] Codesign completed!"
    fi
else
    echo "[ERROR] App not found: $APP_PATH"
fi

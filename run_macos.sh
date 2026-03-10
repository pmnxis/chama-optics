#!/usr/bin/env bash
# Run chama-optics in a bundle-like directory structure for macOS development testing.
# This mirrors the layout produced by `cargo bundle` / build_mac.sh so that
# resource lookup (Contents/MacOS/../Resources) works identically.

set -e

FEATURES="face_detection_insightface,face_detection_visionkit,ext_res"
BUILD_MODE="release"

# Parse arguments
for arg in "$@"; do
    case "$arg" in
        --release) BUILD_MODE="release" ;;
    esac
done

BUILD_FLAG=""
if [ "$BUILD_MODE" = "release" ]; then
    BUILD_FLAG="--release"
fi

echo "=== Building ($BUILD_MODE) with features: $FEATURES ==="
cargo build $BUILD_FLAG --features "$FEATURES"

# Mirror the app bundle directory layout:
#   target/<mode>/bundle-dev/Contents/MacOS/chama-optics
#   target/<mode>/bundle-dev/Contents/Resources/{Models,Fonts,Logos}
BUNDLE_ROOT="target/$BUILD_MODE/bundle-dev/Contents"
MACOS_DIR="$BUNDLE_ROOT/MacOS"
RES_DIR="$BUNDLE_ROOT/Resources"

mkdir -p "$MACOS_DIR"
mkdir -p "$RES_DIR/Models"
mkdir -p "$RES_DIR/Fonts"
mkdir -p "$RES_DIR/Logos"

# Copy binary
cp "target/$BUILD_MODE/chama-optics" "$MACOS_DIR/"

# --- Resources ---

# 1. Models
MODEL_SOURCE=$(find target -name "det_10g.onnx" -not -path "*/bundle-dev/*" 2>/dev/null | head -1)
if [ -z "$MODEL_SOURCE" ] && [ -f "assets/download/det_10g.onnx" ]; then
    MODEL_SOURCE="assets/download/det_10g.onnx"
fi
if [ -n "$MODEL_SOURCE" ] && [ -f "$MODEL_SOURCE" ]; then
    cp "$MODEL_SOURCE" "$RES_DIR/Models/"
    echo "[OK] det_10g.onnx"
else
    echo "[WARN] det_10g.onnx not found"
fi

# 2. Fonts
if [ -d "assets/fonts" ]; then
    for font in assets/fonts/*.{ttf,otf,ttc}; do
        [ -f "$font" ] && cp "$font" "$RES_DIR/Fonts/"
    done
fi
for build_font in DynaPuff-Variable.ttf "digital-7.ttf" "digital-7 (italic).ttf"; do
    SRC=$(find target -name "$build_font" -not -path "*/bundle-dev/*" 2>/dev/null | head -1)
    [ -n "$SRC" ] && [ -f "$SRC" ] && cp "$SRC" "$RES_DIR/Fonts/"
done
FONT_COUNT=$(ls "$RES_DIR/Fonts/"*.{ttf,otf,ttc} 2>/dev/null | wc -l | xargs)
echo "[OK] $FONT_COUNT font(s)"

# 3. Logos
LOGO_DIR=$(find target -type d -name "download" -not -path "*/bundle-dev/*" 2>/dev/null | head -1)
if [ -z "$LOGO_DIR" ] && [ -d "assets/logo_mnf" ]; then
    LOGO_DIR="assets/logo_mnf"
elif [ -z "$LOGO_DIR" ] && [ -d "assets/download" ]; then
    LOGO_DIR="assets/download"
fi
if [ -n "$LOGO_DIR" ] && [ -d "$LOGO_DIR" ]; then
    for logo in "$LOGO_DIR"/*.svg; do
        [ -f "$logo" ] && cp "$logo" "$RES_DIR/Logos/"
    done
fi
LOGO_COUNT=$(ls "$RES_DIR/Logos/"*.svg 2>/dev/null | wc -l | xargs)
echo "[OK] $LOGO_COUNT logo(s)"

echo ""
echo "=== Running from bundle-like path ==="
echo "  $MACOS_DIR/chama-optics"
echo ""

# Ad-hoc codesign to satisfy macOS Gatekeeper (unsigned binary gets Killed: 9)
codesign --force --sign - "$MACOS_DIR/chama-optics" 2>/dev/null || true

cd "$MACOS_DIR"
RUST_LOG=debug ./chama-optics

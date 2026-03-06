#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2020 Johann Woelper (woelper@gmail.com)
#
# SPDX-License-Identifier: MIT

# cloned and forked from https://github.com/woelper/oculante

rustup target list | grep installed
TOOLCHAIN=$(rustc --version --verbose | grep host | cut -f2 -d":" | tr -d "[:space:]")
echo we are using $TOOLCHAIN
export MACOSX_DEPLOYMENT_TARGET=10.15
cargo install cargo-bundle --quiet

# nasm is required for mozjpeg (JPEG encoding assembly optimizations)
# Note: libheif is no longer needed on macOS - native Apple ImageIO is used instead
brew install nasm --quiet

rustup target add aarch64-apple-darwin
# rustup target add x86_64-apple-darwin

# Build with ext_res for macOS (smaller binary for notarization)
cargo bundle --release --features "face_detection_insightface,ext_res"

echo otool for aarch64:
# otool -L target/aarch64-apple-darwin/release/chama-optics
echo "# Linked shared library"
otool -L target/release/chama-optics
# lipo -create -output target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics target/x86_64-apple-darwin/release/chama-optics target/aarch64-apple-darwin/release/chama-optics
file target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics
# echo otool for universal binary:
# otool -L target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics

# Copy resources to bundle
echo ""
echo "=== Copying resources to bundle ==="
mkdir -p target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Models
mkdir -p target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Fonts
mkdir -p target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Logos

# 1. Copy InsightFace ONNX model
echo "Copying models..."
MODEL_SOURCE=$(find target/release/build -name "det_10g.onnx" 2>/dev/null | head -1)
if [ -n "$MODEL_SOURCE" ] && [ -f "$MODEL_SOURCE" ]; then
    cp "$MODEL_SOURCE" target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Models/
    MODEL_SIZE=$(du -h "$MODEL_SOURCE" | cut -f1)
    echo "  ✓ det_10g.onnx ($MODEL_SIZE)"
else
    echo "  ⚠ WARNING: det_10g.onnx not found"
fi

# 2. Copy fonts
echo "Copying fonts..."
FONT_COUNT=0
if [ -d "assets/fonts" ]; then
    for font in assets/fonts/*.{ttf,otf,ttc}; do
        if [ -f "$font" ]; then
            cp "$font" target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Fonts/
            FONT_SIZE=$(du -h "$font" | cut -f1)
            FONT_NAME=$(basename "$font")
            echo "  ✓ $FONT_NAME ($FONT_SIZE)"
            FONT_COUNT=$((FONT_COUNT + 1))
        fi
    done
fi

if [ $FONT_COUNT -eq 0 ]; then
    echo "  ⚠ No fonts found in assets/fonts/"
fi

# Also copy build-time downloaded fonts (e.g. DynaPuff from build.rs)
for build_font in DynaPuff-Variable.ttf "digital-7.ttf" "digital-7 (italic).ttf"; do
    FONT_SOURCE=$(find target/release/build -name "$build_font" 2>/dev/null | head -1)
    if [ -n "$FONT_SOURCE" ] && [ -f "$FONT_SOURCE" ]; then
        cp "$FONT_SOURCE" target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Fonts/
        FONT_SIZE=$(du -h "$FONT_SOURCE" | cut -f1)
        echo "  ✓ $build_font ($FONT_SIZE) [from build cache]"
        FONT_COUNT=$((FONT_COUNT + 1))
    else
        echo "  ⚠ WARNING: $build_font not found in build artifacts"
    fi
done

# 3. Copy logos (SVG files downloaded during build)
echo "Copying logos..."
LOGO_COUNT=0
LOGO_SOURCE_DIR=$(find target/release/build -type d -name "download" 2>/dev/null | head -1)
if [ -n "$LOGO_SOURCE_DIR" ] && [ -d "$LOGO_SOURCE_DIR" ]; then
    for logo in "$LOGO_SOURCE_DIR"/*.svg; do
        if [ -f "$logo" ]; then
            cp "$logo" target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Logos/
            LOGO_COUNT=$((LOGO_COUNT + 1))
        fi
    done
    if [ $LOGO_COUNT -gt 0 ]; then
        echo "  ✓ Copied $LOGO_COUNT SVG logos"
    fi
else
    echo "  ⚠ No logos found (this is normal if logos are not used)"
fi

echo ""
echo "=== Resources in bundle ==="
echo "Models:   $(ls target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Models/*.onnx 2>/dev/null | wc -l | xargs) file(s)"
echo "Fonts:    $(ls target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Fonts/*.{ttf,otf,ttc} 2>/dev/null | wc -l | xargs) file(s)"
echo "Logos:    $(ls target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Logos/*.svg 2>/dev/null | wc -l | xargs) file(s)"
echo ""

# Note: No external dynamic libraries needed on macOS
# HEIF support is provided by native Apple ImageIO framework

echo ""
echo "###########################################################"
echo "#                    BUILD SUMMARY                        #"
echo "###########################################################"
echo ""
echo "Binary size (executable only):"
ls -lh target/release/bundle/osx/Chama\ Optics.app/Contents/MacOS/chama-optics | awk '{print "  " $5 "  " $9}'
echo ""
echo "Total app bundle size:"
du -sh target/release/bundle/osx/Chama\ Optics.app | awk '{print "  " $1}'
echo ""
echo "Resources/Models/ size:"
du -sh target/release/bundle/osx/Chama\ Optics.app/Contents/Resources/Models 2>/dev/null | awk '{print "  " $1}' || echo "  (none)"
echo ""
echo "You can test: target/release/bundle/osx/Chama Optics.app"
echo ""
echo "###########################################################"
echo ""
echo "Setting up development Resources for cargo run..."
./setup_dev_resources.sh

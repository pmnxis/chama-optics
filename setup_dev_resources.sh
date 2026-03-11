#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: MIT
#
# Setup Resources directory for development (cargo run/cargo build)
# This copies fonts and models to target/debug/Resources and target/release/Resources
# so that the app can run without being in an app bundle

set -e

echo "Setting up development Resources directories..."

# Create Resources directories
mkdir -p target/debug/Resources/Fonts
mkdir -p target/debug/Resources/Models
mkdir -p target/debug/Resources/Logos
mkdir -p target/release/Resources/Fonts
mkdir -p target/release/Resources/Models
mkdir -p target/release/Resources/Logos

# Copy fonts if they exist
if [ -d "assets/fonts" ]; then
    echo "Copying fonts..."
    for font in assets/fonts/*.{ttf,otf,ttc}; do
        if [ -f "$font" ]; then
            cp "$font" target/debug/Resources/Fonts/
            cp "$font" target/release/Resources/Fonts/
            echo "[INFO] $(basename "$font")"
        fi
    done
else
    echo "[WARN] assets/fonts directory not found"
fi

# Also copy build-time downloaded fonts (e.g. DynaPuff from build.rs)
for build_font in DynaPuff-Variable.ttf "digital-7.ttf" "digital-7 (italic).ttf"; do
    FONT_SOURCE=$(find target -name "$build_font" 2>/dev/null | head -1)
    if [ -n "$FONT_SOURCE" ] && [ -f "$FONT_SOURCE" ]; then
        cp "$FONT_SOURCE" target/debug/Resources/Fonts/
        cp "$FONT_SOURCE" target/release/Resources/Fonts/
        FONT_SIZE=$(du -h "$FONT_SOURCE" | cut -f1)
        echo "[INFO] $build_font ($FONT_SIZE) [from build cache]"
    else
        echo "[WARN] $build_font not found in build artifacts"
    fi
done
# Rename "digital-7 (italic).ttf" to match the expected runtime filename
for dir in target/debug/Resources/Fonts target/release/Resources/Fonts; do
    if [ -f "$dir/digital-7 (italic).ttf" ]; then
        mv "$dir/digital-7 (italic).ttf" "$dir/digital-7-italic.ttf"
    fi
done

# Copy model if it exists
# Try 1: Build directory (downloaded during build)
MODEL_SOURCE=$(find target -name "det_10g.onnx" 2>/dev/null | head -1)
# Try 2: Assets directory (pre-downloaded)
if [ -z "$MODEL_SOURCE" ] || [ ! -f "$MODEL_SOURCE" ]; then
    if [ -f "assets/download/det_10g.onnx" ]; then
        MODEL_SOURCE="assets/download/det_10g.onnx"
    fi
fi

if [ -n "$MODEL_SOURCE" ] && [ -f "$MODEL_SOURCE" ]; then
    echo "Copying model..."
    cp "$MODEL_SOURCE" target/debug/Resources/Models/
    cp "$MODEL_SOURCE" target/release/Resources/Models/
    MODEL_SIZE=$(du -h "$MODEL_SOURCE" | cut -f1)
    echo "[INFO] det_10g.onnx ($MODEL_SIZE)"
else
    echo "[WARN] det_10g.onnx not found in target/ or assets/download/"
fi

# Copy logos if they exist
# Try 1: Build directory (downloaded during build)
LOGO_SOURCE_DIR=$(find target -type d -name "download" 2>/dev/null | head -1)
# Try 2: Assets logo directories
if [ -z "$LOGO_SOURCE_DIR" ] || [ ! -d "$LOGO_SOURCE_DIR" ]; then
    if [ -d "assets/logo_mnf" ]; then
        LOGO_SOURCE_DIR="assets/logo_mnf"
    elif [ -d "assets/download" ]; then
        LOGO_SOURCE_DIR="assets/download"
    fi
fi

if [ -n "$LOGO_SOURCE_DIR" ] && [ -d "$LOGO_SOURCE_DIR" ]; then
    echo "Copying logos..."
    LOGO_COUNT=0
    for logo in "$LOGO_SOURCE_DIR"/*.svg; do
        if [ -f "$logo" ]; then
            cp "$logo" target/debug/Resources/Logos/
            cp "$logo" target/release/Resources/Logos/
            LOGO_COUNT=$((LOGO_COUNT + 1))
        fi
    done
    if [ $LOGO_COUNT -gt 0 ]; then
        echo "[INFO] $LOGO_COUNT SVG logos"
    else
        echo "[WARN] No SVG files found in $LOGO_SOURCE_DIR"
    fi
else
    echo "[WARN] No logos found (this is normal if logos are not used)"
fi

echo ""
echo "Development Resources setup complete!"
echo ""
echo "You can now run:"
echo "  cargo run --features \"face_detection_insightface,face_detection_visionkit,ext_res\""
echo ""
echo "Resources locations:"
echo "  - target/debug/Resources/"
echo "  - target/release/Resources/"
echo ""

#!/bin/sh
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: CC0-1.0
#
# build-freebsd.sh — Build chama-optics on the current FreeBSD environment
#
# Detects the FreeBSD version and applies the correct build configuration
# (cargo features, workarounds) for that specific release.
#
# Usage: sh build-freebsd.sh [--debug]
#
# Supported & tested versions:
#
#   Version         Tested       Features                            Notes
#   --------------- ------------ ----------------------------------- -----------------------------------
#   FreeBSD 14.3    2026-02-18   desktop,libheif,embedded_libheif    Standard
#   FreeBSD 15.0    2026-02-18   desktop,libheif                     System libheif (x264 link issue with embedded)

set -e
cd "$(dirname "$0")"

BUILD_MODE="--release"
if [ "$1" = "--debug" ]; then
    BUILD_MODE=""
fi

# ============================================================================
# Detect FreeBSD version
# ============================================================================

FREEBSD_VERSION=$(freebsd-version 2>/dev/null || uname -r)
FREEBSD_MAJOR=$(echo "$FREEBSD_VERSION" | sed 's/\..*//')

# ============================================================================
# Determine features and special handling per version
# ============================================================================

FEATURES="desktop,libheif,embedded_libheif"
SPECIAL_NOTES=""
NEEDS_LIBSTDCXX=false

case "$FREEBSD_MAJOR" in
    15*)
        # FreeBSD 15: use system libheif (embedded_libheif has x264 linking issues)
        FEATURES="desktop,libheif"
        SPECIAL_NOTES="Using system libheif (embedded_libheif disabled due to x264 link issue)"
        NEEDS_LIBSTDCXX=true
        ;;
esac

# ============================================================================
# Display build info
# ============================================================================

echo "==========================================================================="
echo "  chama-optics Build (FreeBSD)"
echo "==========================================================================="
echo ""
echo "  FreeBSD:  $FREEBSD_VERSION"
echo "  Features: $FEATURES"
echo "  Mode:     ${BUILD_MODE:---debug}"
if [ -n "$SPECIAL_NOTES" ]; then
    echo "  Notes:    $SPECIAL_NOTES"
fi
echo ""

# ============================================================================
# Source cargo env
# ============================================================================

if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

# ============================================================================
# Install dependencies via pkg
# ============================================================================

install_if_missing() {
    for _pkg in "$@"; do
        if ! pkg info -e "$_pkg" >/dev/null 2>&1; then
            echo "  Installing $_pkg..."
            pkg install -y "$_pkg"
        fi
    done
}

echo "  Checking dependencies..."

# Core build tools
install_if_missing git nasm cmake pkgconf

# Libraries needed at build/link time
install_if_missing freetype2 fontconfig libxcb libX11 libxkbcommon

# FreeBSD 15+: gcc for libstdc++, system libheif
if [ "$NEEDS_LIBSTDCXX" = true ]; then
    install_if_missing gcc libheif

    # Symlink libstdc++ to standard search path if not present
    if [ ! -f /usr/local/lib/libstdc++.so ]; then
        GCC_VER=$(pkg info -x gcc | grep -oE 'gcc[0-9]+' | head -1 | sed 's/gcc//')
        if [ -n "$GCC_VER" ] && [ -f "/usr/local/lib/gcc${GCC_VER}/libstdc++.so" ]; then
            echo "  Symlinking libstdc++ from gcc${GCC_VER}..."
            ln -sf "/usr/local/lib/gcc${GCC_VER}/libstdc++.so" /usr/local/lib/libstdc++.so
            ln -sf "/usr/local/lib/gcc${GCC_VER}/libstdc++.so.6" /usr/local/lib/libstdc++.so.6
        fi
    fi
fi

echo ""

# ============================================================================
# Pre-build checks
# ============================================================================

# Check Rust toolchain, install if missing
if ! command -v cargo >/dev/null 2>&1; then
    echo "  Rust toolchain not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    . "$HOME/.cargo/env"

    if ! command -v cargo >/dev/null 2>&1; then
        echo "ERROR: cargo is still not available after rustup install."
        echo "       Please install Rust manually: https://rustup.rs/"
        exit 1
    fi
fi

FREETYPE_VER=$(pkg-config --modversion freetype2 2>/dev/null || echo "not found")
echo "  freetype2: $FREETYPE_VER"
echo "  rustc:     $(rustc --version 2>/dev/null || echo 'not found')"
echo ""

# ============================================================================
# Build
# ============================================================================

echo "==========================================================================="
echo "  Starting build: cargo build $BUILD_MODE --features \"$FEATURES\""
echo "==========================================================================="
echo ""

cargo build $BUILD_MODE --features "$FEATURES"

echo ""
echo "==========================================================================="
echo "  Build complete!"
echo "==========================================================================="

if [ -n "$BUILD_MODE" ]; then
    BINARY="target/release/chama-optics"
else
    BINARY="target/debug/chama-optics"
fi

if [ -f "$BINARY" ]; then
    echo "  Binary: $BINARY ($(du -h "$BINARY" | cut -f1))"
fi

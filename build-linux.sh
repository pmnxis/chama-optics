#!/bin/bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: CC0-1.0
#
# build-linux.sh — Build chama-optics on the current Linux environment
#
# Detects the current Linux distribution and applies the correct build
# configuration (cargo features, workarounds) for that specific distro.
# For building across ALL distros via Proxmox, use: bash package-all-linux-distro.sh
#
# Usage: bash build-linux.sh [--release]
#
# Supported & tested distros:
#
#   Distro          Tested       Features                            Notes
#   --------------- ------------ ----------------------------------- -----------------------------------
#   Debian 12       2026-02-18   desktop,libheif,embedded_libheif    Standard
#   Debian 13       2026-02-18   desktop,libheif,embedded_libheif    Standard
#   Ubuntu 22.04    2026-02-18   desktop,libheif,embedded_libheif    libdav1d-dev MUST be removed
#   Ubuntu 24.04    2026-02-18   desktop,libheif,embedded_libheif    Standard
#   Fedora 41       2026-02-18   desktop,libheif,embedded_libheif    Standard
#   Rocky 9         2026-02-18   desktop,libheif,embedded_libheif    freetype 2.13.3 from source required
#   Arch Linux      2026-02-18   desktop,libheif                     System libheif (no embedded_libheif)

set -e
cd "$(dirname "$0")/.."

BUILD_MODE="--release"
if [ "$1" = "--debug" ]; then
    BUILD_MODE=""
fi

# ============================================================================
# Detect distro
# ============================================================================

detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo "$ID"
    elif [ -f /etc/arch-release ]; then
        echo "arch"
    else
        echo "unknown"
    fi
}

detect_version() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo "${VERSION_ID:-rolling}"
    else
        echo "unknown"
    fi
}

detect_pretty_name() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo "${PRETTY_NAME:-$ID}"
    else
        echo "Unknown Linux"
    fi
}

DISTRO=$(detect_distro)
DISTRO_VERSION=$(detect_version)
DISTRO_PRETTY=$(detect_pretty_name)

# ============================================================================
# Determine features and special handling per distro
# ============================================================================

FEATURES="desktop,libheif,embedded_libheif"
SPECIAL_NOTES=""
NEEDS_CONFIRM=false

case "$DISTRO" in
    arch|manjaro)
        # Arch: use system libheif (embedded_libheif has x264 linking issues)
        FEATURES="desktop,libheif"
        SPECIAL_NOTES="Using system libheif (embedded_libheif disabled due to x264 link issue)"
        ;;
    ubuntu)
        case "$DISTRO_VERSION" in
            22.04*)
                SPECIAL_NOTES="Ubuntu 22.04 requires libdav1d-dev to be REMOVED"
                NEEDS_CONFIRM=true
                ;;
        esac
        ;;
    rocky|rhel|almalinux|centos)
        case "$DISTRO_VERSION" in
            9*)
                SPECIAL_NOTES="freetype 2.13.3 must be built from source (setup-linux.sh handles this)"
                ;;
        esac
        ;;
esac

# ============================================================================
# Display build info
# ============================================================================

echo "==========================================================================="
echo "  chama-optics Build"
echo "==========================================================================="
echo ""
echo "  Distro:   $DISTRO_PRETTY"
echo "  ID:       $DISTRO $DISTRO_VERSION"
echo "  Features: $FEATURES"
echo "  Mode:     ${BUILD_MODE:---debug}"
if [ -n "$SPECIAL_NOTES" ]; then
    echo "  Notes:    $SPECIAL_NOTES"
fi
echo ""

# ============================================================================
# Source cargo env and freetype PKG_CONFIG_PATH
# ============================================================================

if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

if [ -f /etc/profile.d/freetype-local.sh ]; then
    source /etc/profile.d/freetype-local.sh
fi

# ============================================================================
# Pre-build checks
# ============================================================================

# Check Rust toolchain, install if missing
if ! command -v cargo &>/dev/null; then
    echo "  Rust toolchain not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"

    if ! command -v cargo &>/dev/null; then
        echo "ERROR: cargo is still not available after rustup install."
        echo "       Please install Rust manually: https://rustup.rs/"
        exit 1
    fi
fi

# Check freetype version for distros that need source build
FREETYPE_VER=$(pkg-config --modversion freetype2 2>/dev/null || echo "not found")
echo "  freetype2: $FREETYPE_VER"
echo "  rustc:     $(rustc --version 2>/dev/null || echo 'not found')"
echo ""

# ============================================================================
# Ubuntu 22.04: Strong warning about libdav1d-dev
# ============================================================================

if [ "$NEEDS_CONFIRM" = true ]; then
    echo "==========================================================================="
    echo "  !! WARNING !! — Ubuntu 22.04 Specific Issue"
    echo "==========================================================================="
    echo ""
    echo "  Ubuntu 22.04's system libdav1d-dev provides an OLD dav1d API (0.9.x)"
    echo "  that is INCOMPATIBLE with embedded libheif."
    echo ""
    echo "  Problem: struct field 'Dav1dSettings.n_threads' does not exist in"
    echo "  Ubuntu 22.04's dav1d headers (it uses 'n_tile_threads' instead)."
    echo ""
    echo "  Resolution: 'libdav1d-dev' will be REMOVED before building."
    echo "  The resulting binary will NOT have dav1d (AV1) decode support."
    echo ""
    echo "  The following command will be executed:"
    echo "    sudo apt-get remove -y libdav1d-dev"
    echo ""
    echo "==========================================================================="
    echo ""
    read -rp "Accept libdav1d-dev removal and proceed? [y/N] " confirm < /dev/tty
    if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
        echo "Aborted by user."
        exit 1
    fi

    echo ""
    echo "Removing libdav1d-dev..."
    sudo apt-get remove -y libdav1d-dev 2>/dev/null || true
    echo "libdav1d-dev removed."
    echo ""
fi

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

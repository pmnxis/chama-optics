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
#   Distro          Tested       glibc  ort     Features                                                    Notes
#   --------------- ------------ ------ ------- ----------------------------------------------------------- -----------------------------------
#   Debian 12       2026-02-18   2.36   rc.10   desktop,libheif,embedded_libheif,face_detection_insightface  Standard
#   Debian 13       2026-02-18   2.41   rc.11   desktop,libheif,embedded_libheif,face_detection_insightface  Standard
#   Ubuntu 22.04    2026-02-18   2.35   rc.10   desktop,libheif,embedded_libheif,face_detection_insightface  libdav1d-dev MUST be removed
#   Ubuntu 24.04    2026-02-18   2.39   rc.11   desktop,libheif,embedded_libheif,face_detection_insightface  Standard
#   Fedora 41       2026-02-18   2.40   rc.11   desktop,libheif,embedded_libheif,face_detection_insightface  Standard
#   Rocky 9         2026-02-18   2.34   rc.10   desktop,libheif,embedded_libheif,face_detection_insightface  freetype 2.13.3 from source required
#   Arch Linux      2026-02-18   2.43   rc.11   desktop,libheif,face_detection_insightface                   System libheif (no embedded_libheif)

set -e
cd "$(dirname "$0")"

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

detect_glibc_version() {
    # Returns glibc version as "MAJOR.MINOR" (e.g. "2.38")
    local ver
    ver=$(ldd --version 2>&1 | head -1 | grep -oE '[0-9]+\.[0-9]+$') || true
    echo "${ver:-0.0}"
}

# Compare two version strings: returns 0 if $1 >= $2
version_ge() {
    [ "$(printf '%s\n%s' "$1" "$2" | sort -V | head -1)" = "$2" ]
}

DISTRO=$(detect_distro)
DISTRO_VERSION=$(detect_version)
DISTRO_PRETTY=$(detect_pretty_name)
GLIBC_VERSION=$(detect_glibc_version)

# ============================================================================
# Determine features and special handling per distro
# ============================================================================

FEATURES="desktop,libheif,embedded_libheif,face_detection_insightface"
SPECIAL_NOTES=""
NEEDS_CONFIRM=false

case "$DISTRO" in
    arch|manjaro)
        # Arch: use system libheif (embedded_libheif has x264 linking issues)
        FEATURES="desktop,libheif,face_detection_insightface"
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
# Determine ort (ONNX Runtime) version based on glibc
# ============================================================================
#
# ort rc.11 bundles ONNX Runtime 1.21, built on Ubuntu 24.04 (glibc 2.39).
# Its prebuilt binaries reference __isoc23_strtol@GLIBC_2.38, which is
# absent on systems with glibc < 2.38.
#
# ort rc.10 bundles ONNX Runtime 1.20, compatible with glibc 2.34+.
#
# Cargo.toml defaults to rc.11 (for Windows/macOS).
# On Linux systems with glibc < 2.38, we override via .cargo/config.toml [patch].
#
# See: https://github.com/pykeio/ort/issues/523

ORT_PATCHED=false
ORT_VERSION="rc.11"

if ! version_ge "$GLIBC_VERSION" "2.38"; then
    ORT_VERSION="rc.10"
    mkdir -p .cargo
    cat > .cargo/config.toml <<'CARGO_CONFIG'
[patch.crates-io]
ort = { git = "https://github.com/pykeio/ort", tag = "v2.0.0-rc.10" }
CARGO_CONFIG
    ORT_PATCHED=true
fi

# ============================================================================
# Display build info
# ============================================================================

echo "==========================================================================="
echo "  chama-optics Build"
echo "==========================================================================="
echo ""
echo "  Distro:   $DISTRO_PRETTY"
echo "  ID:       $DISTRO $DISTRO_VERSION"
echo "  glibc:    $GLIBC_VERSION"
echo "  Features: $FEATURES"
echo "  ort:      v2.0.0-$ORT_VERSION (ONNX Runtime)"
echo "  Mode:     ${BUILD_MODE:---debug}"
if [ "$ORT_PATCHED" = true ]; then
    echo "  Override: .cargo/config.toml patches ort rc.11 -> rc.10 (glibc $GLIBC_VERSION < 2.38)"
fi
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
    if [ -t 0 ] && [ -e /dev/tty ]; then
        read -rp "Accept libdav1d-dev removal and proceed? [y/N] " confirm < /dev/tty
        if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
            echo "Aborted by user."
            exit 1
        fi
    else
        echo "  (non-interactive: auto-accepting)"
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

if [ "$ORT_PATCHED" = true ]; then
    cargo update -p ort
fi

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

#!/bin/bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: CC0-1.0
#
# quick-install-linux.sh — One-liner installer for chama-optics on Linux
#
# Usage:
#   curl -sSf https://raw.githubusercontent.com/pmnxis/chama-optics/master/quick-install-linux.sh | bash
#   curl -sSf ... | bash -s -- stable   (non-interactive)
#   curl -sSf ... | bash -s -- latest   (non-interactive)

set -e

REPO="https://github.com/pmnxis/chama-optics.git"
INSTALL_DIR="$HOME/chama-optics"

# Accept channel as argument (non-interactive) or prompt interactively
if [ -n "$1" ]; then
    choice="$1"
else
    echo "==========================================================================="
    echo "  chama-optics Quick Installer for Linux"
    echo "==========================================================================="
    echo ""
    echo "  1) stable  — Latest released version (recommended)"
    echo "  2) latest  — Master branch (may be unstable)"
    echo ""
    # Read from /dev/tty so it works with curl | bash (stdin is the script)
    read -rp "Select channel [1/2]: " choice < /dev/tty
fi

case "$choice" in
    1|stable)
        CHANNEL="stable"
        ;;
    2|latest)
        CHANNEL="latest"
        ;;
    *)
        echo "Invalid choice '$choice'. Use: stable or latest"
        exit 1
        ;;
esac

# Resolve git ref
#
# Tag convention: vX.Y.Z[-suffix]
#   v0.1.9, v0.2.0-rc, v0.1.8-alpha, v0.1.1-hotfix2, etc.
#   All v* tags are considered valid releases.
#
if [ "$CHANNEL" = "stable" ]; then
    echo ""
    echo "Fetching latest release tag..."
    GIT_REF=$(git ls-remote --tags --sort=-v:refname "$REPO" 'v*' 2>/dev/null \
        | sed 's|.*refs/tags/||; s|\^{}||' \
        | grep -E '^v[0-9]+\.' \
        | sort -u \
        | sort -V -r \
        | head -1)
    if [ -z "$GIT_REF" ]; then
        echo "No release tags found, falling back to master."
        GIT_REF="master"
    fi
else
    GIT_REF="master"
fi

echo ""
echo "  Channel:  $CHANNEL"
echo "  Ref:      $GIT_REF"
echo "  Install:  $INSTALL_DIR"
echo ""

# Clone or update
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing clone..."
    cd "$INSTALL_DIR"
    if ! git fetch origin --tags; then
        echo "ERROR: git fetch failed. Check your network connection."
        exit 1
    fi
    if ! git checkout "$GIT_REF"; then
        echo "ERROR: Failed to checkout '$GIT_REF'."
        exit 1
    fi
    git pull origin "$GIT_REF" 2>/dev/null || true
else
    echo "Cloning repository..."
    if ! git clone "$REPO" "$INSTALL_DIR"; then
        echo "ERROR: git clone failed. Check your network connection or install git."
        exit 1
    fi
    cd "$INSTALL_DIR"
    if ! git checkout "$GIT_REF"; then
        echo "ERROR: Failed to checkout '$GIT_REF'."
        exit 1
    fi
fi

# Ensure build-linux.sh exists (older tags may not have it)
if [ ! -f "build-linux.sh" ]; then
    echo "  build-linux.sh not found in $GIT_REF, fetching from master..."
    curl -sSf "https://raw.githubusercontent.com/pmnxis/chama-optics/master/build-linux.sh" -o build-linux.sh
    chmod +x build-linux.sh
fi

# Run build
echo ""
exec bash build-linux.sh

#!/bin/bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: MIT

set -eu

# Build script for web (WASM) target using trunk

echo "Building Chama Optics for Web (WASM)..."

# Check if trunk is installed
if ! command -v trunk &> /dev/null; then
    echo "Error: trunk is not installed"
    echo "Install it with: cargo install trunk"
    exit 1
fi

# Check if wasm32-unknown-unknown target is installed
if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Build for release
echo "Building with trunk..."
trunk build --release

echo ""
echo "Build complete!"
echo "Output is in: dist/"
echo ""
echo "To serve locally, run:"
echo "  trunk serve --open"
echo ""
echo "To deploy, upload the contents of dist/ to your web server"

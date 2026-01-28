#!/bin/bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: MIT

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building Chama Optics for iOS...${NC}"

# Use cargo from standard location
if [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo not found${NC}"
    echo "Cargo (Rust) is required but not found in PATH"
    echo "Looking for cargo in: $HOME/.cargo/bin/cargo"
    exit 1
fi

echo "Using cargo from: $(which cargo)"

# Build directory
BUILD_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$BUILD_DIR/rust-core"

# Set minimum iOS deployment target for CoreML, PhotosPicker, and modern Metal API
# iOS 16.0+ required for PhotosPicker and optimal performance
export IPHONEOS_DEPLOYMENT_TARGET=16.0
rustc_flags="-C link-arg=-mios-version-min=16.0"

# Clean previous builds
echo -e "${YELLOW}Cleaning previous builds...${NC}"

# Build for iOS simulator (aarch64 - for Apple Silicon Macs)
echo -e "${GREEN}Building for iOS simulator (aarch64-apple-ios-sim)...${NC}"
cd "$PROJECT_ROOT"
RUSTFLAGS="$rustc_flags" cargo build --lib --target aarch64-apple-ios-sim --no-default-features --features ios_integration

# Build for iOS device (aarch64)
echo -e "${GREEN}Building for iOS device (aarch64-apple-ios)...${NC}"
RUSTFLAGS="$rustc_flags" cargo build --lib --target aarch64-apple-ios --no-default-features --features ios_integration --release

# Build for iOS simulator (x86_64 - for Intel Macs)
# Disabled for build speed - comment out if needed
# echo -e "${GREEN}Building for iOS simulator (x86_64-apple-ios)...${NC}"
# RUSTFLAGS="$rustc_flags" cargo build --lib --target x86_64-apple-ios --no-default-features --features ios_integration

echo -e "${GREEN}✓ Build completed successfully!${NC}"
echo ""
echo "Libraries are located at:"
echo "  - Device: aarch64-apple-ios/release/libchama_optics.a"
echo "  - Simulator (aarch64): aarch64-apple-ios-sim/debug/libchama_optics.a"
echo ""

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

# Check if rustup is installed
if ! command -v rustup &> /dev/null; then
    echo -e "${RED}Error: rustup is not installed${NC}"
    echo "Please install rustup from https://rustup.rs/"
    exit 1
fi

# Check if required targets are installed
TARGETS=("aarch64-apple-ios" "aarch64-apple-ios-sim" "x86_64-apple-ios")
for TARGET in "${TARGETS[@]}"; do
    if ! rustup target list | grep -q "$TARGET (installed)"; then
        echo -e "${YELLOW}Installing target: $TARGET${NC}"
        rustup target add $TARGET
    fi
done

# Build directory
BUILD_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$BUILD_DIR/.."
LIB_DIR="$BUILD_DIR/ChamaOptics/ChamaOptics/libs"

# Clean previous builds
echo -e "${YELLOW}Cleaning previous builds...${NC}"
rm -rf "$LIB_DIR"
mkdir -p "$LIB_DIR"

# Build for iOS device (aarch64)
echo -e "${GREEN}Building for iOS device (aarch64-apple-ios)...${NC}"
cd "$PROJECT_ROOT"
cargo build --release --lib --target aarch64-apple-ios --no-default-features --features ios_integration

# Build for iOS simulator (aarch64 - for Apple Silicon Macs)
echo -e "${GREEN}Building for iOS simulator (aarch64-apple-ios-sim)...${NC}"
cargo build --release --lib --target aarch64-apple-ios-sim --no-default-features --features ios_integration

# Build for iOS simulator (x86_64 - for Intel Macs)
echo -e "${GREEN}Building for iOS simulator (x86_64-apple-ios)...${NC}"
cargo build --release --lib --target x86_64-apple-ios --no-default-features --features ios_integration

# Copy libraries to iOS project
echo -e "${GREEN}Copying libraries to iOS project...${NC}"
mkdir -p "$LIB_DIR/aarch64-apple-ios"
mkdir -p "$LIB_DIR/aarch64-apple-ios-sim"
mkdir -p "$LIB_DIR/x86_64-apple-ios"

cp "$PROJECT_ROOT/target/aarch64-apple-ios/release/libchama_optics.a" "$LIB_DIR/aarch64-apple-ios/"
cp "$PROJECT_ROOT/target/aarch64-apple-ios-sim/release/libchama_optics.a" "$LIB_DIR/aarch64-apple-ios-sim/"
cp "$PROJECT_ROOT/target/x86_64-apple-ios/release/libchama_optics.a" "$LIB_DIR/x86_64-apple-ios/"

# Create universal simulator library (combines aarch64 and x86_64 for simulator)
echo -e "${GREEN}Creating universal simulator library...${NC}"
mkdir -p "$LIB_DIR/universal-sim"
lipo -create \
    "$LIB_DIR/aarch64-apple-ios-sim/libchama_optics.a" \
    "$LIB_DIR/x86_64-apple-ios/libchama_optics.a" \
    -output "$LIB_DIR/universal-sim/libchama_optics.a"

echo -e "${GREEN}✓ Build completed successfully!${NC}"
echo ""
echo "Libraries are located at:"
echo "  - Device: $LIB_DIR/aarch64-apple-ios/libchama_optics.a"
echo "  - Simulator (universal): $LIB_DIR/universal-sim/libchama_optics.a"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Open Xcode: open ios/ChamaOptics/ChamaOptics.xcodeproj"
echo "2. Build and run the project"

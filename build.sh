#!/bin/bash
set -e
cd "$(dirname "$0")"

echo "🔨 Building Rust (debug)..."
MACOSX_DEPLOYMENT_TARGET=13.0 cargo build --lib --target aarch64-apple-darwin

echo "🧹 Cleaning Xcode..."
rm -rf ~/Library/Developer/Xcode/DerivedData/ChamaOptics-*
cd swift/ChamaOptics
xcodebuild clean -project ChamaOptics.xcodeproj -scheme ChamaOptics >/dev/null 2>&1

echo "📦 Building Swift..."
xcodebuild -project ChamaOptics.xcodeproj -scheme ChamaOptics -configuration Debug build

echo "🚀 Launching..."
APP=$(find ~/Library/Developer/Xcode/DerivedData/ChamaOptics-*/Build/Products/Debug -name "ChamaOptics.app" -type d 2>/dev/null | head -1)
open "$APP"
echo "✅ Done!"

<!--
SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
SPDX-License-Identifier: MIT
-->

# iOS Development Setup

## Prerequisites

### 1. Xcode Installation

The iOS build requires **full Xcode**, not just Command Line Tools.

#### Check Current Setup
```bash
xcode-select -p
```

If this shows `/Library/Developer/CommandLineTools`, you need to install full Xcode.

#### Install Full Xcode

1. **Download Xcode** from App Store or [Apple Developer](https://developer.apple.com/xcode/)

2. **Set Xcode as active developer directory**:
   ```bash
   sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
   ```

3. **Accept license agreement**:
   ```bash
   sudo xcodebuild -license accept
   ```

4. **Verify iOS SDK is available**:
   ```bash
   xcrun --show-sdk-path --sdk iphoneos
   # Should output: /Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk
   ```

### 2. Rust iOS Targets

```bash
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
rustup target add x86_64-apple-ios
```

## Building for iOS

### Option 1: Using Build Script (Recommended)

```bash
cd ios
./build_ios.sh
```

This will:
- Build for all iOS architectures
- Create universal simulator library
- Copy libraries to iOS project

### Option 2: Manual Build

**For Device (aarch64):**
```bash
cargo build --release --lib --target aarch64-apple-ios --features ios_integration
```

**For Simulator (Apple Silicon):**
```bash
cargo build --release --lib --target aarch64-apple-ios-sim --features ios_integration
```

**For Simulator (Intel):**
```bash
cargo build --release --lib --target x86_64-apple-ios --features ios_integration
```

## Testing Without iOS Device

While waiting for full Xcode installation, you can test mobile UI features on desktop:

```bash
# Run with mobile_ui feature enabled
cargo run --features mobile_ui
```

This will apply mobile UI optimizations (larger touch targets, etc.) on desktop.

## Troubleshooting

### Error: SDK "iphoneos" cannot be located

**Cause**: Command Line Tools don't include iOS SDK.

**Solution**: Install full Xcode and set it as active developer directory (see above).

### Error: No such file or directory (os error 2) when running simulator

**Cause**: Simulator-specific library not built.

**Solution**:
```bash
cargo build --lib --target aarch64-apple-ios-sim --features ios_integration
```

### Error: Library not found

**Cause**: Libraries not in Xcode project search path.

**Solution**: Check that `ios/ChamaOptics/libs/` contains the built libraries.

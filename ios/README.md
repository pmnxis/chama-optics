<!--
SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
SPDX-License-Identifier: MIT
-->

# Chama Optics iOS

iOS integration for Chama Optics using SwiftUI + Rust/egui.

## Architecture

```
┌─────────────────────────────────────┐
│         SwiftUI iOS App             │
│  - File Picker (Photos Library)     │
│  - Share Extension                  │
│  - Native iOS Navigation            │
└──────────────┬──────────────────────┘
               │ Swift-Rust FFI Bridge
┌──────────────▼──────────────────────┐
│      Rust/egui Core Library         │
│  - Image Processing                 │
│  - Theme Application                │
│  - EXIF Handling                    │
│  - egui UI (embedded in Metal view) │
└─────────────────────────────────────┘
```

## Building for iOS

### Prerequisites

1. Xcode 15+ installed
2. Rust iOS targets:
   ```bash
   rustup target add aarch64-apple-ios
   rustup target add aarch64-apple-ios-sim
   rustup target add x86_64-apple-ios
   ```

### Build Steps

1. **Build Rust library for iOS**:
   ```bash
   ./ios/build_ios.sh
   ```

2. **Open Xcode project**:
   ```bash
   open ios/ChamaOptics/ChamaOptics.xcodeproj
   ```

3. **Build and run in Xcode**

## Project Structure

```
ios/
├── README.md                    # This file
├── build_ios.sh                 # Script to build Rust library for iOS
├── ChamaOptics/                 # Xcode project
│   ├── ChamaOptics.xcodeproj
│   ├── ChamaOptics/             # Swift source files
│   │   ├── ChamaOpticsApp.swift
│   │   ├── ContentView.swift
│   │   ├── RustBridge.swift     # Swift-Rust FFI
│   │   ├── EguiView.swift       # egui Metal view wrapper
│   │   └── Info.plist
│   └── libs/                    # Compiled Rust libraries
│       ├── aarch64-apple-ios/
│       └── aarch64-apple-ios-sim/
└── framework/                   # Optional: XCFramework bundle
```

## Features

- ✅ Native iOS file picker with Photos library access
- ✅ Share extension support
- ✅ Touch-optimized UI (larger buttons, gestures)
- ✅ Portrait and landscape support
- ✅ Dark mode support
- ✅ Background image processing
- ✅ Memory-efficient preview generation

## References

- [Fast & Fluid: Integrating Rust egui into SwiftUI](https://medium.com/@djalex566/fast-fluid-integrating-rust-egui-into-swiftui-30a218c502c1)
- [egui iOS examples](https://github.com/emilk/egui/tree/master/examples/ios)

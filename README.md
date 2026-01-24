<!--
SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Chama Optics

[![dependency status](https://deps.rs/repo/github/pmnxis/chama-optics/status.svg)](https://deps.rs/repo/github/pmnxis/chama-optics)
[![Build Status](https://github.com/pmnxis/chama-optics/workflows/CI/badge.svg)](https://github.com/pmnxis/chama-optics/actions?workflow=CI)

<p align="center"><img src="assets/mac-icon.png" width="256" height="256"/></p>

Chama Optics is a program created for recent mirrorless and DSLR cameras, <br/>
inspired by the travel VTuber Akai Haato([赤井はあと](https://www.youtube.com/@AkaiHaato)), who loves photography.

It analyzes the EXIF data embedded in photos along with the user’s settings to resize, compress, and tag images before saving them, while also offering additional convenient features.

This program is developed in [Rust](https://rust-lang.org/) using the [eframe](https://github.com/emilk/egui/tree/master/crates/eframe)/[egui](https://github.com/emilk/egui/) framework, along with libraries such as libheif and exif-rs.

## Current Status
- [x] Read JPEG/PNG and other common formats
- [x] Read HEIF photos (libheif)
- [x] Read EXIF data (supports up to 2.3.x standard; not yet compliant with 3.0)
- [x] Save photos with selected frames and settings
- [x] Themes from genally use in another case
- [x] Read Panasonic lumix LUT/PhotoStyle and Nikon Picture Control names
- [ ] More themes
- [ ] Save photos with EXIF
- [x] Face Detection with CPU
- [x] Face Detection with neural engine in macOS.
- [x] Face Detection utilize NPU or GPU in Windows / Linux (Optional)
- [x] Multi core usage
- [x] Utilize camera maker logo
- [x] Watermark feature
- [x] When loading HEIF / JPEG images, generate thumbnails by prioritizing the Thumbnail / Preview metadata inside EXIF instead of resizing pixels from the full image (improves performance)
- [ ] Feature to create 4-cut or 2-cut photos with idol images, similar to photo sticker booths
- [x] Function to group similar photos or images taken around the same time
- [ ] Preset and adjustment controls for contrast, brightness, grain, texture, and LUT
- [ ] Web application supports (libheif wasm)


## Building and Running

### Quick Start
```bash
cargo run --release --features face_detection_insightface
```

### Web (WASM)
Build and run the web version:
```sh
# Install trunk (one-time setup)
cargo install trunk

# Add wasm target (one-time setup)
rustup target add wasm32-unknown-unknown

# Build for web (creates dist/ folder)
./build_web.sh

# Or serve locally with hot-reload
trunk serve --open
```

**Note:** Web version has limitations:
- No native file dialogs (uses HTML file input)
- No libheif support (HEIF images not supported in browser)
- File operations are limited to browser capabilities

### Windows

#### Prerequisites
1. Install [Rust](https://rustup.rs/)
2. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with "C++ build tools"
3. (Optional) Install vcpkg for HEIF support

#### Using vcpkg for HEIF support (Optional)
For HEIF/HEIC image support, you need libheif. The easiest way on Windows is using vcpkg:

```bat
git clone https://github.com/microsoft/vcpkg.git
cd vcpkg
.\bootstrap-vcpkg.bat
vcpkg integrate install
vcpkg install libheif:x64-windows-static-md
vcpkg install libheif:x64-windows-static
cd ..
```

#### Building
Use the provided build script:

```bat
.\build_windows.bat
```

Or build manually:

```bat
# Build release version
cargo build --release --features desktop

# Run directly
cargo run --release --features desktop

# Create bundled executable
cargo install cargo-bundle
cargo bundle --release
```

#### Output
- Debug build: `target\debug\chama-optics.exe`
- Release build: `target\release\chama-optics.exe`
- Bundled app: `target\release\bundle\windows\Chama Optics.exe`

#### Features on Windows
- ✅ Full GUI support
- ✅ File dialogs
- ✅ System fonts
- ✅ HEIF/HEIC support (with vcpkg)
- ⚠️ Face detection: CPU only (No GPU/NPU support yet)

### Face Detection Options

**For Face Detection, Use macOS/iOS:**
- ✅ **VisionKit** (macOS/iOS): Works perfectly, fast, accurate
- Built-in Apple framework, no external dependencies
- Optimized for Apple Silicon

### macOS
```sh
brew install pkgconf libheif
cargo run --release
# When make *.app
./build_mac.sh
cd ./target/release/bundle/osx
open -n "Chama Optics.app"
```

### Linux

#### Prerequisites
1. Install [Rust](https://rustup.rs/)
2. Install system dependencies

#### System Dependencies

**Ubuntu/Debian:**
```bash
# GUI dependencies
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev pkg-config fontconfig

# Optional: HEIF support
sudo apt-get install libheif-dev libde265-dev x265

# Optional: Build libheif from source (if needed)
source ./build_deps_debian.sh
```

**Fedora/RHEL:**
```bash
sudo dnf install clang clang-devel clang-tools-extra libxkbcommon-devel \
    pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel

# Optional: HEIF support
sudo dnf install libheif-devel libde265-devel x265-devel
```

**Arch Linux:**
```bash
sudo pacman -S base-devel pkg-config fontconfig libheif
```

#### Building
Use the provided build script:

```bash
chmod +x build_linux.sh
./build_linux.sh
```

Or build manually:

```bash
# Build release version
cargo build --release --features desktop

# Run directly
cargo run --release --features desktop

# Create bundled executable
cargo install cargo-bundle
cargo bundle --release
```

#### Output
- Debug build: `target/debug/chama-optics`
- Release build: `target/release/chama-optics`
- Bundled app: `target/release/bundle/linux/chama-optics`

#### Features on Linux
- ✅ Full GUI support
- ✅ File dialogs
- ✅ System fonts
- ✅ HEIF/HEIC support (with libheif)
- ✅ Wayland and X11 support
- ⚠️ Face detection: CPU only (No GPU/NPU support yet)

#### Known Limitations on Linux

**Face Detection with InsightFace Feature:**
The `--features face_detection_insightface` flag may have compatibility issues on Linux depending on your system configuration and ONNX Runtime version.

**Recommendation:**
- Default build (without `face_detection_insightface`): ✅ Recommended
- With `face_detection_insightface` feature: ⚠️ Experimental, may require additional system dependencies

### Web (WASM)

Build for web browsers using WebAssembly.

#### Prerequisites
```sh
# Install trunk (web build tool for Rust)
cargo install trunk

# Add wasm32 target
rustup target add wasm32-unknown-unknown
```

#### Development Server
```sh
# Start local development server (opens browser automatically)
./serve_web.sh

# Or manually:
trunk serve --open
```

This will start a server at `http://127.0.0.1:8080` with hot-reload.

#### Production Build
```sh
# Build optimized version to dist/ folder
./build_web.sh

# Or manually:
trunk build --release
```

#### Requirements
- **WebGL support required**: Your browser must support WebGL (WebGL 1.0 or WebGL 2.0)
- Modern browser: Chrome, Firefox, Edge, Safari (latest versions)
- Hardware acceleration enabled in browser settings

To check WebGL support, visit: https://get.webgl.org/

#### Limitations
- **Requires WebGL**: Will not work without WebGL support
- No file system access (browser security restrictions)
- File dialogs disabled (use drag & drop instead)
- System fonts not available (builtin fonts only)
- Update checker disabled
- No HEIC support
- Uses fallback image encoders (PNG/JPEG from `image` crate)

#### Troubleshooting

If you see "WebGL Not Supported":
1. Update your browser to the latest version
2. Enable hardware acceleration:
   - Chrome: `chrome://settings` → Advanced → System → "Use hardware acceleration"
   - Firefox: `about:preferences` → General → Performance → "Use hardware acceleration"
3. Update graphics drivers
4. Try a different browser

### License
Most of the code depends on the NON-AI-MIT license, while some portions are under the MIT or Apache 2.0 licenses.

In particular, the image data has been processed by pmnxis, but the original vector icons were used with permission from シエミカ (X: shiemika324).

All icons are strictly prohibited from being used for any form of AI training without exception.

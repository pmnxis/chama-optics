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

This program is developed in [Rust](https://rust-lang.org/) using the [eframe](https://github.com/emilk/egui/tree/master/crates/eframe)/[egui](https://github.com/emilk/egui/) framework, along with libraries such as exif-rs.

## Current Status
- [x] Read JPEG/PNG and other common formats
- [x] Read HEIF photos (native Apple ImageIO on macOS/iOS, libheif on Windows/Linux)
- [x] Read EXIF data (supports up to 2.3.x standard; not yet compliant with 3.0)
- [x] Save photos with selected frames and settings
- [x] Themes from genally use in another case
- [x] Read Panasonic lumix LUT/PhotoStyle and Nikon Picture Control names
- [ ] More themes
- [x] Save photos with EXIF
- [x] Face Detection with CPU
- [x] Face Detection with neural engine in macOS.
- [x] Face Detection utilize NPU or GPU in Windows / Linux (Optional)
- [x] Multi core usage
- [x] Utilize camera maker logo
- [x] Watermark feature
- [x] When loading HEIF / JPEG images, generate thumbnails by prioritizing the Thumbnail / Preview metadata inside EXIF instead of resizing pixels from the full image (improves performance)
- [ ] Feature to create 4-cut or 2-cut photos with idol images, similar to photo sticker booths
- [x] Function to group similar photos or images taken around the same time
- [x] Adjustment controls for contrast, brightness, grain, texture, and LUT
- [ ] Preset for color grading and complex procedure by EXIF or user request


## Building and Running

### Quick Start
```bash
cargo run --release --features face_detection_insightface
```

### Windows

#### Prerequisites
1. Install [Rust](https://rustup.rs/)
2. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with "C++ build tools"
3. (Optional) Install vcpkg for HEIF support

#### Using vcpkg for HEIF support (Required for HEIC/HEIF images)
For HEIF/HEIC image support on Windows, you need libheif via vcpkg:

```bat
git clone https://github.com/microsoft/vcpkg.git
cd vcpkg
.\bootstrap-vcpkg.bat
vcpkg integrate install
vcpkg install libheif:x64-windows-static-md
vcpkg install libheif:x64-windows-static
cd ..
```

> **Note:** Build with `--features libheif` to enable HEIF support on Windows.

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
- ✅ Face detection: Due to driver condition.

### Face Detection Options

**For Face Detection, Use macOS/iOS:**
- ✅ **VisionKit** (macOS/iOS): Works perfectly, fast, accurate
- Built-in Apple framework, no external dependencies
- Optimized for Apple Silicon

### macOS
```sh
# Note: libheif is NOT needed on macOS - native Apple ImageIO is used for HEIF support
brew install nasm  # Required for mozjpeg JPEG encoding
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

# HEIF support (required for HEIC/HEIF images on Linux)
sudo apt-get install libheif-dev libde265-dev x265

# Optional: Build libheif from source (if needed)
source ./build_deps_debian.sh
```

**Fedora/RHEL:**
```bash
sudo dnf install clang clang-devel clang-tools-extra libxkbcommon-devel \
    pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel

# HEIF support (required for HEIC/HEIF images on Linux)
sudo dnf install libheif-devel libde265-devel x265-devel
```

**Arch Linux:**
```bash
sudo pacman -S base-devel pkg-config fontconfig libheif
```

> **Note:** On Linux/Windows, use `--features libheif` to enable HEIF support.
> On macOS/iOS, native Apple ImageIO is used instead (no libheif needed).

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
- ✅ HEIF/HEIC support (with `--features libheif`)
- ✅ Wayland and X11 support
- ⚠️ Face detection: Not tested

#### Known Limitations on Linux

**Face Detection with InsightFace Feature:**
The `--features face_detection_insightface` flag may have compatibility issues on Linux depending on your system configuration and ONNX Runtime version.

**Recommendation:**
- Default build (without `face_detection_insightface`): ✅ Recommended
- With `face_detection_insightface` feature: ⚠️ Experimental, may require additional system dependencies


### iOS FFI
```bash
./build_ios.sh
```

Used with iOS xcode swift native project.

### License
Most of the code depends on the NON-AI-MIT license, while some portions are under the MIT or Apache 2.0 licenses.

In particular, the image data has been processed by pmnxis, but the original vector icons were used with permission from シエミカ (X: shiemika324).

All icons are strictly prohibited from being used for any form of AI training without exception.

The "Cheki" tab icon is derived from an illustration by **いらすとや** (みふねたかし):
["かわいいアイドルファンのイラスト（ペンライトあり）"](https://www.irasutoya.com/2020/08/blog-post_978.html),
modified to a monochrome silhouette. Used under [irasutoya's terms of use](https://www.irasutoya.com/p/terms.html).

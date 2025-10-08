<!--
SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Chama Optics

[![dependency status](https://deps.rs/repo/github/pmnxis/chama-optics/status.svg)](https://deps.rs/repo/github/pmnxis/chama-optics)
[![Build Status](https://github.com/pmnxis/chama-optics/workflows/CI/badge.svg)](https://github.com/pmnxis/chama-optics/actions?workflow=CI)


![Build Status](assets/chama-optics.png)

Chama Optics is a program created for mirrorless and DSLR cameras, inspired by the travel VTuber Akai Haato([赤井はあと](https://www.youtube.com/@AkaiHaato)), who loves photography.

It analyzes the EXIF data embedded in photos along with the user’s settings to resize, compress, and tag images before saving them, while also offering additional convenient features.

This program is developed in [Rust](https://rust-lang.org/) using the [eframe](https://github.com/emilk/egui/tree/master/crates/eframe)/[egui](https://github.com/emilk/egui/) framework, along with libraries such as libheif and exif-rs.

## Current Status
- [x] Read JPEG/PNG and other common formats
- [x] Read HEIF photos (libheif)
- [x] Read EXIF data (supports up to 2.3.x standard; not yet compliant with 3.0)
- [ ] Save photos with selected frames and settings
- [ ] Watermark feature
- [ ] Web application supports (libheif wasm)


### Testing locally

`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`

On Fedora Rawhide you need to run:

`dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`

#### macOS
```sh
brew install pkgconf libheif
```

### License
Non-AI-MIT
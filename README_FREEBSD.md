<!--
SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Chama Optics — FreeBSD Build Guide

## Install (one-liner)

```sh
fetch -o - https://raw.githubusercontent.com/pmnxis/chama-optics/master/quick-install-freebsd.sh | sh
```

Prompts you to choose between `stable` (latest release tag) or `latest` (master branch).

## Build from source

```sh
sh build-freebsd.sh
```

The script auto-detects your FreeBSD version, installs required dependencies via `pkg`, and applies the correct cargo features.

## Scripts

| Script | Description |
|---|---|
| `build-freebsd.sh` | Build on the current FreeBSD environment. Auto-detects version and applies the correct cargo features. |
| `quick-install-freebsd.sh` | One-liner installer. Clones the repo, selects stable/latest channel, and runs `build-freebsd.sh`. |

## Supported Versions

| Version | Tested | Cargo Features | Notes |
|---|---|---|---|
| FreeBSD 14.3 | 2026-02-18 | `desktop,libheif,embedded_libheif` | Standard |
| FreeBSD 15.0 | 2026-02-18 | `desktop,libheif` | System libheif (x264 link issue with embedded) |

## Dependencies

The build script automatically installs these via `pkg install`:

**All versions:**
- `git`, `nasm`, `cmake`, `pkgconf` — build tools
- `freetype2`, `fontconfig` — font rendering
- `libxcb`, `libX11`, `libxkbcommon` — X11 GUI
- Rust toolchain via [rustup](https://rustup.rs/)

**FreeBSD 15.0 additionally:**
- `gcc` — provides `libstdc++` (required by embedded libheif codecs)
- `libheif` — system libheif (since `embedded_libheif` has x264 linking issues)

## HEIF Support

- **FreeBSD 14.x**: Uses `embedded_libheif` feature, which statically links libheif. Codec backends (libde265, x265, libaom) are dynamically linked.
- **FreeBSD 15.0**: Uses system libheif (`--features desktop,libheif` without `embedded_libheif`) due to x264 undefined symbol errors during the embedded build — same issue as Arch Linux.

## Per-Version Notes

### FreeBSD 15.0 — libstdc++

FreeBSD 15.0's default C++ runtime is `libc++`, but the embedded libheif build system links against `libstdc++`. The build script installs `gcc` and creates a symlink from `/usr/local/lib/gcc{ver}/libstdc++.so` to `/usr/local/lib/libstdc++.so`.

### Face Detection

`face_detection_insightface` is **not supported** on FreeBSD. ONNX Runtime does not provide prebuilt binaries for the `x86_64-unknown-freebsd` target.

## Output

- Release build: `target/release/chama-optics`

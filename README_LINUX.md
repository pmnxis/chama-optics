<!--
SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Chama Optics — Linux Build Guide

## Install (one-liner)

```bash
curl -sSf https://raw.githubusercontent.com/pmnxis/chama-optics/master/quick-install-linux.sh | bash
```

Prompts you to choose between `stable` (latest release tag) or `latest` (master branch).

## Build from source

```bash
bash build-linux.sh
```

## Supported Distributions

| Distro | Tested | Cargo Features | Notes |
|---|---|---|---|
| Debian 12 (Bookworm) | 2026-02-18 | `desktop,libheif,embedded_libheif` | Standard |
| Debian 13 (Trixie) | 2026-02-18 | `desktop,libheif,embedded_libheif` | Standard |
| Ubuntu 22.04 LTS | 2026-02-18 | `desktop,libheif,embedded_libheif` | `libdav1d-dev` must be removed |
| Ubuntu 24.04 LTS | 2026-02-18 | `desktop,libheif,embedded_libheif` | Standard |
| Fedora 41 | 2026-02-18 | `desktop,libheif,embedded_libheif` | Standard |
| Rocky Linux 9 | 2026-02-18 | `desktop,libheif,embedded_libheif` | freetype 2.13.3 built from source |
| Arch Linux | 2026-02-18 | `desktop,libheif` | System libheif (no `embedded_libheif`) |

## Scripts

| Script | Description |
|---|---|
| `build-linux.sh` | Build on the current Linux environment. Auto-detects distro and applies the correct cargo features. |
| `package-all-linux-distro.sh` | Build & package across all supported distros via Proxmox LXC containers (VMIDs 400–406). Run from a Mac/host machine. |

## HEIF Support

On Linux, HEIF/HEIC image support is provided by [libheif](https://github.com/nicecapj/libheif-rs).

- **Most distros**: Use `embedded_libheif` feature, which statically links libheif. Codec backends (libde265, x265, libaom) are still dynamically linked.
- **Arch Linux**: Use system libheif (`--features desktop,libheif` without `embedded_libheif`) due to x264 linking issues with the embedded build.

## Per-Distro Notes

### Ubuntu 22.04 LTS

Ubuntu 22.04's `libdav1d-dev` provides an old dav1d API (0.9.x) that is **incompatible** with embedded libheif's expected API. Specifically, `Dav1dSettings.n_threads` does not exist (Ubuntu 22.04 uses `n_tile_threads` instead).

`build-linux.sh` will warn and ask for confirmation before removing `libdav1d-dev`. The resulting binary will **not** have dav1d (AV1) decode support.

### Ubuntu 22.04 / Rocky Linux 9 — freetype

These distros ship freetype2 versions older than what `freetype-sys` requires (pkg-config version >= 24.3.18, i.e. freetype 2.13.3+).

freetype 2.13.3 must be built from source and installed to `/usr/local` with `PKG_CONFIG_PATH` configured via `/etc/profile.d/freetype-local.sh`. `build-linux.sh` checks the freetype version before building.

### Arch Linux

Arch uses the system-provided libheif (1.21+) rather than the embedded build. The `embedded_libheif` feature causes x264 undefined symbol errors on Arch due to how cmake discovers x264 during the embedded libheif build but the Rust linker doesn't link it.

## Features

- Wayland and X11 support
- HEIF/HEIC support (via libheif or embedded_libheif)
- Face detection with InsightFace (experimental, may require additional system dependencies depending on ONNX Runtime version)

## Output

- Release build: `target/release/chama-optics`

## Logo Assets

During build, manufacturer logo SVGs are downloaded from Wikipedia. If a Wikipedia URL fails (403, 404, rate limit), the build system automatically falls back to a GitHub mirror at [pmnxis/chama-optics-assets](https://github.com/pmnxis/chama-optics-assets).

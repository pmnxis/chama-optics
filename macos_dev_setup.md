<!--
SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Development Setup for External Resources

## Overview

The `ext_res` feature allows resources (fonts, models, logos) to be loaded from an external `Resources/` directory instead of being embedded in the binary. This reduces binary size from ~60MB to ~16MB, which is crucial for Apple notarization.

## Directory Structure

When using `ext_res`:

### Production (App Bundle)
```
Chama Optics.app/
├── Contents/
│   ├── MacOS/
│   │   └── chama-optics         # 16MB binary
│   └── Resources/
│       ├── Fonts/                # ~40MB fonts
│       ├── Models/               # ~16MB ONNX model
│       └── Logos/                # SVG files
```

### Development (cargo run)
```
rust-core/
├── target/
│   ├── debug/
│   │   ├── chama-optics
│   │   └── Resources/           # Copied by setup_dev_resources.sh
│   │       ├── Fonts/
│   │       ├── Models/
│   │       └── Logos/
│   └── release/
│       ├── chama-optics
│       └── Resources/           # Copied by setup_dev_resources.sh
│           ├── Fonts/
│           ├── Models/
│           └── Logos/
```

## Development Workflow

### First Time Setup

1. **Build the project** (this downloads the model):
   ```bash
   cargo build --release --features "face_detection_insightface,ext_res"
   ```

2. **Setup development resources**:
   ```bash
   ./setup_dev_resources.sh
   ```

   This copies fonts, models, and logos to `target/debug/Resources/` and `target/release/Resources/`.

### Daily Development

After the initial setup, you can use `cargo run` normally:

```bash
# Run in debug mode
cargo run --features "face_detection_insightface,ext_res"

# Run in release mode
cargo run --release --features "face_detection_insightface,ext_res"
```

The app will find resources in `target/debug/Resources/` or `target/release/Resources/`.

### Re-run Setup Script When:

- You add new font files to `assets/fonts/`
- After cleaning the build (`cargo clean`)
- After updating the ONNX model

```bash
./setup_dev_resources.sh
```

### Building macOS App Bundle

For production builds with proper code signing:

```bash
./build_mac.sh
```

This will:
1. Build with `ext_res` feature
2. Create the app bundle
3. Copy resources to `Contents/Resources/`
4. Copy and link required dylibs
5. **Automatically run `setup_dev_resources.sh`** for subsequent cargo run

## Without ext_res (Embedded Resources)

If you want a standalone binary with all resources embedded:

```bash
cargo build --release --features "face_detection_insightface"
```

Binary will be ~60MB but can run anywhere without external resources.

## Resource Loading Logic

The code checks multiple locations in order:

1. **App bundle**: `Contents/Resources/` (for production)
2. **Development**: `target/{debug,release}/Resources/` (for cargo run)

See [src/resources.rs](src/resources.rs) for implementation details.

## .gitignore

Resources in `target/` are automatically ignored (covered by `target` entry in `.gitignore`).

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

include!("prebuilt_src/fonts.rs");
include!("prebuilt_src/logo.rs");

use builtin_fonts::*;
use std::env;
use std::path::PathBuf;

/// Build assets for face detection models
#[allow(unused)]
pub const BUILTIN_FACE_MODELS: [BuildAsset; 1] = [BuildAsset {
    // InsightFace buffalo_l model (v0.7)
    // Source: https://github.com/deepinsight/insightface/releases/tag/v0.7
    url: "https://github.com/deepinsight/insightface/releases/download/v0.7/buffalo_l.zip",
    expected_md5: "6c0e929fd3b6ab517170b732ced18c68", // MD5 for buffalo_l.zip
    file_name: Some("buffalo_l.zip"),
    unzip: true,
    extract_file_names: Some(&["det_10g.onnx"]),
    env_keys: Some(&["INSIGHTFACE_MODEL_PATH"]),
}];

fn get_git_commit_hash(short: bool) -> Option<String> {
    let args = if short {
        vec!["rev-parse", "--short", "HEAD"]
    } else {
        vec!["rev-parse", "HEAD"]
    };

    let output = std::process::Command::new("git").args(args).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let hash = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string()
        .to_ascii_lowercase();
    Some(hash)
}

fn main() {
    let logo_csv_path = PathBuf::from("assets/logo_mnf.csv");
    let tmp_dir = PathBuf::from("assets/download");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=locales");
    println!("cargo:rerun-if-changed={}", logo_csv_path.display());

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/chama-optics-x256.ico");
        _ = res.compile();
    }

    // Get project name and version
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Failed to get cargo metadata.");

    if let Some(package) = metadata.packages.first() {
        let project_name = &package.name;
        let project_version = package.version.to_string();

        println!("cargo:rustc-env=PROJECT_NAME={project_name}");
        println!("cargo:rustc-env=PROJECT_VERSION={project_version}");
    } else {
        println!("cargo:rustc-env=PROJECT_NAME=unknown");
        println!("cargo:rustc-env=PROJECT_VERSION=Unknown");
    }

    // Get the Git commit hash
    if let (Some(commit_hash), Some(commit_short_hash)) =
        (get_git_commit_hash(false), get_git_commit_hash(true))
    {
        let is_dirty = {
            let output = std::process::Command::new("git")
                .args(["status", "--porcelain"])
                .output()
                .expect("Failed to execute git status");

            !output.stdout.is_empty()
        };

        let (dirty_str, short_dirty_str) = if is_dirty {
            ("-dirty".to_owned(), "-dirty".to_owned())
        } else {
            ("".to_owned(), "  ".to_owned())
        };

        let output = std::process::Command::new("git")
            .args(["log", "-1", "--format=%ai", &commit_hash])
            .output()
            .expect("Failed to execute command");
        let commit_datetime = String::from_utf8_lossy(&output.stdout);

        // Output the version and commit hash to a file
        // This is u8 array

        println!("cargo:rustc-env=GIT_COMMIT_HASH={commit_hash}{dirty_str}");

        println!("cargo:rustc-env=GIT_COMMIT_SHORT_HASH={commit_short_hash}{short_dirty_str}");
        println!("cargo:rustc-env=GIT_COMMIT_DATETIME={commit_datetime}");
    } else {
        println!("cargo:rustc-env=GIT_COMMIT_HASH=unknown");

        println!("cargo:rustc-env=GIT_COMMIT_SHORT_HASH=unknown");
        println!("cargo:rustc-env=GIT_COMMIT_DATETIME=unknown");
    }

    // Enable only build-script logic in build_asset.rs
    for asset in BUILTIN_FONTS {
        asset.load(&out_dir);
    }

    for asset in BUILTIN_FACE_MODELS {
        asset.load(&tmp_dir);
    }

    // Logo related
    std::fs::create_dir_all(&tmp_dir).expect("failed to create temp_dir directory");
    let generated_dir = PathBuf::from("assets/auto_generated");
    std::fs::create_dir_all(&generated_dir).expect("failed to create src/generated directory");
    let output_file = generated_dir.join("logo_assets.rs");

    // Generate Rust source code
    let generated_code = builtin_logos::generate(&tmp_dir, &logo_csv_path);

    // std::fs::write(&output_file, generated_code).expect("failed to write generated logo_assets.rs");
    write_if_changed(&output_file, &generated_code);

    println!(
        "cargo:rustc-env=LOGO_ASSET_PATH={}",
        std::fs::canonicalize(&output_file)
            .unwrap_or_else(|_| output_file.clone())
            .display()
    );
    if std::env::var("LOGO_ASSET_PATH").is_ok() {
        println!("cargo:rustc-cfg=has_logo_asset_path");
    }
    println!("✅ Generated {}", output_file.display());

    #[cfg(target_os = "linux")]
    {
        if let Ok(pkg_path) = env::var("PKG_CONFIG_PATH") {
            for path in pkg_path.split(':') {
                if path.contains("libheif/build") {
                    let lib_path = format!("{}/libheif", path.trim_end_matches('/'));
                    println!("cargo:rustc-link-search=native={}", lib_path);
                    println!("cargo:rustc-link-lib=heif");
                    break;
                }
            }
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_TIME={now}");

    // Generate swift-bridge code for Metal rendering (iOS/macOS only)
    // Skip this on Windows and Linux
    #[cfg(all(
        any(target_os = "macos", target_os = "ios"),
        feature = "metal_rendering"
    ))]
    {
        let bridge_files = vec!["src/metal_renderer/ffi_bridge.rs"];
        swift_bridge_build::parse_bridges(bridge_files)
            .write_all_concatenated(out_dir, env!("CARGO_PKG_NAME"));
    }
}

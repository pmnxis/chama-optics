/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

mod builtin_fonts {
    include!("src/fonts/builtin_fonts.rs");

    impl BuildAsset {
        /// Download, verify MD5, unzip (if needed), and set cargo env var
        pub fn load(&self, out_dir: &std::path::Path) {
            use md5::{Digest, Md5};
            use std::fs;
            use std::io::{self, Cursor, Read};
            use zip::ZipArchive;

            let file_name = self.file_name.unwrap_or_else(|| {
                self.url
                    .split('/')
                    .next_back()
                    .expect("Cannot determine file name from URL")
            });

            let zip_path = out_dir.join(file_name);

            println!("cargo:rerun-if-changed=build.rs");

            // Download
            if !zip_path.exists() {
                println!("Downloading {} ...", self.url);
                let resp = reqwest::blocking::get(self.url).expect("failed to download file");
                let bytes = resp.bytes().expect("failed to read response bytes");
                fs::write(&zip_path, &bytes).expect("failed to write downloaded file");
            }

            // MD5 check
            let mut buffer = Vec::new();
            fs::File::open(&zip_path)
                .and_then(|mut f| f.read_to_end(&mut buffer))
                .expect("failed to read downloaded file for MD5");

            let mut hasher = Md5::new();
            hasher.update(&buffer);
            let actual_md5 = format!("{:x}", hasher.finalize());

            if actual_md5 != self.expected_md5 {
                panic!(
                    "MD5 checksum mismatch for {}!\nExpected: {}\nActual:   {}",
                    zip_path.display(),
                    self.expected_md5,
                    actual_md5
                );
            } else {
                println!("MD5 checksum verified ✅");
            }

            // Unzip if necessary
            let final_path = if self.unzip {
                let extract_name = self
                    .extract_file_name
                    .expect("extract_file_name required when unzip=true");
                let font_path = out_dir.join(extract_name);

                if !font_path.exists() {
                    let reader = Cursor::new(buffer);
                    let mut archive = ZipArchive::new(reader).expect("failed to open zip archive");
                    let mut file = archive
                        .by_name(extract_name)
                        .unwrap_or_else(|_| panic!("{extract_name} not found in ZIP"));
                    let mut extracted = Vec::new();
                    io::copy(&mut file, &mut extracted).expect("failed to extract file");
                    fs::write(&font_path, extracted).expect("failed to write extracted file");
                }
                font_path
            } else {
                zip_path
            };

            // Expose to cargo environment
            println!("cargo:rustc-env={}={}", self.env_key, final_path.display());
        }
    }
}

use builtin_fonts::*;
use std::env;
use std::path::PathBuf;

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
    println!("cargo:rerun-if-changed=locales");

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
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    for asset in BUILTIN_FONTS {
        asset.load(&out_dir);
    }
}

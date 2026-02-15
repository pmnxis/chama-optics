/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

// Build-time asset definition & handler.
// This module defines static assets (download sources, MD5, etc.).
// At build-time, it can download and verify these assets automatically.
// At runtime, only constant metadata is exposed.

/// Build asset to download
#[allow(dead_code)]
pub struct BuildAsset {
    /// Resource download URL
    pub url: &'static str,
    /// Expected MD5 checksum
    pub expected_md5: &'static str,
    /// Optional local file name (if None, inferred from URL)
    pub file_name: Option<&'static str>,
    /// Whether to unzip after download
    pub unzip: bool,
    /// File to extract from the ZIP archive
    pub extract_file_names: Option<&'static [&'static str]>,
    /// Cargo environment variable key to export
    pub env_keys: Option<&'static [&'static str]>,
}

// Common definitions (usable both in build.rs and src)
#[allow(unused)] // actually it used
pub const BUILTIN_FONTS: [BuildAsset; 2] = [
    BuildAsset {
        url: "https://dl.dafont.com/dl/?f=digital_7",
        expected_md5: "50960f1aa2b138b3a81fa2b48d4f87bc",
        file_name: Some("digital_7.zip"),
        unzip: true,
        extract_file_names: Some(&["digital-7.ttf", "digital-7 (italic).ttf"]),
        env_keys: Some(&["DIGITAL_7_FONT_PATH", "DIGITAL_7_ITALIC_FONT_PATH"]),
    },
    BuildAsset {
        url: "https://github.com/googlefonts/dynapuff/raw/main/fonts/variable/DynaPuff%5Bwdth%2Cwght%5D.ttf",
        expected_md5: "b66fd4ae8edcf807beb5136d89d0f6cb",
        file_name: Some("DynaPuff-Variable.ttf"),
        unzip: false,
        extract_file_names: None,
        env_keys: Some(&["DYNAPUFF_FONT_PATH"]),
    },
];

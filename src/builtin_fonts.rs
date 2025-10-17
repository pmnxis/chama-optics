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
    pub extract_file_name: Option<&'static str>,
    /// Cargo environment variable key to export
    pub env_key: &'static str,
}

// Common definitions (usable both in build.rs and src)
pub const BUILTIN_FONTS: [BuildAsset; 1] = [BuildAsset {
    url: "https://dl.dafont.com/dl/?f=digital_7",
    expected_md5: "50960f1aa2b138b3a81fa2b48d4f87bc",
    file_name: Some("digital_7.zip"),
    unzip: true,
    extract_file_name: Some("digital-7.ttf"),
    env_key: "DIGITAL_7_FONT_PATH",
}];

#[allow(unused)] // actually it used
pub const ASSET_DS_DIGITAL: &BuildAsset = &BUILTIN_FONTS[0];

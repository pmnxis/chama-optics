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
        url: "https://codeload.github.com/jpt/barlow/zip/refs/heads/master",
        expected_md5: "89900ce5621c6efdeed006232cbccc6f",
        file_name: Some("barlow-master.zip"),
        unzip: true,
        extract_file_names: Some(&[
            "barlow-master/fonts/otf/Barlow-Black.otf",
            // "barlow-master/fonts/otf/Barlow-BlackItalic.otf",
            "barlow-master/fonts/otf/Barlow-Bold.otf",
            // "barlow-master/fonts/otf/Barlow-BoldItalic.otf",
            "barlow-master/fonts/otf/Barlow-ExtraBold.otf",
            // "barlow-master/fonts/otf/Barlow-ExtraBoldItalic.otf",
            "barlow-master/fonts/otf/Barlow-ExtraLight.otf",
            // "barlow-master/fonts/otf/Barlow-ExtraLightItalic.otf",
            // "barlow-master/fonts/otf/Barlow-Italic.otf",
            "barlow-master/fonts/otf/Barlow-Light.otf",
            // "barlow-master/fonts/otf/Barlow-LightItalic.otf",
            "barlow-master/fonts/otf/Barlow-Medium.otf",
            // "barlow-master/fonts/otf/Barlow-MediumItalic.otf",
            "barlow-master/fonts/otf/Barlow-Regular.otf",
            "barlow-master/fonts/otf/Barlow-SemiBold.otf",
            // "barlow-master/fonts/otf/Barlow-SemiBoldItalic.otf",
            "barlow-master/fonts/otf/Barlow-Thin.otf",
            // "barlow-master/fonts/otf/Barlow-ThinItalic.otf",
        ]),
        env_keys: Some(&[
            "BARLOW_900_FONT_PATH",
            // "BARLOW_900_ITALIC_FONT_PATH",
            "BARLOW_700_FONT_PATH",
            // "BARLOW_700_ITALIC_FONT_PATH",
            "BARLOW_800_FONT_PATH",
            // "BARLOW_800_ITALIC_FONT_PATH",
            "BARLOW_200_FONT_PATH",
            // "BARLOW_200_ITALIC_FONT_PATH",
            // "BARLOW_400_ITALIC_FONT_PATH",
            "BARLOW_300_FONT_PATH",
            // "BARLOW_300_ITALIC_FONT_PATH",
            "BARLOW_500_FONT_PATH",
            // "BARLOW_500_ITALIC_FONT_PATH",
            "BARLOW_400_FONT_PATH",
            "BARLOW_600_FONT_PATH",
            // "BARLOW_600_ITALIC_FONT_PATH",
            "BARLOW_100_FONT_PATH",
            // "BARLOW_100_ITALIC_FONT_PATH",
        ]),
    },
];

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[cfg(all(feature = "desktop", not(feature = "ios_integration")))]
pub(crate) mod check_update;

#[cfg(feature = "rfd")]
pub(crate) mod async_file_dialog;

#[cfg(target_arch = "wasm32")]
pub(crate) mod web_helper;

#[cfg(target_arch = "wasm32")]
pub(crate) mod web_download;

/// Apply NFC normalization for display on macOS.
/// macOS APFS/HFS+ uses NFD for file paths, which decomposes Korean characters (한글 → ㅎㅏㄴㄱㅡㄹ).
/// This function recomposes them for proper display.
#[cfg(target_os = "macos")]
pub(crate) fn normalize_for_display(s: String) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfc().collect()
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn normalize_for_display(s: String) -> String {
    s
}

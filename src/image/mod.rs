/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[cfg(all(feature = "desktop", not(feature = "ios_integration")))] // todo! - heif for iOS
pub(crate) mod heic;

#[allow(dead_code)]
pub(crate) mod common;

// Always compile exif_impl as it's needed by core
pub(crate) mod exif_impl;

#[cfg(all(feature = "desktop", not(feature = "ios_integration")))]
pub(crate) mod loader;

pub(crate) mod make_note;

pub(crate) mod packed_image;

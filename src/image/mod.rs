/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

// libheif is only used on Windows/Linux (macOS/iOS use native Apple ImageIO)
#[cfg(feature = "libheif")]
pub(crate) mod heic;

#[cfg(target_arch = "wasm32")]
pub(crate) mod heic_web;

#[allow(dead_code)]
pub(crate) mod common;

// Always compile exif_impl as it's needed by core
pub(crate) mod exif_impl;

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub(crate) mod loader;

pub(crate) mod make_note;

pub(crate) mod datetime_edit;

pub(crate) mod packed_image;

pub(crate) mod exif_inject;

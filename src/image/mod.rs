/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

// Conditional compilation for desktop-only modules
#[cfg(feature = "desktop")]
pub(crate) mod heic;

#[cfg(feature = "desktop")]
#[allow(dead_code)]
pub(crate) mod common;

// Always compile exif_impl as it's needed by core
pub(crate) mod exif_impl;

#[cfg(feature = "desktop")]
pub(crate) mod loader;
#[cfg(feature = "desktop")]
pub(crate) mod make_note;
#[cfg(feature = "desktop")]
pub(crate) mod packed_image;

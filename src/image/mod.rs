/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

// Conditional compilation for desktop-only modules
#[cfg(feature = "desktop")]
pub(crate) mod heic;

#[cfg(any(feature = "desktop", feature = "web", feature = "ios_integration"))]
#[allow(dead_code)]
pub(crate) mod common;

// Always compile exif_impl as it's needed by core
pub(crate) mod exif_impl;

#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) mod loader;
#[cfg(any(feature = "desktop", feature = "web", feature = "ios_integration"))]
pub(crate) mod make_note;
#[cfg(any(feature = "desktop", feature = "web", feature = "ios_integration"))]
pub(crate) mod packed_image;

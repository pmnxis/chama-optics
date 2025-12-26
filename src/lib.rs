/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

rust_i18n::i18n!("locales");

// Conditional compilation based on features
#[cfg(feature = "desktop")]
mod app;
#[cfg(feature = "desktop")]
mod app_state;
#[cfg(feature = "desktop")]
mod tabs;
#[cfg(feature = "desktop")]
mod ui_components;
#[cfg(feature = "desktop")]
mod ui_state;

#[cfg(feature = "desktop")]
pub mod export_config;
#[cfg(feature = "desktop")]
pub(crate) mod fonts;
#[cfg(feature = "desktop")]
pub(crate) mod import_config;
#[cfg(feature = "desktop")]
pub(crate) use export_config::scale_config;
#[cfg(feature = "desktop")]
pub(crate) mod langs;
#[cfg(feature = "desktop")]
pub(crate) use art::ART_UNIFY;
#[cfg(feature = "desktop")]
pub(crate) use fonts::FONTS_UNIFY;
#[cfg(feature = "desktop")]
pub(crate) use fonts::font_unify::{BuiltinFontIndex, FontSelection};

// Image module - shared between desktop and iOS
pub(crate) mod image;
#[cfg(not(feature = "desktop"))]
pub(crate) use image::exif_impl;
#[cfg(feature = "desktop")]
pub(crate) use image::{exif_impl, packed_image};

#[cfg(feature = "desktop")]
pub mod art;
#[cfg(feature = "desktop")]
pub(crate) mod effect;

#[cfg(feature = "desktop")]
pub mod test_helper;

mod util;

#[cfg(feature = "desktop")]
pub mod theme;

// Mobile UI optimizations
pub mod mobile;

// Headless core library (no GUI dependencies)
pub mod core;

// FFI for iOS/Swift integration
#[cfg(any(target_os = "ios", feature = "ios_integration"))]
pub mod ffi;

#[cfg(feature = "desktop")]
pub use app::ChamaOptics;
#[cfg(feature = "desktop")]
pub use app_state::AppState;
#[cfg(feature = "desktop")]
pub use ui_state::UiState;

#[macro_use]
pub mod dump;

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

rust_i18n::i18n!("locales");

// Conditional compilation based on features
// GUI modules available for both desktop and web
#[cfg(any(feature = "desktop", feature = "web"))]
mod app;
#[cfg(any(feature = "desktop", feature = "web"))]
mod app_state;
#[cfg(any(feature = "desktop", feature = "web"))]
mod tabs;
#[cfg(any(feature = "desktop", feature = "web"))]
mod ui_components;
#[cfg(any(feature = "desktop", feature = "web"))]
mod ui_state;

#[cfg(any(feature = "desktop", feature = "web"))]
pub mod export_config;
#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) mod fonts;
#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) mod import_config;
#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) use export_config::scale_config;
#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) mod langs;
#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) use art::ART_UNIFY;
#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) use fonts::FONTS_UNIFY;
#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) use fonts::font_unify::{BuiltinFontIndex, FontSelection};

// Image module - shared between desktop and iOS
pub(crate) mod image;
#[cfg(not(any(feature = "desktop", feature = "web")))]
pub(crate) use image::exif_impl;
#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) use image::{exif_impl, packed_image};

#[cfg(any(feature = "desktop", feature = "web"))]
pub mod art;

// Effect modules - available for desktop, web, and iOS integration
#[cfg(any(feature = "desktop", feature = "web", feature = "ios_integration"))]
pub mod effect;

#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) mod image_group;

#[cfg(feature = "desktop")]
pub mod test_helper;

mod util;

#[cfg(any(feature = "desktop", feature = "web"))]
pub mod theme;

// Mobile UI optimizations
pub mod mobile;

// Headless core library (no GUI dependencies)
pub mod core;

// FFI for iOS/macOS Swift integration
#[cfg(any(target_os = "ios", target_os = "macos", feature = "ios_integration"))]
pub mod ffi;

// Metal renderer for iOS/macOS egui integration
#[cfg(any(feature = "metal_rendering", target_os = "macos", target_os = "ios"))]
pub mod metal_renderer;

#[cfg(any(feature = "desktop", feature = "web"))]
pub use app::ChamaOptics;
#[cfg(any(feature = "desktop", feature = "web"))]
pub use app_state::AppState;
#[cfg(any(feature = "desktop", feature = "web"))]
pub use ui_state::UiState;

#[macro_use]
pub mod dump;

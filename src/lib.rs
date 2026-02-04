/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

rust_i18n::i18n!("locales");

// Conditional compilation based on features
// GUI modules available for both desktop and web
#[cfg(not(feature = "ios_integration"))]
mod app;
#[cfg(not(feature = "ios_integration"))]
mod app_state;

#[cfg(not(feature = "ios_integration"))]
mod tabs;
#[cfg(not(feature = "ios_integration"))]
mod ui_components;
#[cfg(not(feature = "ios_integration"))]
mod ui_state;

// Support modules for desktop, web, and iOS
// GUI-only modules
pub mod export_config;
#[cfg(not(feature = "ios_integration"))]
pub(crate) mod import_config;
pub(crate) use export_config::scale_config;

// Modules needed by iOS FFI
pub(crate) mod fonts;
#[cfg(not(feature = "ios_integration"))]
pub(crate) mod langs;
pub(crate) mod resources; // Unified resource management (fonts, models, logos)
pub(crate) use art::ART_UNIFY;
#[cfg(not(feature = "ios_integration"))]
pub(crate) use fonts::FONTS_UNIFY;
#[cfg(not(feature = "ios_integration"))]
pub(crate) use fonts::font_unify::{BuiltinFontIndex, FontSelection};

// Image module - shared between all platforms
pub(crate) mod image;
pub(crate) use image::{exif_impl, packed_image};

// Art module - needed by iOS FFI
pub mod art;

// Effect modules - available for desktop, web, and iOS integration
pub mod effect;

#[cfg(not(feature = "ios_integration"))]
pub(crate) mod image_group;

#[cfg(feature = "desktop")]
pub mod test_helper;

mod util;

// Theme module - needed by iOS FFI
pub mod theme;

// Mobile UI optimizations (GUI-only)
#[cfg(not(feature = "ios_integration"))]
pub mod mobile;

// Headless core library (no GUI dependencies)
pub mod core;

// FFI for iOS/macOS Swift integration
#[cfg(any(target_os = "ios", feature = "ios_integration"))]
pub mod ffi;

// Metal FFI for iOS Swift integration (full theme support)
#[cfg(feature = "ios_integration")]
pub mod ffi_metal;

// Metal renderer for iOS/macOS egui integration
#[cfg(all(
    feature = "metal_rendering",
    any(target_os = "macos", target_os = "ios")
))]
pub mod metal_renderer;

#[cfg(not(feature = "ios_integration"))]
pub use app::ChamaOptics;
#[cfg(not(feature = "ios_integration"))]
pub use app_state::AppState;
#[cfg(not(feature = "ios_integration"))]
pub use ui_state::UiState;

#[macro_use]
pub mod dump;

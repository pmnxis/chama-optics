/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

rust_i18n::i18n!("locales");

// Conditional compilation based on features
// GUI modules available for both desktop and web
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
mod app;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
mod app_state;

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
mod tabs;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
mod ui_components;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
mod ui_state;

// Support modules for desktop, web, and iOS
// GUI-only modules
pub mod export_config;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub(crate) mod import_config;
pub(crate) use export_config::scale_config;

// Modules needed by iOS FFI
pub(crate) mod fonts;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub(crate) mod langs;
pub mod resources; // Unified resource management (fonts, models, logos)
pub(crate) use art::ART_UNIFY;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub(crate) use fonts::FONTS_UNIFY;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub(crate) use fonts::font_unify::{BuiltinFontIndex, FontSelection};

// Image module - shared between all platforms
pub(crate) mod image;
pub(crate) use image::{exif_impl, packed_image};

// Art module - needed by iOS FFI
pub mod art;

// Effect modules - available for desktop, web, and iOS integration
pub mod effect;

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub(crate) mod image_group;

#[cfg(feature = "desktop")]
pub mod test_helper;

pub mod error;
mod util;

// Theme module - needed by iOS FFI
pub mod theme;

// Headless core library (no GUI dependencies)
pub mod core;

// Mobile FFI for iOS/Android integration (full theme support with Metal rendering)
#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
pub mod ffi_mobile;

// Apple FFI for both iOS and macOS (shared platform-agnostic functions)
// Also used by Android via Skip Fuse (Swift compiled natively for Android)
#[cfg(any(
    target_os = "ios",
    target_os = "macos",
    feature = "ios_integration",
    feature = "android_integration"
))]
pub mod ffi_apple;

// Apple native HEIF decoder (iOS mandatory, macOS optional)
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub mod ffi_apple_heif;

// Metal renderer for iOS/macOS egui integration
#[cfg(all(
    feature = "metal_rendering",
    any(target_os = "macos", target_os = "ios")
))]
pub mod metal_renderer;

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub use app::ChamaOptics;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub use app_state::AppState;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub use ui_state::UiState;

#[macro_use]
pub mod dump;

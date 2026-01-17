/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Metal rendering backend for egui integration with iOS/macOS
//! This module is only available on macOS/iOS or when metal_rendering feature is enabled

#[cfg(all(
    feature = "metal_rendering",
    any(target_os = "macos", target_os = "ios")
))]
pub mod renderer;

#[cfg(all(
    feature = "metal_rendering",
    any(target_os = "macos", target_os = "ios")
))]
pub mod ffi_bridge;

#[cfg(all(
    feature = "metal_rendering",
    any(target_os = "macos", target_os = "ios")
))]
pub mod input_output;

#[cfg(target_os = "macos")]
pub mod face_detection_macos;

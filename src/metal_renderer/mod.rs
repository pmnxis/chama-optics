/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Metal rendering backend for egui integration with iOS/macOS

#[cfg(feature = "metal_rendering")]
pub mod renderer;

#[cfg(feature = "metal_rendering")]
pub mod ffi_bridge;

#[cfg(feature = "metal_rendering")]
pub mod input_output;

#[cfg(any(feature = "metal_rendering", target_os = "macos"))]
pub mod face_detection_macos;

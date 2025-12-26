/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Headless core library for Chama Optics
//!
//! This module provides core image processing functionality without any GUI dependencies.
//! It can be used for:
//! - iOS/Android integration via FFI
//! - CLI tools
//! - Headless batch processing
//! - Testing

pub mod image_data;
pub mod image_utils;
pub mod processor;
pub mod scale_config;
pub mod theme_data;
pub mod theme_renderer;

pub use image_data::CoreImage;
pub use processor::ImageProcessor;
pub use scale_config::{ScaleConfig, ScaleMode};
pub use theme_data::{ThemeConfig, ThemeType};

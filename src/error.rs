/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Unified error types for the chama-optics crate.

/// Unified error type for image processing operations.
#[derive(Debug, thiserror::Error)]
pub enum ChamaOpticsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image load error: {0}")]
    ImageLoad(image::ImageError),

    #[error("Image processing error: {0}")]
    ImageProcess(image::ImageError),

    #[error("Invalid theme")]
    InvalidTheme,

    #[error("Invalid font")]
    InvalidFont,

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("EXIF error")]
    ExifError,

    #[error("LUT parse error: {0}")]
    LutParse(String),

    #[error("Font not available: {0}")]
    FontNotAvailable(String),
}

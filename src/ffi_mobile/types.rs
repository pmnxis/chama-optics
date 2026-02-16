/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! FFI type definitions for mobile platforms (iOS/Android)
//!
//! This module contains all `#[repr(C)]` structs and enums used in the FFI boundary,
//! plus internal helper types like `ThemeExportParams`.

use std::os::raw::c_char;

use crate::error::ChamaOpticsError;

// ============================================================================
// Core FFI Types
// ============================================================================

/// Theme configuration passed from Swift (legacy, used by chama_generate_preview)
#[repr(C)]
pub struct ThemeConfig {
    /// Theme name (e.g., "one_line", "film")
    pub theme_name: *const c_char,
    /// JSON string of parameter updates
    pub parameters_json: *const c_char,
    /// File path to font (e.g., "/path/to/Barlow-Regular.ttf")
    pub font_path: *const c_char,
    /// Font weight (100-900, e.g., 400 for regular, 700 for bold)
    pub font_weight: u32,
}

/// Internal theme export parameters (Rust-only, not FFI)
pub(crate) struct ThemeExportParams<'a> {
    pub image_path: &'a str,
    pub exif_source_path: &'a str,
    pub output_path: &'a str,
    pub theme_name: &'a str,
    pub params_json: &'a str,
    pub font_path: &'a str,
    pub font_weight: u32,
    pub scale_config: Option<crate::scale_config::ScaleConfig>,
    pub output_format_config: Option<COutputFormatConfig>,
    pub get_alt_fnumber: bool,
    pub use_35mm_focal_length: bool,
    pub exif_override_json: Option<&'a str>,
}

/// RGBA image buffer passed from Swift (supports HEIF via Image I/O)
#[repr(C)]
pub struct RGBAImageBuffer {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Pointer to RGBA data (4 bytes per pixel, interleaved)
    pub data: *const u8,
    /// Data length (width * height * 4)
    pub data_length: usize,
}

/// Error codes returned to Swift
#[repr(C)]
pub enum ChamaError {
    Success = 0,
    InvalidPath = 1,
    InvalidTheme = 2,
    InvalidFont = 3,
    InvalidParameters = 4,
    ImageLoadError = 5,
    ImageProcessError = 6,
    ExifError = 7,
    Unknown = 99,
}

/// Convert ChamaOpticsError to ChamaError (FFI error code)
impl From<ChamaOpticsError> for ChamaError {
    fn from(e: ChamaOpticsError) -> Self {
        match e {
            ChamaOpticsError::Io(_) => ChamaError::ImageLoadError,
            ChamaOpticsError::ImageLoad(_) => ChamaError::ImageLoadError,
            ChamaOpticsError::ImageProcess(_) => ChamaError::ImageProcessError,
            ChamaOpticsError::InvalidTheme => ChamaError::InvalidTheme,
            ChamaOpticsError::InvalidFont => ChamaError::InvalidFont,
            ChamaOpticsError::InvalidParameters(_) => ChamaError::InvalidParameters,
            ChamaOpticsError::ExifError => ChamaError::ExifError,
            ChamaOpticsError::LutParse(_) => ChamaError::ImageProcessError,
            ChamaOpticsError::FontNotAvailable(_) => ChamaError::InvalidFont,
        }
    }
}

// ============================================================================
// Combined Export Pipeline Types
// ============================================================================

/// Face effect type for combined export
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CFaceEffectType {
    None = 0,
    Mosaic = 1,
    Stroke = 2,
    Sticker = 3,
    /// Combined Mosaic + Stroke effect (mosaic inside, stroke border outside)
    MosaicStroke = 4,
}

/// Output format for export
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum COutputFormat {
    Jpeg = 0,
    Png = 1,
    Webp = 2,
}

/// Scale mode for image resizing (matches Swift ScaleMode enum)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CScaleMode {
    /// No scaling - keep original size
    None = 0,
    /// Resize to max width, maintaining aspect ratio
    MaxWidth = 1,
    /// Resize to max height, maintaining aspect ratio
    MaxHeight = 2,
    /// Resize longest side to target, maintaining aspect ratio
    Longside = 3,
    /// Divide both dimensions by scale_value
    Divide = 4,
    /// Find nearest width that preserves aspect ratio using GCD
    NearCommonWidth = 5,
    /// Find nearest height that preserves aspect ratio using GCD
    NearCommonHeight = 6,
    /// Resize and crop to exact dimensions
    ResizeAndCrop = 7,
}

/// Scale configuration for image resizing
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CScaleConfig {
    /// Scale mode
    pub mode: CScaleMode,
    /// Primary value (target width/height/longside depending on mode)
    pub value: u32,
    /// Secondary value (used for ResizeAndCrop height)
    pub sub_value: u32,
    /// Scale divisor (used for Divide mode)
    pub scale_value: f64,
}

/// Theme export configuration
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct COutputFormatConfig {
    /// Output format (JPEG, PNG, or WebP)
    pub output_format: COutputFormat,
    /// Quality (1-100 for JPEG/WebP, ignored for PNG)
    pub quality: u8,
}

/// RGBA color (reusable across stroke, face_detection, etc.)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CRgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Face effect configuration (replaces individual mosaic/stroke/sticker functions)
#[repr(C)]
pub struct CFaceEffectConfig {
    /// Face effect type (None, Mosaic, Stroke, Sticker, MosaicStroke)
    pub effect_type: CFaceEffectType,
    // Mosaic params
    pub mosaic_block_size: u32,
    pub mosaic_intensity: f32,
    // Stroke params
    pub stroke_color: CRgbaColor,
    pub stroke_thickness: u32,
    // Sticker params (sticker_path for file-based, sticker_id for built-in)
    pub sticker_path: *const c_char,
    pub sticker_id: *const c_char,
    pub sticker_scale: f32,
    pub sticker_offset_x: i32,
    pub sticker_offset_y: i32,
}

/// Face detection configuration
#[repr(C)]
pub struct CFaceDetectionConfig {
    pub engine_type: u32,
    pub border_color: CRgbaColor,
    pub border_thickness: u32,
    pub mask_faces: bool,
    pub mask_blur_radius: f32,
    pub speed_mode: u32,
}

/// Theme export configuration (replaces 5 theme function variants)
#[repr(C)]
pub struct CThemeExportConfig {
    // Required
    pub image_path: *const c_char,
    pub output_path: *const c_char,
    pub theme_name: *const c_char,
    pub params_json: *const c_char,
    pub font_path: *const c_char,
    pub font_weight: u32,
    // Optional (NULL = use defaults)
    pub exif_source_path: *const c_char,
    pub scale_config: *const CScaleConfig,
    pub output_format_config: *const COutputFormatConfig,
    pub exif_override_json: *const c_char,
    // Flags
    pub get_alt_fnumber: bool,
    pub use_35mm_focal_length: bool,
}

/// Configuration for combined export pipeline
#[repr(C)]
pub struct CombinedExportConfig {
    /// Face effect configuration (embeds CFaceEffectConfig)
    pub face_effect: CFaceEffectConfig,

    // Theme settings (NULL if no theme)
    pub theme_name: *const c_char,
    pub theme_params_json: *const c_char,
    pub font_path: *const c_char,
    pub font_weight: u32,

    // Scale settings
    pub scale_config: CScaleConfig,

    // Export settings
    pub output_format: COutputFormat,
    pub quality: u8, // 1-100 for JPEG/WebP

    // EXIF override settings
    pub get_alt_fnumber: bool,
    pub use_35mm_focal_length: bool,
    pub exif_override_json: *const c_char,
}

// ============================================================================
// LUT Types
// ============================================================================

/// C-compatible LUT item for FFI
#[repr(C)]
pub struct CLutItem {
    /// UUID as string (36 characters + null terminator)
    pub id: *const c_char,
    /// Display name
    pub name: *const c_char,
    /// LUT type (0 = Unknown, 1 = 1D, 2 = 3D)
    pub lut_type: u8,
    /// Size info string (e.g., "3D 33x33x33")
    pub size_info: *const c_char,
    /// Whether file is missing
    pub file_missing: bool,
    /// Whether file hash mismatches
    pub hash_mismatch: bool,
}

// ============================================================================
// Color Adjustments Types
// ============================================================================

/// C-compatible struct for color adjustments parameters
#[repr(C)]
pub struct CColorAdjustments {
    pub enabled: bool,
    pub exposure: f32,
    pub contrast: i32,
    pub highlights: i32,
    pub shadows: i32,
    pub whites: i32,
    pub blacks: i32,
    pub clarity: i32,
    pub vibrance: i32,
    pub saturation: i32,
}

// ============================================================================
// Face Detection Types
// ============================================================================

/// C-compatible face rectangle
#[repr(C)]
#[derive(Copy, Clone)]
pub struct CFaceRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Array of face rectangles
#[repr(C)]
pub struct CFaceRectList {
    pub faces: *mut CFaceRect,
    pub count: usize,
}

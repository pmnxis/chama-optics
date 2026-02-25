/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Color Adjustments FFI functions
//!
//! Provides C-compatible functions for applying color adjustments to images
//! from mobile platforms (iOS/Android).

use std::ffi::CStr;
use std::os::raw::c_char;

use super::types::*;

/// Apply color adjustments to an image
///
/// # Parameters
/// - `image_path`: Path to input image
/// - `output_path`: Path for output image
/// - `adjustments`: Pointer to CColorAdjustments struct
/// - `output_format`: Output format (0=JPEG, 1=PNG, 2=WebP)
/// - `quality`: Quality for JPEG/WebP (1-100)
///
/// # Safety
/// - All path pointers must be valid null-terminated C strings
/// - `adjustments` must point to a valid CColorAdjustments struct
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_color_adjustments_apply(
    image_path: *const c_char,
    output_path: *const c_char,
    adjustments: *const CColorAdjustments,
    output_format: COutputFormat,
    quality: u8,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || adjustments.is_null() {
        return ChamaError::InvalidPath;
    }

    let image_path_str = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);

    let adj = unsafe { &*adjustments };

    log::info!(
        "Applying color adjustments to image: {} (exposure={}, contrast={}, saturation={})",
        image_path_str,
        adj.exposure,
        adj.contrast,
        adj.saturation
    );

    // Load image with EXIF orientation
    let mut dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };
    dyn_image.apply_orientation(super::read_exif_orientation(image_path_str));

    // Convert C struct to Rust struct
    let color_adj = crate::effect::color_adjustments::ColorAdjustments {
        enabled: adj.enabled,
        exposure: adj.exposure,
        contrast: adj.contrast,
        highlights: adj.highlights,
        shadows: adj.shadows,
        whites: adj.whites,
        blacks: adj.blacks,
        clarity: adj.clarity,
        vibrance: adj.vibrance,
        saturation: adj.saturation,
    };

    // Apply adjustments
    color_adj.apply(&mut dyn_image);

    // Save with specified format and quality
    match super::save_image_with_c_format(&dyn_image, output_path_str, output_format, quality) {
        Ok(_) => {
            log::info!("Successfully saved adjusted image to: {}", output_path_str);
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to save image: {}", e);
            ChamaError::ImageProcessError
        }
    }
}

/// Apply color adjustments from JSON parameters
///
/// # Parameters
/// - `image_path`: Path to input image
/// - `output_path`: Path for output image
/// - `adjustments_json`: JSON string with adjustment parameters
/// - `output_format`: Output format (0=JPEG, 1=PNG, 2=WebP)
/// - `quality`: Quality for JPEG/WebP (1-100)
///
/// JSON format:
/// ```json
/// {
///     "enabled": true,
///     "exposure": 0.0,
///     "contrast": 0,
///     "highlights": 0,
///     "shadows": 0,
///     "whites": 0,
///     "blacks": 0,
///     "clarity": 0,
///     "vibrance": 0,
///     "saturation": 0
/// }
/// ```
///
/// # Safety
/// - All pointers must be valid null-terminated C strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_color_adjustments_apply_json(
    image_path: *const c_char,
    output_path: *const c_char,
    adjustments_json: *const c_char,
    output_format: COutputFormat,
    quality: u8,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || adjustments_json.is_null() {
        return ChamaError::InvalidPath;
    }

    let image_path_str = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);
    let json_str = cstr_to_str!(adjustments_json, return ChamaError::InvalidParameters);

    let total_start = std::time::Instant::now();

    // Parse JSON into ColorAdjustments
    let color_adj: crate::effect::color_adjustments::ColorAdjustments =
        match serde_json::from_str(json_str) {
            Ok(adj) => adj,
            Err(e) => {
                log::error!("Failed to parse color adjustments JSON: {}", e);
                return ChamaError::InvalidParameters;
            }
        };

    log::info!(
        "⏱️ [CADJ-PERF] START — image: {} (exposure={}, contrast={}, saturation={})",
        image_path_str,
        color_adj.exposure,
        color_adj.contrast,
        color_adj.saturation
    );

    // Load image with EXIF orientation
    let t0 = std::time::Instant::now();
    let mut dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };
    let load_ms = t0.elapsed().as_millis();
    log::info!(
        "⏱️ [CADJ-PERF] image::open() = {}ms ({}x{})",
        load_ms,
        dyn_image.width(),
        dyn_image.height()
    );

    let t1 = std::time::Instant::now();
    dyn_image.apply_orientation(super::read_exif_orientation(image_path_str));
    let orient_ms = t1.elapsed().as_millis();
    log::info!("⏱️ [CADJ-PERF] apply_orientation = {}ms", orient_ms);

    // Apply adjustments
    let t2 = std::time::Instant::now();
    color_adj.apply(&mut dyn_image);
    let apply_ms = t2.elapsed().as_millis();
    log::info!("⏱️ [CADJ-PERF] color_adj.apply = {}ms", apply_ms);

    // Save with specified format and quality
    let t3 = std::time::Instant::now();
    match super::save_image_with_c_format(&dyn_image, output_path_str, output_format, quality) {
        Ok(_) => {
            let save_ms = t3.elapsed().as_millis();
            let total_ms = total_start.elapsed().as_millis();
            log::info!("⏱️ [CADJ-PERF] save_image = {}ms", save_ms);
            log::info!("⏱️ [CADJ-PERF] TOTAL = {}ms", total_ms);
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to save image: {}", e);
            ChamaError::ImageProcessError
        }
    }
}

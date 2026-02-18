/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! FFI interface for mobile platforms (iOS/Android)
//!
//! This module provides C-compatible functions for Swift (iOS) and JNA (Android) to call.
//! Key differences from desktop (egui) version:
//! - Font loading is path-based (no built-in fonts)
//! - Preview-first strategy (EXIF preview extraction)
//! - JSON-based parameter exchange

use std::path::Path;

// ============================================================================
// FFI Helper Macros
// ============================================================================

/// Convert a `*const c_char` to `&str`, returning `$on_err` on failure.
///
/// Usage:
///   let s = cstr_to_str!(ptr, return false);
///   let s = cstr_to_str!(ptr, return ChamaError::InvalidPath);
///   let s = cstr_to_str!(ptr, return std::ptr::null_mut());
macro_rules! cstr_to_str {
    ($ptr:expr, return $err:expr) => {{
        #[allow(unused_unsafe)]
        let __cstr = unsafe { CStr::from_ptr($ptr) };
        match __cstr.to_str() {
            Ok(s) => s,
            Err(_) => return $err,
        }
    }};
}

/// Convert a `*const c_char` to `&str`, returning a default value if NULL or invalid UTF-8.
///
/// Usage:
///   let s = cstr_to_str_or!(ptr, "{}");
///   let s = cstr_to_str_or!(ptr, "");
macro_rules! cstr_to_str_or {
    ($ptr:expr, $default:expr) => {
        if $ptr.is_null() {
            $default
        } else {
            #[allow(unused_unsafe)]
            unsafe { CStr::from_ptr($ptr) }.to_str().unwrap_or($default)
        }
    };
}

// ============================================================================
// Sub-modules
// ============================================================================

mod cheki;
mod color;
mod combined;
mod face;
mod lut;
mod pipeline;
mod theme;
pub mod types;

pub use types::*;

// ============================================================================
// Shared FFI Helper Functions
// ============================================================================

/// Convert CScaleMode FFI enum to core ScaleMode
fn convert_c_scale_mode(mode: CScaleMode) -> crate::scale_config::ScaleMode {
    match mode {
        CScaleMode::None => crate::scale_config::ScaleMode::None,
        CScaleMode::MaxWidth => crate::scale_config::ScaleMode::MaxWidth,
        CScaleMode::MaxHeight => crate::scale_config::ScaleMode::MaxHeight,
        CScaleMode::Longside => crate::scale_config::ScaleMode::Longside,
        CScaleMode::Divide => crate::scale_config::ScaleMode::Divide,
        CScaleMode::NearCommonWidth => {
            crate::scale_config::ScaleMode::NearCommonDivisorConsiderWidth
        }
        CScaleMode::NearCommonHeight => {
            crate::scale_config::ScaleMode::NearCommonDivisorConsiderHeight
        }
        CScaleMode::ResizeAndCrop => crate::scale_config::ScaleMode::ResizeAndCrop,
    }
}

/// Convert CScaleConfig pointer to Option<core::ScaleConfig>
/// Returns None if pointer is null or mode is None
unsafe fn convert_c_scale_config(
    scale_config: *const CScaleConfig,
) -> Option<crate::scale_config::ScaleConfig> {
    if scale_config.is_null() {
        return None;
    }
    let config_ref = unsafe { &*scale_config };
    if config_ref.mode == CScaleMode::None {
        return None;
    }
    Some(crate::scale_config::ScaleConfig {
        mode: convert_c_scale_mode(config_ref.mode),
        value: config_ref.value,
        sub_value: config_ref.sub_value,
        scale_value: config_ref.scale_value as f32,
    })
}

/// Convert COutputFormat to export_config OutputExtension
fn convert_c_output_format(
    format: COutputFormat,
) -> crate::export_config::output_format::OutputExtension {
    match format {
        COutputFormat::Jpeg => crate::export_config::output_format::OutputExtension::Jpeg,
        COutputFormat::Png => crate::export_config::output_format::OutputExtension::PngOptimized,
        COutputFormat::Webp => crate::export_config::output_format::OutputExtension::Webp,
    }
}

/// Save a DynamicImage with the specified output format and quality.
/// Consolidates the repeated JPEG/PNG/WebP save blocks.
fn save_image_with_c_format(
    image: &image::DynamicImage,
    output_path: &str,
    format: COutputFormat,
    quality: u8,
) -> Result<(), image::ImageError> {
    match format {
        COutputFormat::Jpeg => {
            use image::codecs::jpeg::JpegEncoder;
            let file =
                std::fs::File::create(output_path).map_err(|e| image::ImageError::IoError(e))?;
            let mut encoder = JpegEncoder::new_with_quality(file, quality);
            encoder.encode_image(image)
        }
        COutputFormat::Png => image.save(output_path),
        COutputFormat::Webp => {
            use image::codecs::webp::WebPEncoder;
            let file =
                std::fs::File::create(output_path).map_err(|e| image::ImageError::IoError(e))?;
            let rgba = image.to_rgba8();
            WebPEncoder::new_lossless(file).encode(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
        }
    }
}

/// Collect face rectangles from a raw CFaceRect pointer into a Vec of tuples.
#[allow(dead_code)]
unsafe fn collect_face_areas(
    face_rects: *const CFaceRect,
    face_count: usize,
) -> Vec<(i32, i32, u32, u32)> {
    let mut face_areas = Vec::with_capacity(face_count);
    if !face_rects.is_null() && face_count > 0 {
        for i in 0..face_count {
            let face_rect = unsafe { *face_rects.add(i) };
            face_areas.push((face_rect.x, face_rect.y, face_rect.width, face_rect.height));
        }
    }
    face_areas
}

/// Read EXIF orientation from an image file and return the Orientation value.
fn read_exif_orientation(image_path: &str) -> image::metadata::Orientation {
    use exif::{In, Tag};
    let file = match std::fs::File::open(image_path) {
        Ok(f) => f,
        Err(_) => return image::metadata::Orientation::NoTransforms,
    };
    let mut buf_reader = std::io::BufReader::new(file);
    match exif::Reader::new().read_from_container(&mut buf_reader) {
        Ok(exif) => {
            let value = exif
                .get_field(Tag::Orientation, In::PRIMARY)
                .and_then(|field| field.value.get_uint(0));
            image::metadata::Orientation::from_exif(value.unwrap_or(0) as u8)
                .unwrap_or(image::metadata::Orientation::NoTransforms)
        }
        Err(_) => image::metadata::Orientation::NoTransforms,
    }
}

/// Extract a human-readable message from a panic payload.
fn extract_panic_message(panic_info: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic_info.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic_info.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

// ============================================================================
// Standalone EXIF Injection
// ============================================================================

/// Inject EXIF metadata from original image into an already-exported output file.
///
/// Reads EXIF from `original_path`, filters out MakerNote and GPS,
/// applies user overrides from `exif_override_json`, and injects into
/// the output file (JPEG or WebP).
///
/// # Safety
/// - All C string pointers must be valid null-terminated strings or null
#[unsafe(no_mangle)]
#[cfg(any(target_os = "ios", target_os = "android"))]
pub unsafe extern "C" fn chama_inject_exif(
    original_path: *const std::os::raw::c_char,
    output_path: *const std::os::raw::c_char,
    exif_override_json: *const std::os::raw::c_char,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
) -> ChamaError {
    use std::ffi::CStr;

    if original_path.is_null() || output_path.is_null() {
        return ChamaError::InvalidPath;
    }

    let original_path_str = cstr_to_str!(original_path, return ChamaError::InvalidPath);
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);
    let override_json = if exif_override_json.is_null() {
        None
    } else {
        let s = cstr_to_str_or!(exif_override_json, "");
        if !s.is_empty() { Some(s) } else { None }
    };

    match crate::image::exif_inject::inject_exif_to_output(
        original_path_str,
        output_path_str,
        override_json,
        get_alt_fnumber,
        use_35mm_focal_length,
    ) {
        Ok(_) => ChamaError::Success,
        Err(e) => {
            log::error!("EXIF injection failed: {}", e);
            ChamaError::ExifError
        }
    }
}

// ============================================================================
// Image Loading with HEIF Support
// ============================================================================

/// Load image with Apple native HEIF support on iOS/macOS
/// Falls back to standard image::open for non-HEIF formats
fn load_image_with_heif_support(path: &Path) -> Result<image::DynamicImage, image::ImageError> {
    #[cfg(any(target_os = "ios", target_os = "macos"))]
    {
        // Check if file is HEIF format
        if crate::ffi_apple_heif::is_heif_format(path) {
            log::info!("Detected HEIF format, using Apple native decoder");
            return crate::ffi_apple_heif::decode_heif(path).map_err(|e| {
                log::error!("Apple HEIF decoder failed: {}", e);
                image::ImageError::Decoding(image::error::DecodingError::new(
                    image::error::ImageFormatHint::Unknown,
                    e,
                ))
            });
        }
    }

    // For non-HEIF or non-Apple platforms, use standard decoder
    image::open(path)
}

// ============================================================================
// Preview Extraction
// ============================================================================

/// Extract EXIF preview from image
/// Returns the preview as a DynamicImage, or None if no suitable preview exists
fn extract_exif_preview(image_path: &str) -> Option<image::DynamicImage> {
    let file = std::fs::File::open(image_path).ok()?;
    let mut buf_reader = std::io::BufReader::new(file);

    let exif = exif::Reader::new()
        .read_from_container(&mut buf_reader)
        .ok()?;

    // Store thumbnails to extend lifetime
    let thumbnails = exif.thumbnails();

    // Find the largest thumbnail
    let biggest_thumbnail = thumbnails.iter().max_by_key(|t| t.length)?;

    // Only use thumbnails larger than 100KB (avoid tiny previews)
    if biggest_thumbnail.length < 100 * 1024 {
        log::info!(
            "EXIF thumbnail too small: {} bytes",
            biggest_thumbnail.length
        );
        return None;
    }

    log::info!("Found EXIF thumbnail: {} bytes", biggest_thumbnail.length);

    // Reopen file to extract thumbnail data
    let file = std::fs::File::open(image_path).ok()?;
    let mut buf_reader = std::io::BufReader::new(file);

    let thumbnail_data = biggest_thumbnail.extract_data(&mut buf_reader).ok()?;

    // Load the thumbnail as an image
    image::load_from_memory(&thumbnail_data).ok()
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! FFI entry points for Pipeline V1 execution.
//!
//! All pipeline configuration is passed as JSON strings (PipelineConfig is serde-enabled).
//! This avoids complex C struct bridging — the native side serializes to JSON,
//! Rust deserializes and executes.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use super::types::ChamaError;

// ============================================================================
// Pipeline V1 FFI Entry Points
// ============================================================================

/// Execute a pipeline V1 export from JSON configuration.
///
/// # Parameters
/// - `image_path`: Path to the input image file.
/// - `output_path`: Path where the processed image will be saved.
/// - `pipeline_config_json`: JSON string of `PipelineConfig` (stages + decoration).
/// - `exif_json`: Optional JSON string of `SimplifiedExif` for theme text overlays.
///   Pass NULL if no EXIF data is needed.
/// - `lut_paths_json`: Optional JSON object mapping UUID strings to LUT file paths.
///   Example: `{"550e8400-...": "/path/to/lut.cube"}`. Pass NULL if no LUT stages.
/// - `output_format`: 0=JPEG, 1=PNG, 2=WebP
/// - `quality`: JPEG/WebP quality (1-100)
///
/// # Returns
/// `ChamaError::Success` (0) on success, or an error code on failure.
///
/// # JSON Format
/// ```json
/// {
///   "stages": [
///     { "enabled": true, "stage": { "type": "ColorAdjustments", "enabled": true, "exposure": 0.5 } },
///     { "enabled": true, "stage": { "type": "Lut", "lut_id": "550e8400-..." } }
///   ],
///   "decoration": {
///     "enabled": true,
///     "decoration": { "type": "Theme", "name": "film" }
///   }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_pipeline_execute(
    image_path: *const c_char,
    output_path: *const c_char,
    pipeline_config_json: *const c_char,
    exif_json: *const c_char,
    lut_paths_json: *const c_char,
    output_format: u32,
    quality: u8,
) -> ChamaError {
    let image_path = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path = cstr_to_str!(output_path, return ChamaError::InvalidPath);
    let config_json = cstr_to_str!(pipeline_config_json, return ChamaError::InvalidParameters);

    // Parse pipeline config
    let config: crate::pipeline::v1::PipelineConfig = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Pipeline config parse error: {}", e);
            return ChamaError::InvalidParameters;
        }
    };

    // Parse optional EXIF
    let exif: Option<crate::image::exif_impl::SimplifiedExif> = if exif_json.is_null() {
        None
    } else {
        let exif_str = cstr_to_str!(exif_json, return ChamaError::InvalidParameters);
        if exif_str.is_empty() {
            None
        } else {
            match serde_json::from_str(exif_str) {
                Ok(e) => Some(e),
                Err(e) => {
                    log::warn!("EXIF parse warning (continuing without EXIF): {}", e);
                    None
                }
            }
        }
    };

    // Parse optional LUT paths and load LUT data
    let mut lut_map: HashMap<uuid::Uuid, wagahai_lut::CubeLut> = HashMap::new();
    if !lut_paths_json.is_null() {
        let lut_str = cstr_to_str!(lut_paths_json, return ChamaError::InvalidParameters);
        if !lut_str.is_empty() {
            let paths: HashMap<String, String> = match serde_json::from_str(lut_str) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("LUT paths parse error: {}", e);
                    return ChamaError::InvalidParameters;
                }
            };
            for (uuid_str, path) in &paths {
                let uuid = match uuid::Uuid::parse_str(uuid_str) {
                    Ok(u) => u,
                    Err(e) => {
                        log::error!("LUT UUID parse error for '{}': {}", uuid_str, e);
                        continue;
                    }
                };
                match wagahai_lut::CubeParser::from_file(path) {
                    Ok(lut) => {
                        lut_map.insert(uuid, lut);
                    }
                    Err(e) => {
                        log::error!("LUT load error for '{}': {:?}", path, e);
                        return ChamaError::ImageProcessError;
                    }
                }
            }
        }
    }

    // Load image
    let image = match image::open(image_path) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Image load error: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

    // Build context
    let theme_registry = crate::theme::ThemeRegistry::new();
    let export_config = crate::export_config::ExportConfig::default();
    let sticker_storage = crate::effect::sticker_storage::StickerStorage::default();

    let ctx = crate::pipeline::v1::PipelineContext {
        sticker_storage: Some(&sticker_storage),
        lut_map: if lut_map.is_empty() {
            None
        } else {
            Some(&lut_map)
        },
        font_map: None, // TODO: font loading from paths
        theme_registry: Some(&theme_registry),
        export_config: Some(&export_config),
        exif: exif.as_ref(),
    };

    // Execute pipeline
    let pipeline = crate::pipeline::v1::ExportPipeline::new(image, config);
    let result = match pipeline.execute(&ctx) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Pipeline execution error: {}", e);
            return ChamaError::ImageProcessError;
        }
    };

    // Save output
    let output_path = std::path::Path::new(output_path);
    let save_result = match output_format {
        1 => result.save_with_format(output_path, image::ImageFormat::Png),
        2 => {
            // WebP (lossless)
            let file = match std::fs::File::create(output_path) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Output file create error: {}", e);
                    return ChamaError::InvalidPath;
                }
            };
            let rgba = result.to_rgba8();
            image::codecs::webp::WebPEncoder::new_lossless(file).encode(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
        }
        _ => {
            // JPEG
            let file = match std::fs::File::create(output_path) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Output file create error: {}", e);
                    return ChamaError::InvalidPath;
                }
            };
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, quality);
            result.write_with_encoder(encoder)
        }
    };

    match save_result {
        Ok(()) => ChamaError::Success,
        Err(e) => {
            log::error!("Image save error: {}", e);
            ChamaError::ImageProcessError
        }
    }
}

/// Validate a pipeline configuration JSON without executing it.
///
/// # Parameters
/// - `pipeline_config_json`: JSON string of `PipelineConfig`.
///
/// # Returns
/// - On success: NULL pointer (config is valid).
/// - On error: A C string with the error message (caller must free with `chama_free_string`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_pipeline_validate(
    pipeline_config_json: *const c_char,
) -> *mut c_char {
    let config_json = cstr_to_str!(pipeline_config_json, return CString::new("Invalid UTF-8 in config JSON").unwrap().into_raw());

    let config: crate::pipeline::v1::PipelineConfig = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("JSON parse error: {}", e);
            return CString::new(msg).unwrap().into_raw();
        }
    };

    match config.validate() {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => {
            let msg = format!("{}", e);
            CString::new(msg).unwrap().into_raw()
        }
    }
}

/// Get the default pipeline configuration as a JSON string.
///
/// Returns a JSON representation of `PipelineConfig::default()`.
/// Caller must free the returned string with `chama_free_string()`.
#[unsafe(no_mangle)]
pub extern "C" fn chama_pipeline_default_config() -> *mut c_char {
    let config = crate::pipeline::v1::PipelineConfig::default();
    let json = serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string());
    CString::new(json).unwrap().into_raw()
}

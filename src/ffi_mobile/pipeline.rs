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

/// Execute the combined export pipeline using Pipeline V1 engine.
///
/// This is the pipeline-based replacement for `chama_export_combined`.
/// It accepts the same C struct parameters but routes execution through
/// the Pipeline V1 engine (`ExportPipeline`) instead of ad-hoc procedural code.
///
/// # Pipeline
/// 1. Load image with EXIF orientation
/// 2. Extract EXIF metadata (for theme text overlays)
/// 3. Convert C structs → PipelineConfig (via bridge)
/// 4. Execute pipeline: face effects → theme decoration
/// 5. Save with format/quality from config
/// 6. Inject EXIF metadata (if save_exif enabled)
///
/// # Safety
/// - All C string pointers must be valid null-terminated strings or NULL
/// - face_rects must point to a valid array of CFaceRect with face_count elements
/// - config must point to a valid CombinedExportConfig
#[unsafe(no_mangle)]
#[cfg(any(target_os = "ios", target_os = "android"))]
pub unsafe extern "C" fn chama_pipeline_export_combined(
    image_path: *const c_char,
    output_path: *const c_char,
    exif_source_path: *const c_char,
    face_rects: *const super::types::CFaceRect,
    face_count: usize,
    config: *const super::types::CombinedExportConfig,
) -> ChamaError {
    use crate::pipeline::v1;

    if image_path.is_null() || output_path.is_null() || config.is_null() {
        return ChamaError::InvalidPath;
    }

    let image_path_str = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);
    let exif_source_str = if exif_source_path.is_null() {
        image_path_str
    } else {
        cstr_to_str!(exif_source_path, return ChamaError::InvalidPath)
    };

    let config_ref = unsafe { &*config };

    log::info!("Pipeline combined export started:");
    log::info!("  Input: {}", image_path_str);
    log::info!("  Output: {}", output_path_str);
    log::info!("  Face count: {}", face_count);

    // Step 1: Load image and apply EXIF orientation
    let mut dyn_image = match super::load_image_with_heif_support(std::path::Path::new(image_path_str)) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

    let orientation = super::read_exif_orientation(image_path_str);
    dyn_image.apply_orientation(orientation);
    log::info!(
        "  Image size after orientation: {}x{}",
        dyn_image.width(),
        dyn_image.height()
    );

    // Ensure RGBA8 for face effects compatibility
    if dyn_image.as_rgba8().is_none() {
        dyn_image = image::DynamicImage::ImageRgba8(dyn_image.to_rgba8());
    }

    // Step 2: Extract EXIF for theme text overlays
    let mut exif = v1::extract_simplified_exif(
        exif_source_str,
        config_ref.get_alt_fnumber,
        config_ref.use_35mm_focal_length,
    );

    // Apply EXIF overrides if provided
    if !config_ref.exif_override_json.is_null() {
        let override_str = cstr_to_str_or!(config_ref.exif_override_json, "");
        if !override_str.is_empty() {
            v1::apply_exif_overrides(&mut exif, override_str);
        }
    }

    // If image differs from EXIF source, orientation was already applied
    if image_path_str != exif_source_str {
        exif.orientation = image::metadata::Orientation::NoTransforms;
    }

    // Step 3: Collect face rectangles
    let face_areas = unsafe { super::collect_face_areas(face_rects, face_count) };

    // Step 4: Convert C struct params → bridge params → PipelineConfig
    let theme_name_str = cstr_to_str_or!(config_ref.theme_name, "");
    let theme_params_str = cstr_to_str_or!(config_ref.theme_params_json, "{}");

    let bridge_face_type = match config_ref.face_effect.effect_type {
        super::types::CFaceEffectType::None => v1::BridgeFaceEffectType::None,
        super::types::CFaceEffectType::Mosaic => v1::BridgeFaceEffectType::Mosaic,
        super::types::CFaceEffectType::Stroke => v1::BridgeFaceEffectType::Stroke,
        super::types::CFaceEffectType::Sticker => v1::BridgeFaceEffectType::Sticker,
        super::types::CFaceEffectType::MosaicStroke => v1::BridgeFaceEffectType::MosaicStroke,
    };

    let scale_mode = match config_ref.scale_config.mode {
        super::types::CScaleMode::None => 0u8,
        super::types::CScaleMode::MaxWidth => 1,
        super::types::CScaleMode::MaxHeight => 2,
        super::types::CScaleMode::Longside => 3,
        super::types::CScaleMode::Divide => 4,
        super::types::CScaleMode::NearCommonWidth => 5,
        super::types::CScaleMode::NearCommonHeight => 6,
        super::types::CScaleMode::ResizeAndCrop => 7,
    };

    let output_format_num = match config_ref.output_format {
        super::types::COutputFormat::Jpeg => 0u32,
        super::types::COutputFormat::Png => 1,
        super::types::COutputFormat::Webp => 2,
    };

    let bridge_params = v1::BridgeExportParams {
        face_rects: &face_areas,
        face_effect: v1::BridgeFaceEffectParams {
            effect_type: bridge_face_type,
            mosaic_block_size: config_ref.face_effect.mosaic_block_size,
            mosaic_intensity: config_ref.face_effect.mosaic_intensity,
            stroke_color: [
                config_ref.face_effect.stroke_color.r,
                config_ref.face_effect.stroke_color.g,
                config_ref.face_effect.stroke_color.b,
                config_ref.face_effect.stroke_color.a,
            ],
            stroke_thickness: config_ref.face_effect.stroke_thickness,
            sticker_scale: config_ref.face_effect.sticker_scale,
            sticker_offset_x: config_ref.face_effect.sticker_offset_x,
            sticker_offset_y: config_ref.face_effect.sticker_offset_y,
        },
        theme_name: theme_name_str,
        theme_params_json: theme_params_str,
        scale_mode,
        scale_value: config_ref.scale_config.value,
        scale_sub_value: config_ref.scale_config.sub_value,
        scale_divisor: config_ref.scale_config.scale_value,
        output_format: output_format_num,
        quality: config_ref.quality,
    };

    let pipeline_config = v1::build_pipeline_config(&bridge_params);
    log::info!(
        "  Pipeline config: {} stages, decoration={}",
        pipeline_config.stages.len(),
        pipeline_config.decoration.is_some()
    );

    // Step 5: Build context and execute pipeline
    let theme_registry = crate::theme::ThemeRegistry::new();
    let sticker_storage = crate::effect::sticker_storage::StickerStorage::default();

    // Build export config from pipeline's scale/output_format for theme rendering
    let export_config = crate::export_config::ExportConfig {
        scale_config: pipeline_config.scale,
        output_format: pipeline_config.output_format,
        ..crate::export_config::ExportConfig::default()
    };

    let ctx = v1::PipelineContext {
        sticker_storage: Some(&sticker_storage),
        lut_map: None,
        font_map: None,
        theme_registry: Some(&theme_registry),
        export_config: Some(&export_config),
        exif: Some(&exif),
    };

    let pipeline = v1::ExportPipeline::new(dyn_image, pipeline_config);
    let result = match pipeline.execute(&ctx) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Pipeline execution error: {}", e);
            return ChamaError::ImageProcessError;
        }
    };

    // Step 6: Save output using the configured format
    match super::save_image_with_c_format(
        &result,
        output_path_str,
        config_ref.output_format,
        config_ref.quality,
    ) {
        Ok(_) => {
            // Step 7: Inject EXIF if enabled
            if config_ref.save_exif && config_ref.output_format != super::types::COutputFormat::Png {
                let exif_override_str = if config_ref.exif_override_json.is_null() {
                    None
                } else {
                    let s = cstr_to_str_or!(config_ref.exif_override_json, "");
                    if !s.is_empty() { Some(s) } else { None }
                };
                if let Err(e) = crate::image::exif_inject::inject_exif_to_output(
                    exif_source_str,
                    output_path_str,
                    exif_override_str,
                    config_ref.get_alt_fnumber,
                    config_ref.use_35mm_focal_length,
                ) {
                    log::warn!("EXIF injection failed (non-fatal): {}", e);
                }
            }
            log::info!("Pipeline combined export completed successfully");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to save image: {}", e);
            ChamaError::ImageProcessError
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

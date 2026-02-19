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

    // Load image (with HEIF support on Apple platforms)
    let image = match super::load_image_with_heif_support(std::path::Path::new(image_path)) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Image load error: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

    // Build context with config's scale/output_format for theme rendering
    let theme_registry = crate::theme::ThemeRegistry::new();
    let sticker_storage = crate::effect::sticker_storage::StickerStorage::default();

    // Use output_format/quality params as override if provided, otherwise use config values
    let save_format = crate::pipeline::v1::build_output_format(output_format, quality);
    let export_config = crate::export_config::ExportConfig {
        scale_config: config.scale,
        output_format: save_format,
        ..crate::export_config::ExportConfig::default()
    };

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
    let scale_config = config.scale;
    let pipeline = crate::pipeline::v1::ExportPipeline::new(image, config);
    let result = match pipeline.execute(&ctx) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Pipeline execution error: {}", e);
            return ChamaError::ImageProcessError;
        }
    };

    // Apply scaling from config
    let result = match crate::pipeline::v1::apply_scale(result, &scale_config) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Pipeline scale error: {}", e);
            return ChamaError::ImageProcessError;
        }
    };

    // Save output using OutputFormat
    let output_path = std::path::Path::new(output_path);
    match save_format.save_image(&result, output_path) {
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
    let config_json = cstr_to_str!(
        pipeline_config_json,
        return CString::new("Invalid UTF-8 in config JSON")
            .unwrap()
            .into_raw()
    );

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
    let mut dyn_image =
        match super::load_image_with_heif_support(std::path::Path::new(image_path_str)) {
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

    // Load font if path is provided (for watermark rendering)
    let font_path_str = cstr_to_str_or!(config_ref.font_path, "");
    let mut font_map: HashMap<String, ab_glyph::FontArc> = HashMap::new();
    if !font_path_str.is_empty() {
        match std::fs::read(font_path_str) {
            Ok(font_data) => match ab_glyph::FontArc::try_from_vec(font_data) {
                Ok(font) => {
                    font_map.insert("default".to_string(), font);
                    log::info!("  Loaded font: {}", font_path_str);
                }
                Err(e) => log::warn!("  Font parse error (non-fatal): {}", e),
            },
            Err(e) => log::warn!("  Font read error (non-fatal): {}", e),
        }
    }

    // Build export config from pipeline's scale/output_format for theme rendering
    let export_config = crate::export_config::ExportConfig {
        scale_config: pipeline_config.scale,
        output_format: pipeline_config.output_format,
        ..crate::export_config::ExportConfig::default()
    };

    let ctx = v1::PipelineContext {
        sticker_storage: Some(&sticker_storage),
        lut_map: None,
        font_map: if font_map.is_empty() {
            None
        } else {
            Some(&font_map)
        },
        theme_registry: Some(&theme_registry),
        export_config: Some(&export_config),
        exif: Some(&exif),
    };

    let scale_config = pipeline_config.scale;
    let pipeline = v1::ExportPipeline::new(dyn_image, pipeline_config);
    let result = match pipeline.execute(&ctx) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Pipeline execution error: {}", e);
            return ChamaError::ImageProcessError;
        }
    };

    // Step 6: Apply scaling from config
    let result = match v1::apply_scale(result, &scale_config) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Pipeline scale error: {}", e);
            return ChamaError::ImageProcessError;
        }
    };

    // Step 7: Save output using the configured format
    match super::save_image_with_c_format(
        &result,
        output_path_str,
        config_ref.output_format,
        config_ref.quality,
    ) {
        Ok(_) => {
            // Step 7: Inject EXIF if enabled
            if config_ref.save_exif && config_ref.output_format != super::types::COutputFormat::Png
            {
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

// ============================================================================
// Preview Pipeline FFI Entry Points
// ============================================================================

/// Opaque handle to a `PreviewPipeline` with owned context resources.
///
/// Created via `chama_preview_pipeline_create`, destroyed via `chama_preview_pipeline_destroy`.
/// The handle owns all resources (LUTs, fonts, sticker storage, theme registry)
/// so that the native caller only manages this single pointer.
pub struct PreviewPipelineHandle {
    pipeline: crate::pipeline::v1::PreviewPipeline,
    theme_registry: crate::theme::ThemeRegistry,
    sticker_storage: crate::effect::sticker_storage::StickerStorage,
    export_config: crate::export_config::ExportConfig,
    exif: Option<crate::image::exif_impl::SimplifiedExif>,
    lut_map: HashMap<uuid::Uuid, wagahai_lut::CubeLut>,
    font_map: HashMap<String, ab_glyph::FontArc>,
}

/// Helper: render preview through a handle, splitting borrows correctly.
///
/// Takes individual field references to avoid borrow conflicts between
/// `&mut pipeline` and `&context_fields`.
fn render_preview<'a>(
    pipeline: &'a mut crate::pipeline::v1::PreviewPipeline,
    theme_registry: &crate::theme::ThemeRegistry,
    sticker_storage: &crate::effect::sticker_storage::StickerStorage,
    export_config: &crate::export_config::ExportConfig,
    exif: Option<&crate::image::exif_impl::SimplifiedExif>,
    lut_map: &HashMap<uuid::Uuid, wagahai_lut::CubeLut>,
    font_map: &HashMap<String, ab_glyph::FontArc>,
    with_decoration: bool,
) -> Result<image::DynamicImage, crate::pipeline::v1::PipelineError> {
    let ctx = crate::pipeline::v1::PipelineContext {
        sticker_storage: Some(sticker_storage),
        lut_map: if lut_map.is_empty() {
            None
        } else {
            Some(lut_map)
        },
        font_map: if font_map.is_empty() {
            None
        } else {
            Some(font_map)
        },
        theme_registry: Some(theme_registry),
        export_config: Some(export_config),
        exif,
    };

    if with_decoration {
        pipeline.render_with_decoration(&ctx)
    } else {
        pipeline.render(&ctx).map(|img| img.clone())
    }
}

/// Create a new PreviewPipeline for interactive editing.
///
/// Loads the image, parses config/EXIF/LUTs, and returns an opaque handle.
/// The base image is used as-is (caller should provide a thumbnail for fast preview).
///
/// # Parameters
/// - `image_path`: Path to the preview image (thumbnail or EXIF preview).
/// - `pipeline_config_json`: JSON string of `PipelineConfig`.
/// - `exif_json`: Optional JSON string of `SimplifiedExif`. Pass NULL if not needed.
/// - `lut_paths_json`: Optional JSON object `{"uuid": "/path/to/lut.cube"}`. Pass NULL if none.
/// - `font_path`: Optional path to a font file for watermark rendering. Pass NULL if none.
///
/// # Returns
/// An opaque handle pointer, or NULL on failure.
/// Caller must free with `chama_preview_pipeline_destroy`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_preview_pipeline_create(
    image_path: *const c_char,
    pipeline_config_json: *const c_char,
    exif_json: *const c_char,
    lut_paths_json: *const c_char,
    font_path: *const c_char,
) -> *mut PreviewPipelineHandle {
    let image_path_str = cstr_to_str!(image_path, return std::ptr::null_mut());
    let config_json = cstr_to_str!(pipeline_config_json, return std::ptr::null_mut());

    // Parse pipeline config
    let config: crate::pipeline::v1::PipelineConfig = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Preview pipeline config parse error: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Load image (with HEIF support)
    let image = match super::load_image_with_heif_support(std::path::Path::new(image_path_str)) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Preview pipeline image load error: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Parse optional EXIF
    let exif: Option<crate::image::exif_impl::SimplifiedExif> = if exif_json.is_null() {
        None
    } else {
        let exif_str = cstr_to_str!(exif_json, return std::ptr::null_mut());
        if exif_str.is_empty() {
            None
        } else {
            serde_json::from_str(exif_str).ok()
        }
    };

    // Parse optional LUT paths
    let mut lut_map: HashMap<uuid::Uuid, wagahai_lut::CubeLut> = HashMap::new();
    if !lut_paths_json.is_null() {
        let lut_str = cstr_to_str!(lut_paths_json, return std::ptr::null_mut());
        if !lut_str.is_empty() {
            if let Ok(paths) = serde_json::from_str::<HashMap<String, String>>(lut_str) {
                for (uuid_str, path) in &paths {
                    if let Ok(uuid) = uuid::Uuid::parse_str(uuid_str) {
                        match wagahai_lut::CubeParser::from_file(path) {
                            Ok(lut) => {
                                lut_map.insert(uuid, lut);
                            }
                            Err(e) => {
                                log::error!("Preview LUT load error for '{}': {:?}", path, e);
                            }
                        }
                    }
                }
            }
        }
    }

    // Load font if path provided
    let mut font_map: HashMap<String, ab_glyph::FontArc> = HashMap::new();
    let font_path_str = cstr_to_str_or!(font_path, "");
    if !font_path_str.is_empty() {
        if let Ok(font_data) = std::fs::read(font_path_str) {
            if let Ok(font) = ab_glyph::FontArc::try_from_vec(font_data) {
                font_map.insert("default".to_string(), font);
            }
        }
    }

    // Build export config from pipeline config
    let export_config = crate::export_config::ExportConfig {
        scale_config: config.scale,
        output_format: config.output_format,
        ..crate::export_config::ExportConfig::default()
    };

    let pipeline = crate::pipeline::v1::PreviewPipeline::new(image, config);

    let handle = Box::new(PreviewPipelineHandle {
        pipeline,
        theme_registry: crate::theme::ThemeRegistry::new(),
        sticker_storage: crate::effect::sticker_storage::StickerStorage::default(),
        export_config,
        exif,
        lut_map,
        font_map,
    });

    log::info!("Preview pipeline created successfully");
    Box::into_raw(handle)
}

/// Render the preview pipeline and save result to a file.
///
/// Re-executes only dirty stages (incremental caching).
///
/// # Parameters
/// - `handle`: Opaque handle from `chama_preview_pipeline_create`.
/// - `output_path`: Path to save the rendered preview image.
/// - `output_format`: 0=JPEG, 1=PNG, 2=WebP.
/// - `quality`: Encoding quality (1-100).
/// - `with_decoration`: If true, apply decoration (Theme/Cheki) after stages.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_preview_pipeline_render(
    handle: *mut PreviewPipelineHandle,
    output_path: *const c_char,
    output_format: u32,
    quality: u8,
    with_decoration: bool,
) -> ChamaError {
    if handle.is_null() {
        return ChamaError::InvalidParameters;
    }
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);
    let handle = unsafe { &mut *handle };

    let result = match render_preview(
        &mut handle.pipeline,
        &handle.theme_registry,
        &handle.sticker_storage,
        &handle.export_config,
        handle.exif.as_ref(),
        &handle.lut_map,
        &handle.font_map,
        with_decoration,
    ) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Preview render error: {}", e);
            return ChamaError::ImageProcessError;
        }
    };

    let save_format = crate::pipeline::v1::build_output_format(output_format, quality);
    match save_format.save_image(&result, output_path_str) {
        Ok(()) => ChamaError::Success,
        Err(e) => {
            log::error!("Preview save error: {}", e);
            ChamaError::ImageProcessError
        }
    }
}

/// Render the preview pipeline and return encoded image bytes.
///
/// More efficient than `chama_preview_pipeline_render` for UI display since
/// it avoids writing to disk. The caller receives encoded bytes (JPEG/PNG)
/// that can be decoded directly into a UIImage (iOS) or Bitmap (Android).
///
/// # Parameters
/// - `handle`: Opaque handle from `chama_preview_pipeline_create`.
/// - `output_format`: 0=JPEG, 1=PNG, 2=WebP.
/// - `quality`: Encoding quality (1-100).
/// - `with_decoration`: If true, apply decoration after stages.
/// - `out_data`: Output pointer to the encoded byte buffer. Caller must free with
///   `chama_preview_pipeline_free_bytes`.
/// - `out_len`: Output length of the encoded byte buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_preview_pipeline_render_bytes(
    handle: *mut PreviewPipelineHandle,
    output_format: u32,
    quality: u8,
    with_decoration: bool,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> ChamaError {
    if handle.is_null() || out_data.is_null() || out_len.is_null() {
        return ChamaError::InvalidParameters;
    }
    let handle = unsafe { &mut *handle };

    let result = match render_preview(
        &mut handle.pipeline,
        &handle.theme_registry,
        &handle.sticker_storage,
        &handle.export_config,
        handle.exif.as_ref(),
        &handle.lut_map,
        &handle.font_map,
        with_decoration,
    ) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Preview render error: {}", e);
            return ChamaError::ImageProcessError;
        }
    };

    let format = crate::pipeline::v1::build_output_format(output_format, quality);
    let bytes = match format.encode_to_bytes(&result) {
        Ok(b) => b,
        Err(e) => {
            log::error!("Preview encode error: {}", e);
            return ChamaError::ImageProcessError;
        }
    };

    let len = bytes.len();
    let boxed = bytes.into_boxed_slice();
    let ptr = Box::into_raw(boxed) as *mut u8;

    unsafe {
        *out_data = ptr;
        *out_len = len;
    }

    ChamaError::Success
}

/// Free bytes returned by `chama_preview_pipeline_render_bytes`.
///
/// # Safety
/// - `data` must be a pointer returned by `chama_preview_pipeline_render_bytes`.
/// - `len` must be the exact length returned alongside the data pointer.
/// - Must not be called more than once for the same pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_preview_pipeline_free_bytes(data: *mut u8, len: usize) {
    if !data.is_null() && len > 0 {
        let _ = unsafe { Box::from_raw(std::slice::from_raw_parts_mut(data, len)) };
    }
}

/// Update a pipeline stage's configuration by kind.
///
/// Finds the stage matching the type in `stage_json` and replaces its config.
/// Only the affected stage and subsequent stages will be re-executed on next render.
///
/// # Parameters
/// - `handle`: Opaque handle from `chama_preview_pipeline_create`.
/// - `stage_json`: JSON string of the new `PipelineStage` (must include `"type"` field).
///   Example: `{"type": "ColorAdjustments", "enabled": true, "exposure": 0.5}`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_preview_pipeline_update_stage(
    handle: *mut PreviewPipelineHandle,
    stage_json: *const c_char,
) -> ChamaError {
    if handle.is_null() {
        return ChamaError::InvalidParameters;
    }
    let stage_str = cstr_to_str!(stage_json, return ChamaError::InvalidParameters);
    let handle = unsafe { &mut *handle };

    let new_stage: crate::pipeline::v1::PipelineStage = match serde_json::from_str(stage_str) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Preview stage parse error: {}", e);
            return ChamaError::InvalidParameters;
        }
    };

    let kind = new_stage.kind();
    if handle.pipeline.update_stage(kind, new_stage) {
        ChamaError::Success
    } else {
        log::warn!("Preview update_stage: no stage of kind {:?} found", kind);
        ChamaError::InvalidParameters
    }
}

/// Toggle a pipeline stage's enabled flag by kind.
///
/// # Parameters
/// - `handle`: Opaque handle from `chama_preview_pipeline_create`.
/// - `stage_kind`: Stage type to toggle:
///   0=CropRotate, 1=ColorAdjustments, 2=Lut, 3=FaceEffect, 4=Watermark.
/// - `enabled`: New enabled state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_preview_pipeline_toggle_stage(
    handle: *mut PreviewPipelineHandle,
    stage_kind: u32,
    enabled: bool,
) -> ChamaError {
    if handle.is_null() {
        return ChamaError::InvalidParameters;
    }
    let handle = unsafe { &mut *handle };

    let kind = match stage_kind {
        0 => crate::pipeline::v1::StageKind::CropRotate,
        1 => crate::pipeline::v1::StageKind::ColorAdjustments,
        2 => crate::pipeline::v1::StageKind::Lut,
        3 => crate::pipeline::v1::StageKind::FaceEffect,
        4 => crate::pipeline::v1::StageKind::Watermark,
        _ => {
            log::error!("Preview toggle_stage: invalid stage_kind {}", stage_kind);
            return ChamaError::InvalidParameters;
        }
    };

    if handle.pipeline.toggle_stage(kind, enabled) {
        ChamaError::Success
    } else {
        log::warn!("Preview toggle_stage: no stage of kind {:?} found", kind);
        ChamaError::InvalidParameters
    }
}

/// Replace the entire pipeline configuration.
///
/// This invalidates all cached snapshots. Use for reordering stages,
/// adding/removing stages, or changing decoration.
///
/// # Parameters
/// - `handle`: Opaque handle from `chama_preview_pipeline_create`.
/// - `pipeline_config_json`: New JSON string of `PipelineConfig`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_preview_pipeline_update_config(
    handle: *mut PreviewPipelineHandle,
    pipeline_config_json: *const c_char,
) -> ChamaError {
    if handle.is_null() {
        return ChamaError::InvalidParameters;
    }
    let config_json = cstr_to_str!(pipeline_config_json, return ChamaError::InvalidParameters);
    let handle = unsafe { &mut *handle };

    let config: crate::pipeline::v1::PipelineConfig = match serde_json::from_str(config_json) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Preview config parse error: {}", e);
            return ChamaError::InvalidParameters;
        }
    };

    // Update export config to match new pipeline config
    handle.export_config.scale_config = config.scale;
    handle.export_config.output_format = config.output_format;

    // Rebuild pipeline with new config, keeping the same base image
    // We need to get the current config to extract the base image...
    // PreviewPipeline doesn't expose base_image, so we recreate by
    // reordering stages (which invalidates all caches)
    let order: Vec<crate::pipeline::v1::StageKind> =
        config.stages.iter().map(|e| e.stage.kind()).collect();
    handle.pipeline.reorder_stages(&order);

    // Update individual stages with new config values
    for entry in &config.stages {
        handle
            .pipeline
            .update_stage(entry.stage.kind(), entry.stage.clone());
        if !entry.enabled {
            handle.pipeline.toggle_stage(entry.stage.kind(), false);
        }
    }

    ChamaError::Success
}

/// Get the current pipeline configuration as JSON.
///
/// # Parameters
/// - `handle`: Opaque handle from `chama_preview_pipeline_create`.
///
/// # Returns
/// JSON string of the current `PipelineConfig`.
/// Caller must free with `chama_free_string()`. Returns NULL on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_preview_pipeline_get_config(
    handle: *mut PreviewPipelineHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle = unsafe { &*handle };

    let json =
        serde_json::to_string_pretty(handle.pipeline.config()).unwrap_or_else(|_| "{}".to_string());
    CString::new(json)
        .unwrap_or_else(|_| CString::new("{}").unwrap())
        .into_raw()
}

/// Destroy a PreviewPipeline handle and free all owned resources.
///
/// # Safety
/// - `handle` must be a pointer returned by `chama_preview_pipeline_create`.
/// - Must not be called more than once for the same handle.
/// - Must not use the handle after calling this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_preview_pipeline_destroy(handle: *mut PreviewPipelineHandle) {
    if !handle.is_null() {
        let _ = unsafe { Box::from_raw(handle) };
        log::info!("Preview pipeline destroyed");
    }
}

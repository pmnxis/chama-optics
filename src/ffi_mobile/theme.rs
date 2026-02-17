/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Theme-related FFI functions for mobile platforms (iOS/Android)
//!
//! This module contains theme schema, preview generation, full-resolution export,
//! EXIF extraction, font validation, version info, memory management, and tests.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

use super::types::*;
use crate::error::ChamaOpticsError;

// ============================================================================
// Parameter Update Helper
// ============================================================================

/// Update theme parameters from JSON string
/// Uses macro-based downcasting to access ThemeParameters trait
fn update_theme_from_json(
    theme: &mut dyn crate::theme::Theme,
    params_json: &str,
) -> Result<(), ChamaOpticsError> {
    use crate::theme::parameter_schema::ThemeParameters;

    // Parse JSON
    let updates: serde_json::Map<String, serde_json::Value> = serde_json::from_str(params_json)
        .map_err(|e| ChamaOpticsError::InvalidParameters(format!("JSON parse error: {}", e)))?;

    if updates.is_empty() {
        log::debug!("No parameter updates provided");
        return Ok(());
    }

    // Try to downcast to each concrete theme type and update
    // This is necessary because Rust doesn't allow trait upcasting
    macro_rules! try_update {
        ($theme_type:ty) => {
            if let Some(concrete_theme) =
                (theme as &mut dyn std::any::Any).downcast_mut::<$theme_type>()
            {
                return concrete_theme
                    .update_from_json(&updates)
                    .map_err(|e| ChamaOpticsError::InvalidParameters(e));
            }
        };
    }

    try_update!(crate::theme::just_frame::JustFrame);
    try_update!(crate::theme::one_line::OneLine);
    try_update!(crate::theme::two_line::TwoLine);
    try_update!(crate::theme::shot_on_one_line::ShotOnOneLine);
    try_update!(crate::theme::shot_on_two_line::ShotOnTwoLine);
    try_update!(crate::theme::strap::Strap);
    try_update!(crate::theme::monitor::Monitor);
    try_update!(crate::theme::lightroom::Lightroom);
    try_update!(crate::theme::film::Film);
    try_update!(crate::theme::film_date::FilmDate);
    try_update!(crate::theme::film_glow::FilmGlow);

    log::warn!("Could not downcast theme to update parameters");
    Err(ChamaOpticsError::InvalidTheme)
}

// ============================================================================
// Implementation Functions
// ============================================================================

/// Internal implementation of preview generation
fn generate_preview_impl(
    image_path: &str,
    output_path: &str,
    theme_name: &str,
    params_json: &str,
    font_path: &str,
    font_weight: u32,
) -> Result<(), ChamaOpticsError> {
    // 1. Try to load EXIF preview first, fallback to full image
    let _dyn_image = if let Some(preview) = super::extract_exif_preview(image_path) {
        log::info!("✅ Loaded EXIF preview");
        preview
    } else {
        log::info!("⚠️ No EXIF preview, loading full image");
        super::load_image_with_heif_support(Path::new(image_path))
            .map_err(ChamaOpticsError::ImageLoad)?
    };

    // 2. Parse EXIF for metadata
    let exif = {
        let file = std::fs::File::open(image_path).map_err(ChamaOpticsError::Io)?;
        let mut buf_reader = std::io::BufReader::new(file);
        exif::Reader::new()
            .read_from_container(&mut buf_reader)
            .ok()
    };

    let original_exif = crate::image::exif_impl::OriginalExif::new(exif);
    let view_exif = crate::image::exif_impl::SimplifiedExif::from(&original_exif);

    // 3. Create theme instance
    let mut theme = crate::theme::create_theme(theme_name).ok_or(ChamaOpticsError::InvalidTheme)?;

    // 4. Update theme parameters from JSON
    update_theme_from_json(&mut *theme, params_json)?;

    // 5. Validate font path and weight
    // Font path and weight will be used by theme rendering
    // Note: font_weight (100-900) will be handled by Swift's variable font system
    if !Path::new(font_path).exists() {
        log::warn!("Font path does not exist: {}", font_path);
        return Err(ChamaOpticsError::InvalidFont);
    }
    log::info!(
        "Font validation passed: {} (weight: {})",
        font_path,
        font_weight
    );

    // 6. Load image bytes for non-desktop platforms (iOS sandbox)
    use crate::image::common::PackedTexture;
    use crate::image::packed_image::PackedImage;
    use uuid::Uuid;

    let packed_image = PackedImage {
        uuid: Uuid::new_v4(),
        path: Path::new(image_path).to_path_buf(),
        src_exif: original_exif,
        view_exif,
        editable: false,
        texture: PackedTexture::Dummy,
        #[cfg(not(feature = "desktop"))]
        image_bytes: std::fs::read(image_path).ok(),
        sticker_bytes: None,
        perceptual_hash: None,
        // For now normal preview doesn't draw face effect.
        configured_faces: Vec::with_capacity(0),
        lut_id: None, // iOS FFI doesn't use LUT yet
        crop_rotate: crate::effect::crop_rotate::CropRotateTransform::default(),
        #[cfg(feature = "rfd")]
        pending_save: None,
    };

    // 7. Apply theme
    let export_config = crate::export_config::ExportConfig::default();
    let mut themed_image = theme
        .apply_to_image(&packed_image, &export_config)
        .map_err(ChamaOpticsError::ImageProcess)?;

    // 8. Save output
    export_config
        .save_image(&mut themed_image, None, Path::new(output_path))
        .map_err(ChamaOpticsError::ImageProcess)?;

    Ok(())
}

/// Unified internal implementation of theme export
pub(super) fn export_final_impl(params: &ThemeExportParams) -> Result<(), ChamaOpticsError> {
    // Verify image file exists and is readable before proceeding
    // Note: We intentionally do NOT load the full image here, as it will be loaded
    // again from image_bytes in PackedImage::get_image(). Double-loading wastes ~100MB.
    {
        let img_path = Path::new(params.image_path);
        if !img_path.exists() {
            log::error!("Image file does not exist: {}", params.image_path);
            return Err(ChamaOpticsError::ImageLoad(image::ImageError::IoError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "Image file not found"),
            )));
        }
        let metadata = std::fs::metadata(img_path).map_err(ChamaOpticsError::Io)?;
        log::info!(
            "✅ Image file verified: {} ({} bytes)",
            params.image_path,
            metadata.len()
        );
    }

    // Parse EXIF from the original source (not the modified image)
    let exif = {
        let file = std::fs::File::open(params.exif_source_path).map_err(ChamaOpticsError::Io)?;
        let mut buf_reader = std::io::BufReader::new(file);
        exif::Reader::new()
            .read_from_container(&mut buf_reader)
            .ok()
    };

    if exif.is_some() {
        log::info!("✅ Loaded EXIF from: {}", params.exif_source_path);
    } else {
        log::warn!("⚠️ No EXIF data found in: {}", params.exif_source_path);
    }

    let original_exif = crate::image::exif_impl::OriginalExif::new(exif);
    let mut view_exif = crate::image::exif_impl::SimplifiedExif::from(&original_exif);

    // Apply import configuration settings
    if params.get_alt_fnumber {
        view_exif.replace_with_fnumber_alt_when_invalid();
    }
    if params.use_35mm_focal_length {
        view_exif.use_35mm_focal_length(&original_exif);
    }

    // Apply EXIF overrides from user edits (if provided)
    if let Some(override_json) = params.exif_override_json {
        if !override_json.is_empty() && override_json != "{}" {
            match serde_json::from_str::<crate::image::exif_impl::SimplifiedExif>(override_json) {
                Ok(override_exif) => {
                    if !override_exif.camera_mnf.is_empty() {
                        view_exif.camera_mnf = override_exif.camera_mnf;
                    }
                    if !override_exif.camera_model.is_empty() {
                        view_exif.camera_model = override_exif.camera_model;
                    }
                    if !override_exif.lens_mnf.is_empty() {
                        view_exif.lens_mnf = override_exif.lens_mnf;
                    }
                    if !override_exif.lens_model.is_empty() {
                        view_exif.lens_model = override_exif.lens_model;
                    }
                    if !override_exif.focal.is_empty() {
                        view_exif.focal = override_exif.focal;
                    }
                    if !override_exif.fnumber.is_empty() {
                        view_exif.fnumber = override_exif.fnumber;
                    }
                    if !override_exif.exposure.is_empty() {
                        view_exif.exposure = override_exif.exposure;
                    }
                    if override_exif.iso_speed.is_some() {
                        view_exif.iso_speed = override_exif.iso_speed;
                    }
                    if override_exif.datetime.is_some() {
                        view_exif.datetime = override_exif.datetime;
                    }
                    log::info!("✅ Applied EXIF overrides from user edits");
                }
                Err(e) => {
                    log::warn!("⚠️ Failed to parse EXIF override JSON: {}", e);
                }
            }
        }
    }

    // If image_path differs from exif_source_path, the image has already been processed
    // (e.g., face effects applied via load_image_direct which applies orientation).
    // In this case, we should NOT apply orientation again to avoid double rotation.
    if params.image_path != params.exif_source_path {
        log::info!(
            "  Image path differs from EXIF source - skipping orientation (already applied)"
        );
        view_exif.orientation = image::metadata::Orientation::NoTransforms;
    }

    // 3. Create theme instance
    let mut theme =
        crate::theme::create_theme(params.theme_name).ok_or(ChamaOpticsError::InvalidTheme)?;

    // 4. Update theme parameters from JSON
    update_theme_from_json(&mut *theme, params.params_json)?;

    // 5. Validate font path and weight
    // Font path and weight will be used by theme rendering
    // Note: font_weight (100-900) will be handled by Swift's variable font system
    if !Path::new(params.font_path).exists() {
        log::warn!("Font path does not exist: {}", params.font_path);
        return Err(ChamaOpticsError::InvalidFont);
    }
    log::info!(
        "Font validation passed: {} (weight: {})",
        params.font_path,
        params.font_weight
    );

    // 6. Create a PackedImage for theme application
    use crate::image::common::PackedTexture;
    use crate::image::packed_image::PackedImage;
    use uuid::Uuid;

    // Read the image bytes from the input path (which may be a temp file with face effects)
    #[cfg(not(feature = "desktop"))]
    let image_bytes = match &std::fs::read(params.image_path) {
        Ok(bytes) => {
            log::info!(
                "✅ Read {} bytes from input image: {}",
                bytes.len(),
                params.image_path
            );
            Some(bytes.clone())
        }
        Err(e) => {
            log::error!("❌ Failed to read input image bytes: {}", e);
            None
        }
    };

    let packed_image = PackedImage {
        uuid: Uuid::new_v4(),
        path: Path::new(params.image_path).to_path_buf(),
        src_exif: original_exif,
        view_exif,
        editable: false,
        texture: PackedTexture::Dummy,
        #[cfg(not(feature = "desktop"))]
        image_bytes,
        sticker_bytes: None,
        perceptual_hash: None,
        configured_faces: Vec::with_capacity(0), // todo - check is this right?
        lut_id: None,                            // iOS FFI doesn't use LUT yet
        crop_rotate: crate::effect::crop_rotate::CropRotateTransform::default(),
        #[cfg(feature = "rfd")]
        pending_save: None,
    };

    // 7. Apply theme with custom scale config if provided
    let mut export_config = crate::export_config::ExportConfig::default();

    // Apply custom scale config if provided
    if let Some(custom_scale) = params.scale_config.as_ref() {
        // Convert core ScaleConfig to export_config ScaleConfig
        export_config.scale_config = crate::export_config::scale_config::ScaleConfig {
            mode: match custom_scale.mode {
                crate::scale_config::ScaleMode::None => {
                    crate::export_config::scale_config::ScaleMode::None
                }
                crate::scale_config::ScaleMode::MaxWidth => {
                    crate::export_config::scale_config::ScaleMode::MaxWidth
                }
                crate::scale_config::ScaleMode::MaxHeight => {
                    crate::export_config::scale_config::ScaleMode::MaxHeight
                }
                crate::scale_config::ScaleMode::Longside => {
                    crate::export_config::scale_config::ScaleMode::Longside
                }
                crate::scale_config::ScaleMode::Divide => {
                    crate::export_config::scale_config::ScaleMode::Divide
                }
                crate::scale_config::ScaleMode::NearCommonDivisorConsiderWidth => {
                    crate::export_config::scale_config::ScaleMode::NearCommonDivisorConsiderWidth
                }
                crate::scale_config::ScaleMode::NearCommonDivisorConsiderHeight => {
                    crate::export_config::scale_config::ScaleMode::NearCommonDivisorConsiderHeight
                }
                crate::scale_config::ScaleMode::ResizeAndCrop => {
                    crate::export_config::scale_config::ScaleMode::ResizeAndCrop
                }
            },
            value: custom_scale.value,
            sub_value: custom_scale.sub_value,
            scale_value: custom_scale.scale_value,
        };
        log::info!(
            "  Using custom scale config: mode={:?}, value={}",
            custom_scale.mode,
            custom_scale.value
        );
    }

    // Apply custom export config (output format and quality) if provided
    if let Some(output_config) = params.output_format_config.as_ref() {
        // Set output format
        export_config.output_format = crate::export_config::output_format::OutputFormat {
            ext: super::convert_c_output_format(output_config.output_format),
            quality: output_config.quality,
        };
        log::info!(
            "  Using custom export config: format={:?}, quality={}",
            output_config.output_format,
            output_config.quality
        );
    }
    let mut themed_image = theme
        .apply_to_image(&packed_image, &export_config)
        .map_err(ChamaOpticsError::ImageProcess)?;

    // 8. Save output
    export_config
        .save_image(&mut themed_image, None, Path::new(params.output_path))
        .map_err(ChamaOpticsError::ImageProcess)?;

    Ok(())
}

/// Get theme schema by name
/// This function creates a theme instance and calls schema() on it
fn get_theme_schema_impl(theme_name: &str) -> Option<crate::theme::parameter_schema::ThemeSchema> {
    use crate::theme::parameter_schema::ThemeParameters;

    match theme_name {
        "just_frame" => Some(crate::theme::just_frame::JustFrame::default().schema()),
        "one_line" => Some(crate::theme::one_line::OneLine::default().schema()),
        "two_line" => Some(crate::theme::two_line::TwoLine::default().schema()),
        "shot_on_one_line" => {
            Some(crate::theme::shot_on_one_line::ShotOnOneLine::default().schema())
        }
        "shot_on_two_line" => {
            Some(crate::theme::shot_on_two_line::ShotOnTwoLine::default().schema())
        }
        "strap" => Some(crate::theme::strap::Strap::default().schema()),
        "monitor" => Some(crate::theme::monitor::Monitor::default().schema()),
        "lightroom" => Some(crate::theme::lightroom::Lightroom::default().schema()),
        "film" => Some(crate::theme::film::Film::default().schema()),
        "film_date" => Some(crate::theme::film_date::FilmDate::default().schema()),
        "film_glow" => Some(crate::theme::film_glow::FilmGlow::default().schema()),
        _ => None,
    }
}

/// Extract EXIF data as JSON string
fn extract_exif_json_impl(image_path: &str) -> Result<String, ChamaOpticsError> {
    use crate::image::exif_impl::OriginalExif;

    // Open file and parse EXIF
    let file = std::fs::File::open(image_path).map_err(ChamaOpticsError::Io)?;
    let mut buf_reader = std::io::BufReader::new(file);

    let exif = exif::Reader::new()
        .read_from_container(&mut buf_reader)
        .map_err(|_| ChamaOpticsError::ExifError)?;

    let original_exif = OriginalExif::new_with_exif(exif);

    // Build JSON with all EXIF fields used in templates
    let exif_data = serde_json::json!({
        "camera_mnf": original_exif.camera_mnf(),
        "camera_model": original_exif.camera_model(),
        "lens_mnf": original_exif.lens_mnf(),
        "lens_maker": original_exif.lens_maker(),
        "lens_model": original_exif.lens_model(),
        "focal": original_exif.focal(),
        "fnumber": original_exif.fnumber(),
        "exposure": original_exif.exposure(),
        "iso_speed": original_exif.iso_speed().map(|v| v.to_string()).unwrap_or_default(),
        "datetime": original_exif.datetime(),
    });

    serde_json::to_string(&exif_data)
        .map_err(|e| ChamaOpticsError::InvalidParameters(e.to_string()))
}

// ============================================================================
// Theme Schema
// ============================================================================

/// Get theme schema as JSON
///
/// Returns a JSON string describing all parameters for the theme.
/// The JSON includes parameter types, labels (i18n keys), min/max values, etc.
///
/// # Example JSON
/// ```json
/// {
///   "theme_name": "one_line",
///   "theme_label": "One Line",
///   "parameters": [
///     {
///       "key": "border.bottom",
///       "label": "Bottom Border",
///       "type": "slider",
///       "min": 50.0,
///       "max": 900.0,
///       "default": 80,
///       "current": 80
///     }
///   ]
/// }
/// ```
///
/// # Safety
/// - `theme_name` must be a valid null-terminated C string
/// - Returned pointer must be freed with `chama_free_string`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_get_theme_schema(theme_name: *const c_char) -> *mut c_char {
    if theme_name.is_null() {
        return std::ptr::null_mut();
    }

    let theme_name = cstr_to_str!(theme_name, return std::ptr::null_mut());

    // Get theme schema using helper function
    let schema = match get_theme_schema_impl(theme_name) {
        Some(schema) => schema,
        None => {
            log::error!("Unknown theme or schema not available: {}", theme_name);
            return std::ptr::null_mut();
        }
    };

    // Serialize to JSON
    let json = match serde_json::to_string(&schema) {
        Ok(json) => json,
        Err(e) => {
            log::error!("Failed to serialize schema: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Convert to C string
    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get theme parameters as JSON
///
/// # Example
/// ```json
/// {
///   "theme_name": "one_line",
///   "parameters": [
///     {
///       "name": "font_size",
///       "label": "Font Size",
///       "type": "slider",
///       "min": 10.0,
///       "max": 100.0,
///       "current": 24
///     }
///   ]
/// }
/// ```
///
/// # Safety
/// - Returned pointer must be freed with `chama_free_string`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_get_theme_parameters(theme_name: *const c_char) -> *mut c_char {
    if theme_name.is_null() {
        return std::ptr::null_mut();
    }

    let theme_name = cstr_to_str!(theme_name, return std::ptr::null_mut());

    // Get theme schema using helper function
    let schema = match get_theme_schema_impl(theme_name) {
        Some(schema) => schema,
        None => {
            log::error!("Unknown theme or schema not available: {}", theme_name);
            return std::ptr::null_mut();
        }
    };

    // Build parameters response
    use serde_json::json;
    let parameters: Vec<serde_json::Value> = schema
        .parameters
        .iter()
        .map(|param| {
            // Convert enum to lowercase string for Swift matching
            let param_type_str = format!("{:?}", param.param_type).to_lowercase();
            json!({
                "name": param.name,
                "label": param.label,
                "type": param_type_str,
                "min": param.min,
                "max": param.max,
                "current": param.default,
                "exif_fields": param.exif_fields,
            })
        })
        .collect();

    let response = json!({
        "theme_name": theme_name,
        "parameters": parameters
    });

    // Serialize to JSON
    let json = match serde_json::to_string(&response) {
        Ok(json) => json,
        Err(e) => {
            log::error!("Failed to serialize parameters: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Convert to C string
    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get list of available themes as JSON array
///
/// # Example
/// ```json
/// [
///   {"name": "one_line", "label": "One Line"},
///   {"name": "film", "label": "Film"}
/// ]
/// ```
///
/// # Safety
/// - Returned pointer must be freed with `chama_free_string`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_get_available_themes() -> *mut c_char {
    let themes = vec![
        ("just_frame", "Just Frame"),
        ("one_line", "One Line"),
        ("two_line", "Two Line"),
        ("shot_on_one_line", "Shot On (One Line)"),
        ("shot_on_two_line", "Shot On (Two Line)"),
        ("strap", "Strap"),
        ("monitor", "Monitor"),
        ("lightroom", "Lightroom"),
        ("film", "Film"),
        ("film_date", "Film Date"),
        ("film_glow", "Film Glow"),
    ];

    let theme_list: Vec<_> = themes
        .into_iter()
        .map(|(name, label)| {
            serde_json::json!({
                "name": name,
                "label": label
            })
        })
        .collect();

    let json = match serde_json::to_string(&theme_list) {
        Ok(json) => json,
        Err(e) => {
            log::error!("Failed to serialize theme list: {}", e);
            return std::ptr::null_mut();
        }
    };

    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// Image Processing - Preview (Fast)
// ============================================================================

/// Generate preview with theme applied
///
/// This function prioritizes EXIF preview extraction for fast loading.
/// Perfect for real-time parameter adjustment UI.
///
/// # Strategy
/// 1. Try to extract EXIF preview (very fast, ~0.1s)
/// 2. Fallback to full image if no preview exists
/// 3. Apply theme with provided parameters
/// 4. Save to output path
///
/// # Safety
/// - All pointers must be valid null-terminated C strings
/// - `config` must point to a valid ThemeConfig struct
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_generate_preview(
    image_path: *const c_char,
    output_path: *const c_char,
    config: *const ThemeConfig,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || config.is_null() {
        return ChamaError::InvalidPath;
    }

    // Convert C strings
    let image_path = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path = cstr_to_str!(output_path, return ChamaError::InvalidPath);
    let theme_name = cstr_to_str!((*config).theme_name, return ChamaError::InvalidTheme);
    let params_json = cstr_to_str!(
        (*config).parameters_json,
        return ChamaError::InvalidParameters
    );
    let font_path = cstr_to_str!((*config).font_path, return ChamaError::InvalidFont);

    log::info!("Preview generation:");
    log::info!("  Image: {}", image_path);
    log::info!("  Theme: {}", theme_name);
    log::info!("  Font: {}", font_path);

    // Implementation of preview generation
    // SAFETY: config pointer is checked for null above
    let font_weight = unsafe { (*config).font_weight };
    match generate_preview_impl(
        image_path,
        output_path,
        theme_name,
        params_json,
        font_path,
        font_weight,
    ) {
        Ok(_) => {
            log::info!("✅ Preview generated successfully");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("❌ Preview generation failed: {}", e);
            e.into()
        }
    }
}

// ============================================================================
// Image Processing - Full Resolution (Slow)
// ============================================================================

/// Export final image with theme applied at full resolution
///
/// This always loads the original image at full resolution.
/// Use this for final export only, not for real-time preview.
///
/// # Safety
/// - All pointers must be valid null-terminated C strings
/// - `config` must point to a valid ThemeConfig struct
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_export_final(
    image_path: *const c_char,
    output_path: *const c_char,
    config: *const ThemeConfig,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || config.is_null() {
        return ChamaError::InvalidPath;
    }

    // Convert C strings
    let image_path = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path = cstr_to_str!(output_path, return ChamaError::InvalidPath);
    let theme_name = cstr_to_str!((*config).theme_name, return ChamaError::InvalidTheme);
    let params_json = cstr_to_str!(
        (*config).parameters_json,
        return ChamaError::InvalidParameters
    );
    let font_path = cstr_to_str!((*config).font_path, return ChamaError::InvalidFont);

    log::info!("Final export:");
    log::info!("  Image: {}", image_path);
    log::info!("  Output: {}", output_path);
    log::info!("  Theme: {}", theme_name);
    log::info!("  Font: {}", font_path);

    // Implementation of final export
    // SAFETY: config pointer is checked for null above
    let font_weight = unsafe { (*config).font_weight };
    let theme_params = ThemeExportParams {
        image_path,
        exif_source_path: image_path,
        output_path,
        theme_name,
        params_json,
        font_path,
        font_weight,
        scale_config: None,
        output_format_config: None,
        get_alt_fnumber: false,
        use_35mm_focal_length: false,
        exif_override_json: None,
    };
    match export_final_impl(&theme_params) {
        Ok(_) => {
            log::info!("✅ Final export completed successfully");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("❌ Final export failed: {}", e);
            e.into()
        }
    }
}

// ============================================================================
// EXIF Information
// ============================================================================

/// Extract EXIF data as JSON
///
/// Returns all EXIF fields that can be used in templates.
///
/// # Example
/// ```json
/// {
///   "camera_mnf": "Canon",
///   "camera_model": "EOS R5",
///   "lens_model": "RF 24-70mm F2.8 L IS USM",
///   "iso_speed": "800",
///   "fnumber": "2.8",
///   "exposure": "1/125",
///   "focal": "50",
///   "datetime": "2025:01:08 22:30:00"
/// }
/// ```
///
/// # Safety
/// - `image_path` must be a valid null-terminated C string
/// - Returned pointer must be freed with `chama_free_string`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_get_exif_json(image_path: *const c_char) -> *mut c_char {
    if image_path.is_null() {
        return std::ptr::null_mut();
    }

    let image_path = cstr_to_str!(image_path, return std::ptr::null_mut());

    // Extract EXIF data
    let exif_json = match extract_exif_json_impl(image_path) {
        Ok(json) => json,
        Err(e) => {
            log::error!("Failed to extract EXIF: {}", e);
            serde_json::json!({
                "error": format!("{}", e)
            })
            .to_string()
        }
    };

    let json = exif_json;

    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// Font Validation
// ============================================================================

/// Validate if a font path is loadable
///
/// Returns true if the font file exists and can be parsed.
///
/// # Safety
/// - `font_path` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_validate_font_path(font_path: *const c_char) -> bool {
    if font_path.is_null() {
        return false;
    }

    let font_path = cstr_to_str!(font_path, return false);

    // Check file exists
    let path = Path::new(font_path);
    if !path.exists() {
        log::warn!("Font file does not exist: {}", font_path);
        return false;
    }

    // Try to load and parse
    match std::fs::read(font_path) {
        Ok(font_data) => match ab_glyph::FontArc::try_from_vec(font_data) {
            Ok(_) => {
                log::info!("✅ Font validated: {}", font_path);
                true
            }
            Err(e) => {
                log::error!("❌ Font parse error: {}", e);
                false
            }
        },
        Err(e) => {
            log::error!("❌ Font read error: {}", e);
            false
        }
    }
}

// ============================================================================
// Version Info
// ============================================================================

/// Get library version
///
/// # Safety
/// - Returned pointer must be freed with `chama_free_string`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_get_version() -> *mut c_char {
    let version = env!("CARGO_PKG_VERSION");
    match CString::new(version) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get available themes (iOS alias)
/// This is an alias for chama_get_available_themes() to match Swift naming
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_get_available_themes_ios() -> *mut c_char {
    unsafe { chama_get_available_themes() }
}

// ============================================================================
// RGBA Theme Application
// ============================================================================

/// Apply theme to image from raw RGBA pixel data (for HEIF/HEIC support)
///
/// This function accepts raw RGBA pixel data decoded by Apple's native image APIs,
/// avoiding lossy JPEG conversion and leveraging iOS/macOS native HEIF decoding.
///
/// # Parameters
/// - `rgba_data`: Pointer to RGBA pixel data (4 bytes per pixel)
/// - `width`: Image width in pixels
/// - `height`: Image height in pixels
/// - `data_length`: Total data length (must equal width * height * 4)
/// - `exif_source_path`: Path to original image for EXIF extraction (can be null)
/// - `output_path`: Path for the output file
/// - `theme_name`: Name of the theme to apply
/// - `params_json`: Theme parameters as JSON string
/// - `font_path`: Path to the font file
/// - `font_weight`: Font weight (100-900)
///
/// # Returns
/// ChamaError enum (0 = Success, non-zero = error code)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_optics_apply_theme_from_rgba(
    rgba_data: *const u8,
    width: u32,
    height: u32,
    data_length: usize,
    _exif_source_path: *const c_char,
    output_path: *const c_char,
    theme_name: *const c_char,
    params_json: *const c_char,
    font_path: *const c_char,
    font_weight: u32,
) -> ChamaError {
    // Validate inputs
    if rgba_data.is_null() || output_path.is_null() || theme_name.is_null() {
        log::error!("Null pointer in apply_theme_from_rgba");
        return ChamaError::InvalidPath;
    }

    // Validate data length
    let expected_length = (width as usize) * (height as usize) * 4;
    if data_length != expected_length {
        log::error!(
            "Invalid RGBA data length: expected {}, got {}",
            expected_length,
            data_length
        );
        return ChamaError::ImageLoadError;
    }

    log::info!("Applying theme from RGBA data: {}x{}", width, height);

    // Convert C strings
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);
    let theme_name_str = cstr_to_str!(theme_name, return ChamaError::InvalidTheme);
    let params_json_str = cstr_to_str_or!(params_json, "{}");
    let font_path_str = cstr_to_str_or!(font_path, "");

    // Convert raw RGBA data to DynamicImage
    let rgba_slice = unsafe { std::slice::from_raw_parts(rgba_data, data_length) };
    let rgba_vec = rgba_slice.to_vec();

    let img_buffer = match image::RgbaImage::from_raw(width, height, rgba_vec) {
        Some(buffer) => buffer,
        None => {
            log::error!("Failed to create RgbaImage from raw data");
            return ChamaError::ImageLoadError;
        }
    };

    let dynamic_img = image::DynamicImage::ImageRgba8(img_buffer);
    log::info!("Successfully created DynamicImage from RGBA data");

    // Save RGBA image to a temporary file
    let temp_dir = std::env::temp_dir();
    let temp_image_path = temp_dir.join(format!("chama_rgba_{}.jpg", uuid::Uuid::new_v4()));

    if let Err(e) = dynamic_img.save(&temp_image_path) {
        log::error!("Failed to save temp image: {}", e);
        return ChamaError::ImageProcessError;
    }

    log::info!(
        "Saved RGBA data to temp file: {}",
        temp_image_path.display()
    );

    // Call existing preview generation logic with temp file
    let temp_path_str = temp_image_path.to_str().unwrap_or("");

    let result = match generate_preview_impl(
        temp_path_str,
        output_path_str,
        theme_name_str,
        params_json_str,
        font_path_str,
        font_weight,
    ) {
        Ok(()) => {
            log::info!("Successfully applied theme from RGBA data");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to apply theme: {}", e);
            e.into()
        }
    };

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_image_path);

    result
}

// ============================================================================
// Theme v2
// ============================================================================

/// Unified theme export function (replaces apply_theme, _with_exif, _with_exif_scale_and_export, _with_exif_override)
///
/// All optional fields in CThemeExportConfig can be NULL for defaults:
/// - exif_source_path: NULL = use image_path
/// - scale_config: NULL = default 4K scaling
/// - output_format_config: NULL = WebP quality 90
/// - exif_override_json: NULL = no override
#[unsafe(no_mangle)]
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe extern "C" fn chama_optics_apply_theme_v2(
    config: *const CThemeExportConfig,
) -> ChamaError {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if config.is_null() {
            return ChamaError::InvalidPath;
        }

        let config_ref = &*config;

        if config_ref.image_path.is_null()
            || config_ref.output_path.is_null()
            || config_ref.theme_name.is_null()
        {
            return ChamaError::InvalidPath;
        }

        let image_path_str = cstr_to_str!(config_ref.image_path, return ChamaError::InvalidPath);

        let exif_source_str = cstr_to_str_or!(config_ref.exif_source_path, image_path_str);

        let output_path_str = cstr_to_str!(config_ref.output_path, return ChamaError::InvalidPath);
        let theme_name_str = cstr_to_str!(config_ref.theme_name, return ChamaError::InvalidTheme);
        let params_json_str = cstr_to_str_or!(config_ref.params_json, "{}");
        let font_path_str = cstr_to_str_or!(config_ref.font_path, "");

        let exif_override_str = if config_ref.exif_override_json.is_null() {
            None
        } else {
            let s = cstr_to_str_or!(config_ref.exif_override_json, "");
            if !s.is_empty() { Some(s) } else { None }
        };

        let core_scale_config = super::convert_c_scale_config(config_ref.scale_config);

        let export_config_option = if config_ref.output_format_config.is_null() {
            None
        } else {
            Some(*config_ref.output_format_config)
        };

        log::info!(
            "apply_theme_v2: image={}, theme={}",
            image_path_str,
            theme_name_str
        );

        let theme_params = ThemeExportParams {
            image_path: image_path_str,
            exif_source_path: exif_source_str,
            output_path: output_path_str,
            theme_name: theme_name_str,
            params_json: params_json_str,
            font_path: font_path_str,
            font_weight: config_ref.font_weight,
            scale_config: core_scale_config,
            output_format_config: export_config_option,
            get_alt_fnumber: config_ref.get_alt_fnumber,
            use_35mm_focal_length: config_ref.use_35mm_focal_length,
            exif_override_json: exif_override_str,
        };

        match export_final_impl(&theme_params) {
            Ok(_) => {
                log::info!("✅ Theme applied successfully");
                ChamaError::Success
            }
            Err(e) => {
                log::error!("Failed to apply theme: {}", e);
                e.into()
            }
        }
    }));

    match result {
        Ok(error_code) => error_code,
        Err(panic_info) => {
            let msg = super::extract_panic_message(&panic_info);
            log::error!("Caught panic in apply_theme_v2: {}", msg);
            ChamaError::ImageProcessError
        }
    }
}

// ============================================================================
// Font Directory Configuration
// ============================================================================

/// Set the base directory for font files
///
/// This must be called before rendering themes to ensure fonts can be loaded.
/// The directory should contain font files like "Barlow-Variable-Remapped.ttf", "digital-7.ttf", etc.
///
/// # Example
/// ```c
/// chama_set_fonts_base_directory("/var/mobile/Containers/Bundle/Application/.../ChamaOptics.app/fonts");
/// ```
///
/// # Safety
/// - `path` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_set_fonts_base_directory(path: *const c_char) {
    if path.is_null() {
        log::warn!("chama_set_fonts_base_directory: null path provided");
        return;
    }

    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            log::error!("chama_set_fonts_base_directory: invalid UTF-8: {}", e);
            return;
        }
    };

    crate::effect::variable_text::set_fonts_base_directory(path_str);
}

// ============================================================================
// Memory Management
// ============================================================================

/// Free a string returned by other FFI functions
///
/// # Safety
/// - `ptr` must have been returned by a chama_* function that returns *mut c_char
/// - Must only be called once per pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        unsafe {
            let version_ptr = chama_get_version();
            assert!(!version_ptr.is_null());

            let version = unsafe { CStr::from_ptr(version_ptr) }.to_str().unwrap();
            assert!(!version.is_empty());

            chama_free_string(version_ptr);
        }
    }

    #[test]
    fn test_theme_list() {
        unsafe {
            let themes_ptr = chama_get_available_themes();
            assert!(!themes_ptr.is_null());

            let themes_json = unsafe { CStr::from_ptr(themes_ptr) }.to_str().unwrap();
            let themes: Vec<serde_json::Value> = serde_json::from_str(themes_json).unwrap();

            assert!(themes.len() > 0);
            assert!(themes[0]["name"].is_string());
            assert!(themes[0]["label"].is_string());

            chama_free_string(themes_ptr);
        }
    }
}

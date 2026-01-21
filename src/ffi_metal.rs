/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! FFI interface for Metal-based iOS/macOS integration
//!
//! This module provides C-compatible functions for Swift to call.
//! Key differences from egui version:
//! - Font loading is path-based (no built-in fonts)
//! - Preview-first strategy (EXIF preview extraction)
//! - JSON-based parameter exchange

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

// ============================================================================
// Internal Error Types
// ============================================================================

#[derive(Debug)]
enum PreviewError {
    IoError(std::io::Error),
    ImageLoad(image::ImageError),
    ImageProcess(image::ImageError),
    InvalidTheme,
    InvalidFont,
    InvalidParameters(String),
    ExifError,
}

impl std::fmt::Display for PreviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreviewError::IoError(e) => write!(f, "IO error: {}", e),
            PreviewError::ImageLoad(e) => write!(f, "Image load error: {}", e),
            PreviewError::ImageProcess(e) => write!(f, "Image process error: {}", e),
            PreviewError::InvalidTheme => write!(f, "Invalid theme"),
            PreviewError::InvalidFont => write!(f, "Invalid font"),
            PreviewError::InvalidParameters(s) => write!(f, "Invalid parameters: {}", s),
            PreviewError::ExifError => write!(f, "EXIF error"),
        }
    }
}

impl std::error::Error for PreviewError {}

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

// ============================================================================
// Parameter Update Helper
// ============================================================================

/// Update theme parameters from JSON string
/// Uses macro-based downcasting to access ThemeParameters trait
fn update_theme_from_json(
    theme: &mut dyn crate::theme::Theme,
    params_json: &str,
) -> Result<(), PreviewError> {
    use crate::theme::parameter_schema::ThemeParameters;

    // Parse JSON
    let updates: serde_json::Map<String, serde_json::Value> = serde_json::from_str(params_json)
        .map_err(|e| PreviewError::InvalidParameters(format!("JSON parse error: {}", e)))?;

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
                    .map_err(|e| PreviewError::InvalidParameters(e));
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
    Err(PreviewError::InvalidTheme)
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
) -> Result<(), PreviewError> {
    // 1. Try to load EXIF preview first, fallback to full image
    let _dyn_image = if let Some(preview) = extract_exif_preview(image_path) {
        log::info!("✅ Loaded EXIF preview");
        preview
    } else {
        log::info!("⚠️ No EXIF preview, loading full image");
        image::open(image_path).map_err(PreviewError::ImageLoad)?
    };

    // 2. Parse EXIF for metadata
    let exif = {
        let file = std::fs::File::open(image_path).map_err(PreviewError::IoError)?;
        let mut buf_reader = std::io::BufReader::new(file);
        exif::Reader::new()
            .read_from_container(&mut buf_reader)
            .ok()
    };

    let original_exif = crate::image::exif_impl::OriginalExif::new(exif);
    let view_exif = crate::image::exif_impl::SimplifiedExif::from(&original_exif);

    // 3. Create theme instance
    let mut theme = crate::theme::create_theme(theme_name).ok_or(PreviewError::InvalidTheme)?;

    // 4. Update theme parameters from JSON
    update_theme_from_json(&mut *theme, params_json)?;

    // 5. Validate font path and weight
    // Font path and weight will be used by theme rendering
    // Note: font_weight (100-900) will be handled by Swift's variable font system
    if !Path::new(font_path).exists() {
        log::warn!("Font path does not exist: {}", font_path);
        return Err(PreviewError::InvalidFont);
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
    };

    // 7. Apply theme
    let export_config = crate::export_config::ExportConfig::default();
    let mut themed_image = theme
        .apply_to_image(&packed_image, &export_config)
        .map_err(PreviewError::ImageProcess)?;

    // 8. Save output
    export_config
        .save_image(&mut themed_image, None, Path::new(output_path))
        .map_err(PreviewError::ImageProcess)?;

    Ok(())
}

/// Internal implementation of final export
fn export_final_impl(
    image_path: &str,
    output_path: &str,
    theme_name: &str,
    params_json: &str,
    font_path: &str,
    font_weight: u32,
) -> Result<(), PreviewError> {
    export_final_impl_with_exif_source(
        image_path,
        image_path, // Use same path for EXIF source by default
        output_path,
        theme_name,
        params_json,
        font_path,
        font_weight,
        None, // Use default scale config
    )
}

/// Export with theme, allowing separate EXIF source and optional scale config
/// This is useful when the image has been modified (e.g., stickers applied)
/// but we want to read EXIF from the original image
fn export_final_impl_with_exif_source(
    image_path: &str,
    exif_source_path: &str,
    output_path: &str,
    theme_name: &str,
    params_json: &str,
    font_path: &str,
    font_weight: u32,
    scale_config: Option<crate::scale_config::ScaleConfig>,
) -> Result<(), PreviewError> {
    // For final export, always load full resolution
    let _dyn_image = image::open(image_path).map_err(PreviewError::ImageLoad)?;

    log::info!(
        "✅ Loaded full resolution image: {}x{}",
        _dyn_image.width(),
        _dyn_image.height()
    );

    // Parse EXIF from the original source (not the modified image)
    let exif = {
        let file = std::fs::File::open(exif_source_path).map_err(PreviewError::IoError)?;
        let mut buf_reader = std::io::BufReader::new(file);
        exif::Reader::new()
            .read_from_container(&mut buf_reader)
            .ok()
    };

    if exif.is_some() {
        log::info!("✅ Loaded EXIF from: {}", exif_source_path);
    } else {
        log::warn!("⚠️ No EXIF data found in: {}", exif_source_path);
    }

    let original_exif = crate::image::exif_impl::OriginalExif::new(exif);
    let mut view_exif = crate::image::exif_impl::SimplifiedExif::from(&original_exif);

    // If image_path differs from exif_source_path, the image has already been processed
    // (e.g., face effects applied via load_image_direct which applies orientation).
    // In this case, we should NOT apply orientation again to avoid double rotation.
    if image_path != exif_source_path {
        log::info!(
            "  Image path differs from EXIF source - skipping orientation (already applied)"
        );
        view_exif.orientation = image::metadata::Orientation::NoTransforms;
    }

    // 3. Create theme instance
    let mut theme = crate::theme::create_theme(theme_name).ok_or(PreviewError::InvalidTheme)?;

    // 4. Update theme parameters from JSON
    update_theme_from_json(&mut *theme, params_json)?;

    // 5. Validate font path and weight
    // Font path and weight will be used by theme rendering
    // Note: font_weight (100-900) will be handled by Swift's variable font system
    if !Path::new(font_path).exists() {
        log::warn!("Font path does not exist: {}", font_path);
        return Err(PreviewError::InvalidFont);
    }
    log::info!(
        "Font validation passed: {} (weight: {})",
        font_path,
        font_weight
    );

    // 6. Create a PackedImage for theme application
    use crate::image::common::PackedTexture;
    use crate::image::packed_image::PackedImage;
    use uuid::Uuid;

    // Read the image bytes from the input path (which may be a temp file with face effects)
    #[cfg(not(feature = "desktop"))]
    let image_bytes = match &std::fs::read(image_path) {
        Ok(bytes) => {
            log::info!(
                "✅ Read {} bytes from input image: {}",
                bytes.len(),
                image_path
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
        path: Path::new(image_path).to_path_buf(),
        src_exif: original_exif,
        view_exif,
        editable: false,
        texture: PackedTexture::Dummy,
        #[cfg(not(feature = "desktop"))]
        image_bytes,
        sticker_bytes: None,
        perceptual_hash: None,
    };

    // 7. Apply theme with custom scale config if provided
    let mut export_config = crate::export_config::ExportConfig::default();
    if let Some(custom_scale) = scale_config {
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
    let mut themed_image = theme
        .apply_to_image(&packed_image, &export_config)
        .map_err(PreviewError::ImageProcess)?;

    // 8. Save output
    export_config
        .save_image(&mut themed_image, None, Path::new(output_path))
        .map_err(PreviewError::ImageProcess)?;

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
fn extract_exif_json_impl(image_path: &str) -> Result<String, PreviewError> {
    use crate::image::exif_impl::OriginalExif;

    // Open file and parse EXIF
    let file = std::fs::File::open(image_path).map_err(PreviewError::IoError)?;
    let mut buf_reader = std::io::BufReader::new(file);

    let exif = exif::Reader::new()
        .read_from_container(&mut buf_reader)
        .map_err(|_| PreviewError::ExifError)?;

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

    serde_json::to_string(&exif_data).map_err(|e| PreviewError::InvalidParameters(e.to_string()))
}

/// Theme configuration passed from Swift
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

    let theme_name = match unsafe { CStr::from_ptr(theme_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

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

    let theme_name = match unsafe { CStr::from_ptr(theme_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

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
    let image_path = match unsafe { CStr::from_ptr(image_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };
    let output_path = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let theme_name = match unsafe { CStr::from_ptr((*config).theme_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidTheme,
    };

    let params_json = match unsafe { CStr::from_ptr((*config).parameters_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidParameters,
    };

    let font_path = match unsafe { CStr::from_ptr((*config).font_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidFont,
    };

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
            match e {
                PreviewError::IoError(_) => ChamaError::ImageLoadError,
                PreviewError::ImageLoad(_) => ChamaError::ImageLoadError,
                PreviewError::ImageProcess(_) => ChamaError::ImageProcessError,
                PreviewError::InvalidTheme => ChamaError::InvalidTheme,
                PreviewError::InvalidFont => ChamaError::InvalidFont,
                PreviewError::InvalidParameters(_) => ChamaError::InvalidParameters,
                PreviewError::ExifError => ChamaError::ExifError,
            }
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
    let image_path = match unsafe { CStr::from_ptr(image_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };
    let output_path = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let theme_name = match unsafe { CStr::from_ptr((*config).theme_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidTheme,
    };

    let params_json = match unsafe { CStr::from_ptr((*config).parameters_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidParameters,
    };

    let font_path = match unsafe { CStr::from_ptr((*config).font_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidFont,
    };

    log::info!("Final export:");
    log::info!("  Image: {}", image_path);
    log::info!("  Output: {}", output_path);
    log::info!("  Theme: {}", theme_name);
    log::info!("  Font: {}", font_path);

    // Implementation of final export
    // SAFETY: config pointer is checked for null above
    let font_weight = unsafe { (*config).font_weight };
    match export_final_impl(
        image_path,
        output_path,
        theme_name,
        params_json,
        font_path,
        font_weight,
    ) {
        Ok(_) => {
            log::info!("✅ Final export completed successfully");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("❌ Final export failed: {}", e);
            match e {
                PreviewError::IoError(_) => ChamaError::ImageLoadError,
                PreviewError::ImageLoad(_) => ChamaError::ImageLoadError,
                PreviewError::ImageProcess(_) => ChamaError::ImageProcessError,
                PreviewError::InvalidTheme => ChamaError::InvalidTheme,
                PreviewError::InvalidFont => ChamaError::InvalidFont,
                PreviewError::InvalidParameters(_) => ChamaError::InvalidParameters,
                PreviewError::ExifError => ChamaError::ExifError,
            }
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

    let image_path = match unsafe { CStr::from_ptr(image_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

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

    let font_path = match unsafe { CStr::from_ptr(font_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

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

/// Apply theme to image (alias for Swift)
/// This is an alias for chama_generate_preview to maintain API compatibility
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_optics_apply_theme(
    image_path: *const c_char,
    output_path: *const c_char,
    theme_name: *const c_char,
    params_json: *const c_char,
    font_path: *const c_char,
    font_weight: u32,
) -> ChamaError {
    // Create config struct
    let config = ThemeConfig {
        theme_name,
        parameters_json: params_json,
        font_path,
        font_weight,
    };

    // Call preview generation function
    unsafe { chama_generate_preview(image_path, output_path, &config) }
}

/// Apply theme to image with separate EXIF source
///
/// This function is useful when the image has been modified (e.g., stickers applied)
/// but we want to read EXIF metadata from the original image file.
///
/// # Parameters
/// - `image_path`: Path to the image to apply theme to (may be modified)
/// - `exif_source_path`: Path to the original image for reading EXIF data
/// - `output_path`: Path for the output file
/// - `theme_name`: Name of the theme to apply
/// - `params_json`: Theme parameters as JSON string
/// - `font_path`: Path to the font file
/// - `font_weight`: Font weight (100-900)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_optics_apply_theme_with_exif(
    image_path: *const c_char,
    exif_source_path: *const c_char,
    output_path: *const c_char,
    theme_name: *const c_char,
    params_json: *const c_char,
    font_path: *const c_char,
    font_weight: u32,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || theme_name.is_null() {
        return ChamaError::InvalidPath;
    }

    let image_path_str = unsafe {
        match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        }
    };

    // Use image_path as EXIF source if exif_source_path is null
    let exif_source_str = if exif_source_path.is_null() {
        image_path_str
    } else {
        unsafe {
            match CStr::from_ptr(exif_source_path).to_str() {
                Ok(s) => s,
                Err(_) => image_path_str,
            }
        }
    };

    let output_path_str = unsafe {
        match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        }
    };

    let theme_name_str = unsafe {
        match CStr::from_ptr(theme_name).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidTheme,
        }
    };

    let params_json_str = if params_json.is_null() {
        "{}"
    } else {
        unsafe { CStr::from_ptr(params_json).to_str().unwrap_or("{}") }
    };

    let font_path_str = if font_path.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(font_path).to_str().unwrap_or("") }
    };

    log::info!("Applying theme with separate EXIF source:");
    log::info!("  Image: {}", image_path_str);
    log::info!("  EXIF source: {}", exif_source_str);
    log::info!("  Theme: {}", theme_name_str);

    match export_final_impl_with_exif_source(
        image_path_str,
        exif_source_str,
        output_path_str,
        theme_name_str,
        params_json_str,
        font_path_str,
        font_weight,
        None, // No custom scale config - use default
    ) {
        Ok(_) => {
            log::info!("✅ Theme applied successfully with EXIF from original");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to apply theme: {}", e);
            match e {
                PreviewError::InvalidTheme => ChamaError::InvalidTheme,
                PreviewError::InvalidFont => ChamaError::InvalidFont,
                PreviewError::ImageLoad(_) => ChamaError::ImageLoadError,
                _ => ChamaError::ImageProcessError,
            }
        }
    }
}

/// Apply theme to image with separate EXIF source and custom scale config
///
/// Same as `chama_optics_apply_theme_with_exif` but allows specifying custom scale settings.
/// Use this function when you need to control the output image size during theme application.
///
/// # Parameters
/// - `image_path`: Path to the image to apply theme to (may be modified)
/// - `exif_source_path`: Path to the original image for reading EXIF data
/// - `output_path`: Path for the output file
/// - `theme_name`: Name of the theme to apply
/// - `params_json`: Theme parameters as JSON string
/// - `font_path`: Path to the font file
/// - `font_weight`: Font weight (100-900)
/// - `scale_config`: Pointer to CScaleConfig for custom scaling (pass null for default 4K scaling)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_optics_apply_theme_with_exif_and_scale(
    image_path: *const c_char,
    exif_source_path: *const c_char,
    output_path: *const c_char,
    theme_name: *const c_char,
    params_json: *const c_char,
    font_path: *const c_char,
    font_weight: u32,
    scale_config: *const CScaleConfig,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || theme_name.is_null() {
        return ChamaError::InvalidPath;
    }

    let image_path_str = unsafe {
        match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        }
    };

    // Use image_path as EXIF source if exif_source_path is null
    let exif_source_str = if exif_source_path.is_null() {
        image_path_str
    } else {
        unsafe {
            match CStr::from_ptr(exif_source_path).to_str() {
                Ok(s) => s,
                Err(_) => image_path_str,
            }
        }
    };

    let output_path_str = unsafe {
        match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        }
    };

    let theme_name_str = unsafe {
        match CStr::from_ptr(theme_name).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidTheme,
        }
    };

    let params_json_str = if params_json.is_null() {
        "{}"
    } else {
        unsafe { CStr::from_ptr(params_json).to_str().unwrap_or("{}") }
    };

    let font_path_str = if font_path.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(font_path).to_str().unwrap_or("") }
    };

    // Convert CScaleConfig to core ScaleConfig if provided
    let core_scale_config = if scale_config.is_null() {
        None
    } else {
        let config_ref = unsafe { &*scale_config };
        if config_ref.mode == CScaleMode::None {
            None
        } else {
            Some(crate::scale_config::ScaleConfig {
                mode: match config_ref.mode {
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
                },
                value: config_ref.value,
                sub_value: config_ref.sub_value,
                scale_value: config_ref.scale_value as f32,
            })
        }
    };

    log::info!("Applying theme with separate EXIF source and scale config:");
    log::info!("  Image: {}", image_path_str);
    log::info!("  EXIF source: {}", exif_source_str);
    log::info!("  Theme: {}", theme_name_str);
    if let Some(ref sc) = core_scale_config {
        log::info!("  Scale mode: {:?}, value: {}", sc.mode, sc.value);
    } else {
        log::info!("  Scale: default (4K)");
    }

    match export_final_impl_with_exif_source(
        image_path_str,
        exif_source_str,
        output_path_str,
        theme_name_str,
        params_json_str,
        font_path_str,
        font_weight,
        core_scale_config,
    ) {
        Ok(_) => {
            log::info!("✅ Theme applied successfully with EXIF from original");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to apply theme: {}", e);
            match e {
                PreviewError::InvalidTheme => ChamaError::InvalidTheme,
                PreviewError::InvalidFont => ChamaError::InvalidFont,
                PreviewError::ImageLoad(_) => ChamaError::ImageLoadError,
                _ => ChamaError::ImageProcessError,
            }
        }
    }
}

// ============================================================================
// Combined Export Pipeline (Face Effects + Theme + Export Quality)
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

/// Configuration for combined export pipeline
#[repr(C)]
pub struct CombinedExportConfig {
    /// Face effect type (None, Mosaic, Stroke, Sticker)
    pub face_effect_type: CFaceEffectType,

    // Mosaic settings
    pub mosaic_block_size: u32,
    pub mosaic_intensity: f32,

    // Stroke settings
    pub stroke_color_r: u8,
    pub stroke_color_g: u8,
    pub stroke_color_b: u8,
    pub stroke_color_a: u8,
    pub stroke_thickness: u32,

    // Sticker settings (path to sticker image)
    pub sticker_image_path: *const c_char,
    pub sticker_scale: f32,
    pub sticker_offset_x: i32,
    pub sticker_offset_y: i32,

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
}

// ============================================================================
// Scale Image Helper
// ============================================================================

/// Apply scaling to a DynamicImage based on CScaleConfig
fn apply_scale_to_image(
    image: &image::DynamicImage,
    scale_config: &CScaleConfig,
) -> image::DynamicImage {
    use image::imageops::FilterType;

    if scale_config.mode == CScaleMode::None {
        return image.clone();
    }

    let src_width = image.width();
    let src_height = image.height();

    // Calculate target dimensions based on scale mode
    let (target_width, target_height) = match scale_config.mode {
        CScaleMode::None => (src_width, src_height),

        CScaleMode::MaxWidth => {
            let target_w = scale_config.value;
            if src_width <= target_w {
                (src_width, src_height)
            } else {
                let ratio = target_w as f64 / src_width as f64;
                (target_w, (src_height as f64 * ratio).round() as u32)
            }
        }

        CScaleMode::MaxHeight => {
            let target_h = scale_config.value;
            if src_height <= target_h {
                (src_width, src_height)
            } else {
                let ratio = target_h as f64 / src_height as f64;
                ((src_width as f64 * ratio).round() as u32, target_h)
            }
        }

        CScaleMode::Longside => {
            let target = scale_config.value;
            let longside = src_width.max(src_height);
            if longside <= target {
                (src_width, src_height)
            } else {
                let ratio = target as f64 / longside as f64;
                (
                    (src_width as f64 * ratio).round() as u32,
                    (src_height as f64 * ratio).round() as u32,
                )
            }
        }

        CScaleMode::Divide => {
            let divider = scale_config.scale_value;
            if divider <= 1.0 {
                (src_width, src_height)
            } else {
                (
                    (src_width as f64 / divider).round() as u32,
                    (src_height as f64 / divider).round() as u32,
                )
            }
        }

        CScaleMode::NearCommonWidth => {
            let target_w = scale_config.value;
            // Simplified: just resize to target width maintaining aspect ratio
            let ratio = target_w as f64 / src_width as f64;
            (target_w, (src_height as f64 * ratio).round() as u32)
        }

        CScaleMode::NearCommonHeight => {
            let target_h = scale_config.value;
            // Simplified: just resize to target height maintaining aspect ratio
            let ratio = target_h as f64 / src_height as f64;
            ((src_width as f64 * ratio).round() as u32, target_h)
        }

        CScaleMode::ResizeAndCrop => {
            let target_w = scale_config.value;
            let target_h = scale_config.sub_value;

            // Calculate scale to fill the target area
            let width_ratio = target_w as f64 / src_width as f64;
            let height_ratio = target_h as f64 / src_height as f64;
            let ratio = width_ratio.max(height_ratio);

            let scaled_w = (src_width as f64 * ratio).round() as u32;
            let scaled_h = (src_height as f64 * ratio).round() as u32;

            // First resize, then crop to exact dimensions
            let resized = image.resize(scaled_w, scaled_h, FilterType::Lanczos3);

            // Calculate crop position (center crop)
            let crop_x = (scaled_w.saturating_sub(target_w)) / 2;
            let crop_y = (scaled_h.saturating_sub(target_h)) / 2;

            return resized.crop_imm(crop_x, crop_y, target_w, target_h);
        }
    };

    // Apply resize if dimensions changed
    if target_width != src_width || target_height != src_height {
        log::info!(
            "  Scaling image from {}x{} to {}x{}",
            src_width,
            src_height,
            target_width,
            target_height
        );
        image.resize(target_width, target_height, FilterType::Lanczos3)
    } else {
        image.clone()
    }
}

/// Scale image standalone function
///
/// Scales an image according to the provided configuration and saves to output path.
///
/// # Safety
/// - image_path and output_path must be valid null-terminated C strings
/// - scale_config must point to a valid CScaleConfig struct
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_scale_image(
    image_path: *const c_char,
    output_path: *const c_char,
    scale_config: *const CScaleConfig,
    output_format: COutputFormat,
    quality: u8,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || scale_config.is_null() {
        return ChamaError::InvalidPath;
    }

    let image_path_str = unsafe {
        match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        }
    };

    let output_path_str = unsafe {
        match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        }
    };

    let config_ref = unsafe { &*scale_config };

    log::info!("Scale image:");
    log::info!("  Input: {}", image_path_str);
    log::info!("  Output: {}", output_path_str);
    log::info!("  Scale mode: {:?}", config_ref.mode);

    // Load image
    let dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

    log::info!(
        "  Original size: {}x{}",
        dyn_image.width(),
        dyn_image.height()
    );

    // Apply scaling
    let scaled_image = apply_scale_to_image(&dyn_image, config_ref);

    log::info!(
        "  Scaled size: {}x{}",
        scaled_image.width(),
        scaled_image.height()
    );

    // Save with specified format and quality
    let save_result = match output_format {
        COutputFormat::Jpeg => {
            use image::codecs::jpeg::JpegEncoder;
            let file = match std::fs::File::create(output_path_str) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create output file: {}", e);
                    return ChamaError::ImageProcessError;
                }
            };
            let mut encoder = JpegEncoder::new_with_quality(file, quality);
            encoder.encode_image(&scaled_image)
        }
        COutputFormat::Png => scaled_image.save(output_path_str).map_err(|e| e.into()),
        COutputFormat::Webp => {
            use image::ImageFormat;
            let file = match std::fs::File::create(output_path_str) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create output file: {}", e);
                    return ChamaError::ImageProcessError;
                }
            };
            let mut buf_writer = std::io::BufWriter::new(file);
            scaled_image.write_to(&mut buf_writer, ImageFormat::WebP)
        }
    };

    match save_result {
        Ok(_) => {
            log::info!("✅ Scale image completed successfully");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to save scaled image: {}", e);
            ChamaError::ImageProcessError
        }
    }
}

/// Combined export: Face Effects → Theme → Scale → Save with Quality
///
/// This is the recommended function for iOS to handle the full export pipeline
/// in a single call, minimizing file I/O and providing atomic operations.
///
/// # Pipeline
/// 1. Load original image
/// 2. Apply face effects (if faces provided and effect != None)
/// 3. Apply theme (if theme_name provided)
/// 4. Apply scaling (if scale_mode != None)
/// 5. Save with specified format and quality
///
/// # Safety
/// - All C string pointers must be valid null-terminated strings or NULL
/// - face_rects must point to a valid array of CFaceRect with face_count elements
#[unsafe(no_mangle)]
#[cfg(target_os = "ios")]
pub unsafe extern "C" fn chama_export_combined(
    image_path: *const c_char,
    output_path: *const c_char,
    face_rects: *const crate::ffi::CFaceRect,
    face_count: usize,
    config: *const CombinedExportConfig,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || config.is_null() {
        return ChamaError::InvalidPath;
    }

    let image_path_str = unsafe {
        match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        }
    };

    let output_path_str = unsafe {
        match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        }
    };

    let config_ref = unsafe { &*config };

    log::info!("Combined export pipeline started:");
    log::info!("  Input: {}", image_path_str);
    log::info!("  Output: {}", output_path_str);
    log::info!("  Face effect: {:?}", config_ref.face_effect_type);
    log::info!("  Face count: {}", face_count);

    // Step 1: Load original image and apply EXIF orientation
    let mut dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

    log::info!(
        "  Raw image size: {}x{}",
        dyn_image.width(),
        dyn_image.height()
    );

    // Read EXIF orientation and apply it so face coordinates match the display orientation
    let orientation = {
        use exif::{In, Tag};
        let file = match std::fs::File::open(image_path_str) {
            Ok(f) => f,
            Err(e) => {
                log::warn!("Failed to open file for EXIF: {}", e);
                return ChamaError::ImageLoadError;
            }
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
    };

    log::info!("  EXIF orientation: {:?}", orientation);
    dyn_image.apply_orientation(orientation);
    log::info!(
        "  After orientation: {}x{}",
        dyn_image.width(),
        dyn_image.height()
    );

    // Step 2: Apply face effects (if faces provided and effect != None)
    if !face_rects.is_null()
        && face_count > 0
        && config_ref.face_effect_type != CFaceEffectType::None
    {
        let mut face_areas: Vec<(i32, i32, u32, u32)> = Vec::with_capacity(face_count);
        for i in 0..face_count {
            let face = unsafe { *face_rects.add(i) };
            face_areas.push((face.x, face.y, face.width, face.height));
        }

        log::info!("  Applying face effect to {} faces...", face_areas.len());

        match config_ref.face_effect_type {
            CFaceEffectType::Mosaic => {
                let mosaic_config = crate::effect::mosaic::MosaicEffect {
                    block_size: config_ref.mosaic_block_size,
                    intensity: config_ref.mosaic_intensity,
                };
                if let Err(e) = crate::effect::mosaic::MosaicEffect::apply(
                    &mut dyn_image,
                    &face_areas,
                    &mosaic_config,
                ) {
                    log::error!("Failed to apply mosaic: {}", e);
                    return ChamaError::ImageProcessError;
                }
                log::info!("  Mosaic applied successfully");
            }
            CFaceEffectType::Stroke => {
                let stroke_config = crate::effect::stroke::StrokeEffect {
                    thickness: config_ref.stroke_thickness,
                    color: (
                        config_ref.stroke_color_r,
                        config_ref.stroke_color_g,
                        config_ref.stroke_color_b,
                        config_ref.stroke_color_a,
                    ),
                };
                if let Err(e) = crate::effect::stroke::StrokeEffect::apply(
                    &mut dyn_image,
                    &face_areas,
                    &stroke_config,
                ) {
                    log::error!("Failed to apply stroke: {}", e);
                    return ChamaError::ImageProcessError;
                }
                log::info!("  Stroke applied successfully");
            }
            CFaceEffectType::Sticker => {
                // Load sticker path
                let sticker_config = if !config_ref.sticker_image_path.is_null() {
                    let sticker_path_str = unsafe {
                        match CStr::from_ptr(config_ref.sticker_image_path).to_str() {
                            Ok(s) => s,
                            Err(_) => {
                                log::error!("Invalid sticker path");
                                return ChamaError::InvalidPath;
                            }
                        }
                    };
                    crate::effect::sticker::StickerConfig::with_image_path(
                        std::path::PathBuf::from(sticker_path_str),
                        config_ref.sticker_scale,
                        config_ref.sticker_offset_x,
                        config_ref.sticker_offset_y,
                    )
                } else {
                    crate::effect::sticker::StickerConfig::with_builtin(
                        "heart".to_string(),
                        config_ref.sticker_scale,
                        config_ref.sticker_offset_x,
                        config_ref.sticker_offset_y,
                    )
                };
                dyn_image =
                    crate::effect::sticker::apply_sticker(dyn_image, face_areas, &sticker_config);
                log::info!("  Sticker applied successfully");
            }
            CFaceEffectType::MosaicStroke => {
                // Apply mosaic first (inside the face area)
                let mosaic_config = crate::effect::mosaic::MosaicEffect {
                    block_size: config_ref.mosaic_block_size,
                    intensity: config_ref.mosaic_intensity,
                };
                if let Err(e) = crate::effect::mosaic::MosaicEffect::apply(
                    &mut dyn_image,
                    &face_areas,
                    &mosaic_config,
                ) {
                    log::error!("Failed to apply mosaic in MosaicStroke: {}", e);
                    return ChamaError::ImageProcessError;
                }

                // Then apply stroke (border around the face area)
                let stroke_config = crate::effect::stroke::StrokeEffect {
                    thickness: config_ref.stroke_thickness,
                    color: (
                        config_ref.stroke_color_r,
                        config_ref.stroke_color_g,
                        config_ref.stroke_color_b,
                        config_ref.stroke_color_a,
                    ),
                };
                if let Err(e) = crate::effect::stroke::StrokeEffect::apply(
                    &mut dyn_image,
                    &face_areas,
                    &stroke_config,
                ) {
                    log::error!("Failed to apply stroke in MosaicStroke: {}", e);
                    return ChamaError::ImageProcessError;
                }
                log::info!("  MosaicStroke applied successfully");
            }
            CFaceEffectType::None => {}
        }
    }

    // Step 3: Apply theme (if theme_name provided)
    if !config_ref.theme_name.is_null() {
        let theme_name_str = unsafe {
            match CStr::from_ptr(config_ref.theme_name).to_str() {
                Ok(s) if !s.is_empty() => s,
                _ => {
                    log::info!("  No theme specified, skipping theme application");
                    // Skip theme if empty string
                    ""
                }
            }
        };

        if !theme_name_str.is_empty() {
            log::info!("  Applying theme: {}", theme_name_str);

            let params_json = if !config_ref.theme_params_json.is_null() {
                unsafe {
                    CStr::from_ptr(config_ref.theme_params_json)
                        .to_str()
                        .unwrap_or("{}")
                }
            } else {
                "{}"
            };

            let font_path = if !config_ref.font_path.is_null() {
                unsafe { CStr::from_ptr(config_ref.font_path).to_str().unwrap_or("") }
            } else {
                ""
            };

            // Save intermediate image to temp file, apply theme, load back
            // This is necessary because theme application uses PackedImage
            let temp_path = format!("{}.temp_face_effect.jpg", output_path_str);

            // Save face-effected image to temp
            if let Err(e) = dyn_image.save(&temp_path) {
                log::error!("Failed to save temp image: {}", e);
                return ChamaError::ImageProcessError;
            }

            // Apply theme to temp image, but read EXIF from original image
            // (temp file doesn't have EXIF data after sticker/mosaic processing)
            // Convert CScaleConfig to core ScaleConfig for theme application
            let core_scale_config = if config_ref.scale_config.mode != CScaleMode::None {
                Some(crate::scale_config::ScaleConfig {
                    mode: match config_ref.scale_config.mode {
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
                    },
                    value: config_ref.scale_config.value,
                    sub_value: config_ref.scale_config.sub_value,
                    scale_value: config_ref.scale_config.scale_value as f32,
                })
            } else {
                None
            };

            let theme_result = export_final_impl_with_exif_source(
                &temp_path,     // Image with face effects
                image_path_str, // Original image for EXIF data
                output_path_str,
                theme_name_str,
                params_json,
                font_path,
                config_ref.font_weight,
                core_scale_config,
            );

            // Clean up temp file
            let _ = std::fs::remove_file(&temp_path);

            match theme_result {
                Ok(_) => {
                    log::info!("  Theme applied successfully");
                    // Theme already saved to output_path, we're done
                    log::info!("✅ Combined export completed successfully");
                    return ChamaError::Success;
                }
                Err(e) => {
                    log::error!("Failed to apply theme: {}", e);
                    return ChamaError::ImageProcessError;
                }
            }
        }
    }

    // Step 4: Apply scaling (if scale_mode != None)
    let final_image = if config_ref.scale_config.mode != CScaleMode::None {
        log::info!("  Applying scale: {:?}", config_ref.scale_config.mode);
        apply_scale_to_image(&dyn_image, &config_ref.scale_config)
    } else {
        dyn_image
    };

    // Step 5: Save with specified format and quality (if no theme was applied)
    log::info!(
        "  Saving with format: {:?}, quality: {}",
        config_ref.output_format,
        config_ref.quality
    );

    let save_result = match config_ref.output_format {
        COutputFormat::Jpeg => {
            use image::codecs::jpeg::JpegEncoder;
            let file = match std::fs::File::create(output_path_str) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create output file: {}", e);
                    return ChamaError::ImageProcessError;
                }
            };
            let mut encoder = JpegEncoder::new_with_quality(file, config_ref.quality);
            encoder.encode_image(&final_image)
        }
        COutputFormat::Png => final_image.save(output_path_str).map_err(|e| e.into()),
        COutputFormat::Webp => {
            // WebP support via image crate
            use image::ImageFormat;
            let file = match std::fs::File::create(output_path_str) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create output file: {}", e);
                    return ChamaError::ImageProcessError;
                }
            };
            let mut buf_writer = std::io::BufWriter::new(file);
            final_image.write_to(&mut buf_writer, ImageFormat::WebP)
        }
    };

    match save_result {
        Ok(_) => {
            log::info!("✅ Combined export completed successfully");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to save image: {}", e);
            ChamaError::ImageProcessError
        }
    }
}

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

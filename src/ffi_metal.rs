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
    // For final export, always load full resolution
    let _dyn_image = image::open(image_path).map_err(PreviewError::ImageLoad)?;

    log::info!(
        "✅ Loaded full resolution image: {}x{}",
        _dyn_image.width(),
        _dyn_image.height()
    );

    // The rest is the same as preview generation
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

    // 6. Create a PackedImage for theme application
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

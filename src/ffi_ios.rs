/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
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
use std::path::{Path, PathBuf};

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
        load_image_with_heif_support(Path::new(image_path)).map_err(PreviewError::ImageLoad)?
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
        // For now normal preview doesn't draw face effect.
        configured_faces: Vec::with_capacity(0),
        lut_id: None, // iOS FFI doesn't use LUT yet
        crop_rotate: crate::effect::crop_rotate::CropRotateTransform::default(),
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
        None,  // Use default scale config
        None,  // Use default export config (WebP)
        false, // get_alt_fnumber - default to false
        false, // use_35mm_focal_length - default to false
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
    output_format_config: Option<COutputFormatConfig>,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
) -> Result<(), PreviewError> {
    export_final_impl_with_exif_source_and_override(
        image_path,
        exif_source_path,
        output_path,
        theme_name,
        params_json,
        font_path,
        font_weight,
        scale_config,
        output_format_config,
        get_alt_fnumber,
        use_35mm_focal_length,
        None,
    )
}

fn export_final_impl_with_exif_source_and_override(
    image_path: &str,
    exif_source_path: &str,
    output_path: &str,
    theme_name: &str,
    params_json: &str,
    font_path: &str,
    font_weight: u32,
    scale_config: Option<crate::scale_config::ScaleConfig>,
    output_format_config: Option<COutputFormatConfig>,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
    exif_override_json: Option<&str>,
) -> Result<(), PreviewError> {
    // Verify image file exists and is readable before proceeding
    // Note: We intentionally do NOT load the full image here, as it will be loaded
    // again from image_bytes in PackedImage::get_image(). Double-loading wastes ~100MB.
    {
        let img_path = Path::new(image_path);
        if !img_path.exists() {
            log::error!("Image file does not exist: {}", image_path);
            return Err(PreviewError::ImageLoad(image::ImageError::IoError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "Image file not found"),
            )));
        }
        let metadata = std::fs::metadata(img_path).map_err(PreviewError::IoError)?;
        log::info!(
            "✅ Image file verified: {} ({} bytes)",
            image_path,
            metadata.len()
        );
    }

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

    // Apply import configuration settings
    if get_alt_fnumber {
        view_exif.replace_with_fnumber_alt_when_invalid();
    }
    if use_35mm_focal_length {
        view_exif.use_35mm_focal_length(&original_exif);
    }

    // Apply EXIF overrides from user edits (if provided)
    if let Some(override_json) = exif_override_json {
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
        configured_faces: Vec::with_capacity(0), // todo - check is this right?
        lut_id: None,                            // iOS FFI doesn't use LUT yet
        crop_rotate: crate::effect::crop_rotate::CropRotateTransform::default(),
    };

    // 7. Apply theme with custom scale config if provided
    let mut export_config = crate::export_config::ExportConfig::default();

    // Apply custom scale config if provided
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

    // Apply custom export config (output format and quality) if provided
    if let Some(output_config) = output_format_config {
        // Set output format
        export_config.output_format = crate::export_config::output_format::OutputFormat {
            ext: match output_config.output_format {
                COutputFormat::Jpeg => crate::export_config::output_format::OutputExtension::Jpeg,
                COutputFormat::Png => {
                    crate::export_config::output_format::OutputExtension::PngOptimized
                }
                COutputFormat::Webp => crate::export_config::output_format::OutputExtension::Webp,
            },
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
    let output_path_str = unsafe {
        match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in output path");
                return ChamaError::InvalidPath;
            }
        }
    };

    let theme_name_str = unsafe {
        match CStr::from_ptr(theme_name).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in theme name");
                return ChamaError::InvalidTheme;
            }
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
            match e {
                PreviewError::InvalidTheme => ChamaError::InvalidTheme,
                PreviewError::InvalidFont => ChamaError::InvalidFont,
                PreviewError::InvalidParameters(_) => ChamaError::InvalidParameters,
                PreviewError::ImageLoad(_) => ChamaError::ImageLoadError,
                PreviewError::ImageProcess(_) => ChamaError::ImageProcessError,
                PreviewError::ExifError => ChamaError::ExifError,
                PreviewError::IoError(_) => ChamaError::InvalidPath,
            }
        }
    };

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_image_path);

    result
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
        None,  // No custom scale config - use default
        None,  // Use default export config (WebP)
        false, // get_alt_fnumber - default to false
        false, // use_35mm_focal_length - default to false
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

/// Apply theme to image with separate EXIF source, custom scale config, and export config
///
/// Same as `chama_optics_apply_theme_with_exif` but allows specifying custom scale settings
/// AND output format (JPEG/PNG/WebP) with quality settings.
/// Use this function when you need to control output image size and format during theme application.
///
/// # Parameters
/// - `image_path`: Path to image to apply theme to (may be modified)
/// - `exif_source_path`: Path to the original image for reading EXIF data
/// - `output_path`: Path for the output file (extension should match export format)
/// - `theme_name`: Name of the theme to apply
/// - `params_json`: Theme parameters as JSON string
/// - `font_path`: Path to font file
/// - `font_weight`: Font weight (100-900)
/// - `scale_config`: Pointer to CScaleConfig for custom scaling (pass null for default 4K scaling)
/// - `output_format_config`: Pointer to COutputFormatConfig for format/quality (pass null for default WebP 90)
#[unsafe(no_mangle)]
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe extern "C" fn chama_optics_apply_theme_with_exif_scale_and_export(
    image_path: *const c_char,
    exif_source_path: *const c_char,
    output_path: *const c_char,
    theme_name: *const c_char,
    params_json: *const c_char,
    font_path: *const c_char,
    font_weight: u32,
    scale_config: *const CScaleConfig,
    output_format_config: *const COutputFormatConfig,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
) -> ChamaError {
    // Wrap entire function body in catch_unwind to prevent panics from
    // unwinding across the FFI boundary (which causes SIGABRT on Android)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        chama_optics_apply_theme_with_exif_scale_and_export_impl(
            image_path,
            exif_source_path,
            output_path,
            theme_name,
            params_json,
            font_path,
            font_weight,
            scale_config,
            output_format_config,
            get_alt_fnumber,
            use_35mm_focal_length,
        )
    }));

    match result {
        Ok(error_code) => error_code,
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            log::error!("Caught panic in FFI: {}", msg);
            ChamaError::ImageProcessError
        }
    }
}

unsafe fn chama_optics_apply_theme_with_exif_scale_and_export_impl(
    image_path: *const c_char,
    exif_source_path: *const c_char,
    output_path: *const c_char,
    theme_name: *const c_char,
    params_json: *const c_char,
    font_path: *const c_char,
    font_weight: u32,
    scale_config: *const CScaleConfig,
    output_format_config: *const COutputFormatConfig,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
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

    // Convert COutputFormatConfig if provided
    let export_config_option = if output_format_config.is_null() {
        None
    } else {
        let config_ref = unsafe { &*output_format_config };
        Some(*config_ref)
    };

    match export_final_impl_with_exif_source(
        image_path_str,
        exif_source_str,
        output_path_str,
        theme_name_str,
        params_json_str,
        font_path_str,
        font_weight,
        core_scale_config,
        export_config_option,
        get_alt_fnumber,
        use_35mm_focal_length,
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
// Theme with EXIF Override (for user-edited EXIF data)
// ============================================================================

/// Apply theme with EXIF override JSON
/// Same as chama_optics_apply_theme_with_exif_scale_and_export but accepts
/// an additional exif_override_json parameter to override EXIF fields from user edits.
#[unsafe(no_mangle)]
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe extern "C" fn chama_optics_apply_theme_with_exif_override(
    image_path: *const c_char,
    exif_source_path: *const c_char,
    output_path: *const c_char,
    theme_name: *const c_char,
    params_json: *const c_char,
    font_path: *const c_char,
    font_weight: u32,
    scale_config: *const CScaleConfig,
    output_format_config: *const COutputFormatConfig,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
    exif_override_json: *const c_char,
) -> ChamaError {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if image_path.is_null() || output_path.is_null() || theme_name.is_null() {
            return ChamaError::InvalidPath;
        }

        let image_path_str = match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        };

        let exif_source_str = if exif_source_path.is_null() {
            image_path_str
        } else {
            match CStr::from_ptr(exif_source_path).to_str() {
                Ok(s) => s,
                Err(_) => image_path_str,
            }
        };

        let output_path_str = match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidPath,
        };

        let theme_name_str = match CStr::from_ptr(theme_name).to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidTheme,
        };

        let params_json_str = if params_json.is_null() {
            "{}"
        } else {
            CStr::from_ptr(params_json).to_str().unwrap_or("{}")
        };

        let font_path_str = if font_path.is_null() {
            ""
        } else {
            CStr::from_ptr(font_path).to_str().unwrap_or("")
        };

        let exif_override_str = if exif_override_json.is_null() {
            None
        } else {
            match CStr::from_ptr(exif_override_json).to_str() {
                Ok(s) if !s.is_empty() => Some(s),
                _ => None,
            }
        };

        // Convert CScaleConfig
        let core_scale_config = if scale_config.is_null() {
            None
        } else {
            let config_ref = &*scale_config;
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

        let export_config_option = if output_format_config.is_null() {
            None
        } else {
            let config_ref = &*output_format_config;
            Some(*config_ref)
        };

        log::info!(
            "Applying theme with EXIF override: image={}, exif_source={}, override={}",
            image_path_str,
            exif_source_str,
            exif_override_str.is_some()
        );

        match export_final_impl_with_exif_source_and_override(
            image_path_str,
            exif_source_str,
            output_path_str,
            theme_name_str,
            params_json_str,
            font_path_str,
            font_weight,
            core_scale_config,
            export_config_option,
            get_alt_fnumber,
            use_35mm_focal_length,
            exif_override_str,
        ) {
            Ok(_) => {
                log::info!("✅ Theme applied successfully with EXIF override");
                ChamaError::Success
            }
            Err(e) => {
                log::error!("Failed to apply theme with EXIF override: {}", e);
                match e {
                    PreviewError::InvalidTheme => ChamaError::InvalidTheme,
                    PreviewError::InvalidFont => ChamaError::InvalidFont,
                    PreviewError::ImageLoad(_) => ChamaError::ImageLoadError,
                    _ => ChamaError::ImageProcessError,
                }
            }
        }
    }));

    match result {
        Ok(error_code) => error_code,
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            log::error!("Caught panic in FFI (EXIF override): {}", msg);
            ChamaError::ImageProcessError
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

/// Theme export configuration
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct COutputFormatConfig {
    /// Output format (JPEG, PNG, or WebP)
    pub output_format: COutputFormat,
    /// Quality (1-100 for JPEG/WebP, ignored for PNG)
    pub quality: u8,
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

    // EXIF override settings
    pub get_alt_fnumber: bool,
    pub use_35mm_focal_length: bool,
    pub exif_override_json: *const c_char,
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
            use image::codecs::webp::WebPEncoder;
            let file = match std::fs::File::create(output_path_str) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create output file: {}", e);
                    return ChamaError::ImageProcessError;
                }
            };
            // WebPEncoder only supports lossless encoding currently
            // Convert image to RGBA bytes
            let rgba = scaled_image.to_rgba8();
            WebPEncoder::new_lossless(file).encode(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
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
#[cfg(any(target_os = "ios", target_os = "android"))]
pub unsafe extern "C" fn chama_export_combined(
    image_path: *const c_char,
    output_path: *const c_char,
    face_rects: *const CFaceRect,
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

            // Parse EXIF override JSON if provided
            let exif_override_str = if !config_ref.exif_override_json.is_null() {
                unsafe {
                    CStr::from_ptr(config_ref.exif_override_json)
                        .to_str()
                        .ok()
                        .filter(|s| !s.is_empty())
                }
            } else {
                None
            };

            let theme_result = export_final_impl_with_exif_source_and_override(
                &temp_path,     // Image with face effects
                image_path_str, // Original image for EXIF data
                output_path_str,
                theme_name_str,
                params_json,
                font_path,
                config_ref.font_weight,
                core_scale_config,
                None, // Use default export config
                config_ref.get_alt_fnumber,
                config_ref.use_35mm_focal_length,
                exif_override_str,
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
            // WebP support (lossless)
            use image::codecs::webp::WebPEncoder;
            let file = match std::fs::File::create(output_path_str) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create output file: {}", e);
                    return ChamaError::ImageProcessError;
                }
            };
            // WebPEncoder only supports lossless encoding currently
            // Convert image to RGBA bytes
            let rgba = final_image.to_rgba8();
            WebPEncoder::new_lossless(file).encode(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
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

// ============================================================================
// LUT (Color Grading) Functions
// ============================================================================

use std::sync::Mutex;

lazy_static::lazy_static! {
    /// Global LUT storage for iOS
    /// Uses a Mutex to allow thread-safe access from Swift
    static ref LUT_STORAGE: Mutex<crate::effect::lut_storage::LutStorage> = {
        // let mut storage = crate::effect::lut_storage::LutStorage::new();
        // Set iOS-specific storage path
        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            let mut storage = crate::effect::lut_storage::LutStorage::new();
            if let Some(docs_dir) = dirs::document_dir() {
                storage.storage_directory = docs_dir.join("ChamaOptics").join("luts");
                let _ = storage.ensure_directory();
            }
            Mutex::new(storage)
        }
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            let storage = crate::effect::lut_storage::LutStorage::new();
            Mutex::new(storage)
        }
    };
}

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

/// Set the LUT storage directory path
/// Must be called before chama_lut_init() for the path to take effect
///
/// # Safety
/// - `path` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_set_storage_path(path: *const c_char) {
    if path.is_null() {
        log::warn!("chama_lut_set_storage_path: null path provided");
        return;
    }

    let path_str = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(e) => {
                log::error!("chama_lut_set_storage_path: invalid UTF-8: {}", e);
                return;
            }
        }
    };

    log::info!("Setting LUT storage path to: {}", path_str);

    if let Ok(mut storage) = LUT_STORAGE.lock() {
        storage.storage_directory = std::path::PathBuf::from(path_str);
        if let Err(e) = storage.ensure_directory() {
            log::error!("Failed to create LUT storage directory: {}", e);
        }
    }
}

/// Initialize LUT storage and load existing LUTs
/// Call this on app startup after chama_lut_set_storage_path()
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_init() {
    log::info!("Initializing LUT storage for iOS");

    // Access storage to trigger lazy initialization
    if let Ok(mut storage) = LUT_STORAGE.lock() {
        // Verify all existing LUTs
        storage.verify_all_luts();
        log::info!("LUT storage initialized with {} LUTs", storage.luts.len());
    }
}

/// Get list of available LUTs as JSON
/// Returns JSON array of LUT items
///
/// # Safety
/// - Returned pointer must be freed with `chama_free_string`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_get_list() -> *mut c_char {
    let storage = match LUT_STORAGE.lock() {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to lock LUT storage: {}", e);
            return std::ptr::null_mut();
        }
    };

    let luts: Vec<serde_json::Value> = storage
        .luts
        .iter()
        .map(|lut| {
            serde_json::json!({
                "id": lut.id.to_string(),
                "name": lut.name,
                "lut_type": match lut.lut_type {
                    crate::effect::lut_storage::StoredLutType::Unknown => 0,
                    crate::effect::lut_storage::StoredLutType::Lut1D => 1,
                    crate::effect::lut_storage::StoredLutType::Lut3D => 2,
                },
                "size_info": lut.lut_size_info,
                "file_missing": lut.file_missing,
                "hash_mismatch": lut.hash_mismatch,
            })
        })
        .collect();

    let json = match serde_json::to_string(&luts) {
        Ok(j) => j,
        Err(e) => {
            log::error!("Failed to serialize LUT list: {}", e);
            return std::ptr::null_mut();
        }
    };

    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Add a new LUT from a .cube file
/// Returns the UUID of the new LUT as a string, or NULL on failure
///
/// # Parameters
/// - `name`: Display name for the LUT
/// - `source_path`: Path to the .cube file
///
/// # Safety
/// - Returned pointer must be freed with `chama_free_string`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_add(
    name: *const c_char,
    source_path: *const c_char,
) -> *mut c_char {
    if name.is_null() || source_path.is_null() {
        log::error!("chama_lut_add: null parameter");
        return std::ptr::null_mut();
    }

    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let path_str = match unsafe { CStr::from_ptr(source_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    log::info!("Adding LUT '{}' from path: {}", name_str, path_str);

    let mut storage = match LUT_STORAGE.lock() {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to lock LUT storage: {}", e);
            return std::ptr::null_mut();
        }
    };

    match storage.add_lut(name_str.to_string(), Path::new(path_str)) {
        Ok(uuid) => {
            log::info!("Successfully added LUT with ID: {}", uuid);
            match CString::new(uuid.to_string()) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(e) => {
            log::error!("Failed to add LUT: {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Remove a LUT by its UUID
///
/// # Parameters
/// - `lut_id`: UUID string of the LUT to remove
///
/// # Returns
/// - true if successful, false otherwise
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_remove(lut_id: *const c_char) -> bool {
    if lut_id.is_null() {
        return false;
    }

    let id_str = match unsafe { CStr::from_ptr(lut_id) }.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let uuid = match uuid::Uuid::parse_str(id_str) {
        Ok(u) => u,
        Err(e) => {
            log::error!("Invalid UUID: {}", e);
            return false;
        }
    };

    let mut storage = match LUT_STORAGE.lock() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let result = storage.remove_lut(uuid);
    if result {
        log::info!("Removed LUT: {}", id_str);
    }
    result
}

/// Apply a LUT to an image
///
/// # Parameters
/// - `lut_id`: UUID string of the LUT to apply
/// - `image_path`: Path to the input image
/// - `output_path`: Path for the output image
///
/// # Returns
/// - ChamaError::Success on success
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_apply(
    lut_id: *const c_char,
    image_path: *const c_char,
    output_path: *const c_char,
) -> ChamaError {
    if lut_id.is_null() || image_path.is_null() || output_path.is_null() {
        return ChamaError::InvalidPath;
    }

    let id_str = match unsafe { CStr::from_ptr(lut_id) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let image_path_str = match unsafe { CStr::from_ptr(image_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let output_path_str = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let uuid = match uuid::Uuid::parse_str(id_str) {
        Ok(u) => u,
        Err(e) => {
            log::error!("Invalid LUT UUID: {}", e);
            return ChamaError::InvalidParameters;
        }
    };

    log::info!("Applying LUT {} to image: {}", id_str, image_path_str);

    // Load image
    let mut dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

    // Apply LUT
    {
        let mut storage = match LUT_STORAGE.lock() {
            Ok(s) => s,
            Err(_) => return ChamaError::Unknown,
        };

        if !storage.apply_lut_to_image(uuid, &mut dyn_image) {
            log::error!("Failed to apply LUT");
            return ChamaError::ImageProcessError;
        }
    }

    // Save output
    if let Err(e) = dyn_image.save(output_path_str) {
        log::error!("Failed to save image: {}", e);
        return ChamaError::ImageProcessError;
    }

    log::info!("Successfully applied LUT and saved to: {}", output_path_str);
    ChamaError::Success
}

/// Apply a LUT to an image and return as JPEG with specified quality
///
/// # Parameters
/// - `lut_id`: UUID string of the LUT to apply (can be NULL for no LUT)
/// - `image_path`: Path to the input image
/// - `output_path`: Path for the output image
/// - `output_format`: Output format (0=JPEG, 1=PNG, 2=WebP)
/// - `quality`: Quality for JPEG/WebP (1-100)
///
/// # Returns
/// - ChamaError::Success on success
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_apply_with_format(
    lut_id: *const c_char,
    image_path: *const c_char,
    output_path: *const c_char,
    output_format: COutputFormat,
    quality: u8,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() {
        return ChamaError::InvalidPath;
    }

    let image_path_str = match unsafe { CStr::from_ptr(image_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let output_path_str = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    log::info!("Applying LUT to image: {}", image_path_str);

    // Load image
    let mut dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

    // Apply LUT if specified
    if !lut_id.is_null() {
        let id_str = match unsafe { CStr::from_ptr(lut_id) }.to_str() {
            Ok(s) => s,
            Err(_) => return ChamaError::InvalidParameters,
        };

        if !id_str.is_empty() {
            let uuid = match uuid::Uuid::parse_str(id_str) {
                Ok(u) => u,
                Err(e) => {
                    log::error!("Invalid LUT UUID: {}", e);
                    return ChamaError::InvalidParameters;
                }
            };

            let mut storage = match LUT_STORAGE.lock() {
                Ok(s) => s,
                Err(_) => return ChamaError::Unknown,
            };

            if !storage.apply_lut_to_image(uuid, &mut dyn_image) {
                log::error!("Failed to apply LUT");
                return ChamaError::ImageProcessError;
            }
            log::info!("LUT applied successfully");
        }
    }

    // Save with specified format
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
            encoder.encode_image(&dyn_image)
        }
        COutputFormat::Png => dyn_image.save(output_path_str).map_err(|e| e.into()),
        COutputFormat::Webp => {
            use image::codecs::webp::WebPEncoder;
            let file = match std::fs::File::create(output_path_str) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create output file: {}", e);
                    return ChamaError::ImageProcessError;
                }
            };
            let rgba = dyn_image.to_rgba8();
            WebPEncoder::new_lossless(file).encode(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
        }
    };

    match save_result {
        Ok(_) => {
            log::info!("Successfully saved to: {}", output_path_str);
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to save image: {}", e);
            ChamaError::ImageProcessError
        }
    }
}

/// Get LUT storage directory path
/// Returns the path where LUT files are stored
///
/// # Safety
/// - Returned pointer must be freed with `chama_free_string`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_get_storage_path() -> *mut c_char {
    let storage = match LUT_STORAGE.lock() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let path_str = storage.storage_directory.to_string_lossy().to_string();
    match CString::new(path_str) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Verify all LUTs (check file integrity)
/// Updates file_missing and hash_mismatch flags
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_verify_all() {
    if let Ok(mut storage) = LUT_STORAGE.lock() {
        storage.verify_all_luts();
        log::info!("Verified {} LUTs", storage.luts.len());
    }
}

/// Save LUT storage state to disk (persist LUT list)
/// Call this after adding/removing LUTs
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_save_state() -> bool {
    let storage = match LUT_STORAGE.lock() {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Get the storage directory
    let state_file = storage.storage_directory.join("lut_state.json");

    // Serialize storage to JSON
    let json = match serde_json::to_string_pretty(&*storage) {
        Ok(j) => j,
        Err(e) => {
            log::error!("Failed to serialize LUT state: {}", e);
            return false;
        }
    };

    // Write to file
    match std::fs::write(&state_file, json) {
        Ok(_) => {
            log::info!("Saved LUT state to: {:?}", state_file);
            true
        }
        Err(e) => {
            log::error!("Failed to save LUT state: {}", e);
            false
        }
    }
}

/// Load LUT storage state from disk
/// Call this on app startup after chama_lut_init
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_load_state() -> bool {
    let mut storage = match LUT_STORAGE.lock() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let state_file = storage.storage_directory.join("lut_state.json");

    if !state_file.exists() {
        log::info!("No LUT state file found, starting fresh");
        return true;
    }

    // Read file
    let json = match std::fs::read_to_string(&state_file) {
        Ok(j) => j,
        Err(e) => {
            log::error!("Failed to read LUT state: {}", e);
            return false;
        }
    };

    // Deserialize
    match serde_json::from_str::<crate::effect::lut_storage::LutStorage>(&json) {
        Ok(loaded_storage) => {
            // Keep the storage directory from current instance
            let current_dir = storage.storage_directory.clone();
            *storage = loaded_storage;
            storage.storage_directory = current_dir;

            // Update file paths to use current storage directory (fixes iOS container UUID changes)
            storage.update_file_paths();

            // Verify all LUTs
            storage.verify_all_luts();

            log::info!("Loaded LUT state: {} LUTs", storage.luts.len());
            true
        }
        Err(e) => {
            log::error!("Failed to parse LUT state: {}", e);
            false
        }
    }
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
// Color Adjustments FFI
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

    let image_path_str = match unsafe { CStr::from_ptr(image_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let output_path_str = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let adj = unsafe { &*adjustments };

    log::info!(
        "Applying color adjustments to image: {} (exposure={}, contrast={}, saturation={})",
        image_path_str,
        adj.exposure,
        adj.contrast,
        adj.saturation
    );

    // Load image
    let mut dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

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
            encoder.encode_image(&dyn_image)
        }
        COutputFormat::Png => dyn_image.save(output_path_str).map_err(|e| e.into()),
        COutputFormat::Webp => {
            use image::codecs::webp::WebPEncoder;
            let file = match std::fs::File::create(output_path_str) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create output file: {}", e);
                    return ChamaError::ImageProcessError;
                }
            };
            let rgba = dyn_image.to_rgba8();
            WebPEncoder::new_lossless(file).encode(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
        }
    };

    match save_result {
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

    let image_path_str = match unsafe { CStr::from_ptr(image_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let output_path_str = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let json_str = match unsafe { CStr::from_ptr(adjustments_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidParameters,
    };

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
        "Applying color adjustments (JSON) to image: {} (exposure={}, contrast={}, saturation={})",
        image_path_str,
        color_adj.exposure,
        color_adj.contrast,
        color_adj.saturation
    );

    // Load image
    let mut dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

    // Apply adjustments
    color_adj.apply(&mut dyn_image);

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
            encoder.encode_image(&dyn_image)
        }
        COutputFormat::Png => dyn_image.save(output_path_str).map_err(|e| e.into()),
        COutputFormat::Webp => {
            use image::codecs::webp::WebPEncoder;
            let file = match std::fs::File::create(output_path_str) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to create output file: {}", e);
                    return ChamaError::ImageProcessError;
                }
            };
            let rgba = dyn_image.to_rgba8();
            WebPEncoder::new_lossless(file).encode(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
        }
    };

    match save_result {
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

// ============================================================================
// Cheki (Polaroid) Decoration Export
// ============================================================================

/// Apply cheki (polaroid) decoration to an image and save the result.
///
/// This performs the full cheki export pipeline:
/// 1. Load image with EXIF orientation
/// 2. Apply crop/rotate transform (if provided in JSON)
/// 3. Apply color adjustments + LUT (if provided)
/// 4. Apply cheki decoration (border, stickers, text, date stamp)
/// 5. Save result
///
/// # Parameters
/// - `image_path`: Path to the source image
/// - `output_path`: Path to save the decorated image
/// - `cheki_json`: JSON string with ChekiDecoration configuration
/// - `sticker_dir`: Directory containing sticker images (sticker filenames are UUIDs)
/// - `crop_rotate_json`: Optional JSON string with CropRotateTransform (null = no transform)
/// - `color_adjustments_json`: Optional JSON string with color adjustments (null = none)
/// - `lut_id`: Optional LUT ID to apply (null = none)
/// - `output_format_config`: Optional output format config (null = default based on extension)
///
/// # Safety
/// - All string pointers must be valid null-terminated C strings or null
/// - `output_format_config` must be a valid pointer or null
#[unsafe(no_mangle)]
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe extern "C" fn chama_export_cheki(
    image_path: *const c_char,
    output_path: *const c_char,
    cheki_json: *const c_char,
    sticker_dir: *const c_char,
    crop_rotate_json: *const c_char,
    color_adjustments_json: *const c_char,
    lut_id: *const c_char,
    output_format_config: *const COutputFormatConfig,
) -> ChamaError {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        chama_export_cheki_impl(
            image_path,
            output_path,
            cheki_json,
            sticker_dir,
            crop_rotate_json,
            color_adjustments_json,
            lut_id,
            output_format_config,
        )
    }));

    match result {
        Ok(error_code) => error_code,
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            log::error!("Caught panic in chama_export_cheki: {}", msg);
            ChamaError::ImageProcessError
        }
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn chama_export_cheki_impl(
    image_path: *const c_char,
    output_path: *const c_char,
    cheki_json: *const c_char,
    sticker_dir: *const c_char,
    crop_rotate_json: *const c_char,
    color_adjustments_json: *const c_char,
    lut_id: *const c_char,
    output_format_config: *const COutputFormatConfig,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || cheki_json.is_null() {
        return ChamaError::InvalidPath;
    }

    let image_path_str = match CStr::from_ptr(image_path).to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let output_path_str = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidPath,
    };

    let cheki_json_str = match CStr::from_ptr(cheki_json).to_str() {
        Ok(s) => s,
        Err(_) => return ChamaError::InvalidParameters,
    };

    let sticker_dir_str = if sticker_dir.is_null() {
        ""
    } else {
        CStr::from_ptr(sticker_dir).to_str().unwrap_or("")
    };

    log::info!("Cheki export pipeline started:");
    log::info!("  Input: {}", image_path_str);
    log::info!("  Output: {}", output_path_str);
    log::info!("  Sticker dir: {}", sticker_dir_str);

    // Step 1: Parse ChekiDecoration from JSON
    log::info!(
        "  Cheki JSON (first 500): {}",
        &cheki_json_str[..cheki_json_str.len().min(500)]
    );
    let decoration: crate::effect::cheki::ChekiDecoration =
        match serde_json::from_str(cheki_json_str) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to parse ChekiDecoration JSON: {}", e);
                log::error!("  Full JSON: {}", cheki_json_str);
                return ChamaError::InvalidParameters;
            }
        };

    // Step 2: Load image with HEIF support and apply EXIF orientation
    let mut dyn_image = match load_image_with_heif_support(Path::new(image_path_str)) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };

    // Apply EXIF orientation
    {
        use exif::{In, Tag};
        let orientation = if let Ok(f) = std::fs::File::open(image_path_str) {
            let mut buf_reader = std::io::BufReader::new(f);
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
        } else {
            image::metadata::Orientation::NoTransforms
        };
        dyn_image.apply_orientation(orientation);
    }

    log::info!(
        "  Image size after orientation: {}x{}",
        dyn_image.width(),
        dyn_image.height()
    );

    // Step 3: Apply crop/rotate transform (if provided)
    if !crop_rotate_json.is_null() {
        let crop_rotate_str = CStr::from_ptr(crop_rotate_json).to_str().unwrap_or("{}");
        if !crop_rotate_str.is_empty() && crop_rotate_str != "{}" {
            match serde_json::from_str::<crate::effect::crop_rotate::CropRotateTransform>(
                crop_rotate_str,
            ) {
                Ok(transform) => {
                    if !transform.is_identity() {
                        dyn_image = transform.apply(&dyn_image);
                        log::info!(
                            "  After crop/rotate: {}x{}",
                            dyn_image.width(),
                            dyn_image.height()
                        );
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse crop/rotate JSON: {}", e);
                }
            }
        }
    }

    // Step 4: Apply color adjustments (if provided)
    log::info!(
        "  color_adjustments_json is_null={}, lut_id is_null={}",
        color_adjustments_json.is_null(),
        lut_id.is_null()
    );
    if !color_adjustments_json.is_null() {
        let adjustments_str = CStr::from_ptr(color_adjustments_json)
            .to_str()
            .unwrap_or("{}");
        if !adjustments_str.is_empty() && adjustments_str != "{}" {
            match serde_json::from_str::<crate::effect::color_adjustments::ColorAdjustments>(
                adjustments_str,
            ) {
                Ok(adjustments) => {
                    if !adjustments.is_identity() {
                        adjustments.apply(&mut dyn_image);
                        log::info!("  Color adjustments applied");
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse color adjustments JSON: {}", e);
                }
            }
        }
    }

    // Step 5: Apply LUT (if provided)
    if !lut_id.is_null() {
        let lut_id_str = CStr::from_ptr(lut_id).to_str().unwrap_or("");
        if !lut_id_str.is_empty() {
            if let Ok(uuid) = uuid::Uuid::parse_str(lut_id_str) {
                let mut storage = match LUT_STORAGE.lock() {
                    Ok(s) => s,
                    Err(_) => {
                        log::warn!("Failed to lock LUT storage");
                        return ChamaError::Unknown;
                    }
                };
                if storage.apply_lut_to_image(uuid, &mut dyn_image) {
                    log::info!("  LUT applied: {}", lut_id_str);
                } else {
                    log::warn!("  LUT not found or failed: {}", lut_id_str);
                }
            }
        }
    }

    // Step 6: Build sticker storage from directory
    let sticker_storage = build_sticker_storage_from_dir_and_json(sticker_dir_str, &decoration);

    // Step 7: Apply cheki decoration
    let result = crate::effect::cheki_renderer::apply_cheki_decoration(
        dyn_image,
        &decoration,
        &sticker_storage,
    );

    // Step 8: Save result
    if output_format_config.is_null() {
        match result.save(output_path_str) {
            Ok(_) => {
                log::info!("✅ Cheki export completed: {}", output_path_str);
                ChamaError::Success
            }
            Err(e) => {
                log::error!("Failed to save cheki result: {}", e);
                ChamaError::ImageProcessError
            }
        }
    } else {
        let config_ref = &*output_format_config;
        let output_format = crate::export_config::output_format::OutputFormat {
            ext: match config_ref.output_format {
                COutputFormat::Jpeg => crate::export_config::output_format::OutputExtension::Jpeg,
                COutputFormat::Png => {
                    crate::export_config::output_format::OutputExtension::PngOptimized
                }
                COutputFormat::Webp => crate::export_config::output_format::OutputExtension::Webp,
            },
            quality: config_ref.quality,
        };
        match output_format.save_image(&result, output_path_str) {
            Ok(_) => {
                log::info!("✅ Cheki export completed: {}", output_path_str);
                ChamaError::Success
            }
            Err(e) => {
                log::error!("Failed to save cheki result: {}", e);
                ChamaError::ImageProcessError
            }
        }
    }
}

/// Build a StickerStorage from a directory path and ChekiDecoration's sticker references.
/// Each sticker file is expected to be named by its UUID (e.g., "uuid.png") or "uuid_name.ext".
fn build_sticker_storage_from_dir_and_json(
    sticker_dir: &str,
    decoration: &crate::effect::cheki::ChekiDecoration,
) -> crate::effect::sticker_storage::StickerStorage {
    use crate::effect::sticker_storage::{StickerItem, StickerStorage};

    let mut storage = StickerStorage::new();

    if sticker_dir.is_empty() {
        return storage;
    }

    storage.storage_directory = PathBuf::from(sticker_dir);

    // Scan for sticker files referenced in the decoration
    for placed in &decoration.dice_stickers {
        let sticker_id = placed.sticker_id;
        let sticker_id_str = sticker_id.to_string();

        // If filename is provided (from mobile FFI), use it directly
        if let Some(ref fname) = placed.filename {
            let path = PathBuf::from(sticker_dir).join(fname);
            if path.exists() {
                let mut item = StickerItem::new(fname.clone(), path);
                item.id = sticker_id;
                item.is_character = true;
                storage.stickers.push(item);
                continue;
            }
        }

        // Fallback: Try common image extensions with UUID as filename
        let extensions = ["png", "jpg", "jpeg", "webp", "gif"];
        let mut found = false;

        for ext in &extensions {
            let filename = format!("{}.{}", sticker_id_str, ext);
            let path = PathBuf::from(sticker_dir).join(&filename);
            if path.exists() {
                let mut item = StickerItem::new(filename, path);
                item.id = sticker_id;
                item.is_character = true;
                storage.stickers.push(item);
                found = true;
                break;
            }
        }

        if !found {
            // Try scanning directory for files starting with the UUID
            if let Ok(entries) = std::fs::read_dir(sticker_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with(&sticker_id_str) {
                        let mut item = StickerItem::new(name, entry.path());
                        item.id = sticker_id;
                        item.is_character = true;
                        storage.stickers.push(item);
                        break;
                    }
                }
            }
        }
    }

    storage
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

// ============================================================================
// Functions moved from ffi.rs
// ============================================================================

use crate::core::ImageProcessor;

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

/// Opaque pointer to ChamaOptics instance (now using headless core)
pub struct ChamaOpticsHandle {
    processor: ImageProcessor,
}

// Note: chama_optics_init(), chama_optics_version(), and chama_optics_free_string()
// are now in ffi_apple.rs (shared platform-agnostic functions)

/// Create a new ChamaOptics instance
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_create() -> *mut ChamaOpticsHandle {
    log::info!("Creating ChamaOptics instance");
    Box::into_raw(Box::new(ChamaOpticsHandle {
        processor: ImageProcessor::new(),
    }))
}

/// Destroy a ChamaOptics instance
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_destroy(handle: *mut ChamaOpticsHandle) {
    if !handle.is_null() {
        unsafe {
            let _ = Box::from_raw(handle);
        }
        log::info!("ChamaOptics instance destroyed");
    }
}

/// Apply Mosaic effect to detected face areas
#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_apply_mosaic(
    handle: *mut ChamaOpticsHandle,
    face_rects: *const CFaceRect,
    face_count: usize,
    image_path: *const c_char,
    output_path: *const c_char,
    mosaic_size: u32,      // Size of mosaic blocks in pixels
    effect_intensity: f32, // 0.0 to 1.0, blend intensity
) -> bool {
    if handle.is_null() || image_path.is_null() || output_path.is_null() {
        return false;
    }

    unsafe {
        let image_str = match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in image path");
                return false;
            }
        };

        let output_str = match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in output path");
                return false;
            }
        };

        let handle_ref = &mut *handle;

        // Load image
        let path_buf = std::path::PathBuf::from(image_str);
        let mut dyn_image = match handle_ref.processor.load_image_direct(&path_buf) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image {}: {}", image_str, e);
                return false;
            }
        };

        // Collect face rectangles
        let mut face_areas = vec![];
        if !face_rects.is_null() && face_count > 0 {
            for i in 0..face_count {
                let face_rect = *face_rects.add(i);
                face_areas.push((face_rect.x, face_rect.y, face_rect.width, face_rect.height));
            }
        }

        // Create Mosaic effect config
        let mosaic_config = crate::effect::mosaic::MosaicEffect {
            block_size: mosaic_size,
            intensity: effect_intensity,
        };

        // Apply Mosaic effect - pass slice to apply() method
        log::info!(
            "Applying Mosaic effect: size={}, intensity={}, {} faces",
            mosaic_size,
            effect_intensity,
            face_areas.len()
        );

        if let Err(e) =
            crate::effect::mosaic::MosaicEffect::apply(&mut dyn_image, &face_areas, &mosaic_config)
        {
            log::error!("Failed to apply Mosaic effect: {}", e);
            return false;
        }

        // Save image
        let output_path_buf = std::path::PathBuf::from(output_str);
        match handle_ref
            .processor
            .save_image_direct(&dyn_image, &output_path_buf)
        {
            Ok(_) => {
                log::info!(
                    "Successfully applied Mosaic effect and saved to {}",
                    output_str
                );
                true
            }
            Err(e) => {
                log::error!("Failed to save image: {}", e);
                false
            }
        }
    }
}

/// Apply Stroke effect to detected face areas
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_apply_stroke(
    handle: *mut ChamaOpticsHandle,
    face_rects: *const CFaceRect,
    face_count: usize,
    image_path: *const c_char,
    output_path: *const c_char,
    stroke_color_r: u8,
    stroke_color_g: u8,
    stroke_color_b: u8,
    stroke_color_a: u8,
    stroke_thickness: u32,
) -> bool {
    if handle.is_null() || image_path.is_null() || output_path.is_null() {
        return false;
    }

    unsafe {
        let image_str = match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in image path");
                return false;
            }
        };

        let output_str = match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in output path");
                return false;
            }
        };

        let handle_ref = &mut *handle;

        // Load image
        let path_buf = std::path::PathBuf::from(image_str);
        let mut dyn_image = match handle_ref.processor.load_image_direct(&path_buf) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image {}: {}", image_str, e);
                return false;
            }
        };

        // Collect face rectangles
        let mut face_areas = vec![];
        if !face_rects.is_null() && face_count > 0 {
            for i in 0..face_count {
                let face_rect = *face_rects.add(i);
                face_areas.push((face_rect.x, face_rect.y, face_rect.width, face_rect.height));
            }
        }

        // Create Stroke effect config
        let stroke_config = crate::effect::stroke::StrokeEffect {
            thickness: stroke_thickness,
            color: (
                stroke_color_r,
                stroke_color_g,
                stroke_color_b,
                stroke_color_a,
            ),
        };

        // Apply Stroke effect - pass slice to apply() method
        log::info!(
            "Applying Stroke effect: thickness=({}, {}, {}, {}, {}), {} faces",
            stroke_thickness,
            stroke_color_r,
            stroke_color_g,
            stroke_color_b,
            stroke_color_a,
            face_areas.len()
        );

        if let Err(e) =
            crate::effect::stroke::StrokeEffect::apply(&mut dyn_image, &face_areas, &stroke_config)
        {
            log::error!("Failed to apply Stroke effect: {}", e);
            return false;
        }

        // Save image
        let output_path_buf = std::path::PathBuf::from(output_str);
        match handle_ref
            .processor
            .save_image_direct(&dyn_image, &output_path_buf)
        {
            Ok(_) => {
                log::info!(
                    "Successfully applied Stroke effect and saved to {}",
                    output_str
                );
                true
            }
            Err(e) => {
                log::error!("Failed to save image: {}", e);
                false
            }
        }
    }
}

/// Apply Sticker to detected face areas using built-in sticker ID
/// For custom image stickers, use chama_optics_apply_sticker_from_path instead
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[cfg(any(target_os = "ios", target_os = "android"))]
pub extern "C" fn chama_optics_apply_sticker(
    handle: *mut ChamaOpticsHandle,
    face_rects: *const CFaceRect,
    face_count: usize,
    image_path: *const c_char,
    output_path: *const c_char,
    sticker_id: *const c_char, // ID of sticker to apply
    sticker_scale: f32,        // Scale factor for sticker
    sticker_offset_x: i32,     // X offset from face center
    sticker_offset_y: i32,     // Y offset from face center
) -> bool {
    if handle.is_null() || image_path.is_null() || output_path.is_null() || sticker_id.is_null() {
        return false;
    }

    unsafe {
        let image_str = match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in image path");
                return false;
            }
        };

        let output_str = match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in output path");
                return false;
            }
        };

        let sticker_str = match CStr::from_ptr(sticker_id).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in sticker ID");
                return false;
            }
        };

        let handle_ref = &mut *handle;

        // Load image
        let path_buf = std::path::PathBuf::from(image_str);
        let mut dyn_image = match handle_ref.processor.load_image_direct(&path_buf) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image {}: {}", image_str, e);
                return false;
            }
        };

        // Collect face rectangles
        let mut face_areas = vec![];
        if !face_rects.is_null() && face_count > 0 {
            for i in 0..face_count {
                let face_rect = *face_rects.add(i);
                face_areas.push((face_rect.x, face_rect.y, face_rect.width, face_rect.height));
            }
        }

        log::info!(
            "Applying Sticker: id={}, scale={}, offset=({}, {}), {} faces",
            sticker_str,
            sticker_scale,
            sticker_offset_x,
            sticker_offset_y,
            face_areas.len()
        );

        // Apply sticker effect with built-in sticker
        let config = crate::effect::sticker::StickerConfig::with_builtin(
            sticker_str.to_string(),
            sticker_scale,
            sticker_offset_x,
            sticker_offset_y,
        );

        dyn_image = crate::effect::sticker::apply_sticker(dyn_image, face_areas, &config);

        // Save image
        let output_path_buf = std::path::PathBuf::from(output_str);
        match handle_ref
            .processor
            .save_image_direct(&dyn_image, &output_path_buf)
        {
            Ok(_) => {
                log::info!(
                    "Successfully applied Sticker effect and saved to {}",
                    output_str
                );
                true
            }
            Err(e) => {
                log::error!("Failed to save image: {}", e);
                false
            }
        }
    }
}

/// Apply Sticker to detected face areas using a custom image path
/// This is the preferred method for iOS which loads sticker images from file paths
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[cfg(any(target_os = "ios", target_os = "android"))]
pub extern "C" fn chama_optics_apply_sticker_from_path(
    handle: *mut ChamaOpticsHandle,
    face_rects: *const CFaceRect,
    face_count: usize,
    image_path: *const c_char,
    output_path: *const c_char,
    sticker_image_path: *const c_char, // Path to sticker image file (PNG, JPG, etc.)
    sticker_scale: f32,                // Scale factor for sticker
    sticker_offset_x: i32,             // X offset from face center
    sticker_offset_y: i32,             // Y offset from face center
) -> bool {
    if handle.is_null()
        || image_path.is_null()
        || output_path.is_null()
        || sticker_image_path.is_null()
    {
        return false;
    }

    unsafe {
        let image_str = match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in image path");
                return false;
            }
        };

        let output_str = match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in output path");
                return false;
            }
        };

        let sticker_path_str = match CStr::from_ptr(sticker_image_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in sticker image path");
                return false;
            }
        };

        let handle_ref = &mut *handle;

        // Load image
        let path_buf = std::path::PathBuf::from(image_str);
        let mut dyn_image = match handle_ref.processor.load_image_direct(&path_buf) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image {}: {}", image_str, e);
                return false;
            }
        };

        // Collect face rectangles
        let mut face_areas = vec![];
        if !face_rects.is_null() && face_count > 0 {
            for i in 0..face_count {
                let face_rect = *face_rects.add(i);
                face_areas.push((face_rect.x, face_rect.y, face_rect.width, face_rect.height));
            }
        }

        log::info!(
            "Applying Sticker from path: {}, scale={}, offset=({}, {}), {} faces",
            sticker_path_str,
            sticker_scale,
            sticker_offset_x,
            sticker_offset_y,
            face_areas.len()
        );

        // Apply sticker effect with image path
        let sticker_path_buf = std::path::PathBuf::from(sticker_path_str);
        let config = crate::effect::sticker::StickerConfig::with_image_path(
            sticker_path_buf,
            sticker_scale,
            sticker_offset_x,
            sticker_offset_y,
        );

        dyn_image = crate::effect::sticker::apply_sticker(dyn_image, face_areas, &config);

        // Save image
        let output_path_buf = std::path::PathBuf::from(output_str);
        match handle_ref
            .processor
            .save_image_direct(&dyn_image, &output_path_buf)
        {
            Ok(_) => {
                log::info!(
                    "Successfully applied Sticker from path and saved to {}",
                    output_str
                );
                true
            }
            Err(e) => {
                log::error!("Failed to save image: {}", e);
                false
            }
        }
    }
}

// Additional iOS face detection functions

/// Load an image from path
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_load_image(
    handle: *mut ChamaOpticsHandle,
    path: *const c_char,
) -> bool {
    if handle.is_null() || path.is_null() {
        return false;
    }

    unsafe {
        let path_str = match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in path");
                return false;
            }
        };

        let handle_ref = &mut *handle;
        let path_buf = PathBuf::from(path_str);

        match handle_ref.processor.load_image(path_buf) {
            Ok(index) => {
                log::info!("Successfully loaded image at index {}: {}", index, path_str);
                true
            }
            Err(e) => {
                log::error!("Failed to load image {}: {}", path_str, e);
                false
            }
        }
    }
}

/// Detect faces in an image using VisionKit
/// Returns a list of face rectangles
/// The returned list must be freed with chama_optics_free_face_rect_list
#[cfg(any(target_os = "ios", target_os = "android"))]
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_detect_faces_ios(
    _handle: *mut ChamaOpticsHandle,
    _image_path: *const c_char,
) -> *mut CFaceRectList {
    // This is a placeholder - actual implementation will be in Swift
    // The Swift side will call VisionKit and return face rectangles
    // For now, return empty list
    log::info!("Face detection placeholder called on iOS");
    let faces: Vec<CFaceRect> = vec![];
    let mut faces = faces.into_boxed_slice();
    let list = Box::new(CFaceRectList {
        faces: faces.as_mut_ptr(),
        count: faces.len(),
    });
    std::mem::forget(faces);
    Box::into_raw(list)
}

/// Apply face detection rectangles to an image
/// This function takes face rectangles from VisionKit and applies them to image
#[unsafe(no_mangle)]
#[cfg(any(target_os = "ios", target_os = "android"))]
pub extern "C" fn chama_optics_apply_face_detection(
    handle: *mut ChamaOpticsHandle,
    face_rects: *const CFaceRect,
    face_count: usize,
    image_path: *const c_char,
    output_path: *const c_char,
    engine_type: u32, // 0 = VisionKit, 3 = InsightFace
    border_color_r: u8,
    border_color_g: u8,
    border_color_b: u8,
    border_color_a: u8,
    border_thickness: u32,
    mask_faces: bool,
    _mask_blur_radius: f32,
    _speed_mode: u32, // 0 = Fastest, 1 = Fast, 2 = Normal, 3 = Slow, 4 = Slowest
) -> bool {
    if handle.is_null() || image_path.is_null() || output_path.is_null() {
        return false;
    }

    unsafe {
        let image_str = match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in image path");
                return false;
            }
        };

        let output_str = match CStr::from_ptr(output_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in output path");
                return false;
            }
        };

        let handle_ref = &mut *handle;

        // Load image
        let path_buf = std::path::PathBuf::from(image_str);
        let mut dyn_image = match handle_ref.processor.load_image_direct(&path_buf) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image {}: {}", image_str, e);
                return false;
            }
        };

        // Collect face rectangles
        let mut face_areas = vec![];
        if !face_rects.is_null() && face_count > 0 {
            for i in 0..face_count {
                let face_rect = *face_rects.add(i);
                face_areas.push((face_rect.x, face_rect.y, face_rect.width, face_rect.height));
            }
        }

        // Create FaceDetection config with engine
        let border_color = egui::Color32::from_rgba_unmultiplied(
            border_color_r,
            border_color_g,
            border_color_b,
            border_color_a,
        );

        #[cfg(feature = "face_detection_insightface")]
        let speed_mode = match _speed_mode {
            0 => crate::effect::insightface_detector::SpeedMode::Fastest,
            1 => crate::effect::insightface_detector::SpeedMode::Fast,
            2 => crate::effect::insightface_detector::SpeedMode::Normal,
            3 => crate::effect::insightface_detector::SpeedMode::Slow,
            4 => crate::effect::insightface_detector::SpeedMode::Slowest,
            _ => {
                log::warn!("Invalid speed_mode {}, using Normal", _speed_mode);
                crate::effect::insightface_detector::SpeedMode::Normal
            }
        };

        let engine = match engine_type {
            3 => {
                #[cfg(feature = "face_detection_insightface")]
                {
                    crate::effect::face_detection::FaceDetectionEngine::InsightFace
                }
                #[cfg(not(feature = "face_detection_insightface"))]
                {
                    log::warn!("InsightFace requested but feature not enabled");
                    #[cfg(feature = "face_detection_visionkit")]
                    {
                        crate::effect::face_detection::FaceDetectionEngine::VisionKit
                    }
                    #[cfg(not(feature = "face_detection_visionkit"))]
                    {
                        log::error!("No face detection engine available!");
                        return false;
                    }
                }
            }
            _ => {
                #[cfg(feature = "face_detection_visionkit")]
                {
                    crate::effect::face_detection::FaceDetectionEngine::VisionKit
                }
                #[cfg(not(feature = "face_detection_visionkit"))]
                {
                    log::warn!("VisionKit requested but feature not enabled");
                    #[cfg(feature = "face_detection_insightface")]
                    {
                        crate::effect::face_detection::FaceDetectionEngine::InsightFace
                    }
                    #[cfg(not(feature = "face_detection_insightface"))]
                    {
                        log::error!("No face detection engine available!");
                        return false;
                    }
                }
            }
        };

        let face_detection = crate::effect::face_detection::FaceDetection {
            engine,
            border_color,
            border_thickness,
            mask_faces,
            #[cfg(feature = "face_detection_insightface")]
            speed_mode,
            #[cfg(feature = "face_detection_insightface")]
            provider: crate::effect::insightface_detector::ExecutionProvider::CPUExecutionProvider,
            recursive_detection: false,
            recursive_min_size: 64,
            recursive_max_depth: 4,
            recursive_overlap: true,
            recursive_overlap_ratio: 0.25,
            effect_mode: crate::effect::face_detection::FaceEffectMode::None,
            mosaic_block_size: 10,
        };

        // Apply face detection - pass owned Vec (not borrowed reference)
        if let Err(e) = face_detection.apply(&mut dyn_image, face_areas) {
            log::error!("Failed to apply face detection: {}", e);
            return false;
        }

        // Save image
        let output_path_buf = std::path::PathBuf::from(output_str);
        match handle_ref
            .processor
            .save_image_direct(&dyn_image, &output_path_buf)
        {
            Ok(_) => {
                log::info!(
                    "Successfully applied face detection and saved to {}",
                    output_str
                );
                true
            }
            Err(e) => {
                log::error!("Failed to save image: {}", e);
                false
            }
        }
    }
}

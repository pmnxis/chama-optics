/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Cheki (Polaroid) Decoration Export FFI functions
//!
//! Provides C-compatible functions for applying cheki (polaroid-style) decorations
//! to images from mobile platforms (iOS/Android).

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::{Path, PathBuf};

use super::types::*;

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
    save_exif: bool,
    exif_override_json: *const c_char,
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
            save_exif,
            exif_override_json,
        )
    }));

    match result {
        Ok(error_code) => error_code,
        Err(panic_info) => {
            log::error!(
                "Caught panic in chama_export_cheki: {}",
                super::extract_panic_message(&panic_info)
            );
            ChamaError::ImageProcessError
        }
    }
}

/// Render cheki decoration to encoded image bytes for UI preview.
///
/// Uses the exact same rendering path as `chama_export_cheki` to guarantee
/// preview output matches the final exported result. The image is downscaled
/// to `max_dimension` (long edge) before rendering to keep preview fast.
///
/// # Parameters
/// - `image_path`: Path to the source image
/// - `cheki_json`: JSON string with ChekiDecoration configuration
/// - `sticker_dir`: Directory containing sticker images (null = no stickers)
/// - `crop_rotate_json`: Optional crop/rotate transform (null = none)
/// - `color_adjustments_json`: Optional color adjustments (null = none)
/// - `lut_id`: Optional LUT UUID string (null = none)
/// - `max_dimension`: Long-edge cap for the preview image (e.g. 1024)
/// - `out_data`: Output pointer to the encoded JPEG byte buffer
/// - `out_len`: Output length of the byte buffer
///
/// Free the returned buffer with `chama_preview_pipeline_free_bytes(out_data, out_len)`.
///
/// # Safety
/// All string pointers must be valid null-terminated C strings or null.
#[unsafe(no_mangle)]
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe extern "C" fn chama_preview_cheki_bytes(
    image_path: *const c_char,
    cheki_json: *const c_char,
    sticker_dir: *const c_char,
    crop_rotate_json: *const c_char,
    color_adjustments_json: *const c_char,
    lut_id: *const c_char,
    max_dimension: u32,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> ChamaError {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        chama_preview_cheki_bytes_impl(
            image_path,
            cheki_json,
            sticker_dir,
            crop_rotate_json,
            color_adjustments_json,
            lut_id,
            max_dimension,
            out_data,
            out_len,
        )
    }));

    match result {
        Ok(error_code) => error_code,
        Err(panic_info) => {
            log::error!(
                "Caught panic in chama_preview_cheki_bytes: {}",
                super::extract_panic_message(&panic_info)
            );
            ChamaError::ImageProcessError
        }
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn chama_preview_cheki_bytes_impl(
    image_path: *const c_char,
    cheki_json: *const c_char,
    sticker_dir: *const c_char,
    crop_rotate_json: *const c_char,
    color_adjustments_json: *const c_char,
    lut_id: *const c_char,
    max_dimension: u32,
    out_data: *mut *mut u8,
    out_len: *mut usize,
) -> ChamaError {
    if image_path.is_null() || cheki_json.is_null() || out_data.is_null() || out_len.is_null() {
        return ChamaError::InvalidParameters;
    }

    // Build the rendered cheki image using the shared helper (same path as export)
    let rendered = match build_cheki_image(
        image_path,
        cheki_json,
        sticker_dir,
        crop_rotate_json,
        color_adjustments_json,
        lut_id,
        Some(max_dimension),
    ) {
        Ok(img) => img,
        Err(e) => return e,
    };

    // Encode to JPEG bytes (quality 85 is sufficient for preview)
    let format = crate::export_config::output_format::OutputFormat {
        ext: crate::export_config::output_format::OutputExtension::Jpeg,
        quality: 85,
    };
    let bytes = match format.encode_to_bytes(&rendered) {
        Ok(b) => b,
        Err(e) => {
            log::error!("Preview cheki encode error: {}", e);
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
    save_exif: bool,
    exif_override_json: *const c_char,
) -> ChamaError {
    if image_path.is_null() || output_path.is_null() || cheki_json.is_null() {
        return ChamaError::InvalidPath;
    }

    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);

    // Build cheki image using shared helper (full resolution, no max_dimension cap)
    let result = match build_cheki_image(
        image_path,
        cheki_json,
        sticker_dir,
        crop_rotate_json,
        color_adjustments_json,
        lut_id,
        None,
    ) {
        Ok(img) => img,
        Err(e) => return e,
    };

    // Save result
    let save_result = if output_format_config.is_null() {
        result.save(output_path_str).map_err(|e| {
            log::error!("Failed to save cheki result: {}", e);
            e
        })
    } else {
        let config_ref = unsafe { &*output_format_config };
        let output_format = crate::export_config::output_format::OutputFormat {
            ext: super::convert_c_output_format(config_ref.output_format),
            quality: config_ref.quality,
        };
        output_format
            .save_image(&result, output_path_str)
            .map_err(|e| {
                log::error!("Failed to save cheki result: {}", e);
                e
            })
    };

    match save_result {
        Ok(_) => {
            // Inject EXIF if enabled
            if save_exif {
                let image_path_str = cstr_to_str!(image_path, return ChamaError::InvalidPath);
                let exif_override_str = if exif_override_json.is_null() {
                    None
                } else {
                    let s = cstr_to_str_or!(exif_override_json, "");
                    if !s.is_empty() { Some(s) } else { None }
                };
                if let Err(e) = crate::image::exif_inject::inject_exif_to_output(
                    image_path_str,
                    output_path_str,
                    exif_override_str,
                    false,
                    false,
                ) {
                    log::warn!("EXIF injection failed (non-fatal): {}", e);
                }
            }
            log::info!("✅ Cheki export completed: {}", output_path_str);
            ChamaError::Success
        }
        Err(_) => ChamaError::ImageProcessError,
    }
}

/// Shared cheki rendering logic for both export and preview.
///
/// Steps 1-7 of the cheki pipeline:
/// 1. Parse ChekiDecoration JSON
/// 2. Load image with EXIF orientation
/// 3. Apply crop/rotate
/// 4. Apply color adjustments
/// 5. Apply LUT
/// 6. Build sticker storage
/// 7. Apply cheki decoration
///
/// `max_dimension`: if `Some(n)`, downscale long edge to `n` before rendering.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn build_cheki_image(
    image_path: *const c_char,
    cheki_json: *const c_char,
    sticker_dir: *const c_char,
    crop_rotate_json: *const c_char,
    color_adjustments_json: *const c_char,
    lut_id: *const c_char,
    max_dimension: Option<u32>,
) -> Result<image::DynamicImage, ChamaError> {
    let image_path_str = cstr_to_str!(image_path, return Err(ChamaError::InvalidPath));
    let cheki_json_str = cstr_to_str!(cheki_json, return Err(ChamaError::InvalidParameters));
    let sticker_dir_str = cstr_to_str_or!(sticker_dir, "");

    log::info!("Cheki pipeline: input={}", image_path_str);

    // Step 1: Parse ChekiDecoration
    log::info!(
        "  Cheki JSON (first 500): {}",
        &cheki_json_str[..cheki_json_str.len().min(500)]
    );
    let decoration: crate::effect::cheki::ChekiDecoration =
        match serde_json::from_str(cheki_json_str) {
            Ok(d) => d,
            Err(e) => {
                log::error!("Failed to parse ChekiDecoration JSON: {}", e);
                return Err(ChamaError::InvalidParameters);
            }
        };

    // Step 2: Load image with HEIF support and apply EXIF orientation
    let mut dyn_image = match super::load_image_with_heif_support(Path::new(image_path_str)) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return Err(ChamaError::ImageLoadError);
        }
    };
    dyn_image.apply_orientation(super::read_exif_orientation(image_path_str));
    log::info!(
        "  Image size after orientation: {}x{}",
        dyn_image.width(),
        dyn_image.height()
    );

    // Downscale for preview if max_dimension is set
    if let Some(max_dim) = max_dimension {
        if max_dim > 0 {
            let long_edge = dyn_image.width().max(dyn_image.height());
            if long_edge > max_dim {
                dyn_image =
                    dyn_image.resize(max_dim, max_dim, image::imageops::FilterType::Triangle);
                log::info!(
                    "  Downscaled for preview: {}x{}",
                    dyn_image.width(),
                    dyn_image.height()
                );
            }
        }
    }

    // Step 3: Apply crop/rotate transform
    if !crop_rotate_json.is_null() {
        let crop_rotate_str = cstr_to_str_or!(crop_rotate_json, "{}");
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

    // Step 4: Apply color adjustments
    log::info!(
        "  color_adjustments_json is_null={}, lut_id is_null={}",
        color_adjustments_json.is_null(),
        lut_id.is_null()
    );
    if !color_adjustments_json.is_null() {
        let adjustments_str = cstr_to_str_or!(color_adjustments_json, "{}");
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

    // Step 5: Apply LUT
    if !lut_id.is_null() {
        let lut_id_str = cstr_to_str_or!(lut_id, "");
        if !lut_id_str.is_empty() {
            if let Ok(uuid) = uuid::Uuid::parse_str(lut_id_str) {
                let mut storage = match super::lut::LUT_STORAGE.lock() {
                    Ok(s) => s,
                    Err(_) => {
                        log::warn!("Failed to lock LUT storage");
                        return Err(ChamaError::Unknown);
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

    // Step 6: Build sticker storage
    let sticker_storage = build_sticker_storage_from_dir_and_json(sticker_dir_str, &decoration);

    // Step 7: Apply cheki decoration
    Ok(crate::effect::cheki_renderer::apply_cheki_decoration(
        dyn_image,
        &decoration,
        &sticker_storage,
    ))
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

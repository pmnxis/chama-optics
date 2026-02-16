/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Combined Export Pipeline FFI functions
//!
//! Provides the combined export pipeline (Face Effects -> Theme -> Scale -> Save)
//! and standalone scale image functions for mobile platforms (iOS/Android).

use std::ffi::CStr;
use std::os::raw::c_char;

use super::types::*;

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

    let image_path_str = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);

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
    match super::save_image_with_c_format(&scaled_image, output_path_str, output_format, quality) {
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

    let image_path_str = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);

    let config_ref = unsafe { &*config };

    log::info!("Combined export pipeline started:");
    log::info!("  Input: {}", image_path_str);
    log::info!("  Output: {}", output_path_str);
    log::info!("  Face effect: {:?}", config_ref.face_effect.effect_type);
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
    let orientation = super::read_exif_orientation(image_path_str);
    log::info!("  EXIF orientation: {:?}", orientation);
    dyn_image.apply_orientation(orientation);
    log::info!(
        "  After orientation: {}x{}",
        dyn_image.width(),
        dyn_image.height()
    );

    // Ensure image is RGBA8 (JPEG decodes as RGB8, but face effects require RGBA8)
    if dyn_image.as_rgba8().is_none() {
        dyn_image = image::DynamicImage::ImageRgba8(dyn_image.to_rgba8());
    }

    // Step 2: Apply face effects (if faces provided and effect != None)
    if !face_rects.is_null()
        && face_count > 0
        && config_ref.face_effect.effect_type != CFaceEffectType::None
    {
        let face_areas = unsafe { super::collect_face_areas(face_rects, face_count) };

        log::info!("  Applying face effect to {} faces...", face_areas.len());

        match config_ref.face_effect.effect_type {
            CFaceEffectType::Mosaic => {
                let mosaic_config = crate::effect::mosaic::MosaicEffect {
                    block_size: config_ref.face_effect.mosaic_block_size,
                    intensity: config_ref.face_effect.mosaic_intensity,
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
                    thickness: config_ref.face_effect.stroke_thickness,
                    color: (
                        config_ref.face_effect.stroke_color.r,
                        config_ref.face_effect.stroke_color.g,
                        config_ref.face_effect.stroke_color.b,
                        config_ref.face_effect.stroke_color.a,
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
                let sticker_config = if !config_ref.face_effect.sticker_path.is_null() {
                    let sticker_path_str = cstr_to_str!(
                        config_ref.face_effect.sticker_path,
                        return ChamaError::InvalidPath
                    );
                    crate::effect::sticker::StickerConfig::with_image_path(
                        std::path::PathBuf::from(sticker_path_str),
                        config_ref.face_effect.sticker_scale,
                        config_ref.face_effect.sticker_offset_x,
                        config_ref.face_effect.sticker_offset_y,
                    )
                } else if !config_ref.face_effect.sticker_id.is_null() {
                    let sticker_id_str = cstr_to_str!(
                        config_ref.face_effect.sticker_id,
                        return ChamaError::InvalidPath
                    );
                    crate::effect::sticker::StickerConfig::with_builtin(
                        sticker_id_str.to_string(),
                        config_ref.face_effect.sticker_scale,
                        config_ref.face_effect.sticker_offset_x,
                        config_ref.face_effect.sticker_offset_y,
                    )
                } else {
                    crate::effect::sticker::StickerConfig::with_builtin(
                        "heart".to_string(),
                        config_ref.face_effect.sticker_scale,
                        config_ref.face_effect.sticker_offset_x,
                        config_ref.face_effect.sticker_offset_y,
                    )
                };
                dyn_image =
                    crate::effect::sticker::apply_sticker(dyn_image, face_areas, &sticker_config);
                log::info!("  Sticker applied successfully");
            }
            CFaceEffectType::MosaicStroke => {
                // Apply mosaic first (inside the face area)
                let mosaic_config = crate::effect::mosaic::MosaicEffect {
                    block_size: config_ref.face_effect.mosaic_block_size,
                    intensity: config_ref.face_effect.mosaic_intensity,
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
                    thickness: config_ref.face_effect.stroke_thickness,
                    color: (
                        config_ref.face_effect.stroke_color.r,
                        config_ref.face_effect.stroke_color.g,
                        config_ref.face_effect.stroke_color.b,
                        config_ref.face_effect.stroke_color.a,
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
        let theme_name_str = cstr_to_str_or!(config_ref.theme_name, "");
        if theme_name_str.is_empty() {
            log::info!("  No theme specified, skipping theme application");
        }

        if !theme_name_str.is_empty() {
            log::info!("  Applying theme: {}", theme_name_str);

            let params_json = cstr_to_str_or!(config_ref.theme_params_json, "{}");
            let font_path = cstr_to_str_or!(config_ref.font_path, "");

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
            let core_scale_config = unsafe {
                super::convert_c_scale_config(&config_ref.scale_config as *const CScaleConfig)
            };

            // Parse EXIF override JSON if provided
            let exif_override_str = if config_ref.exif_override_json.is_null() {
                None
            } else {
                let s = cstr_to_str_or!(config_ref.exif_override_json, "");
                if !s.is_empty() { Some(s) } else { None }
            };

            let theme_params = ThemeExportParams {
                image_path: &temp_path,
                exif_source_path: image_path_str,
                output_path: output_path_str,
                theme_name: theme_name_str,
                params_json,
                font_path,
                font_weight: config_ref.font_weight,
                scale_config: core_scale_config,
                output_format_config: None,
                get_alt_fnumber: config_ref.get_alt_fnumber,
                use_35mm_focal_length: config_ref.use_35mm_focal_length,
                exif_override_json: exif_override_str,
            };
            let theme_result = super::theme::export_final_impl(&theme_params);

            // Clean up temp file
            let _ = std::fs::remove_file(&temp_path);

            match theme_result {
                Ok(_) => {
                    log::info!("  Theme applied successfully");
                    // Theme already saved to output_path
                    // Inject EXIF if enabled (after theme save)
                    if config_ref.save_exif && config_ref.output_format != COutputFormat::Png {
                        let exif_override_str = if config_ref.exif_override_json.is_null() {
                            None
                        } else {
                            let s = cstr_to_str_or!(config_ref.exif_override_json, "");
                            if !s.is_empty() { Some(s) } else { None }
                        };
                        if let Err(e) = crate::image::exif_inject::inject_exif_to_output(
                            image_path_str,
                            output_path_str,
                            exif_override_str,
                            config_ref.get_alt_fnumber,
                            config_ref.use_35mm_focal_length,
                        ) {
                            log::warn!("EXIF injection failed (non-fatal): {}", e);
                        }
                    }
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

    match super::save_image_with_c_format(
        &final_image,
        output_path_str,
        config_ref.output_format,
        config_ref.quality,
    ) {
        Ok(_) => {
            // Inject EXIF if enabled (after pixel-only save)
            if config_ref.save_exif && config_ref.output_format != COutputFormat::Png {
                let exif_override_str = if config_ref.exif_override_json.is_null() {
                    None
                } else {
                    let s = cstr_to_str_or!(config_ref.exif_override_json, "");
                    if !s.is_empty() { Some(s) } else { None }
                };
                if let Err(e) = crate::image::exif_inject::inject_exif_to_output(
                    image_path_str,
                    output_path_str,
                    exif_override_str,
                    config_ref.get_alt_fnumber,
                    config_ref.use_35mm_focal_length,
                ) {
                    log::warn!("EXIF injection failed (non-fatal): {}", e);
                }
            }
            log::info!("✅ Combined export completed successfully");
            ChamaError::Success
        }
        Err(e) => {
            log::error!("Failed to save image: {}", e);
            ChamaError::ImageProcessError
        }
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Face detection and face effect FFI functions for mobile platforms.
//!
//! This module contains the opaque `ChamaOpticsHandle` and functions for
//! creating / destroying instances, loading images, detecting faces, and
//! applying face effects (mosaic, stroke, sticker, combined).

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

// Types used by target_os-gated FFI functions (appear unused on host cargo check)
#[allow(unused_imports)]
use super::types::*;
use crate::core::ImageProcessor;

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

/// Apply face effect to detected face areas (consolidated: mosaic, stroke, sticker, mosaic+stroke)
/// Dispatches to the appropriate effect based on config.effect_type
#[cfg(any(target_os = "ios", target_os = "android"))]
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_apply_face_effect(
    handle: *mut ChamaOpticsHandle,
    face_rects: *const CFaceRect,
    face_count: usize,
    image_path: *const c_char,
    output_path: *const c_char,
    config: *const CFaceEffectConfig,
) -> bool {
    if handle.is_null() || image_path.is_null() || output_path.is_null() || config.is_null() {
        return false;
    }

    unsafe {
        let config_ref = &*config;

        if config_ref.effect_type == CFaceEffectType::None {
            log::info!("Face effect type is None, skipping");
            return true;
        }

        let image_str = cstr_to_str!(image_path, return false);
        let output_str = cstr_to_str!(output_path, return false);

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
        let face_areas = super::collect_face_areas(face_rects, face_count);

        log::info!(
            "Applying face effect {:?} to {} faces",
            config_ref.effect_type,
            face_areas.len()
        );

        match config_ref.effect_type {
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
                    log::error!("Failed to apply Mosaic effect: {}", e);
                    return false;
                }
            }
            CFaceEffectType::Stroke => {
                let stroke_config = crate::effect::stroke::StrokeEffect {
                    thickness: config_ref.stroke_thickness,
                    color: (
                        config_ref.stroke_color.r,
                        config_ref.stroke_color.g,
                        config_ref.stroke_color.b,
                        config_ref.stroke_color.a,
                    ),
                };
                if let Err(e) = crate::effect::stroke::StrokeEffect::apply(
                    &mut dyn_image,
                    &face_areas,
                    &stroke_config,
                ) {
                    log::error!("Failed to apply Stroke effect: {}", e);
                    return false;
                }
            }
            CFaceEffectType::Sticker => {
                let sticker_config = if !config_ref.sticker_path.is_null() {
                    let sticker_path_str = cstr_to_str!(config_ref.sticker_path, return false);
                    crate::effect::sticker::StickerConfig::with_image_path(
                        std::path::PathBuf::from(sticker_path_str),
                        config_ref.sticker_scale,
                        config_ref.sticker_offset_x,
                        config_ref.sticker_offset_y,
                    )
                } else if !config_ref.sticker_id.is_null() {
                    let sticker_id_str = cstr_to_str!(config_ref.sticker_id, return false);
                    crate::effect::sticker::StickerConfig::with_builtin(
                        sticker_id_str.to_string(),
                        config_ref.sticker_scale,
                        config_ref.sticker_offset_x,
                        config_ref.sticker_offset_y,
                    )
                } else {
                    // Fallback to default built-in sticker
                    crate::effect::sticker::StickerConfig::with_builtin(
                        "heart".to_string(),
                        config_ref.sticker_scale,
                        config_ref.sticker_offset_x,
                        config_ref.sticker_offset_y,
                    )
                };
                dyn_image = crate::effect::sticker::apply_sticker(
                    dyn_image,
                    face_areas.clone(),
                    &sticker_config,
                );
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
                    return false;
                }

                // Then apply stroke (border around the face area)
                let stroke_config = crate::effect::stroke::StrokeEffect {
                    thickness: config_ref.stroke_thickness,
                    color: (
                        config_ref.stroke_color.r,
                        config_ref.stroke_color.g,
                        config_ref.stroke_color.b,
                        config_ref.stroke_color.a,
                    ),
                };
                if let Err(e) = crate::effect::stroke::StrokeEffect::apply(
                    &mut dyn_image,
                    &face_areas,
                    &stroke_config,
                ) {
                    log::error!("Failed to apply stroke in MosaicStroke: {}", e);
                    return false;
                }
            }
            CFaceEffectType::None => {}
        }

        // Save image
        let output_path_buf = std::path::PathBuf::from(output_str);
        match handle_ref
            .processor
            .save_image_direct(&dyn_image, &output_path_buf)
        {
            Ok(_) => {
                log::info!(
                    "Successfully applied face effect and saved to {}",
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

    let path_str = cstr_to_str!(path, return false);

    unsafe {
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

/// Apply face detection rectangles to an image (v2: uses CFaceDetectionConfig struct)
/// This function takes face rectangles from VisionKit and applies them to image
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[cfg(any(target_os = "ios", target_os = "android"))]
pub extern "C" fn chama_optics_apply_face_detection_v2(
    handle: *mut ChamaOpticsHandle,
    face_rects: *const CFaceRect,
    face_count: usize,
    image_path: *const c_char,
    output_path: *const c_char,
    config: *const CFaceDetectionConfig,
) -> bool {
    // Early return if no face detection engine is available
    #[cfg(not(any(
        feature = "face_detection_insightface",
        feature = "face_detection_visionkit"
    )))]
    {
        let _ = (
            handle,
            face_rects,
            face_count,
            image_path,
            output_path,
            config,
        );
        log::error!("No face detection engine available!");
        return false;
    }

    #[cfg(any(
        feature = "face_detection_insightface",
        feature = "face_detection_visionkit"
    ))]
    {
        if handle.is_null() || image_path.is_null() || output_path.is_null() || config.is_null() {
            return false;
        }

        unsafe {
            let config_ref = &*config;

            let image_str = cstr_to_str!(image_path, return false);
            let output_str = cstr_to_str!(output_path, return false);

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
            let face_areas = super::collect_face_areas(face_rects, face_count);

            // Create FaceDetection config with engine
            let border_color = egui::Color32::from_rgba_unmultiplied(
                config_ref.border_color.r,
                config_ref.border_color.g,
                config_ref.border_color.b,
                config_ref.border_color.a,
            );

            #[cfg(feature = "face_detection_insightface")]
            let speed_mode = match config_ref.speed_mode {
                0 => crate::effect::insightface_detector::SpeedMode::Fastest,
                1 => crate::effect::insightface_detector::SpeedMode::Fast,
                2 => crate::effect::insightface_detector::SpeedMode::Normal,
                3 => crate::effect::insightface_detector::SpeedMode::Slow,
                4 => crate::effect::insightface_detector::SpeedMode::Slowest,
                _ => {
                    log::warn!("Invalid speed_mode {}, using Normal", config_ref.speed_mode);
                    crate::effect::insightface_detector::SpeedMode::Normal
                }
            };

            let engine = match config_ref.engine_type {
                #[cfg(feature = "face_detection_insightface")]
                3 => crate::effect::face_detection::FaceDetectionEngine::InsightFace,
                _ => {
                    #[cfg(feature = "face_detection_visionkit")]
                    {
                        crate::effect::face_detection::FaceDetectionEngine::VisionKit
                    }
                    #[cfg(not(feature = "face_detection_visionkit"))]
                    {
                        crate::effect::face_detection::FaceDetectionEngine::InsightFace
                    }
                }
            };

            let face_detection = crate::effect::face_detection::FaceDetection {
                engine,
                border_color,
                border_thickness: config_ref.border_thickness,
                mask_faces: config_ref.mask_faces,
                #[cfg(feature = "face_detection_insightface")]
                speed_mode,
                #[cfg(feature = "face_detection_insightface")]
                provider:
                    crate::effect::insightface_detector::ExecutionProvider::CPUExecutionProvider,
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
}

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
    #[cfg(feature = "face_detection_insightface")]
    #[allow(dead_code)]
    insightface_detector: std::sync::Mutex<
        Option<(
            crate::effect::face_detection::SpeedMode,
            crate::effect::insightface_detector::InsightFaceDetector,
        )>,
    >,
}

// Note: chama_optics_init(), chama_optics_version(), and chama_optics_free_string()
// are now in ffi_apple.rs (shared platform-agnostic functions)

/// Create a new ChamaOptics instance
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_create() -> *mut ChamaOpticsHandle {
    log::info!("Creating ChamaOptics instance");
    Box::into_raw(Box::new(ChamaOpticsHandle {
        processor: ImageProcessor::new(),
        #[cfg(feature = "face_detection_insightface")]
        insightface_detector: std::sync::Mutex::new(None),
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

/// Free a CFaceRectList allocated by face detection functions
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_free_face_rect_list(list: *mut CFaceRectList) {
    if !list.is_null() {
        unsafe {
            let list_box = Box::from_raw(list);
            if !list_box.faces.is_null() && list_box.count > 0 {
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    list_box.faces,
                    list_box.count,
                )));
            }
        }
    }
}

/// Detect faces using InsightFace ONNX sliding-window algorithm.
///
/// `camera_mnf` is the EXIF Make string (e.g. `"PANASONIC"`). When `speed_mode`
/// is `4` (Slowest) and the camera is a known ILC brand, the sliding-window
/// pyramid extends one extra level down to the 640 px base window.
///
/// `speed_mode`: 0=Fastest, 1=Fast, 2=Normal, 3=Slow, 4=Slowest
///
/// Returns a `CFaceRectList` that must be freed with `chama_optics_free_face_rect_list`,
/// or null on error.
#[cfg(all(target_os = "ios", feature = "face_detection_insightface"))]
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_detect_faces_insightface(
    handle: *mut ChamaOpticsHandle,
    image_path: *const c_char,
    camera_mnf: *const c_char,
    speed_mode: u32,
) -> *mut CFaceRectList {
    if handle.is_null() || image_path.is_null() {
        return std::ptr::null_mut();
    }

    let image_str = cstr_to_str!(image_path, return std::ptr::null_mut());
    let camera_mnf_str = cstr_to_str_or!(camera_mnf, "");

    let speed = match speed_mode {
        0 => crate::effect::face_detection::SpeedMode::Fastest,
        1 => crate::effect::face_detection::SpeedMode::Fast,
        2 => crate::effect::face_detection::SpeedMode::Normal,
        3 => crate::effect::face_detection::SpeedMode::Slow,
        4 => crate::effect::face_detection::SpeedMode::Slowest,
        _ => {
            log::warn!("Invalid speed_mode {}, using Normal", speed_mode);
            crate::effect::face_detection::SpeedMode::Normal
        }
    };

    unsafe {
        let handle_ref = &mut *handle;

        // Load image
        let path_buf = std::path::PathBuf::from(image_str);
        let img = match handle_ref.processor.load_image_direct(&path_buf) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image for InsightFace detection: {}", e);
                return std::ptr::null_mut();
            }
        };

        // Initialize or re-initialize detector if speed mode changed
        {
            let mut cache = match handle_ref.insightface_detector.lock() {
                Ok(c) => c,
                Err(poisoned) => {
                    log::error!("InsightFace detector mutex poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            let needs_reinit = match &*cache {
                None => true,
                Some((cached_mode, _)) => *cached_mode != speed,
            };
            if needs_reinit {
                log::info!(
                    "Initializing InsightFace detector with speed_mode={:?}",
                    speed
                );
                match crate::effect::insightface_detector::InsightFaceDetector::new(
                    speed,
                    crate::effect::insightface_detector::ExecutionProvider::CPUExecutionProvider,
                ) {
                    Ok(detector) => {
                        *cache = Some((speed, detector));
                    }
                    Err(e) => {
                        log::error!("Failed to create InsightFace detector: {}", e);
                        return std::ptr::null_mut();
                    }
                }
            }
        }

        // Run detection
        let faces = {
            let cache = match handle_ref.insightface_detector.lock() {
                Ok(c) => c,
                Err(poisoned) => {
                    log::error!("InsightFace detector mutex poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            match &*cache {
                Some((_, detector)) => {
                    log::info!(
                        "Running InsightFace detection on iOS: make='{}'",
                        camera_mnf_str
                    );
                    detector.detect_faces_from_image(&img, camera_mnf_str)
                }
                None => return std::ptr::null_mut(),
            }
        };

        log::info!("InsightFace detected {} faces", faces.len());

        // Convert to CFaceRectList
        let c_faces: Vec<CFaceRect> = faces
            .into_iter()
            .map(|(x, y, w, h)| CFaceRect {
                x,
                y,
                width: w,
                height: h,
            })
            .collect();
        let mut c_faces = c_faces.into_boxed_slice();
        let list = Box::new(CFaceRectList {
            faces: c_faces.as_mut_ptr(),
            count: c_faces.len(),
        });
        std::mem::forget(c_faces);
        Box::into_raw(list)
    }
}

// chama_optics_apply_face_detection_v2 removed — confirmed dead (no [LEGACY] log ever fired),
// replaced entirely by chama_optics_apply_face_effect.

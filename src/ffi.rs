/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! FFI (Foreign Function Interface) for iOS/Swift integration
//!
//! This module provides C-compatible functions that can be called from Swift.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;

use crate::core::ImageProcessor;

#[cfg(any(feature = "desktop", feature = "web"))]
use crate::core::ThemeType;

/// C-compatible theme metadata
#[repr(C)]
pub struct CThemeInfo {
    pub unique_name: *const c_char,
    pub label: *const c_char,
    pub has_parameters: bool,
}

/// Array of theme infos
#[repr(C)]
pub struct CThemeList {
    pub themes: *mut CThemeInfo,
    pub count: usize,
}

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

/// C-compatible EXIF data structure
#[repr(C)]
pub struct CExifData {
    pub camera_manufacturer: *const c_char,
    pub camera_model: *const c_char,
    pub lens_manufacturer: *const c_char,
    pub lens_model: *const c_char,
    pub focal_length: *const c_char,
    pub f_number: *const c_char,
    pub exposure_time: *const c_char,
    pub iso_speed: u32, // 0 if not available
    pub datetime: *const c_char,
    pub has_exif: bool,
}

/// Initialize Chama Optics library
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_init() {
    // Force debug level logging for iOS
    // Use try_init to avoid panic if already initialized
    let _ = env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .try_init();
    log::info!("Chama Optics library initialized with DEBUG logging");
}

/// Get library version string
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_version() -> *const c_char {
    let version = CString::new(env!("CARGO_PKG_VERSION")).expect("Failed to create version string");
    version.into_raw()
}

/// Free a string allocated by Rust
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Free a face rectangle list
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_free_face_rect_list(list: *mut CFaceRectList) {
    if list.is_null() {
        return;
    }

    unsafe {
        let list_box = Box::from_raw(list);
        let _ = Vec::from_raw_parts(list_box.faces, list_box.count, list_box.count);
    }
}

/// Free EXIF data
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_free_exif_data(exif_data: *mut CExifData) {
    if exif_data.is_null() {
        return;
    }

    unsafe {
        let exif_box = Box::from_raw(exif_data);

        // Free all of the strings
        if !exif_box.camera_manufacturer.is_null() {
            let _ = CString::from_raw(exif_box.camera_manufacturer as *mut c_char);
        }
        if !exif_box.camera_model.is_null() {
            let _ = CString::from_raw(exif_box.camera_model as *mut c_char);
        }
        if !exif_box.lens_manufacturer.is_null() {
            let _ = CString::from_raw(exif_box.lens_manufacturer as *mut c_char);
        }
        if !exif_box.lens_model.is_null() {
            let _ = CString::from_raw(exif_box.lens_model as *mut c_char);
        }
        if !exif_box.focal_length.is_null() {
            let _ = CString::from_raw(exif_box.focal_length as *mut c_char);
        }
        if !exif_box.f_number.is_null() {
            let _ = CString::from_raw(exif_box.f_number as *mut c_char);
        }
        if !exif_box.exposure_time.is_null() {
            let _ = CString::from_raw(exif_box.exposure_time as *mut c_char);
        }
        if !exif_box.datetime.is_null() {
            let _ = CString::from_raw(exif_box.datetime as *mut c_char);
        }
    }
}

/// Extract EXIF data from an image file
/// Returns a CExifData pointer that must be freed with chama_optics_free_exif_data
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_extract_exif(image_path: *const c_char) -> *mut CExifData {
    if image_path.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let path_str = match CStr::from_ptr(image_path).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::error!("Invalid UTF-8 in image path");
                return std::ptr::null_mut();
            }
        };

        let path_buf = PathBuf::from(path_str);

        // Load to CoreImage to get EXIF data
        let core_image = match crate::core::CoreImage::from_path(path_buf) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image for EXIF extraction: {}", e);
                return std::ptr::null_mut();
            }
        };

        let exif = &core_image.view_exif;
        let has_exif = !exif.camera_model.is_empty() || !exif.lens_model.is_empty();

        // Helper to create CString or return null pointer
        let to_cstring = |s: &str| -> *const c_char {
            if s.is_empty() {
                std::ptr::null()
            } else {
                CString::new(s)
                    .map(|cs| cs.into_raw())
                    .unwrap_or(core::ptr::null_mut())
            }
        };

        let exif_data = Box::new(CExifData {
            camera_manufacturer: to_cstring(&exif.camera_mnf),
            camera_model: to_cstring(&exif.camera_model),
            lens_manufacturer: to_cstring(&exif.lens_mnf),
            lens_model: to_cstring(&exif.lens_model),
            focal_length: to_cstring(&exif.focal),
            f_number: to_cstring(&exif.fnumber),
            exposure_time: to_cstring(&exif.exposure),
            iso_speed: exif.iso_speed.unwrap_or(0),
            datetime: to_cstring(&exif.datetime),
            has_exif,
        });

        log::info!(
            "Extracted EXIF from {}: Camera={}, Lens={}, ISO={}",
            path_str,
            exif.camera_model,
            exif.lens_model,
            exif.iso_speed.unwrap_or(0)
        );

        Box::into_raw(exif_data)
    }
}

/// Opaque pointer to ChamaOptics instance (now using headless core)
pub struct ChamaOpticsHandle {
    processor: ImageProcessor,
    #[cfg(any(feature = "desktop", feature = "web"))]
    theme_registry: crate::theme::ThemeRegistry,
}

/// Create a new ChamaOptics instance
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_create() -> *mut ChamaOpticsHandle {
    log::info!("Creating ChamaOptics instance");
    #[cfg(any(feature = "desktop", feature = "web"))]
    {
        Box::into_raw(Box::new(ChamaOpticsHandle {
            processor: ImageProcessor::new(),
            theme_registry: crate::theme::ThemeRegistry::new(),
        }))
    }
    #[cfg(not(any(feature = "desktop", feature = "web")))]
    {
        Box::into_raw(Box::new(ChamaOpticsHandle {
            processor: ImageProcessor::new(),
        }))
    }
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

// Face detection FFI functions (for iOS and macOS)
#[cfg(any(
    feature = "desktop",
    feature = "ios_integration",
    feature = "metal_rendering"
))]
mod face_detection_ffi {
    #[allow(unused_imports)]
    use super::*;

    /// Detect faces in an image using VisionKit
    /// Returns a list of face rectangles
    /// The returned list must be freed with chama_optics_free_face_rect_list
    #[cfg(all(target_os = "ios", feature = "face_detection_visionkit"))]
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
#[cfg(target_os = "ios")]
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
        mask_blur_radius: f32,
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

// Face Effect FFI Functions
/// Apply Mosaic effect to detected face areas
#[cfg(any(
    feature = "desktop",
    feature = "ios_integration",
    feature = "metal_rendering"
))]
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
#[cfg(target_os = "ios")]
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
#[cfg(target_os = "ios")]
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

// Theme-related C FFI functions (requires desktop or web features)
#[cfg(any(feature = "desktop", feature = "web"))]
mod theme_ffi {
    use super::*;

    /// Apply theme to image and save
    #[unsafe(no_mangle)]
    pub extern "C" fn chama_optics_apply_theme(
        handle: *mut ChamaOpticsHandle,
        theme_name: *const c_char,
        output_path: *const c_char,
    ) -> bool {
        if handle.is_null() || theme_name.is_null() || output_path.is_null() {
            return false;
        }

        unsafe {
            let theme_str = match CStr::from_ptr(theme_name).to_str() {
                Ok(s) => s,
                Err(_) => {
                    log::error!("Invalid UTF-8 in theme name");
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

            // Apply to first image (index 0)
            if handle_ref.processor.image_count() == 0 {
                log::error!("No images loaded");
                return false;
            }

            // Find theme in registry (which has updated parameters)
            let theme_arc = match handle_ref.theme_registry.themes.iter().find(|t| {
                t.read()
                    .map(|theme| theme.unique_name() == theme_str)
                    .unwrap_or(false)
            }) {
                Some(t) => t.clone(),
                None => {
                    log::error!("Theme '{}' not found in registry", theme_str);
                    return false;
                }
            };

            // Get theme and apply it directly
            let output_path_buf = PathBuf::from(output_str);
            match theme_arc.read() {
                Ok(theme) => {
                    // Apply theme directly to image
                    match handle_ref
                        .processor
                        .apply_theme_direct(0, &*theme, &output_path_buf)
                    {
                        Ok(_) => {
                            log::info!(
                                "Successfully applied theme {} to {}",
                                theme_str,
                                output_str
                            );
                            true
                        }
                        Err(e) => {
                            log::error!("Failed to apply theme: {}", e);
                            false
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to lock theme: {}", e);
                    false
                }
            }
        }
    }

    /// Get list of all available themes
    /// Returns a CThemeList that must be freed with chama_optics_free_theme_list
    #[unsafe(no_mangle)]
    pub extern "C" fn chama_optics_get_themes() -> *mut CThemeList {
        let theme_types = ThemeType::all();
        let count = theme_types.len();

        let mut infos: Vec<CThemeInfo> = theme_types
            .iter()
            .map(|theme_type| {
                let unique_name = CString::new(theme_type.unique_name())
                    .expect("Failed to create unique_name")
                    .into_raw();
                let label = CString::new(theme_type.label())
                    .expect("Failed to create label")
                    .into_raw();

                // Determine if theme has configurable parameters
                let has_parameters = matches!(
                    theme_type,
                    ThemeType::Film
                        | ThemeType::FilmDate
                        | ThemeType::FilmGlow
                        | ThemeType::OneLine
                        | ThemeType::TwoLine
                        | ThemeType::ShotOnOneLine
                        | ThemeType::ShotOnTwoLine
                        | ThemeType::Strap
                        | ThemeType::Monitor
                        | ThemeType::Lightroom
                );

                CThemeInfo {
                    unique_name,
                    label,
                    has_parameters,
                }
            })
            .collect();

        let list = Box::new(CThemeList {
            themes: infos.as_mut_ptr(),
            count,
        });

        // Prevent Vec from being dropped (we're transferring ownership to C)
        std::mem::forget(infos);

        Box::into_raw(list)
    }

    /// Free a theme list returned by chama_optics_get_themes
    #[unsafe(no_mangle)]
    pub extern "C" fn chama_optics_free_theme_list(list: *mut CThemeList) {
        if list.is_null() {
            return;
        }

        unsafe {
            let list_box = Box::from_raw(list);

            // Free all of the strings in theme infos
            for i in 0..list_box.count {
                let info = list_box.themes.add(i);
                if !(*info).unique_name.is_null() {
                    let _ = CString::from_raw((*info).unique_name as *mut c_char);
                }
                if !(*info).label.is_null() {
                    let _ = CString::from_raw((*info).label as *mut c_char);
                }
            }

            // Free to array
            let _ = Vec::from_raw_parts(list_box.themes, list_box.count, list_box.count);
        }
    }

    /// Get theme parameters as JSON string
    /// Returns a JSON string that must be freed with chama_optics_free_string
    #[unsafe(no_mangle)]
    pub extern "C" fn chama_optics_get_theme_parameters(
        handle: *mut ChamaOpticsHandle,
        theme_name: *const c_char,
    ) -> *const c_char {
        if handle.is_null() || theme_name.is_null() {
            return std::ptr::null_mut();
        }

        unsafe {
            let theme_str = match CStr::from_ptr(theme_name).to_str() {
                Ok(s) => s,
                Err(_) => return std::ptr::null_mut(),
            };

            // Use to handle's theme registry
            let handle_ref = &*handle;
            let registry = &handle_ref.theme_registry;

            let json = if let Some(theme) = registry.find(theme_str) {
                // Call to theme's get_parameters_json() method
                let json_result = theme.get_parameters_json();
                println!(
                    "🔍 FFI: Theme '{}' returned JSON: {}",
                    theme_str, json_result
                );
                json_result
            } else {
                log::warn!("Theme '{}' not found in registry", theme_str);
                r#"{"parameters": []}"#.to_string()
            };

            match CString::new(json) {
                Ok(s) => s.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    }

    /// Set a theme parameter value (for future use)
    /// For now, this is a placeholder - actual implementation requires
    /// storing parameter state in ChamaOpticsHandle
    #[unsafe(no_mangle)]
    pub extern "C" fn chama_optics_set_theme_parameter(
        handle: *mut ChamaOpticsHandle,
        theme_name: *const c_char,
        param_name: *const c_char,
        value_json: *const c_char,
    ) -> bool {
        if handle.is_null() || theme_name.is_null() || param_name.is_null() || value_json.is_null()
        {
            return false;
        }

        let _handle_ref = unsafe { &mut *handle };

        let theme_str = match unsafe { CStr::from_ptr(theme_name).to_str() } {
            Ok(s) => s,
            Err(_) => return false,
        };

        let param_str = match unsafe { CStr::from_ptr(param_name).to_str() } {
            Ok(s) => s,
            Err(_) => return false,
        };

        let value_str = match unsafe { CStr::from_ptr(value_json).to_str() } {
            Ok(s) => s,
            Err(_) => return false,
        };

        log::info!(
            "Parameter update: theme={}, param={}, value={}",
            theme_str,
            param_str,
            value_str
        );

        // Parse to JSON value
        let json_value: serde_json::Value = match serde_json::from_str(value_str) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to parse parameter value JSON: {}", e);
                return false;
            }
        };

        // Create a map with single parameter update
        let mut updates = serde_json::Map::new();
        updates.insert(param_str.to_string(), json_value);

        // Find theme in registry and update it
        // For shot_on_one_line, we need to downcast and call update_from_json
        if theme_str == "shot_on_one_line" {
            use crate::theme::parameter_schema::ThemeParameters;
            use crate::theme::shot_on_one_line::ShotOnOneLine;

            // Access to theme from to registry
            for theme_arc in &_handle_ref.theme_registry.themes {
                // Check if this is to right theme
                let is_match = theme_arc
                    .read()
                    .map(|t| t.unique_name() == theme_str)
                    .unwrap_or(false);

                if !is_match {
                    continue;
                }

                // Try to write-lock and update
                if let Ok(mut theme_guard) = theme_arc.write() {
                    // Get to concrete type using Any
                    use std::any::Any;

                    // We need to get a &mut dyn Any from &mut dyn Theme
                    // The trick is to use to fact that Theme now extends Any
                    if let Some(shot_on_one_line) =
                        (&mut *theme_guard as &mut dyn Any).downcast_mut::<ShotOnOneLine>()
                    {
                        return match shot_on_one_line.update_from_json(&updates) {
                            Ok(_) => {
                                log::info!(
                                    "Successfully updated parameter {} for theme {}",
                                    param_str,
                                    theme_str
                                );
                                true
                            }
                            Err(e) => {
                                log::error!("Failed to update parameter: {}", e);
                                false
                            }
                        };
                    } else {
                        log::warn!("Failed to downcast theme {} to ShotOnOneLine", theme_str);
                        return false;
                    }
                }
            }
        }

        // For other themes, we don't have update_from_json implemented yet
        log::warn!(
            "Parameter update not yet implemented for theme: {}",
            theme_str
        );
        true
    }
} // end theme_ffi module

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let version_ptr = chama_optics_version();
        assert!(!version_ptr.is_null());

        unsafe {
            let version = CStr::from_ptr(version_ptr).to_str().unwrap();
            assert!(!version.is_empty());
            chama_optics_free_string(version_ptr as *mut c_char);
        }
    }

    #[test]
    fn test_create_destroy() {
        let handle = chama_optics_create();
        assert!(!handle.is_null());
        chama_optics_destroy(handle);
    }
}

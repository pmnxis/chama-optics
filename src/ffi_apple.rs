/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! FFI (Foreign Function Interface) for Apple platforms (iOS and macOS)
//!
//! This module provides C-compatible functions that work on both iOS and macOS.
//! Platform-specific functions should go in ffi_mobile.rs (iOS/Android).

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ============================================================================
// Common C Structures for Apple Platforms
// ============================================================================

/// C-compatible datetime structure (all 0 if not available)
#[repr(C)]
#[derive(Copy, Clone)]
pub struct CDatetime {
    pub year: u16,  // 0-65535 (more than enough for any realistic year)
    pub month: u8,  // 1-12
    pub day: u8,    // 1-31
    pub hour: u8,   // 0-23
    pub minute: u8, // 0-59
    pub second: u8, // 0-59
}

/// C-compatible EXIF data structure
/// IMPORTANT: Keep field order consistent with Swift definition to avoid alignment issues
#[repr(C)]
pub struct CExifData {
    // Pointer fields first (all same size)
    pub camera_manufacturer: *const c_char,
    pub camera_model: *const c_char,
    pub lens_manufacturer: *const c_char,
    pub lens_model: *const c_char,
    pub focal_length: *const c_char,
    pub f_number: *const c_char,
    pub exposure_time: *const c_char,

    // Non-pointer fields last (grouped by size for alignment)
    pub datetime: CDatetime,
    pub iso_speed: u32, // 0 if not available
    pub has_exif: bool,
}

// ============================================================================
// Library Initialization and Version
// ============================================================================

/// Initialize Chama Optics library
/// Safe to call multiple times (uses try_init internally)
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_init() {
    #[cfg(target_os = "android")]
    {
        let _ = android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("chama_optics"),
        );
    }
    #[cfg(not(target_os = "android"))]
    {
        // Force debug level logging for Apple/desktop platforms
        // Use try_init to avoid panic if already initialized
        let _ = env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }
    log::info!("Chama Optics library initialized with DEBUG logging");
}

/// Get library version string
/// Returns a C string that must be freed with chama_optics_free_string()
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_version() -> *const c_char {
    let version = CString::new(env!("CARGO_PKG_VERSION")).expect("Failed to create version string");
    version.into_raw()
}

// ============================================================================
// Memory Management
// ============================================================================

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

/// Free EXIF data structure
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_free_exif_data(exif_data: *mut CExifData) {
    if !exif_data.is_null() {
        unsafe {
            let data = Box::from_raw(exif_data);

            // Free all string fields
            if !data.camera_manufacturer.is_null() {
                let _ = CString::from_raw(data.camera_manufacturer as *mut c_char);
            }
            if !data.camera_model.is_null() {
                let _ = CString::from_raw(data.camera_model as *mut c_char);
            }
            if !data.lens_manufacturer.is_null() {
                let _ = CString::from_raw(data.lens_manufacturer as *mut c_char);
            }
            if !data.lens_model.is_null() {
                let _ = CString::from_raw(data.lens_model as *mut c_char);
            }
            if !data.focal_length.is_null() {
                let _ = CString::from_raw(data.focal_length as *mut c_char);
            }
            if !data.f_number.is_null() {
                let _ = CString::from_raw(data.f_number as *mut c_char);
            }
            if !data.exposure_time.is_null() {
                let _ = CString::from_raw(data.exposure_time as *mut c_char);
            }
        }
    }
}

// ============================================================================
// EXIF Extraction (Platform-Agnostic)
// ============================================================================

/// Extract EXIF data from image
/// Returns a pointer to CExifData that must be freed with chama_optics_free_exif_data()
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_extract_exif(
    image_path: *const c_char,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
) -> *mut CExifData {
    if image_path.is_null() {
        log::error!("Null image path provided to extract_exif");
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

        log::info!("Extracting EXIF from: {}", path_str);

        // Open file and create buffered reader
        let file = match std::fs::File::open(path_str) {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to open image file: {}", e);
                return std::ptr::null_mut();
            }
        };

        let mut buf_reader = std::io::BufReader::new(file);

        // Parse EXIF data
        let exif = match exif::Reader::new().read_from_container(&mut buf_reader) {
            Ok(e) => e,
            Err(e) => {
                log::warn!("No EXIF data found: {}", e);
                // Return empty EXIF structure
                return Box::into_raw(Box::new(CExifData {
                    camera_manufacturer: std::ptr::null(),
                    camera_model: std::ptr::null(),
                    lens_manufacturer: std::ptr::null(),
                    lens_model: std::ptr::null(),
                    focal_length: std::ptr::null(),
                    f_number: std::ptr::null(),
                    exposure_time: std::ptr::null(),
                    datetime: CDatetime {
                        year: 0,
                        month: 0,
                        day: 0,
                        hour: 0,
                        minute: 0,
                        second: 0,
                    },
                    iso_speed: 0,
                    has_exif: false,
                }));
            }
        };

        // Helper function to get field as string (with EXIF cleanup)
        let get_field_string = |tag: exif::Tag| -> *const c_char {
            exif.get_field(tag, exif::In::PRIMARY)
                .and_then(|field| {
                    let raw = field.display_value().to_string();
                    let cleaned = crate::exif_impl::simplify_exif_string(&raw);
                    CString::new(cleaned)
                        .ok()
                        .map(|s| s.into_raw() as *const c_char)
                })
                .unwrap_or(std::ptr::null())
        };

        // Extract camera info
        let camera_manufacturer = get_field_string(exif::Tag::Make);
        let camera_model = get_field_string(exif::Tag::Model);
        let lens_manufacturer = get_field_string(exif::Tag::LensMake);
        let lens_model = get_field_string(exif::Tag::LensModel);

        // Extract focal length (choose 35mm equivalent if requested)
        let focal_length = if use_35mm_focal_length {
            exif.get_field(exif::Tag::FocalLengthIn35mmFilm, exif::In::PRIMARY)
                .and_then(|field| {
                    CString::new(field.display_value().to_string())
                        .ok()
                        .map(|s| s.into_raw() as *const c_char)
                })
                .unwrap_or_else(|| get_field_string(exif::Tag::FocalLength))
        } else {
            get_field_string(exif::Tag::FocalLength)
        };

        // Extract F-number (choose alternative if requested)
        let f_number = if get_alt_fnumber {
            exif.get_field(exif::Tag::MaxApertureValue, exif::In::PRIMARY)
                .and_then(|field| {
                    CString::new(field.display_value().to_string())
                        .ok()
                        .map(|s| s.into_raw() as *const c_char)
                })
                .unwrap_or_else(|| get_field_string(exif::Tag::FNumber))
        } else {
            get_field_string(exif::Tag::FNumber)
        };

        let exposure_time = get_field_string(exif::Tag::ExposureTime);

        // Extract ISO
        let iso_speed = exif
            .get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY)
            .and_then(|field| field.value.get_uint(0))
            .unwrap_or(0);

        // Extract datetime
        let datetime = exif
            .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
            .or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY))
            .and_then(|field| {
                if let exif::Value::Ascii(ref vec) = field.value {
                    if let Some(datetime_bytes) = vec.first() {
                        let datetime_str = String::from_utf8_lossy(datetime_bytes);
                        parse_exif_datetime(&datetime_str)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or(CDatetime {
                year: 0,
                month: 0,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
            });

        Box::into_raw(Box::new(CExifData {
            camera_manufacturer,
            camera_model,
            lens_manufacturer,
            lens_model,
            focal_length,
            f_number,
            exposure_time,
            datetime,
            iso_speed,
            has_exif: true,
        }))
    }
}

/// Extract verbose EXIF data as JSON string
/// Returns a JSON string that must be freed with chama_optics_free_string()
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn chama_optics_extract_verbose_exif(image_path: *const c_char) -> *mut c_char {
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

        match crate::image::exif_impl::extract_verbose_exif(path_str) {
            Ok(json_string) => match CString::new(json_string) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => {
                    log::error!("Failed to create C string from JSON");
                    std::ptr::null_mut()
                }
            },
            Err(e) => {
                log::error!("Failed to extract verbose EXIF: {}", e);
                std::ptr::null_mut()
            }
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse EXIF datetime string (format: "YYYY:MM:DD HH:MM:SS")
fn parse_exif_datetime(datetime_str: &str) -> Option<CDatetime> {
    let parts: Vec<&str> = datetime_str.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }

    let date_parts: Vec<&str> = parts[0].split(':').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();

    if date_parts.len() != 3 || time_parts.len() != 3 {
        return None;
    }

    Some(CDatetime {
        year: date_parts[0].parse().ok()?,
        month: date_parts[1].parse().ok()?,
        day: date_parts[2].parse().ok()?,
        hour: time_parts[0].parse().ok()?,
        minute: time_parts[1].parse().ok()?,
        second: time_parts[2].parse().ok()?,
    })
}

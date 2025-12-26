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

use crate::core::{ImageProcessor, ThemeConfig, ThemeType};

/// Initialize the Chama Optics library
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_init() {
    // Force debug level logging for iOS
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
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
pub extern "C" fn chama_optics_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// Opaque pointer to ChamaOptics instance (now using headless core)
pub struct ChamaOpticsHandle {
    processor: ImageProcessor,
}

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

        // Parse theme type from name
        let theme_type = match ThemeType::from_name(theme_str) {
            Some(t) => t,
            None => {
                log::error!("Unknown theme: {}", theme_str);
                return false;
            }
        };

        // Set the theme
        let theme_config = ThemeConfig::new(theme_type);
        handle_ref.processor.set_theme(theme_config);

        // Apply to the first image (index 0)
        // In a full implementation, you might want to track which image to process
        if handle_ref.processor.image_count() == 0 {
            log::error!("No images loaded");
            return false;
        }

        let output_path_buf = PathBuf::from(output_str);
        match handle_ref
            .processor
            .apply_theme_to_image(0, &output_path_buf)
        {
            Ok(_) => {
                log::info!("Successfully applied theme {} to {}", theme_str, output_str);
                true
            }
            Err(e) => {
                log::error!("Failed to apply theme: {}", e);
                false
            }
        }
    }
}

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

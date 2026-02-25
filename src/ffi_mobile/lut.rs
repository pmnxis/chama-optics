/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! LUT (Color Grading) FFI functions for mobile platforms.
//!
//! Provides C-compatible functions for managing LUT files:
//! adding, removing, applying color grading, and persisting state.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Mutex;

use super::{COutputFormat, ChamaError, read_exif_orientation, save_image_with_c_format};

lazy_static::lazy_static! {
    /// Global LUT storage for iOS
    /// Uses a Mutex to allow thread-safe access from Swift
    pub(super) static ref LUT_STORAGE: Mutex<crate::effect::lut_storage::LutStorage> = {
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

        // Initialize built-in LUTs (skips items already registered)
        let added = crate::builtins::lut_presets::init_builtin_luts(&mut storage);
        log::info!(
            "LUT storage initialized with {} LUTs ({} built-in added)",
            storage.luts.len(),
            added
        );
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
        .filter(|lut| !lut.is_hidden)
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
                "is_builtin": lut.is_builtin,
                "is_hidden": lut.is_hidden,
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

    let name_str = cstr_to_str!(name, return std::ptr::null_mut());
    let path_str = cstr_to_str!(source_path, return std::ptr::null_mut());

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

    let id_str = cstr_to_str!(lut_id, return false);

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

    let id_str = cstr_to_str!(lut_id, return ChamaError::InvalidPath);
    let image_path_str = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);

    let uuid = match uuid::Uuid::parse_str(id_str) {
        Ok(u) => u,
        Err(e) => {
            log::error!("Invalid LUT UUID: {}", e);
            return ChamaError::InvalidParameters;
        }
    };

    log::info!("Applying LUT {} to image: {}", id_str, image_path_str);

    // Load image with EXIF orientation
    let mut dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };
    dyn_image.apply_orientation(read_exif_orientation(image_path_str));

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

    let image_path_str = cstr_to_str!(image_path, return ChamaError::InvalidPath);
    let output_path_str = cstr_to_str!(output_path, return ChamaError::InvalidPath);

    let total_start = std::time::Instant::now();
    log::info!(
        "⏱️ [LUT-PERF] chama_lut_apply_with_format START — image: {}",
        image_path_str
    );

    // Load image with EXIF orientation
    let t0 = std::time::Instant::now();
    let mut dyn_image = match image::open(image_path_str) {
        Ok(img) => img,
        Err(e) => {
            log::error!("Failed to load image: {}", e);
            return ChamaError::ImageLoadError;
        }
    };
    let load_ms = t0.elapsed().as_millis();
    log::info!(
        "⏱️ [LUT-PERF] image::open() = {}ms ({}x{})",
        load_ms,
        dyn_image.width(),
        dyn_image.height()
    );

    let t1 = std::time::Instant::now();
    dyn_image.apply_orientation(read_exif_orientation(image_path_str));
    let orient_ms = t1.elapsed().as_millis();
    log::info!("⏱️ [LUT-PERF] apply_orientation = {}ms", orient_ms);

    // Apply LUT if specified
    if !lut_id.is_null() {
        let id_str = cstr_to_str!(lut_id, return ChamaError::InvalidParameters);

        if !id_str.is_empty() {
            let uuid = match uuid::Uuid::parse_str(id_str) {
                Ok(u) => u,
                Err(e) => {
                    log::error!("Invalid LUT UUID: {}", e);
                    return ChamaError::InvalidParameters;
                }
            };

            let t2 = std::time::Instant::now();
            let mut storage = match LUT_STORAGE.lock() {
                Ok(s) => s,
                Err(_) => return ChamaError::Unknown,
            };
            let lock_ms = t2.elapsed().as_millis();
            log::info!("⏱️ [LUT-PERF] LUT_STORAGE.lock() = {}ms", lock_ms);

            let t3 = std::time::Instant::now();
            if !storage.apply_lut_to_image(uuid, &mut dyn_image) {
                log::error!("Failed to apply LUT");
                return ChamaError::ImageProcessError;
            }
            let apply_ms = t3.elapsed().as_millis();
            log::info!("⏱️ [LUT-PERF] apply_lut_to_image = {}ms", apply_ms);
        }
    }

    // Save with specified format
    let t4 = std::time::Instant::now();
    match save_image_with_c_format(&dyn_image, output_path_str, output_format, quality) {
        Ok(_) => {
            let save_ms = t4.elapsed().as_millis();
            let total_ms = total_start.elapsed().as_millis();
            log::info!(
                "⏱️ [LUT-PERF] save_image = {}ms (format={:?}, quality={})",
                save_ms,
                output_format,
                quality
            );
            log::info!("⏱️ [LUT-PERF] TOTAL = {}ms", total_ms);
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

            // Initialize built-in LUTs (adds any that weren't in saved state)
            let added = crate::builtins::lut_presets::init_builtin_luts(&mut storage);

            log::info!(
                "Loaded LUT state: {} LUTs ({} built-in added)",
                storage.luts.len(),
                added
            );
            true
        }
        Err(e) => {
            log::error!("Failed to parse LUT state: {}", e);
            false
        }
    }
}

/// Restore all hidden built-in LUTs
/// Returns the number of LUTs restored, or -1 on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn chama_lut_restore_builtins() -> i32 {
    let mut storage = match LUT_STORAGE.lock() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let restored = storage.restore_builtin_luts() as i32;
    log::info!("Restored {} hidden built-in LUTs", restored);
    restored
}

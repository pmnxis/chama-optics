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

use crate::core::{ImageProcessor, ThemeType};

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
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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
    theme_registry: crate::theme::ThemeRegistry,
}

/// Create a new ChamaOptics instance
#[unsafe(no_mangle)]
pub extern "C" fn chama_optics_create() -> *mut ChamaOpticsHandle {
    log::info!("Creating ChamaOptics instance");
    Box::into_raw(Box::new(ChamaOpticsHandle {
        processor: ImageProcessor::new(),
        theme_registry: crate::theme::ThemeRegistry::new(),
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

            // Apply to the first image (index 0)
            if handle_ref.processor.image_count() == 0 {
                log::error!("No images loaded");
                return false;
            }

            // Find the theme in the registry (which has updated parameters)
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

            // Get the theme and apply it directly
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

            // Free all the strings in the theme infos
            for i in 0..list_box.count {
                let info = list_box.themes.add(i);
                if !(*info).unique_name.is_null() {
                    let _ = CString::from_raw((*info).unique_name as *mut c_char);
                }
                if !(*info).label.is_null() {
                    let _ = CString::from_raw((*info).label as *mut c_char);
                }
            }

            // Free the array
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
            return std::ptr::null();
        }

        unsafe {
            let theme_str = match CStr::from_ptr(theme_name).to_str() {
                Ok(s) => s,
                Err(_) => return std::ptr::null(),
            };

            // Use the handle's theme registry
            let handle_ref = &*handle;
            let registry = &handle_ref.theme_registry;

            let json = if let Some(theme) = registry.find(theme_str) {
                // Call the theme's get_parameters_json() method
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
                Err(_) => std::ptr::null(),
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

        // Parse the JSON value
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

        // Find the theme in the registry and update it
        // For shot_on_one_line, we need to downcast and call update_from_json
        if theme_str == "shot_on_one_line" {
            use crate::theme::parameter_schema::ThemeParameters;
            use crate::theme::shot_on_one_line::ShotOnOneLine;

            // Access the theme from the registry
            for theme_arc in &_handle_ref.theme_registry.themes {
                // Check if this is the right theme
                let is_match = theme_arc
                    .read()
                    .map(|t| t.unique_name() == theme_str)
                    .unwrap_or(false);

                if !is_match {
                    continue;
                }

                // Try to write-lock and update
                if let Ok(mut theme_guard) = theme_arc.write() {
                    // Get the concrete type using Any
                    use std::any::Any;

                    // We need to get a &mut dyn Any from &mut dyn Theme
                    // The trick is to use the fact that Theme now extends Any
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

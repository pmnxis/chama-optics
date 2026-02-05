/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Apple native HEIF/HEIC image decoding for iOS and macOS
//!
//! This module uses Apple's ImageIO framework to decode HEIF images natively,
//! avoiding the need for libheif which doesn't work on iOS.

use std::path::Path;

// External C declarations for Apple's ImageIO framework
#[allow(unused)]
unsafe extern "C" {
    // CFData
    fn CFDataCreate(
        allocator: *const std::ffi::c_void,
        bytes: *const u8,
        length: isize,
    ) -> *const std::ffi::c_void;
    fn CFRelease(cf: *const std::ffi::c_void);

    // CFString (for creating dictionary keys)
    fn CFStringCreateWithCString(
        allocator: *const std::ffi::c_void,
        c_str: *const i8,
        encoding: u32,
    ) -> *const std::ffi::c_void;

    // CFDictionary
    fn CFDictionaryCreate(
        allocator: *const std::ffi::c_void,
        keys: *const *const std::ffi::c_void,
        values: *const *const std::ffi::c_void,
        num_values: isize,
        key_callbacks: *const std::ffi::c_void,
        value_callbacks: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;

    // CFBoolean
    static kCFBooleanTrue: *const std::ffi::c_void;

    // CFNumber
    fn CFNumberCreate(
        allocator: *const std::ffi::c_void,
        the_type: i32,
        value_ptr: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;

    // CGImageSource
    fn CGImageSourceCreateWithData(
        data: *const std::ffi::c_void,
        options: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;

    fn CGImageSourceCopyPropertiesAtIndex(
        isrc: *const std::ffi::c_void,
        index: usize,
        options: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;

    fn CGImageSourceCreateImageAtIndex(
        isrc: *const std::ffi::c_void,
        index: usize,
        options: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;

    // CFDictionary accessors
    fn CFDictionaryGetValue(
        the_dict: *const std::ffi::c_void,
        key: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;

    // CFNumber accessors
    fn CFNumberGetValue(
        number: *const std::ffi::c_void,
        the_type: i32,
        value_ptr: *mut std::ffi::c_void,
    ) -> bool;

    // CGImage
    fn CGImageGetWidth(image: *const std::ffi::c_void) -> usize;
    fn CGImageGetHeight(image: *const std::ffi::c_void) -> usize;
    fn CGImageGetBitsPerComponent(image: *const std::ffi::c_void) -> usize;
    fn CGImageGetBytesPerRow(image: *const std::ffi::c_void) -> usize;

    // CGColorSpace
    fn CGColorSpaceCreateDeviceRGB() -> *const std::ffi::c_void;

    // CGContext
    fn CGBitmapContextCreate(
        data: *mut u8,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        color_space: *const std::ffi::c_void,
        bitmap_info: u32,
    ) -> *const std::ffi::c_void;

    fn CGContextDrawImage(
        context: *const std::ffi::c_void,
        rect: CGRect,
        image: *const std::ffi::c_void,
    );
}

#[repr(C)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[repr(C)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
struct CGSize {
    width: f64,
    height: f64,
}

const K_CG_IMAGE_ALPHA_PREMULTIPLIED_LAST: u32 = 1;
const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;
const K_CF_NUMBER_INT_TYPE: i32 = 9; // CFNumberType for int

// EXIF orientation values
const EXIF_ORIENTATION_UP: i32 = 1; // Normal
const EXIF_ORIENTATION_UP_MIRRORED: i32 = 2;
const EXIF_ORIENTATION_DOWN: i32 = 3; // 180 degree rotation
const EXIF_ORIENTATION_DOWN_MIRRORED: i32 = 4;
const EXIF_ORIENTATION_LEFT_MIRRORED: i32 = 5;
const EXIF_ORIENTATION_RIGHT: i32 = 6; // 90 degree CW rotation
const EXIF_ORIENTATION_RIGHT_MIRRORED: i32 = 7;
const EXIF_ORIENTATION_LEFT: i32 = 8; // 90 degree CCW rotation

/// Get EXIF orientation from CGImageSource
unsafe fn get_image_orientation(image_source: *const std::ffi::c_void) -> i32 {
    unsafe {
        // Get image properties
        let properties = CGImageSourceCopyPropertiesAtIndex(image_source, 0, std::ptr::null());
        if properties.is_null() {
            return EXIF_ORIENTATION_UP; // Default to normal orientation
        }

        // Create CFString for "Orientation" key
        #[allow(clippy::manual_c_str_literals)]
        let orientation_key = CFStringCreateWithCString(
            std::ptr::null(),
            b"Orientation\0".as_ptr() as *const i8,
            K_CF_STRING_ENCODING_UTF8,
        );

        if orientation_key.is_null() {
            CFRelease(properties);
            return EXIF_ORIENTATION_UP;
        }

        // Get orientation value from dictionary
        let orientation_value = CFDictionaryGetValue(properties, orientation_key);
        CFRelease(orientation_key);

        let mut orientation: i32 = EXIF_ORIENTATION_UP;
        if !orientation_value.is_null() {
            CFNumberGetValue(
                orientation_value,
                K_CF_NUMBER_INT_TYPE,
                &mut orientation as *mut i32 as *mut std::ffi::c_void,
            );
        }

        CFRelease(properties);

        log::debug!("📐 EXIF Orientation: {}", orientation);
        orientation
    }
}

/// Decode HEIF image from file path using Apple's native ImageIO
pub fn decode_heif(path: &Path) -> Result<image::DynamicImage, String> {
    log::info!("Decoding HEIF using Apple native ImageIO: {:?}", path);

    // Read file data
    let file_data = std::fs::read(path).map_err(|e| format!("Failed to read HEIF file: {}", e))?;

    decode_heif_from_data(&file_data)
}

/// Decode HEIF image from raw bytes using Apple's native ImageIO
pub fn decode_heif_from_data(data: &[u8]) -> Result<image::DynamicImage, String> {
    unsafe {
        // Create CFData from bytes
        let cf_data = CFDataCreate(std::ptr::null(), data.as_ptr(), data.len() as isize);
        if cf_data.is_null() {
            return Err("Failed to create CFData".to_string());
        }

        // Create CGImageSource
        let image_source = CGImageSourceCreateWithData(cf_data, std::ptr::null());
        CFRelease(cf_data);

        if image_source.is_null() {
            return Err("Failed to create CGImageSource".to_string());
        }

        // Get EXIF orientation before creating the image
        let orientation = get_image_orientation(image_source);

        // Create CGImage from source (first image)
        let cg_image = CGImageSourceCreateImageAtIndex(image_source, 0, std::ptr::null());
        CFRelease(image_source);

        if cg_image.is_null() {
            return Err("Failed to create CGImage from source".to_string());
        }

        // Get image dimensions
        let width = CGImageGetWidth(cg_image);
        let height = CGImageGetHeight(cg_image);
        let bytes_per_row = width * 4; // RGBA

        log::info!("HEIF image dimensions: {}x{}", width, height);

        // Allocate buffer for RGBA data
        let buffer_size = height * bytes_per_row;
        let mut buffer = vec![0u8; buffer_size];

        // Create color space
        let color_space = CGColorSpaceCreateDeviceRGB();
        if color_space.is_null() {
            CFRelease(cg_image);
            return Err("Failed to create color space".to_string());
        }

        // Create bitmap context
        let context = CGBitmapContextCreate(
            buffer.as_mut_ptr(),
            width,
            height,
            8, // bits per component
            bytes_per_row,
            color_space,
            K_CG_IMAGE_ALPHA_PREMULTIPLIED_LAST,
        );

        CFRelease(color_space);

        if context.is_null() {
            CFRelease(cg_image);
            return Err("Failed to create bitmap context".to_string());
        }

        // Draw image to context
        let rect = CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize {
                width: width as f64,
                height: height as f64,
            },
        };

        CGContextDrawImage(context, rect, cg_image);

        // Clean up
        CFRelease(context);
        CFRelease(cg_image);

        // Convert RGBA buffer to image::DynamicImage
        let rgba_image = image::RgbaImage::from_raw(width as u32, height as u32, buffer)
            .ok_or_else(|| "Failed to create RgbaImage from buffer".to_string())?;

        let mut dynamic_image = image::DynamicImage::ImageRgba8(rgba_image);

        // Apply EXIF orientation transformation
        dynamic_image = match orientation {
            EXIF_ORIENTATION_UP => {
                // No rotation needed
                log::debug!("No rotation needed (orientation = 1)");
                dynamic_image
            }
            EXIF_ORIENTATION_DOWN => {
                // 180 degree rotation
                log::debug!("Rotating 180 degrees (orientation = 3)");
                dynamic_image.rotate180()
            }
            EXIF_ORIENTATION_RIGHT => {
                // 90 degree CW rotation (which is 270 CCW in image crate terms)
                log::debug!("Rotating 90° CW / 270° CCW (orientation = 6)");
                dynamic_image.rotate270()
            }
            EXIF_ORIENTATION_LEFT => {
                // 90 degree CCW rotation (which is 90 in image crate terms)
                log::debug!("Rotating 90° CCW (orientation = 8)");
                dynamic_image.rotate90()
            }
            EXIF_ORIENTATION_UP_MIRRORED => {
                log::debug!("Flipping horizontally (orientation = 2)");
                dynamic_image.fliph()
            }
            EXIF_ORIENTATION_DOWN_MIRRORED => {
                log::debug!("Rotating 180° + flip (orientation = 4)");
                dynamic_image.rotate180().fliph()
            }
            EXIF_ORIENTATION_LEFT_MIRRORED => {
                log::debug!("Rotating 90° CCW + flip (orientation = 5)");
                dynamic_image.rotate90().fliph()
            }
            EXIF_ORIENTATION_RIGHT_MIRRORED => {
                log::debug!("Rotating 270° CCW + flip (orientation = 7)");
                dynamic_image.rotate270().fliph()
            }
            _ => {
                log::warn!("⚠️ Unknown orientation value: {}, using as-is", orientation);
                dynamic_image
            }
        };

        log::info!(
            "Successfully decoded HEIF image using Apple native APIs with orientation applied"
        );
        Ok(dynamic_image)
    }
}

/// Check if a file is HEIF/HEIC format by extension
pub fn is_heif_format(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        matches!(ext_lower.as_str(), "heic" | "heif")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_heif_format() {
        assert!(is_heif_format(Path::new("test.heic")));
        assert!(is_heif_format(Path::new("test.HEIC")));
        assert!(is_heif_format(Path::new("test.heif")));
        assert!(is_heif_format(Path::new("test.HEIF")));
        assert!(!is_heif_format(Path::new("test.jpg")));
        assert!(!is_heif_format(Path::new("test.png")));
    }
}

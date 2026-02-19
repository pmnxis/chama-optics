/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Synthetic test image generators and pixel comparison utilities.
//!
//! Used by integration tests to generate deterministic test images without
//! requiring external image files. All generated images are reproducible.

use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use sha2::{Digest, Sha256};

/// Generate a horizontal gradient image.
///
/// R channel = x/width, G channel = y/height, B = 128, A = 255.
/// Deterministic for a given (w, h).
pub fn make_gradient(w: u32, h: u32) -> DynamicImage {
    let img: RgbaImage = ImageBuffer::from_fn(w, h, |x, y| {
        let r = ((x as f32 / w.max(1) as f32) * 255.0) as u8;
        let g = ((y as f32 / h.max(1) as f32) * 255.0) as u8;
        Rgba([r, g, 128, 255])
    });
    DynamicImage::ImageRgba8(img)
}

/// Generate a checkerboard image with alternating black/white cells.
///
/// Cell color alternates: even cells = white (255, 255, 255), odd = dark gray (40, 40, 40).
pub fn make_checkerboard(w: u32, h: u32, cell: u32) -> DynamicImage {
    let cell = cell.max(1);
    let img: RgbaImage = ImageBuffer::from_fn(w, h, |x, y| {
        let cx = x / cell;
        let cy = y / cell;
        if (cx + cy).is_multiple_of(2) {
            Rgba([255, 255, 255, 255])
        } else {
            Rgba([40, 40, 40, 255])
        }
    });
    DynamicImage::ImageRgba8(img)
}

/// Generate a solid-color image.
pub fn make_solid(w: u32, h: u32, r: u8, g: u8, b: u8) -> DynamicImage {
    let img: RgbaImage = ImageBuffer::from_pixel(w, h, Rgba([r, g, b, 255]));
    DynamicImage::ImageRgba8(img)
}

/// Compute SHA-256 hash of raw RGBA pixel bytes.
///
/// The hash is stable for the same image regardless of how it was constructed.
/// Converts to RGBA8 before hashing to normalize the pixel format.
pub fn pixel_hash(img: &DynamicImage) -> String {
    let rgba = img.to_rgba8();
    let mut hasher = Sha256::new();
    hasher.update(rgba.width().to_le_bytes());
    hasher.update(rgba.height().to_le_bytes());
    hasher.update(rgba.as_raw());
    format!("{:x}", hasher.finalize())
}

/// Assert that two images produce the same pixel hash.
///
/// On failure, prints the dimensions and hashes for both images.
#[track_caller]
pub fn assert_images_identical(label_a: &str, a: &DynamicImage, label_b: &str, b: &DynamicImage) {
    let hash_a = pixel_hash(a);
    let hash_b = pixel_hash(b);
    assert_eq!(
        hash_a,
        hash_b,
        "Image pixel mismatch!\n  {}: {}x{} hash={}\n  {}: {}x{} hash={}",
        label_a,
        a.width(),
        a.height(),
        hash_a,
        label_b,
        b.width(),
        b.height(),
        hash_b,
    );
}

/// Assert that running the same operation twice produces identical output.
#[track_caller]
pub fn assert_deterministic<F>(label: &str, f: F)
where
    F: Fn() -> DynamicImage,
{
    let a = f();
    let b = f();
    assert_images_identical(
        &format!("{} run1", label),
        &a,
        &format!("{} run2", label),
        &b,
    );
}

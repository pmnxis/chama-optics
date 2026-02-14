// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Crop and rotate transform module
//!
//! Provides per-image crop and rotation transforms that are applied
//! after EXIF orientation correction but before theme rendering.
//! Face detection runs on the original image; face coordinates are
//! transformed through this module when applying effects.

use image::DynamicImage;
use serde::{Deserialize, Serialize};

/// Normalized rectangle with coordinates in 0.0-1.0 range relative to image dimensions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl NormalizedRect {
    pub fn full() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }
}

/// Per-image crop and rotation transform
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CropRotateTransform {
    /// Free-angle rotation in degrees (-45.0 to +45.0)
    pub rotation_degrees: f32,
    /// Number of 90-degree clockwise rotations applied (0, 1, 2, 3)
    pub rotation_90_count: u8,
    /// Crop rectangle in normalized coordinates (0.0-1.0 relative to image after rotation)
    /// None means no crop (use full image)
    pub crop_rect: Option<NormalizedRect>,
}

impl Default for CropRotateTransform {
    fn default() -> Self {
        Self {
            rotation_degrees: 0.0,
            rotation_90_count: 0,
            crop_rect: None,
        }
    }
}

impl CropRotateTransform {
    /// Compute a content hash for cache invalidation.
    /// Changes to any transform parameter will produce a different hash.
    pub fn content_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.rotation_degrees.to_bits().hash(&mut hasher);
        self.rotation_90_count.hash(&mut hasher);
        if let Some(ref rect) = self.crop_rect {
            1u8.hash(&mut hasher);
            rect.x.to_bits().hash(&mut hasher);
            rect.y.to_bits().hash(&mut hasher);
            rect.width.to_bits().hash(&mut hasher);
            rect.height.to_bits().hash(&mut hasher);
        } else {
            0u8.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Check if this transform is a no-op (identity)
    pub fn is_identity(&self) -> bool {
        self.rotation_degrees == 0.0
            && self.rotation_90_count.is_multiple_of(4)
            && self.crop_rect.is_none()
    }

    /// Apply this transform to an image.
    /// Order: 90-degree rotations -> free-angle rotation -> crop
    pub fn apply(&self, image: &DynamicImage) -> DynamicImage {
        if self.is_identity() {
            return image.clone();
        }

        let mut result = image.clone();

        // Step 1: Apply 90-degree rotations (fast, lossless pixel rearrangement)
        match self.rotation_90_count % 4 {
            1 => result = result.rotate90(),
            2 => result = result.rotate180(),
            3 => result = result.rotate270(),
            _ => {}
        }

        // Step 2: Apply free-angle rotation
        if self.rotation_degrees.abs() > 0.01 {
            result = self.apply_free_rotation(result);
        }

        // Step 3: Apply crop
        if let Some(ref crop) = self.crop_rect {
            let w = result.width();
            let h = result.height();
            let cx = (crop.x * w as f32).round() as u32;
            let cy = (crop.y * h as f32).round() as u32;
            let cw = (crop.width * w as f32).round().max(1.0) as u32;
            let ch = (crop.height * h as f32).round().max(1.0) as u32;

            // Clamp to image bounds
            let cx = cx.min(w.saturating_sub(1));
            let cy = cy.min(h.saturating_sub(1));
            let cw = cw.min(w - cx);
            let ch = ch.min(h - cy);

            if cw > 0 && ch > 0 {
                result = result.crop_imm(cx, cy, cw, ch);
            }
        }

        result
    }

    /// Apply free-angle rotation using imageproc
    fn apply_free_rotation(&self, image: DynamicImage) -> DynamicImage {
        use image::Rgba;
        use imageproc::geometric_transformations::{Interpolation, rotate_about_center};

        let rgba = image.to_rgba8();
        let radians = self.rotation_degrees.to_radians();
        let default_pixel = Rgba([0u8, 0, 0, 0]); // transparent background

        let rotated = rotate_about_center(&rgba, radians, Interpolation::Bilinear, default_pixel);
        DynamicImage::ImageRgba8(rotated)
    }

    /// Transform face coordinates from original image space to crop/rotated image space.
    /// Returns None if the face is entirely outside the crop area after transformation.
    ///
    /// `orig_w` and `orig_h` are the dimensions of the image AFTER EXIF orientation
    /// but BEFORE this crop/rotate transform.
    pub fn transform_face_coords(
        &self,
        face: (i32, i32, u32, u32),
        orig_w: u32,
        orig_h: u32,
    ) -> Option<(i32, i32, u32, u32)> {
        if self.is_identity() {
            return Some(face);
        }

        let (mut fx, mut fy, mut fw, mut fh) =
            (face.0 as f64, face.1 as f64, face.2 as f64, face.3 as f64);
        let mut img_w = orig_w as f64;
        let mut img_h = orig_h as f64;

        // Step 1: Transform through 90-degree rotations
        match self.rotation_90_count % 4 {
            1 => {
                // 90 CW: (x,y) -> (h-y-fh, x), dims swap
                let new_x = img_h - fy - fh;
                let new_y = fx;
                fx = new_x;
                fy = new_y;
                std::mem::swap(&mut fw, &mut fh);
                std::mem::swap(&mut img_w, &mut img_h);
            }
            2 => {
                // 180: (x,y) -> (w-x-fw, h-y-fh)
                fx = img_w - fx - fw;
                fy = img_h - fy - fh;
            }
            3 => {
                // 270 CW (= 90 CCW): (x,y) -> (y, w-x-fw), dims swap
                let new_x = fy;
                let new_y = img_w - fx - fw;
                fx = new_x;
                fy = new_y;
                std::mem::swap(&mut fw, &mut fh);
                std::mem::swap(&mut img_w, &mut img_h);
            }
            _ => {}
        }

        // Step 2: Transform through free-angle rotation
        if self.rotation_degrees.abs() > 0.01 {
            let radians = (self.rotation_degrees as f64).to_radians();
            let cos_a = radians.cos();
            let sin_a = radians.sin();

            // The rotated image may be larger than the original.
            // imageproc::rotate_about_center keeps the same dimensions but the
            // content shifts. We need to compute the center-based rotation.
            let cx = img_w / 2.0;
            let cy = img_h / 2.0;

            // Rotate face center
            let face_cx = fx + fw / 2.0 - cx;
            let face_cy = fy + fh / 2.0 - cy;
            let new_cx = face_cx * cos_a - face_cy * sin_a + cx;
            let new_cy = face_cx * sin_a + face_cy * cos_a + cy;

            // Approximate rotated bounding box (conservative)
            let half_diag = (fw * fw + fh * fh).sqrt() / 2.0;
            let new_half_w = (fw * cos_a.abs() + fh * sin_a.abs()) / 2.0;
            let new_half_h = (fw * sin_a.abs() + fh * cos_a.abs()) / 2.0;

            fx = new_cx - new_half_w;
            fy = new_cy - new_half_h;
            fw = new_half_w * 2.0;
            fh = new_half_h * 2.0;

            // Note: img_w/img_h stay the same since rotate_about_center
            // produces same-sized output
            let _ = half_diag; // suppress unused warning
        }

        // Step 3: Transform through crop
        if let Some(ref crop) = self.crop_rect {
            let crop_x = crop.x as f64 * img_w;
            let crop_y = crop.y as f64 * img_h;
            let crop_w = crop.width as f64 * img_w;
            let crop_h = crop.height as f64 * img_h;

            // Translate to crop coordinate space
            fx -= crop_x;
            fy -= crop_y;

            // Check if face is completely outside crop area
            if fx + fw <= 0.0 || fy + fh <= 0.0 || fx >= crop_w || fy >= crop_h {
                return None;
            }

            // Clamp to crop bounds
            if fx < 0.0 {
                fw += fx;
                fx = 0.0;
            }
            if fy < 0.0 {
                fh += fy;
                fy = 0.0;
            }
            fw = fw.min(crop_w - fx);
            fh = fh.min(crop_h - fy);
        }

        // Ensure valid dimensions
        if fw <= 0.0 || fh <= 0.0 {
            return None;
        }

        Some((
            fx.round() as i32,
            fy.round() as i32,
            fw.round().max(1.0) as u32,
            fh.round().max(1.0) as u32,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let transform = CropRotateTransform::default();
        assert!(transform.is_identity());
    }

    #[test]
    fn test_90_rotation_coords() {
        let transform = CropRotateTransform {
            rotation_90_count: 1,
            ..Default::default()
        };
        // Face at (10, 20, 30, 40) in 100x200 image
        // After 90 CW: new_x = 200-20-40=140, new_y = 10, w<->h = (40, 30)
        let result = transform.transform_face_coords((10, 20, 30, 40), 100, 200);
        assert_eq!(result, Some((140, 10, 40, 30)));
    }

    #[test]
    fn test_crop_excludes_face() {
        let transform = CropRotateTransform {
            crop_rect: Some(NormalizedRect {
                x: 0.5,
                y: 0.5,
                width: 0.5,
                height: 0.5,
            }),
            ..Default::default()
        };
        // Face at (10, 10, 30, 30) in 100x100 image - in top-left quadrant
        // Crop is bottom-right quadrant (50-100, 50-100)
        let result = transform.transform_face_coords((10, 10, 30, 30), 100, 100);
        assert_eq!(result, None); // Face is entirely outside crop
    }
}

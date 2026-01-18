/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Stroke/border effect for face areas
//! Draws a colored border around specified regions of an image

use image::DynamicImage;
use image::GenericImage;
use image::GenericImageView;

/// Stroke effect configuration
#[derive(Debug, Clone)]
pub struct StrokeEffect {
    /// Thickness of stroke border in pixels
    pub thickness: u32,
    /// Color of stroke border (R, G, B, A)
    pub color: (u8, u8, u8, u8),
}

impl Default for StrokeEffect {
    fn default() -> Self {
        Self {
            thickness: 4,
            color: (255, 0, 0, 255),
        }
    }
}

impl StrokeEffect {
    /// Create a new stroke effect
    #[allow(dead_code)]
    pub fn new(thickness: u32, color: (u8, u8, u8, u8)) -> Self {
        Self { thickness, color }
    }

    /// Apply stroke effect to detected face areas
    ///
    /// # Arguments
    /// * `image` - Mutable image reference
    /// * `face_areas` - Slice of (x, y, width, height) face rectangles
    /// * `config` - Stroke configuration
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` on error
    #[allow(dead_code)]
    pub fn apply(
        image: &mut DynamicImage,
        face_areas: &[(i32, i32, u32, u32)],
        config: &StrokeEffect,
    ) -> Result<(), String> {
        let (stroke_r, stroke_g, stroke_b, stroke_a) = config.color;
        let stroke_color = image::Rgba([stroke_r, stroke_g, stroke_b, stroke_a]);
        let stroke_width = config.thickness.max(1);

        let (img_width, img_height) = image.dimensions();

        for &(x, y, width, height) in face_areas {
            // Ensure face area is within image bounds
            if x < 0 || y < 0 || width == 0 || height == 0 {
                continue;
            }

            let start_x = x.max(0) as u32;
            let start_y = y.max(0) as u32;
            // Clamp face dimensions to image boundaries
            let available_width = img_width.saturating_sub(start_x);
            let available_height = img_height.saturating_sub(start_y);
            let face_width = width.min(available_width);
            let face_height = height.min(available_height);

            if face_width == 0 || face_height == 0 {
                continue;
            }

            let end_x = start_x + face_width;
            let end_y = start_y + face_height;

            log::debug!(
                "Applying stroke to face at ({}, {}) size {}x{} with thickness {}",
                start_x,
                start_y,
                face_width,
                face_height,
                stroke_width
            );

            // Draw top border (horizontal line at top)
            for thickness_offset in 0..stroke_width {
                let py = start_y + thickness_offset;
                if py >= img_height {
                    break;
                }
                for px in start_x..end_x.min(img_width) {
                    image.put_pixel(px, py, stroke_color);
                }
            }

            // Draw bottom border (horizontal line at bottom)
            for thickness_offset in 0..stroke_width {
                let py = end_y.saturating_sub(1 + thickness_offset);
                if py < start_y {
                    break;
                }
                for px in start_x..end_x.min(img_width) {
                    image.put_pixel(px, py, stroke_color);
                }
            }

            // Draw left border (vertical line at left)
            for thickness_offset in 0..stroke_width {
                let px = start_x + thickness_offset;
                if px >= img_width {
                    break;
                }
                for py in start_y..end_y.min(img_height) {
                    image.put_pixel(px, py, stroke_color);
                }
            }

            // Draw right border (vertical line at right)
            for thickness_offset in 0..stroke_width {
                let px = end_x.saturating_sub(1 + thickness_offset);
                if px < start_x {
                    break;
                }
                for py in start_y..end_y.min(img_height) {
                    image.put_pixel(px, py, stroke_color);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stroke_effect_default() {
        let config = StrokeEffect::default();
        assert_eq!(config.thickness, 4);
        assert_eq!(config.color, (255, 0, 0, 255));
    }

    #[test]
    fn test_stroke_effect_custom() {
        let config = StrokeEffect::new(8, (255, 255, 0, 128));
        assert_eq!(config.thickness, 8);
        assert_eq!(config.color, (255, 255, 0, 128));
    }
}

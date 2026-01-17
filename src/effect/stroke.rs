/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Stroke/border effect for face areas
//! Draws a colored border around specified regions of an image

use image::DynamicImage;
use image::GenericImage;

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
        let stroke_width = config.thickness;

        for &(x, y, width, height) in face_areas {
            // Ensure face area is within image bounds
            if x < 0 || y < 0 || width == 0 || height == 0 {
                continue;
            }

            let start_x = x.max(0) as u32;
            let start_y = y.max(0) as u32;
            let dimm = imageproc::drawing::Canvas::dimensions(image);
            let face_width = width.saturating_sub(dimm.0 - start_x);
            let face_height = height.saturating_sub(dimm.1 - start_y);

            if face_width == 0 || face_height == 0 {
                continue;
            }

            // Draw top border
            for px in 0..face_width {
                let py = start_y;
                image.put_pixel(px, py, stroke_color);
            }

            // Draw bottom border
            for px in 0..face_width {
                let py = start_y + face_height - stroke_width.min(1);
                image.put_pixel(px, py, stroke_color);
            }

            // Draw left border
            for py in 0..face_height {
                let px = start_x;
                image.put_pixel(px, py, stroke_color);
            }

            // Draw right border
            for py in 0..face_height {
                let px = start_x + face_width - stroke_width.min(1);
                image.put_pixel(px, py, stroke_color);
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

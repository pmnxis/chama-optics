/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Stroke/border effect for face areas
//! Draws a colored border around specified regions of an image

use image::DynamicImage;

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

    /// Apply stroke effect to detected face areas using direct buffer access.
    #[allow(dead_code)]
    pub fn apply(
        image: &mut DynamicImage,
        face_areas: &[(i32, i32, u32, u32)],
        config: &StrokeEffect,
    ) -> Result<(), String> {
        let (stroke_r, stroke_g, stroke_b, stroke_a) = config.color;
        let stroke_width = config.thickness.max(1);

        let rgba = image.as_mut_rgba8().ok_or("Image is not RGBA8")?;
        let img_width = rgba.width();
        let img_height = rgba.height();
        let stride = img_width as usize * 4;
        let pixels = rgba.as_mut();

        for &(x, y, width, height) in face_areas {
            if x < 0 || y < 0 || width == 0 || height == 0 {
                continue;
            }

            let start_x = x.max(0) as u32;
            let start_y = y.max(0) as u32;
            let face_width = width.min(img_width.saturating_sub(start_x));
            let face_height = height.min(img_height.saturating_sub(start_y));

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

            // Helper: fill a horizontal span [px_start..px_end) at row py
            let fill_hline = |pixels: &mut [u8], py: u32, px_start: u32, px_end: u32| {
                let px_end = px_end.min(img_width);
                if py >= img_height || px_start >= px_end {
                    return;
                }
                let row_offset = py as usize * stride;
                let start = row_offset + px_start as usize * 4;
                let end = row_offset + px_end as usize * 4;
                for chunk in pixels[start..end].chunks_exact_mut(4) {
                    chunk[0] = stroke_r;
                    chunk[1] = stroke_g;
                    chunk[2] = stroke_b;
                    chunk[3] = stroke_a;
                }
            };

            // Top border
            for t in 0..stroke_width {
                let py = start_y + t;
                if py >= img_height {
                    break;
                }
                fill_hline(pixels, py, start_x, end_x);
            }

            // Bottom border
            for t in 0..stroke_width {
                let py = end_y.saturating_sub(1 + t);
                if py < start_y {
                    break;
                }
                fill_hline(pixels, py, start_x, end_x);
            }

            // Left border (vertical)
            for t in 0..stroke_width {
                let px = start_x + t;
                if px >= img_width {
                    break;
                }
                for py in start_y..end_y.min(img_height) {
                    let idx = py as usize * stride + px as usize * 4;
                    pixels[idx] = stroke_r;
                    pixels[idx + 1] = stroke_g;
                    pixels[idx + 2] = stroke_b;
                    pixels[idx + 3] = stroke_a;
                }
            }

            // Right border (vertical)
            for t in 0..stroke_width {
                let px = end_x.saturating_sub(1 + t);
                if px < start_x {
                    break;
                }
                for py in start_y..end_y.min(img_height) {
                    let idx = py as usize * stride + px as usize * 4;
                    pixels[idx] = stroke_r;
                    pixels[idx + 1] = stroke_g;
                    pixels[idx + 2] = stroke_b;
                    pixels[idx + 3] = stroke_a;
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

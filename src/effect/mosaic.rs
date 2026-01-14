/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Mosaic blur effect for face areas
//! Applies pixelated blur to specified regions of an image

use image::DynamicImage;
use image::GenericImage;
use image::GenericImageView;
use image::Rgba;

/// Mosaic effect configuration
#[derive(Debug, Clone)]
pub struct MosaicEffect {
    /// Block size for mosaic (in pixels)
    pub block_size: u32,
    /// Blend intensity (0.0 = full mosaic, 1.0 = no effect)
    #[allow(dead_code)]
    pub intensity: f32,
}

impl Default for MosaicEffect {
    fn default() -> Self {
        Self {
            block_size: 10,
            intensity: 1.0,
        }
    }
}

impl MosaicEffect {
    /// Create a new mosaic effect
    #[allow(dead_code)]
    pub fn new(block_size: u32, intensity: f32) -> Self {
        Self {
            block_size,
            intensity,
        }
    }

    /// Apply mosaic effect to detected face areas
    ///
    /// # Arguments
    /// * `image` - Mutable image reference
    /// * `face_areas` - Slice of (x, y, width, height) face rectangles
    /// * `config` - Mosaic configuration
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` on error
    pub fn apply(
        image: &mut DynamicImage,
        face_areas: &[(i32, i32, u32, u32)],
        config: &MosaicEffect,
    ) -> Result<(), String> {
        let (img_width, img_height) = image.dimensions();

        for &(x, y, rect_width, rect_height) in face_areas {
            if x < 0 || y < 0 || rect_width == 0 || rect_height == 0 {
                continue;
            }

            let start_x = x.max(0) as u32;
            let start_y = y.max(0) as u32;
            let face_width = rect_width.saturating_sub(img_width - start_x);
            let face_height = rect_height.saturating_sub(img_height - start_y);

            if face_width == 0 || face_height == 0 {
                continue;
            }

            let block_size = config.block_size.max(1);

            for block_y in (start_y..start_y + face_height).step_by(block_size as usize) {
                for block_x in (start_x..start_x + face_width).step_by(block_size as usize) {
                    let block_end_x = (block_x + block_size).min(start_x + face_width);
                    let block_end_y = (block_y + block_size).min(start_y + face_height);
                    let actual_block_width = block_end_x - block_x;
                    let actual_block_height = block_end_y - block_y;

                    if actual_block_width == 0 || actual_block_height == 0 {
                        continue;
                    }

                    let avg_color = calculate_average_color(
                        image,
                        block_x,
                        block_y,
                        actual_block_width,
                        actual_block_height,
                    );
                    draw_solid_rect(
                        image,
                        block_x,
                        block_y,
                        actual_block_width,
                        actual_block_height,
                        avg_color,
                    );
                }
            }
        }

        Ok(())
    }
}

/// Draw a solid rectangle on image
fn draw_solid_rect(
    image: &mut DynamicImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: Rgba<u8>,
) {
    for py in y..y + height {
        for px in x..x + width {
            image.put_pixel(px, py, color);
        }
    }
}

/// Calculate average color of a rectangular area
fn calculate_average_color(
    image: &mut DynamicImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Rgba<u8> {
    let mut r: u64 = 0;
    let mut g: u64 = 0;
    let mut b: u64 = 0;
    let mut count: u64 = 0;

    for py in (y..y + height).step_by(4) {
        for px in (x..x + width).step_by(4) {
            let pixel = image.get_pixel(px, py);
            let [red, green, blue, _alpha] = pixel.0;
            r += red as u64;
            g += green as u64;
            b += blue as u64;
            count += 1;
        }
    }

    if count == 0 {
        return Rgba([0, 0, 0, 255]);
    }

    let avg_r = (r / count) as u8;
    let avg_g = (g / count) as u8;
    let avg_b = (b / count) as u8;

    Rgba([avg_r, avg_g, avg_b, 255])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mosaic_effect_default() {
        let config = MosaicEffect::default();
        assert_eq!(config.block_size, 10);
        assert_eq!(config.intensity, 1.0);
    }

    #[test]
    fn test_mosaic_effect_custom() {
        let config = MosaicEffect::new(15, 0.8);
        assert_eq!(config.block_size, 15);
        assert_eq!(config.intensity, 0.8);
    }
}

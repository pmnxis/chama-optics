/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Mosaic blur effect for face areas
//! Applies pixelated blur to specified regions of an image

use image::DynamicImage;

/// Mosaic effect configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MosaicEffect {
    /// Block size for mosaic (in pixels)
    pub block_size: u32,
    /// Blend intensity (0.0 = full mosaic, 1.0 = no effect)
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

    /// Apply mosaic effect to detected face areas using direct buffer access.
    #[allow(dead_code)]
    pub fn apply(
        image: &mut DynamicImage,
        face_areas: &[(i32, i32, u32, u32)],
        config: &MosaicEffect,
    ) -> Result<(), String> {
        // JPEG and some other formats load as Rgb8 — convert to Rgba8 so as_mut_rgba8() works
        if image.as_rgba8().is_none() {
            *image = DynamicImage::ImageRgba8(image.to_rgba8());
        }
        let rgba = image.as_mut_rgba8().ok_or("Image is not RGBA8")?;
        let img_width = rgba.width();
        let img_height = rgba.height();
        let stride = img_width as usize * 4;
        let block_size = config.block_size.max(1);

        for &(x, y, rect_width, rect_height) in face_areas {
            if x < 0 || y < 0 || rect_width == 0 || rect_height == 0 {
                continue;
            }

            let start_x = x.max(0) as u32;
            let start_y = y.max(0) as u32;
            let face_width = rect_width.min(img_width.saturating_sub(start_x));
            let face_height = rect_height.min(img_height.saturating_sub(start_y));

            if face_width == 0 || face_height == 0 {
                continue;
            }

            log::debug!(
                "Applying mosaic to face at ({}, {}) size {}x{}",
                start_x,
                start_y,
                face_width,
                face_height
            );

            let pixels: &mut [u8] = rgba.as_mut();

            for block_y in (start_y..start_y + face_height).step_by(block_size as usize) {
                for block_x in (start_x..start_x + face_width).step_by(block_size as usize) {
                    let block_end_x = (block_x + block_size).min(start_x + face_width);
                    let block_end_y = (block_y + block_size).min(start_y + face_height);
                    let bw = block_end_x - block_x;
                    let bh = block_end_y - block_y;

                    if bw == 0 || bh == 0 {
                        continue;
                    }

                    // Calculate average color by sampling every 4th pixel
                    let (mut r, mut g, mut b, mut count) = (0u64, 0u64, 0u64, 0u64);
                    for py in (block_y..block_end_y).step_by(4) {
                        let row_start = py as usize * stride + block_x as usize * 4;
                        let mut idx = row_start;
                        for _px in (block_x..block_end_x).step_by(4) {
                            r += pixels[idx] as u64;
                            g += pixels[idx + 1] as u64;
                            b += pixels[idx + 2] as u64;
                            count += 1;
                            idx += 16; // step_by(4) * 4 bytes per pixel
                        }
                    }

                    if count == 0 {
                        continue;
                    }

                    let avg_r = (r / count) as u8;
                    let avg_g = (g / count) as u8;
                    let avg_b = (b / count) as u8;

                    // Fill block with average color
                    for py in block_y..block_end_y {
                        let row_start = py as usize * stride + block_x as usize * 4;
                        let row_end = row_start + bw as usize * 4;
                        let row = &mut pixels[row_start..row_end];
                        for chunk in row.chunks_exact_mut(4) {
                            chunk[0] = avg_r;
                            chunk[1] = avg_g;
                            chunk[2] = avg_b;
                            chunk[3] = 255;
                        }
                    }
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

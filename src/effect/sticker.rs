// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sticker effect module - applies emoji/sticker overlays to detected faces
//!
//! This is a simplified placeholder implementation that draws colored shapes.
//! Full sticker functionality with PNG assets is planned for future implementation.

use image::{DynamicImage, GenericImage, GenericImageView, Rgba, RgbaImage};

/// Configuration for sticker effect
#[derive(Debug, Clone)]
pub struct StickerConfig {
    pub sticker_id: String,
    pub scale: f32,
    pub offset_x: i32,
    pub offset_y: i32,
}

impl Default for StickerConfig {
    fn default() -> Self {
        Self {
            sticker_id: "heart".to_string(),
            scale: 1.0,
            offset_x: 0,
            offset_y: 0,
        }
    }
}

/// Apply sticker effect to image at specified face locations
///
/// # Arguments
/// * `image` - The image to modify
/// * `face_areas` - Vec of (x, y, width, height) tuples for detected faces
/// * `config` - Sticker configuration
///
/// # Returns
/// * Modified image with stickers applied
pub fn apply_sticker(
    mut image: DynamicImage,
    face_areas: Vec<(i32, i32, u32, u32)>,
    config: &StickerConfig,
) -> DynamicImage {
    for (x, y, width, height) in face_areas {
        // Calculate sticker size based on face size and scale
        let sticker_size = ((width as f32 * config.scale) as u32).max(20);

        // Calculate center of face
        let center_x = x + (width as i32 / 2) + config.offset_x;
        let center_y = y + (height as i32 / 2) + config.offset_y;

        // Create sticker based on sticker_id
        let sticker = create_sticker(&config.sticker_id, sticker_size);

        // Overlay sticker at face location
        overlay_sticker(&mut image, &sticker, center_x, center_y);
    }

    image
}

/// Create a sticker image based on sticker ID
///
/// This is a placeholder that creates simple shapes.
/// Future: Load actual PNG sticker assets from storage
fn create_sticker(sticker_id: &str, size: u32) -> RgbaImage {
    let mut sticker = RgbaImage::new(size, size);

    // Create different shapes based on sticker ID
    match sticker_id {
        id if id.contains("heart") || id.contains("emoji_heart") => {
            draw_heart(&mut sticker, size)
        }
        id if id.contains("star") => draw_star(&mut sticker, size),
        id if id.contains("smile") => draw_smile(&mut sticker, size),
        _ => draw_circle(&mut sticker, size, Rgba([255, 200, 0, 220])), // Default: yellow circle
    }

    sticker
}

/// Draw a simple heart shape
fn draw_heart(img: &mut RgbaImage, size: u32) {
    let center = size as i32 / 2;
    let color = Rgba([255, 50, 100, 220]); // Pink/red

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center as f32;
            let dy = y as f32 - center as f32 - (size as f32 * 0.1);

            // Heart shape equation
            let a = dx.powi(2) + dy.powi(2) - (size as f32 * 0.35);
            let b = dx.powi(2) + (dy - (size as f32 * 0.4).abs()).powi(2);

            if a.powi(3) - dx.powi(2) * dy.powi(3) < 0.0 || b < (size as f32 * 0.15).powi(2) {
                img.put_pixel(x, y, color);
            }
        }
    }
}

/// Draw a simple star shape
fn draw_star(img: &mut RgbaImage, size: u32) {
    draw_circle(img, size, Rgba([255, 220, 0, 220])); // Yellow star placeholder
}

/// Draw a simple smile emoji
fn draw_smile(img: &mut RgbaImage, size: u32) {
    draw_circle(img, size, Rgba([255, 220, 50, 220])); // Yellow face placeholder
}

/// Draw a filled circle
fn draw_circle(img: &mut RgbaImage, size: u32, color: Rgba<u8>) {
    let center = size as f32 / 2.0;
    let radius = center * 0.9;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();

            if distance <= radius {
                // Add anti-aliasing at edges
                let alpha = if distance > radius - 2.0 {
                    ((radius - distance) / 2.0 * 255.0) as u8
                } else {
                    color[3]
                };

                img.put_pixel(x, y, Rgba([color[0], color[1], color[2], alpha]));
            }
        }
    }
}

/// Overlay sticker image onto base image with transparency
fn overlay_sticker(base: &mut DynamicImage, sticker: &RgbaImage, center_x: i32, center_y: i32) {
    let sticker_width = sticker.width() as i32;
    let sticker_height = sticker.height() as i32;

    let start_x = center_x - sticker_width / 2;
    let start_y = center_y - sticker_height / 2;

    for sy in 0..sticker_height {
        for sx in 0..sticker_width {
            let target_x = start_x + sx;
            let target_y = start_y + sy;

            // Check bounds
            if target_x >= 0
                && target_y >= 0
                && target_x < base.width() as i32
                && target_y < base.height() as i32
            {
                let sticker_pixel = sticker.get_pixel(sx as u32, sy as u32);

                // Only overlay if sticker pixel has some alpha
                if sticker_pixel[3] > 0 {
                    let base_pixel = base.get_pixel(target_x as u32, target_y as u32);

                    // Alpha blending
                    let alpha = sticker_pixel[3] as f32 / 255.0;
                    let inv_alpha = 1.0 - alpha;

                    let blended = Rgba([
                        (sticker_pixel[0] as f32 * alpha + base_pixel[0] as f32 * inv_alpha)
                            as u8,
                        (sticker_pixel[1] as f32 * alpha + base_pixel[1] as f32 * inv_alpha)
                            as u8,
                        (sticker_pixel[2] as f32 * alpha + base_pixel[2] as f32 * inv_alpha)
                            as u8,
                        255,
                    ]);

                    base.put_pixel(target_x as u32, target_y as u32, blended);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sticker() {
        let sticker = create_sticker("heart", 50);
        assert_eq!(sticker.width(), 50);
        assert_eq!(sticker.height(), 50);
    }

    #[test]
    fn test_apply_sticker() {
        let img = DynamicImage::new_rgb8(100, 100);
        let face_areas = vec![(25, 25, 50, 50)];
        let config = StickerConfig::default();

        let result = apply_sticker(img, face_areas, &config);
        assert_eq!(result.width(), 100);
        assert_eq!(result.height(), 100);
    }
}

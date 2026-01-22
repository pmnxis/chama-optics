// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sticker effect module - applies emoji/sticker overlays to detected faces
//!
//! Supports two modes:
//! - Built-in procedural stickers (heart, star, smile shapes)
//! - Custom image stickers loaded from file paths (PNG, JPG, etc.)

use image::{DynamicImage, Rgba, RgbaImage};
use std::path::PathBuf;

/// Configuration for sticker effect
#[derive(Debug, Clone)]
pub struct StickerConfig {
    /// Sticker ID for built-in stickers (legacy, for backward compatibility)
    #[allow(dead_code)]
    pub sticker_id: String, // todo - check it's really need
    /// Optional image path for custom stickers (takes precedence over sticker_id)
    pub sticker_path: Option<PathBuf>,
    #[allow(dead_code)]
    pub scale: f32, // todo - check it's really need
    #[allow(dead_code)]
    pub offset_x: i32, // todo - check it's really need
    #[allow(dead_code)]
    pub offset_y: i32, // todo - check it's really need
}

impl Default for StickerConfig {
    fn default() -> Self {
        Self {
            sticker_id: "heart".to_string(),
            sticker_path: None,
            scale: 1.0,
            offset_x: 0,
            offset_y: 0,
        }
    }
}

impl StickerConfig {
    /// Create a new config with an image path (for iOS)
    #[cfg(target_os = "ios")]
    pub fn with_image_path(path: PathBuf, scale: f32, offset_x: i32, offset_y: i32) -> Self {
        Self {
            sticker_id: String::new(),
            sticker_path: Some(path),
            scale,
            offset_x,
            offset_y,
        }
    }

    /// Create a new config with a built-in sticker ID
    #[cfg(target_os = "ios")]
    pub fn with_builtin(sticker_id: String, scale: f32, offset_x: i32, offset_y: i32) -> Self {
        Self {
            sticker_id,
            sticker_path: None,
            scale,
            offset_x,
            offset_y,
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
#[cfg(target_os = "ios")]
pub fn apply_sticker(
    mut image: DynamicImage,
    face_areas: Vec<(i32, i32, u32, u32)>,
    config: &StickerConfig,
) -> DynamicImage {
    // Load sticker image once (either from path or create built-in)
    let sticker_source = load_sticker_source(config);

    for (x, y, width, height) in face_areas {
        // Calculate sticker size based on face size and scale
        let target_size = ((width as f32 * config.scale) as u32).max(20);

        // Calculate sticker dimensions (after aspect ratio preservation)
        let sticker_aspect = sticker_source
            .as_ref()
            .map(|img| img.width() as f32 / img.height() as f32)
            .unwrap_or(1.0);
        let (sticker_w, sticker_h) = if sticker_aspect >= 1.0 {
            (target_size as f32, target_size as f32 / sticker_aspect)
        } else {
            (target_size as f32 * sticker_aspect, target_size as f32)
        };

        // Calculate center of face
        let face_center_x = x as f32 + width as f32 / 2.0;
        let face_center_y = y as f32 + height as f32 / 2.0;

        // Apply offset as percentage of sticker size (matching EGUI behavior)
        // offset_x and offset_y are percentages (-100 to 100) of sticker size
        let offset_pixel_x = sticker_w * config.offset_x as f32 / 100.0;
        let offset_pixel_y = sticker_h * config.offset_y as f32 / 100.0;

        let sticker_center_x = face_center_x + offset_pixel_x;
        let sticker_center_y = face_center_y + offset_pixel_y;

        let center_x = sticker_center_x as i32;
        let center_y = sticker_center_y as i32;

        // Get or create sticker at the right size
        let sticker = match &sticker_source {
            Some(source_img) => {
                // Calculate aspect ratio to maintain original proportions
                let orig_width = source_img.width();
                let orig_height = source_img.height();
                let aspect_ratio = orig_width as f32 / orig_height as f32;

                // Calculate dimensions that maintain aspect ratio within target_size
                let (new_width, new_height) = if aspect_ratio >= 1.0 {
                    // Wider than tall: fit to width
                    let w = target_size;
                    let h = (target_size as f32 / aspect_ratio) as u32;
                    (w, h)
                } else {
                    // Taller than wide: fit to height
                    let h = target_size;
                    let w = (target_size as f32 * aspect_ratio) as u32;
                    (w, h)
                };

                // Resize the loaded image maintaining aspect ratio
                let resized =
                    source_img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);
                resized.to_rgba8()
            }
            None => {
                // Fall back to built-in sticker
                // create_sticker(&config.sticker_id, target_size)
                RgbaImage::new(target_size, target_size)
            }
        };

        // Overlay sticker at face location
        overlay_sticker(&mut image, &sticker, center_x, center_y);
    }

    image
}

/// Load sticker source image from path or return None for built-in
#[allow(dead_code)]
fn load_sticker_source(config: &StickerConfig) -> Option<DynamicImage> {
    if let Some(ref path) = config.sticker_path {
        match image::open(path) {
            Ok(img) => {
                log::info!("Loaded sticker from path: {:?}", path);
                Some(img)
            }
            Err(e) => {
                log::warn!(
                    "Failed to load sticker from {:?}: {}, falling back to built-in",
                    path,
                    e
                );
                None
            }
        }
    } else {
        None
    }
}

// /// Create a sticker image based on sticker ID
// ///
// /// This is a placeholder that creates simple shapes.
// /// Future: Load actual PNG sticker assets from storage
// fn create_sticker(sticker_id: &str, size: u32) -> RgbaImage {
//     let mut sticker = RgbaImage::new(size, size);

//     // Create different shapes based on sticker ID
//     match sticker_id {
//         id if id.contains("heart") || id.contains("emoji_heart") => draw_heart(&mut sticker, size),
//         id if id.contains("star") => draw_star(&mut sticker, size),
//         id if id.contains("smile") => draw_smile(&mut sticker, size),
//         _ => draw_circle(&mut sticker, size, Rgba([255, 200, 0, 220])), // Default: yellow circle
//     }

//     sticker
// }

/// Draw a filled circle
#[allow(dead_code)]
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
#[cfg(target_os = "ios")]
fn overlay_sticker(base: &mut DynamicImage, sticker: &RgbaImage, center_x: i32, center_y: i32) {
    use image::{GenericImage, GenericImageView};
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
                        (sticker_pixel[0] as f32 * alpha + base_pixel[0] as f32 * inv_alpha) as u8,
                        (sticker_pixel[1] as f32 * alpha + base_pixel[1] as f32 * inv_alpha) as u8,
                        (sticker_pixel[2] as f32 * alpha + base_pixel[2] as f32 * inv_alpha) as u8,
                        255,
                    ]);

                    base.put_pixel(target_x as u32, target_y as u32, blended);
                }
            }
        }
    }
}

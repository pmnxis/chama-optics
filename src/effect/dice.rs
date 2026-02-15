// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dice placement algorithm for random character sticker placement
//!
//! Places character stickers randomly on an image while avoiding
//! overlap with detected face areas and previously placed stickers.

use crate::effect::cheki::PlacedSticker;
use crate::effect::sticker_storage::{FaceArea, StickerItem};
use rand::{Rng, RngExt};

/// Configuration for the dice placement algorithm
pub struct DicePlacementConfig {
    /// Maximum retry attempts per sticker placement
    pub max_attempts: u32,
    /// Minimum sticker scale relative to image shorter dimension
    pub min_scale: f32,
    /// Maximum sticker scale relative to image shorter dimension
    pub max_scale: f32,
    /// Padding around face areas (pixels) to keep stickers away from faces
    pub face_padding: f32,
    /// Padding between stickers (pixels)
    pub sticker_padding: f32,
    /// Maximum rotation angle in degrees (+/-)
    pub max_rotation: f32,
}

impl Default for DicePlacementConfig {
    fn default() -> Self {
        Self {
            max_attempts: 100,
            min_scale: 0.08,
            max_scale: 0.18,
            face_padding: 20.0,
            sticker_padding: 10.0,
            max_rotation: 15.0,
        }
    }
}

struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

/// Place character stickers randomly on an image, avoiding face areas and existing placements.
///
/// Returns the newly placed stickers (not including existing ones).
#[allow(clippy::too_many_arguments)]
pub fn place_character_stickers(
    image_width: u32,
    image_height: u32,
    face_areas: &[FaceArea],
    existing_placements: &[PlacedSticker],
    character_stickers: &[&StickerItem],
    count: usize,
    config: &DicePlacementConfig,
    rng: &mut impl Rng,
) -> Vec<PlacedSticker> {
    if character_stickers.is_empty() || image_width == 0 || image_height == 0 {
        return Vec::new();
    }

    let img_w = image_width as f32;
    let img_h = image_height as f32;

    // Build exclusion rects from face areas (with padding)
    let face_rects: Vec<Rect> = face_areas
        .iter()
        .map(|f| Rect {
            x: f.x as f32 - config.face_padding,
            y: f.y as f32 - config.face_padding,
            w: f.width as f32 + 2.0 * config.face_padding,
            h: f.height as f32 + 2.0 * config.face_padding,
        })
        .collect();

    // Track all placed sticker rects (existing + new)
    let mut occupied: Vec<Rect> = existing_placements
        .iter()
        .map(|p| {
            let sw = img_w * p.scale;
            let sh = img_h * p.scale;
            Rect {
                x: p.x * img_w - config.sticker_padding,
                y: p.y * img_h - config.sticker_padding,
                w: sw + 2.0 * config.sticker_padding,
                h: sh + 2.0 * config.sticker_padding,
            }
        })
        .collect();

    let mut new_placements = Vec::new();

    for _ in 0..count {
        // Random sticker selection
        let sticker_idx = rng.random_range(0..character_stickers.len());
        let sticker = character_stickers[sticker_idx];

        // Random scale
        let scale = rng.random_range(config.min_scale..=config.max_scale);
        let sticker_w = img_w * scale;
        let sticker_h = img_h * scale;

        // Random rotation
        let rotation = rng.random_range(-config.max_rotation..=config.max_rotation);

        // Try to find non-overlapping position
        let mut placed = false;
        for _ in 0..config.max_attempts {
            let x = rng.random_range(0.0..=(img_w - sticker_w).max(0.0));
            let y = rng.random_range(0.0..=(img_h - sticker_h).max(0.0));

            let candidate = Rect {
                x,
                y,
                w: sticker_w,
                h: sticker_h,
            };

            // Check overlap with face areas
            let overlaps_face = face_rects.iter().any(|f| rects_overlap(&candidate, f));
            if overlaps_face {
                continue;
            }

            // Check overlap with already-placed stickers
            let overlaps_existing = occupied.iter().any(|o| rects_overlap(&candidate, o));
            if overlaps_existing {
                continue;
            }

            // Place the sticker
            let placement = PlacedSticker {
                sticker_id: sticker.id,
                filename: Some(sticker.name.clone()),
                x: x / img_w,
                y: y / img_h,
                scale,
                rotation,
            };

            // Add to occupied list (with padding)
            occupied.push(Rect {
                x: x - config.sticker_padding,
                y: y - config.sticker_padding,
                w: sticker_w + 2.0 * config.sticker_padding,
                h: sticker_h + 2.0 * config.sticker_padding,
            });

            new_placements.push(placement);
            placed = true;
            break;
        }

        if !placed {
            log::warn!(
                "Could not place sticker after {} attempts",
                config.max_attempts
            );
        }
    }

    new_placements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_stickers() {
        let mut rng = rand::rng();
        let result = place_character_stickers(
            100,
            100,
            &[],
            &[],
            &[],
            5,
            &DicePlacementConfig::default(),
            &mut rng,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_basic_placement() {
        let mut rng = rand::rng();
        let sticker = StickerItem::new("test".to_string(), std::path::PathBuf::from("/test.png"));
        let stickers: Vec<&StickerItem> = vec![&sticker];

        let result = place_character_stickers(
            1000,
            1000,
            &[],
            &[],
            &stickers,
            3,
            &DicePlacementConfig::default(),
            &mut rng,
        );
        assert_eq!(result.len(), 3);

        // All placements should be within bounds
        for p in &result {
            assert!(p.x >= 0.0 && p.x <= 1.0);
            assert!(p.y >= 0.0 && p.y <= 1.0);
            assert!(p.scale >= 0.08 && p.scale <= 0.18);
        }
    }
}

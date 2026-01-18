/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Combined mosaic + stroke effect for face areas
//! Applies pixelated blur inside face areas with a colored border around them

use image::DynamicImage;

/// Combined mosaic + stroke effect configuration
#[derive(Debug, Clone)]
pub struct MosaicStrokeEffect {
    /// Mosaic block size in pixels
    pub block_size: u32,
    /// Stroke border thickness in pixels
    pub stroke_thickness: u32,
    /// Stroke border color (R, G, B, A)
    pub stroke_color: (u8, u8, u8, u8),
}

impl Default for MosaicStrokeEffect {
    fn default() -> Self {
        Self {
            block_size: 10,
            stroke_thickness: 4,
            stroke_color: (255, 0, 0, 255),
        }
    }
}

impl MosaicStrokeEffect {
    /// Create a new mosaic+stroke effect
    #[allow(dead_code)]
    pub fn new(block_size: u32, stroke_thickness: u32, stroke_color: (u8, u8, u8, u8)) -> Self {
        Self {
            block_size,
            stroke_thickness,
            stroke_color,
        }
    }

    /// Apply combined mosaic + stroke effect to detected face areas
    ///
    /// # Arguments
    /// * `image` - Mutable image reference
    /// * `face_areas` - Slice of (x, y, width, height) face rectangles
    /// * `config` - MosaicStroke configuration
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(String)` on error
    pub fn apply(
        image: &mut DynamicImage,
        face_areas: &[(i32, i32, u32, u32)],
        config: &MosaicStrokeEffect,
    ) -> Result<(), String> {
        // First apply mosaic inside the face areas
        let mosaic_config = super::mosaic::MosaicEffect {
            block_size: config.block_size,
            intensity: 1.0,
        };

        super::mosaic::MosaicEffect::apply(image, face_areas, &mosaic_config)?;

        // Then apply stroke border around the face areas
        let stroke_config = super::stroke::StrokeEffect {
            thickness: config.stroke_thickness,
            color: config.stroke_color,
        };

        super::stroke::StrokeEffect::apply(image, face_areas, &stroke_config)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mosaic_stroke_effect_default() {
        let config = MosaicStrokeEffect::default();
        assert_eq!(config.block_size, 10);
        assert_eq!(config.stroke_thickness, 4);
        assert_eq!(config.stroke_color, (255, 0, 0, 255));
    }

    #[test]
    fn test_mosaic_stroke_effect_custom() {
        let config = MosaicStrokeEffect::new(15, 6, (255, 255, 0, 128));
        assert_eq!(config.block_size, 15);
        assert_eq!(config.stroke_thickness, 6);
        assert_eq!(config.stroke_color, (255, 255, 0, 128));
    }
}

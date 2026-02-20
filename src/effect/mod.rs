/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

pub mod border;
pub mod cheki;
pub mod cheki_renderer;
pub mod color_adjustments;
pub mod crop_rotate;
pub(crate) mod custom_weighted_sum;
pub mod dice;
pub(crate) mod draw_with_transparency;
pub mod face_detection;
pub mod face_detectors;
pub(crate) mod glow;
pub mod lut_storage;
pub(crate) mod mosaic;
pub(crate) mod sticker;
pub mod sticker_storage;
pub(crate) mod stroke;
pub(crate) mod variable_text;
pub(crate) mod watermark;

#[cfg(feature = "face_detection_insightface")]
pub mod insightface_detector;

#[cfg(feature = "face_detection_candle")]
pub mod candle_face_detector;

#[cfg(all(target_arch = "wasm32", feature = "face_detection_candle"))]
pub mod ort_web_detector;

// Re-export FaceEffectMode for easier access
pub use face_detection::FaceEffectMode;

/// Apply a mutation to a DynamicImage's pixel buffer, handling RGBA8/RGB8 fast paths
/// and converting other formats to RGBA8.
///
/// This avoids duplicating the `match image { RGBA8 => ..., RGB8 => ..., _ => to_rgba8 }`
/// pattern across LUT application, pipeline execution, and color adjustments.
pub fn with_image_buffer_mut(
    image: &mut image::DynamicImage,
    apply_rgba: impl FnOnce(&mut image::RgbaImage),
    apply_rgb: impl FnOnce(&mut image::RgbImage),
) {
    match image {
        image::DynamicImage::ImageRgba8(img) => apply_rgba(img),
        image::DynamicImage::ImageRgb8(img) => apply_rgb(img),
        _ => {
            let mut rgba = image.to_rgba8();
            apply_rgba(&mut rgba);
            *image = image::DynamicImage::ImageRgba8(rgba);
        }
    }
}

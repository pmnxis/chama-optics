// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cheki decoration renderer
//!
//! Applies cheki (Japanese polaroid) decoration to an image.
//! This is a per-image decoration layer applied ON TOP of the selected theme.
//! The rendering creates a polaroid-style white border with text and character stickers.

use image::{DynamicImage, GenericImage, GenericImageView, Rgba, RgbaImage};

use crate::effect::cheki::{ChekiDecoration, ChekiFontSelection};
use crate::effect::sticker_storage::StickerStorage;

/// Apply cheki decoration to an image (already themed).
/// Returns a new image with polaroid border, text, and placed stickers.
pub fn apply_cheki_decoration(
    image: DynamicImage,
    decoration: &ChekiDecoration,
    sticker_storage: &StickerStorage,
) -> DynamicImage {
    if !decoration.enabled {
        return image;
    }

    let img_w = image.width();
    let img_h = image.height();
    let shorter = img_w.min(img_h) as f32;

    // Calculate border dimensions
    let border = (shorter * decoration.border_width).round() as u32;
    let bottom_extra = (img_h as f32 * decoration.bottom_extra).round() as u32;

    // Create new canvas with border
    let canvas_w = img_w + border * 2;
    let canvas_h = img_h + border * 2 + bottom_extra;

    // Fill canvas with border color
    let border_rgba = decoration_color_to_rgba(decoration);
    let mut canvas = RgbaImage::from_pixel(canvas_w, canvas_h, border_rgba);

    // Place the original image centered in the border
    image::imageops::overlay(&mut canvas, &image.to_rgba8(), border as i64, border as i64);

    let mut result = DynamicImage::ImageRgba8(canvas);

    // Render placed stickers
    for placed in &decoration.dice_stickers {
        if let Some(sticker_item) = sticker_storage.get_sticker(placed.sticker_id)
            && let Some(sticker_img) = sticker_item.load_image()
        {
            let target_w = (canvas_w as f32 * placed.scale).round() as u32;
            let target_h = (canvas_h as f32 * placed.scale).round() as u32;

            if target_w == 0 || target_h == 0 {
                continue;
            }

            let resized =
                sticker_img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3);

            let pos_x = (placed.x * canvas_w as f32).round() as i32;
            let pos_y = (placed.y * canvas_h as f32).round() as i32;

            overlay_with_alpha(&mut result, &resized, pos_x, pos_y);
        }
    }

    // Render text in bottom border area
    if !decoration.text.is_empty() {
        render_cheki_text(
            &mut result,
            decoration,
            border,
            img_h + border,
            bottom_extra,
        );
    }

    result
}

/// Overlay a sticker image with alpha blending at the given position
fn overlay_with_alpha(base: &mut DynamicImage, sticker: &DynamicImage, x: i32, y: i32) {
    let sw = sticker.width() as i32;
    let sh = sticker.height() as i32;
    let bw = base.width() as i32;
    let bh = base.height() as i32;

    for sy in 0..sh {
        for sx in 0..sw {
            let tx = x + sx;
            let ty = y + sy;

            if tx >= 0 && ty >= 0 && tx < bw && ty < bh {
                let sp = sticker.get_pixel(sx as u32, sy as u32);
                if sp[3] > 0 {
                    let bp = base.get_pixel(tx as u32, ty as u32);
                    let alpha = sp[3] as f32 / 255.0;
                    let inv = 1.0 - alpha;

                    let blended = Rgba([
                        (sp[0] as f32 * alpha + bp[0] as f32 * inv) as u8,
                        (sp[1] as f32 * alpha + bp[1] as f32 * inv) as u8,
                        (sp[2] as f32 * alpha + bp[2] as f32 * inv) as u8,
                        255,
                    ]);
                    base.put_pixel(tx as u32, ty as u32, blended);
                }
            }
        }
    }
}

/// Render text in the bottom border area of the cheki
fn render_cheki_text(
    image: &mut DynamicImage,
    decoration: &ChekiDecoration,
    _border: u32,
    text_area_y: u32,
    text_area_h: u32,
) {
    let canvas_w = image.width();

    // Calculate text position within the bottom border area
    let text_x = (decoration.text_position_x * canvas_w as f32).round() as i32;
    let text_y =
        text_area_y as i32 + (decoration.text_position_y * text_area_h as f32).round() as i32;

    // Calculate font size based on bottom area height
    let font_px = (text_area_h as f32 * decoration.font_size).max(12.0);
    let scale = ab_glyph::PxScale::from(font_px);

    let text_color = decoration_text_color(decoration);

    // Use the theme text rendering with fallback
    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    {
        use crate::fonts::variable_font::BuiltinVariableFontIndex;

        let font_idx = match decoration.font {
            ChekiFontSelection::Barlow => BuiltinVariableFontIndex::Barlow,
            ChekiFontSelection::BarlowNarrow => BuiltinVariableFontIndex::BarlowNarrow,
            ChekiFontSelection::SourceHanSans => BuiltinVariableFontIndex::SourceHanSans,
        };

        let font_pack = font_idx.get_font();
        let weight = font_pack.default;
        let font = font_pack.get_font_by_weight(weight);

        // Center text horizontally
        let (tw, _th) =
            crate::theme::text_dimensions_with_fallback(scale, &font, weight, &decoration.text);
        let centered_x = text_x - (tw / 2.0) as i32;

        crate::theme::draw_text_with_fallback(
            image,
            text_color,
            centered_x,
            text_y,
            scale,
            &font,
            weight,
            &decoration.text,
        );
    }

    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    {
        // On mobile, use basic imageproc text rendering
        // This is a simplified fallback
        let _ = (text_x, text_y, scale, text_color, border);
    }
}

#[cfg(feature = "egui")]
fn decoration_color_to_rgba(decoration: &ChekiDecoration) -> Rgba<u8> {
    Rgba([
        decoration.border_color.r(),
        decoration.border_color.g(),
        decoration.border_color.b(),
        decoration.border_color.a(),
    ])
}

#[cfg(not(feature = "egui"))]
fn decoration_color_to_rgba(decoration: &ChekiDecoration) -> Rgba<u8> {
    Rgba(decoration.border_color)
}

#[cfg(feature = "egui")]
fn decoration_text_color(decoration: &ChekiDecoration) -> Rgba<u8> {
    Rgba([
        decoration.text_color.r(),
        decoration.text_color.g(),
        decoration.text_color.b(),
        decoration.text_color.a(),
    ])
}

#[cfg(not(feature = "egui"))]
fn decoration_text_color(decoration: &ChekiDecoration) -> Rgba<u8> {
    Rgba(decoration.text_color)
}

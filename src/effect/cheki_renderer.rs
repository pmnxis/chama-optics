// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cheki decoration renderer
//!
//! Applies cheki (Japanese polaroid) decoration to an image.
//! This is a per-image decoration layer applied ON TOP of the selected theme.
//! The rendering creates a polaroid-style white border with text and character stickers.

use image::{DynamicImage, Rgba, RgbaImage};

use crate::effect::cheki::ChekiDecoration;
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
    // When clip_stickers is enabled, only blend pixels within the image area (not border)
    let clip_rect = if decoration.clip_stickers {
        Some((
            border as i32,
            border as i32,
            (border + img_w) as i32,
            (border + img_h) as i32,
        ))
    } else {
        None
    };

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

            overlay_with_alpha_clipped(&mut result, &resized, pos_x, pos_y, clip_rect);
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

    // Render date stamp
    if decoration.date_enabled && !decoration.date_text.is_empty() {
        render_cheki_date_stamp(&mut result, decoration, border, img_h, bottom_extra);
    }

    result
}

/// Overlay a sticker image with alpha blending, optionally clipped to a rect.
/// `clip` is `Some((left, top, right, bottom))` in canvas pixel coordinates.
/// Uses direct buffer access for performance.
fn overlay_with_alpha_clipped(
    base: &mut DynamicImage,
    sticker: &DynamicImage,
    x: i32,
    y: i32,
    clip: Option<(i32, i32, i32, i32)>,
) {
    let sticker_rgba = sticker.to_rgba8();
    let sw = sticker_rgba.width() as i32;
    let sh = sticker_rgba.height() as i32;
    let sticker_stride = sw as usize * 4;
    let sticker_pixels = sticker_rgba.as_raw();

    let base_rgba = match base.as_mut_rgba8() {
        Some(b) => b,
        None => return,
    };
    let bw = base_rgba.width() as i32;
    let bh = base_rgba.height() as i32;
    let base_stride = bw as usize * 4;
    let base_pixels: &mut [u8] = base_rgba.as_mut();

    // Calculate visible row range
    let sy_start = (-y).max(0);
    let sy_end = sh.min(bh - y);
    let sx_start = (-x).max(0);
    let sx_end = sw.min(bw - x);

    if sy_start >= sy_end || sx_start >= sx_end {
        return;
    }

    for sy in sy_start..sy_end {
        let ty = (y + sy) as usize;

        // Apply vertical clip
        if let Some((_, ct, _, cb)) = clip
            && ((ty as i32) < ct || (ty as i32) >= cb)
        {
            continue;
        }

        let sticker_row = sy as usize * sticker_stride;
        let base_row = ty * base_stride;

        for sx in sx_start..sx_end {
            let tx = (x + sx) as usize;

            // Apply horizontal clip
            if let Some((cl, _, cr, _)) = clip
                && ((tx as i32) < cl || (tx as i32) >= cr)
            {
                continue;
            }

            let si = sticker_row + sx as usize * 4;
            let sa = sticker_pixels[si + 3];

            if sa == 0 {
                continue;
            }

            let bi = base_row + tx * 4;

            if sa == 255 {
                // Fully opaque — direct copy
                base_pixels[bi] = sticker_pixels[si];
                base_pixels[bi + 1] = sticker_pixels[si + 1];
                base_pixels[bi + 2] = sticker_pixels[si + 2];
                base_pixels[bi + 3] = 255;
            } else {
                // Alpha blend using integer arithmetic (avoid f32 per pixel)
                let alpha = sa as u16;
                let inv = 255 - alpha;
                base_pixels[bi] = ((sticker_pixels[si] as u16 * alpha
                    + base_pixels[bi] as u16 * inv)
                    / 255) as u8;
                base_pixels[bi + 1] = ((sticker_pixels[si + 1] as u16 * alpha
                    + base_pixels[bi + 1] as u16 * inv)
                    / 255) as u8;
                base_pixels[bi + 2] = ((sticker_pixels[si + 2] as u16 * alpha
                    + base_pixels[bi + 2] as u16 * inv)
                    / 255) as u8;
                base_pixels[bi + 3] = 255;
            }
        }
    }
}

/// Render text in the bottom border area of the cheki
fn render_cheki_text(
    image: &mut DynamicImage,
    decoration: &ChekiDecoration,
    border: u32,
    text_area_y: u32,
    text_area_h: u32,
) {
    let canvas_w = image.width();

    // Calculate text position within the bottom border area
    let text_x = (decoration.text_position_x * canvas_w as f32).round() as i32;
    // text_position_y centers the text vertically (0.5 = center of text area)
    let text_y_center =
        text_area_y as i32 + (decoration.text_position_y * text_area_h as f32).round() as i32;

    // Calculate font size based on bottom area height
    let font_px = (text_area_h as f32 * decoration.font_size).max(12.0);
    let scale = ab_glyph::PxScale::from(font_px);

    let text_color = decoration_text_color(decoration);

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    {
        use crate::effect::variable_text::VariableOrNot;

        let font = match &decoration.font {
            VariableOrNot::Variable(idx) => idx.get_font_by_weight(decoration.font_weight),
            VariableOrNot::Others(fs) => match crate::fonts::FONTS_UNIFY.search(fs) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to load cheki text font: {:?}", e);
                    return;
                }
            },
        };
        let weight = decoration.font_weight;

        // Center text horizontally, clamped to canvas padding
        let (tw, th) =
            crate::theme::text_dimensions_with_fallback(scale, &font, weight, &decoration.text);
        let text_pad = border as i32;
        let centered_x = (text_x - (tw / 2.0) as i32)
            .max(text_pad)
            .min(canvas_w as i32 - text_pad - tw as i32);
        let text_y = text_y_center - (th / 2.0) as i32;

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
        let font = match load_font_from_file(&decoration.font_file, Some(decoration.font_weight)) {
            Some(f) => f,
            None => {
                log::error!("Failed to load cheki text font: {}", decoration.font_file);
                return;
            }
        };
        let weight = decoration.font_weight;

        // Center text horizontally, clamped to canvas padding
        let (tw, th) =
            crate::theme::text_dimensions_with_fallback(scale, &font, weight, &decoration.text);
        let text_pad = border as i32;
        let centered_x = (text_x - (tw / 2.0) as i32)
            .max(text_pad)
            .min(canvas_w as i32 - text_pad - tw as i32);
        let text_y = text_y_center - (th / 2.0) as i32;

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
}

/// Render date stamp on the cheki canvas
fn render_cheki_date_stamp(
    image: &mut DynamicImage,
    decoration: &ChekiDecoration,
    border: u32,
    img_h: u32,
    bottom_extra: u32,
) {
    use crate::effect::cheki::DatePosition;

    let canvas_w = image.width();
    // Use border width as horizontal padding to align date text with image edges
    let pad = border as i32;

    // Determine font size and position based on DatePosition
    let (date_x, date_y, font_area_h) = match decoration.date_position {
        DatePosition::TopLeft => (pad, (border as f32 / 2.0).round() as i32, border as f32),
        DatePosition::TopCenter => (
            (canvas_w / 2) as i32,
            (border as f32 / 2.0).round() as i32,
            border as f32,
        ),
        DatePosition::TopRight => (
            canvas_w as i32 - pad,
            (border as f32 / 2.0).round() as i32,
            border as f32,
        ),
        DatePosition::BottomLeft => (
            pad,
            (img_h + 2 * border) as i32 + (bottom_extra as f32 / 2.0).round() as i32,
            bottom_extra as f32,
        ),
        DatePosition::BottomCenter => (
            (canvas_w / 2) as i32,
            (img_h + 2 * border) as i32 + (bottom_extra as f32 / 2.0).round() as i32,
            bottom_extra as f32,
        ),
        DatePosition::BottomRight => (
            canvas_w as i32 - pad,
            (img_h + 2 * border) as i32 + (bottom_extra as f32 / 2.0).round() as i32,
            bottom_extra as f32,
        ),
    };

    let font_px = (font_area_h * decoration.date_font_size).max(12.0);
    let scale = ab_glyph::PxScale::from(font_px);
    let date_color = decoration_date_color(decoration);

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    {
        use crate::effect::variable_text::VariableOrNot;

        let font = match &decoration.date_font {
            VariableOrNot::Variable(idx) => idx.get_font_by_weight(decoration.date_font_weight),
            VariableOrNot::Others(fs) => match crate::fonts::FONTS_UNIFY.search(fs) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to load cheki date font: {:?}", e);
                    return;
                }
            },
        };
        let weight = decoration.date_font_weight;

        // Calculate text dimensions for alignment
        let (tw, _th) = crate::theme::text_dimensions_with_fallback(
            scale,
            &font,
            weight,
            &decoration.date_text,
        );

        // Adjust x position based on alignment (left/center/right)
        let aligned_x = match decoration.date_position {
            DatePosition::TopLeft | DatePosition::BottomLeft => date_x,
            DatePosition::TopCenter | DatePosition::BottomCenter => date_x - (tw / 2.0) as i32,
            DatePosition::TopRight | DatePosition::BottomRight => date_x - tw as i32,
        };

        // Clamp to stay within canvas padding
        let aligned_x = aligned_x.max(pad).min(canvas_w as i32 - pad - tw as i32);

        crate::theme::draw_text_with_fallback(
            image,
            date_color,
            aligned_x,
            date_y,
            scale,
            &font,
            weight,
            &decoration.date_text,
        );
    }

    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    {
        let font = match load_font_from_file(
            &decoration.date_font_file,
            Some(decoration.date_font_weight),
        ) {
            Some(f) => f,
            None => {
                log::error!(
                    "Failed to load cheki date font: {}",
                    decoration.date_font_file
                );
                return;
            }
        };
        let weight = decoration.date_font_weight;

        // Calculate text dimensions for alignment
        let (tw, _th) = crate::theme::text_dimensions_with_fallback(
            scale,
            &font,
            weight,
            &decoration.date_text,
        );

        // Adjust x position based on alignment (left/center/right)
        let aligned_x = match decoration.date_position {
            DatePosition::TopLeft | DatePosition::BottomLeft => date_x,
            DatePosition::TopCenter | DatePosition::BottomCenter => date_x - (tw / 2.0) as i32,
            DatePosition::TopRight | DatePosition::BottomRight => date_x - tw as i32,
        };

        // Clamp to stay within canvas padding
        let aligned_x = aligned_x.max(pad).min(canvas_w as i32 - pad - tw as i32);

        crate::theme::draw_text_with_fallback(
            image,
            date_color,
            aligned_x,
            date_y,
            scale,
            &font,
            weight,
            &decoration.date_text,
        );
    }
}

/// Load a font from a file path (with fonts base directory resolution)
/// For variable fonts (e.g. DynaPuff-Variable.ttf), applies the `wght` variation axis.
#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
fn load_font_from_file(font_file: &str, weight: Option<u16>) -> Option<ab_glyph::FontArc> {
    use ab_glyph::VariableFont;
    use std::path::{Path, PathBuf};

    if font_file.is_empty() {
        return None;
    }

    let base_dir = crate::effect::variable_text::get_fonts_base_directory();
    let full_path = if base_dir.is_empty() {
        PathBuf::from(font_file)
    } else {
        let font_path = Path::new(font_file);
        if font_path.is_absolute() {
            PathBuf::from(font_file)
        } else {
            PathBuf::from(&base_dir).join(font_file)
        }
    };

    match std::fs::read(&full_path) {
        Ok(data) => match ab_glyph::FontVec::try_from_vec(data) {
            Ok(mut font) => {
                // Apply weight variation axis for variable fonts (e.g. DynaPuff wght 400-700)
                if let Some(w) = weight {
                    font.set_variation(b"wght", w as f32);
                }
                Some(font.into())
            }
            Err(e) => {
                log::error!("Failed to parse font {}: {}", full_path.display(), e);
                None
            }
        },
        Err(e) => {
            log::error!("Failed to read font file {}: {}", full_path.display(), e);
            None
        }
    }
}

#[cfg(feature = "egui")]
fn decoration_date_color(decoration: &ChekiDecoration) -> Rgba<u8> {
    Rgba([
        decoration.date_color.r(),
        decoration.date_color.g(),
        decoration.date_color.b(),
        decoration.date_color.a(),
    ])
}

#[cfg(not(feature = "egui"))]
fn decoration_date_color(decoration: &ChekiDecoration) -> Rgba<u8> {
    Rgba(decoration.date_color)
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

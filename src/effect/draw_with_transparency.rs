/*
 * SPDX-FileCopyrightText: © 2025 PistonDevelopers (https://github.com/image-rs) and Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! This code is copied and modified from https://github.com/image-rs/imageproc/blob/main/src/drawing/text.rs
//! Existing code had limitation of not draw transparency when draw text

#[allow(unused_imports)] // there's possibility use again in iOS
use ab_glyph::{ScaleFont, point};

use crate::effect::custom_weighted_sum::*;

#[cfg(not(feature = "ios_integration"))]
fn layout_glyphs(
    scale: impl Into<ab_glyph::PxScale> + Copy,
    font: &impl ab_glyph::Font,
    text: &str,
    mut f: impl FnMut(ab_glyph::OutlinedGlyph, ab_glyph::Rect),
) -> (u32, u32) {
    let (mut w, mut h) = (0f32, 0f32);

    let font = font.as_scaled(scale);
    let mut last: Option<ab_glyph::GlyphId> = None;

    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        let glyph = glyph_id.with_scale_and_position(scale, point(w, font.ascent()));
        w += font.h_advance(glyph_id);
        if let Some(g) = font.outline_glyph(glyph) {
            if let Some(last) = last {
                w += font.kern(glyph_id, last);
            }
            last = Some(glyph_id);
            let bb = g.px_bounds();
            h = h.max(bb.height());
            f(g, bb);
        }
    }

    (w as u32, h as u32)
}

#[allow(clippy::too_many_arguments)]
#[cfg(not(feature = "ios_integration"))]
pub fn draw_text_transparency_mut<C>(
    canvas: &mut C,
    color: C::Pixel,
    x: i32,
    y: i32,
    scale: impl Into<ab_glyph::PxScale> + Copy,
    font: &impl ab_glyph::Font,
    font_transparency: u8,
    text: &str,
) where
    C: imageproc::drawing::Canvas,
    <C::Pixel as image::Pixel>::Subpixel: Into<u8> + From<u8> + imageproc::definitions::Clamp<f32>,
{
    let image_width = canvas.width() as i32;
    let image_height = canvas.height() as i32;

    layout_glyphs(scale, font, text, |g, bb| {
        g.draw(|gx, gy, gv| {
            let image_x = gx as i32 + x + bb.min.x.round() as i32;
            let image_y = gy as i32 + y + bb.min.y.round() as i32;
            let gv = gv.clamp(0.0, 1.0);
            let gv_bin = ((gv * font_transparency as f32).round().clamp(0.0, 255.0)) as u8;

            if (0..image_width).contains(&image_x) && (0..image_height).contains(&image_y) {
                let image_x = image_x as u32;
                let image_y = image_y as u32;
                let pixel = canvas.get_pixel(image_x, image_y);
                // let weighted_color = weighted_sum(pixel, color, 1.0 - gv, gv);
                let weighted_color = weighted_sum(pixel, color, gv_bin as u32);
                canvas.draw_pixel(image_x, image_y, weighted_color);
            }
        })
    });
}

#[allow(clippy::too_many_arguments)]
#[cfg(not(feature = "ios_integration"))] // iOS version not use watermark for now
pub fn draw_text_screen_transparency_mut<C>(
    canvas: &mut C,
    color: C::Pixel,
    x: i32,
    y: i32,
    scale: impl Into<ab_glyph::PxScale> + Copy,
    font: &impl ab_glyph::Font,
    font_transparency: u8,
    text: &str,
) where
    C: imageproc::drawing::Canvas,
    <C::Pixel as image::Pixel>::Subpixel: Into<u8> + From<u8> + imageproc::definitions::Clamp<f32>,
{
    let image_width = canvas.width() as i32;
    let image_height = canvas.height() as i32;

    layout_glyphs(scale, font, text, |g, bb| {
        g.draw(|gx, gy, gv| {
            let image_x = gx as i32 + x + bb.min.x.round() as i32;
            let image_y = gy as i32 + y + bb.min.y.round() as i32;
            let gv = gv.clamp(0.0, 1.0);
            let gv_bin = ((gv * font_transparency as f32).round().clamp(0.0, 255.0)) as u8;

            if (0..image_width).contains(&image_x) && (0..image_height).contains(&image_y) {
                let image_x = image_x as u32;
                let image_y = image_y as u32;
                let pixel = canvas.get_pixel(image_x, image_y);
                // let weighted_color = weighted_sum(pixel, color, 1.0 - gv, gv);
                let weighted_color = sh_weighted_sum(pixel, color, gv_bin as u32);
                canvas.draw_pixel(image_x, image_y, weighted_color);
            }
        })
    });
}

pub fn overlay_alpha_screen_mode(
    base: &mut image::DynamicImage,
    overlay: &image::DynamicImage,
    x_offset: u32,
    y_offset: u32,
) {
    use imageproc::drawing::Canvas;
    let (w, h) = overlay.dimensions();

    for oy in 0..h {
        for ox in 0..w {
            let bx = x_offset + ox;
            let by = y_offset + oy;

            if bx >= base.width() || by >= base.height() {
                continue;
            }

            let bg = base.get_pixel(bx, by);
            let fg = overlay.get_pixel(ox, oy);

            let alpha_top = fg[3] as u32;
            let blended = sh_weighted_sum(bg, fg, alpha_top);

            base.draw_pixel(bx, by, blended);
        }
    }
}

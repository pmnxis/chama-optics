/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Headless theme rendering for iOS and other platforms
//! This module provides theme rendering without GUI dependencies

use crate::core::ThemeType;
use crate::exif_impl::SimplifiedExif;
use ab_glyph::{Font, ScaleFont};
use image::{DynamicImage, GenericImage, Rgba};

// Built-in fonts for theme rendering
lazy_static::lazy_static! {
    static ref DIGITAL_7_FONT: ab_glyph::FontArc = ab_glyph::FontArc::try_from_slice(
        include_bytes!(env!("DIGITAL_7_FONT_PATH"))
    ).expect("Failed to load Digital 7 font");

    static ref SOURCE_HAN_SANS: ab_glyph::FontArc = ab_glyph::FontArc::try_from_slice(
        include_bytes!("../../assets/fonts/SourceHanSansVF-remapped.otf")
    ).expect("Failed to load SourceHanSans font");
}

const FILM_COLOR: Rgba<u8> = Rgba([255, 153, 0, 255]);
#[allow(unused)]
const DEFAULT_FILM_FONT_SIZE: u32 = 25;
const DEFAULT_LIGHTROOM_FONT_HEIGHT: u32 = 60;

/// Calculate text dimensions with fallback to SourceHanSans for unsupported characters
fn text_dimensions_with_fallback(
    scale: ab_glyph::PxScale,
    primary_font: &ab_glyph::FontArc,
    text: &str,
) -> (f32, f32) {
    let scaled_primary = primary_font.as_scaled(scale);
    let max_height = scaled_primary.height();

    let total_width: f32 = text
        .chars()
        .map(|ch| {
            let glyph_id = primary_font.glyph_id(ch);
            if glyph_id != ab_glyph::GlyphId(0) {
                scaled_primary.h_advance(glyph_id)
            } else {
                // Fallback to SourceHanSans
                let scaled_fallback = SOURCE_HAN_SANS.as_scaled(scale);
                scaled_fallback.h_advance(SOURCE_HAN_SANS.glyph_id(ch))
            }
        })
        .sum();

    (total_width, max_height)
}

/// Draw text with automatic fallback to SourceHanSans for unsupported characters
fn draw_text_with_fallback<I>(
    image: &mut I,
    color: Rgba<u8>,
    x: i32,
    y: i32,
    scale: ab_glyph::PxScale,
    primary_font: &ab_glyph::FontArc,
    text: &str,
) where
    I: GenericImage<Pixel = Rgba<u8>>,
{
    let mut current_x = x as f32;
    let scaled_primary = primary_font.as_scaled(scale);

    for ch in text.chars() {
        let glyph_id = primary_font.glyph_id(ch);
        let (font_to_use, scaled_font): (&ab_glyph::FontArc, _) =
            if glyph_id != ab_glyph::GlyphId(0) {
                (primary_font, scaled_primary)
            } else {
                // Fallback to SourceHanSans
                (&*SOURCE_HAN_SANS, SOURCE_HAN_SANS.as_scaled(scale))
            };

        // Draw single character
        imageproc::drawing::draw_text_mut(
            image,
            color,
            current_x as i32,
            y,
            scale,
            font_to_use,
            &ch.to_string(),
        );

        // Advance x position
        current_x += scaled_font.h_advance(font_to_use.glyph_id(ch));
    }
}

/// Render Film theme
pub fn render_film(
    image: DynamicImage,
    exif: &SimplifiedExif,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let (dyn_w, dyn_h) = (image.width(), image.height());
    let dyn_wh = dyn_w.max(dyn_h);
    let font = &*DIGITAL_7_FONT;
    let color = FILM_COLOR;

    // Convert to RGBA8 for drawing
    let mut rgba_image = image.to_rgba8();

    // Helper function for relative sizing
    let rel_size = |size: f32| -> f32 { size * (dyn_wh as f32 / 4000.0) };

    let rel_scale = |size: f32| -> ab_glyph::PxScale { ab_glyph::PxScale::from(rel_size(size)) };

    let margin = rel_size(120.0).trunc() as i32;
    let base_y = dyn_h as i32 - margin;

    // Left side text (camera info)
    let mut y = base_y as f32;
    let cam_scale = rel_scale(75.0);
    let left_list = {
        let mut list = Vec::new();

        log::info!(
            "EXIF - Camera: {} {}, Lens: {}",
            exif.camera_mnf,
            exif.camera_model,
            exif.lens_model
        );

        if !(exif.camera_mnf.is_empty() || exif.camera_model.is_empty()) {
            list.push(format!("{}  {}", exif.camera_mnf, exif.camera_model));
        }
        if !exif.lens_model.is_empty() {
            list.push(exif.lens_model.clone());
        }
        list
    };

    log::info!("Left text items: {} items", left_list.len());

    for left_str in left_list.iter().rev() {
        let draw_y = (y - font.as_scaled(cam_scale).ascent()) as i32;
        draw_text_with_fallback(
            &mut rgba_image,
            color,
            margin,
            draw_y,
            cam_scale,
            font,
            left_str,
        );
        y -= cam_scale.y;
    }

    // Right side text (exposure settings)
    let pairs: Vec<(&str, String)> = {
        let mut list = Vec::new();

        if let Some(f) = exif.get_fnumber() {
            log::info!("EXIF - F-number: {}", f);
            list.push(("F", f));
        }
        if let Some(sec) = exif.get_exposure() {
            log::info!("EXIF - Exposure: {}", sec);
            list.push(("SEC", sec));
        }
        if let Some(iso) = exif.get_iso() {
            log::info!("EXIF - ISO: {}", iso);
            list.push(("ISO", iso));
        }
        list
    };

    log::info!("Right text pairs: {} items", pairs.len());

    let prefix_scale = rel_scale(65.0);
    let number_scale = rel_scale(100.0);
    let spacing = rel_size(8.0);
    let mut y: f32 = base_y as f32;

    for (prefix, number) in pairs.iter().rev() {
        let (prefix_w, prefix_h) = text_dimensions_with_fallback(prefix_scale, font, prefix);
        let (number_w, number_h) =
            text_dimensions_with_fallback(number_scale, font, number.as_str());
        let line_h = number_h.max(prefix_h);
        let total_w = prefix_w + spacing + number_w;

        // Right alignment
        let x_right = dyn_w as f32 - margin as f32;
        let x_prefix = (x_right - total_w).round() as i32;
        let x_number = (x_right - number_w).round() as i32;

        let draw_y = y as i32;
        draw_text_with_fallback(
            &mut rgba_image,
            color,
            x_prefix,
            draw_y,
            prefix_scale,
            font,
            prefix,
        );
        draw_text_with_fallback(
            &mut rgba_image,
            color,
            x_number,
            draw_y,
            number_scale,
            font,
            number.as_str(),
        );

        y -= line_h;
    }

    Ok(DynamicImage::ImageRgba8(rgba_image))
}

/// Render Lightroom theme (simplified)
pub fn render_lightroom(
    image: DynamicImage,
    exif: &SimplifiedExif,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let (dyn_w, dyn_h) = (image.width(), image.height());
    let dyn_wh = dyn_w.max(dyn_h);
    let font = &*DIGITAL_7_FONT;
    let color = FILM_COLOR;

    log::info!("LIGHTROOM: Starting render for {}x{} image", dyn_w, dyn_h);

    // Add bottom border
    let border_size = (dyn_wh / 10).clamp(50, 150);
    log::info!("LIGHTROOM: Border size = {}", border_size);

    let mut new_image = image::RgbaImage::new(dyn_w, dyn_h + border_size);
    log::info!(
        "LIGHTROOM: New image size = {}x{}",
        dyn_w,
        dyn_h + border_size
    );

    // Fill border with black
    for px in 0..dyn_w {
        for py in dyn_h..(dyn_h + border_size) {
            new_image.put_pixel(px, py, Rgba([0, 0, 0, 255]));
        }
    }
    log::info!("LIGHTROOM: Black border filled");

    // Copy original image
    let rgba_src = image.to_rgba8();
    image::imageops::overlay(&mut new_image, &rgba_src, 0, 0);
    log::info!("LIGHTROOM: Original image overlaid");

    // Calculate text scale
    let font_height_ratio = DEFAULT_LIGHTROOM_FONT_HEIGHT as f32 / 100.0;
    let txt_scale = ab_glyph::PxScale::from(font_height_ratio * border_size as f32);
    let y = new_image.height() - (border_size / 2);

    // Left text
    let left_txt = if let Some(iso) = exif.get_iso() {
        if let Some(exposure) = exif.get_exposure() {
            if let Some(fnumber) = exif.get_fnumber() {
                if !exif.focal.is_empty() {
                    format!(
                        "[ISO{}]    [{}s]    [F{}]    [{}mm]",
                        iso, exposure, fnumber, exif.focal
                    )
                } else {
                    format!("[ISO{}]    [{}s]    [F{}]", iso, exposure, fnumber)
                }
            } else {
                format!("[ISO{}]    [{}s]", iso, exposure)
            }
        } else {
            format!("[ISO{}]", iso)
        }
    } else {
        String::new()
    };

    let left_x = (border_size / 10).min(2) as i32;
    let draw_y = (y as f32
        - ((font.as_scaled(txt_scale).ascent() + font.as_scaled(txt_scale).descent().abs()) * 0.55))
        as i32;

    log::info!(
        "LIGHTROOM: Drawing left text '{}' at ({}, {})",
        left_txt,
        left_x,
        draw_y
    );
    draw_text_with_fallback(
        &mut new_image,
        color,
        left_x,
        draw_y,
        txt_scale,
        font,
        &left_txt,
    );

    // Center text
    let center_txt = if !exif.camera_mnf.is_empty() && !exif.camera_model.is_empty() {
        if !exif.lens_model.is_empty() {
            format!(
                "{}  {}    {}",
                exif.camera_mnf, exif.camera_model, exif.lens_model
            )
        } else {
            format!("{}  {}", exif.camera_mnf, exif.camera_model)
        }
    } else {
        exif.lens_model.clone()
    };

    let (center_w, _) = text_dimensions_with_fallback(txt_scale, font, &center_txt);
    let center_x = ((dyn_w as f32 - center_w) / 2.0).max(left_x as f32 + 100.0) as i32;

    log::info!(
        "LIGHTROOM: Drawing center text '{}' at ({}, {})",
        center_txt,
        center_x,
        draw_y
    );
    draw_text_with_fallback(
        &mut new_image,
        color,
        center_x,
        draw_y,
        txt_scale,
        font,
        &center_txt,
    );

    log::info!(
        "LIGHTROOM: Render complete, final size = {}x{}",
        new_image.width(),
        new_image.height()
    );
    Ok(DynamicImage::ImageRgba8(new_image))
}

/// Render Strap theme - white bottom bar with 4 lines of text (2 left, 2 right)
pub fn render_strap(
    image: DynamicImage,
    exif: &SimplifiedExif,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let (dyn_w, dyn_h) = (image.width(), image.height());
    let dyn_wh = dyn_w.max(dyn_h);
    let font = &*DIGITAL_7_FONT;
    let color = Rgba([0, 0, 0, 255]); // Black text on white bar

    log::info!("STRAP: Starting render for {}x{} image", dyn_w, dyn_h);

    // Add bottom border (slightly larger than Monitor)
    let border_size = (dyn_wh / 12).clamp(60, 150);
    log::info!("STRAP: Border size = {}", border_size);

    let mut new_image = image::RgbaImage::new(dyn_w, dyn_h + border_size);
    log::info!("STRAP: New image size = {}x{}", dyn_w, dyn_h + border_size);

    // Fill border with white
    for px in 0..dyn_w {
        for py in dyn_h..(dyn_h + border_size) {
            new_image.put_pixel(px, py, Rgba([255, 255, 255, 255]));
        }
    }
    log::info!("STRAP: White border filled");

    // Copy original image
    let rgba_src = image.to_rgba8();
    image::imageops::overlay(&mut new_image, &rgba_src, 0, 0);
    log::info!("STRAP: Original image overlaid");

    // Calculate text scale
    let font_height_ratio = 0.35; // Smaller than Monitor to fit 2 lines
    let txt_scale = ab_glyph::PxScale::from(font_height_ratio * border_size as f32);
    let line_height = font.as_scaled(txt_scale).ascent().abs();

    // Left side: exposure info + date (simplified without date for now)
    let mut exposure_parts = Vec::new();
    if let Some(iso) = exif.get_iso() {
        exposure_parts.push(format!("[ISO{}]", iso));
    }
    if !exif.focal.is_empty() {
        exposure_parts.push(format!("[{}mm]", exif.focal));
    }
    if let Some(f) = exif.get_fnumber() {
        exposure_parts.push(format!("[F{}]", f));
    }
    if let Some(sec) = exif.get_exposure() {
        exposure_parts.push(format!("[{}s]", sec));
    }

    let (left_top, left_bot) = (
        exposure_parts.join(" "),
        "2025-01-01 12:00".to_string(), // Placeholder - would need datetime from EXIF
    );

    // Right side: camera + lens
    let (right_top, right_bot) = (
        if !exif.camera_mnf.is_empty() && !exif.camera_model.is_empty() {
            format!("{} {}", exif.camera_mnf, exif.camera_model)
        } else {
            "".to_string()
        },
        exif.lens_model.clone(),
    );

    // Calculate positions
    let margin = (border_size as f32 * 0.15) as i32;
    let y_base = new_image.height() as f32 - (border_size as f32 * 0.3);

    // Draw left side (2 lines)
    let mut y = y_base;
    for txt in [&left_top, &left_bot].iter().rev() {
        let draw_y = (y - font.as_scaled(txt_scale).ascent()) as i32;
        log::info!("STRAP: Drawing left '{}' at ({}, {})", txt, margin, draw_y);
        draw_text_with_fallback(&mut new_image, color, margin, draw_y, txt_scale, font, txt);
        y -= line_height;
    }

    // Draw right side (2 lines)
    let mut y = y_base;
    for txt in [&right_top, &right_bot].iter().rev() {
        let (txt_w, _) = text_dimensions_with_fallback(txt_scale, font, txt);
        let draw_x = (dyn_w as f32 - txt_w - margin as f32) as i32;
        let draw_y = (y - font.as_scaled(txt_scale).ascent()) as i32;
        log::info!("STRAP: Drawing right '{}' at ({}, {})", txt, draw_x, draw_y);
        draw_text_with_fallback(&mut new_image, color, draw_x, draw_y, txt_scale, font, txt);
        y -= line_height;
    }

    log::info!(
        "STRAP: Render complete, final size = {}x{}",
        new_image.width(),
        new_image.height()
    );
    Ok(DynamicImage::ImageRgba8(new_image))
}

/// Render Monitor theme - black bottom bar with 4 evenly spaced text items
pub fn render_monitor(
    image: DynamicImage,
    exif: &SimplifiedExif,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let (dyn_w, dyn_h) = (image.width(), image.height());
    let dyn_wh = dyn_w.max(dyn_h);
    let font = &*DIGITAL_7_FONT;
    let color = Rgba([255, 255, 255, 255]); // White text on black bar

    log::info!("MONITOR: Starting render for {}x{} image", dyn_w, dyn_h);

    // Add bottom border
    let border_size = (dyn_wh / 16).clamp(50, 120);
    log::info!("MONITOR: Border size = {}", border_size);

    let mut new_image = image::RgbaImage::new(dyn_w, dyn_h + border_size);
    log::info!(
        "MONITOR: New image size = {}x{}",
        dyn_w,
        dyn_h + border_size
    );

    // Fill border with black
    for px in 0..dyn_w {
        for py in dyn_h..(dyn_h + border_size) {
            new_image.put_pixel(px, py, Rgba([0, 0, 0, 255]));
        }
    }
    log::info!("MONITOR: Black border filled");

    // Copy original image
    let rgba_src = image.to_rgba8();
    image::imageops::overlay(&mut new_image, &rgba_src, 0, 0);
    log::info!("MONITOR: Original image overlaid");

    // Calculate text scale (smaller than lightroom)
    let font_height_ratio = 0.6; // 60% of border height
    let txt_scale = ab_glyph::PxScale::from(font_height_ratio * border_size as f32);
    let y = new_image.height() - (border_size / 2);

    // DEBUG: Use dummy data if EXIF is empty
    let has_exif =
        exif.get_iso().is_some() || exif.get_exposure().is_some() || exif.get_fnumber().is_some();

    // 4 items to display evenly spaced
    let items: Vec<String> = if !has_exif {
        log::info!("MONITOR: No EXIF - using dummy exposure data");
        vec![
            "[F1.78]".to_string(),
            "[1/60s]".to_string(),
            "[ISO640]".to_string(),
            "[7.5mm]".to_string(),
        ]
    } else {
        let mut result = Vec::new();
        if let Some(f) = exif.get_fnumber() {
            result.push(format!("[F{}]", f));
        }
        if let Some(sec) = exif.get_exposure() {
            result.push(format!("[{}s]", sec));
        }
        if let Some(iso) = exif.get_iso() {
            result.push(format!("[ISO{}]", iso));
        }
        if !exif.focal.is_empty() {
            result.push(format!("[{}mm]", exif.focal));
        }
        result
    };

    log::info!("MONITOR: Drawing {} items evenly spaced", items.len());

    // Draw items evenly spaced
    let num_items = items.len() as f32;
    for (idx, txt) in items.iter().enumerate() {
        let (txt_w, _) = text_dimensions_with_fallback(txt_scale, font, txt);

        // Center each item in its section
        let section_x = (dyn_w as f32 / (num_items + 1.0)) * (idx as f32 + 1.0);
        let draw_x = (section_x - txt_w / 2.0) as i32;

        let yg =
            (font.as_scaled(txt_scale).ascent() + font.as_scaled(txt_scale).descent().abs()) * 0.6;
        let draw_y = (y as f32 - yg) as i32;

        log::info!("MONITOR: Drawing '{}' at ({}, {})", txt, draw_x, draw_y);
        draw_text_with_fallback(&mut new_image, color, draw_x, draw_y, txt_scale, font, txt);
    }

    log::info!(
        "MONITOR: Render complete, final size = {}x{}",
        new_image.width(),
        new_image.height()
    );
    Ok(DynamicImage::ImageRgba8(new_image))
}

/// Apply theme to image based on theme type
pub fn apply_theme(
    image: DynamicImage,
    exif: &SimplifiedExif,
    theme_type: ThemeType,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    match theme_type {
        ThemeType::Film | ThemeType::FilmDate | ThemeType::FilmGlow => render_film(image, exif),
        ThemeType::Lightroom => render_lightroom(image, exif),
        ThemeType::Strap => render_strap(image, exif),
        ThemeType::Monitor => render_monitor(image, exif),
        _ => {
            // For unsupported themes, return image as-is
            Ok(image)
        }
    }
}

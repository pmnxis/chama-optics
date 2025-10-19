/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use libheif_rs::{Channel, ColorSpace, HeifContext, LibHeif, RgbChroma};
use std::path::PathBuf;

#[allow(clippy::ptr_arg)]
pub(crate) fn load_heif(path: &PathBuf) -> Result<image::DynamicImage, Box<dyn std::error::Error>> {
    let lib = LibHeif::new();

    let ctx = HeifContext::read_from_file(path.to_str().expect("Invalid path"))?;
    let handle = ctx.primary_image_handle()?;
    let decode_opt = if let Some(mut opt) = libheif_rs::DecodingOptions::new() {
        opt.set_ignore_transformations(true);
        Some(opt)
    } else {
        log::warn!("There's possibility HEIF image get rotated");
        None
    };

    let img = lib.decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), decode_opt)?;
    let color_space = img.color_space().ok_or("Unknown HEIF color space")?;

    // color_space and bpp dependency
    let bpp = match color_space {
        ColorSpace::Monochrome => img.bits_per_pixel(Channel::Y),
        ColorSpace::Rgb(_) => img.bits_per_pixel(Channel::R),
        // ColorSpace::Rgb(RgbChroma::Rgba) => img.bits_per_pixel(Channel::Alpha),
        other => return Err(format!("Unsupported HEIF color space: {other:?}").into()),
    };

    let width = img.width();
    let height = img.height();

    // interleaved plane
    let inter = img
        .planes()
        .interleaved
        .ok_or("Unsupported: no interleaved RGBA plane")?;

    log::info!(
        "HEIF decoded: color_space={:?}, bpp={}, {}x{}",
        color_space,
        bpp.map_or("Unknown".to_owned(), |x| x.to_string()),
        width,
        height
    );

    // 8bit only considered
    let bpp = bpp.expect("Null bpp not supported.");
    if bpp == 255 {
        log::warn!("Strangely libheiff grab HEIF bpp as 255");
    } else if bpp != 8 {
        return Err(format!("Unsupported bit depth: {bpp}").into());
    }
    // Up to ColorSpace
    let dyn_img = match color_space {
        ColorSpace::Rgb(RgbChroma::Rgb) => {
            let buf: image::RgbImage =
                image::ImageBuffer::from_raw(width, height, inter.data.to_vec())
                    .ok_or("Failed to build RGB8 image buffer")?;
            image::DynamicImage::ImageRgb8(buf)
        }
        ColorSpace::Rgb(RgbChroma::Rgba) => {
            let buf: image::RgbaImage =
                image::ImageBuffer::from_raw(width, height, inter.data.to_vec())
                    .ok_or("Failed to build RGBA8 image buffer")?;
            image::DynamicImage::ImageRgba8(buf)
        }
        ColorSpace::Monochrome => {
            let buf: image::ImageBuffer<image::Luma<u8>, Vec<u8>> =
                image::ImageBuffer::from_raw(width, height, inter.data.to_vec())
                    .ok_or("Failed to build Luma8 image buffer")?;
            image::DynamicImage::ImageLuma8(buf)
        }
        other => {
            return Err(format!("Unsupported HEIF color space: {other:?}").into());
        }
    };

    Ok(dyn_img)
}

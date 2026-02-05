/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Image utility functions without GUI dependencies
//! Headless versions of functions from image/common.rs

use fast_image_resize as fr;

pub const THUMBNAIL_MAX_WIDTH: u32 = 330 * 2;
pub const THUMBNAIL_MAX_HEIGHT: u32 = 220 * 2;

pub const THUMBMANIL_SCALE: super::scale_config::ScaleConfig = super::scale_config::ScaleConfig {
    mode: super::scale_config::ScaleMode::ResizeAndCrop,
    value: THUMBNAIL_MAX_WIDTH,
    sub_value: THUMBNAIL_MAX_HEIGHT,
    scale_value: 2.0, // Don't care
};

/// Resize an image using fast_image_resize
pub fn resize_image(
    decoded_image: image::DynamicImage,
    new_width: u32,
    new_height: u32,
) -> Result<fr::images::Image<'static>, image::ImageError> {
    let src_image = decoded_image.to_rgba8();
    let (src_width, src_height) = src_image.dimensions();

    let src_image = fr::images::Image::from_vec_u8(
        src_width,
        src_height,
        src_image.into_raw(),
        fr::PixelType::U8x4,
    )
    .map_err(|e| {
        log::error!("resize_image error: {e:?}");

        image::ImageError::Encoding(image::error::EncodingError::new(
            image::error::ImageFormatHint::Unknown,
            format!("resize failed: {e:?}"),
        ))
    })?;

    let mut dst_image: fr::images::Image<'static> =
        fr::images::Image::new(new_width, new_height, fr::PixelType::U8x4);

    let mut resizer = fr::Resizer::new();
    resizer.resize(&src_image, &mut dst_image, None).unwrap();

    Ok(dst_image)
}

/// Load an image from path
/// Returns (DynamicImage, needs_orientation_applied)
pub fn load_image(
    path: &std::path::Path,
    buf_reader: &mut std::io::BufReader<std::fs::File>,
) -> Result<(image::DynamicImage, bool), image::ImageError> {
    let img_format = path
        .extension()
        .filter(|ext| !ext.is_empty())
        .and_then(image::ImageFormat::from_extension);

    let decoder = if let Some(fmt) = img_format {
        image::ImageReader::with_format(buf_reader, fmt)
    } else {
        image::ImageReader::new(buf_reader)
    };

    decoder.decode().map_or_else(
        |heic_suppose_or_err| {
            // Attempt HEIC/HEIF loading
            match heic_suppose_or_err {
                image::ImageError::Unsupported(unsp_e) => {
                    if img_format.is_none() {
                        // Try HEIC loader if available (libheif on Windows/Linux)
                        #[cfg(feature = "libheif")]
                        {
                            crate::image::heic::load_heif(path)
                                .map(|img| (img, false))
                                .map_err(|e| {
                                    image::error::ImageError::Unsupported(
                                        image::error::UnsupportedError::from_format_and_kind(
                                            image::error::ImageFormatHint::PathExtension(
                                                path.to_path_buf(),
                                            ),
                                            image::error::UnsupportedErrorKind::GenericFeature(
                                                format!("libheif error {e} and unsp_e: {unsp_e}"),
                                            ),
                                        ),
                                    )
                                })
                        }
                        #[cfg(not(feature = "libheif"))]
                        {
                            Err(image::error::ImageError::Unsupported(unsp_e))
                        }
                    } else {
                        Err(image::error::ImageError::Unsupported(unsp_e))
                    }
                }
                other_err => Err(other_err),
            }
        },
        |img| Ok((img, true)),
    )
}

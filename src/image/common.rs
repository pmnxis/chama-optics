/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[cfg(any(feature = "desktop", feature = "ios_integration"))]
use egui::ColorImage;
use fast_image_resize as fr;

pub const THUMBNAIL_MAX_WIDTH: u32 = 330 * 2;
pub const THUMBNAIL_MAX_HEIGHT: u32 = 220 * 2;
pub const THUMBNAIL_MAX_WIDTH_AS_F32: f32 = 110.0; // considering retina display
pub const THUMBNAIL_MAX_HEIGHT_AS_F32: f32 = 165.0; // considering retina display
#[cfg(any(feature = "desktop", feature = "ios_integration"))]
pub const THUMBNAIL_DIMM: egui::Vec2 =
    egui::Vec2::new(THUMBNAIL_MAX_HEIGHT_AS_F32, THUMBNAIL_MAX_WIDTH_AS_F32);

#[cfg(any(feature = "desktop", feature = "web", feature = "ios_integration"))]
pub const THUMBMANIL_SCALE: crate::export_config::scale_config::ScaleConfig =
    crate::export_config::scale_config::ScaleConfig {
        mode: crate::export_config::scale_config::ScaleMode::ResizeAndCrop,
        value: THUMBNAIL_MAX_WIDTH,
        sub_value: THUMBNAIL_MAX_HEIGHT,
        scale_value: 2.0, // Don't care
    };

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum PackedImageEvent {
    None,
    Remove,
}

pub(crate) fn resize_image(
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
        log::error!("thumbnail : {e:?}");

        image::ImageError::Encoding(image::error::EncodingError::new(
            image::error::ImageFormatHint::Unknown,
            format!("thumbnail source prepare failed by {e:?}"),
        ))
    })?;

    // use fr instead of image
    // decoded_image.thumbnail(new_width, new_height)

    // let src_image = fr::images::Image::from_vec_u8(src_width, src_height, src_image.into_raw(), fr::PixelType::U8x4);
    // let src_image = fr::images::Image::from_slice_u8(src_width, src_height, &mut src_image.as_bytes(), src_image.as_bytes(), fr::PixelType::U8x4);

    // egui's ColorImage will upload to GPU, and it use F32x4
    let mut dst_image: fr::images::Image<'static> =
        fr::images::Image::new(new_width, new_height, fr::PixelType::U8x4);
    // let mut dst_image = fr::images::Image::new(new_width, new_height, fr::PixelType::F32x4);
    let mut resizer = fr::Resizer::new();
    resizer.resize(&src_image, &mut dst_image, None).unwrap();

    Ok(dst_image)
}

#[cfg(any(feature = "desktop", feature = "ios_integration"))]
pub(crate) fn gen_thumbnail(
    decoded_image: image::DynamicImage,
    orientation: image::metadata::Orientation,
) -> Result<ColorImage, image::ImageError> {
    // future todo
    // resolve RGB -> RGBA makes clone+compute resource
    // Also this function does not cover U16 slice such as HDR
    let (src_width, src_height) = (decoded_image.width(), decoded_image.height());

    let is_vert_rot = crate::exif_impl::__is_vertical_rotated(orientation);

    let (mid_width, mid_height) = THUMBMANIL_SCALE.apply(src_width, src_height, is_vert_rot);
    let resized_image: fast_image_resize::images::Image<'static> =
        resize_image(decoded_image, mid_width, mid_height)?;

    let image_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        mid_width,
        mid_height,
        resized_image.into_vec(),
    )
    .expect("Failed to convert to ImageBuffer");

    let mut dyn_image = image::DynamicImage::ImageRgba8(image_buffer);

    dyn_image.apply_orientation(orientation);
    let x = (dyn_image.width() - THUMBNAIL_MAX_WIDTH) / 2;
    let y = (dyn_image.height() - THUMBNAIL_MAX_HEIGHT) / 2;

    let dyn_image = dyn_image.crop(x, y, THUMBNAIL_MAX_WIDTH, THUMBNAIL_MAX_HEIGHT);

    Ok(ColorImage::from_rgba_unmultiplied(
        [THUMBNAIL_MAX_WIDTH as usize, THUMBNAIL_MAX_HEIGHT as usize],
        dyn_image.as_bytes(),
    ))
}

/// image::DynamicImage and is orientation required or not with boolean signal
pub(crate) fn __load_image(
    path: &std::path::PathBuf,
    buf_reader: &mut std::io::BufReader<std::fs::File>,
) -> Result<(image::DynamicImage, bool), image::ImageError> {
    let img_format = path
        .extension()
        .filter(|ext| !ext.is_empty())
        .and_then(image::ImageFormat::from_extension);

    let buf_reader = if img_format.is_some() {
        buf_reader
    } else {
        &mut std::io::BufReader::new(std::fs::File::open(path)?)
    };

    let decoder = if let Some(fmt) = img_format {
        image::ImageReader::with_format(
            buf_reader, // std::io::BufReader::new(std::fs::File::open(path)?),
            fmt,
        )
    } else {
        image::ImageReader::new(buf_reader)
    };

    decoder.decode().map_or_else(
        // let dyn_image = image::ImageReader::open(path)?.decode().map_or_else(
        |heic_suppose_or_err| {
            // Suppose HEIC/HEIF
            match heic_suppose_or_err {
                // Since libheif is depend on FFIed C library.
                // Pass buffer reader in to ffi is difficult.
                // Keep using path
                image::ImageError::Unsupported(unsp_e) => {
                    #[cfg(feature = "desktop")]
                    {
                        if img_format.is_none() {
                            crate::image::heic::load_heif(path)
                                .map(|img| (img, false))
                                .map_err(|e| {
                                    image::error::ImageError::Unsupported(
                                        image::error::UnsupportedError::from_format_and_kind(
                                            image::error::ImageFormatHint::PathExtension(
                                                path.to_path_buf(),
                                            ),
                                            image::error::UnsupportedErrorKind::GenericFeature(
                                                format!(
                                                    "libheif internal error {e} and unsp_e : {unsp_e}"
                                                ),
                                            ),
                                        ),
                                    )
                                })
                        } else {
                            Err(image::error::ImageError::Unsupported(unsp_e))
                        }
                    }
                    #[cfg(not(feature = "desktop"))]
                    {
                        Err(image::error::ImageError::Unsupported(unsp_e))
                    }
                }
                other_err => Err(other_err),
            }
        },
        |img| Ok((img, true)),
    )
}

pub(crate) fn __load_image_from_vec(
    path: &std::path::PathBuf, // just hint
    v: std::vec::Vec<u8>,
) -> Result<(image::DynamicImage, bool), image::ImageError> {
    let img_format = path
        .extension()
        .filter(|ext| !ext.is_empty())
        .and_then(image::ImageFormat::from_extension);

    let buf_reader = std::io::BufReader::new(std::io::Cursor::new(v));

    let decoder = if let Some(fmt) = img_format {
        image::ImageReader::with_format(
            buf_reader, // std::io::BufReader::new(std::fs::File::open(path)?),
            fmt,
        )
    } else {
        image::ImageReader::new(buf_reader)
    };

    decoder.decode().map_or_else(
        // let dyn_image = image::ImageReader::open(path)?.decode().map_or_else(
        |heic_suppose_or_err| {
            // Suppose HEIC/HEIF
            match heic_suppose_or_err {
                // Since libheif is depend on FFIed C library.
                // Pass buffer reader in to ffi is difficult.
                // Keep using path
                image::ImageError::Unsupported(unsp_e) => {
                    #[cfg(feature = "desktop")]
                    {
                        if img_format.is_none() {
                            crate::image::heic::load_heif(path)
                                .map(|img| (img, false))
                                .map_err(|e| {
                                    image::error::ImageError::Unsupported(
                                        image::error::UnsupportedError::from_format_and_kind(
                                            image::error::ImageFormatHint::PathExtension(
                                                path.to_path_buf(),
                                            ),
                                            image::error::UnsupportedErrorKind::GenericFeature(
                                                format!(
                                                    "libheif internal error {e} and unsp_e : {unsp_e}"
                                                ),
                                            ),
                                        ),
                                    )
                                })
                        } else {
                            Err(image::error::ImageError::Unsupported(unsp_e))
                        }
                    }
                    #[cfg(not(feature = "desktop"))]
                    {
                        Err(image::error::ImageError::Unsupported(unsp_e))
                    }
                }
                other_err => Err(other_err),
            }
        },
        |img| Ok((img, true)),
    )
}

#[cfg(any(feature = "desktop", feature = "ios_integration"))]
#[derive(PartialEq, Eq, Clone)]
pub enum PackedTexture {
    Real { texture: egui::TextureHandle },
    Dummy,
}

#[cfg(any(feature = "desktop", feature = "ios_integration"))]
impl PackedTexture {
    pub fn get(&self) -> &egui::TextureHandle {
        #[cfg(test)]
        {
            panic!("Under test environment, cannot use egui texture");
        }
        #[allow(unreachable_code)]
        #[cfg(not(test))]
        {
            if let PackedTexture::Real { texture } = self {
                texture
            } else {
                panic!("Something wrong because tried to access dummy texture")
            }
        }
    }

    #[cfg(any(feature = "desktop", feature = "ios_integration"))]
    pub fn new(texture: egui::TextureHandle) -> PackedTexture {
        Self::Real { texture }
    }

    pub fn dummy() -> PackedTexture {
        Self::Dummy
    }
}

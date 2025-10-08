/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use egui::ColorImage;
use exif::Exif;
use fast_image_resize as fr;
use std::io::Seek;
use std::path::PathBuf;

pub const THUMBNAIL_MAX_HEIGHT: u32 = 320;
pub const THUMBNAIL_MAX_HEIGHT_AS_F32: f32 = 160.0; // considering retina display

#[non_exhaustive]
pub struct PackedImage {
    /// path of image
    pub path: PathBuf,

    /// EXIF from image
    pub src_exif: Option<Exif>,
    // /// editable EXIF
    // pub
}

fn to_thumbnail_colorimage(
    decoded_image: image::DynamicImage,
    max_height: u32,
) -> Result<ColorImage, image::ImageError> {
    // future todo
    // resolve RGB -> RGBA makes clone+compute resource
    // Also this function does not cover U16 slice such as HDR
    let src_image = decoded_image.to_rgba8();
    let (src_width, src_height) = src_image.dimensions();

    let new_height = max_height;
    let new_width = (max_height * src_width) / src_height;

    // use fr instead of image
    // decoded_image.thumbnail(new_width, new_height)

    // let src_image = fr::images::Image::from_vec_u8(src_width, src_height, src_image.into_raw(), fr::PixelType::U8x4);
    // let src_image = fr::images::Image::from_slice_u8(src_width, src_height, &mut src_image.as_bytes(), src_image.as_bytes(), fr::PixelType::U8x4);

    // future todo replace to from_slice_u8
    let src_image = fr::images::Image::from_vec_u8(
        src_width,
        src_height,
        src_image.into_raw(),
        fr::PixelType::U8x4,
    )
    .map_err(|e| {
        println!("thumbnail : {:?}", e);
        // log::error!("thumbnail : {:?}", e);
        image::ImageError::Encoding(image::error::EncodingError::new(
            image::error::ImageFormatHint::Unknown,
            format!("thumbnail source prepare failed by {:?}", e),
        ))
    })?;

    // egui's ColorImage will upload to GPU, and it use F32x4
    let mut dst_image = fr::images::Image::new(new_width, new_height, fr::PixelType::U8x4);
    // let mut dst_image = fr::images::Image::new(new_width, new_height, fr::PixelType::F32x4);
    let mut resizer = fr::Resizer::new();
    resizer.resize(&src_image, &mut dst_image, None).unwrap();

    // at last FR do U8x4 -> Resize -> Floating -> F32x4
    // let data = ColorImage {
    //     size: [new_width as usize, new_height as usize],
    //     pixels: dst_image.buffer().into(),
    // };

    // Finally return GPU friendly object
    Ok(ColorImage::from_rgba_unmultiplied(
        [new_width as usize, new_height as usize],
        dst_image.buffer(),
    ))
}

impl PackedImage {
    pub fn try_from_path(path: &PathBuf) -> Result<(Self, ColorImage), image::ImageError> {
        let file = std::fs::File::open(path)?;
        let mut buf_reader = std::io::BufReader::new(file);

        // Parse EXIF first
        let exif = match exif::Reader::new().read_from_container(&mut buf_reader) {
            Ok(exif) => Some(exif),
            Err(e) => {
                log::error!("Failed to parse EXIF from image: {e:?}");
                None
            }
        };

        buf_reader
            .seek(std::io::SeekFrom::Start(0))
            .expect("Failed reset seek zero");

        let img_format = path
            .extension()
            .filter(|ext| !ext.is_empty())
            .and_then(image::ImageFormat::from_extension);

        let decoder = if let Some(fmt) = img_format {
            image::ImageReader::with_format(
                std::io::BufReader::new(std::fs::File::open(path)?),
                fmt,
            )
        } else {
            image::ImageReader::new(std::io::BufReader::new(std::fs::File::open(path)?))
        };

        let dyn_image = decoder.decode().map_or_else(
            // let dyn_image = image::ImageReader::open(path)?.decode().map_or_else(
            |heic_suppose_or_err| {
                // Suppose HEIC/HEIF
                match heic_suppose_or_err {
                    // Since libheif is depend on FFIed C library.
                    // Pass buffer reader in to ffi is difficult.
                    // Keep using path
                    image::ImageError::Unsupported(unsp_e) => {
                        if img_format.is_none() {
                            crate::heic::load_heif(path).map_err(|e| {
                                image::error::ImageError::Unsupported(
                                    image::error::UnsupportedError::from_format_and_kind(
                                        image::error::ImageFormatHint::PathExtension(
                                            path.to_path_buf(),
                                        ),
                                        image::error::UnsupportedErrorKind::GenericFeature(
                                            format!(
                                                "libheif internal error {} and unsp_e : {}",
                                                e, unsp_e
                                            ),
                                        ),
                                    ),
                                )
                            })
                        } else {
                            Err(image::error::ImageError::Unsupported(unsp_e))
                        }
                    }
                    other_err => Err(other_err),
                }
            },
            Ok,
        )?;

        let thumbnail = to_thumbnail_colorimage(dyn_image, THUMBNAIL_MAX_HEIGHT)?;

        Ok((
            PackedImage {
                path: path.clone(),
                src_exif: exif,
            },
            thumbnail,
        ))
    }

    pub fn file_name(&self) -> String {
        self.path
            .clone()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    pub fn file_path(&self) -> String {
        self.path.clone().to_string_lossy().to_string()
    }
}

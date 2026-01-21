/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Core image data structures without GUI dependencies

use std::path::PathBuf;

use crate::exif_impl::{OriginalExif, SimplifiedExif};

/// Core image structure without any GUI dependencies
/// This is a headless version of PackedImage
pub struct CoreImage {
    /// Path of the image file
    pub path: PathBuf,

    /// Original EXIF data from the image
    pub src_exif: OriginalExif,

    /// Editable EXIF metadata
    pub view_exif: SimplifiedExif,

    /// Whether EXIF is editable
    pub editable: bool,

    /// Cached thumbnail data (RGBA8, width, height)
    /// Using Option to avoid loading until needed
    thumbnail_cache: Option<(Vec<u8>, u32, u32)>,
}

impl CoreImage {
    /// Create a new CoreImage from a file path
    pub fn from_path(path: PathBuf) -> Result<Self, image::ImageError> {
        log::info!("Loading image from: {:?}", path);
        let file = std::fs::File::open(&path)?;
        let mut buf_reader = std::io::BufReader::new(file);

        // Get EXIF data
        let (exif_opt, thumbnail_data) = Self::get_exif_with_thumbnail(&mut buf_reader);

        log::info!("EXIF data present: {}", exif_opt.is_some());

        let src_exif = OriginalExif::new(exif_opt);
        let view_exif = SimplifiedExif::from(&src_exif);

        log::info!(
            "Parsed EXIF - Camera: {}, Lens: {}",
            view_exif.camera_model,
            view_exif.lens_model
        );

        Ok(Self {
            path,
            src_exif,
            view_exif,
            editable: true,
            thumbnail_cache: thumbnail_data.map(|data| {
                // Decode thumbnail to RGBA8 for caching
                if let Ok(img) = image::load_from_memory(&data) {
                    let rgba = img.to_rgba8();
                    let (w, h) = rgba.dimensions();
                    (rgba.into_raw(), w, h)
                } else {
                    // Return empty cache if thumbnail decode fails
                    (Vec::new(), 0, 0)
                }
            }),
        })
    }

    /// Get EXIF data with thumbnail
    fn get_exif_with_thumbnail(
        buf_reader: &mut std::io::BufReader<std::fs::File>,
    ) -> (Option<exif::Exif>, Option<Vec<u8>>) {
        match exif::Reader::new().read_from_container(buf_reader) {
            Ok(exif) => {
                let thumbnail =
                    if let Some(biggest) = exif.thumbnails().iter().max_by_key(|e| e.length) {
                        if biggest.length >= 100 * 1024 {
                            biggest.extract_data(buf_reader).ok()
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                (Some(exif), thumbnail)
            }
            Err(e) => {
                log::info!("Failed to read EXIF from image: {}", e);
                (None, None)
            }
        }
    }

    /// Load the full image as DynamicImage
    /// Returns (image, needs_orientation_applied)
    pub fn load_image(&self) -> Result<(image::DynamicImage, bool), image::ImageError> {
        let file = std::fs::File::open(&self.path)?;
        let mut buf_reader = std::io::BufReader::new(file);
        super::image_utils::load_image(&self.path, &mut buf_reader)
    }

    /// Load image with scale and orientation applied
    pub fn load_with_scale(
        &self,
        scale: super::scale_config::ScaleConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        use image::ImageBuffer;
        use image::Rgba;

        let (dyn_image, need_orientation) = self.load_image()?;
        let orientation = if need_orientation {
            self.view_exif.orientation
        } else {
            image::metadata::Orientation::NoTransforms
        };

        let (old_width, old_height) = (dyn_image.width(), dyn_image.height());
        let (new_width, new_height) =
            scale.apply(old_width, old_height, self.view_exif.is_vertical_rotated());

        log::debug!("Scaling: ({old_width} x {old_height}) -> ({new_width} x {new_height})");

        let resized_image: fast_image_resize::images::Image<'static> =
            super::image_utils::resize_image(dyn_image, new_width, new_height)?;

        let buffer =
            ImageBuffer::<Rgba<u8>, _>::from_raw(new_width, new_height, resized_image.into_vec())
                .expect("Failed to convert to ImageBuffer");

        let mut dyn_image = image::DynamicImage::ImageRgba8(buffer);
        dyn_image.apply_orientation(orientation);

        Ok(dyn_image)
    }

    /// Load an image directly from a path without any CoreImage metadata
    /// This is a static method used for FFI functions where we need to apply effects and save immediately
    /// The image is automatically rotated according to EXIF orientation
    pub fn load_image_direct(
        path: &std::path::Path,
    ) -> Result<image::DynamicImage, image::ImageError> {
        use image::ImageReader;
        let mut dyn_image = ImageReader::open(path)?.decode()?;

        // Read EXIF orientation and apply it so the image matches the visual orientation
        let orientation = {
            use exif::{In, Tag};
            let file = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(_) => return Ok(dyn_image), // No EXIF available, return as-is
            };
            let mut buf_reader = std::io::BufReader::new(file);
            match exif::Reader::new().read_from_container(&mut buf_reader) {
                Ok(exif) => {
                    let value = exif
                        .get_field(Tag::Orientation, In::PRIMARY)
                        .and_then(|field| field.value.get_uint(0));
                    image::metadata::Orientation::from_exif(value.unwrap_or(0) as u8)
                        .unwrap_or(image::metadata::Orientation::NoTransforms)
                }
                Err(_) => image::metadata::Orientation::NoTransforms,
            }
        };

        log::debug!(
            "load_image_direct: {:?}, orientation: {:?}, size: {}x{}",
            path,
            orientation,
            dyn_image.width(),
            dyn_image.height()
        );

        dyn_image.apply_orientation(orientation);

        log::debug!(
            "load_image_direct after orientation: {}x{}",
            dyn_image.width(),
            dyn_image.height()
        );

        Ok(dyn_image)
    }

    /// Get thumbnail data (RGBA8, width, height)
    pub fn get_thumbnail(&mut self) -> Result<(Vec<u8>, u32, u32), image::ImageError> {
        // If thumbnail is already cached, return a clone
        if let Some((ref data, w, h)) = self.thumbnail_cache
            && !data.is_empty()
        {
            return Ok((data.clone(), w, h));
        }

        // Otherwise, generate thumbnail
        let (dyn_image, need_orientation) = self.load_image()?;
        let orientation = if need_orientation {
            self.view_exif.orientation
        } else {
            image::metadata::Orientation::NoTransforms
        };

        // Generate thumbnail using the same logic as common.rs
        let (src_width, src_height) = (dyn_image.width(), dyn_image.height());
        let is_vert_rot = self.view_exif.is_vertical_rotated();

        use super::image_utils::{THUMBMANIL_SCALE, THUMBNAIL_MAX_HEIGHT, THUMBNAIL_MAX_WIDTH};

        let (mid_width, mid_height) = THUMBMANIL_SCALE.apply(src_width, src_height, is_vert_rot);
        let resized_image: fast_image_resize::images::Image<'static> =
            super::image_utils::resize_image(dyn_image, mid_width, mid_height)?;

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
        let data = dyn_image.to_rgba8().into_raw();

        // Cache the thumbnail
        self.thumbnail_cache = Some((data.clone(), THUMBNAIL_MAX_WIDTH, THUMBNAIL_MAX_HEIGHT));

        Ok((data, THUMBNAIL_MAX_WIDTH, THUMBNAIL_MAX_HEIGHT))
    }
}

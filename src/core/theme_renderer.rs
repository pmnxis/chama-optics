/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Headless theme rendering for iOS and other platforms
//! This module delegates to actual theme implementations in src/theme/*.rs

use crate::core::ThemeType;
use crate::exif_impl::SimplifiedExif;
use crate::theme::Theme;
use image::DynamicImage;

/// Helper function to create a PackedImage from DynamicImage and EXIF
/// This is used for headless rendering on iOS and other platforms
fn create_packed_image_from_dynamic(
    _image: DynamicImage,
    exif: SimplifiedExif,
) -> Result<crate::packed_image::PackedImage, Box<dyn std::error::Error>> {
    use crate::exif_impl::OriginalExif;
    use std::path::PathBuf;

    // Create a dummy path since we're working with in-memory images
    let dummy_path = PathBuf::from("in_memory_image.jpg");

    Ok(crate::packed_image::PackedImage {
        uuid: uuid::Uuid::new_v4(),
        path: dummy_path.clone(),
        src_exif: OriginalExif::new(None),
        view_exif: exif,
        editable: false,
        texture: crate::image::common::PackedTexture::dummy(),
        #[cfg(not(feature = "desktop"))]
        image_bytes: None,
        sticker_bytes: None,
        perceptual_hash: None,
        configured_faces: Vec::new(),
    })
}

/// Apply theme to image based on theme type
/// This delegates to actual theme implementations in src/theme/*.rs
pub fn apply_theme(
    image: DynamicImage,
    exif: &SimplifiedExif,
    theme_type: ThemeType,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    match theme_type {
        ThemeType::Film => {
            let theme = crate::theme::film::Film::default();
            let pi = create_packed_image_from_dynamic(image, exif.clone())?;
            let export_config = crate::export_config::ExportConfig::default();
            theme
                .apply_to_image(&pi, &export_config)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
        ThemeType::FilmDate => {
            let theme = crate::theme::film_date::FilmDate::default();
            let pi = create_packed_image_from_dynamic(image, exif.clone())?;
            let export_config = crate::export_config::ExportConfig::default();
            theme
                .apply_to_image(&pi, &export_config)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
        ThemeType::FilmGlow => {
            let theme = crate::theme::film_glow::FilmGlow::default();
            let pi = create_packed_image_from_dynamic(image, exif.clone())?;
            let export_config = crate::export_config::ExportConfig::default();
            theme
                .apply_to_image(&pi, &export_config)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
        ThemeType::Lightroom => {
            let theme = crate::theme::lightroom::Lightroom::default();
            let pi = create_packed_image_from_dynamic(image, exif.clone())?;
            let export_config = crate::export_config::ExportConfig::default();
            theme
                .apply_to_image(&pi, &export_config)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
        ThemeType::Strap => {
            let theme = crate::theme::strap::Strap::default();
            let pi = create_packed_image_from_dynamic(image, exif.clone())?;
            let export_config = crate::export_config::ExportConfig::default();
            theme
                .apply_to_image(&pi, &export_config)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
        ThemeType::Monitor => {
            let theme = crate::theme::monitor::Monitor::default();
            let pi = create_packed_image_from_dynamic(image, exif.clone())?;
            let export_config = crate::export_config::ExportConfig::default();
            theme
                .apply_to_image(&pi, &export_config)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
        _ => {
            // For unsupported themes, return image as-is
            Ok(image)
        }
    }
}

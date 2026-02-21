/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Output format, but actually describe about encoder configuration together

use image::{DynamicImage, ImageEncoder};
use serde::{Deserialize, Serialize};
use std::path::Path;
use strum::EnumIter;

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
use rust_i18n::t;

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
use strum::IntoEnumIterator;

#[derive(Debug, EnumIter, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutputExtension {
    Jpeg,
    Webp,
    PngOptimized,
}

impl OutputExtension {
    fn extension(&self) -> &str {
        match self {
            Self::Jpeg { .. } => "jpg",
            Self::Webp { .. } => "webp",
            Self::PngOptimized => "png",
        }
    }

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    fn label(&self) -> &str {
        match self {
            Self::Jpeg { .. } => "JPEG",
            Self::Webp { .. } => "WEBP",
            Self::PngOptimized => "PNG",
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputFormat {
    pub ext: OutputExtension,
    pub quality: u8,
}

impl OutputFormat {
    pub fn extension(&self) -> &str {
        self.ext.extension()
    }
}

impl core::default::Default for OutputFormat {
    fn default() -> Self {
        Self {
            ext: OutputExtension::Webp,
            quality: 90,
        }
    }
}

// --- Native encoders (mozjpeg, webp) — desktop only ---

#[cfg(feature = "native_encoders")]
fn save_jpeg_moz<P: AsRef<Path>>(
    img: image::RgbImage,
    path: P,
    quality: u8,
) -> Result<(), image::ImageError> {
    use mozjpeg::ColorSpace;

    // Ensure parent directory exists
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut comp = mozjpeg::Compress::new(ColorSpace::JCS_RGB);
    comp.set_size(img.width() as usize, img.height() as usize);
    comp.set_quality(quality as f32);
    comp.set_optimize_scans(true);
    comp.set_progressive_mode();

    let mut comp = comp.start_compress(Vec::new())?;
    comp.write_scanlines(&img)?;
    let jpeg_data = comp.finish()?;

    std::fs::write(path, jpeg_data)?;
    Ok(())
}

#[cfg(feature = "native_encoders")]
fn save_webp<P: AsRef<Path>>(
    img: image::RgbImage,
    path: P,
    quality: u8,
) -> Result<(), image::ImageError> {
    use webp::Encoder;

    // Ensure parent directory exists
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    let encoder = Encoder::from_rgb(&img, img.width(), img.height());
    let webp_data = encoder.encode(quality as f32);
    std::fs::write(path, &*webp_data)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn save_png<P: AsRef<Path>>(img: &DynamicImage, path: P) -> Result<(), image::ImageError> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};

    // Ensure parent directory exists
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::File::create(path)?;
    let writer = std::io::BufWriter::new(file);
    let encoder = PngEncoder::new_with_quality(writer, CompressionType::Best, FilterType::Adaptive);

    encoder.write_image(
        &img.to_rgb8(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgb8,
    )?;

    Ok(())
}

// --- Fallback encoders (pure Rust, filesystem) — non-WASM only ---

/// Encode JPEG using image crate's built-in encoder (fallback for non-native-encoders desktop)
#[cfg(all(not(feature = "native_encoders"), not(target_arch = "wasm32")))]
fn save_jpeg_fallback<P: AsRef<Path>>(
    img: image::RgbImage,
    path: P,
    quality: u8,
) -> Result<(), image::ImageError> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::File::create(path)?;
    let writer = std::io::BufWriter::new(file);
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, quality);
    encoder.write_image(
        &img,
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(())
}

/// Encode image to bytes in memory (for WASM download)
pub fn encode_jpeg_to_bytes(
    img: &image::RgbImage,
    quality: u8,
) -> Result<Vec<u8>, image::ImageError> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    encoder.write_image(
        img,
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(buf.into_inner())
}

/// Encode PNG to bytes in memory (for WASM download)
pub fn encode_png_to_bytes(img: &DynamicImage) -> Result<Vec<u8>, image::ImageError> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};

    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder =
        PngEncoder::new_with_quality(&mut buf, CompressionType::Best, FilterType::Adaptive);
    encoder.write_image(
        &img.to_rgb8(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(buf.into_inner())
}

impl OutputFormat {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_image<P: AsRef<Path>>(
        &self,
        img: &DynamicImage,
        path: P,
    ) -> Result<(), image::ImageError> {
        match self.ext {
            OutputExtension::Jpeg | OutputExtension::Webp => {
                let rgb = img.to_rgb8();
                match self.ext {
                    #[cfg(feature = "native_encoders")]
                    OutputExtension::Jpeg => save_jpeg_moz(rgb, path, self.quality),
                    #[cfg(not(feature = "native_encoders"))]
                    OutputExtension::Jpeg => save_jpeg_fallback(rgb, path, self.quality),
                    #[cfg(feature = "native_encoders")]
                    OutputExtension::Webp => save_webp(rgb, path, self.quality),
                    #[cfg(not(feature = "native_encoders"))]
                    OutputExtension::Webp => {
                        // Fallback: save as JPEG when native WebP encoder not available
                        save_jpeg_fallback(rgb, path, self.quality)
                    }
                    _ => unreachable!(),
                }
            }
            OutputExtension::PngOptimized => save_png(img, path),
        }
    }

    /// WASM stub: save_image can't write to filesystem, so encode to bytes and discard.
    /// The WASM export pipeline should use encode_to_bytes() directly instead.
    #[cfg(target_arch = "wasm32")]
    pub fn save_image<P: AsRef<Path>>(
        &self,
        _img: &DynamicImage,
        _path: P,
    ) -> Result<(), image::ImageError> {
        log::warn!("OutputFormat::save_image called on WASM — filesystem not available");
        Ok(())
    }

    /// Encode image to bytes in memory (for WASM download or other in-memory use)
    pub fn encode_to_bytes(&self, img: &DynamicImage) -> Result<Vec<u8>, image::ImageError> {
        match self.ext {
            OutputExtension::Jpeg | OutputExtension::Webp => {
                let rgb = img.to_rgb8();
                // Use JPEG encoder for both JPEG and WebP fallback in WASM
                encode_jpeg_to_bytes(&rgb, self.quality)
            }
            OutputExtension::PngOptimized => encode_png_to_bytes(img),
        }
    }

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    fn has_quality(&self) -> bool {
        matches!(self.ext, OutputExtension::Jpeg | OutputExtension::Webp)
    }

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(t!("output_format.label"));

            egui::ComboBox::from_id_salt("export_format_combo")
                .selected_text(self.ext.label())
                .show_ui(ui, |ui| {
                    for ext in OutputExtension::iter() {
                        // WebP uses native C encoder (libwebp) — not available on WASM
                        #[cfg(target_arch = "wasm32")]
                        if ext == OutputExtension::Webp {
                            continue;
                        }
                        ui.selectable_value(&mut self.ext, ext, ext.label());
                    }
                });

            if self.has_quality() {
                ui.add(
                    egui::Slider::new(&mut self.quality, 1..=100).text(t!("output_format.quality")),
                );
            }
        });
    }
}

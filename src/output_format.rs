/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Output format, but actually describe about encoder configuration together

use image::{DynamicImage, ImageEncoder};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::path::Path;
use strum::{EnumIter, IntoEnumIterator};

#[derive(EnumIter, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
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

    fn label(&self) -> &str {
        match self {
            Self::Jpeg { .. } => "JPEG",
            Self::Webp { .. } => "WEBP",
            Self::PngOptimized => "PNG",
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
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

fn save_jpeg_moz<P: AsRef<Path>>(
    img: image::RgbImage,
    path: P,
    quality: u8,
) -> Result<(), image::ImageError> {
    use mozjpeg::ColorSpace;
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

fn save_webp<P: AsRef<Path>>(
    img: image::RgbImage,
    path: P,
    quality: u8,
) -> Result<(), image::ImageError> {
    use webp::Encoder;
    let encoder = Encoder::from_rgb(&img, img.width(), img.height());
    let webp_data = encoder.encode(quality as f32);
    std::fs::write(path, &*webp_data)?;
    Ok(())
}

fn save_png<P: AsRef<Path>>(img: &DynamicImage, path: P) -> Result<(), image::ImageError> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
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

impl OutputFormat {
    pub fn save_image<P: AsRef<Path>>(
        &self,
        img: &DynamicImage,
        path: P,
    ) -> Result<(), image::ImageError> {
        match self.ext {
            OutputExtension::Jpeg => save_jpeg_moz(img.to_rgb8(), path, self.quality),
            OutputExtension::Webp => save_webp(img.to_rgb8(), path, self.quality),
            OutputExtension::PngOptimized => save_png(img, path),
        }
    }

    fn has_quality(&self) -> bool {
        matches!(self.ext, OutputExtension::Jpeg | OutputExtension::Webp)
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(t!("output_format.label"));

            egui::ComboBox::from_id_salt("export_format_combo")
                .selected_text(self.ext.label())
                .show_ui(ui, |ui| {
                    for ext in OutputExtension::iter() {
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

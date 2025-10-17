/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use std::path::Path;

use image::{DynamicImage, ImageEncoder};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

// todo! - change to struct and enum/u8
#[derive(EnumIter, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum OutputFormat {
    Jpeg { quality: u8 }, // quality 0~100
    Webp { quality: u8 }, // quality 0~100
    PngOptimized,
}

impl std::default::Default for OutputFormat {
    fn default() -> Self {
        Self::Webp { quality: 90 }
    }
}

impl OutputFormat {
    pub fn extension(&self) -> &str {
        match self {
            OutputFormat::Jpeg { .. } => "jpg",
            OutputFormat::Webp { .. } => "webp",
            OutputFormat::PngOptimized => "png",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            OutputFormat::Jpeg { .. } => "JPEG",
            OutputFormat::Webp { .. } => "WEBP",
            OutputFormat::PngOptimized => "PNG",
        }
    }
}

fn save_jpeg_moz<P: AsRef<Path>>(img: image::RgbImage, path: P, quality: u8) -> anyhow::Result<()> {
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

fn save_webp<P: AsRef<Path>>(img: image::RgbImage, path: P, quality: u8) -> anyhow::Result<()> {
    use webp::Encoder;
    let encoder = Encoder::from_rgb(&img, img.width(), img.height());
    let webp_data = encoder.encode(quality as f32);
    std::fs::write(path, &*webp_data)?;
    Ok(())
}

pub fn save_png<P: AsRef<Path>>(img: &DynamicImage, path: P) -> anyhow::Result<()> {
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
    pub fn save_image<P: AsRef<Path>>(&self, img: &DynamicImage, path: P) -> anyhow::Result<()> {
        match self {
            OutputFormat::Jpeg { quality } => save_jpeg_moz(img.to_rgb8(), path, *quality)?,
            OutputFormat::Webp { quality } => save_webp(img.to_rgb8(), path, *quality)?,
            OutputFormat::PngOptimized => save_png(img, path)?,
        }
        Ok(())
    }

    fn quality(&self) -> Option<u8> {
        match self {
            OutputFormat::Jpeg { quality } => Some(*quality),
            OutputFormat::Webp { quality } => Some(*quality),
            _ => None,
        }
    }

    fn new_q(&self, quality: Option<u8>) -> Self {
        let quality = quality.unwrap_or(90);
        match self {
            OutputFormat::Jpeg { .. } => OutputFormat::Jpeg { quality },
            OutputFormat::Webp { .. } => OutputFormat::Webp { quality },
            default => *default,
        }
    }

    fn has_quality(&self) -> bool {
        matches!(self, OutputFormat::Jpeg { .. } | OutputFormat::Webp { .. })
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        let mut quality = self.quality();

        egui::ComboBox::from_id_salt("export_format_combo")
            .selected_text(self.label())
            .show_ui(ui, |ui| {
                for format in OutputFormat::iter() {
                    ui.selectable_value(self, format.new_q(quality), format.label());
                    let new_quality = self.quality();

                    quality = match (quality.is_some(), new_quality.is_some()) {
                        (_, true) => new_quality,
                        (true, false) => quality,
                        (false, false) => Some(90),
                    };
                }
            });

        if self.has_quality() {
            let mut q = quality.expect("explicted export format handling");
            ui.add(egui::Slider::new(&mut q, 1..=100).text("Quality"));
            *self = self.new_q(Some(q));
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct ExportConfig {
    pub scale_config: crate::scale_config::ScaleConfig,
    pub output_format: OutputFormat,
}

impl std::default::Default for ExportConfig {
    fn default() -> Self {
        Self {
            scale_config: crate::scale_config::SCALE_NEAR_COMMON_4K,
            output_format: OutputFormat::default(),
        }
    }
}

impl ExportConfig {
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Export Configuration");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Scale");
                self.scale_config.update_ui(ui);
            });

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Output format");
                self.output_format.update_ui(ui);
            });
        });
    }

    pub fn extension(&self) -> &str {
        self.output_format.extension()
    }
}

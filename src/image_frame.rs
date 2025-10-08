/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use ab_glyph::{FontArc, PxScale, ScaleFont};
use image::{DynamicImage, Rgba};
use imageproc::drawing::draw_text_mut;

use exif::{In, Reader as ExifReader, Tag};

/// Save image with decorate image with EXIF
pub fn overlay_exif_and_save(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    let ext = input_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    println!("ext : {ext}");
    let mut img = if ext == "heic" || ext == "heif" || ext == "hif" {
        println!("will use libheif");
        let lib = LibHeif::new();
        let ctx = HeifContext::read_from_file(input_path.to_str().unwrap())?;
        let handle = ctx.primary_image_handle()?;
        let decoded = lib.decode(&handle, ColorSpace::Rgb(RgbChroma::Rgba), None)?;
        let inter = decoded
            .planes()
            .interleaved
            .ok_or_else(|| anyhow::anyhow!("Unsupported: no interleaved plane"))?;

        let width = decoded.width() as usize;
        let height = decoded.height() as usize;
        image::RgbaImage::from_raw(width as u32, height as u32, inter.data.to_vec())
            .ok_or_else(|| anyhow::anyhow!("Failed to create image from raw HEIF"))?
    } else {
        image::open(input_path)?.to_rgba8()
    };

    // 텍스트 초기화
    let mut text = String::new();
    if ext == "heic" || ext == "heif" || ext == "hif" {
        let file = std::fs::File::open(input_path)?;

        let mut bufreader = std::io::BufReader::new(file);
        // let exif = ExifReader::new().read_from_container(&mut bufreader)?;
        match ExifReader::new().read_from_container(&mut bufreader) {
            Ok(exif) => {
                let model = exif
                    .get_field(Tag::Model, In::PRIMARY)
                    .map(|f| f.display_value().to_string());
                let fnumber = exif
                    .get_field(Tag::FNumber, In::PRIMARY)
                    .map(|f| f.display_value().to_string());
                let exposure = exif
                    .get_field(Tag::ExposureTime, In::PRIMARY)
                    .map(|f| f.display_value().to_string());
                let iso = exif
                    .get_field(Tag::ISOSpeed, In::PRIMARY)
                    .map(|f| f.display_value().to_string());
                let focal = exif
                    .get_field(Tag::FocalLength, In::PRIMARY)
                    .map(|f| f.display_value().to_string());

                text = format!(
                    "{}\n{} {}
{}
ISO {}",
                    model.unwrap_or_default(),
                    focal.unwrap_or_default(),
                    fnumber.unwrap_or_default(),
                    exposure.unwrap_or_default(),
                    iso.unwrap_or_default(),
                );
            }
            Err(e) => {
                log::error!("Failed to parse EXIF from HEIF: {e:?}");
            }
        }
    } else {
        // 일반 파일에서 EXIF 추출
        let file = std::fs::File::open(input_path)?;
        let mut bufreader = std::io::BufReader::new(file);
        let exif = ExifReader::new().read_from_container(&mut bufreader)?;

        let model = exif
            .get_field(Tag::Model, In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string());
        let fnumber = exif
            .get_field(Tag::FNumber, In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string());
        let exposure = exif
            .get_field(Tag::ExposureTime, In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string());
        let iso = exif
            .get_field(Tag::ISOSpeed, In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string());
        let focal = exif
            .get_field(Tag::FocalLength, In::PRIMARY)
            .map(|f| f.display_value().with_unit(&exif).to_string());

        text = format!(
            "{}\n{} {}
{}
ISO {}",
            model.unwrap_or_default(),
            focal.unwrap_or_default(),
            fnumber.unwrap_or_default(),
            exposure.unwrap_or_default(),
            iso.unwrap_or_default(),
        );
    }

    let font_data = include_bytes!("../assets/fonts/DejaVuSansMono.ttf");
    let font = FontArc::try_from_slice(font_data)?;

    let scale = PxScale::from(100.0);
    let color = Rgba([255, 153, 0, 255]);

    // add text
    let margin = 20;
    let lines: Vec<&str> = text.lines().collect();
    let line_height = 100;
    let total_height = lines.len() * line_height;

    let y_start = img.height() as i32 - total_height as i32 - margin;

    for (i, line) in lines.iter().enumerate() {
        let y = y_start + (i as i32 * line_height as i32);
        draw_text_mut(&mut img, color, margin, y, scale, &font, line);
    }

    // store
    let rgb = DynamicImage::ImageRgba8(img).to_rgb8();
    DynamicImage::ImageRgb8(rgb).save(output_path)?;
    Ok(())
}

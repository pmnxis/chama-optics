/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use crate::{px_h, px_w, pxscale_w};
use ab_glyph::{Font, PxScale, ScaleFont};

pub struct Film {
    // font_size: f32,
}

const FILM_COLOR: image::Rgba<u8> = image::Rgba([255, 153, 0, 255]);

fn text_dimensions(scale: PxScale, font: &impl Font, text: &str) -> (f32, f32) {
    let scaled = font.as_scaled(scale);
    (
        text.chars()
            .map(|c| scaled.h_advance(font.glyph_id(c)))
            .sum::<f32>(),
        scaled.height(),
    )
}

impl Theme for Film {
    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        let exif = &pi.view_exif;
        let scale_config = export_config.scale_config;
        let mut dyn_image = pi.with_scale_and_orientation(scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let font = crate::fonts::FONT_DIGITS.clone();

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $scale:expr, $text:expr) => {
                imageproc::drawing::draw_text_mut(&mut dyn_image, FILM_COLOR, ($xxx) as i32, ($yyy as f32 - font.as_scaled($scale).ascent()) as i32, $scale, &font, $text);
            };
        }

        let margin = px_w!(120, dyn_w).trunc() as i32;

        // Left
        let base_y = (dyn_h as i32 * 13) / 14;

        let cam_scale = pxscale_w!(75, dyn_w);
        draw!(
            margin,
            base_y as f32 - px_h!(75, dyn_h),
            cam_scale,
            &format!("{}  {}", exif.camera_mnf, exif.camera_model)
        );

        draw!(margin, base_y, cam_scale, &exif.lens_model.clone());

        // Right
        let pairs = {
            let mut list = Vec::new();
            if let Some(f) = exif.get_fnumber() {
                list.push(("F", f));
            }
            if let Some(sec) = exif.get_exposure() {
                list.push(("SEC", sec));
            }
            if let Some(iso) = exif.get_iso() {
                list.push(("ISO", iso));
            }
            list
        };

        let prefix_scale = pxscale_w!(65, dyn_w);
        let number_scale = pxscale_w!(100, dyn_w);

        let spacing = px_w!(10, dyn_w);
        let mut y: f32 = base_y as f32;

        for (prefix, number) in pairs.iter().rev() {
            let (prefix_w, prefix_h) = text_dimensions(prefix_scale, &font, prefix);
            let (number_w, number_h) = text_dimensions(number_scale, &font, number);
            let line_h = number_h.max(prefix_h);
            let total_w = prefix_w + spacing + number_w;

            // For right alignment
            let x_right = dyn_w as f32 - margin as f32;
            let x_prefix = (x_right - total_w).round() as i32;
            let x_number = (x_right - number_w).round() as i32;

            draw!(x_prefix, y, prefix_scale, prefix);
            draw!(x_number, y, number_scale, number);

            y -= line_h;
        }

        if let Err(e) = export_config
            .output_format
            .save_image(&dyn_image, output_path)
        {
            println!("{:?}", e);
        }
        Ok(())
    }

    fn ui_config(&mut self, _ui: &mut egui::Ui) {
        unimplemented!()
    }
}

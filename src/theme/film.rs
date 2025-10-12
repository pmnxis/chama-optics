/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use crate::{px_h, px_w, pxscale_w};
use ab_glyph::{Font, PxScale, ScaleFont};
use imageproc::drawing::draw_text_mut;

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

        let margin = px_w!(120, dyn_w).trunc() as i32;

        // Left
        let camera_text = format!("{} {}", exif.camera_mnf, exif.camera_model);
        let lens_text = exif.lens_model.clone();

        let base_y = (dyn_h as i32 * 11) / 12;
        draw_text_mut(
            &mut dyn_image,
            FILM_COLOR,
            margin,
            (base_y as f32 - px_h!(80, dyn_h)).trunc() as i32,
            px_w!(75, dyn_w),
            &font,
            &camera_text,
        );
        draw_text_mut(
            &mut dyn_image,
            FILM_COLOR,
            margin,
            (base_y as f32 + px_h!(20, dyn_h)).trunc() as i32,
            px_w!(75, dyn_w),
            &font,
            &lens_text,
        );

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

            draw_text_mut(
                &mut dyn_image,
                FILM_COLOR,
                x_prefix,
                (y + number_h - prefix_h - px_h!(4.25, dyn_h)).round() as i32,
                prefix_scale,
                &font,
                prefix,
            );

            draw_text_mut(
                &mut dyn_image,
                FILM_COLOR,
                x_number,
                y.round() as i32,
                number_scale,
                &font,
                number,
            );

            y -= line_h;
        }

        dyn_image.save(output_path)
    }

    fn ui_config(&mut self, _ui: &mut egui::Ui) {
        unimplemented!()
    }
}

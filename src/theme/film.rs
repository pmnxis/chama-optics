/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

pub struct Film {}

use crate::theme::Theme;
use ab_glyph::PxScale;
use imageproc::drawing::draw_text_mut;

const FILM_COLOR: image::Rgba<u8> = image::Rgba([255, 153, 0, 255]);

fn merge_option_string(
    x: Option<String>,
    y: Option<String>,
    y_prefix: &str,
    y_postfix: &str,
) -> Option<String> {
    match (x, y) {
        (None, None) => None,
        (Some(xv), None) => Some(xv),
        (None, Some(yv)) => Some(format!("{y_prefix}{yv}{y_postfix}")),
        (Some(xv), Some(yv)) => Some(format!("{xv}\n{y_prefix}{yv}{y_postfix}")),
    }
}

impl Theme for Film {
    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        let exif: &crate::exif_impl::SimplifiedExif = &pi.view_exif;
        let scale_config = export_config.scale_config;
        let left_str = format!(
            "{}  {}\n{}",
            exif.camera_mnf, exif.camera_model, exif.lens_model
        );

        let right_str = {
            let mut ret = exif
                .get_fnumber()
                .map_or(None, |x| Some(format!("F {}", x)));

            ret = merge_option_string(ret, exif.get_exposure(), "", " sec");

            merge_option_string(ret, exif.get_iso(), "ISO ", "")
        };

        let mut dyn_image = pi.with_scale_and_orientation(scale_config)?;

        // todo - asepct ratio font size
        let font = crate::fonts::FONT_DIGITS.clone();
        let scale = PxScale::from(80.0);
        let margin = 60;
        let line_height = 100;

        // split by vec
        let left_lines: Vec<&str> = left_str.lines().collect();
        let left_total_height = left_lines.len() * line_height;
        let right_lines: Vec<&str> = right_str
            .as_ref()
            .map(|s| s.lines().collect::<Vec<_>>())
            .unwrap_or_default();
        let right_total_height = right_lines.len() * line_height;

        // starting y place
        let y_start_left = dyn_image.height() as i32 - left_total_height as i32 - margin as i32;
        let y_start_right = dyn_image.height() as i32 - right_total_height as i32 - margin as i32;

        // left side
        for (i, line) in left_lines.iter().enumerate() {
            let y = y_start_left + (i as i32 * line_height as i32);
            draw_text_mut(&mut dyn_image, FILM_COLOR, margin, y, scale, &font, line);
        }

        // right side
        let img_width = dyn_image.width() as i32;
        for (i, line) in right_lines.iter().enumerate() {
            let text_width_est = (line.len() as i32) * 40;
            let x = img_width - text_width_est - margin as i32;
            let y = y_start_right + (i as i32 * line_height as i32);

            draw_text_mut(&mut dyn_image, FILM_COLOR, x, y, scale, &font, line);
        }

        dyn_image.save(output_path)
    }

    fn ui_config(&mut self, _ui: &mut egui::Ui) {
        unimplemented!()
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use ab_glyph::{Font, PxScale, ScaleFont};
use imageproc::integral_image::ArrayData;
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Film {
    font_color: egui::Color32,
    font_size: f32,
}

const FILM_COLOR: image::Rgba<u8> = image::Rgba([255, 153, 0, 255]);
const DEFAULT_FONT_SIZE: u32 = 25;

fn text_dimensions(scale: PxScale, font: &impl Font, text: &str) -> (f32, f32) {
    let scaled = font.as_scaled(scale);
    (
        text.chars()
            .map(|c| scaled.h_advance(font.glyph_id(c)))
            .sum::<f32>(),
        scaled.height(),
    )
}

impl core::default::Default for Film {
    fn default() -> Self {
        let [r, g, b, a] = FILM_COLOR.data();

        Self {
            font_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
            font_size: DEFAULT_FONT_SIZE as f32,
        }
    }
}

impl Film {
    fn rel_size<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> f32 {
        size.as_() * (self.font_size / (DEFAULT_FONT_SIZE as f32)) * (dyn_wh.as_() / 4000.0)
    }

    fn rel_scale<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> ab_glyph::PxScale {
        ab_glyph::PxScale::from(self.rel_size(size, dyn_wh))
    }
}

impl Theme for Film {
    fn unique_name(&self) -> &'static str {
        "film"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.film")
    }

    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        let exif = &pi.view_exif;
        let color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);
        let scale_config = &export_config.scale_config;
        let mut dyn_image = pi.with_scale_and_orientation(*scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);
        let font = crate::fonts::FONT_DIGITS.clone();

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $scale:expr, $text:expr) => {
                imageproc::drawing::draw_text_mut(&mut dyn_image, color, ($xxx) as i32, ($yyy as f32 - font.as_scaled($scale).ascent()) as i32, $scale, &font, $text);
            };
        }

        let margin = self.rel_size(120, dyn_wh).trunc() as i32;

        // Left
        let base_y = dyn_h as i32 - margin;

        let cam_scale = self.rel_scale(75, dyn_wh);
        draw!(
            margin,
            base_y as f32 - cam_scale.y,
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

        let prefix_scale = self.rel_scale(65, dyn_wh);
        let number_scale = self.rel_scale(100, dyn_wh);
        let spacing = self.rel_size(8.0, dyn_wh);
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

        export_config
            .output_format
            .save_image(&dyn_image, output_path)
    }

    fn ui_config(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.add(egui::Slider::new(&mut self.font_size, 1.0..=100.0).text(t!("theme.font_size")))
                .on_hover_text(t!(
                    "theme.font_size_description",
                    default = DEFAULT_FONT_SIZE
                ));
            ui.add_space(1.0);
            egui::color_picker::color_picker_color32(
                ui,
                &mut self.font_color,
                egui::color_picker::Alpha::Opaque,
            );
        });
    }
}

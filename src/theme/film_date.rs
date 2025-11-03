/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use crate::theme::text_dimensions;
use ab_glyph::{Font, ScaleFont};
use imageproc::integral_image::ArrayData;
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct FilmDate {
    font: crate::FontSelection,
    font_date: crate::FontSelection,
    font_color: egui::Color32,
    glow_color: egui::Color32,
    font_size: f32,
    glow_gain: f32,
    hide_camera_exif: bool,
}

const FILM_COLOR: image::Rgba<u8> = image::Rgba([255, 138, 0, 255]);
const FILM_COLOR_GLOW: image::Rgba<u8> = image::Rgba([238, 140, 128, 255]);
const DEFAULT_FONT_SIZE: u32 = 25;

impl core::default::Default for FilmDate {
    fn default() -> Self {
        let [r, g, b, a] = FILM_COLOR.data();
        let [gr, gg, gb, ga] = FILM_COLOR_GLOW.data();
        Self {
            font: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::Digital7),
            font_date: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::Digital7Italic),
            font_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
            glow_color: egui::Color32::from_rgba_unmultiplied_const(gr, gg, gb, ga),
            font_size: DEFAULT_FONT_SIZE as f32,
            glow_gain: 8.0,
            hide_camera_exif: true,
        }
    }
}

impl FilmDate {
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

impl Theme for FilmDate {
    fn unique_name(&self) -> &'static str {
        "film_date"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.film_date")
    }

    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        let exif = &pi.view_exif;
        let color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);
        let glow_color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.glow_color);
        let scale_config = &export_config.scale_config;
        let mut dyn_image: image::DynamicImage = pi.with_scale_and_orientation(*scale_config)?;
        let mut luma_text = image::GrayImage::new(dyn_image.width(), dyn_image.height());
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);
        let font = crate::FONTS_UNIFY.search(&self.font)?;
        let font_date = crate::FONTS_UNIFY.search(&self.font_date)?;

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $font:expr, $scale:expr, $text:expr) => {
                imageproc::drawing::draw_text_mut(&mut luma_text, image::Luma([255]), ($xxx) as i32, ($yyy as f32 - $font.as_scaled($scale).ascent()) as i32, $scale, $font, $text);
            };
        }

        let margin = self.rel_size(120, dyn_wh).trunc() as i32;
        let base_y = dyn_h as i32 - margin;

        // Left
        let mut y = base_y as f32;
        let cam_scale = self.rel_scale(75, dyn_wh);
        let left_list = {
            let mut list = Vec::new();
            if !(exif.camera_mnf.is_empty() || exif.camera_model.is_empty()) {
                list.push(format!("{}  {}", exif.camera_mnf, exif.camera_model));
            }
            if !exif.lens_model.is_empty() {
                list.push(exif.lens_model.clone());
            }
            list
        };
        for left_str in left_list.iter().rev() {
            draw!(margin, y, &font, cam_scale, left_str);
            y -= cam_scale.y;
        }

        // Right
        let date = {
            use chrono::NaiveDateTime;
            NaiveDateTime::parse_from_str(&exif.datetime, "%Y-%m-%d %H:%M:%S")
                .map(|dt| dt.format("'%y  %m  %d").to_string())
                .unwrap_or_else(|_| "".to_string())
        };

        let datetime_scale = self.rel_scale(105, dyn_wh);
        let y: f32 = base_y as f32;

        let (datetime_w, _) = text_dimensions(datetime_scale, &font_date, &date);

        let x_right = dyn_w as f32 - margin as f32;
        let x_datetime = (x_right - datetime_w).round() as i32;
        draw!(x_datetime, y, &font_date, datetime_scale, &date);

        let rgba_image = dyn_image
            .as_mut_rgba8()
            .ok_or(image::ImageError::Parameter(
                image::error::ParameterError::from_kind(image::error::ParameterErrorKind::Generic(
                    "Mismatch RGBA channel internally, it should not happened".to_owned(),
                )),
            ))?;

        crate::effect::glow::final_glow_effect(
            rgba_image,
            &luma_text,
            color,
            glow_color,
            self.rel_size(self.glow_gain, dyn_wh),
        );

        export_config.save_image(&mut dyn_image, Some(margin), output_path)
    }

    fn ui_config(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.checkbox(
            &mut self.hide_camera_exif,
            t!("theme.film_config.hide_camera_exif.label"),
        )
        .on_hover_text(t!("theme.film_config.hide_camera_exif.description"));

        self.font.update_ui_with_default_label(ctx, ui);

        self.font_date
            .update_ui_with_label(ctx, ui, t!("theme.film_date_config.date_font_select"));

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.font_size, 1.0..=100.0).text(t!("theme.font_size")),
                )
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
            ui.vertical(|ui| {
                ui.add(
                    egui::Slider::new(&mut self.glow_gain, 1.0..=30.0)
                        .text(t!("theme.film_config.glow_range")),
                );
                ui.add_space(1.0);
                egui::color_picker::color_picker_color32(
                    ui,
                    &mut self.font_color,
                    egui::color_picker::Alpha::Opaque,
                );
            });
        });
    }
}

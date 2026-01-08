/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use crate::update_param;
use ab_glyph::{Font, ScaleFont};
use imageproc::integral_image::ArrayData;
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize, chama_optics_macros::ThemeParameters)]
pub struct FilmDate {
    pub font: crate::FontSelection,
    pub font_date: crate::FontSelection,

    #[param(color, label_key = "theme.font_color", default = "rgb(255, 138, 0)")]
    pub font_color: egui::Color32,

    #[param(color, label_key = "theme.glow_color", default = "rgb(238, 140, 128)")]
    pub glow_color: egui::Color32,

    #[param(
        slider,
        label_key = "theme.font_size",
        min = 10,
        max = 100,
        default_const = "DEFAULT_FONT_SIZE"
    )]
    pub font_size: u32,

    #[param(
        slider,
        label_key = "theme.glow_gain",
        min = 0,
        max = 20,
        default_const = "DEFAULT_GLOW_GAIN"
    )]
    pub glow_gain: u32,

    pub hide_camera_exif: bool,
    pub show_ps: bool,
}

const FILM_COLOR: image::Rgba<u8> = image::Rgba([255, 138, 0, 255]);
const FILM_COLOR_GLOW: image::Rgba<u8> = image::Rgba([238, 140, 128, 255]);
const DEFAULT_FONT_SIZE: u32 = 25;
const DEFAULT_GLOW_GAIN: u32 = 8;

impl core::default::Default for FilmDate {
    fn default() -> Self {
        let [r, g, b, a] = FILM_COLOR.data();
        let [gr, gg, gb, ga] = FILM_COLOR_GLOW.data();
        Self {
            font: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::Digital7),
            font_date: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::Digital7Italic),
            font_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
            glow_color: egui::Color32::from_rgba_unmultiplied_const(gr, gg, gb, ga),
            font_size: DEFAULT_FONT_SIZE,
            glow_gain: DEFAULT_GLOW_GAIN,
            hide_camera_exif: true,
            show_ps: false,
        }
    }
}

impl FilmDate {
    fn rel_size<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> f32 {
        size.as_() * (self.font_size as f32 / (DEFAULT_FONT_SIZE as f32)) * (dyn_wh.as_() / 4000.0)
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

    fn apply_to_image(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
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
                crate::theme::draw_text_with_fallback_luma(&mut luma_text, image::Luma([255]), ($xxx) as i32, ($yyy as f32 - $font.as_scaled($scale).ascent()) as i32, $scale, $font, 400, $text);
            };
        }

        let margin = self.rel_size(120, dyn_wh).trunc() as i32;
        let base_y = dyn_h as i32 - margin;

        // Left
        let mut y = base_y as f32;
        let cam_scale = self.rel_scale(75, dyn_wh);
        let left_list = {
            let mut list = Vec::new();
            if !self.hide_camera_exif {
                if !(exif.camera_mnf.is_empty() || exif.camera_model.is_empty()) {
                    list.push(format!("{}  {}", exif.camera_mnf, exif.camera_model));
                }
                if !exif.lens_model.is_empty() {
                    list.push(exif.lens_model.clone());
                }
            }

            if let Some(ps_main) = exif.get_ps_main()
                && self.show_ps
            {
                list.push(if let Some(ps_sub) = exif.get_lut_detail() {
                    if !ps_sub.is_empty() {
                        format!("{ps_main} = {ps_sub}")
                    } else {
                        ps_main
                    }
                } else {
                    ps_main
                })
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

        let (datetime_w, _) =
            crate::theme::text_dimensions_with_fallback(datetime_scale, &font_date, 400, &date);

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
            self.rel_size(self.glow_gain as f32, dyn_wh),
        );

        Ok(dyn_image)
    }

    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        let scale_config = &export_config.scale_config;
        let dyn_image = pi.with_scale_and_orientation(*scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);

        let margin = self.rel_size(120, dyn_wh).trunc() as i32;

        let mut themed_image = self.apply_to_image(pi, export_config)?;
        export_config.save_image(&mut themed_image, Some(margin), output_path)
    }

    fn ui_config(&mut self, ui: &mut egui::Ui) {
        self.auto_ui_config(ui);

        // Custom UI for font selection and checkboxes
        ui.vertical(|ui| {
            ui.checkbox(
                &mut self.hide_camera_exif,
                t!("theme.film_config.hide_camera_exif.label"),
            )
            .on_hover_text(t!("theme.film_config.hide_camera_exif.description"));

            ui.add_space(1.0);
            ui.checkbox(
                &mut self.show_ps,
                t!("theme.film_config.show_photo_style.label"),
            )
            .on_hover_text(t!("theme.film_config.show_photo_style.description"));

            self.font.update_ui_with_default_label(ui);

            self.font_date
                .update_ui_with_label(ui, t!("theme.date_config.date_font_select"));
        });
    }

    fn is_ui_config_available(&self) -> bool {
        true
    }

    fn get_parameters_json(&self) -> String {
        self.auto_get_parameters_json()
    }
}

// ThemeParameters implementation is now auto-generated by #[derive(ThemeParameters)]

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use crate::update_param;
use ab_glyph::{Font, ScaleFont};
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize, chama_optics_macros::ThemeParameters)]
pub struct Film {
    pub font: crate::FontSelection,

    #[param(color, label_key = "theme.font_color", default = "rgb(255, 153, 0)")]
    pub font_color: egui::Color32,

    #[param(
        slider,
        label_key = "theme.font_size",
        min = 10,
        max = 100,
        default_const = "DEFAULT_FONT_SIZE"
    )]
    pub font_size: u32,

    pub show_ps: bool,
}

const FILM_COLOR: image::Rgba<u8> = image::Rgba([255, 153, 0, 255]);
const DEFAULT_FONT_SIZE: u32 = 25;

impl core::default::Default for Film {
    fn default() -> Self {
        use imageproc::integral_image::ArrayData;
        let [r, g, b, a] = FILM_COLOR.data();

        Self {
            font: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::Digital7),
            font_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
            font_size: DEFAULT_FONT_SIZE,
            show_ps: false,
        }
    }
}

impl Film {
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

impl Theme for Film {
    fn unique_name(&self) -> &'static str {
        "film"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.film")
    }

    fn apply_to_image(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        let exif = &pi.view_exif;
        let color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);
        let scale_config = &export_config.scale_config;
        let mut dyn_image = pi.with_scale_and_orientation(*scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);
        let font = crate::FONTS_UNIFY.search(&self.font)?;

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $scale:expr, $text:expr) => {
                crate::theme::draw_text_with_fallback(&mut dyn_image, color, ($xxx) as i32, ($yyy as f32 - font.as_scaled($scale).ascent()) as i32, $scale, &font, 400, $text);
            };
        }

        let margin = self.rel_size(120, dyn_wh).trunc() as i32;
        let base_y = dyn_h as i32 - margin;

        // Left
        let mut y = base_y as f32;
        let cam_scale = self.rel_scale(75, dyn_wh);
        let left_list = {
            let mut list = Vec::new();

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

            if !(exif.camera_mnf.is_empty() || exif.camera_model.is_empty()) {
                list.push(format!("{}  {}", exif.camera_mnf, exif.camera_model));
            }
            if !exif.lens_model.is_empty() {
                list.push(exif.lens_model.clone());
            }
            list
        };
        for left_str in left_list.iter().rev() {
            draw!(margin, y, cam_scale, left_str);
            y -= cam_scale.y;
        }

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
            let (prefix_w, prefix_h) =
                crate::theme::text_dimensions_with_fallback(prefix_scale, &font, 400, prefix);
            let (number_w, number_h) =
                crate::theme::text_dimensions_with_fallback(number_scale, &font, 400, number);
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

        // Custom UI for font selection and show_ps checkbox
        ui.vertical(|ui| {
            self.font.update_ui_with_default_label(ui);

            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.show_ps,
                    t!("theme.film_config.show_photo_style.label"),
                )
                .on_hover_text(t!("theme.film_config.show_photo_style.description"));
            });
        });
    }

    fn is_ui_config_available(&self) -> bool {
        true
    }

    fn get_parameters_json(&self) -> String {
        self.auto_get_parameters_json()
    }
}

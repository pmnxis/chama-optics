/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::effect::variable_text::{VariableTextSlot, VariableTextSlotDefault};
use crate::theme::Theme;
use crate::update_param;
use ab_glyph::{Font, ScaleFont};
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize, chama_optics_macros::ThemeParameters)]
pub struct OneLine {
    #[param(
        border,
        label_key = "theme.border",
        default_border = "DEFAULT_BORDER",
        default_limit = "DEFAULT_LIMIT"
    )]
    pub border: crate::effect::border::Border,

    #[param(color, label_key = "theme.font_color", default = "BLACK")]
    pub font_color: egui::Color32,

    #[param(
        slider,
        label_key = "theme.font_height_ratio.label",
        hint_key = "theme.font_height_ratio.hint",
        min = 5,
        max = 80,
        default_const = "DEFAULT_FONT_HEIGHT"
    )]
    pub font_height: u32,

    #[param(
        slider,
        label_key = "theme.font_height_ratio.label",
        hint_key = "theme.font_height_ratio.hint",
        min = 5,
        max = 90,
        default_const = "DEFAULT_TOP_FONT_HEIGHT"
    )]
    pub top_font_height: u32,

    #[param(
        text,
        label_key = "theme.exif_left_bot",
        hint_key = "theme.template_format_hint.description",
        default_const = "DEFAULT_LEFT"
    )]
    pub left: VariableTextSlot,

    #[param(
        text,
        label_key = "theme.exif_right_bot",
        hint_key = "theme.template_format_hint.description",
        default_const = "DEFAULT_RIGHT"
    )]
    pub right: VariableTextSlot,

    #[param(
        text,
        label_key = "theme.exif_center_top",
        hint_key = "theme.template_format_hint.description",
        default_const = "DEFAULT_TOP"
    )]
    pub top: VariableTextSlot,

    pub show_hint: bool,
}

const DEFAULT_FONT_HEIGHT: u32 = 30;
const DEFAULT_TOP_FONT_HEIGHT: u32 = 50;

const DEFAULT_LEFT: VariableTextSlotDefault = VariableTextSlotDefault::with_digital7(
    "[{camera_mnf}  •  ][{camera_model}][  •  {lens_model}]",
);

const DEFAULT_RIGHT: VariableTextSlotDefault = VariableTextSlotDefault::with_digital7(
    "[ISO{iso_speed}  •  ][{focal}mm  •  ][F{fnumber}  •  ][{exposure}s]",
);

const DEFAULT_TOP: VariableTextSlotDefault = VariableTextSlotDefault::with_barlow_weight("", 300);

// const DEFAULT_BORDER_DEFAULT_SIZE: u32 = 90;
const DEFAULT_BORDER_MIN_SIZE: u32 = 10;
const DEFAULT_BORDER_OTHER_SIZE: u32 = 55;
const DEFAULT_BORDER_BOTTOM_SIZE: u32 = 135;
const DEFAULT_LIMIT: crate::effect::border::BorderLimit =
    crate::effect::border::BorderLimit::top_and_bottom(DEFAULT_BORDER_MIN_SIZE, 900);

const DEFAULT_BORDER: crate::effect::border::Border = crate::effect::border::Border {
    left: DEFAULT_BORDER_OTHER_SIZE,
    right: DEFAULT_BORDER_OTHER_SIZE,
    top: DEFAULT_BORDER_OTHER_SIZE,
    bottom: DEFAULT_BORDER_BOTTOM_SIZE,
    color: egui::Color32::WHITE,
    is_relative: true,
};

impl core::default::Default for OneLine {
    fn default() -> Self {
        Self {
            border: DEFAULT_BORDER,
            font_color: egui::Color32::BLACK,
            font_height: DEFAULT_FONT_HEIGHT,
            top_font_height: DEFAULT_TOP_FONT_HEIGHT,
            left: (&DEFAULT_LEFT).into(),
            right: (&DEFAULT_RIGHT).into(),
            top: (&DEFAULT_TOP).into(),
            show_hint: false,
        }
    }
}

impl OneLine {
    // 0.0~1.0
    fn rel_size<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> f32 {
        size.as_() * (dyn_wh.as_())
    }

    fn rel_scale<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> ab_glyph::PxScale {
        ab_glyph::PxScale::from(self.rel_size(size, dyn_wh))
    }
}

impl Theme for OneLine {
    fn unique_name(&self) -> &'static str {
        "one_line"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.one_line.title")
    }

    fn apply_to_dynamic_image(
        &self,
        dyn_image: image::DynamicImage,
        exif: &crate::image::exif_impl::SimplifiedExif,
        _export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        let font_color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);

        let (ll, rr, tt, bb) = self.border.border_size(dyn_wh);
        // let font_height_ratio = self.font_height.clamp(5, 80) as f32 / 100.0;
        let font_height_ratio_x100 = (self.font_height).clamp(5, 800);

        let mut new_image = self.border.take_from_exist(&dyn_image, dyn_wh)?;

        // TODO - Need more profer way
        let y = new_image.height() - (bb / 2);

        // left and right first
        let left_x = ((bb / 4).max(2) + ll) as i32;
        let right_x_end = (new_image.width() - rr - (bb / 4).max(2)) as i32;
        let available = right_x_end - left_x;
        let left_font = &self.left.get_font();
        let left_txt = self.left.format_custom(exif);
        let right_font = &self.right.get_font();
        let right_txt = self.right.format_custom(exif);

        if available < 1 {
            panic!("unreachable");
        }

        let txt_scale = {
            let mut ret = self.rel_scale(0.05, bb);
            let left_txt = format!("{left_txt}  "); // for margin

            for ratio_x100 in (5..=font_height_ratio_x100).rev() {
                let font_height_ratio = ratio_x100 as f32 / 100.0;
                let try_scale = self.rel_scale(font_height_ratio, bb);

                let (left_www, _) = crate::theme::text_dimensions_with_fallback(
                    try_scale,
                    left_font,
                    self.left.weight,
                    &left_txt,
                );
                let (right_www, _) = crate::theme::text_dimensions_with_fallback(
                    try_scale,
                    right_font,
                    self.right.weight,
                    &right_txt,
                );

                if (left_www + right_www).floor() as i32 <= available {
                    ret = try_scale;
                    break;
                }
            }
            ret
        };

        // left - after
        crate::theme::draw_text_with_fallback(
            &mut new_image,
            font_color,
            left_x,
            (y as f32
                - ((left_font.as_scaled(txt_scale).ascent()
                    + left_font.as_scaled(txt_scale).descent().abs())
                    * 0.55)) as i32,
            txt_scale,
            left_font,
            self.left.weight,
            &left_txt,
        );

        // right - after
        let (right_www, _) = crate::theme::text_dimensions_with_fallback(
            txt_scale,
            right_font,
            self.right.weight,
            &right_txt,
        );
        let right_x = (new_image.width() - rr - (bb / 4).max(2)) as i32 - (right_www as i32);
        crate::theme::draw_text_with_fallback(
            &mut new_image,
            font_color,
            right_x,
            (y as f32
                - ((right_font.as_scaled(txt_scale).ascent()
                    + right_font.as_scaled(txt_scale).descent().abs())
                    * 0.55)) as i32,
            txt_scale,
            right_font,
            self.right.weight,
            &right_txt,
        );

        // top
        if tt >= 10 {
            let y = tt / 2;
            let top_font = &self.top.get_font();
            let top_txt = self.top.format_custom(exif);
            let top_scale = self.rel_scale(self.top_font_height as f32 / 100.0, tt);
            let (top_www, _) = crate::theme::text_dimensions_with_fallback(
                top_scale,
                top_font,
                self.top.weight,
                &top_txt,
            );

            let top_x = (((dyn_w as f32 - top_www) / 2.0) + ll as f32).max(0.0) as i32;

            crate::theme::draw_text_with_fallback(
                &mut new_image,
                font_color,
                top_x,
                (y as f32
                    - ((top_font.as_scaled(top_scale).ascent()
                        + top_font.as_scaled(top_scale).descent().abs())
                        * 0.5)) as i32,
                top_scale,
                top_font,
                self.top.weight,
                &top_txt,
            );
        } else {
            log::warn!("Cannot create top title with {tt} pixel margin");
        }

        Ok(new_image)
    }

    fn apply_to_image(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        let scale_config = &export_config.scale_config;
        let dyn_image = pi.with_scale_and_orientation(*scale_config)?;
        self.apply_to_dynamic_image(dyn_image, &pi.view_exif, export_config)
    }

    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        let scale_config = &export_config.scale_config;
        let dyn_image: image::DynamicImage = pi.with_scale_and_orientation(*scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);

        let (_ll, _rr, _tt, bb) = self.border.border_size(dyn_wh);
        let temp_margin = self.border.interactive_watermark_padding(dyn_w, dyn_h) / 6;
        let border_margin = (temp_margin * 5).max(bb) + temp_margin;

        let mut themed_image = self.apply_to_image(pi, export_config)?;
        export_config.save_image(&mut themed_image, Some(border_margin as i32), output_path)
    }

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    fn ui_config(&mut self, ui: &mut egui::Ui) {
        self.auto_ui_config(ui);

        // Custom hint toggle UI
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(t!("theme.template_format_hint.title"));
                if ui.button("?").clicked() {
                    self.show_hint = !self.show_hint;
                }
            });

            if self.show_hint {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(t!("theme.template_format_hint.description"));
                });
            }
        });
    }

    fn is_ui_config_available(&self) -> bool {
        true
    }

    fn get_parameters_json(&self) -> String {
        self.auto_get_parameters_json()
    }
}

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
pub struct TwoLine {
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
        max = 39,
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

    pub bottom_align: crate::fonts::align::TextAlign,

    #[param(
        text,
        label_key = "theme.bottom1",
        hint_key = "theme.template_format_hint.description",
        default_const = "DEFAULT_FIRST"
    )]
    pub first: VariableTextSlot,

    #[param(
        text,
        label_key = "theme.bottom2",
        hint_key = "theme.template_format_hint.description",
        default_const = "DEFAULT_SECOND"
    )]
    pub second: VariableTextSlot,

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

const DEFAULT_FIRST: VariableTextSlotDefault =
    VariableTextSlotDefault::with_barlow("[{camera_mnf}  ·  ][{camera_model}][  ·  {lens_model}]");

const DEFAULT_SECOND: VariableTextSlotDefault = VariableTextSlotDefault::with_barlow(
    "[ISO{iso_speed}  ·  ][{focal}mm  ·  ][F{fnumber}  ·  ][{exposure}s]",
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

impl core::default::Default for TwoLine {
    fn default() -> Self {
        Self {
            border: DEFAULT_BORDER,
            font_color: egui::Color32::BLACK,
            font_height: DEFAULT_FONT_HEIGHT,
            top_font_height: DEFAULT_TOP_FONT_HEIGHT,
            bottom_align: crate::fonts::align::TextAlign::Center,
            first: (&DEFAULT_FIRST).into(),
            second: (&DEFAULT_SECOND).into(),
            top: (&DEFAULT_TOP).into(),
            // width_aligned: true,
            show_hint: false,
        }
    }
}

impl TwoLine {
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

impl Theme for TwoLine {
    fn unique_name(&self) -> &'static str {
        "two_line"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.two_line.title")
    }

    fn apply_to_image(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        let scale_config = &export_config.scale_config;
        let font_color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);
        let dyn_image: image::DynamicImage = pi.with_scale_and_orientation(*scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);

        let (ll, rr, tt, bb) = self.border.border_size(dyn_wh);
        let txt_scale = self.rel_scale((self.font_height).clamp(5, 39) as f32 / 100.0, bb);

        let mut new_image = self.border.take_from_exist(&dyn_image, dyn_wh);

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $font:expr, $scale:expr, $weight:expr, $text:expr) => {
                crate::theme::draw_text_with_fallback(&mut new_image, font_color, ($xxx) as i32, ($yyy as f32 - $font.as_scaled($scale).ascent()) as i32, $scale, $font, $weight, $text);
            };
        }
        // let two_line_size = font.as_scaled(txt_scale).ascent().abs() * 2.0;
        let ff = self.first.get_font();
        let sf = self.second.get_font();

        let two_line_size = ff.as_scaled(txt_scale).ascent() + sf.as_scaled(txt_scale).ascent();
        // + font.as_scaled(txt_scale).descent().abs() * 1.0;

        // TODO - Need more profer way
        let txt_b_gap = (bb as f32 - two_line_size) / 2.0;
        let txt_y_base = new_image.height() as f32 - txt_b_gap;

        let gap_x = (bb / 4).max(2).min(ll.min(rr)) as i32;

        let mut y = txt_y_base;

        for item in [&self.first, &self.second].iter().rev() {
            let txt = item.format_custom(&pi.view_exif);
            let (www, _hhh) = item.text_dimensions(txt_scale, &txt);
            let new_bottom_x = self.bottom_align.x_point(ll, dyn_w, gap_x, www as i32);

            draw!(
                new_bottom_x,
                y,
                &item.get_font(),
                txt_scale,
                item.weight,
                &txt
            );
            y -= txt_scale.y;
        }

        // top
        if tt >= 10 {
            let y = tt / 2;
            let top_font = &self.top.get_font();
            let top_txt = self.top.format_custom(&pi.view_exif);
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

    #[cfg(not(feature = "ios_integration"))]
    fn ui_config(&mut self, ui: &mut egui::Ui) {
        self.auto_ui_config(ui);

        // Custom bottom_align UI
        ui.vertical(|ui| {
            ui.add_space(4.0);
            egui::Grid::new("two_line_custom_grid")
                .num_columns(2)
                .spacing([4.0, 3.0])
                .show(ui, |ui| {
                    ui.label(t!("text_align.bottom_text_align"));
                    self.bottom_align.update_ui(ui);
                    ui.end_row();
                });

            // Custom hint toggle UI
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

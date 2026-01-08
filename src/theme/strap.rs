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
pub struct Strap {
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
        label_key = "theme.logo_height",
        min = 10,
        max = 90,
        default_const = "DEFAULT_LOGO_HEIGHT"
    )]
    pub logo_height: u32,

    #[param(
        text,
        label_key = "theme.exif_left_top",
        hint_key = "theme.template_format_hint.description",
        default_const = "DEFAULT_LEFT_TOP"
    )]
    pub left_top: VariableTextSlot,

    #[param(
        text,
        label_key = "theme.exif_left_bot",
        hint_key = "theme.template_format_hint.description",
        default_const = "DEFAULT_LEFT_BOT"
    )]
    pub left_bot: VariableTextSlot,

    #[param(
        text,
        label_key = "theme.exif_right_top",
        hint_key = "theme.template_format_hint.description",
        default_const = "DEFAULT_RIGHT_TOP"
    )]
    pub right_top: VariableTextSlot,

    #[param(
        text,
        label_key = "theme.exif_right_bot",
        hint_key = "theme.template_format_hint.description",
        default_const = "DEFAULT_RIGHT_BOT"
    )]
    pub right_bot: VariableTextSlot,

    pub show_hint: bool,
}

const DEFAULT_LEFT_TOP: VariableTextSlotDefault =
    VariableTextSlotDefault::with_barlow("[ISO{iso_speed}] [{focal}mm] [F{fnumber}] [{exposure}s]");
const DEFAULT_LEFT_BOT: VariableTextSlotDefault =
    VariableTextSlotDefault::with_barlow("{datetime}");
const DEFAULT_RIGHT_TOP: VariableTextSlotDefault =
    VariableTextSlotDefault::with_barlow("{camera_mnf} {camera_model}");
const DEFAULT_RIGHT_BOT: VariableTextSlotDefault =
    VariableTextSlotDefault::with_barlow("{lens_mnf} {lens_model}");

const DEFAULT_BORDER_DEFAULT_SIZE: u32 = 120;
const DEFAULT_BORDER_MIN_SIZE: u32 = 60;
const DEFAULT_LOGO_HEIGHT: u32 = 75;
const DEFAULT_LIMIT: crate::effect::border::BorderLimit =
    crate::effect::border::BorderLimit::bottom(DEFAULT_BORDER_MIN_SIZE, 900);
const DEFAULT_BORDER: crate::effect::border::Border =
    crate::effect::border::Border::bottom(DEFAULT_BORDER_DEFAULT_SIZE, egui::Color32::WHITE);
// const DEFAULT_FONT_SIZE: u32 = 25;

impl core::default::Default for Strap {
    fn default() -> Self {
        Self {
            border: DEFAULT_BORDER,
            font_color: egui::Color32::BLACK,
            logo_height: DEFAULT_LOGO_HEIGHT,
            left_top: (&DEFAULT_LEFT_TOP).into(),
            left_bot: (&DEFAULT_LEFT_BOT).into(),
            right_top: (&DEFAULT_RIGHT_TOP).into(),
            right_bot: (&DEFAULT_RIGHT_BOT).into(),
            show_hint: false,
        }
    }
}

impl Strap {
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

impl Theme for Strap {
    fn unique_name(&self) -> &'static str {
        "strap"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.strap.title")
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
        let font =
            &crate::fonts::variable_font::BuiltinVariableFontIndex::Barlow.get_font_by_weight(800);
        // let font = &crate::fonts::FONT_PACK_BARLOW.font[3];
        let mut is_overflow = false;

        let (ll, rr, _tt, bb) = self.border.border_size(dyn_wh);
        let txt_scale = self.rel_scale(0.385, bb);
        let mut new_image = self.border.take_from_exist(&dyn_image, dyn_wh);

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $font:expr, $scale:expr, $weight:expr, $text:expr) => {
                crate::theme::draw_text_with_fallback(&mut new_image, font_color, ($xxx) as i32, ($yyy as f32 - $font.as_scaled($scale).ascent()) as i32, $scale, $font, $weight, $text);
            };
        }
        let two_line_size = font.as_scaled(txt_scale).ascent().abs() * 2.0;
        // + font.as_scaled(txt_scale).descent().abs() * 1.0;

        // TODO - Need more profer way
        let txt_b_gap = (bb as f32 - two_line_size) / 2.0;
        let txt_y_base = new_image.height() as f32 - txt_b_gap;

        // left
        let mut y = txt_y_base;
        let left_x = txt_b_gap * 1.2 + ll as f32;
        let mut max_left_x = left_x;

        for item in [&self.left_top, &self.left_bot].iter().rev() {
            let txt = item.format_custom(&pi.view_exif);
            let (www, _hhh) = crate::theme::text_dimensions_with_fallback(
                txt_scale,
                &item.get_font(),
                item.weight,
                &txt,
            );

            draw!(left_x, y, &item.get_font(), txt_scale, item.weight, &txt);

            y -= txt_scale.y;
            max_left_x = max_left_x.max(www + left_x);
        }

        // right

        let mut y = txt_y_base;
        let right_x = new_image.width() as f32 - txt_b_gap * 1.2 - rr as f32;
        let mut min_right_x = right_x;
        for item in [&self.right_top, &self.right_bot].iter().rev() {
            let txt = item.format_custom(&pi.view_exif);
            let (www, _hhh) = crate::theme::text_dimensions_with_fallback(
                txt_scale,
                &item.get_font(),
                item.weight,
                &txt,
            );
            let new_right_x = right_x - www;

            draw!(
                new_right_x,
                y,
                &item.get_font(),
                txt_scale,
                item.weight,
                &txt
            );
            y -= txt_scale.y;
            min_right_x = min_right_x.min(new_right_x);
        }

        // temporary implementation
        if let Some(svg) = crate::ART_UNIFY.get_camera_logo(&pi.view_exif) {
            use image::GenericImageView;

            // | Left Str1                                Right Str1 |
            // | Left Str2                                Right Str2 |
            // |           <----- available width ------>            |

            let logo_height_ratio = self.logo_height.clamp(10, 75) as f32 / 100.0;
            let avail_width = min_right_x - max_left_x;
            let avail_height = self.rel_size(logo_height_ratio, bb);
            let logo = svg.draw_fit(avail_width.trunc() as u32, avail_height.trunc() as u32)?;

            let logo_x = (min_right_x - (txt_b_gap * 2.0)) as i32 - logo.width() as i32;
            let logo_y = (new_image.dimensions().1 as i32) - ((bb + logo.height()) / 2) as i32;
            // (new_image.dimensions().1 as i32) - (bb as i32) + self.rel_size(0.125, bb) as i32;

            // resolve overflow issue
            let logo_x = if logo_x < 0 {
                log::error!("export image is too small, logo_x {logo_x}");
                is_overflow = true;
                0
            } else {
                logo_x
            };

            let logo_y = if logo_y < 0 {
                log::error!("export image is too small, logo_y {logo_y}");
                is_overflow = true;
                0
            } else {
                logo_y
            };

            if is_overflow {
                log::error!("{dyn_w} x {dyn_h}");
                log::error!(
                    "logo_height_ratio : {logo_height_ratio}, min_right_x : {min_right_x}, txt_b_gap : {txt_b_gap}, logo : {} x {}",
                    logo.width(),
                    logo.height()
                );
            }

            crate::effect::draw_with_transparency::overlay_alpha_screen_mode(
                &mut new_image,
                &logo,
                logo_x as u32,
                logo_y as u32,
            );
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

// ThemeParameters implementation is now auto-generated by #[derive(ThemeParameters)]

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use ab_glyph::{Font, ScaleFont};
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Strap {
    border: crate::effect::border::Border,
    pub font_color: egui::Color32,
    pub exif_left_top: String,
    pub exif_left_bot: String,
    pub exif_right_top: String,
    pub exif_right_bot: String,
    show_hint: bool,
}

const DEFAULT_BORDER_MIN_SIZE: u32 = 120;
const DEFAULT_LIMIT: crate::effect::border::BorderLimit =
    crate::effect::border::BorderLimit::bottom(DEFAULT_BORDER_MIN_SIZE, 900);
const DEFAULT_BORDER: crate::effect::border::Border =
    crate::effect::border::Border::bottom(DEFAULT_BORDER_MIN_SIZE, egui::Color32::WHITE);
// const DEFAULT_FONT_SIZE: u32 = 25;

impl core::default::Default for Strap {
    fn default() -> Self {
        Self {
            border: DEFAULT_BORDER,
            font_color: egui::Color32::BLACK,
            exif_left_top: "[ISO{iso_speed}] [{focal}mm] [F{fnumber}] [{exposure}s]".to_owned(),
            exif_left_bot: "{datetime}".to_owned(),
            exif_right_top: "{camera_mnf} {camera_model}".to_owned(),
            exif_right_bot: "{lens_mnf} {lens_model}".to_owned(),
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

    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        let scale_config = &export_config.scale_config;
        let font_color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);
        let dyn_image: image::DynamicImage = pi.with_scale_and_orientation(*scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);
        let font = &crate::fonts::FONT_PACK_BARLOW.font[3];

        let (ll, rr, _tt, bb) = self.border.border_size(dyn_wh);
        let txt_scale = self.rel_scale(0.385, bb);
        let mut new_image = self.border.take_from_exist(&dyn_image, dyn_wh);

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $scale:expr, $text:expr) => {
                imageproc::drawing::draw_text_mut(&mut new_image, font_color, ($xxx) as i32, ($yyy as f32 - font.as_scaled($scale).ascent()) as i32, $scale, &font, $text);
            };
        }
        let two_line_size = font.as_scaled(txt_scale).ascent().abs() * 2.0;
        // + font.as_scaled(txt_scale).descent().abs() * 1.0;

        // TODO - Need more profer way
        let txt_b_gap = (bb as f32 - two_line_size) / 2.0;
        let txt_y_base = new_image.height() as f32 - txt_b_gap;

        // left
        let left_top = pi.view_exif.format_custom(&self.exif_left_top);
        let left_bot = pi.view_exif.format_custom(&self.exif_left_bot);

        let mut y = txt_y_base;
        let left_x = txt_b_gap * 1.2 + ll as f32;

        for left_str in [left_top, left_bot].iter().rev() {
            draw!(left_x, y, txt_scale, left_str);
            y -= txt_scale.y;
        }

        // right
        let right_top = pi.view_exif.format_custom(&self.exif_right_top);
        let right_bot = pi.view_exif.format_custom(&self.exif_right_bot);

        let mut y = txt_y_base;
        let right_x = new_image.width() as f32 - txt_b_gap * 1.2 - rr as f32;
        let mut min_right_x = right_x;
        for right_str in [right_top, right_bot].iter().rev() {
            let (www, _hhh) = crate::theme::text_dimensions(txt_scale, &font, right_str);
            let new_right_x = right_x - www;
            draw!(new_right_x, y, txt_scale, right_str);
            y -= txt_scale.y;
            min_right_x = min_right_x.min(new_right_x);
        }

        // temporary implementation
        if let Some(svg) = crate::ART_UNIFY.get_camera_logo(&pi.view_exif) {
            use image::GenericImageView;

            let svg_rel_size = self.rel_size(0.75, bb);
            let logo = svg.draw(svg_rel_size as u32).unwrap();
            let logo_x = (min_right_x - (txt_b_gap * 2.0)) as u32 - logo.width();
            let logo_y = new_image.dimensions().1 - bb + self.rel_size(0.125, bb) as u32;

            crate::effect::draw_with_transparency::overlay_alpha_screen_mode(
                &mut new_image,
                &logo,
                logo_x,
                logo_y,
            );
        }

        export_config.save_image(
            &mut new_image,
            Some(self.border.interactive_watermark_padding(dyn_w, dyn_h) as i32),
            output_path,
        )
    }

    fn ui_config(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        self.border.ui_config(ui, &DEFAULT_BORDER, &DEFAULT_LIMIT);

        ui.vertical(|ui| {
            // Padding configuration
            ui.add_space(4.0);

            // Own configuration
            egui::Grid::new("strap_config_grid")
                .num_columns(2)
                .spacing([4.0, 3.0])
                .show(ui, |ui| {
                    ui.label(t!("theme.strap_config.font_color"));
                    egui::widgets::color_picker::color_edit_button_srgba(
                        ui,
                        &mut self.font_color,
                        egui::color_picker::Alpha::Opaque,
                    );
                    ui.end_row();

                    ui.label(t!("theme.strap_config.exif_left_top"));
                    ui.add_sized(
                        [ui.available_width(), 23.0],
                        egui::TextEdit::singleline(&mut self.exif_left_top)
                            .vertical_align(egui::Align::Center),
                    );
                    ui.end_row();

                    ui.label(t!("theme.strap_config.exif_left_bot"));
                    ui.add_sized(
                        [ui.available_width(), 23.0],
                        egui::TextEdit::singleline(&mut self.exif_left_bot)
                            .vertical_align(egui::Align::Center),
                    );
                    ui.end_row();

                    ui.label(t!("theme.strap_config.exif_right_top"));
                    ui.add_sized(
                        [ui.available_width(), 23.0],
                        egui::TextEdit::singleline(&mut self.exif_right_top)
                            .vertical_align(egui::Align::Center),
                    );
                    ui.end_row();

                    ui.label(t!("theme.strap_config.exif_right_bot"));
                    ui.add_sized(
                        [ui.available_width(), 23.0],
                        egui::TextEdit::singleline(&mut self.exif_right_bot)
                            .vertical_align(egui::Align::Center),
                    );
                    ui.end_row();
                });

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
}

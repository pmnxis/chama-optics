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
    pub padding: u32,
    pub color: egui::Color32,
    font: crate::FontSelection,
    pub font_color: egui::Color32,
    pub is_relative: bool,
    pub exif_left_top: String,
    pub exif_left_bot: String,
    pub exif_right_top: String,
    pub exif_right_bot: String,
}

const DEFAULT_BOARDER_SIZE: u32 = 110;
// const DEFAULT_FONT_SIZE: u32 = 25;

impl core::default::Default for Strap {
    fn default() -> Self {
        Self {
            padding: DEFAULT_BOARDER_SIZE,
            color: egui::Color32::WHITE,
            font: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::D2Coding),
            font_color: egui::Color32::BLACK,
            is_relative: false,
            exif_left_top: "ISO{iso_speed} {focal}mm F{fnumber} {exposure}".to_owned(),
            exif_left_bot: "{datetime}".to_owned(),
            exif_right_top: "{camera_mnf} {camera_model}".to_owned(),
            exif_right_bot: "{lens_mnf} {lens_model}".to_owned(),
        }
    }
}

impl Strap {
    fn padding_rel_size(&self, dyn_wh: u32) -> u32 {
        ((self.padding as f32) * ((dyn_wh as f32) / 2000.0)) as u32
    }

    fn rel_size<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> f32 {
        // size.as_() * (self.font_size / (DEFAULT_FONT_SIZE as f32)) * (dyn_wh.as_() / 4000.0)
        size.as_() * (dyn_wh.as_() / 4000.0)
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
        let min_wh = dyn_w.min(dyn_h);

        let rel_padding = self.padding_rel_size(dyn_wh);

        let boarder = crate::effect::boarder::Border::bottom(
            if self.is_relative {
                rel_padding
            } else {
                self.padding
            },
            self.color,
        );

        let mut new_image = boarder.take_from_exist(&dyn_image);
        let text_margin = self.rel_size(12, dyn_wh);
        let margin = self.padding.min(min_wh / 2) + rel_padding;

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $scale:expr, $text:expr) => {
                imageproc::drawing::draw_text_mut(&mut new_image, font_color, ($xxx) as i32, ($yyy as f32 - font.as_scaled($scale).ascent()) as i32, $scale, &font, $text);
            };
        }

        let cam_scale = self.rel_scale(75, dyn_wh);

        // left
        let left_top = pi.view_exif.format_custom(&self.exif_left_top);
        let left_bot = pi.view_exif.format_custom(&self.exif_left_bot);

        let mut y = new_image.height() as f32 - text_margin;
        let left_x = text_margin + boarder.left as f32;

        for left_str in [left_top, left_bot].iter().rev() {
            draw!(left_x, y, cam_scale, left_str);
            y -= cam_scale.y;
        }

        // right
        let right_top = pi.view_exif.format_custom(&self.exif_right_top);
        let right_bot = pi.view_exif.format_custom(&self.exif_right_bot);

        let mut y = new_image.height() as f32 - text_margin;
        let right_x = new_image.width() as f32 - text_margin - boarder.right as f32;
        let mut min_right_x = right_x;
        for right_str in [right_top, right_bot].iter().rev() {
            let (www, _hhh) = crate::theme::text_dimensions(cam_scale, &font, right_str);
            let new_right_x = right_x - www;
            draw!(new_right_x, y, cam_scale, right_str);
            y -= cam_scale.y;
            min_right_x = min_right_x.min(new_right_x);
        }

        // temporary implementation
        if let Some(svg) = crate::ART_UNIFY.get_camera_logo(&pi.view_exif) {
            use image::GenericImageView;

            let logo = svg.draw(boarder.bottom - 10).unwrap();
            let logo_x = (min_right_x - self.rel_size(12, dyn_wh)) as u32 - logo.width();
            let logo_y = new_image.dimensions().1 - boarder.bottom + 5;

            crate::effect::draw_with_transparency::overlay_alpha_screen_mode(
                &mut new_image,
                &logo,
                logo_x,
                logo_y,
            );
        }

        export_config.save_image(&mut new_image, Some(margin as i32), output_path)
    }

    fn ui_config(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Padding configuration
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(t!("theme.just_frame.padding_mode.label"));

                    ui.horizontal(|ui| {
                        let relative_selected = self.is_relative;
                        let absolute_selected = !self.is_relative;

                        if ui
                            .selectable_label(
                                absolute_selected,
                                t!("theme.just_frame.padding_mode.absolute"),
                            )
                            .on_hover_text(t!("theme.just_frame.padding_mode.absolute_hint"))
                            .clicked()
                        {
                            self.is_relative = false;
                        }

                        if ui
                            .selectable_label(
                                relative_selected,
                                t!("theme.just_frame.padding_mode.relative"),
                            )
                            .on_hover_text(t!("theme.just_frame.padding_mode.relative_hint"))
                            .clicked()
                        {
                            self.is_relative = true;
                        }
                    });
                });

                ui.add_space(4.0);

                ui.add(
                    egui::Slider::new(&mut self.padding, 0..=500).text(if self.is_relative {
                        t!("theme.just_frame.padding")
                    } else {
                        t!("scale_config.px_std")
                    }),
                )
                .on_hover_text(t!("theme.just_frame.padding_description"));
            });

            ui.add_space(4.0);

            ui.label(t!("theme.just_frame.color"));
            egui::color_picker::color_picker_color32(
                ui,
                &mut self.color,
                egui::color_picker::Alpha::Opaque,
            );

            ui.add_space(4.0);
        });
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::effect::variable_text::{VariableTextSlot, VariableTextSlotDefault};
use crate::theme::Theme;
use ab_glyph::{Font, ScaleFont};
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ShotOnTwoLine {
    border: crate::effect::border::Border,
    pub font_color: egui::Color32,
    first_font_height: u32,
    second_font_height: u32,
    top_font_height: u32,
    pub bottom_align: crate::fonts::align::TextAlign,
    pub first: VariableTextSlot,
    pub second: VariableTextSlot,
    pub top: VariableTextSlot,
    show_hint: bool,
}

// bottom1 + bottom2 should be under 80
const DEFAULT_BOTTOM1_FONT_HEIGHT: u32 = 35;
const DEFAULT_BOTTOM2_FONT_HEIGHT: u32 = 20;

const DEFAULT_TOP_FONT_HEIGHT: u32 = 50;

const DEFAULT_FIRST: VariableTextSlotDefault =
    VariableTextSlotDefault::with_barlow_weight("shot on [{camera_mnf}  ][{camera_model}]", 500);

const DEFAULT_SECOND: VariableTextSlotDefault = VariableTextSlotDefault::with_barlow_weight(
    "[ISO{iso_speed}  ][{focal}mm  ][F{fnumber}  ][{exposure}s]",
    300,
);

const DEFAULT_TOP: VariableTextSlotDefault =
    VariableTextSlotDefault::with_barlow_weight("[{photo_style}][ = {lut_detail}]", 300);

// const DEFAULT_BORDER_DEFAULT_SIZE: u32 = 90;
const DEFAULT_BORDER_MIN_SIZE: u32 = 25;
const DEFAULT_BORDER_TOP_SIZE: u32 = 70;
const DEFAULT_BORDER_BOTTOM_SIZE: u32 = 150;
const DEFAULT_LIMIT: crate::effect::border::BorderLimit =
    crate::effect::border::BorderLimit::top_and_bottom(0, 900);

const DEFAULT_BORDER: crate::effect::border::Border = crate::effect::border::Border {
    left: DEFAULT_BORDER_MIN_SIZE,
    right: DEFAULT_BORDER_MIN_SIZE,
    top: DEFAULT_BORDER_TOP_SIZE,
    bottom: DEFAULT_BORDER_BOTTOM_SIZE,
    color: egui::Color32::WHITE,
    is_relative: true,
};

impl core::default::Default for ShotOnTwoLine {
    fn default() -> Self {
        Self {
            border: DEFAULT_BORDER,
            font_color: egui::Color32::BLACK,
            first_font_height: DEFAULT_BOTTOM1_FONT_HEIGHT,
            second_font_height: DEFAULT_BOTTOM2_FONT_HEIGHT,
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

impl ShotOnTwoLine {
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

impl Theme for ShotOnTwoLine {
    fn unique_name(&self) -> &'static str {
        "shot_on_two_line"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.shot_on_two_line.title")
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
        let first_txt_scale =
            self.rel_scale((self.first_font_height).clamp(5, 40) as f32 / 100.0, bb);
        let second_txt_scale =
            self.rel_scale((self.second_font_height).clamp(10, 30) as f32 / 100.0, bb);

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

        let ff_a = ff.as_scaled(first_txt_scale).ascent();
        let sf_a = sf.as_scaled(second_txt_scale).ascent();

        let two_line_size = ff_a + (sf_a * 2.0);
        // + font.as_scaled(txt_scale).descent().abs() * 1.0;

        // TODO - Need more profer way
        let txt_b_gap: f32 = (bb as f32 - two_line_size) / 2.0;
        let txt_y_base = new_image.height() as f32 + ff_a + txt_b_gap - bb as f32;

        let gap_x = (bb / 4).max(2).min(ll.min(rr)) as i32;

        let mut y = txt_y_base;

        let first_txt = self.first.format_custom(&pi.view_exif);
        let (www, _hhh) = self.first.text_dimensions(first_txt_scale, &first_txt);
        let new_bottom_x = self.bottom_align.x_point(ll, dyn_w, gap_x, www as i32);

        draw!(
            new_bottom_x,
            y,
            &ff,
            first_txt_scale,
            self.first.weight,
            &first_txt
        );
        y += sf_a * 5.0 / 3.0;

        let second_txt = self.second.format_custom(&pi.view_exif);
        let (www, _hhh) = self.second.text_dimensions(second_txt_scale, &second_txt);
        let new_bottom_x = self.bottom_align.x_point(ll, dyn_w, gap_x, www as i32);

        draw!(
            new_bottom_x,
            y,
            &sf,
            second_txt_scale,
            self.second.weight,
            &second_txt
        );

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

    fn ui_config(&mut self, ui: &mut egui::Ui) {
        self.border.ui_config(ui, &DEFAULT_BORDER, &DEFAULT_LIMIT);

        ui.vertical(|ui| {
            // Padding configuration
            ui.add_space(4.0);

            // Own configuration
            egui::Grid::new("shot_on_two_line_config_grid")
                .num_columns(2)
                .spacing([4.0, 3.0])
                .show(ui, |ui| {
                    ui.label(t!("theme.exif_center_top") + " " + t!("theme.font_color"));
                    egui::widgets::color_picker::color_edit_button_srgba(
                        ui,
                        &mut self.font_color,
                        egui::color_picker::Alpha::Opaque,
                    );
                    ui.end_row();

                    // for top
                    ui.label(
                        t!("theme.exif_center_top") + " " + t!("theme.font_height_ratio.label"),
                    )
                    .on_hover_text(t!("theme.font_height_ratio.hint"));
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut self.top_font_height, 5..=90).step_by(1.0));
                        ui.label("% ");
                        if ui.button("↺").clicked() {
                            self.top_font_height = DEFAULT_TOP_FONT_HEIGHT;
                        }
                    });
                    ui.end_row();

                    // todo - WARN when tt is under the 10
                    self.top.ui(ui, t!("theme.exif_center_top"), &DEFAULT_TOP);
                    ui.end_row();

                    // for bottom
                    ui.label(t!("text_align.bottom_text_align"));
                    self.bottom_align.update_ui(ui);
                    ui.end_row();

                    // bottom 1
                    self.first.ui(ui, t!("theme.bottom1"), &DEFAULT_FIRST);
                    ui.end_row();

                    ui.label(t!("theme.font_height_ratio.first"))
                        .on_hover_text(t!("theme.font_height_ratio.hint"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.first_font_height, 10..=40).step_by(1.0),
                        );
                        ui.label("% ");
                        if ui.button("↺").clicked() {
                            self.first_font_height = DEFAULT_BOTTOM1_FONT_HEIGHT;
                        }
                    });
                    ui.end_row();

                    // bottom 2
                    self.second.ui(ui, t!("theme.bottom2"), &DEFAULT_SECOND);
                    ui.end_row();

                    ui.label(t!("theme.font_height_ratio.second"))
                        .on_hover_text(t!("theme.font_height_ratio.hint"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Slider::new(&mut self.second_font_height, 5..=30).step_by(1.0),
                        );
                        ui.label("% ");
                        if ui.button("↺").clicked() {
                            self.second_font_height = DEFAULT_BOTTOM2_FONT_HEIGHT;
                        }
                    });
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

    fn is_ui_config_available(&self) -> bool {
        true
    }
}

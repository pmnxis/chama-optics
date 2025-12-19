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
pub struct Monitor {
    border: crate::effect::border::Border,
    pub font_color: egui::Color32,
    font_height: u32,
    pub bottoms: [VariableTextSlot; 4],
    pub width_aligned: bool,
    show_hint: bool,
}

const DEFAULT_FONT_HEIGHT: u32 = 75;

const DEFAULT_BOTTOM: [VariableTextSlotDefault; 4] = [
    VariableTextSlotDefault::with_barlow_weight("[F{fnumber}]", 500),
    VariableTextSlotDefault::with_barlow_weight("[{exposure}s]", 500),
    VariableTextSlotDefault::with_barlow_weight("[ISO{iso_speed}]", 500),
    VariableTextSlotDefault::with_barlow_weight("[{focal}mm]", 500),
];

const DEFAULT_BORDER_DEFAULT_SIZE: u32 = 80;
const DEFAULT_BORDER_MIN_SIZE: u32 = 50;
const DEFAULT_LIMIT: crate::effect::border::BorderLimit =
    crate::effect::border::BorderLimit::bottom(DEFAULT_BORDER_MIN_SIZE, 900);
const DEFAULT_BORDER: crate::effect::border::Border =
    crate::effect::border::Border::bottom(DEFAULT_BORDER_DEFAULT_SIZE, egui::Color32::BLACK);
// const DEFAULT_FONT_SIZE: u32 = 25;

impl core::default::Default for Monitor {
    fn default() -> Self {
        Self {
            border: DEFAULT_BORDER,
            font_color: egui::Color32::WHITE,
            font_height: DEFAULT_FONT_HEIGHT,
            bottoms: [
                VariableTextSlot::from_default(&DEFAULT_BOTTOM[0]),
                VariableTextSlot::from_default(&DEFAULT_BOTTOM[1]),
                VariableTextSlot::from_default(&DEFAULT_BOTTOM[2]),
                VariableTextSlot::from_default(&DEFAULT_BOTTOM[3]),
            ],
            width_aligned: true,
            show_hint: false,
        }
    }
}

impl Monitor {
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

impl Theme for Monitor {
    fn unique_name(&self) -> &'static str {
        "monitor"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.monitor.title")
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

        let (ll, _rr, _tt, bb) = self.border.border_size(dyn_wh);
        let font_height_ratio = self.font_height.clamp(10, 80) as f32 / 100.0;
        let txt_scale = self.rel_scale(font_height_ratio, bb);
        let mut new_image = self.border.take_from_exist(&dyn_image, dyn_wh);

        // TODO - Need more profer way

        let y = new_image.height() - (bb / 2);

        if self.width_aligned {
            let mut xxx = ll as f32;
            let total_www: f32 = self
                .bottoms
                .iter()
                .map(|item| {
                    let txt = item.format_custom(&pi.view_exif);
                    let (www, _hhh) = crate::theme::text_dimensions_with_fallback(
                        txt_scale,
                        &item.get_font(),
                        item.weight,
                        &txt,
                    );
                    www
                })
                .sum();

            let gap = ((dyn_w as f32) - total_www) / 5.0;

            for item in self.bottoms.iter() {
                let txt = item.format_custom(&pi.view_exif);
                let (www, _hhh) = crate::theme::text_dimensions_with_fallback(
                    txt_scale,
                    &item.get_font(),
                    item.weight,
                    &txt,
                );
                xxx += gap;

                let yg = (item.get_font().as_scaled(txt_scale).ascent()
                    + item.get_font().as_scaled(txt_scale).descent().abs())
                    * 0.6;

                let yz = y as f32 - (yg);

                crate::theme::draw_text_with_fallback(
                    &mut new_image,
                    font_color,
                    xxx as i32,
                    yz as i32,
                    txt_scale,
                    &item.get_font(),
                    item.weight,
                    &txt,
                );

                xxx += www;
            }
        } else {
            for (idx, item) in self.bottoms.iter().enumerate() {
                let mut xxx = (ll as f32) + ((dyn_w as f32 / 5.0) * (idx + 1) as f32);
                let txt = item.format_custom(&pi.view_exif);
                let (www, _hhh) = crate::theme::text_dimensions_with_fallback(
                    txt_scale,
                    &item.get_font(),
                    item.weight,
                    &txt,
                );
                xxx -= www / 2.0;

                let yg = (item.get_font().as_scaled(txt_scale).ascent()
                    + item.get_font().as_scaled(txt_scale).descent().abs())
                    * 0.6;
                let yz = y as f32 - (yg);

                crate::theme::draw_text_with_fallback(
                    &mut new_image,
                    font_color,
                    xxx as i32,
                    yz as i32,
                    txt_scale,
                    &item.get_font(),
                    item.weight,
                    &txt,
                );
            }
        }

        let temp_margin = self.border.interactive_watermark_padding(dyn_w, dyn_h) / 6;
        let border_margin = (temp_margin * 5).max(bb) + temp_margin;

        export_config.save_image(&mut new_image, Some(border_margin as i32), output_path)
    }

    fn ui_config(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        self.border.ui_config(ui, &DEFAULT_BORDER, &DEFAULT_LIMIT);

        ui.vertical(|ui| {
            // Padding configuration
            ui.add_space(4.0);

            // Own configuration
            egui::Grid::new("monitor_config_grid")
                .num_columns(2)
                .spacing([4.0, 3.0])
                .show(ui, |ui| {
                    ui.label(t!("theme.font_color"));
                    egui::widgets::color_picker::color_edit_button_srgba(
                        ui,
                        &mut self.font_color,
                        egui::color_picker::Alpha::Opaque,
                    );
                    ui.end_row();

                    ui.label(t!("theme.font_height_ratio.label"))
                        .on_hover_text(t!("theme.font_height_ratio.hint"));
                    ui.horizontal(|ui| {
                        ui.add(
                            // [slider_width, 23.0],
                            egui::Slider::new(&mut self.font_height, 50..=80).step_by(1.0),
                        );
                        ui.label("% ");
                        if ui.button("↺").clicked() {
                            self.font_height = DEFAULT_FONT_HEIGHT;
                        }
                    });
                    ui.end_row();

                    ui.label(t!("theme.monitor_config.center_align.label"));
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.width_aligned,
                            false,
                            t!("theme.monitor_config.center_align.simple"),
                        )
                        .on_hover_text(t!("theme.monitor_config.center_align.simple_hint"));
                        ui.selectable_value(
                            &mut self.width_aligned,
                            true,
                            t!("theme.monitor_config.center_align.spacing"),
                        )
                        .on_hover_text(t!("theme.monitor_config.center_align.spacing_hint"));
                    });
                    ui.end_row();

                    for (idx, bottom) in self.bottoms.iter_mut().enumerate() {
                        bottom.ui(
                            ctx,
                            ui,
                            t!(format!("theme.monitor_config.exif_bottom_{idx}")),
                            &DEFAULT_BOTTOM[idx],
                        );
                        ui.end_row();
                    }
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

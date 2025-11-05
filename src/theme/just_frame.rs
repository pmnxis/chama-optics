/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct JustFrame {
    pub padding: u32,
    pub color: egui::Color32,
    pub is_relative: bool,
}

const DEFAULT_BOARDER_SIZE: u32 = 100;

impl core::default::Default for JustFrame {
    fn default() -> Self {
        Self {
            padding: DEFAULT_BOARDER_SIZE,
            color: egui::Color32::WHITE,
            is_relative: false,
        }
    }
}

impl JustFrame {
    fn rel_size(&self, dyn_wh: u32) -> u32 {
        ((self.padding as f32) * ((dyn_wh as f32) / 2000.0)) as u32
    }
}

impl Theme for JustFrame {
    fn unique_name(&self) -> &'static str {
        "just_frame"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.just_frame.title")
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
        let min_wh = dyn_w.min(dyn_h);

        let rel_padding = self.rel_size(dyn_wh);
        println!("{rel_padding} {}", self.padding);
        let boarder = crate::effect::boarder::Border::uniform(
            if self.is_relative {
                rel_padding
            } else {
                self.padding
            },
            self.color,
        );

        let mut new_image = boarder.take_from_exist(&dyn_image);
        let margin = self.padding.min(min_wh / 2) + rel_padding;

        export_config.save_image(&mut new_image, Some(margin as i32), output_path)
    }

    fn ui_config(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // 🟦 Padding 설정
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

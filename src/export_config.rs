/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

pub struct ExportConfig {
    pub scale_config: crate::scale_config::ScaleConfig,
    pub extension: image::ImageFormat,
}

impl std::default::Default for ExportConfig {
    fn default() -> Self {
        Self {
            scale_config: crate::scale_config::SCALE_NEAR_COMMON_4K,
            extension: image::ImageFormat::WebP,
        }
    }
}

impl ExportConfig {
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Export Configuration");
            ui.add_space(4.0);

            // scale specific configuration
            ui.separator();
            ui.label("Scale Settings");
            self.scale_config.update_ui(ui);

            ui.add_space(8.0);

            // general configuration
            ui.separator();
            ui.label("Output Format");

            let mut selected = match self.extension {
                image::ImageFormat::Jpeg => "JPEG",
                image::ImageFormat::WebP => "WEBP",
                _ => "WEBP",
            }
            .to_string();

            egui::ComboBox::from_id_salt("export_format")
                .selected_text(&selected)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, "JPEG".to_string(), "JPEG");
                    ui.selectable_value(&mut selected, "WEBP".to_string(), "WEBP");
                });

            self.extension = match selected.as_str() {
                "WEBP" => image::ImageFormat::WebP,
                _ => image::ImageFormat::Jpeg,
            };
        });
    }

    pub fn extension(&self) -> &str {
        match self.extension {
            image::ImageFormat::Jpeg => "jpg",
            image::ImageFormat::WebP => "webp",
            _ => "webp",
        }
    }
}

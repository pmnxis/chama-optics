/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ExportConfig {
    pub scale_config: crate::scale_config::ScaleConfig,
    pub output_format: crate::output_format::OutputFormat,
}

impl core::default::Default for ExportConfig {
    fn default() -> Self {
        Self {
            scale_config: crate::scale_config::SCALE_NEAR_COMMON_4K,
            output_format: crate::output_format::OutputFormat::default(),
        }
    }
}

impl ExportConfig {
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading(t!("export_config.label"));
            ui.separator();
            self.scale_config.update_ui(ui);
            ui.separator();
            self.output_format.update_ui(ui);
        });
    }
}

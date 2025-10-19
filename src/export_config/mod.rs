/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use rust_i18n::t;

pub(crate) mod output_format;
pub(crate) mod output_name;
pub(crate) mod scale_config;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ExportConfig {
    pub scale_config: scale_config::ScaleConfig,
    pub output_format: output_format::OutputFormat,
    pub output_name: output_name::OutputName,
    pub theme_reg: crate::theme::ThemeRegistry,
}

impl core::default::Default for ExportConfig {
    fn default() -> Self {
        Self {
            scale_config: scale_config::SCALE_NEAR_COMMON_4K,
            output_format: output_format::OutputFormat::default(),
            output_name: output_name::OutputName::default(),
            theme_reg: crate::theme::ThemeRegistry::new(),
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
            ui.separator();
            self.output_name.update_ui(ui);
            ui.separator();
            self.theme_reg.update_ui(ui);
        });
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use rust_i18n::t;

pub(crate) mod open_explorer;
pub(crate) mod output_format;
pub(crate) mod output_name;
pub(crate) mod scale_config;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ExportConfig {
    pub scale_config: scale_config::ScaleConfig,
    pub output_format: output_format::OutputFormat,
    pub output_name: output_name::OutputName,
    pub theme_reg: crate::theme::ThemeRegistry,
    pub watermark: crate::effect::watermark::Watermark,
}

impl core::default::Default for ExportConfig {
    fn default() -> Self {
        #[cfg(not(test))]
        {
            Self {
                scale_config: scale_config::SCALE_NEAR_COMMON_4K,
                output_format: output_format::OutputFormat::default(),
                output_name: output_name::OutputName::default(),
                theme_reg: crate::theme::ThemeRegistry::new(),
                watermark: crate::effect::watermark::Watermark::default(),
            }
        }
        #[cfg(test)]
        {
            Self::testkit_default()
        }
    }
}

impl ExportConfig {
    pub fn update_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading(t!("export_config.label"));
            ui.separator();
            self.scale_config.update_ui(ui);
            ui.collapsing(t!("export_config.detail_of_export"), |ui| {
                ui.separator();
                self.output_format.update_ui(ui);
                ui.separator();
                self.output_name.update_ui(ui);
                ui.separator();
                self.watermark.update_ui(ctx, ui);
            });
            ui.separator();
            self.theme_reg.update_ui(ctx, ui);
        });
    }

    pub fn testkit_default() -> Self {
        use std::str::FromStr;
        Self {
            scale_config: scale_config::SCALE_HALF,
            output_format: output_format::OutputFormat::default(),
            output_name: output_name::OutputName {
                prefix: "".to_owned(),
                postfix: "-testcase".to_owned(),
                folder: std::path::PathBuf::from_str("test_image/export").unwrap(),
                remove_after_bulk_save: false,
            },
            theme_reg: crate::theme::ThemeRegistry::new(),
            watermark: crate::effect::watermark::Watermark::default(),
        }
    }

    pub fn save_image<P: AsRef<std::path::Path>>(
        &self,
        dyn_image: &mut image::DynamicImage,
        margin: Option<i32>,
        path: P,
    ) -> Result<(), image::ImageError> {
        if self.watermark.is_enabled {
            self.watermark.apply(dyn_image, margin)?;
        }
        self.output_format.save_image(dyn_image, path)
    }

    #[cfg(test)]
    pub fn insert_or_replace_theme<T: crate::theme::Theme + 'static>(&mut self, theme: T) {
        self.theme_reg.insert_or_replace_theme(theme);
    }
}

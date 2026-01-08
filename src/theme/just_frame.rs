/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use crate::update_param;
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize, chama_optics_macros::ThemeParameters)]
pub struct JustFrame {
    #[param(
        border,
        label_key = "theme.border",
        default_border = "DEFAULT_BORDER",
        default_limit = "DEFAULT_LIMIT"
    )]
    pub border: crate::effect::border::Border,
}

const DEFAULT_BORDER_SIZE: u32 = 100;
const DEFAULT_BORDER: crate::effect::border::Border =
    crate::effect::border::Border::uniform(DEFAULT_BORDER_SIZE, egui::Color32::WHITE);
const DEFAULT_LIMIT: crate::effect::border::BorderLimit =
    crate::effect::border::BorderLimit::uniform(0, 800);

impl core::default::Default for JustFrame {
    fn default() -> Self {
        Self {
            border: DEFAULT_BORDER,
        }
    }
}

impl Theme for JustFrame {
    fn unique_name(&self) -> &'static str {
        "just_frame"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.just_frame.title")
    }

    fn apply_to_image(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        let scale_config = &export_config.scale_config;
        let dyn_image = pi.with_scale_and_orientation(*scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);

        let new_image = self.border.take_from_exist(&dyn_image, dyn_wh);

        Ok(new_image)
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

        let margin = self.border.interactive_watermark_padding(dyn_w, dyn_h);

        let mut themed_image = self.apply_to_image(pi, export_config)?;
        export_config.save_image(&mut themed_image, Some(margin as i32), output_path)
    }

    fn ui_config(&mut self, ui: &mut egui::Ui) {
        self.auto_ui_config(ui);
    }

    fn is_ui_config_available(&self) -> bool {
        true
    }

    fn get_parameters_json(&self) -> String {
        self.auto_get_parameters_json()
    }
}

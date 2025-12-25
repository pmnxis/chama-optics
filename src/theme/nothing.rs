/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct Nothing {}

impl Theme for Nothing {
    fn unique_name(&self) -> &'static str {
        "nothing"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.nothing")
    }

    fn apply_to_image(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        let scale_config = &export_config.scale_config;
        pi.with_scale_and_orientation(*scale_config)
    }

    fn ui_config(&mut self, _ui: &mut egui::Ui) {
        // nothing, because `is_ui_config_available(&self) is false`
    }

    fn is_ui_config_available(&self) -> bool {
        false
    }
}

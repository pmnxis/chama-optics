/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Nothing {}

impl Theme for Nothing {
    fn unique_name(&self) -> &'static str {
        "nothing"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.nothing")
    }

    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        let scale_config = &export_config.scale_config;
        let dyn_image = pi.with_scale_and_orientation(*scale_config)?;

        export_config
            .output_format
            .save_image(&dyn_image, output_path)
    }

    fn ui_config(&mut self, _ui: &mut egui::Ui) {}
}

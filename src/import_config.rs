/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct ImportConfig {
    /// If true, when EXIF F-number is invalid or missing,
    /// try to extract the minimum aperture value from lens information.
    /// Useful for some manual-focus lenses.
    pub get_alt_fnumber: bool,
}

impl ImportConfig {
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading(t!("import_config.label"));
            ui.add_space(4.0);

            ui.separator();

            // Checkbox with hover hint (no separate label)
            ui.checkbox(
                &mut self.get_alt_fnumber,
                t!("import_config.f_number_recovery.name"),
            )
            .on_hover_text(t!("import_config.f_number_recovery.description"));
        });
    }
}

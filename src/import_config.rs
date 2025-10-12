/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Default)]
pub struct ImportConfig {
    /// If true, when EXIF F-number is invalid or missing,
    /// try to extract the minimum aperture value from lens information.
    /// Useful for some manual-focus lenses.
    pub get_alt_fnumber: bool,
}


impl ImportConfig {
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.heading("Import Configuration");
            ui.add_space(4.0);

            ui.separator();

            // Checkbox with hover hint (no separate label)
            ui.checkbox(&mut self.get_alt_fnumber, "F-number Recovery")
                .on_hover_text(
                    "If the EXIF aperture value is missing or invalid, \
this option attempts to recover it by parsing the lens information. \
Useful for manual-focus lenses that embed aperture data in the lens name (e.g., 'F0.95', 'f3.5-5.6').",
                );
        });
    }
}

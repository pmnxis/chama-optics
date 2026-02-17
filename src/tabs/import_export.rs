/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Import&Export Tab UI

use crate::ChamaOptics;
use rust_i18n::t;

#[cfg(not(feature = "ios_integration"))]
impl ChamaOptics {
    /// Render Tab 3: Import&Export Config
    pub(crate) fn render_import_export_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.import_export"));
        ui.separator();

        // Import config section
        let prev_simplify = self.import_config.simplify_lens_model;
        self.import_config.update_ui(ui);

        // Reapply lens model simplification if the setting changed
        if self.import_config.simplify_lens_model != prev_simplify {
            for img in &mut self.packed_images {
                img.view_exif
                    .reapply_simplify_lens_model(self.import_config.simplify_lens_model);
            }
        }

        ui.add_space(10.0);

        // Export config section
        self.export_config
            .update_ui(ui, self.show_theme_name_in_english);

        // temporary call detect config ui here
        ui.heading(t!("face_detection.detail_of_detection_engine"));
        self.export_config.face_detection.update_ui(ui);
    }
}

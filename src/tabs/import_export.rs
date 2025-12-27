/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Import&Export Tab UI

use crate::ChamaOptics;
use rust_i18n::t;

impl ChamaOptics {
    /// Render Tab 3: Import&Export Config
    pub(crate) fn render_import_export_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.import_export"));
        ui.separator();

        // Import config section
        self.import_config.update_ui(ui);

        ui.add_space(10.0);

        // Export config section
        self.export_config
            .update_ui(ui, self.show_theme_name_in_english);
    }
}

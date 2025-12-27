/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Image List Tab UI

use crate::ChamaOptics;
use rust_i18n::t;

impl ChamaOptics {
    /// Render Tab 1: Image List
    pub(crate) fn render_image_list_tab(&mut self, ui: &mut egui::Ui) {
        // Tab heading
        ui.heading(t!("tabs.image_list"));
        ui.separator();

        // Image list controls
        ui.horizontal(|ui| {
            ui.heading(t!("app.images.list"));

            // File dialog button
            #[cfg(feature = "desktop")]
            if ui.button(t!("app.open_files.button")).clicked()
                && let Some(open_files) = rfd::FileDialog::new().pick_files()
            {
                for file in open_files.iter() {
                    self.pending_paths.push_back(file.to_owned());
                }
            }

            // Web file picker button
            #[cfg(all(target_arch = "wasm32", feature = "web"))]
            if ui.button(t!("app.open_files.button")).clicked() {
                self.trigger_file_picker();
            }

            // Save all button
            if ui.button(t!("app.images.save_all")).clicked() {
                self.save_packed_image_all(ui);
            }

            // Explorer button
            crate::export_config::open_explorer::launch_explorer_ui(
                ui,
                &self.export_config.output_name.folder,
            );

            // Remove all button
            if ui.button(t!("app.images.remove_all")).clicked() {
                self.packed_images.clear();
            }

            // Show grouping buttons if any grouping feature is enabled
            if self.image_grouping.is_any_enabled() {
                // Apply grouping button
                if ui
                    .button(t!("laboratory.group_similar.apply_grouping"))
                    .clicked()
                {
                    self.apply_image_grouping(ui.ctx());
                }

                // Clear grouping button (only show if grouping is active)
                if self.image_groups.is_some()
                    && ui
                        .button(t!("laboratory.group_similar.clear_grouping"))
                        .clicked()
                {
                    self.image_groups = None;
                    ui.ctx().request_repaint();
                    log::info!("Image grouping cleared");
                }
            }
        });

        // Scrollable image list with background hint
        let available_rect = ui.available_rect_before_wrap();

        if self.packed_images.is_empty() && !self.load_progress.is_active() {
            ui.painter().text(
                available_rect.center(),
                egui::Align2::CENTER_CENTER,
                t!("app.open_files.drag_drop"),
                egui::FontId::proportional(42.0),
                egui::Color32::from_rgba_unmultiplied(200, 200, 200, 50),
            );
        }

        ui.allocate_ui_with_layout(
            egui::vec2(available_rect.width(), available_rect.height().max(200.0)),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| self.update_packed_image(ui));
            },
        );
    }
}

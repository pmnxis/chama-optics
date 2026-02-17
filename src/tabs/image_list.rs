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
        // Poll pending pick-files dialog
        #[cfg(feature = "rfd")]
        if let Some(ref pending) = self.pending_pick_files
            && let Some(result) = pending.try_recv()
        {
            self.pending_pick_files = None;
            if let Some(open_files) = result {
                for file in open_files.iter() {
                    self.pending_paths.push_back(file.to_owned());
                }
            }
        }

        // Reduce default spacing for this tab
        ui.spacing_mut().item_spacing.y = 4.0; // Reduced from default ~8.0

        // Tab heading
        ui.heading(t!("tabs.image_list"));

        ui.separator();

        // Image list controls (no extra heading to reduce spacing)
        ui.horizontal(|ui| {
            ui.strong(t!("app.images.list"));

            // File dialog button
            #[cfg(feature = "rfd")]
            {
                let is_pending = self.pending_pick_files.is_some();
                if ui
                    .add_enabled(!is_pending, egui::Button::new(t!("app.open_files.button")))
                    .clicked()
                    && !is_pending
                {
                    self.pending_pick_files =
                        Some(crate::util::async_file_dialog::pick_files_async());
                }
            }

            #[cfg(all(feature = "desktop", not(feature = "rfd")))]
            if ui.button(t!("app.open_files.button")).clicked()
                && let Some(open_files) = rfd::FileDialog::new().pick_files()
            {
                for file in open_files.iter() {
                    self.pending_paths.push_back(file.to_owned());
                }
            }

            // Save all button
            if ui.button(t!("app.images.save_all")).clicked() {
                self.save_packed_image_all(ui);
            }

            // Explorer button
            #[cfg(all(feature = "desktop", not(feature = "ios_integration")))]
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

        // Use Frame::NONE to remove default padding
        egui::Frame::NONE.show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    self.update_packed_image(ui);
                });
        });
    }
}

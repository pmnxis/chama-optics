/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Misc Tab UI (Settings)

use crate::ChamaOptics;
use rust_i18n::t;

#[cfg(not(feature = "ios_integration"))]
impl ChamaOptics {
    /// Render Tab 4: Settings
    pub(crate) fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.misc_heading"));
        ui.separator();

        egui::Grid::new("misc_grid")
            .num_columns(2)
            .spacing([4.0, 3.0])
            .striped(true)
            .show(ui, |ui| {
                // Version Info
                ui.label(t!("settings.version_info"));
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("ChamaOptics");
                        ui.label(format!(
                            "v{} ({})",
                            env!("PROJECT_VERSION"),
                            env!("GIT_COMMIT_SHORT_HASH")
                        ));
                    });
                    self.update.ui(ui);
                });
                ui.end_row();

                // UI Theme
                ui.label(t!("settings.theme"));
                ui.horizontal(|ui| {
                    let current_theme = ui.ctx().options(|o| o.theme_preference);

                    if ui
                        .selectable_label(
                            current_theme == egui::ThemePreference::System,
                            t!("settings.theme_system"),
                        )
                        .clicked()
                    {
                        ui.ctx()
                            .options_mut(|o| o.theme_preference = egui::ThemePreference::System);
                    }
                    if ui
                        .selectable_label(
                            current_theme == egui::ThemePreference::Dark,
                            t!("settings.theme_dark"),
                        )
                        .clicked()
                    {
                        ui.ctx()
                            .options_mut(|o| o.theme_preference = egui::ThemePreference::Dark);
                    }
                    if ui
                        .selectable_label(
                            current_theme == egui::ThemePreference::Light,
                            t!("settings.theme_light"),
                        )
                        .clicked()
                    {
                        ui.ctx()
                            .options_mut(|o| o.theme_preference = egui::ThemePreference::Light);
                    }
                });
                ui.end_row();

                // Language
                ui.label(t!("settings.language"));
                ui.horizontal(|ui| {
                    use strum::IntoEnumIterator;
                    for lang in crate::langs::Language::iter() {
                        if ui
                            .selectable_label(
                                self.lang == lang,
                                t!(format!("language.{}", lang.into_str())),
                            )
                            .clicked()
                        {
                            self.lang = lang;
                            self.lang.update_i18n();
                        }
                    }
                });
                ui.end_row();

                // Theme Name Display
                ui.label(t!("settings.theme_name_display"));
                ui.checkbox(
                    &mut self.show_theme_name_in_english,
                    t!("settings.show_theme_name_in_english"),
                );
                ui.end_row();

                // Temporary Directory (iOS only)
                #[cfg(target_os = "ios")]
                {
                    ui.label(t!("settings.temp_dir"))
                        .on_hover_text(t!("settings.temp_dir_hint"));
                    ui.horizontal(|ui| {
                        use crate::app_state::TempDir;
                        for temp_dir in [TempDir::Tmp, TempDir::VarTmp] {
                            if ui
                                .selectable_label(self.temp_dir == temp_dir, temp_dir.label())
                                .clicked()
                            {
                                self.temp_dir = temp_dir;
                            }
                        }
                    });
                    ui.end_row();
                }
            });

        ui.add_space(20.0);
        ui.separator();

        // 🧪 Laboratory (Experimental Features)
        ui.heading(t!("laboratory.heading"));
        ui.label(
            egui::RichText::new(t!("laboratory.warning"))
                .color(ui.visuals().warn_fg_color)
                .italics(),
        );
        ui.add_space(10.0);

        // Group Similar Images
        egui::CollapsingHeader::new(t!("laboratory.group_similar.title"))
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("laboratory_grouping_grid")
                    .num_columns(2)
                    .spacing([20.0, 10.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Group by date
                        ui.label(t!("laboratory.group_similar.group_by_date"));
                        ui.checkbox(&mut self.image_grouping.group_by_date, "");
                        ui.end_row();

                        // Group by time
                        ui.label(t!("laboratory.group_similar.group_by_time"));
                        ui.checkbox(&mut self.image_grouping.group_by_time, "");
                        ui.end_row();

                        // Group by camera manufacturer
                        ui.label(t!("laboratory.group_similar.group_by_camera_mnf"));
                        ui.checkbox(&mut self.image_grouping.group_by_camera_mnf, "");
                        ui.end_row();

                        // Group by camera model
                        ui.label(t!("laboratory.group_similar.group_by_camera"));
                        ui.checkbox(&mut self.image_grouping.group_by_camera, "");
                        ui.end_row();

                        // Group by lens model
                        ui.label(t!("laboratory.group_similar.group_by_lens"));
                        ui.checkbox(&mut self.image_grouping.group_by_lens, "");
                        ui.end_row();

                        // Group by similarity (with warning)
                        ui.horizontal(|ui| {
                            ui.label(t!("laboratory.group_similar.group_by_similarity"));
                            ui.label(
                                egui::RichText::new("⚠")
                                    .color(egui::Color32::from_rgb(255, 180, 0)),
                            )
                            .on_hover_text(t!("laboratory.group_similar.similarity_warning"));
                        });
                        ui.checkbox(&mut self.image_grouping.group_by_similarity, "");
                        ui.end_row();

                        // Time threshold
                        ui.label(t!("laboratory.group_similar.time_threshold"));
                        ui.add(
                            egui::Slider::new(
                                &mut self.image_grouping.time_threshold_secs,
                                10..=3600,
                            )
                            .suffix(t!("app.image_grouping.time_unit"))
                            .logarithmic(true),
                        );
                        ui.end_row();

                        // Similarity threshold
                        ui.label(t!("laboratory.group_similar.similarity_threshold"));
                        ui.add(
                            egui::Slider::new(
                                &mut self.image_grouping.similarity_threshold,
                                0.0..=1.0,
                            )
                            .step_by(0.05),
                        );
                        ui.end_row();
                    });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
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
                });

                // Show current grouping status
                if let Some(groups) = &self.image_groups {
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "✓ {} {}",
                            groups.len(),
                            t!("laboratory.group_similar.groups_found")
                        ))
                        .color(ui.visuals().strong_text_color()),
                    );
                }
            });
    }

    /// Apply image grouping and reorder the images in the list
    #[cfg(not(feature = "ios_integration"))]
    pub(crate) fn apply_image_grouping(&mut self, ctx: &egui::Context) {
        if self.packed_images.is_empty() {
            log::info!("No images to group");
            return;
        }

        log::info!(
            "Applying image grouping with config: {:?}",
            self.image_grouping
        );

        let groups =
            crate::image_group::group_similar_images(&self.packed_images, &self.image_grouping);

        log::info!("Found {} groups", groups.len());

        // Collect UUIDs in the desired order
        let mut new_order_uuids: Vec<uuid::Uuid> = Vec::new();
        for group in groups.iter() {
            new_order_uuids.extend(&group.image_uuids);
        }

        // Reorder packed_images to match the UUID order using swaps
        // Build a mapping from UUID to desired position
        let mut uuid_to_target_pos: std::collections::HashMap<uuid::Uuid, usize> =
            std::collections::HashMap::new();
        for (target_pos, &uuid) in new_order_uuids.iter().enumerate() {
            uuid_to_target_pos.insert(uuid, target_pos);
        }

        // Reorder using cycle-based swapping
        #[allow(clippy::needless_range_loop)]
        for target_pos in 0..new_order_uuids.len().min(self.packed_images.len()) {
            while self.packed_images[target_pos].uuid != new_order_uuids[target_pos] {
                // Find where the correct image currently is
                let correct_uuid = new_order_uuids[target_pos];
                if let Some(current_pos) = self
                    .packed_images
                    .iter()
                    .position(|img| img.uuid == correct_uuid)
                {
                    self.packed_images.swap(target_pos, current_pos);
                } else {
                    break; // UUID not found, skip
                }
            }
        }

        // Groups already have correct UUIDs - no index rebuilding needed!
        // Just store the groups as-is
        let groups_len = groups.len();
        self.image_groups = Some(groups);

        ctx.request_repaint();

        log::info!(
            "Image grouping applied successfully - {} groups",
            groups_len
        );
    }
}

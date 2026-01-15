/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Sticker Tab UI - Manage sticker storage and settings

use crate::ChamaOptics;
use rust_i18n::t;

impl ChamaOptics {
    /// Render the Sticker management tab
    pub(crate) fn render_sticker_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("effect.sticker_storage.title"));
        ui.separator();

        // Verify stickers on load (check for hash mismatches)
        let mut has_hash_warnings = false;
        for sticker in &self.sticker_storage.stickers {
            if sticker.hash_mismatch {
                has_hash_warnings = true;
                break;
            }
        }

        // Show hash mismatch warning
        if has_hash_warnings {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                t!("effect.sticker_storage.hash_warning"),
            );
            ui.add_space(5.0);
        }

        // Sticker storage management
        self.sticker_storage.update_ui(ui);

        ui.add_space(20.0);
        ui.separator();

        // Sticker application settings
        ui.heading(t!("effect.sticker_storage.settings"));
        ui.add_space(10.0);

        egui::Grid::new("sticker_settings_grid")
            .num_columns(2)
            .spacing([20.0, 10.0])
            .striped(true)
            .show(ui, |ui| {
                // Scale
                ui.label(t!("effect.sticker_storage.scale"));
                ui.add(egui::Slider::new(&mut self.sticker_config.scale, 0.5..=2.0));
                ui.end_row();

                // Offset X
                ui.label(t!("effect.sticker_storage.offset_x"));
                ui.add(egui::Slider::new(
                    &mut self.sticker_config.offset_x,
                    -100..=100,
                ));
                ui.end_row();

                // Offset Y
                ui.label(t!("effect.sticker_storage.offset_y"));
                ui.add(egui::Slider::new(
                    &mut self.sticker_config.offset_y,
                    -100..=100,
                ));
                ui.end_row();
            });

        ui.add_space(20.0);

        // Sticker preview section with images
        if !self.sticker_storage.stickers.is_empty() {
            ui.separator();
            ui.heading(t!("effect.sticker_storage.preview"));
            ui.add_space(10.0);

            // Collect sticker info first to avoid borrow checker issues
            let sticker_infos: Vec<(uuid::Uuid, String, bool, bool, bool)> = self
                .sticker_storage
                .stickers
                .iter()
                .map(|s| {
                    (
                        s.id,
                        s.name.clone(),
                        self.sticker_storage.default_sticker_id == Some(s.id),
                        s.file_missing,
                        s.hash_mismatch,
                    )
                })
                .collect();

            // Show thumbnails of all stickers with images
            ui.horizontal_wrapped(|ui| {
                for (id, name, is_default, file_missing, hash_mismatch) in sticker_infos {
                    ui.group(|ui| {
                        ui.vertical_centered_justified(|ui| {
                            // Show sticker image
                            let thumbnail_size = 80.0;
                            if let Some(texture) = self.sticker_storage.get_texture(ui.ctx(), id) {
                                ui.image((texture.id(), egui::Vec2::splat(thumbnail_size)));
                            } else {
                                // Placeholder for failed to load
                                ui.allocate_ui(egui::Vec2::splat(thumbnail_size), |ui| {
                                    ui.centered_and_justified(|ui| {
                                        if file_missing {
                                            ui.label(
                                                egui::RichText::new("❌")
                                                    .size(32.0)
                                                    .color(ui.visuals().error_fg_color),
                                            );
                                        } else if hash_mismatch {
                                            ui.label(
                                                egui::RichText::new("⚠️")
                                                    .size(32.0)
                                                    .color(ui.visuals().warn_fg_color),
                                            );
                                        } else {
                                            ui.label(egui::RichText::new("📷").size(32.0));
                                        }
                                    });
                                });
                            }

                            // Show sticker name
                            ui.label(&name);

                            // Show default indicator
                            if is_default {
                                ui.label(
                                    egui::RichText::new(t!(
                                        "effect.sticker_storage.default_indicator"
                                    ))
                                    .small()
                                    .color(ui.visuals().warn_fg_color),
                                );
                            }

                            // Show warnings
                            if file_missing {
                                ui.label(
                                    egui::RichText::new(t!("effect.sticker_storage.file_missing"))
                                        .small()
                                        .color(ui.visuals().error_fg_color),
                                );
                            } else if hash_mismatch {
                                ui.label(
                                    egui::RichText::new(t!("effect.sticker_storage.file_modified"))
                                        .small()
                                        .color(ui.visuals().warn_fg_color),
                                );
                            }
                        });
                    });
                }
            });
        }
    }
}

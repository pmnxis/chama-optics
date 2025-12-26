/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Misc Tab UI (Settings)

use crate::ChamaOptics;
use rust_i18n::t;

impl ChamaOptics {
    /// Render Tab 4: Settings
    pub(crate) fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.misc_heading"));
        ui.separator();

        egui::Grid::new("misc_grid")
            .num_columns(2)
            .spacing([20.0, 10.0])
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
                        .selectable_label(current_theme == egui::ThemePreference::System, "System")
                        .clicked()
                    {
                        ui.ctx()
                            .options_mut(|o| o.theme_preference = egui::ThemePreference::System);
                    }
                    if ui
                        .selectable_label(current_theme == egui::ThemePreference::Dark, "Dark")
                        .clicked()
                    {
                        ui.ctx()
                            .options_mut(|o| o.theme_preference = egui::ThemePreference::Dark);
                    }
                    if ui
                        .selectable_label(current_theme == egui::ThemePreference::Light, "Light")
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
            });
    }
}

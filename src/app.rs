/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::packed_image::PackedImage;
use rust_i18n::t;
use std::path::PathBuf;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ChamaOptics {
    pub pending_paths: std::collections::VecDeque<PathBuf>,
    pub import_config: crate::import_config::ImportConfig,
    pub export_config: crate::export_config::ExportConfig,
    pub lang: crate::langs::Language,

    #[serde(skip)]
    pub packed_images: Vec<PackedImage>,
}

impl Default for ChamaOptics {
    fn default() -> Self {
        Self {
            pending_paths: std::collections::VecDeque::new(),
            import_config: crate::import_config::ImportConfig::default(),
            export_config: crate::export_config::ExportConfig::default(),
            lang: crate::langs::Language::get_system(),
            packed_images: vec![],
        }
    }
}

impl ChamaOptics {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::replace_fonts(&cc.egui_ctx);

        log::info!(
            "Current support langs : {:?}",
            rust_i18n::available_locales!()
        );

        let app: ChamaOptics = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();

        app.lang.update_i18n();

        app
    }

    fn save_packed_image_all(&mut self, _ui: &mut egui::Ui) {
        // save each
        fn __save_buck_each(
            idx: usize,
            pi: &mut PackedImage,
            export_config: &crate::export_config::ExportConfig,
        ) {
            let new_path = pi.bulk_path(export_config);

            match export_config
                .theme_reg
                .selected_theme()
                .apply(pi, export_config, &new_path)
            {
                Ok(_) => {
                    log::info!("Bulk saved with EXIF overlay to {idx} {new_path:?}");
                }
                Err(e) => {
                    log::error!("Failed to save EXIF overlay: {e:?}");
                }
            }
        }

        if !self.export_config.output_name.check_folder_available(true) {
            log::error!(
                "Cannot access following directory {:?}",
                self.export_config.output_name.folder
            );
            // todo - warning on UI
        }

        if !self.export_config.output_name.remove_after_bulk_save {
            for (idx, pi) in self.packed_images.iter_mut().enumerate() {
                __save_buck_each(idx, pi, &self.export_config);
            }
        } else {
            for (idx, mut pi) in self.packed_images.drain(..).enumerate() {
                __save_buck_each(idx, &mut pi, &self.export_config);
                // todo - Multi threading and update ui continuously
            }
        }
    }

    fn update_packed_image(&mut self, ui: &mut egui::Ui) {
        let mut remove_index: Option<usize> = None;

        for (idx, pi) in self.packed_images.iter_mut().enumerate() {
            match pi.update_ui(ui, &self.export_config) {
                crate::packed_image::PackedImageEvent::None => { /* Nothing */ }
                crate::packed_image::PackedImageEvent::Remove => {
                    // todo - ordering bigger number of index, and remove later
                    remove_index = Some(idx);
                }
            }
        }

        if let Some(idx) = remove_index {
            let _ = self.packed_images.remove(idx);
        }
    }
}

impl eframe::App for ChamaOptics {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(t!("app.file_menu.root"), |ui| {
                    if ui.button(t!("app.file_menu.quit")).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                self.lang.update_menu_ui(ui);
                ui.add_space(16.0);

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(t!("app.app_name"));

            // show export configuration
            self.import_config.update_ui(ui);
            self.export_config.update_ui(ui);

            ui.separator();
            ui.horizontal(|ui| {
                ui.label(t!("app.open_files.drag_drop"));

                // add image by file open dialog
                if ui.button(t!("app.open_files.button")).clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    log::info!("By file dialog :{path:?}");
                    self.pending_paths.push_back(path);
                }
            });

            // add image by drag and drop
            ctx.input(|i| {
                if !i.raw.dropped_files.is_empty() {
                    for (idx, file) in i.raw.dropped_files.iter().enumerate() {
                        if let Some(dropped_path) = &file.path {
                            log::info!("By dropped[{idx}] : {dropped_path:?}");
                            self.pending_paths.push_back(dropped_path.clone());
                        } else {
                            log::error!("Failed to get file path");
                        }
                    }
                }
            });

            ui.separator();

            ui.heading(t!("app.images.list"));
            ui.horizontal(|ui| {
                if ui.button(t!("app.images.save_all")).clicked() {
                    self.save_packed_image_all(ui);
                }

                if ui.button(t!("app.images.remove_all")).clicked() {
                    // need Arc<RwLock<T>> later
                    self.packed_images.clear();
                }
            });

            // Scrollable stuff
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| self.update_packed_image(ui));

            ui.separator();
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            // ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            egui::warn_if_debug_build(ui);
            ui.horizontal(|ui| {
                ui.label("ChamaOptics");
                ui.add_space(60.0);
                ui.label(format!(
                    "v{} ({})",
                    env!("PROJECT_VERSION"),
                    env!("GIT_COMMIT_SHORT_HASH")
                ));
            });
            // });
        });

        // out side thread
        if let Some(popped_path) = self.pending_paths.pop_front() {
            match PackedImage::try_from_path(&popped_path, ctx) {
                Ok(mut p) => {
                    if self.import_config.get_alt_fnumber {
                        p.view_exif.replace_with_fnumber_alt_when_invalid();
                    }
                    self.packed_images.push(p);
                }
                Err(e) => {
                    log::error!("Error opening file : {e:?}");
                }
            }
        }
    }
}

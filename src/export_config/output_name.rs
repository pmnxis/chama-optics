/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use rust_i18n::t;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputName {
    pub prefix: String,
    pub postfix: String,
    pub folder: std::path::PathBuf,
    pub remove_after_bulk_save: bool,
}

impl core::default::Default for OutputName {
    fn default() -> Self {
        Self {
            prefix: "".to_owned(),
            postfix: "-OPTICS".to_owned(),
            folder: Self::default_path(),
            remove_after_bulk_save: false,
        }
    }
}

impl OutputName {
    fn default_path() -> std::path::PathBuf {
        dirs::home_dir().expect("Failed to get home directory")
    }

    pub fn check_folder_available(&self, create_if_missing: bool) -> bool {
        let folder = &self.folder;

        if folder.exists() {
            if !folder.is_dir() {
                log::error!("Path exists but is not a directory: {}", folder.display());
                return false;
            }
        } else if create_if_missing {
            if let Err(e) = std::fs::create_dir_all(folder) {
                log::error!("Failed to create folder {}: {}", folder.display(), e);
                return false;
            }
        } else {
            log::error!("Folder does not exist: {}", folder.display());
            return false;
        }

        true
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(t!("export_config.output_name.save_directory"));

            #[allow(clippy::collapsible_if)]
            if ui
                .button(t!("export_config.output_name.select_directory"))
                .clicked()
            {
                if let Some(output_path) = rfd::FileDialog::new().pick_folder() {
                    self.folder = output_path.clone();

                    if !self.check_folder_available(true) {
                        log::error!("Cannot access following directory {output_path:?}");
                        self.folder = Self::default_path();
                    }
                }
            }

            ui.label(self.folder.display().to_string());
        });

        ui.horizontal(|ui| {
            ui.label(t!("export_config.output_name.prefix"));

            ui.add(egui::TextEdit::singleline(&mut self.prefix).desired_width(100.0));

            ui.label(t!("export_config.output_name.postfix"));

            ui.add(egui::TextEdit::singleline(&mut self.postfix).desired_width(100.0));
        });

        ui.checkbox(
            &mut self.remove_after_bulk_save,
            t!("export_config.output_name.remove_after_bulk_save"),
        );
        ui.end_row();
    }
}

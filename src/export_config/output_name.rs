/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use rust_i18n::t;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct OutputName {
    pub prefix: String,
    pub postfix: String,
    pub folder: std::path::PathBuf,
    pub remove_after_bulk_save: bool,

    /// Pending async folder picker dialog (macOS drag-drop safe)
    #[serde(skip)]
    #[cfg(feature = "rfd")]
    pub(crate) pending_folder:
        Option<crate::util::async_file_dialog::PendingDialog<Option<std::path::PathBuf>>>,
}

impl Clone for OutputName {
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            postfix: self.postfix.clone(),
            folder: self.folder.clone(),
            remove_after_bulk_save: self.remove_after_bulk_save,
            #[cfg(feature = "rfd")]
            pending_folder: None,
        }
    }
}

impl PartialEq for OutputName {
    fn eq(&self, other: &Self) -> bool {
        self.prefix == other.prefix
            && self.postfix == other.postfix
            && self.folder == other.folder
            && self.remove_after_bulk_save == other.remove_after_bulk_save
    }
}

impl Eq for OutputName {}

impl PartialOrd for OutputName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OutputName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (
            &self.prefix,
            &self.postfix,
            &self.folder,
            &self.remove_after_bulk_save,
        )
            .cmp(&(
                &other.prefix,
                &other.postfix,
                &other.folder,
                &other.remove_after_bulk_save,
            ))
    }
}

impl core::default::Default for OutputName {
    fn default() -> Self {
        Self {
            prefix: "".to_owned(),
            postfix: "-OPTICS".to_owned(),
            folder: Self::default_path(),
            remove_after_bulk_save: false,
            #[cfg(feature = "rfd")]
            pending_folder: None,
        }
    }
}

impl OutputName {
    fn default_path() -> std::path::PathBuf {
        #[cfg(feature = "desktop")]
        {
            dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        }
        #[cfg(not(feature = "desktop"))]
        {
            std::path::PathBuf::from("/tmp")
        }
    }

    pub fn check_folder_available(&self, _create_if_missing: bool) -> bool {
        // WASM: no filesystem — always OK (output goes to browser download)
        #[cfg(target_arch = "wasm32")]
        {
            true
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let folder = &self.folder;

            if folder.exists() {
                if !folder.is_dir() {
                    log::error!("Path exists but is not a directory: {}", folder.display());
                    return false;
                }
            } else if _create_if_missing {
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
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        #[cfg(feature = "rfd")]
        {
            // Poll pending folder dialog
            if let Some(ref pending) = self.pending_folder
                && let Some(result) = pending.try_recv()
            {
                self.pending_folder = None;
                if let Some(output_path) = result {
                    self.folder = output_path.clone();
                    if !self.check_folder_available(true) {
                        log::error!("Cannot access following directory {output_path:?}");
                        self.folder = Self::default_path();
                    }
                }
            }
        }

        // WASM: no folder picker — files are downloaded via browser
        #[cfg(not(target_arch = "wasm32"))]
        ui.horizontal(|ui| {
            ui.label(t!("export_config.output_name.save_directory"));

            #[cfg(feature = "rfd")]
            {
                let is_pending = self.pending_folder.is_some();
                if ui
                    .add_enabled(
                        !is_pending,
                        egui::Button::new(t!("export_config.output_name.select_directory")),
                    )
                    .clicked()
                    && !is_pending
                {
                    self.pending_folder = Some(crate::util::async_file_dialog::pick_folder_async());
                }
            }

            #[cfg(all(feature = "desktop", not(feature = "rfd")))]
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

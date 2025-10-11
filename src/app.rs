/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::packed_image::PackedImage;
use std::path::PathBuf;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ChamaOptics {
    pub label: String,
    pub brightness: f32,
    pub contrast: f32,

    pub pending_paths: std::collections::VecDeque<PathBuf>,

    #[serde(skip)]
    pub packed_images: Vec<PackedImage>,
}

impl Default for ChamaOptics {
    fn default() -> Self {
        Self {
            label: "Hello World!".into(),
            brightness: 0.0,
            contrast: 0.0,
            pending_paths: std::collections::VecDeque::new(),
            packed_images: vec![],
        }
    }
}

impl ChamaOptics {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::replace_fonts(&cc.egui_ctx);

        let app: ChamaOptics = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();

        app
    }

    fn update_packed_image(&mut self, ui: &mut egui::Ui) {
        let mut remove_index: Option<usize> = None;

        for (idx, pi) in self.packed_images.iter_mut().enumerate() {
            match pi.update_ui(ui) {
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
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("ChamaOptics");

            ui.horizontal(|ui| {
                ui.label("Watermark Text : ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.separator();
            ui.label("Drag drop image here\n\n\n");

            // add image by file open dialog
            if ui.button("Open file…").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_file()
            {
                println!("By file dialog :{:?}", path);
                self.pending_paths.push_back(path);
            }

            // add image by drag and drop
            ctx.input(|i| {
                if !i.raw.dropped_files.is_empty() {
                    for (idx, file) in i.raw.dropped_files.iter().enumerate() {
                        if let Some(dropped_path) = &file.path {
                            println!("By dropped[{}] : {:?}", idx, dropped_path);
                            self.pending_paths.push_back(dropped_path.clone());
                        } else {
                            println!("Failed to get file path");
                        }
                    }
                }
            });

            ui.separator();

            ui.heading("📁 Images");
            if ui.button("Remove all").clicked() {
                // need Arc/RwLock later
                self.packed_images.clear();
            }

            // Scrollable stuff
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| self.update_packed_image(ui));

            ui.separator();
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
                ui.label("ChamaOptics");
            });
        });

        // out side thread
        if let Some(popped_path) = self.pending_paths.pop_front() {
            match PackedImage::try_from_path(&popped_path, ctx) {
                Ok(p) => {
                    self.packed_images.push(p);
                }
                Err(e) => {
                    println!("Error opening file : {e:?}");
                }
            }
        }
    }
}

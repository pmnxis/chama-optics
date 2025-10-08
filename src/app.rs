/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::packed_image::PackedImage;
use egui::TextureHandle;
use std::path::PathBuf;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ChamaOptics {
    pub label: String,
    pub brightness: f32,
    pub contrast: f32,

    pub pending_paths: std::collections::VecDeque<PathBuf>,

    #[serde(skip)]
    pub packed_images: Vec<(PackedImage, TextureHandle)>,
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

        for (idx, (pi, texture)) in self.packed_images.iter().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let orient = crate::orientation::Orientation::from_tiff(pi.orientation());
                    let (angle, _origin) = orient.egui_rotate();

                    ui.vertical(|ui| {
                        ui.label(pi.file_name());
                        ui.label(format!(
                            "{} {}  |  {} {}",
                            pi.camera_mnf(),
                            pi.camera_model(),
                            pi.lens_mnf(),
                            pi.lens_model()
                        ));
                        ui.label(format!(
                            "F {}  |  {} |  ISO {}  | {}mm",
                            pi.fnumber(),
                            pi.exposure(),
                            pi.iso_speed().map_or("".to_owned(), |x| x.to_string()),
                            pi.focal()
                        ));
                        ui.label(format!(
                            "{} [{}]  |  {}",
                            pi.orientation(),
                            angle,
                            pi.datetime()
                        ));

                        ui.horizontal(|ui| {
                            if ui.button("💾 Save").clicked() {
                                // add later
                            }

                            if ui.button("🗑 Delete").clicked() {
                                // add later
                                remove_index = Some(idx);
                            }
                        });
                    });

                    // 이미지 썸네일
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        let max_height = crate::packed_image::THUMBNAIL_MAX_HEIGHT_AS_F32;
                        let width = max_height * texture.aspect_ratio();
                        let size = egui::Vec2::new(width, max_height);

                        // let size = if !orient.is_vertical_rotated() {
                        //     let max_height = crate::packed_image::THUMBNAIL_MAX_HEIGHT_AS_F32;
                        //     let width = max_height * texture.aspect_ratio();
                        //     egui::Vec2::new(width, max_height)
                        // } else {
                        //     let max_width = crate::packed_image::THUMBNAIL_MAX_HEIGHT_AS_F32;
                        //     let height = max_width / texture.aspect_ratio();

                        //     egui::Vec2::new(height, max_width)
                        // };
                        ui.add(
                            egui::Image::from_texture(texture)
                                .rotate(angle, egui::Vec2::splat(0.5))
                                .corner_radius(2.0)
                                .fit_to_exact_size(size)
                                .maintain_aspect_ratio(true),
                            // .fit_to_fraction(size),
                        );
                    });
                });
            });
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
            match PackedImage::try_from_path(&popped_path) {
                Ok((p, thumb)) => {
                    let texture =
                        ctx.load_texture(p.file_path(), thumb, egui::TextureOptions::NEAREST);
                    self.packed_images.push((p, texture));
                }
                Err(e) => {
                    println!("Error opening file : {e:?}");
                }
            }
        }
    }
}

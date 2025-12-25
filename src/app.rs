/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::packed_image::PackedImage;
use rust_i18n::t;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ChamaOptics {
    pub pending_paths: std::collections::VecDeque<PathBuf>,
    pub import_config: crate::import_config::ImportConfig,
    pub export_config: crate::export_config::ExportConfig,
    pub lang: crate::langs::Language,

    #[serde(skip)]
    pub packed_images: Vec<PackedImage>,

    #[serde(skip)]
    pub update: crate::util::check_update::CheckRelease,

    #[serde(skip)]
    pub save_progress_current: Arc<AtomicUsize>,

    #[serde(skip)]
    pub save_progress_total: usize,

    #[serde(skip)]
    pub save_completed_time: Option<std::time::Instant>,

    #[serde(skip)]
    pub load_progress_current: Arc<AtomicUsize>,

    #[serde(skip)]
    pub load_progress_total: usize,

    #[serde(skip)]
    pub load_completed_time: Option<std::time::Instant>,
}

impl Default for ChamaOptics {
    fn default() -> Self {
        Self {
            pending_paths: std::collections::VecDeque::new(),
            import_config: crate::import_config::ImportConfig::default(),
            export_config: crate::export_config::ExportConfig::default(),
            lang: crate::langs::Language::get_system(),
            packed_images: vec![],
            update: crate::util::check_update::CheckRelease::new(),
            save_progress_current: Arc::new(AtomicUsize::new(0)),
            save_progress_total: 0,
            save_completed_time: None,
            load_progress_current: Arc::new(AtomicUsize::new(0)),
            load_progress_total: 0,
            load_completed_time: None,
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

    fn save_packed_image_all(&mut self, ui: &mut egui::Ui) {
        use rayon::prelude::*;
        use std::sync::atomic::Ordering;

        // Thread-safe struct for parallel processing
        struct SaveTask {
            path: std::path::PathBuf,
            view_exif: crate::exif_impl::SimplifiedExif,
        }

        // save each
        fn __save_bulk_each(
            idx: usize,
            task: &SaveTask,
            export_config: &crate::export_config::ExportConfig,
        ) -> Result<(), image::ImageError> {
            // Reconstruct PackedImage from path
            let pi = crate::packed_image::PackedImage::try_from_path_cli(&task.path)?;

            // Use the saved view_exif instead of reconstructed one
            let pi_with_view = crate::packed_image::PackedImage {
                view_exif: task.view_exif.clone(),
                ..pi
            };

            let new_path = pi_with_view.bulk_path(export_config);

            export_config.theme_reg.selected_theme_read().apply(
                &pi_with_view,
                export_config,
                &new_path,
            )?;

            log::info!("Bulk saved with EXIF overlay to {idx} {new_path:?}");
            Ok(())
        }

        if !self.export_config.output_name.check_folder_available(true) {
            log::error!(
                "Cannot access following directory {:?}",
                self.export_config.output_name.folder
            );
            // todo - warning on UI
        }

        // Convert PackedImages to SaveTasks for parallel processing
        let tasks: Vec<SaveTask> = self
            .packed_images
            .iter()
            .map(|pi| SaveTask {
                path: pi.path.clone(),
                view_exif: pi.view_exif.clone(),
            })
            .collect();

        let total = tasks.len();

        // Initialize progress tracking
        self.save_progress_current.store(0, Ordering::Relaxed);
        self.save_progress_total = total;

        log::info!("Starting background save of {} images", total);

        // Clone export_config for the background thread
        let clone_start = std::time::Instant::now();
        let export_config = self.export_config.clone();
        log::info!("ExportConfig clone took {:?}", clone_start.elapsed());

        // Clone the progress counter for use in parallel threads
        let progress_counter = self.save_progress_current.clone();

        // Get egui context for requesting repaint from background thread
        let ctx = ui.ctx().clone();

        // Spawn background thread for parallel processing
        std::thread::spawn(move || {
            log::info!("Background thread started");

            // Calculate optimal thread count: (CPU_COUNT / 2).max(CPU_COUNT - 3)
            let cpu_count = num_cpus::get();
            let thread_count = (cpu_count / 2).max(cpu_count.saturating_sub(3)).max(1);
            log::info!("Using {} threads out of {} CPUs", thread_count, cpu_count);

            // Configure rayon thread pool
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build()
                .unwrap();

            // Parallel processing using rayon
            pool.install(|| {
                tasks.par_iter().enumerate().for_each(|(idx, task)| {
                    log::info!("Processing image {}", idx);

                    match __save_bulk_each(idx, task, &export_config) {
                        Ok(_) => {
                            log::info!("Successfully saved image {}", idx);
                        }
                        Err(e) => {
                            log::error!("Failed to save image {}: {e:?}", idx);
                        }
                    }

                    // Update progress counter AFTER processing
                    let current = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                    log::info!("Progress: {}/{}", current, total);

                    // Request UI repaint AFTER processing to show progress
                    ctx.request_repaint();
                })
            });

            log::info!("Background thread completed");
        });

        // Schedule progress bar to disappear after a short delay
        ui.ctx().request_repaint();
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

        if self.packed_images.is_empty() {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.add_space(ui.available_height() / 16.0);

                    ui.label(
                        egui::RichText::new(t!("app.open_files.drag_drop"))
                            .font(egui::FontId::proportional(28.0))
                            .color(egui::Color32::from_rgba_unmultiplied(200, 200, 200, 50)),
                    );
                },
            );
        }
    }
}

impl eframe::App for ChamaOptics {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(t!("app.file_menu.root"), |ui| {
                    ui.set_max_width(130.00);

                    if ui.button(t!("app.file_menu.quit")).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                self.lang.update_menu_ui(ui);
                ui.add_space(16.0);

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            // Show progress bar if loading is in progress
            let load_total = self.load_progress_total;
            if load_total > 0 {
                let current = self.load_progress_current.load(Ordering::Relaxed);
                let progress = current as f32 / load_total.max(1) as f32;

                ui.vertical(|ui| {
                    if current < load_total {
                        // Still in progress - request continuous repainting
                        ui.ctx().request_repaint();

                        ui.label(format!(
                            "Loading images: {}/{} ({:.0}%)",
                            current,
                            load_total,
                            progress * 100.0
                        ));
                        ui.add(
                            egui::ProgressBar::new(progress)
                                .fill(egui::Color32::from_rgb(0, 120, 215)) // Blue color for loading
                                .show_percentage()
                                .animate(true),
                        );
                        // Clear any completion time since we're still in progress
                        self.load_completed_time = None;
                    } else {
                        // Completed
                        if self.load_completed_time.is_none() {
                            // Just completed - record the time
                            self.load_completed_time = Some(std::time::Instant::now());
                            log::info!("All images loaded! {}/{}", current, load_total);
                        }

                        let elapsed = self.load_completed_time.unwrap().elapsed();
                        if elapsed < std::time::Duration::from_secs(2) {
                            // Show completion message for 2 seconds
                            ui.label(format!("✓ Loading completed: {}/{}", current, load_total));
                            ui.add(
                                egui::ProgressBar::new(1.0)
                                    .fill(egui::Color32::from_rgb(0, 120, 215)) // Blue color
                                    .show_percentage(),
                            );
                            // Request repaint only once per frame
                            ui.ctx().request_repaint();
                        } else {
                            // Hide progress bar after 2 seconds
                            self.load_progress_total = 0;
                            self.load_progress_current.store(0, Ordering::Relaxed);
                            self.load_completed_time = None;
                        }
                    }
                });
            }

            // Show progress bar if saving is in progress
            let save_total = self.save_progress_total;
            if save_total > 0 {
                let current = self.save_progress_current.load(Ordering::Relaxed);
                let progress = current as f32 / save_total.max(1) as f32;

                ui.vertical(|ui| {
                    if current < save_total {
                        // Still in progress - request continuous repainting
                        ui.ctx().request_repaint();

                        ui.label(format!(
                            "Saving images: {}/{} ({:.0}%)",
                            current,
                            save_total,
                            progress * 100.0
                        ));
                        ui.add(
                            egui::ProgressBar::new(progress)
                                .show_percentage()
                                .animate(true),
                        );
                        // Clear any completion time since we're still in progress
                        self.save_completed_time = None;
                    } else {
                        // Completed
                        if self.save_completed_time.is_none() {
                            // Just completed - record the time
                            self.save_completed_time = Some(std::time::Instant::now());
                            log::info!("All images saved! {}/{}", current, save_total);
                        }

                        let elapsed = self.save_completed_time.unwrap().elapsed();
                        if elapsed < std::time::Duration::from_secs(2) {
                            // Show completion message for 2 seconds
                            ui.label(format!("✓ Saving completed: {}/{}", current, save_total));
                            ui.add(egui::ProgressBar::new(1.0).show_percentage());
                            // Request repaint only once per frame
                            ui.ctx().request_repaint();
                        } else {
                            // Hide progress bar after 2 seconds
                            self.save_progress_total = 0;
                            self.save_progress_current.store(0, Ordering::Relaxed);
                            self.save_completed_time = None;
                        }
                    }
                });
            }

            // Always show bottom info
            egui::warn_if_debug_build(ui);
            ui.horizontal(|ui| {
                ui.label("ChamaOptics");
                ui.add_space(20.0);
                ui.label(format!(
                    "v{} ({})",
                    env!("PROJECT_VERSION"),
                    env!("GIT_COMMIT_SHORT_HASH")
                ));
                ui.add_space(20.0);
                self.update.ui(ui);
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading(t!("app.app_name"));

            // show export configuration
            self.import_config.update_ui(ui);
            self.export_config.update_ui(ui);

            // add image by drag and drop
            ui.ctx().input(|i| {
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

            ui.horizontal(|ui| {
                ui.heading(t!("app.images.list"));

                // add image by file open dialog
                if ui.button(t!("app.open_files.button")).clicked()
                    && let Some(open_files) = rfd::FileDialog::new().pick_files()
                // && let Some(path) = rfd::FileDialog::new().pick_file()
                {
                    for (idx, file) in open_files.iter().enumerate() {
                        log::info!("By file dialog[{idx}] : {file:?}");
                        self.pending_paths.push_back(file.to_owned());
                    }
                }

                if ui.button(t!("app.images.save_all")).clicked() {
                    self.save_packed_image_all(ui);
                }

                crate::export_config::open_explorer::launch_explorer_ui(
                    ui,
                    &self.export_config.output_name.folder,
                );

                if ui.button(t!("app.images.remove_all")).clicked() {
                    // need Arc<RwLock<T>> later
                    self.packed_images.clear();
                }
            });

            // Scrollable stuff
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| self.update_packed_image(ui));
        });

        // Load one image per frame (single-core)
        if let Some(popped_path) = self.pending_paths.pop_front() {
            // Initialize load progress if this is the first image in the batch
            if self.load_progress_total == 0 {
                self.load_progress_total = self.pending_paths.len() + 1; // +1 for current image
                self.load_progress_current.store(0, Ordering::Relaxed);
                self.load_completed_time = None;
                log::info!("Started loading {} images", self.load_progress_total);
            }

            match PackedImage::try_from_path(&popped_path, ui.ctx()) {
                Ok(mut p) => {
                    if self.import_config.get_alt_fnumber {
                        p.view_exif.replace_with_fnumber_alt_when_invalid();
                    }
                    self.packed_images.push(p);

                    // Update progress
                    let current = self.load_progress_current.fetch_add(1, Ordering::Relaxed) + 1;
                    log::debug!(
                        "Loaded {}/{}: {:?}",
                        current,
                        self.load_progress_total,
                        popped_path.file_name()
                    );
                }
                Err(e) => {
                    log::error!("Error opening file : {e:?}");
                    // Still count as processed even if failed
                    self.load_progress_current.fetch_add(1, Ordering::Relaxed);
                }
            }
        } else if self.load_progress_total > 0 {
            // All images loaded, mark completion
            let current = self.load_progress_current.load(Ordering::Relaxed);
            if current >= self.load_progress_total && self.load_completed_time.is_none() {
                self.load_completed_time = Some(std::time::Instant::now());
                log::info!(
                    "All images loaded! {}/{}",
                    current,
                    self.load_progress_total
                );
            }
        }

        // Reset load progress after 2 seconds
        if let Some(completed_time) = self.load_completed_time
            && completed_time.elapsed() > std::time::Duration::from_secs(2)
        {
            self.load_progress_total = 0;
            self.load_progress_current.store(0, Ordering::Relaxed);
            self.load_completed_time = None;
        }
    }
}

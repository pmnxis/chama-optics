/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::packed_image::PackedImage;
use crate::ui_state::ProgressState;
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

    #[serde(skip)]
    pub update: crate::util::check_update::CheckRelease,

    #[serde(skip)]
    pub save_progress: ProgressState,

    #[serde(skip)]
    pub load_progress: ProgressState,

    #[serde(skip)]
    pub loaded_image_queue: crate::image::loader::LoadedImageQueue,
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
            save_progress: ProgressState::new(),
            load_progress: ProgressState::new(),
            loaded_image_queue: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
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

        // Initialize progress tracking with new ProgressState
        self.save_progress.start(total);

        log::info!("Starting background save of {} images", total);

        // Clone export_config for the background thread
        let clone_start = std::time::Instant::now();
        let export_config = self.export_config.clone();
        log::info!("ExportConfig clone took {:?}", clone_start.elapsed());

        // Clone the progress counter for use in parallel threads
        let progress_counter = self.save_progress.counter();

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
    }
}

impl eframe::App for ChamaOptics {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Render top panel using component
        crate::ui_components::render_top_panel(ui, &mut self.lang);

        // Render bottom panel using component
        crate::ui_components::render_bottom_panel(
            ui,
            &mut self.load_progress,
            &mut self.save_progress,
            &self.update,
        );

        // Render central panel
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading(t!("app.app_name"));

            self.import_config.update_ui(ui);
            self.export_config.update_ui(ui);

            ui.separator();

            // Check for dropped files
            ui.ctx().input(|i| {
                if !i.raw.dropped_files.is_empty() {
                    let paths: Vec<_> = i
                        .raw
                        .dropped_files
                        .iter()
                        .filter_map(|f| f.path.clone())
                        .collect();

                    for path in paths.iter() {
                        self.pending_paths.push_back(path.clone());
                    }
                }
            });

            ui.separator();

            // Image list controls
            ui.horizontal(|ui| {
                ui.heading(t!("app.images.list"));

                // File dialog button
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
                crate::export_config::open_explorer::launch_explorer_ui(
                    ui,
                    &self.export_config.output_name.folder,
                );

                // Remove all button
                if ui.button(t!("app.images.remove_all")).clicked() {
                    self.packed_images.clear();
                }
            });

            // Scrollable image list with background hint text
            let available_rect = ui.available_rect_before_wrap();

            // Draw background hint text if no images loaded
            if self.packed_images.is_empty() && !self.load_progress.is_active() {
                ui.painter().text(
                    available_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    t!("app.open_files.drag_drop"),
                    egui::FontId::proportional(42.0),
                    egui::Color32::from_rgba_unmultiplied(200, 200, 200, 50),
                );
            }

            // ScrollArea on top of background
            ui.allocate_ui_with_layout(
                egui::vec2(available_rect.width(), available_rect.height().max(200.0)),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| self.update_packed_image(ui));
                },
            );
        });

        // Background image loading logic (below UI rendering)
        // Start parallel loading if there are pending paths and no active loading
        if !self.pending_paths.is_empty() && !self.load_progress.is_active() {
            // Collect ALL pending paths
            let all_paths: Vec<PathBuf> = self.pending_paths.drain(..).collect();
            let total = all_paths.len();

            // Start progress tracking
            self.load_progress.start(total);

            // Spawn parallel loader with work queue
            crate::image::loader::spawn_parallel_loader(
                all_paths,
                self.import_config.get_alt_fnumber,
                self.load_progress.counter(),
                self.loaded_image_queue.clone(),
                ui.ctx().clone(),
            );
        }

        // Transfer loaded images from background thread to UI thread
        if let Ok(mut queue) = self.loaded_image_queue.try_lock()
            && !queue.is_empty()
        {
            log::info!("Transferring {} loaded images to UI", queue.len());

            // Process all loaded images from the queue
            for loaded_data in queue.drain(..) {
                log::info!("Creating packed image for {:?}", loaded_data.path);
                match crate::image::loader::create_packed_image_from_data(loaded_data, ui.ctx()) {
                    Some(packed_image) => {
                        log::info!("Successfully created packed image");
                        self.packed_images.push(packed_image);
                    }
                    None => {
                        log::error!("Failed to create packed image");
                    }
                }
            }
            log::info!("Total packed_images now: {}", self.packed_images.len());
        }
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::packed_image::PackedImage;
use crate::ui_state::ProgressState;
use std::path::PathBuf;

/// Main tab selection for the left sidebar
#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone, Copy, Debug, Default)]
pub enum MainTab {
    #[default]
    ImageList,
    ThemePreview,
    ImportExport,
    Settings,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ChamaOptics {
    pub pending_paths: std::collections::VecDeque<PathBuf>,
    pub import_config: crate::import_config::ImportConfig,
    pub export_config: crate::export_config::ExportConfig,
    pub lang: crate::langs::Language,

    /// Currently selected tab in the sidebar
    selected_tab: MainTab,

    #[serde(skip)]
    pub packed_images: Vec<PackedImage>,

    #[serde(skip)]
    /// Selected image index for theme preview tab
    pub(crate) preview_selected_index: Option<usize>,

    #[serde(skip)]
    /// Cached theme preview texture
    pub(crate) theme_preview_texture: Option<egui::TextureHandle>,

    #[serde(skip)]
    /// Last theme preview generation params (to detect when to regenerate)
    pub(crate) theme_preview_cache_key: Option<(usize, String)>, // (image_index, theme_name)

    #[serde(skip)]
    /// Background image texture for main screen
    background_texture: Option<egui::TextureHandle>,

    #[serde(skip)]
    /// Last detected theme mode (to reload texture on theme change)
    last_dark_mode: Option<bool>,

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
            selected_tab: MainTab::default(),
            packed_images: vec![],
            preview_selected_index: None,
            theme_preview_texture: None,
            theme_preview_cache_key: None,
            background_texture: None,
            last_dark_mode: None,
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

        let mut app: ChamaOptics = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();

        // Always start with ImageList tab
        app.selected_tab = MainTab::ImageList;

        app.lang.update_i18n();

        app
    }

    pub(crate) fn save_packed_image_all(&mut self, ui: &mut egui::Ui) {
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

    pub(crate) fn update_packed_image(&mut self, ui: &mut egui::Ui) {
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

    /// Handle drag and drop for file paths
    fn handle_drag_drop(&mut self, ui: &mut egui::Ui) {
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
    }

    /// Load and render background image based on theme
    fn render_background_image(&mut self, ui: &mut egui::Ui) {
        // Detect current theme (dark or light)
        let is_dark_mode = ui.ctx().global_style().visuals.dark_mode;

        // Invalidate texture if theme changed
        if self.last_dark_mode != Some(is_dark_mode) {
            self.background_texture = None;
            self.last_dark_mode = Some(is_dark_mode);
        }

        // Load appropriate background image based on theme
        if self.background_texture.is_none() {
            let image_data: &[u8] = if is_dark_mode {
                include_bytes!("../assets/dark-background.png")
            } else {
                include_bytes!("../assets/light-background.png")
            };

            if let Ok(image) = image::load_from_memory(image_data) {
                let size = [image.width() as _, image.height() as _];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                self.background_texture = Some(ui.ctx().load_texture(
                    "background",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // Draw background image with opacity at bottom-right, 25% size
        if let Some(texture) = &self.background_texture {
            let available_rect = ui.available_rect_before_wrap();
            let texture_size = texture.size_vec2();

            // Scale to 25% of original size
            let scaled_size = texture_size * 0.25;

            // Position at bottom-right
            let image_pos = egui::pos2(
                available_rect.max.x - scaled_size.x,
                available_rect.max.y - scaled_size.y,
            );

            let image_rect = egui::Rect::from_min_size(image_pos, scaled_size);

            ui.painter().image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::from_white_alpha(100), // ~39% opacity
            );
        }
    }

    /// Process pending image loading and loaded images
    fn process_image_loading(&mut self, ui: &mut egui::Ui) {
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

impl eframe::App for ChamaOptics {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Render bottom panel using component
        crate::ui_components::render_bottom_panel(
            ui,
            &mut self.load_progress,
            &mut self.save_progress,
            &self.update,
        );

        // Left sidebar with icon-only tabs
        crate::ui_components::render_tab_sidebar(ui, &mut self.selected_tab);

        // Render central panel with tab-based content
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Handle drag and drop for all tabs
            self.handle_drag_drop(ui);

            // Load and render background image based on theme
            self.render_background_image(ui);

            // Render tab content on top of background
            match self.selected_tab {
                MainTab::ImageList => self.render_image_list_tab(ui),
                MainTab::ThemePreview => self.render_theme_preview_tab(ui),
                MainTab::ImportExport => self.render_import_export_tab(ui),
                MainTab::Settings => self.render_settings_tab(ui),
            }
        });

        // Process pending image loading and loaded images
        self.process_image_loading(ui);
    }
}

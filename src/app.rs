/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::packed_image::PackedImage;
use crate::ui_state::ProgressState;
use rust_i18n::t;
use std::path::PathBuf;

/// Main tab selection for the left sidebar
#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone, Copy, Debug, Default)]
enum MainTab {
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
    preview_selected_index: Option<usize>,

    #[serde(skip)]
    /// Cached theme preview texture
    theme_preview_texture: Option<egui::TextureHandle>,

    #[serde(skip)]
    /// Last theme preview generation params (to detect when to regenerate)
    theme_preview_cache_key: Option<(usize, String)>, // (image_index, theme_name)

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

    /// Render Tab 1: Image List
    fn render_image_list_tab(&mut self, ui: &mut egui::Ui) {
        // App heading
        ui.heading(t!("app.app_name"));
        ui.separator();

        // Check for dropped files (keep drag-drop functionality)
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

        // Scrollable image list with background hint
        let available_rect = ui.available_rect_before_wrap();

        if self.packed_images.is_empty() && !self.load_progress.is_active() {
            ui.painter().text(
                available_rect.center(),
                egui::Align2::CENTER_CENTER,
                t!("app.open_files.drag_drop"),
                egui::FontId::proportional(42.0),
                egui::Color32::from_rgba_unmultiplied(200, 200, 200, 50),
            );
        }

        ui.allocate_ui_with_layout(
            egui::vec2(available_rect.width(), available_rect.height().max(200.0)),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| self.update_packed_image(ui));
            },
        );
    }

    /// Generate theme preview for the selected image
    fn generate_theme_preview(&mut self, ui_ctx: &egui::Context) -> Option<()> {
        let idx = self.preview_selected_index?;
        let pi = self.packed_images.get(idx)?;

        // Get current theme name
        let theme_name = self
            .export_config
            .theme_reg
            .selected_theme_read()
            .unique_name();

        // Check if we need to regenerate (cache invalidation)
        let cache_key = (idx, theme_name.to_string());
        if self.theme_preview_cache_key.as_ref() == Some(&cache_key) {
            // Cache is still valid
            return Some(());
        }

        // Generate preview directly in memory (no file I/O, no encode/decode)
        match self
            .export_config
            .theme_reg
            .selected_theme_read()
            .apply_to_image(pi, &self.export_config)
        {
            Ok(preview_image) => {
                // Convert DynamicImage to egui ColorImage directly
                let size = [
                    preview_image.width() as usize,
                    preview_image.height() as usize,
                ];
                let pixels = preview_image.to_rgba8().into_raw();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

                // Create texture
                let texture = ui_ctx.load_texture(
                    format!("theme_preview_{}", idx),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );

                // Update cache
                self.theme_preview_texture = Some(texture);
                self.theme_preview_cache_key = Some(cache_key);

                Some(())
            }
            Err(e) => {
                log::error!("Failed to apply theme for preview: {:?}", e);
                None
            }
        }
    }

    /// Render Tab 2: Theme Preview
    fn render_theme_preview_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.theme_preview"));
        ui.separator();

        if self.packed_images.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(t!("app.open_files.drag_drop"));
            });
            return;
        }

        // Top: Horizontal scrollable gallery of loaded images
        ui.label(t!("theme_preview.select_image"));
        egui::ScrollArea::horizontal()
            .id_salt("theme_gallery")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (idx, pi) in self.packed_images.iter().enumerate() {
                        let is_selected = self.preview_selected_index == Some(idx);

                        // Small thumbnail (80x80) with optional selection frame
                        let thumbnail_size = egui::vec2(80.0, 80.0);

                        let frame = if is_selected {
                            egui::Frame::new().stroke(egui::Stroke::new(
                                2.0,
                                egui::Color32::from_rgb(0, 150, 255),
                            ))
                        } else {
                            egui::Frame::NONE
                        };

                        let response = frame
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Image::from_texture(pi.texture.get())
                                        .fit_to_exact_size(thumbnail_size)
                                        .sense(egui::Sense::click()),
                                )
                            })
                            .inner;

                        if response.clicked() {
                            self.preview_selected_index = Some(idx);
                        }
                    }
                });
            });

        ui.separator();

        // Middle: Preview area (~50% of remaining space)
        if let Some(idx) = self.preview_selected_index {
            if idx < self.packed_images.len() {
                // Generate preview if needed
                self.generate_theme_preview(ui.ctx());

                ui.vertical(|ui| {
                    ui.label(t!("theme_preview.preview_label"));

                    let preview_height = ui.available_height() * 0.5;
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), preview_height),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            // Display the theme preview texture if available
                            if let Some(texture) = &self.theme_preview_texture {
                                let available_size = ui.available_size();
                                let texture_size = texture.size_vec2();

                                // Calculate scaling to fit within available space while maintaining aspect ratio
                                let scale = (available_size.x / texture_size.x)
                                    .min(available_size.y / texture_size.y)
                                    .min(1.0); // Don't scale up

                                let display_size = texture_size * scale;

                                ui.centered_and_justified(|ui| {
                                    ui.image(egui::ImageSource::Texture(
                                        egui::load::SizedTexture::new(texture.id(), display_size),
                                    ));
                                });
                            } else {
                                // Show loading message
                                ui.centered_and_justified(|ui| {
                                    ui.spinner();
                                    ui.label("Generating preview...");
                                });
                            }
                        },
                    );
                });

                ui.separator();
            }
        } else if !self.packed_images.is_empty() {
            // Auto-select first image if none selected
            self.preview_selected_index = Some(0);
        }

        // Bottom: Theme parameters (export_config theme settings)
        egui::ScrollArea::vertical()
            .id_salt("theme_params")
            .show(ui, |ui| {
                ui.label(t!("theme_preview.theme_settings"));
                self.export_config.theme_reg.update_ui(ui);
            });
    }

    /// Render Tab 3: Import/Export Config
    fn render_import_export_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.import_export"));
        ui.separator();

        // Import config section
        self.import_config.update_ui(ui);

        ui.add_space(10.0);

        // Export config section
        self.export_config.update_ui(ui);
    }

    /// Render Tab 4: Settings
    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.settings"));
        ui.separator();

        // Version information
        ui.group(|ui| {
            ui.label(t!("settings.version_info"));
            ui.horizontal(|ui| {
                ui.label("ChamaOptics");
                ui.label(format!(
                    "v{} ({})",
                    env!("PROJECT_VERSION"),
                    env!("GIT_COMMIT_SHORT_HASH")
                ));
            });

            ui.add_space(10.0);
            self.update.ui(ui);
        });

        ui.add_space(10.0);

        // Language settings
        ui.group(|ui| {
            ui.label(t!("settings.language"));
            self.lang.update_menu_ui(ui);
        });
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

        // Left sidebar with icon-only tabs
        egui::Panel::left("tab_sidebar")
            .resizable(false)
            .exact_size(50.0) // Narrow for icon-only
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);

                    // Tab 1: Image List (☰)
                    if ui
                        .selectable_label(
                            self.selected_tab == MainTab::ImageList,
                            "☰", // Unicode: U+2630 TRIGRAM FOR HEAVEN
                        )
                        .on_hover_text(t!("tabs.image_list"))
                        .clicked()
                    {
                        self.selected_tab = MainTab::ImageList;
                    }

                    ui.add_space(5.0);

                    // Tab 2: Theme Preview (▦)
                    if ui
                        .selectable_label(
                            self.selected_tab == MainTab::ThemePreview,
                            "▦", // Unicode: U+25A6 SQUARE WITH ORTHOGONAL CROSSHATCH
                        )
                        .on_hover_text(t!("tabs.theme_preview"))
                        .clicked()
                    {
                        self.selected_tab = MainTab::ThemePreview;
                    }

                    ui.add_space(5.0);

                    // Tab 3: Import/Export (⚙)
                    if ui
                        .selectable_label(
                            self.selected_tab == MainTab::ImportExport,
                            "⚙", // Unicode: U+2699 GEAR
                        )
                        .on_hover_text(t!("tabs.import_export"))
                        .clicked()
                    {
                        self.selected_tab = MainTab::ImportExport;
                    }

                    ui.add_space(5.0);

                    // Tab 4: Settings (⋮)
                    if ui
                        .selectable_label(
                            self.selected_tab == MainTab::Settings,
                            "⋮", // Unicode: U+22EE VERTICAL ELLIPSIS
                        )
                        .on_hover_text(t!("tabs.settings"))
                        .clicked()
                    {
                        self.selected_tab = MainTab::Settings;
                    }
                });
            });

        // Render central panel with tab-based content
        egui::CentralPanel::default().show_inside(ui, |ui| match self.selected_tab {
            MainTab::ImageList => self.render_image_list_tab(ui),
            MainTab::ThemePreview => self.render_theme_preview_tab(ui),
            MainTab::ImportExport => self.render_import_export_tab(ui),
            MainTab::Settings => self.render_settings_tab(ui),
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

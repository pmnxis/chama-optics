/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */

//! Color Tab UI - LUT-based color grading with split-view preview

use crate::effect::lut_storage::LutUiAction;
use crate::ChamaOptics;
use rust_i18n::t;

impl ChamaOptics {
    /// Generate color preview textures (original and LUT-applied)
    pub(crate) fn generate_color_preview(&mut self, ui_ctx: &egui::Context) -> Option<()> {
        let idx = self.color_selected_index?;
        let packed_image = self.packed_images.get(idx)?;

        // Check if we need to regenerate (cache invalidation)
        let cache_key = (idx, self.lut_storage.selected_lut_id);
        if self.color_preview_cache_key.as_ref() == Some(&cache_key) {
            // Cache is still valid
            return Some(());
        }

        // Load original image
        let image_path = packed_image.path.clone();
        let original_image = match image::open(&image_path) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image {:?}: {:?}", image_path, e);
                return None;
            }
        };

        // Calculate preview size (max 1920px dimension for performance)
        let max_preview_size = 1920u32;
        let (orig_w, orig_h) = (original_image.width(), original_image.height());
        let scale = if orig_w > max_preview_size || orig_h > max_preview_size {
            let scale_w = max_preview_size as f32 / orig_w as f32;
            let scale_h = max_preview_size as f32 / orig_h as f32;
            scale_w.min(scale_h)
        } else {
            1.0
        };

        let preview_image = if scale < 1.0 {
            let new_w = (orig_w as f32 * scale) as u32;
            let new_h = (orig_h as f32 * scale) as u32;
            original_image.resize(new_w, new_h, image::imageops::FilterType::Triangle)
        } else {
            original_image
        };

        // Create original texture
        let original_rgba = preview_image.to_rgba8();
        let size = [original_rgba.width() as usize, original_rgba.height() as usize];
        let original_color_image =
            egui::ColorImage::from_rgba_unmultiplied(size, &original_rgba);
        let original_texture = ui_ctx.load_texture(
            format!("color_original_{}", idx),
            original_color_image,
            egui::TextureOptions::LINEAR,
        );
        self.color_original_texture = Some(original_texture);

        // Create LUT-applied texture (if LUT is selected)
        if self.lut_storage.selected_lut_id.is_some() {
            let mut lut_image = image::DynamicImage::ImageRgba8(original_rgba.clone());
            self.lut_storage.apply_selected_lut(&mut lut_image);

            let lut_rgba = lut_image.to_rgba8();
            let lut_color_image =
                egui::ColorImage::from_rgba_unmultiplied(size, &lut_rgba);
            let lut_texture = ui_ctx.load_texture(
                format!("color_lut_{}", idx),
                lut_color_image,
                egui::TextureOptions::LINEAR,
            );
            self.color_lut_texture = Some(lut_texture);
        } else {
            // No LUT selected - show same as original
            self.color_lut_texture = self.color_original_texture.clone();
        }

        self.color_preview_cache_key = Some(cache_key);
        Some(())
    }

    /// Render the Color tab with split-view preview
    pub(crate) fn render_color_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.color_heading", default = "Color Grading"));
        ui.separator();

        // Show image gallery only if images exist
        if !self.packed_images.is_empty() {
            // Top: Horizontal scrollable gallery of loaded images
            ui.label(t!("color.select_image", default = "Select Image"));

            let current_selected = self.color_selected_index;

            use crate::ui_components::render_horizontal_gallery;

            let image_to_delete = render_horizontal_gallery(
                ui,
                self.packed_images.iter().enumerate(),
                |(idx, _img)| *idx,
                |(_idx, img)| {
                    img.path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                },
                |_ctx, (_idx, img)| Some(img.texture.get().clone()),
                |idx| current_selected == Some(idx),
                None::<fn(&_) -> bool>,
                None::<fn(&_) -> Option<(bool, bool)>>,
                &mut |idx| {
                    self.color_selected_index = Some(idx);
                    // Invalidate cache when selection changes
                    self.color_preview_cache_key = None;
                },
                Some(&mut |idx| {
                    log::info!("Delete button clicked for image index {}", idx);
                }),
            );

            // Handle deletion outside of function call to avoid borrow conflicts
            if let Some(idx) = image_to_delete {
                self.delete_image_by_index(idx);
            }

            ui.separator();

            // Middle: Split-view preview area
            if let Some(idx) = self.color_selected_index {
                if idx < self.packed_images.len() {
                    // Generate preview if needed
                    self.generate_color_preview(ui.ctx());

                    // Header with refresh button
                    ui.horizontal(|ui| {
                        ui.label(t!("color.preview_label", default = "Preview"));

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .button(t!("color.refresh_button", default = "Refresh"))
                                    .clicked()
                                {
                                    // Invalidate cache to force regeneration
                                    self.color_preview_cache_key = None;
                                    self.color_original_texture = None;
                                    self.color_lut_texture = None;
                                    log::info!("Color preview cache invalidated by user");
                                }
                            },
                        );
                    });

                    // Split-view: Original (left) | LUT Applied (right)
                    let preview_height = ui.available_height() * 0.5;
                    let total_width = ui.available_width();
                    let half_width = (total_width - 10.0) / 2.0; // 10px gap between

                    ui.horizontal(|ui| {
                        // Left side: Original image
                        ui.allocate_ui_with_layout(
                            egui::vec2(half_width, preview_height),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.group(|ui| {
                                    ui.label(
                                        egui::RichText::new(t!(
                                            "color.original",
                                            default = "Original"
                                        ))
                                        .strong(),
                                    );

                                    self.render_preview_image(
                                        ui,
                                        &self.color_original_texture.clone(),
                                    );
                                });
                            },
                        );

                        ui.add_space(5.0);

                        // Right side: LUT-applied image
                        ui.allocate_ui_with_layout(
                            egui::vec2(half_width, preview_height),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                ui.group(|ui| {
                                    let lut_label = if self.lut_storage.selected_lut_id.is_some() {
                                        self.lut_storage
                                            .get_selected_lut()
                                            .map(|l| l.name.as_str())
                                            .unwrap_or("LUT Applied")
                                    } else {
                                        "No LUT"
                                    };
                                    ui.label(egui::RichText::new(lut_label).strong());

                                    self.render_preview_image(
                                        ui,
                                        &self.color_lut_texture.clone(),
                                    );
                                });
                            },
                        );
                    });

                    ui.separator();
                }
            } else if !self.packed_images.is_empty() {
                // Auto-select first image if none selected
                self.color_selected_index = Some(0);
            }
        } else {
            // Show placeholder when no images loaded
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(t!("color.no_images", default = "No images loaded"))
                        .size(14.0)
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(10.0);
            });
            ui.separator();
        }

        // LUT settings section
        egui::ScrollArea::vertical()
            .id_salt("color_settings")
            .show(ui, |ui| {
                ui.label(t!("color.lut_settings", default = "LUT Settings"));

                // LUT selection and management UI
                let action = self.lut_storage.update_ui(ui);

                // Handle LUT UI actions
                if action == LutUiAction::OpenAddDialog {
                    self.open_lut_file_dialog();
                }

                // Invalidate cache when LUT selection changes
                // (The combo box selection is handled inside update_ui)

                ui.add_space(10.0);
                ui.separator();

                // Future: Color adjustments panel (collapsed/placeholder)
                ui.collapsing(
                    t!(
                        "color.adjustments_section",
                        default = "Color Adjustments (Coming Soon)"
                    ),
                    |ui| {
                        self.color_adjustments.update_ui(ui);
                    },
                );
            });
    }

    /// Helper to render a preview image within available space
    fn render_preview_image(&self, ui: &mut egui::Ui, texture: &Option<egui::TextureHandle>) {
        if let Some(texture) = texture {
            let available_size = ui.available_size();
            let texture_size = texture.size_vec2();

            // Calculate scaling to fit within available space while maintaining aspect ratio
            let scale = (available_size.x / texture_size.x)
                .min(available_size.y / texture_size.y)
                .min(1.0); // Don't scale up

            let display_size = texture_size * scale;

            ui.centered_and_justified(|ui| {
                ui.image(egui::ImageSource::Texture(egui::load::SizedTexture::new(
                    texture.id(),
                    display_size,
                )));
            });
        } else {
            // Show loading spinner
            ui.centered_and_justified(|ui| {
                ui.spinner();
                ui.label(t!("color.generating", default = "Processing..."));
            });
        }
    }

    /// Open file dialog to add a LUT file
    fn open_lut_file_dialog(&mut self) {
        use rfd::FileDialog;

        let file = FileDialog::new()
            .add_filter("CUBE LUT", &["cube", "CUBE"])
            .set_title(t!("color.select_lut_file", default = "Select LUT File"))
            .pick_file();

        if let Some(path) = file {
            // Extract name from filename (without extension)
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unnamed LUT")
                .to_string();

            match self.lut_storage.add_lut(name.clone(), &path) {
                Ok(id) => {
                    log::info!("Successfully added LUT: {} (id: {})", name, id);
                    // Auto-select the newly added LUT
                    self.lut_storage.selected_lut_id = Some(id);
                    // Invalidate preview cache
                    self.color_preview_cache_key = None;
                }
                Err(e) => {
                    log::error!("Failed to add LUT: {:?}", e);
                }
            }
        }
    }
}

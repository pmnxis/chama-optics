/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Theme Preview Tab UI

use crate::ChamaOptics;
use rust_i18n::t;

impl ChamaOptics {
    /// Generate theme preview for the selected image
    pub(crate) fn generate_theme_preview(&mut self, ui_ctx: &egui::Context) -> Option<()> {
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
    pub(crate) fn render_theme_preview_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.theme_preview_heading"));
        ui.separator();

        // Show image gallery only if images exist
        if !self.packed_images.is_empty() {
            // Top: Horizontal scrollable gallery of loaded images
            ui.label(t!("theme_preview.select_image"));

            let current_selected = self.preview_selected_index;

            use crate::ui_components::render_horizontal_gallery;

            let _image_to_delete = render_horizontal_gallery(
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
                    self.preview_selected_index = Some(idx);
                },
                None, // No delete in theme preview tab
            );

            ui.separator();

            // Middle: Preview area (~50% of remaining space)
            if let Some(idx) = self.preview_selected_index {
                if idx < self.packed_images.len() {
                    // Generate preview if needed
                    self.generate_theme_preview(ui.ctx());

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(t!("theme_preview.preview_label"));

                            // Right-align refresh button in preview area
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // Add refresh button to force preview regeneration
                                    if ui.button(t!("theme_preview.refresh_button")).clicked() {
                                        // Invalidate cache to force regeneration
                                        self.theme_preview_cache_key = None;
                                        self.theme_preview_texture = None;
                                        log::info!("Preview cache invalidated by user");
                                    }
                                },
                            );
                        });

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
                                            egui::load::SizedTexture::new(
                                                texture.id(),
                                                display_size,
                                            ),
                                        ));
                                    });
                                } else {
                                    // Show loading message
                                    ui.centered_and_justified(|ui| {
                                        ui.spinner();
                                        ui.label(t!("theme_preview.generating"));
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
        } else {
            // Show placeholder when no images loaded
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(t!("theme_preview.no_images"))
                        .size(14.0)
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(10.0);
            });
            ui.separator();
        }

        // Always show theme parameters at the bottom
        egui::ScrollArea::vertical()
            .id_salt("theme_params")
            .show(ui, |ui| {
                ui.label(t!("theme_preview.theme_settings"));

                self.export_config
                    .theme_reg
                    .update_ui(ui, self.show_theme_name_in_english);
            });
    }
}

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
            let mut image_to_delete: Option<usize> = None;

            // Fixed height gallery to prevent layout shift during loading
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), 120.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("theme_gallery")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (idx, pi) in self.packed_images.iter().enumerate() {
                                    let is_selected = self.preview_selected_index == Some(idx);

                                    // Container for thumbnail + filename (fixed width to prevent layout shift)
                                    let container_response = ui.allocate_ui_with_layout(
                                        egui::vec2(80.0, 100.0),
                                        egui::Layout::top_down(egui::Align::Center),
                                        |ui| {
                                            // Thumbnail (80x80) with optional selection frame
                                            let thumbnail_size = egui::vec2(80.0, 80.0);

                                            let frame = if is_selected {
                                                egui::Frame::new().stroke(egui::Stroke::new(
                                                    2.0,
                                                    egui::Color32::from_rgb(0, 150, 255),
                                                ))
                                            } else {
                                                egui::Frame::NONE
                                            };

                                            let image_response = frame
                                                .show(ui, |ui| {
                                                    ui.add(
                                                        egui::Image::from_texture(pi.texture.get())
                                                            .fit_to_exact_size(thumbnail_size)
                                                            .sense(egui::Sense::click_and_drag()),
                                                    )
                                                })
                                                .inner;

                                            // File name (small text, centered, max width 80px)
                                            let file_name = pi
                                                .path
                                                .file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy();

                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(file_name.as_ref())
                                                        .size(10.0)
                                                        .color(ui.visuals().weak_text_color()),
                                                )
                                                .truncate(),
                                            )
                                            .on_hover_text(file_name.as_ref());

                                            image_response
                                        },
                                    );

                                    let image_response = container_response.inner;
                                    let rect = image_response.rect;

                                    // Check if mouse is hovering over image
                                    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                                    let is_hovered = if let Some(pos) = pointer_pos {
                                        rect.contains(pos)
                                    } else {
                                        false
                                    };

                                    // Delete button on hover (top-right corner)
                                    if is_hovered {
                                        let button_size = 20.0;
                                        let delete_button_rect = egui::Rect::from_min_size(
                                            rect.right_top() + egui::vec2(-button_size, 0.0),
                                            egui::vec2(button_size, button_size),
                                        );

                                        // Draw delete button visuals (red circle with X)
                                        let center = delete_button_rect.center();
                                        ui.painter().circle_filled(
                                            center,
                                            10.0,
                                            egui::Color32::from_rgba_premultiplied(
                                                220, 50, 50, 220,
                                            ),
                                        );

                                        // Draw X using lines
                                        let x_size = 5.0;
                                        ui.painter().line_segment(
                                            [
                                                center + egui::vec2(-x_size, -x_size),
                                                center + egui::vec2(x_size, x_size),
                                            ],
                                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                                        );
                                        ui.painter().line_segment(
                                            [
                                                center + egui::vec2(x_size, -x_size),
                                                center + egui::vec2(-x_size, x_size),
                                            ],
                                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                                        );

                                        // Check if delete button was clicked
                                        if let Some(pos) = pointer_pos {
                                            if delete_button_rect.contains(pos)
                                                && image_response.clicked()
                                            {
                                                image_to_delete = Some(idx);
                                            } else if image_response.clicked() {
                                                self.preview_selected_index = Some(idx);
                                            }
                                        }
                                    } else if image_response.clicked() {
                                        self.preview_selected_index = Some(idx);
                                    }

                                    ui.add_space(5.0);
                                }
                            });
                        });
                },
            );

            // Delete image if requested
            if let Some(idx) = image_to_delete {
                self.packed_images.remove(idx);
                // Adjust preview_selected_index if necessary
                if let Some(selected) = self.preview_selected_index {
                    if selected == idx {
                        // Deleted the selected image, clear selection and cache
                        self.preview_selected_index = None;
                        self.theme_preview_texture = None;
                        self.theme_preview_cache_key = None;
                    } else if selected > idx {
                        // Selected image moved down, update index and invalidate cache
                        self.preview_selected_index = Some(selected - 1);
                        self.theme_preview_cache_key = None; // Force regeneration
                    }
                }
            }

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

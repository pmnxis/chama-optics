/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Sticker Tab UI - Manage sticker storage and settings

use crate::ChamaOptics;
use rust_i18n::t;

#[cfg(feature = "rfd")]
use rfd;

impl ChamaOptics {
    /// Render Sticker management tab
    pub(crate) fn render_sticker_tab(&mut self, ui: &mut egui::Ui) {
        // Wrap entire tab content in scrollable area
        egui::ScrollArea::vertical()
            .id_salt("sticker_tab_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading(t!("effect.sticker_storage.title"));
                ui.separator();

                // Verify stickers on load (check for hash mismatches)
                let mut has_hash_warnings = false;
                for sticker in &self.sticker_storage.stickers {
                    if sticker.hash_mismatch {
                        has_hash_warnings = true;
                        break;
                    }
                }

                // Show hash mismatch warning
                if has_hash_warnings {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t!("effect.sticker_storage.hash_warning"),
                    );
                    ui.add_space(5.0);
                }

                // Note: Sticker management (add/delete/set default) is now integrated into preview below
                ui.separator();

                // Sticker application settings
                ui.heading(t!("effect.sticker_storage.settings"));
                ui.add_space(5.0);

                egui::Grid::new("sticker_settings_grid")
                    .num_columns(2)
                    .spacing([4.0, 3.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Scale
                        ui.label(t!("effect.sticker_storage.scale"));
                        ui.add(egui::Slider::new(&mut self.sticker_config.scale, 0.5..=2.0));
                        ui.end_row();

                        // Offset X
                        ui.label(t!("effect.sticker_storage.offset_x"));
                        ui.add(egui::Slider::new(
                            &mut self.sticker_config.offset_x,
                            -100..=100,
                        ));
                        ui.end_row();

                        // Offset Y
                        ui.label(t!("effect.sticker_storage.offset_y"));
                        ui.add(egui::Slider::new(
                            &mut self.sticker_config.offset_y,
                            -100..=100,
                        ));
                        ui.end_row();

                        ui.separator();
                        ui.end_row();

                        // Mosaic and Stroke settings (merged into Sticker settings)
                        ui.heading(t!("effect.default_effect.mosaic_and_stroke_settings"));
                        ui.end_row();

                        // Mosaic block size
                        ui.label(t!("effect.default_effect.mosaic_block_size"));
                        ui.add(egui::Slider::new(&mut self.mosaic_block_size, 2..=400))
                            .on_hover_text(t!("effect.default_effect.mosaic_block_size_hint"));
                        ui.end_row();

                        ui.label(t!("effect.default_effect.stroke_color"));
                        ui.horizontal(|ui| {
                            egui::color_picker::color_edit_button_srgba(
                                ui,
                                &mut self.stroke_color,
                                egui::color_picker::Alpha::BlendOrAdditive,
                            )
                        });
                        ui.end_row();

                        // Border thickness
                        ui.label(t!("effect.default_effect.stroke_border_thickness"));
                        ui.add(egui::Slider::new(&mut self.stroke_thickness, 1..=20))
                            .on_hover_text("Thickness of stroke border in pixels");
                        ui.end_row();
                    });
                ui.add_space(5.0);

                // Sticker preview section with images
                if !self.sticker_storage.stickers.is_empty() {
                    ui.separator();
                    ui.heading(t!("effect.sticker_storage.preview"));
                    ui.add_space(10.0);

                    // Sticker gallery - horizontal scrollable thumbnails at top
                    // Clone sticker data to avoid borrow checker issues
                    let stickers: Vec<_> = self.sticker_storage.stickers.to_vec();
                    let default_sticker_id = self.sticker_storage.default_sticker_id;

                    egui::ScrollArea::horizontal()
                        .id_salt("sticker_gallery")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for sticker in &stickers {
                                    let is_selected = self.selected_sticker_id == Some(sticker.id);
                                    let is_default = default_sticker_id == Some(sticker.id);

                                    ui.allocate_ui_with_layout(
                                        egui::vec2(80.0, 100.0),
                                        egui::Layout::top_down(egui::Align::Center),
                                        |ui| {
                                            // Thumbnail (80x80)
                                            let thumbnail_size = egui::vec2(80.0, 80.0);

                                            let frame = if is_selected {
                                                egui::Frame::new().stroke(egui::Stroke::new(
                                                    2.0,
                                                    egui::Color32::from_rgb(0, 150, 255),
                                                ))
                                            } else {
                                                egui::Frame::NONE
                                            };

                                            let texture = self
                                                .sticker_storage
                                                .get_texture(ui.ctx(), sticker.id);

                                            let image_response = if let Some(texture) = texture {
                                                frame
                                                    .show(ui, |ui| {
                                                        ui.add(
                                                            egui::Image::from_texture(&texture)
                                                                .fit_to_exact_size(thumbnail_size)
                                                                .sense(egui::Sense::click()),
                                                        )
                                                    })
                                                    .inner
                                            } else {
                                                let (response, _painter) = ui.allocate_painter(
                                                    thumbnail_size,
                                                    egui::Sense::click(),
                                                );
                                                ui.painter().text(
                                                    response.rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    "📷",
                                                    egui::FontId::proportional(32.0),
                                                    ui.visuals().weak_text_color(),
                                                );
                                                response
                                            };

                                            // File name
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&sticker.name)
                                                        .size(10.0)
                                                        .color(ui.visuals().weak_text_color()),
                                                )
                                                .truncate(),
                                            )
                                            .on_hover_text(&sticker.name);

                                            // Default indicator
                                            if is_default {
                                                ui.label(
                                                    egui::RichText::new("⭐")
                                                        .small()
                                                        .color(ui.visuals().warn_fg_color),
                                                );
                                            }

                                            // Warnings
                                            if sticker.file_missing {
                                                ui.label(
                                                    egui::RichText::new("❌")
                                                        .size(12.0)
                                                        .color(ui.visuals().error_fg_color),
                                                );
                                            } else if sticker.hash_mismatch {
                                                ui.label(
                                                    egui::RichText::new("⚠️")
                                                        .size(12.0)
                                                        .color(ui.visuals().warn_fg_color),
                                                );
                                            }

                                            // Handle click
                                            if image_response.clicked() {
                                                self.selected_sticker_id = Some(sticker.id);
                                            }

                                            // Delete button on hover
                                            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                                            if let Some(pos) = pointer_pos
                                                && image_response.rect.contains(pos)
                                            {
                                                let button_size = 20.0;
                                                let delete_button_rect = egui::Rect::from_min_size(
                                                    image_response.rect.right_top()
                                                        + egui::vec2(-button_size, 0.0),
                                                    egui::vec2(button_size, button_size),
                                                );

                                                // Draw delete button
                                                let center = delete_button_rect.center();
                                                ui.painter().circle_filled(
                                                    center,
                                                    10.0,
                                                    egui::Color32::from_rgba_premultiplied(
                                                        220, 50, 50, 220,
                                                    ),
                                                );
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

                                                // Check if delete button clicked
                                                if delete_button_rect.contains(pos)
                                                    && image_response.clicked()
                                                {
                                                    self.sticker_storage.remove_sticker(sticker.id);
                                                    if self.selected_sticker_id == Some(sticker.id)
                                                    {
                                                        self.selected_sticker_id = None;
                                                    }
                                                }
                                            }
                                        },
                                    );
                                    ui.add_space(5.0);
                                }
                            });
                        });

                    ui.separator();

                    // Large preview area for selected sticker
                    if let Some(selected_id) = self.selected_sticker_id {
                        if let Some(selected_sticker) =
                            self.sticker_storage.get_sticker(selected_id)
                        {
                            // Clone data we need to avoid borrow checker issues
                            let sticker_name = selected_sticker.name.clone();
                            let is_default =
                                self.sticker_storage.default_sticker_id == Some(selected_id);
                            let file_missing = selected_sticker.file_missing;
                            let hash_mismatch = selected_sticker.hash_mismatch;

                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(t!("effect.sticker_storage.selected_sticker"));
                                    ui.label(egui::RichText::new(&sticker_name).strong());

                                    // Right-align action buttons
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            // Delete button (always visible in preview)
                                            if ui
                                                .button("🗑")
                                                .on_hover_text(t!("effect.sticker_storage.delete"))
                                                .clicked()
                                            {
                                                self.sticker_storage.remove_sticker(selected_id);
                                                self.selected_sticker_id = None;
                                            }

                                            // Set as default button
                                            if !is_default
                                                && ui
                                                    .button("⭐")
                                                    .on_hover_text(t!(
                                                        "effect.sticker_storage.set_default"
                                                    ))
                                                    .clicked()
                                            {
                                                self.sticker_storage.default_sticker_id =
                                                    Some(selected_id);
                                            }

                                            // Default indicator
                                            if is_default {
                                                ui.label(
                                                    egui::RichText::new(t!(
                                                        "effect.sticker_storage.default_indicator"
                                                    ))
                                                    .small()
                                                    .color(ui.visuals().warn_fg_color),
                                                );
                                            }
                                        },
                                    );
                                });

                                let preview_height = ui.available_height() * 0.6;
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), preview_height),
                                    egui::Layout::top_down(egui::Align::Center),
                                    |ui| {
                                        // Display selected sticker texture if available
                                        if let Some(texture) =
                                            self.sticker_storage.get_texture(ui.ctx(), selected_id)
                                        {
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
                                                if file_missing {
                                                    ui.label(
                                                        egui::RichText::new("❌")
                                                            .size(32.0)
                                                            .color(ui.visuals().error_fg_color),
                                                    );
                                                    ui.label(t!(
                                                        "effect.sticker_storage.file_missing"
                                                    ));
                                                } else if hash_mismatch {
                                                    ui.label(
                                                        egui::RichText::new("⚠️")
                                                            .size(32.0)
                                                            .color(ui.visuals().warn_fg_color),
                                                    );
                                                    ui.label(t!(
                                                        "effect.sticker_storage.file_modified"
                                                    ));
                                                } else {
                                                    ui.spinner();
                                                    ui.label(t!("theme_preview.generating"));
                                                }
                                            });
                                        }
                                    },
                                );
                            });
                        }
                    } else if !self.sticker_storage.stickers.is_empty() {
                        // Auto-select first sticker if none selected
                        if let Some(first_sticker) = self.sticker_storage.stickers.first() {
                            self.selected_sticker_id = Some(first_sticker.id);
                        }
                    }
                }

                // Add sticker button
                ui.add_space(10.0);
                #[cfg(feature = "rfd")]
                if ui
                    .button(t!("effect.sticker_storage.add_sticker"))
                    .clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
                        .pick_file()
                {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Sticker")
                        .to_string();

                    match self.sticker_storage.add_sticker(name.clone(), &path) {
                        Ok(_) => {
                            log::info!("Added sticker: {}", name);
                        }
                        Err(e) => {
                            log::error!("Failed to add sticker: {}", e);
                        }
                    }
                }
            });
    }
}

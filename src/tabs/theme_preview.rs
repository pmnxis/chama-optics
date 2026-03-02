/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Theme Preview Tab UI

use crate::ChamaOptics;
use rust_i18n::t;

impl ChamaOptics {
    /// Generate theme preview for selected image
    /// Pipeline: Load → Face Effects (mosaic/stroke/sticker) → Color Adj → LUT → Theme
    pub(crate) fn generate_theme_preview(&mut self, ui_ctx: &egui::Context) -> Option<()> {
        use crate::effect::FaceEffectMode;
        use crate::effect::mosaic::MosaicEffect;
        use crate::effect::stroke::StrokeEffect;

        let idx = self.preview_selected_index?;

        // Extract path, theme_name, and lut_id before borrowing
        let packed_image = self.packed_images.get(idx)?;
        let image_path = packed_image.path.clone();
        let image_lut_id = packed_image.lut_id;
        let theme_name = self
            .export_config
            .theme_reg
            .selected_theme_read()
            .unique_name();

        // Hash-based cache key (includes all pipeline parameters)
        let cache_key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::hash::DefaultHasher::new();
            Hash::hash(&theme_name, &mut hasher);
            Hash::hash(&image_lut_id, &mut hasher);
            Hash::hash(&self.detected_faces.len(), &mut hasher);
            Hash::hash(&self.mosaic_block_size, &mut hasher);
            Hash::hash(&self.stroke_thickness, &mut hasher);
            let sc = self.stroke_color;
            Hash::hash(&[sc.r(), sc.g(), sc.b(), sc.a()], &mut hasher);
            Hash::hash(&self.color_adjustments.enabled, &mut hasher);
            Hash::hash(&self.color_adjustments.exposure.to_bits(), &mut hasher);
            Hash::hash(&self.color_adjustments.contrast, &mut hasher);
            Hash::hash(&self.color_adjustments.highlights, &mut hasher);
            Hash::hash(&self.color_adjustments.shadows, &mut hasher);
            Hash::hash(&self.color_adjustments.whites, &mut hasher);
            Hash::hash(&self.color_adjustments.blacks, &mut hasher);
            Hash::hash(&self.color_adjustments.clarity, &mut hasher);
            Hash::hash(&self.color_adjustments.vibrance, &mut hasher);
            Hash::hash(&self.color_adjustments.saturation, &mut hasher);
            for face in &self.detected_faces {
                Hash::hash(&face.effect_mode, &mut hasher);
                Hash::hash(&face.sticker_id, &mut hasher);
            }
            (idx, hasher.finish())
        };
        if self.theme_preview_cache_key.as_ref() == Some(&cache_key) {
            return Some(());
        }

        // Generate preview with full pipeline
        let preview_result = if !self.detected_faces.is_empty() {
            log::info!(
                "Theme preview: Generating with full pipeline for index {}",
                idx
            );

            // Load original image
            let mut dyn_image = match self.packed_images[idx].get_image() {
                Ok((img, _)) => img,
                Err(e) => {
                    log::error!("Failed to load image {:?}: {:?}", image_path, e);
                    return None;
                }
            };

            // Step 1: Apply mosaic/stroke face effects
            let mut mosaic_faces: Vec<(i32, i32, u32, u32)> = vec![];
            let mut stroke_faces: Vec<(i32, i32, u32, u32)> = vec![];

            for face in &self.detected_faces {
                let face_tuple = (face.x, face.y, face.width, face.height);
                match face.effect_mode {
                    FaceEffectMode::None | FaceEffectMode::Sticker => {}
                    FaceEffectMode::Mosaic => {
                        mosaic_faces.push(face_tuple);
                    }
                    FaceEffectMode::Stroke => {
                        stroke_faces.push(face_tuple);
                    }
                    FaceEffectMode::MosaicStroke => {
                        mosaic_faces.push(face_tuple);
                        stroke_faces.push(face_tuple);
                    }
                }
            }

            if !mosaic_faces.is_empty() {
                let mosaic_config = MosaicEffect {
                    block_size: self.mosaic_block_size,
                    intensity: 1.0,
                };
                let _ = MosaicEffect::apply(&mut dyn_image, &mosaic_faces, &mosaic_config);
            }

            if !stroke_faces.is_empty() {
                let border_rgba = crate::theme::color32_to_rgba(self.stroke_color);
                let stroke_config = StrokeEffect {
                    thickness: self.stroke_thickness,
                    color: (
                        border_rgba[0],
                        border_rgba[1],
                        border_rgba[2],
                        border_rgba[3],
                    ),
                };
                let _ = StrokeEffect::apply(&mut dyn_image, &stroke_faces, &stroke_config);
            }

            // Step 2: Apply stickers
            let sticker_processed_image = {
                let mut img_with_stickers = dyn_image.clone();

                for face in &self.detected_faces {
                    if let Some(sticker_id) = face.sticker_id
                        && let Some(sticker_img) =
                            self.sticker_storage.get_sticker_image(sticker_id)
                    {
                        let sticker_aspect =
                            sticker_img.width() as f32 / sticker_img.height() as f32;
                        let face_aspect = face.width as f32 / face.height as f32;

                        let scaled_face_w = face.width as f32 * self.sticker_config.scale;
                        let scaled_face_h = face.height as f32 * self.sticker_config.scale;

                        let (sticker_w, sticker_h) = if sticker_aspect > face_aspect {
                            (
                                scaled_face_w as u32,
                                (scaled_face_w / sticker_aspect) as u32,
                            )
                        } else {
                            (
                                (scaled_face_h * sticker_aspect) as u32,
                                scaled_face_h as u32,
                            )
                        };

                        let resized_sticker = sticker_img.resize(
                            sticker_w,
                            sticker_h,
                            image::imageops::FilterType::Lanczos3,
                        );

                        let face_center_x = face.x as f32 + face.width as f32 / 2.0;
                        let face_center_y = face.y as f32 + face.height as f32 / 2.0;

                        let offset_pixel_x =
                            sticker_w as f32 * self.sticker_config.offset_x as f32 / 100.0;
                        let offset_pixel_y =
                            sticker_h as f32 * self.sticker_config.offset_y as f32 / 100.0;

                        let sticker_center_x = face_center_x + offset_pixel_x;
                        let sticker_center_y = face_center_y + offset_pixel_y;

                        let sticker_x = (sticker_center_x - sticker_w as f32 / 2.0) as i64;
                        let sticker_y = (sticker_center_y - sticker_h as f32 / 2.0) as i64;

                        image::imageops::overlay(
                            &mut img_with_stickers,
                            &resized_sticker,
                            sticker_x,
                            sticker_y,
                        );
                    }
                }

                Some(img_with_stickers)
            };

            // Step 3: Apply color adjustments
            let processed_image = if let Some(mut img) = sticker_processed_image {
                if !self.color_adjustments.is_identity() {
                    self.color_adjustments.apply(&mut img);
                }
                img
            } else {
                let mut img = dyn_image;
                if !self.color_adjustments.is_identity() {
                    self.color_adjustments.apply(&mut img);
                }
                img
            };

            // Step 4: Apply LUT
            let mut final_image = processed_image;
            if let Some(lut_id) = image_lut_id {
                log::info!("Theme preview: Applying LUT {:?} to image", lut_id);
                self.lut_storage
                    .apply_lut_to_image(lut_id, &mut final_image);
            }

            // Save to sticker_bytes for theme to use
            let original_ext = image_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("jpg");

            let format = match original_ext.to_lowercase().as_str() {
                "png" => image::ImageFormat::Png,
                "heic" | "heif" => image::ImageFormat::Jpeg,
                _ => image::ImageFormat::Jpeg,
            };

            let mut bytes = Vec::new();
            if final_image
                .write_to(&mut std::io::Cursor::new(&mut bytes), format)
                .is_ok()
            {
                let bytes_len = bytes.len();
                if let Some(packed_img) = self.packed_images.get_mut(idx) {
                    packed_img.sticker_bytes = Some(bytes);
                    log::info!(
                        "Updated sticker_bytes in PackedImage[{}]: {} bytes (full pipeline)",
                        idx,
                        bytes_len
                    );
                }
            }

            // Step 5: Apply theme
            match self.packed_images.get(idx) {
                Some(pi) => self
                    .export_config
                    .theme_reg
                    .selected_theme_read()
                    .apply_to_image(pi, &self.export_config),
                None => Err(image::ImageError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Image not found",
                ))),
            }
        } else {
            // No faces: apply Color Adj → LUT → Theme
            log::info!(
                "Theme preview: No faces, applying color adj + LUT + theme for index {}",
                idx
            );

            let needs_processing = !self.color_adjustments.is_identity() || image_lut_id.is_some();

            if needs_processing {
                let mut dyn_image = match self.packed_images[idx].get_image() {
                    Ok((img, _)) => img,
                    Err(e) => {
                        log::error!("Failed to load image {:?}: {:?}", image_path, e);
                        return None;
                    }
                };

                // Apply color adjustments
                if !self.color_adjustments.is_identity() {
                    self.color_adjustments.apply(&mut dyn_image);
                }

                // Apply LUT
                if let Some(lut_id) = image_lut_id {
                    log::info!(
                        "Theme preview: Applying LUT {:?} to image (no faces)",
                        lut_id
                    );
                    self.lut_storage.apply_lut_to_image(lut_id, &mut dyn_image);
                }

                // Save to sticker_bytes
                let original_ext = image_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("jpg");

                let format = match original_ext.to_lowercase().as_str() {
                    "png" => image::ImageFormat::Png,
                    "heic" | "heif" => image::ImageFormat::Jpeg,
                    _ => image::ImageFormat::Jpeg,
                };

                let mut bytes = Vec::new();
                if dyn_image
                    .write_to(&mut std::io::Cursor::new(&mut bytes), format)
                    .is_ok()
                    && let Some(packed_img) = self.packed_images.get_mut(idx)
                {
                    packed_img.sticker_bytes = Some(bytes);
                    log::info!(
                        "Updated sticker_bytes in PackedImage[{}] with processed image",
                        idx
                    );
                }
            }

            match self.packed_images.get(idx) {
                Some(pi) => self
                    .export_config
                    .theme_reg
                    .selected_theme_read()
                    .apply_to_image(pi, &self.export_config),
                None => Err(image::ImageError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Image not found",
                ))),
            }
        };

        match preview_result {
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

            let image_to_delete = render_horizontal_gallery(
                ui,
                self.packed_images.iter().enumerate(),
                |(idx, _img)| *idx,
                |(_idx, img)| {
                    crate::util::normalize_for_display(
                        img.path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                    )
                },
                |_ctx, (_idx, img)| Some(img.texture.get().clone()),
                |idx| current_selected == Some(idx),
                None::<fn(&_) -> bool>,
                None::<fn(&_) -> Option<(bool, bool)>>,
                &mut |idx| {
                    self.preview_selected_index = Some(idx);
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
                                // Display's theme preview texture if available
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

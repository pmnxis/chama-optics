/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Edit Tab UI — Unified editing: Color Adjustments + LUT + Crop/Rotate + Decoration (Theme/Cheki)
//!
//! Merges the functionality of the former Color, ThemePreview, and Cheki tabs
//! into a single 2-panel layout with a shared preview canvas.

use crate::ChamaOptics;
use crate::app::DecorationMode;
use rust_i18n::t;

impl ChamaOptics {
    /// Render the unified Edit tab
    pub(crate) fn render_edit_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.edit", default = "Edit"));
        ui.separator();

        if self.packed_images.is_empty() {
            // Show placeholder when no images loaded
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(t!("color.no_images", default = "No images loaded"))
                        .size(14.0)
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new(t!(
                        "color.drop_images_hint",
                        default = "Drop images here to add"
                    ))
                    .size(12.0)
                    .italics()
                    .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(10.0);
            });
            return;
        }

        // Auto-select first image if none selected
        if self.edit_selected_index.is_none() {
            self.edit_selected_index = Some(0);
        }

        // Top: Horizontal scrollable gallery of loaded images
        ui.label(t!("color.select_image", default = "Select Image"));

        let current_selected = self.edit_selected_index;

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
            // Show indicator for images with LUT configured
            Some(|item: &(usize, &crate::packed_image::PackedImage)| item.1.lut_id.is_some()),
            None::<fn(&_) -> Option<(bool, bool)>>,
            &mut |idx| {
                self.edit_selected_index = Some(idx);
                // Invalidate cache when selection changes
                self.edit_preview_cache_key = None;
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

        // 2-Panel layout: Preview (left) | Controls (right)
        if let Some(idx) = self.edit_selected_index
            && idx < self.packed_images.len()
        {
            let available_width = ui.available_width();
            let controls_width = available_width * 0.40;

            // Generate preview for None/Theme modes (Cheki uses its own texture)
            if self.decoration_mode != DecorationMode::Cheki {
                self.generate_edit_preview(ui.ctx());
            }

            // Poll pending LUT file picker dialog (shared with Color tab)
            #[cfg(feature = "rfd")]
            if let Some(ref pending) = self.pending_lut_pick
                && let Some(result) = pending.try_recv()
            {
                self.pending_lut_pick = None;
                if let Some(path) = result {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unnamed LUT")
                        .to_string();

                    match self.lut_storage.add_lut(name.clone(), &path) {
                        Ok(id) => {
                            log::info!("Successfully added LUT: {} (id: {})", name, id);
                            if let Some(pi) = self.packed_images.get_mut(idx) {
                                pi.lut_id = Some(id);
                                log::info!("Auto-assigned new LUT to image index {}", idx);
                            }
                            self.edit_preview_cache_key = None;
                        }
                        Err(e) => {
                            log::error!("Failed to add LUT: {:?}", e);
                        }
                    }
                }
            }

            // Right panel (side panel): Controls — rendered first so the remainder goes to preview
            egui::Panel::right("edit_controls_panel")
                .resizable(true)
                .default_size(controls_width)
                .size_range(200.0..=available_width * 0.6)
                .show_inside(ui, |ui| {
                    self.render_edit_controls_panel(ui, idx);
                });

            // Left panel: Preview canvas (fills remaining space)
            self.render_edit_preview_panel(ui, idx);
        }
    }

    /// Render the left preview panel
    fn render_edit_preview_panel(&mut self, ui: &mut egui::Ui, idx: usize) {
        match self.decoration_mode {
            DecorationMode::Cheki => {
                // Cheki mode: interactive canvas with border/stickers/text
                self.render_edit_cheki_preview(ui, idx);
            }
            _ => {
                // None/Theme mode: standard preview image
                self.render_edit_standard_preview(ui);
            }
        }
    }

    /// Render standard (non-Cheki) preview image
    fn render_edit_standard_preview(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            // Preview header
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(t!("color.preview_label", default = "Preview")).strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(t!("color.refresh_button", default = "Refresh"))
                        .clicked()
                    {
                        self.edit_preview_cache_key = None;
                        self.edit_preview_texture = None;
                        log::info!("Edit preview cache invalidated by user");
                    }
                });
            });

            let available_size = ui.available_size();
            let preview_height = available_size.y.min(500.0);

            ui.allocate_ui_with_layout(
                egui::vec2(available_size.x, preview_height),
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    if let Some(texture) = &self.edit_preview_texture {
                        let texture_size = texture.size_vec2();
                        let scale = (available_size.x / texture_size.x)
                            .min(preview_height / texture_size.y)
                            .min(1.0);
                        let display_size = texture_size * scale;
                        ui.image(egui::ImageSource::Texture(egui::load::SizedTexture::new(
                            texture.id(),
                            display_size,
                        )));
                    } else {
                        ui.spinner();
                        ui.label(t!(
                            "theme_preview.generating",
                            default = "Generating preview..."
                        ));
                    }
                },
            );
        });
    }

    /// Render Cheki interactive canvas in the Edit tab preview panel
    fn render_edit_cheki_preview(&mut self, ui: &mut egui::Ui, idx: usize) {
        let Some(packed_image) = self.packed_images.get(idx) else {
            return;
        };
        let image_uuid = packed_image.uuid;

        // Ensure decoration exists for this image
        self.cheki_decorations.entry(image_uuid).or_default();

        // edit_selected_index is already set by the caller (render_edit_tab)
        // Generate/process base texture
        self.process_cheki_base_texture(ui.ctx());
        self.start_cheki_base_texture_generation();

        // Render interactive canvas
        let available = ui.available_size();
        let canvas_height = (available.y * 0.85).max(200.0);

        ui.allocate_ui(egui::vec2(available.x, canvas_height), |ui| {
            self.render_cheki_canvas(ui, image_uuid);
        });
    }

    /// Render the right controls panel (scrollable)
    fn render_edit_controls_panel(&mut self, ui: &mut egui::Ui, idx: usize) {
        egui::ScrollArea::vertical()
            .id_salt("edit_controls")
            .show(ui, |ui| {
                // Section 1: Color Adjustments
                let adjustments_before = self.color_adjustments.clone();
                ui.collapsing(
                    t!("color.adjustments_section", default = "Color Adjustments"),
                    |ui| {
                        self.color_adjustments.update_ui(ui);
                    },
                );
                // Invalidate caches if color adjustments changed
                if self.color_adjustments != adjustments_before {
                    self.edit_preview_cache_key = None;
                    self.detection_preview_cache_key = None;
                }

                ui.add_space(5.0);
                ui.separator();

                // Section 2: LUT Settings
                ui.collapsing(t!("color.lut_settings", default = "LUT Settings"), |ui| {
                    let action = self.render_per_image_lut_ui_for_edit(ui, idx);
                    if action == crate::effect::lut_storage::LutUiAction::OpenAddDialog {
                        self.spawn_lut_file_dialog();
                    }
                });

                ui.add_space(5.0);
                ui.separator();

                // Section 3: Crop & Rotate
                // edit_selected_index is already set by the caller
                ui.collapsing(
                    t!("color.crop_rotate_section", default = "Crop & Rotate"),
                    |ui| {
                        self.render_crop_rotate_ui(ui);
                    },
                );

                ui.add_space(5.0);
                ui.separator();

                // Section 4: Decoration (None / Theme / Cheki)
                ui.label(
                    egui::RichText::new(t!("edit.decoration", default = "Decoration")).strong(),
                );
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.decoration_mode == DecorationMode::None,
                            t!("edit.decoration_none", default = "None"),
                        )
                        .clicked()
                    {
                        self.decoration_mode = DecorationMode::None;
                        self.edit_preview_cache_key = None;
                    }
                    if ui
                        .selectable_label(
                            self.decoration_mode == DecorationMode::Theme,
                            t!("edit.decoration_theme", default = "Theme"),
                        )
                        .clicked()
                    {
                        self.decoration_mode = DecorationMode::Theme;
                        self.edit_preview_cache_key = None;
                    }
                    if ui
                        .selectable_label(
                            self.decoration_mode == DecorationMode::Cheki,
                            t!("edit.decoration_cheki", default = "Cheki"),
                        )
                        .clicked()
                    {
                        self.decoration_mode = DecorationMode::Cheki;
                        self.edit_preview_cache_key = None;
                    }
                });

                ui.add_space(5.0);

                // Decoration-specific controls
                match self.decoration_mode {
                    DecorationMode::None => {}
                    DecorationMode::Theme => {
                        // Theme selection UI
                        self.export_config
                            .theme_reg
                            .update_ui(ui, self.show_theme_name_in_english);
                    }
                    DecorationMode::Cheki => {
                        // Cheki decoration controls
                        if let Some(packed_image) = self.packed_images.get(idx) {
                            let image_uuid = packed_image.uuid;
                            self.cheki_decorations.entry(image_uuid).or_default();
                            self.render_cheki_controls(ui, image_uuid);
                        }
                    }
                }
            });
    }

    /// LUT UI adapted for Edit tab (edit_selected_index is already set by caller)
    fn render_per_image_lut_ui_for_edit(
        &mut self,
        ui: &mut egui::Ui,
        _idx: usize,
    ) -> crate::effect::lut_storage::LutUiAction {
        use crate::effect::lut_storage::LutUiAction;

        // edit_selected_index is already set, render_per_image_lut_ui uses it directly
        let action = self.render_per_image_lut_ui(ui);

        // If LUT changed, invalidate edit preview cache
        if action != LutUiAction::None {
            self.edit_preview_cache_key = None;
        }

        action
    }

    /// Generate unified Edit tab preview
    /// Pipeline: Load → Orientation → Crop/Rotate → Face Effects → Color Adj → LUT → [Theme]
    /// For DecorationMode::None: stops after LUT
    /// For DecorationMode::Theme: applies theme after LUT
    /// For DecorationMode::Cheki: handled separately (render_edit_cheki_preview)
    fn generate_edit_preview(&mut self, ui_ctx: &egui::Context) -> Option<()> {
        use crate::effect::FaceEffectMode;
        use crate::effect::mosaic::MosaicEffect;
        use crate::effect::stroke::StrokeEffect;

        let idx = self.edit_selected_index?;
        let packed_image = self.packed_images.get(idx)?;
        let image_path = packed_image.path.clone();
        let image_lut_id = packed_image.lut_id;
        let crop_rotate = packed_image.crop_rotate.clone();

        // Build hash-based cache key
        let cache_key = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::hash::DefaultHasher::new();
            Hash::hash(&idx, &mut hasher);
            Hash::hash(&self.decoration_mode, &mut hasher);

            // Theme name (only relevant for Theme mode, but include always for consistency)
            let theme_name = self
                .export_config
                .theme_reg
                .selected_theme_read()
                .unique_name();
            Hash::hash(&theme_name, &mut hasher);

            // LUT
            Hash::hash(&image_lut_id, &mut hasher);

            // Crop/Rotate
            Hash::hash(&crop_rotate.content_hash(), &mut hasher);

            // Face effects
            Hash::hash(&self.detected_faces.len(), &mut hasher);
            Hash::hash(&self.mosaic_block_size, &mut hasher);
            Hash::hash(&self.stroke_thickness, &mut hasher);
            let sc = self.stroke_color;
            Hash::hash(&[sc.r(), sc.g(), sc.b(), sc.a()], &mut hasher);

            // Color adjustments
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

            // Per-face state
            for face in &self.detected_faces {
                Hash::hash(&face.effect_mode, &mut hasher);
                Hash::hash(&face.sticker_id, &mut hasher);
            }

            (idx, hasher.finish())
        };

        if self.edit_preview_cache_key.as_ref() == Some(&cache_key) {
            return Some(());
        }

        // Load image
        let (mut dyn_image, need_orientation) = match self.packed_images[idx].get_image() {
            Ok(result) => result,
            Err(e) => {
                log::error!(
                    "Edit preview: Failed to load image {:?}: {:?}",
                    image_path,
                    e
                );
                return None;
            }
        };

        // Step 1: Apply orientation
        if need_orientation {
            let orientation = self.packed_images[idx].view_exif.orientation;
            dyn_image.apply_orientation(orientation);
        }

        // Step 2: Apply crop/rotate
        if !crop_rotate.is_identity() {
            dyn_image = crop_rotate.apply(&dyn_image);
        }

        // Step 3: Apply face effects (mosaic/stroke/sticker)
        if !self.detected_faces.is_empty() {
            let mut mosaic_faces: Vec<(i32, i32, u32, u32)> = vec![];
            let mut stroke_faces: Vec<(i32, i32, u32, u32)> = vec![];

            for face in &self.detected_faces {
                let face_tuple = (face.x, face.y, face.width, face.height);
                match face.effect_mode {
                    FaceEffectMode::None | FaceEffectMode::Sticker => {}
                    FaceEffectMode::Mosaic => mosaic_faces.push(face_tuple),
                    FaceEffectMode::Stroke => stroke_faces.push(face_tuple),
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

            // Apply stickers
            for face in &self.detected_faces {
                if let Some(sticker_id) = face.sticker_id
                    && let Some(sticker_img) = self.sticker_storage.get_sticker_image(sticker_id)
                {
                    let sticker_aspect = sticker_img.width() as f32 / sticker_img.height() as f32;
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

                    let sticker_x =
                        (face_center_x + offset_pixel_x - sticker_w as f32 / 2.0) as i64;
                    let sticker_y =
                        (face_center_y + offset_pixel_y - sticker_h as f32 / 2.0) as i64;

                    image::imageops::overlay(
                        &mut dyn_image,
                        &resized_sticker,
                        sticker_x,
                        sticker_y,
                    );
                }
            }
        }

        // Step 4: Apply color adjustments
        if !self.color_adjustments.is_identity() {
            self.color_adjustments.apply(&mut dyn_image);
        }

        // Step 5: Apply LUT
        if let Some(lut_id) = image_lut_id {
            self.lut_storage.apply_lut_to_image(lut_id, &mut dyn_image);
        }

        // Step 6: Apply theme (if Theme mode)
        let final_image = if self.decoration_mode == DecorationMode::Theme {
            // Save processed image to sticker_bytes for theme to use
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
            }

            // Apply theme
            match self.packed_images.get(idx) {
                Some(pi) => match self
                    .export_config
                    .theme_reg
                    .selected_theme_read()
                    .apply_to_image(pi, &self.export_config)
                {
                    Ok(themed) => themed,
                    Err(e) => {
                        log::error!("Edit preview: Failed to apply theme: {:?}", e);
                        dyn_image
                    }
                },
                None => dyn_image,
            }
        } else {
            // DecorationMode::None — just show the processed image
            dyn_image
        };

        // Downscale for preview
        let max_preview_size = 1920u32;
        let mut preview = final_image;
        if preview.width() > max_preview_size || preview.height() > max_preview_size {
            let scale = (max_preview_size as f32 / preview.width() as f32)
                .min(max_preview_size as f32 / preview.height() as f32);
            let new_w = (preview.width() as f32 * scale) as u32;
            let new_h = (preview.height() as f32 * scale) as u32;
            preview = preview.resize(new_w, new_h, image::imageops::FilterType::Triangle);
        }

        // Convert to texture
        let rgba = preview.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
        let texture = ui_ctx.load_texture(
            format!("edit_preview_{}", idx),
            color_image,
            egui::TextureOptions::LINEAR,
        );

        self.edit_preview_texture = Some(texture);
        self.edit_preview_cache_key = Some(cache_key);

        Some(())
    }
}

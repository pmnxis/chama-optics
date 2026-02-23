/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */

//! Crop/Rotate & LUT helpers used by the unified Edit tab.
//!
//! Contains: crop canvas generation, crop/rotate UI, LUT gallery,
//! per-image LUT selection, and LUT file dialog.

use crate::ChamaOptics;
use crate::effect::lut_storage::{LutItem, LutUiAction, StoredLutType};
use rust_i18n::t;
use uuid::Uuid;

impl ChamaOptics {
    // generate_color_preview and render_color_tab removed — superseded by Edit tab
    // Remaining functions: crop/rotate, LUT gallery, LUT file dialog (used by Edit tab)

    /// Start async crop canvas texture generation (background thread)
    pub(crate) fn start_crop_canvas_generation(&mut self) -> Option<()> {
        let idx = self.edit_selected_index?;
        let packed_image = self.packed_images.get(idx)?;

        let cache_key = (
            idx,
            packed_image.crop_rotate.rotation_90_count,
            format!("{:.1}", packed_image.crop_rotate.rotation_degrees),
        );
        if self.crop_canvas_cache_key.as_ref() == Some(&cache_key) {
            return Some(());
        }

        // Set cache key immediately to prevent duplicate spawns
        self.crop_canvas_cache_key = Some(cache_key);

        let orientation = packed_image.view_exif.orientation;
        let rotation_90 = packed_image.crop_rotate.rotation_90_count;
        let rotation_deg = packed_image.crop_rotate.rotation_degrees;
        let queue = self.crop_preview_queue.clone();

        // Shared processing logic
        let process_image = move |mut img: image::DynamicImage,
                                  need_orientation: bool|
              -> Option<(egui::ColorImage, usize, (u32, u32))> {
            if need_orientation {
                img.apply_orientation(orientation);
            }

            match rotation_90 % 4 {
                1 => img = img.rotate90(),
                2 => img = img.rotate180(),
                3 => img = img.rotate270(),
                _ => {}
            }

            if rotation_deg.abs() > 0.01 {
                use image::Rgba;
                use imageproc::geometric_transformations::{Interpolation, rotate_about_center};
                let rgba = img.to_rgba8();
                let radians = rotation_deg.to_radians();
                let rotated = rotate_about_center(
                    &rgba,
                    radians,
                    Interpolation::Bilinear,
                    Rgba([0, 0, 0, 0]),
                );
                img = image::DynamicImage::ImageRgba8(rotated);
            }

            let orig_size = (img.width(), img.height());

            let max_size = 1920u32;
            if img.width() > max_size || img.height() > max_size {
                let scale = (max_size as f32 / img.width() as f32)
                    .min(max_size as f32 / img.height() as f32);
                let new_w = (img.width() as f32 * scale) as u32;
                let new_h = (img.height() as f32 * scale) as u32;
                img = img.resize(new_w, new_h, image::imageops::FilterType::Triangle);
            }

            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
            Some((color_image, idx, orig_size))
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let image_path = packed_image.path.clone();
            std::thread::spawn(move || {
                let load_result = {
                    let file = match std::fs::File::open(&image_path) {
                        Ok(f) => f,
                        Err(e) => {
                            log::error!("Failed to open image for crop canvas: {:?}", e);
                            return;
                        }
                    };
                    let mut buf_reader = std::io::BufReader::new(file);
                    crate::image::common::__load_image(&image_path, &mut buf_reader)
                };
                let (img, need_orientation) = match load_result {
                    Ok(result) => result,
                    Err(e) => {
                        log::error!("Failed to load image for crop canvas: {:?}", e);
                        return;
                    }
                };

                if let Some(result) = process_image(img, need_orientation)
                    && let Ok(mut q) = queue.lock()
                {
                    *q = Some(result);
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            let img = packed_image
                .image_bytes
                .as_ref()
                .and_then(|bytes| image::load_from_memory(bytes).ok());
            if let Some(img) = img {
                if let Some(result) = process_image(img, true)
                    && let Ok(mut q) = queue.lock()
                {
                    *q = Some(result);
                }
            } else {
                log::error!("WASM: No image_bytes for crop canvas preview");
            }
        }

        Some(())
    }

    /// Process crop canvas preview from background thread queue
    pub(crate) fn process_crop_preview(&mut self, ui_ctx: &egui::Context) {
        if let Ok(mut queue) = self.crop_preview_queue.try_lock()
            && let Some((color_image, idx, orig_size)) = queue.take()
        {
            let still_relevant = self.edit_selected_index == Some(idx);
            if still_relevant {
                let texture = ui_ctx.load_texture(
                    format!("crop_canvas_{}", idx),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                self.crop_canvas_texture = Some(texture);
                self.crop_canvas_original_size = Some(orig_size);
            }
        }
    }

    /// Render crop/rotate interactive canvas and controls
    pub(crate) fn render_crop_rotate_ui(&mut self, ui: &mut egui::Ui) {
        let Some(idx) = self.edit_selected_index else {
            ui.label(t!(
                "color.select_image_first",
                default = "Select an image first"
            ));
            return;
        };

        if self.packed_images.get(idx).is_none() {
            return;
        }

        // Generate crop canvas texture (async) and process results
        self.process_crop_preview(ui.ctx());
        self.start_crop_canvas_generation();

        // Canvas area
        let canvas_height = (ui.available_height() * 0.55).max(200.0);
        ui.allocate_ui(egui::vec2(ui.available_width(), canvas_height), |ui| {
            self.render_crop_canvas(ui, idx);
        });

        ui.add_space(5.0);

        // Controls below canvas
        let mut rotation_changed = false;
        let mut crop_changed = false;

        if let Some(packed_image) = self.packed_images.get_mut(idx) {
            ui.horizontal(|ui| {
                ui.label(t!("color.rotation", default = "Rotation"));
                if ui
                    .add(
                        egui::Slider::new(
                            &mut packed_image.crop_rotate.rotation_degrees,
                            -45.0..=45.0,
                        )
                        .suffix("°")
                        .step_by(0.5),
                    )
                    .changed()
                {
                    rotation_changed = true;
                }
            });

            ui.horizontal(|ui| {
                if ui
                    .button(t!("color.rotate_ccw", default = "↺ CCW"))
                    .clicked()
                {
                    packed_image.crop_rotate.rotation_90_count =
                        (packed_image.crop_rotate.rotation_90_count + 3) % 4;
                    rotation_changed = true;
                }
                if ui.button(t!("color.rotate_cw", default = "↻ CW")).clicked() {
                    packed_image.crop_rotate.rotation_90_count =
                        (packed_image.crop_rotate.rotation_90_count + 1) % 4;
                    rotation_changed = true;
                }
                ui.separator();
                if packed_image.crop_rotate.crop_rect.is_none() {
                    if ui
                        .button(t!("color.enable_crop", default = "Enable Crop"))
                        .clicked()
                    {
                        packed_image.crop_rotate.crop_rect =
                            Some(crate::effect::crop_rotate::NormalizedRect {
                                x: 0.1,
                                y: 0.1,
                                width: 0.8,
                                height: 0.8,
                            });
                        crop_changed = true;
                    }
                } else if ui
                    .button(t!("color.clear_crop", default = "Clear Crop"))
                    .clicked()
                {
                    packed_image.crop_rotate.crop_rect = None;
                    crop_changed = true;
                }
                if ui
                    .button(t!("color.reset_crop_rotate", default = "Reset All"))
                    .clicked()
                {
                    packed_image.crop_rotate =
                        crate::effect::crop_rotate::CropRotateTransform::default();
                    rotation_changed = true;
                    crop_changed = true;
                }
            });
        }

        if rotation_changed {
            self.crop_canvas_cache_key = None;
            self.edit_preview_cache_key = None;
        }
        if crop_changed {
            self.edit_preview_cache_key = None;
        }
    }

    /// Render interactive crop canvas with image and crop overlay
    fn render_crop_canvas(&mut self, ui: &mut egui::Ui, idx: usize) {
        let Some(texture) = self.crop_canvas_texture.as_ref() else {
            ui.centered_and_justified(|ui| {
                ui.label(t!("color.loading_preview", default = "Loading preview..."));
            });
            return;
        };
        let texture = texture.clone();
        let texture_size = texture.size_vec2();
        let available_size = ui.available_size();

        // Fit image to available space
        let aspect = texture_size.x / texture_size.y;
        let image_display_size = if available_size.x / aspect > available_size.y {
            egui::vec2(available_size.y * aspect, available_size.y)
        } else {
            egui::vec2(available_size.x, available_size.x / aspect)
        };

        // Allocate interactive area
        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());
        let viewport_rect = response.rect;

        // Center image in viewport
        let offset = (available_size - image_display_size) / 2.0;
        let image_rect = egui::Rect::from_min_size(
            egui::pos2(
                viewport_rect.min.x + offset.x,
                viewport_rect.min.y + offset.y,
            ),
            image_display_size,
        );

        // Draw the rotated image
        painter.image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // Draw crop overlay if crop is enabled
        let crop_rect_data = self
            .packed_images
            .get(idx)
            .and_then(|pi| pi.crop_rotate.crop_rect.clone());

        if let Some(ref crop) = crop_rect_data {
            // Convert normalized crop to screen coordinates
            let crop_screen = egui::Rect::from_min_size(
                egui::pos2(
                    image_rect.min.x + crop.x * image_rect.width(),
                    image_rect.min.y + crop.y * image_rect.height(),
                ),
                egui::vec2(
                    crop.width * image_rect.width(),
                    crop.height * image_rect.height(),
                ),
            );

            // Draw dimming overlay (4 rectangles around crop area)
            let dim_color = egui::Color32::from_black_alpha(128);
            // Top strip
            painter.rect_filled(
                egui::Rect::from_min_max(
                    image_rect.min,
                    egui::pos2(image_rect.max.x, crop_screen.min.y),
                ),
                0.0,
                dim_color,
            );
            // Bottom strip
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(image_rect.min.x, crop_screen.max.y),
                    image_rect.max,
                ),
                0.0,
                dim_color,
            );
            // Left strip
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(image_rect.min.x, crop_screen.min.y),
                    egui::pos2(crop_screen.min.x, crop_screen.max.y),
                ),
                0.0,
                dim_color,
            );
            // Right strip
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(crop_screen.max.x, crop_screen.min.y),
                    egui::pos2(image_rect.max.x, crop_screen.max.y),
                ),
                0.0,
                dim_color,
            );

            // Draw crop rectangle border
            painter.rect_stroke(
                crop_screen,
                0.0,
                (2.0, egui::Color32::WHITE),
                egui::StrokeKind::Inside,
            );

            // Draw corner resize handles
            const HANDLE_SIZE: f32 = 8.0;
            let corners = [
                crop_screen.min,
                egui::pos2(crop_screen.max.x, crop_screen.min.y),
                egui::pos2(crop_screen.min.x, crop_screen.max.y),
                crop_screen.max,
            ];
            for corner in corners {
                let handle_rect =
                    egui::Rect::from_center_size(corner, egui::vec2(HANDLE_SIZE, HANDLE_SIZE));
                painter.rect_filled(handle_rect, 0.0, egui::Color32::WHITE);
                painter.rect_stroke(
                    handle_rect,
                    0.0,
                    (1.0, egui::Color32::BLACK),
                    egui::StrokeKind::Inside,
                );
            }

            // Handle crop interactions
            self.handle_crop_interactions(response, image_rect, idx);
        } else {
            // No crop — show hint
            painter.text(
                image_rect.center(),
                egui::Align2::CENTER_CENTER,
                t!(
                    "color.crop_hint",
                    default = "Click 'Enable Crop' to add a crop area"
                ),
                egui::FontId::proportional(14.0),
                egui::Color32::from_white_alpha(180),
            );
        }
    }

    /// Handle interactive crop rectangle drag/resize
    fn handle_crop_interactions(
        &mut self,
        response: egui::Response,
        image_rect: egui::Rect,
        idx: usize,
    ) {
        use crate::app::{CropInteractionState, ResizeCorner};

        let mouse_pos = response.hover_pos();

        // On drag stop → finalize and invalidate caches
        if response.drag_stopped() {
            self.crop_interaction_state = CropInteractionState::Idle;
            self.edit_preview_cache_key = None;
            return;
        }

        match self.crop_interaction_state.clone() {
            CropInteractionState::Idle => {
                if response.drag_started()
                    && let Some(pos) = mouse_pos
                    && let Some(crop) = self
                        .packed_images
                        .get(idx)
                        .and_then(|pi| pi.crop_rotate.crop_rect.clone())
                {
                    let crop_screen = egui::Rect::from_min_size(
                        egui::pos2(
                            image_rect.min.x + crop.x * image_rect.width(),
                            image_rect.min.y + crop.y * image_rect.height(),
                        ),
                        egui::vec2(
                            crop.width * image_rect.width(),
                            crop.height * image_rect.height(),
                        ),
                    );

                    // Check corner handles first (10px threshold)
                    let corners = [
                        (crop_screen.min, ResizeCorner::TopLeft),
                        (
                            egui::pos2(crop_screen.max.x, crop_screen.min.y),
                            ResizeCorner::TopRight,
                        ),
                        (
                            egui::pos2(crop_screen.min.x, crop_screen.max.y),
                            ResizeCorner::BottomLeft,
                        ),
                        (crop_screen.max, ResizeCorner::BottomRight),
                    ];

                    let mut hit_corner = false;
                    for (corner_pos, corner) in corners {
                        if pos.distance(corner_pos) < 12.0 {
                            self.crop_interaction_state = CropInteractionState::ResizingCrop {
                                corner,
                                start_pos: pos,
                                original_rect: crop.clone(),
                            };
                            hit_corner = true;
                            break;
                        }
                    }

                    // If not on a corner, check if inside crop rect for dragging
                    if !hit_corner && crop_screen.contains(pos) {
                        self.crop_interaction_state = CropInteractionState::DraggingCrop {
                            start_pos: pos,
                            original_rect: crop,
                        };
                    }
                }
            }
            CropInteractionState::DraggingCrop {
                start_pos,
                original_rect,
            } => {
                if response.dragged()
                    && let Some(pos) = mouse_pos
                {
                    let delta_x = (pos.x - start_pos.x) / image_rect.width();
                    let delta_y = (pos.y - start_pos.y) / image_rect.height();

                    let new_x = (original_rect.x + delta_x).clamp(0.0, 1.0 - original_rect.width);
                    let new_y = (original_rect.y + delta_y).clamp(0.0, 1.0 - original_rect.height);

                    if let Some(pi) = self.packed_images.get_mut(idx)
                        && let Some(ref mut crop) = pi.crop_rotate.crop_rect
                    {
                        crop.x = new_x;
                        crop.y = new_y;
                    }
                }
            }
            CropInteractionState::ResizingCrop {
                corner,
                start_pos,
                original_rect,
            } => {
                if response.dragged()
                    && let Some(pos) = mouse_pos
                {
                    let delta_x = (pos.x - start_pos.x) / image_rect.width();
                    let delta_y = (pos.y - start_pos.y) / image_rect.height();

                    if let Some(pi) = self.packed_images.get_mut(idx)
                        && let Some(ref mut crop) = pi.crop_rotate.crop_rect
                    {
                        match corner {
                            ResizeCorner::TopLeft => {
                                let new_x = original_rect.x + delta_x;
                                let new_y = original_rect.y + delta_y;
                                let new_w = original_rect.width - delta_x;
                                let new_h = original_rect.height - delta_y;
                                if new_w > 0.05 && new_h > 0.05 {
                                    crop.x = new_x.clamp(0.0, 0.95);
                                    crop.y = new_y.clamp(0.0, 0.95);
                                    crop.width = new_w.clamp(0.05, 1.0);
                                    crop.height = new_h.clamp(0.05, 1.0);
                                }
                            }
                            ResizeCorner::TopRight => {
                                let new_y = original_rect.y + delta_y;
                                let new_w = original_rect.width + delta_x;
                                let new_h = original_rect.height - delta_y;
                                if new_w > 0.05 && new_h > 0.05 {
                                    crop.y = new_y.clamp(0.0, 0.95);
                                    crop.width = new_w.clamp(0.05, 1.0);
                                    crop.height = new_h.clamp(0.05, 1.0);
                                }
                            }
                            ResizeCorner::BottomLeft => {
                                let new_x = original_rect.x + delta_x;
                                let new_w = original_rect.width - delta_x;
                                let new_h = original_rect.height + delta_y;
                                if new_w > 0.05 && new_h > 0.05 {
                                    crop.x = new_x.clamp(0.0, 0.95);
                                    crop.width = new_w.clamp(0.05, 1.0);
                                    crop.height = new_h.clamp(0.05, 1.0);
                                }
                            }
                            ResizeCorner::BottomRight => {
                                let new_w = original_rect.width + delta_x;
                                let new_h = original_rect.height + delta_y;
                                if new_w > 0.05 && new_h > 0.05 {
                                    crop.width = new_w.clamp(0.05, 1.0);
                                    crop.height = new_h.clamp(0.05, 1.0);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Double-click to create crop at position
        if response.double_clicked()
            && let Some(pos) = mouse_pos
            && image_rect.contains(pos)
        {
            let norm_x = ((pos.x - image_rect.min.x) / image_rect.width() - 0.3).clamp(0.0, 0.4);
            let norm_y = ((pos.y - image_rect.min.y) / image_rect.height() - 0.3).clamp(0.0, 0.4);
            if let Some(pi) = self.packed_images.get_mut(idx) {
                pi.crop_rotate.crop_rect = Some(crate::effect::crop_rotate::NormalizedRect {
                    x: norm_x,
                    y: norm_y,
                    width: 0.6,
                    height: 0.6,
                });
                self.edit_preview_cache_key = None;
            }
        }
    }

    /// Generate or get cached LUT icon texture
    fn get_or_create_lut_icon(
        &mut self,
        ui_ctx: &egui::Context,
        lut_item: &LutItem,
    ) -> egui::TextureHandle {
        // Check cache first
        if let Some(texture) = self.lut_icon_textures.get(&lut_item.id) {
            return texture.clone();
        }

        // Generate a simple icon texture (80x80)
        let size = 80usize;
        let mut pixels = vec![0u8; size * size * 4];

        // Choose color based on LUT type
        let (base_r, base_g, base_b) = match lut_item.lut_type {
            StoredLutType::Lut3D => (70, 130, 180), // Steel blue for 3D
            StoredLutType::Lut1D => (180, 130, 70), // Orange for 1D
            StoredLutType::Unknown => (128, 128, 128), // Gray for unknown
        };

        // Create gradient background
        for y in 0..size {
            for x in 0..size {
                let idx = (y * size + x) * 4;
                // Gradient from top-left to bottom-right
                let factor = (x + y) as f32 / (2.0 * size as f32);
                let r = (base_r as f32 * (0.7 + 0.3 * factor)) as u8;
                let g = (base_g as f32 * (0.7 + 0.3 * factor)) as u8;
                let b = (base_b as f32 * (0.7 + 0.3 * factor)) as u8;

                // Add border
                let border = x < 2 || x >= size - 2 || y < 2 || y >= size - 2;
                if border {
                    pixels[idx] = 40;
                    pixels[idx + 1] = 40;
                    pixels[idx + 2] = 40;
                    pixels[idx + 3] = 255;
                } else {
                    pixels[idx] = r;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = b;
                    pixels[idx + 3] = 255;
                }
            }
        }

        // Create texture
        let color_image = egui::ColorImage::from_rgba_unmultiplied([size, size], &pixels);
        let texture = ui_ctx.load_texture(
            format!("lut_icon_{}", lut_item.id),
            color_image,
            egui::TextureOptions::LINEAR,
        );

        // Cache it
        self.lut_icon_textures.insert(lut_item.id, texture.clone());

        texture
    }

    /// Render horizontal LUT gallery
    fn render_lut_gallery(&mut self, ui: &mut egui::Ui) -> (LutUiAction, Option<Uuid>) {
        let mut action = LutUiAction::None;
        let mut lut_to_delete: Option<Uuid> = None;

        // Get current image's LUT ID for selection highlight
        let current_lut_id = self
            .edit_selected_index
            .and_then(|idx| self.packed_images.get(idx))
            .and_then(|pi| pi.lut_id);

        // Collect visible LUT data to avoid borrow conflicts
        // Tuple: (id, name, lut_type, size_info, file_missing, hash_mismatch, is_builtin)
        let lut_data: Vec<(Uuid, String, StoredLutType, String, bool, bool, bool)> = self
            .lut_storage
            .luts
            .iter()
            .filter(|l| !l.is_hidden)
            .map(|l| {
                (
                    l.id,
                    l.name.clone(),
                    l.lut_type,
                    l.lut_size_info.clone(),
                    l.file_missing,
                    l.hash_mismatch,
                    l.is_builtin,
                )
            })
            .collect();

        // Fixed height gallery
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 120.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("lut_gallery")
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // "Add LUT" button as first item
                            let add_response = ui.allocate_ui_with_layout(
                                egui::vec2(80.0, 100.0),
                                egui::Layout::top_down(egui::Align::Center),
                                |ui| {
                                    let thumbnail_size = egui::vec2(80.0, 80.0);

                                    // Draw add button with dashed border
                                    let (response, painter) =
                                        ui.allocate_painter(thumbnail_size, egui::Sense::click());
                                    let rect = response.rect;

                                    // Draw dashed border
                                    painter.rect_stroke(
                                        rect.shrink(2.0),
                                        4.0,
                                        egui::Stroke::new(2.0, ui.visuals().weak_text_color()),
                                        egui::StrokeKind::Inside,
                                    );

                                    // Draw plus sign
                                    painter.text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "+",
                                        egui::FontId::proportional(36.0),
                                        ui.visuals().weak_text_color(),
                                    );

                                    // Label
                                    ui.label(
                                        egui::RichText::new(t!("color.add_lut", default = "+ Add"))
                                            .size(10.0)
                                            .color(ui.visuals().weak_text_color()),
                                    );

                                    response
                                },
                            );

                            if add_response.inner.clicked() {
                                action = LutUiAction::OpenAddDialog;
                            }

                            ui.add_space(5.0);

                            // Show hint if no LUTs
                            if lut_data.is_empty() {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(20.0);
                                    ui.label(
                                        egui::RichText::new(t!(
                                            "color.no_luts",
                                            default = "No LUTs loaded. Click + or drop .cube files"
                                        ))
                                        .size(12.0)
                                        .italics()
                                        .color(ui.visuals().weak_text_color()),
                                    );
                                });
                            }

                            // Render each LUT item
                            for (
                                lut_id,
                                name,
                                lut_type,
                                size_info,
                                file_missing,
                                hash_mismatch,
                                is_builtin,
                            ) in &lut_data
                            {
                                let is_selected = current_lut_id == Some(*lut_id);

                                // Create a temporary LutItem for icon generation
                                let temp_lut_item = LutItem {
                                    id: *lut_id,
                                    name: name.clone(),
                                    file_path: std::path::PathBuf::new(),
                                    timestamp: 0,
                                    file_hash: None,
                                    lut_type: *lut_type,
                                    lut_size_info: size_info.clone(),
                                    hash_mismatch: *hash_mismatch,
                                    file_missing: *file_missing,
                                    is_builtin: *is_builtin,
                                    is_hidden: false,
                                };

                                let texture = self.get_or_create_lut_icon(ui.ctx(), &temp_lut_item);

                                let container_response = ui.allocate_ui_with_layout(
                                    egui::vec2(80.0, 100.0),
                                    egui::Layout::top_down(egui::Align::Center),
                                    |ui| {
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
                                                    egui::Image::from_texture(&texture)
                                                        .fit_to_exact_size(thumbnail_size)
                                                        .sense(egui::Sense::click()),
                                                )
                                            })
                                            .inner;

                                        // LUT type label on the icon
                                        let type_label = match lut_type {
                                            StoredLutType::Lut3D => "3D",
                                            StoredLutType::Lut1D => "1D",
                                            StoredLutType::Unknown => "?",
                                        };

                                        // Draw type label overlay at bottom-right of icon
                                        let label_pos = image_response.rect.right_bottom()
                                            + egui::vec2(-20.0, -16.0);
                                        ui.painter().text(
                                            label_pos,
                                            egui::Align2::CENTER_CENTER,
                                            type_label,
                                            egui::FontId::proportional(12.0),
                                            egui::Color32::WHITE,
                                        );

                                        // Name label (with built-in badge prefix)
                                        let display_name = if *is_builtin {
                                            format!("📦 {}", name)
                                        } else {
                                            name.clone()
                                        };
                                        let mut name_text = egui::RichText::new(&display_name)
                                            .size(10.0)
                                            .color(ui.visuals().weak_text_color());

                                        // Color based on status
                                        if *file_missing {
                                            name_text = name_text.color(egui::Color32::RED);
                                        } else if *hash_mismatch {
                                            name_text = name_text.color(ui.visuals().warn_fg_color);
                                        }

                                        ui.add(egui::Label::new(name_text).truncate())
                                            .on_hover_text(format!("{} - {}", name, size_info));

                                        // Show warning icons
                                        if *file_missing {
                                            ui.label(
                                                egui::RichText::new("❌")
                                                    .size(12.0)
                                                    .color(ui.visuals().error_fg_color),
                                            );
                                        } else if *hash_mismatch {
                                            ui.label(
                                                egui::RichText::new("⚠️")
                                                    .size(12.0)
                                                    .color(ui.visuals().warn_fg_color),
                                            );
                                        }

                                        image_response
                                    },
                                );

                                let image_response = container_response.inner;
                                let rect = image_response.rect;

                                // Check if mouse is hovering
                                let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                                let is_hovered =
                                    pointer_pos.map(|pos| rect.contains(pos)).unwrap_or(false);

                                // Delete/hide button on hover
                                if is_hovered {
                                    let button_size = 20.0;
                                    let delete_button_rect = egui::Rect::from_min_size(
                                        rect.right_top() + egui::vec2(-button_size, 0.0),
                                        egui::vec2(button_size, button_size),
                                    );

                                    let center = delete_button_rect.center();

                                    if *is_builtin {
                                        // Amber button with eye-slash for built-in (hide, not delete)
                                        ui.painter().circle_filled(
                                            center,
                                            10.0,
                                            egui::Color32::from_rgba_premultiplied(
                                                200, 140, 0, 220,
                                            ),
                                        );
                                        ui.painter().text(
                                            center,
                                            egui::Align2::CENTER_CENTER,
                                            "👁",
                                            egui::FontId::proportional(11.0),
                                            egui::Color32::WHITE,
                                        );
                                    } else {
                                        // Red button with X for user LUTs (delete)
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
                                    }

                                    // Check delete/hide click
                                    if let Some(pos) = pointer_pos {
                                        if delete_button_rect.contains(pos)
                                            && image_response.clicked()
                                        {
                                            lut_to_delete = Some(*lut_id);
                                        } else if image_response.clicked() {
                                            // Select this LUT for current image
                                            if let Some(idx) = self.edit_selected_index
                                                && let Some(pi) = self.packed_images.get_mut(idx)
                                            {
                                                pi.lut_id = Some(*lut_id);
                                                self.edit_preview_cache_key = None;
                                                log::info!(
                                                    "Selected LUT {} for image {}",
                                                    lut_id,
                                                    idx
                                                );
                                            }
                                        }
                                    }
                                } else if image_response.clicked() {
                                    // Select this LUT for current image
                                    if let Some(idx) = self.edit_selected_index
                                        && let Some(pi) = self.packed_images.get_mut(idx)
                                    {
                                        pi.lut_id = Some(*lut_id);
                                        self.edit_preview_cache_key = None;
                                        log::info!("Selected LUT {} for image {}", lut_id, idx);
                                    }
                                }

                                ui.add_space(5.0);
                            }
                        });
                    });
            },
        );

        (action, lut_to_delete)
    }

    /// Render per-image LUT selection UI with horizontal gallery
    /// Sets lut_id on the current PackedImage instead of global selection
    pub(crate) fn render_per_image_lut_ui(&mut self, ui: &mut egui::Ui) -> LutUiAction {
        let mut action = LutUiAction::None;

        // Horizontal LUT gallery
        ui.label(t!("color.lut_settings", default = "LUT Settings"));

        let (gallery_action, lut_to_delete) = self.render_lut_gallery(ui);
        if gallery_action != LutUiAction::None {
            action = gallery_action;
        }

        // Handle LUT deletion/hiding
        if let Some(lut_id) = lut_to_delete {
            // Clear this LUT from all images that use it
            for pi in &mut self.packed_images {
                if pi.lut_id == Some(lut_id) {
                    pi.lut_id = None;
                }
            }
            // Remove (user) or hide (built-in) from storage
            self.lut_storage.remove_lut(lut_id);
            // Remove cached icon texture
            self.lut_icon_textures.remove(&lut_id);
            // Invalidate preview cache
            self.edit_preview_cache_key = None;
            log::info!("Removed/hid LUT {}", lut_id);
        }

        // "Restore Default LUTs" button — shown when any built-in is hidden
        let has_hidden_builtins = self
            .lut_storage
            .luts
            .iter()
            .any(|l| l.is_builtin && l.is_hidden);
        if has_hidden_builtins
            && ui
                .button(t!(
                    "color.restore_defaults",
                    default = "Restore Default LUTs"
                ))
                .on_hover_text(t!(
                    "color.restore_defaults_hint",
                    default = "Restore built-in LUT presets that were hidden"
                ))
                .clicked()
        {
            let restored = self.lut_storage.restore_builtin_luts();
            log::info!("Restored {} hidden built-in LUTs", restored);
        }

        let Some(idx) = self.edit_selected_index else {
            ui.label(t!(
                "color.select_image_first",
                default = "Select an image first to assign a LUT"
            ));
            return action;
        };

        let Some(packed_image) = self.packed_images.get(idx) else {
            return action;
        };

        // Get current LUT ID for this image
        let current_lut_id = packed_image.lut_id;

        // Show current LUT info
        ui.horizontal(|ui| {
            ui.label(t!("color.lut_select", default = "Current LUT:"));

            let current_lut_name = current_lut_id
                .and_then(|id| self.lut_storage.get_lut(id))
                .map(|l| format!("{} ({})", l.name, l.lut_size_info))
                .unwrap_or_else(|| t!("color.no_lut", default = "None").to_string());

            ui.label(egui::RichText::new(current_lut_name).strong());

            // Clear LUT button (only if LUT is configured)
            if current_lut_id.is_some()
                && ui
                    .button(t!("color.clear_lut", default = "Clear"))
                    .clicked()
                && let Some(pi) = self.packed_images.get_mut(idx)
            {
                pi.lut_id = None;
                self.edit_preview_cache_key = None;
                log::info!("Cleared LUT for image index {}", idx);
            }
        });

        ui.add_space(5.0);

        // Apply to all / Clear all buttons
        ui.horizontal(|ui| {
            if ui
                .button(t!("color.apply_to_all", default = "Apply to All Images"))
                .clicked()
            {
                let lut_to_apply = current_lut_id;
                for pi in &mut self.packed_images {
                    pi.lut_id = lut_to_apply;
                }
                self.edit_preview_cache_key = None;
                log::info!("Applied LUT {:?} to all images", lut_to_apply);
            }

            if ui
                .button(t!("color.clear_all", default = "Clear All LUTs"))
                .clicked()
            {
                for pi in &mut self.packed_images {
                    pi.lut_id = None;
                }
                self.edit_preview_cache_key = None;
                log::info!("Cleared LUT from all images");
            }
        });

        action
    }

    /// Spawn async file dialog to add a LUT file
    pub(crate) fn spawn_lut_file_dialog(&mut self) {
        #[cfg(feature = "rfd")]
        if self.pending_lut_pick.is_none() {
            let title: String =
                t!("color.select_lut_file", default = "Select LUT File").to_string();
            self.pending_lut_pick =
                Some(crate::util::async_file_dialog::pick_file_with_title_async(
                    &title,
                    "CUBE LUT",
                    &["cube", "CUBE"],
                ));
        }

        #[cfg(all(feature = "desktop", not(feature = "rfd")))]
        {
            use rfd::FileDialog;

            let file = FileDialog::new()
                .add_filter("CUBE LUT", &["cube", "CUBE"])
                .set_title(t!("color.select_lut_file", default = "Select LUT File"))
                .pick_file();

            if let Some(path) = file {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unnamed LUT")
                    .to_string();

                match self.lut_storage.add_lut(name.clone(), &path) {
                    Ok(id) => {
                        log::info!("Successfully added LUT: {} (id: {})", name, id);
                        if let Some(idx) = self.edit_selected_index
                            && let Some(pi) = self.packed_images.get_mut(idx)
                        {
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
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Face Detection Tab - Preview and edit face detection results with sticker assignment

use crate::app::{ChamaOptics, FaceInteractionState, ResizeCorner};
use crate::effect::sticker_storage::FaceWithSticker;
use rust_i18n::t;

impl ChamaOptics {
    /// Render the face detection preview and editing tab
    pub fn render_detection_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("detection.title", default = "Face Detection"));
        ui.separator();

        // Image selection
        if self.packed_images.is_empty() {
            ui.label(t!("detection.no_images"));
            return;
        }

        // Image selector dropdown
        ui.horizontal(|ui| {
            ui.label(t!("detection.select_image"));

            let current_name = self
                .preview_selected_index
                .and_then(|idx| self.packed_images.get(idx))
                .and_then(|img| img.path.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Select...");

            egui::ComboBox::from_id_salt("detection_image_select")
                .selected_text(current_name)
                .show_ui(ui, |ui| {
                    for (idx, img) in self.packed_images.iter().enumerate() {
                        let img_name = img
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        if ui
                            .selectable_label(self.preview_selected_index == Some(idx), img_name)
                            .clicked()
                        {
                            self.preview_selected_index = Some(idx);
                            // Reset zoom and pan when image changes
                            self.detection_zoom = 1.0;
                            self.detection_pan = egui::Vec2::ZERO;
                            self.detection_preview_texture = None;
                            self.detection_preview_cache_key = None;
                            self.detected_faces.clear();
                            self.selected_face_index = None;
                        }
                    }
                });
        });

        ui.separator();

        // Detection controls and zoom controls
        ui.horizontal(|ui| {
            let is_detecting = self.detection_progress.is_active();

            if ui.button(t!("detection.detect_faces")).clicked() && !is_detecting {
                self.run_face_detection();
            }

            // Show progress bar while detecting
            if is_detecting {
                ui.add(
                    egui::ProgressBar::new(self.detection_progress.fraction())
                        .show_percentage()
                        .animate(true),
                );
            } else if !self.detected_faces.is_empty() {
                ui.label(format!(
                    "{} {}",
                    self.detected_faces.len(),
                    t!("detection.faces_found")
                ));

                if ui.button(t!("detection.clear_all")).clicked() {
                    self.detected_faces.clear();
                    self.selected_face_index = None;
                }
            }

            ui.separator();

            // Zoom controls
            if ui.button("➖").clicked() {
                self.detection_zoom = (self.detection_zoom / 1.2).max(0.1);
            }

            ui.label(format!("{:.0}%", self.detection_zoom * 100.0));

            if ui.button("➕").clicked() {
                self.detection_zoom = (self.detection_zoom * 1.2).min(10.0);
            }

            if ui
                .button("🔄")
                .on_hover_text(t!("detection.reset_view"))
                .clicked()
            {
                self.detection_zoom = 1.0;
                self.detection_pan = egui::Vec2::ZERO;
            }
        });

        ui.separator();

        // Two-column layout: preview on left, face list on right
        ui.columns(2, |columns| {
            // Left column: Image preview with face rectangles
            columns[0].group(|ui| {
                ui.label(t!("detection.preview"));

                if let Some(idx) = self.preview_selected_index {
                    if let Some(packed_image) = self.packed_images.get(idx) {
                        // Generate base preview texture (without rectangles)
                        let cache_key = (idx, self.detected_faces.len());

                        let needs_regenerate = self.detection_preview_cache_key != Some(cache_key);

                        // Clone data needed before mutable operations
                        let image_path = packed_image.path.clone();
                        let image_uuid = packed_image.uuid;

                        if needs_regenerate {
                            // Start async preview generation
                            self.start_async_preview_generation(&image_path, image_uuid, idx);
                        }

                        // Display preview with interactive editing
                        let texture = self.detection_preview_texture.clone();
                        if let Some(texture) = texture {
                            self.render_zoomable_preview(ui, &texture, image_path);
                        }
                    }
                } else {
                    ui.label(t!("detection.select_image_first"));
                }
            });

            // Right column: Face list and sticker assignment
            columns[1].group(|ui| {
                ui.label(t!("detection.faces_and_stickers"));
                ui.separator();

                if self.detected_faces.is_empty() {
                    ui.label(t!("detection.no_faces"));
                } else {
                    // Face list
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            let mut faces_to_remove = vec![];

                            for (idx, face) in self.detected_faces.iter_mut().enumerate() {
                                let is_selected = self.selected_face_index == Some(idx);

                                ui.horizontal(|ui| {
                                    // Selection indicator
                                    if ui
                                        .selectable_label(
                                            is_selected,
                                            t!(
                                                "detection.face_label_format",
                                                n1 = idx + 1,
                                                n2 = face.width,
                                                n3 = face.height
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.selected_face_index = Some(idx);
                                    }

                                    // Sticker indicator
                                    if face.sticker_id.is_some() {
                                        ui.label("🎭");
                                    }

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            // Delete button
                                            if ui.button("🗑").clicked() {
                                                faces_to_remove.push(idx);
                                            }
                                        },
                                    );
                                });
                            }

                            // Remove faces (in reverse order to preserve indices)
                            for idx in faces_to_remove.into_iter().rev() {
                                self.detected_faces.remove(idx);
                                if self.selected_face_index == Some(idx) {
                                    self.selected_face_index = None;
                                }
                            }
                        });

                    ui.separator();

                    // Sticker assignment for selected face
                    if let Some(selected_idx) = self.selected_face_index
                        && selected_idx < self.detected_faces.len()
                    {
                        ui.label(t!("detection.selected_face", n = selected_idx + 1));

                        // Sticker picker
                        ui.horizontal(|ui| {
                            ui.label(t!("detection.assign_sticker"));

                            let current_sticker_name = self.detected_faces[selected_idx]
                                .sticker_id
                                .and_then(|id| self.sticker_storage.get_sticker(id))
                                .map(|s| s.name.as_str())
                                .unwrap_or("None");

                            egui::ComboBox::from_id_salt("face_sticker_combo")
                                .selected_text(current_sticker_name)
                                .show_ui(ui, |ui| {
                                    // None option
                                    if ui
                                        .selectable_label(
                                            self.detected_faces[selected_idx].sticker_id.is_none(),
                                            "None",
                                        )
                                        .clicked()
                                    {
                                        self.detected_faces[selected_idx].sticker_id = None;
                                    }

                                    // Sticker options
                                    for sticker in &self.sticker_storage.stickers {
                                        if ui
                                            .selectable_label(
                                                self.detected_faces[selected_idx].sticker_id
                                                    == Some(sticker.id),
                                                &sticker.name,
                                            )
                                            .clicked()
                                        {
                                            self.detected_faces[selected_idx].sticker_id =
                                                Some(sticker.id);
                                        }
                                    }
                                });
                        });
                    }
                }
            });
        });

        // Hint about sticker storage location
        if self.sticker_storage.stickers.is_empty() {
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(t!("detection.sticker_hint"))
                    .weak()
                    .italics(),
            );
        }
    }

    /// Render zoomable preview with overlay rectangles
    fn render_zoomable_preview(
        &mut self,
        ui: &mut egui::Ui,
        texture: &egui::TextureHandle,
        _packed_image_path: std::path::PathBuf,
    ) {
        let available_size = ui.available_size();
        let texture_size = texture.size_vec2();

        // Calculate base size to fit available space
        let base_aspect = texture_size.x / texture_size.y;
        let base_size = if available_size.x / base_aspect > available_size.y {
            egui::vec2(available_size.y * base_aspect, available_size.y)
        } else {
            egui::vec2(available_size.x, available_size.x / base_aspect)
        };

        // Apply zoom
        let zoomed_size = base_size * self.detection_zoom;

        // Center zoomed image
        let offset = (available_size - zoomed_size) / 2.0 + self.detection_pan;

        // Allocate the interaction area and get actual screen position
        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());
        let viewport_rect = response.rect;

        // Calculate offset position relative to viewport
        let offset_pos = egui::pos2(
            viewport_rect.min.x + offset.x,
            viewport_rect.min.y + offset.y,
        );

        // Draw image with zoom and pan
        let image_rect = egui::Rect::from_min_size(offset_pos, zoomed_size);
        painter.image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // Calculate scale factor (image to viewport)
        let scale = zoomed_size.x / texture_size.x;

        // Use cached original image dimensions (to avoid loading image every frame)
        let (orig_w, _orig_h) = self
            .detection_preview_original_size
            .unwrap_or((1000u32, 1000u32));

        // Calculate scale from original image to viewport
        let orig_to_viewport_scale = scale * (texture_size.x / orig_w as f32);

        // Draw face rectangles as overlays
        for (idx, face) in self.detected_faces.iter().enumerate() {
            // Transform face coordinates to screen space (using viewport_rect.min as origin)
            let x = face.x as f32 * orig_to_viewport_scale + offset_pos.x;
            let y = face.y as f32 * orig_to_viewport_scale + offset_pos.y;
            let w = face.width as f32 * orig_to_viewport_scale;
            let h = face.height as f32 * orig_to_viewport_scale;

            // Skip if too small to see
            if w < 2.0 || h < 2.0 {
                continue;
            }

            let face_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));

            // Color based on state
            let stroke_color = if self.selected_face_index == Some(idx) {
                egui::Color32::LIGHT_BLUE // Blue for selected
            } else if face.sticker_id.is_some() {
                egui::Color32::ORANGE // Orange if has sticker
            } else {
                egui::Color32::LIGHT_GREEN // Green for normal
            };

            // Draw rectangle
            painter.rect_stroke(
                face_rect,
                0.0,
                (2.0, stroke_color),
                egui::StrokeKind::Inside,
            );

            // Draw resize handles if selected
            if self.selected_face_index == Some(idx) {
                self.draw_resize_handles(&painter, face_rect);
            }
        }

        // Handle mouse interactions (pass image_rect for face interactions)
        self.handle_zoom_interactions(
            ui,
            &response,
            base_size,
            available_size,
            viewport_rect,
            image_rect,
        );

        // Show navigation window if zoomed
        if self.detection_zoom > 1.5 {
            self.render_navigation_window(ui, texture, available_size, offset, zoomed_size);
        }
    }

    /// Draw resize handles for a selected face
    fn draw_resize_handles(&self, painter: &egui::Painter, face_rect: egui::Rect) {
        const HANDLE_SIZE: f32 = 8.0;
        const HANDLE_COLOR: egui::Color32 = egui::Color32::WHITE;

        let corners = [
            face_rect.min,
            egui::pos2(face_rect.max.x, face_rect.min.y), // Top right
            egui::pos2(face_rect.min.x, face_rect.max.y), // Bottom left
            face_rect.max,                                // Bottom right
        ];

        for corner in corners {
            let handle_rect =
                egui::Rect::from_center_size(corner, egui::vec2(HANDLE_SIZE, HANDLE_SIZE));
            painter.rect_filled(handle_rect, 0.0, HANDLE_COLOR);
            painter.rect_stroke(
                handle_rect,
                0.0,
                (1.0, egui::Color32::BLACK),
                egui::StrokeKind::Inside,
            );
        }
    }

    /// Handle zoom and pan interactions
    fn handle_zoom_interactions(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        base_size: egui::Vec2,
        available_size: egui::Vec2,
        viewport_rect: egui::Rect,
        image_rect: egui::Rect,
    ) {
        let zoomed_size = base_size * self.detection_zoom;
        let _offset = (available_size - zoomed_size) / 2.0 + self.detection_pan;

        // Get mouse position relative to viewport (in screen coordinates)
        let mouse_pos = response.hover_pos();

        // Handle mouse wheel zoom
        ui.ctx().input(|i| {
            if let Some(pos) = mouse_pos {
                for event in &i.events {
                    if let egui::Event::MouseWheel { delta, .. } = event
                        && response.hovered()
                    {
                        let zoom_factor = if delta.y > 0.0 { 1.1 } else { 0.9 };
                        let old_zoom = self.detection_zoom;
                        self.detection_zoom = (self.detection_zoom * zoom_factor).clamp(0.1, 10.0);

                        // Adjust pan to zoom towards mouse position (relative to viewport center)
                        let zoom_ratio = self.detection_zoom / old_zoom;
                        let pos_in_viewport = egui::Vec2 {
                            x: pos.x - viewport_rect.min.x,
                            y: pos.y - viewport_rect.min.y,
                        };
                        let mouse_offset =
                            pos_in_viewport - (available_size / 2.0) - self.detection_pan;
                        self.detection_pan += mouse_offset - mouse_offset / zoom_ratio;
                    }
                }
            }
        });

        // Handle middle-click or shift+drag for panning
        let is_panning_key = ui.input(|i| i.modifiers.shift);
        let is_middle_click = ui.input(|i| i.pointer.middle_down());

        if (is_middle_click || is_panning_key) && response.dragged() && !self.detection_is_panning {
            self.detection_is_panning = true;
            self.detection_pan_start = mouse_pos
                .map(|p| egui::Vec2 { x: p.x, y: p.y })
                .unwrap_or(egui::Vec2::ZERO);
        }

        if self.detection_is_panning {
            if let Some(pos) = mouse_pos {
                let current = egui::Vec2 { x: pos.x, y: pos.y };
                let delta = current - self.detection_pan_start;
                self.detection_pan += delta;
                self.detection_pan_start = current;
            }

            if response.drag_stopped() || (!is_middle_click && !is_panning_key) {
                self.detection_is_panning = false;
            }
        }

        // Handle face interactions (drag, resize, delete) - only when not panning
        if !self.detection_is_panning {
            self.handle_face_interactions(ui, response.clone(), viewport_rect, image_rect);
        }
    }

    /// Handle face rectangle interactions
    fn handle_face_interactions(
        &mut self,
        _ui: &mut egui::Ui,
        response: egui::Response,
        _viewport_rect: egui::Rect,
        image_rect: egui::Rect,
    ) {
        // Use screen coordinates directly
        let mouse_pos = response.hover_pos();

        // Calculate total scale from screen coordinates to original image coordinates
        let texture_size = self
            .detection_preview_texture
            .as_ref()
            .map_or(egui::vec2(1000.0, 1000.0), |t| t.size_vec2());
        let (orig_w, _orig_h) = self.detection_preview_original_size.unwrap_or((1000, 1000));
        let screen_to_texture = image_rect.width() / texture_size.x;
        let texture_to_orig = texture_size.x / orig_w as f32;
        let total_scale = screen_to_texture * texture_to_orig; // screen -> original

        // Handle mouse release - reset to Idle state
        if response.drag_stopped() {
            self.face_interaction_state = FaceInteractionState::Idle;
            return;
        }

        match &self.face_interaction_state {
            FaceInteractionState::Idle => {
                // Only process interactions on actual click (drag_started), not just hover
                let is_clicking = response.drag_started();
                let is_clicked = response.clicked();

                if let Some(pos) = mouse_pos {
                    // Check resize handles first (if a face is selected) - only on click
                    if is_clicking {
                        if let Some(selected_idx) = self.selected_face_index
                            && let Some(face) = self.detected_faces.get(selected_idx)
                            && let Some(corner) = self.check_resize_handle(pos, face, image_rect)
                        {
                            self.face_interaction_state = FaceInteractionState::Resizing {
                                face_index: selected_idx,
                                corner,
                                start_pos: egui::pos2(pos.x, pos.y),
                                original_rect: (face.x, face.y, face.width, face.height),
                            };
                            return;
                        }

                        // Check if clicking on a face rectangle to start dragging
                        for (idx, face) in self.detected_faces.iter().enumerate() {
                            if self.is_point_in_face(pos, face, image_rect) {
                                self.selected_face_index = Some(idx);
                                self.face_interaction_state = FaceInteractionState::Dragging {
                                    face_index: idx,
                                    start_pos: egui::pos2(pos.x, pos.y),
                                };
                                return;
                            }
                        }
                    }

                    // Handle simple click (not drag) - select face or deselect
                    if is_clicked {
                        let mut clicked_on_face = false;
                        for (idx, face) in self.detected_faces.iter().enumerate() {
                            if self.is_point_in_face(pos, face, image_rect) {
                                self.selected_face_index = Some(idx);
                                clicked_on_face = true;
                                break;
                            }
                        }

                        // Clicked on empty space - deselect
                        if !clicked_on_face {
                            self.selected_face_index = None;
                        }
                    }
                }
            }
            FaceInteractionState::Dragging {
                face_index,
                start_pos,
            } => {
                let face_index = *face_index;
                let start_pos = *start_pos;

                if response.dragged()
                    && let Some(face) = self.detected_faces.get_mut(face_index)
                    && let Some(current_pos) = mouse_pos
                {
                    let delta = egui::Vec2 {
                        x: current_pos.x - start_pos.x,
                        y: current_pos.y - start_pos.y,
                    };
                    // Convert screen delta to original image coordinates
                    face.x = (face.x as f32 + delta.x / total_scale).round() as i32;
                    face.y = (face.y as f32 + delta.y / total_scale).round() as i32;

                    // Update start position for next frame
                    self.face_interaction_state = FaceInteractionState::Dragging {
                        face_index,
                        start_pos: egui::pos2(current_pos.x, current_pos.y),
                    };
                }
            }
            FaceInteractionState::Resizing {
                face_index,
                corner,
                original_rect,
                start_pos,
            } => {
                let face_index = *face_index;
                let corner = *corner;
                let original_rect = *original_rect;
                let start_pos = *start_pos;

                if response.dragged()
                    && let Some(face) = self.detected_faces.get_mut(face_index)
                    && let Some(current_pos) = mouse_pos
                {
                    let delta = egui::Vec2 {
                        x: current_pos.x - start_pos.x,
                        y: current_pos.y - start_pos.y,
                    };

                    // Convert screen delta to original image coordinates
                    match corner {
                        ResizeCorner::TopLeft => {
                            let new_x = original_rect.0 as f32 + delta.x / total_scale;
                            let new_y = original_rect.1 as f32 + delta.y / total_scale;
                            let new_w = original_rect.2 as f32 - delta.x / total_scale;
                            let new_h = original_rect.3 as f32 - delta.y / total_scale;

                            if new_w > 10.0 && new_h > 10.0 {
                                face.x = new_x.round() as i32;
                                face.y = new_y.round() as i32;
                                face.width = new_w.round() as u32;
                                face.height = new_h.round() as u32;
                            }
                        }
                        ResizeCorner::TopRight => {
                            let new_y = original_rect.1 as f32 + delta.y / total_scale;
                            let new_w = original_rect.2 as f32 + delta.x / total_scale;
                            let new_h = original_rect.3 as f32 - delta.y / total_scale;

                            if new_w > 10.0 && new_h > 10.0 {
                                face.y = new_y.round() as i32;
                                face.width = new_w.round() as u32;
                                face.height = new_h.round() as u32;
                            }
                        }
                        ResizeCorner::BottomLeft => {
                            let new_x = original_rect.0 as f32 + delta.x / total_scale;
                            let new_w = original_rect.2 as f32 - delta.x / total_scale;
                            let new_h = original_rect.3 as f32 + delta.y / total_scale;

                            if new_w > 10.0 && new_h > 10.0 {
                                face.x = new_x.round() as i32;
                                face.width = new_w.round() as u32;
                                face.height = new_h.round() as u32;
                            }
                        }
                        ResizeCorner::BottomRight => {
                            let new_w = original_rect.2 as f32 + delta.x / total_scale;
                            let new_h = original_rect.3 as f32 + delta.y / total_scale;

                            if new_w > 10.0 && new_h > 10.0 {
                                face.width = new_w.round() as u32;
                                face.height = new_h.round() as u32;
                            }
                        }
                    }
                }
            }
        }

        // Handle right-click to delete
        if response.secondary_clicked()
            && let Some(pos) = mouse_pos
        {
            for (idx, face) in self.detected_faces.iter().enumerate() {
                if self.is_point_in_face(pos, face, image_rect) {
                    self.detected_faces.remove(idx);
                    if self.selected_face_index == Some(idx) {
                        self.selected_face_index = None;
                    }
                    if let Some(s) = self.selected_face_index
                        && s > idx
                    {
                        self.selected_face_index = Some(s - 1);
                    }
                    return;
                }
            }
        }

        // Handle double-click to add new face area
        if response.double_clicked()
            && let Some(pos) = mouse_pos
        {
            // Check if double-clicked on empty space (not on existing face)
            let clicked_on_face = self
                .detected_faces
                .iter()
                .any(|face| self.is_point_in_face(pos, face, image_rect));

            if !clicked_on_face {
                // Convert viewport position to original image coordinates
                let texture_size = self
                    .detection_preview_texture
                    .as_ref()
                    .map_or(egui::vec2(1000.0, 1000.0), |t| t.size_vec2());
                let viewport_scale = image_rect.width() / texture_size.x;

                // Get original image dimensions
                let (orig_w, _orig_h) =
                    self.detection_preview_original_size.unwrap_or((1000, 1000));
                let orig_scale = texture_size.x / orig_w as f32;

                // Calculate position in original image coordinates
                let img_x =
                    ((pos.x - image_rect.min.x) / viewport_scale / orig_scale).round() as i32;
                let img_y =
                    ((pos.y - image_rect.min.y) / viewport_scale / orig_scale).round() as i32;

                // Create new face with default size (100x100 in original image coordinates)
                let default_size = 100u32;
                let new_face = FaceWithSticker::new(
                    img_x - (default_size as i32 / 2),
                    img_y - (default_size as i32 / 2),
                    default_size,
                    default_size,
                );

                self.detected_faces.push(new_face);
                self.selected_face_index = Some(self.detected_faces.len() - 1);
            }
        }
    }

    /// Check if point is inside a face rectangle (pos is in screen coordinates)
    fn is_point_in_face(
        &self,
        pos: egui::Pos2,
        face: &FaceWithSticker,
        image_rect: egui::Rect,
    ) -> bool {
        let texture_size = self
            .detection_preview_texture
            .as_ref()
            .map_or(egui::vec2(1000.0, 1000.0), |t| t.size_vec2());
        let (orig_w, _orig_h) = self.detection_preview_original_size.unwrap_or((1000, 1000));

        // Scale from original image coordinates to screen coordinates
        let scale = image_rect.width() / texture_size.x;
        let orig_scale = texture_size.x / orig_w as f32;
        let total_scale = scale * orig_scale;

        let x = face.x as f32 * total_scale + image_rect.min.x;
        let y = face.y as f32 * total_scale + image_rect.min.y;
        let w = face.width as f32 * total_scale;
        let h = face.height as f32 * total_scale;

        let face_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
        face_rect.contains(pos)
    }

    /// Check if point is on a resize handle (pos is in screen coordinates)
    fn check_resize_handle(
        &self,
        pos: egui::Pos2,
        face: &FaceWithSticker,
        image_rect: egui::Rect,
    ) -> Option<ResizeCorner> {
        const HANDLE_SIZE: f32 = 10.0;

        let texture_size = self
            .detection_preview_texture
            .as_ref()
            .map_or(egui::vec2(1000.0, 1000.0), |t| t.size_vec2());
        let (orig_w, _orig_h) = self.detection_preview_original_size.unwrap_or((1000, 1000));

        // Scale from original image coordinates to screen coordinates
        let scale = image_rect.width() / texture_size.x;
        let orig_scale = texture_size.x / orig_w as f32;
        let total_scale = scale * orig_scale;

        let x = face.x as f32 * total_scale + image_rect.min.x;
        let y = face.y as f32 * total_scale + image_rect.min.y;
        let w = face.width as f32 * total_scale;
        let h = face.height as f32 * total_scale;

        // Check each corner
        if is_near_point(pos, egui::pos2(x, y), HANDLE_SIZE) {
            return Some(ResizeCorner::TopLeft);
        }
        if is_near_point(pos, egui::pos2(x + w, y), HANDLE_SIZE) {
            return Some(ResizeCorner::TopRight);
        }
        if is_near_point(pos, egui::pos2(x, y + h), HANDLE_SIZE) {
            return Some(ResizeCorner::BottomLeft);
        }
        if is_near_point(pos, egui::pos2(x + w, y + h), HANDLE_SIZE) {
            return Some(ResizeCorner::BottomRight);
        }

        None
    }

    /// Render navigation window (mini-map)
    fn render_navigation_window(
        &self,
        ui: &mut egui::Ui,
        texture: &egui::TextureHandle,
        available_size: egui::Vec2,
        offset: egui::Vec2,
        zoomed_size: egui::Vec2,
    ) {
        const NAV_SIZE: f32 = 150.0;

        let painter = ui.painter_at(ui.clip_rect());

        // Position in bottom-right corner of preview area
        let nav_pos = egui::pos2(
            ui.clip_rect().max.x - NAV_SIZE - 10.0,
            ui.clip_rect().max.y - NAV_SIZE - 10.0,
        );

        let nav_rect = egui::Rect::from_min_size(nav_pos, egui::vec2(NAV_SIZE, NAV_SIZE));

        // Draw background
        painter.rect_filled(nav_rect, 5.0, egui::Color32::from_black_alpha(200));
        painter.rect_stroke(
            nav_rect,
            5.0,
            (1.0, egui::Color32::WHITE),
            egui::StrokeKind::Inside,
        );

        // Draw thumbnail
        painter.image(
            texture.id(),
            nav_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );

        // Calculate viewport indicator
        let viewport_x = -offset.x * (NAV_SIZE / zoomed_size.x);
        let viewport_y = -offset.y * (NAV_SIZE / zoomed_size.y);
        let viewport_w = available_size.x * (NAV_SIZE / zoomed_size.x);
        let viewport_h = available_size.y * (NAV_SIZE / zoomed_size.y);

        let viewport_indicator = egui::Rect::from_min_size(
            egui::pos2(nav_pos.x + viewport_x, nav_pos.y + viewport_y),
            egui::vec2(viewport_w, viewport_h),
        );

        // Draw viewport indicator
        painter.rect_stroke(
            viewport_indicator,
            0.0,
            (2.0, egui::Color32::RED),
            egui::StrokeKind::Inside,
        );
    }

    /// Run face detection on selected image (asynchronous)
    fn run_face_detection(&mut self) {
        let Some(idx) = self.preview_selected_index else {
            return;
        };
        let Some(packed_image) = self.packed_images.get(idx) else {
            return;
        };

        let filename = packed_image
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown");
        log::info!("Starting async face detection on: {}", filename);

        // Clear previous results
        self.detected_faces.clear();
        self.selected_face_index = None;
        self.detection_preview_cache_key = None;

        // Start progress tracking (1 item to detect)
        self.detection_progress.start(1);

        // Clone necessary data for background thread
        let image_path = packed_image.path.clone();
        let image_uuid = packed_image.uuid;
        let results_queue = self.detection_results_queue.clone();

        #[cfg(feature = "face_detection_insightface")]
        let speed_mode = self.export_config.face_detection.speed_mode;
        #[cfg(feature = "face_detection_insightface")]
        let provider = self.export_config.face_detection.provider;
        #[cfg(feature = "face_detection_insightface")]
        let detector_cache = self.insightface_detector.clone();

        // Spawn background thread for face detection
        std::thread::spawn(move || {
            log::info!("Face detection background thread started");

            #[cfg(feature = "face_detection_insightface")]
            use crate::effect::face_detectors::FaceDetector;

            #[cfg(feature = "face_detection_insightface")]
            {
                // Get or create detector from cache (store in Arc for thread safety)
                let detector = {
                    let mut cache = detector_cache.lock().unwrap();
                    if let Some(detector) = cache.as_ref() {
                        // Use cached detector (clone Arc, not the detector itself)
                        log::info!("Using cached InsightFace detector");
                        std::sync::Arc::clone(detector)
                    } else {
                        // Create new detector (this may take time, but only once)
                        log::info!("Creating new InsightFace detector (first time)");

                        let new_detector = std::sync::Arc::new(
                            crate::effect::insightface_detector::InsightFaceDetector::new(
                                speed_mode, provider,
                            ),
                        );

                        // Cache detector for future use (store Arc)
                        *cache = Some(std::sync::Arc::clone(&new_detector));
                        new_detector
                    }
                };

                let faces = detector.detect_faces(&image_path);

                log::info!("Detected {} faces", faces.len());

                // Send results to queue
                if let Ok(mut queue) = results_queue.lock() {
                    *queue = Some((faces, image_uuid));
                }
            }

            #[cfg(not(feature = "face_detection_insightface"))]
            {
                log::warn!("Face detection feature not enabled");
                if let Ok(mut queue) = results_queue.lock() {
                    *queue = Some((vec![], image_uuid));
                }
            }
        });
    }

    /// Process face detection results from background thread
    pub(crate) fn process_detection_results(&mut self) {
        // Check if there are results in the queue
        if let Ok(mut queue) = self.detection_results_queue.try_lock()
            && let Some((faces, image_uuid)) = queue.take()
        {
            // Verify this result is for the currently selected image
            if let Some(selected_idx) = self.preview_selected_index {
                if let Some(selected_image) = self.packed_images.get(selected_idx) {
                    if selected_image.uuid == image_uuid {
                        log::info!("Applying detection results for current image");

                        // Convert to FaceWithSticker and apply default sticker if set
                        self.detected_faces = faces
                            .into_iter()
                            .map(|(x, y, w, h)| {
                                let mut face = FaceWithSticker::new(x, y, w, h);
                                // Apply default sticker from storage
                                face.sticker_id = self.sticker_storage.default_sticker_id;
                                face
                            })
                            .collect();

                        self.selected_face_index = if self.detected_faces.is_empty() {
                            None
                        } else {
                            Some(0)
                        };
                    } else {
                        log::warn!("Detection results are for different image, ignoring");
                    }
                }
            } else {
                log::warn!("No image selected, ignoring detection results");
            }
        }

        // Update progress
        if self.detection_progress.is_active() {
            if self.detection_progress.current() < self.detection_progress.total() {
                // Increment progress
                let _ = self
                    .detection_progress
                    .counter()
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }

            if self.detection_progress.is_complete() {
                self.detection_progress.mark_complete();
            }
        }
    }

    /// Start async preview generation in background thread
    fn start_async_preview_generation(
        &mut self,
        image_path: &std::path::Path,
        image_uuid: uuid::Uuid,
        idx: usize,
    ) {
        let cache_key = (idx, self.detected_faces.len());
        let texture_queue = self.preview_texture_queue.clone();
        let image_path = image_path.to_path_buf();
        let image_uuid = image_uuid;

        // Update cache key immediately to prevent duplicate requests
        self.detection_preview_cache_key = Some(cache_key);

        // Spawn background thread to generate preview
        std::thread::spawn(move || {
            log::info!(
                "Starting async preview generation for image: {:?}",
                image_path
            );

            // Load and process image
            let color_image = Self::generate_detection_preview_sync(&image_path);

            if let Some((color_image, orig_size)) = color_image {
                // Send color image AND original size to queue
                if let Ok(mut queue) = texture_queue.lock() {
                    *queue = Some((color_image, image_uuid, orig_size));
                    log::info!("Preview generated successfully");
                }
            } else {
                log::warn!("Failed to generate preview");
            }
        });
    }

    /// Generate a preview image (without rectangles) - synchronous version
    fn generate_detection_preview_sync(
        image_path: &std::path::Path,
    ) -> Option<(egui::ColorImage, (u32, u32))> {
        // Load original image
        let mut dyn_image = image::open(image_path).ok()?;

        // Get original dimensions BEFORE scaling
        let orig_w = dyn_image.width();
        let orig_h = dyn_image.height();

        // Scale down for preview if needed
        let max_preview_size = 800u32;
        let (w, h) = (orig_w, orig_h);
        if w > max_preview_size || h > max_preview_size {
            let scale = max_preview_size as f32 / w.max(h) as f32;
            let new_w = (w as f32 * scale) as u32;
            let new_h = (h as f32 * scale) as u32;
            dyn_image = dyn_image.resize(new_w, new_h, image::imageops::FilterType::Triangle);
        }

        // Convert to ColorImage (no rectangles drawn)
        let rgba_image = dyn_image.to_rgba8();
        let size = [rgba_image.width() as usize, rgba_image.height() as usize];
        let pixels = rgba_image.into_raw();
        Some((
            egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
            (orig_w, orig_h),
        ))
    }

    /// Process preview texture from background thread
    pub(crate) fn process_preview_texture(&mut self, ui: &mut egui::Ui) {
        // Check if there's a preview texture ready
        if let Ok(mut queue) = self.preview_texture_queue.try_lock()
            && let Some((color_image, image_uuid, orig_size)) = queue.take()
        {
            // Verify this is for the currently selected image
            if let Some(selected_idx) = self.preview_selected_index
                && let Some(selected_image) = self.packed_images.get(selected_idx)
                && selected_image.uuid == image_uuid
            {
                // Create texture from color image
                let texture = ui.ctx().load_texture(
                    format!("detection_preview_{}", image_uuid),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                self.detection_preview_texture = Some(texture);
                self.detection_preview_original_size = Some(orig_size);
                log::info!("Preview texture loaded, original size: {:?}", orig_size);
            }
        }
    }
}

/// Helper function to check if a point is near another point
fn is_near_point(pos: egui::Pos2, target: egui::Pos2, threshold: f32) -> bool {
    pos.distance(target) < threshold
}

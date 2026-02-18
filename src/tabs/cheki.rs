/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Cheki Tab UI - Japanese polaroid style photo decoration
//!
//! Per-image decoration layer with polaroid border, text/sign input,
//! and random character sticker placement ("Play Dice!").
//! Preview uses egui-native drawing for instant response;
//! export uses cheki_renderer for pixel-accurate output.

use crate::ChamaOptics;
use crate::effect::cheki::{ChekiDecoration, DatePosition};
use rust_i18n::t;

/// Map a VariableOrNot font to an egui FontFamily for preview rendering.
fn cheki_font_to_egui_family(
    font: &crate::effect::variable_text::VariableOrNot,
) -> egui::FontFamily {
    use crate::effect::variable_text::VariableOrNot;
    use crate::fonts::variable_font::BuiltinVariableFontIndex;
    match font {
        VariableOrNot::Variable(idx) => match idx {
            BuiltinVariableFontIndex::Barlow => egui::FontFamily::Name("Barlow".into()),
            BuiltinVariableFontIndex::BarlowNarrow => {
                egui::FontFamily::Name("Barlow Narrow".into())
            }
            BuiltinVariableFontIndex::SourceHanSans => {
                egui::FontFamily::Name("Source Han Sans".into())
            }
            BuiltinVariableFontIndex::DynaPuff => egui::FontFamily::Name("DynaPuff".into()),
        },
        VariableOrNot::Others(fs) => {
            use crate::fonts::font_unify::FontSort;
            match fs.select.sort {
                FontSort::Builtin => match fs.name.as_str() {
                    "Barlow" => egui::FontFamily::Name("Barlow".into()),
                    "Barlow Narrow" => egui::FontFamily::Name("Barlow Narrow".into()),
                    "DynaPuff" => egui::FontFamily::Name("DynaPuff".into()),
                    "Source Han Sans" => egui::FontFamily::Name("Source Han Sans".into()),
                    _ => egui::FontFamily::Proportional,
                },
                FontSort::System => egui::FontFamily::Proportional,
            }
        }
    }
}

const CORNER_HANDLE_RADIUS: f32 = 6.0;
const ROTATION_HANDLE_OFFSET: f32 = 24.0;
const ROTATION_HANDLE_RADIUS: f32 = 5.0;

/// Pre-computed sticker display geometry for rendering and interaction
struct StickerDisplay {
    index: usize,
    center: egui::Pos2,
    half_size: egui::Vec2,
    rotation_rad: f32,
}

impl StickerDisplay {
    /// Rotate a point (dx, dy) relative to center
    fn rotate_point(&self, dx: f32, dy: f32) -> egui::Pos2 {
        let cos_r = self.rotation_rad.cos();
        let sin_r = self.rotation_rad.sin();
        egui::pos2(
            self.center.x + dx * cos_r - dy * sin_r,
            self.center.y + dx * sin_r + dy * cos_r,
        )
    }

    /// Get the 4 rotated corners: TL, TR, BR, BL
    fn corners(&self) -> [egui::Pos2; 4] {
        let hx = self.half_size.x;
        let hy = self.half_size.y;
        [
            self.rotate_point(-hx, -hy),
            self.rotate_point(hx, -hy),
            self.rotate_point(hx, hy),
            self.rotate_point(-hx, hy),
        ]
    }

    /// Get the rotation handle position (above top edge center)
    fn rotation_handle_pos(&self) -> egui::Pos2 {
        self.rotate_point(0.0, -self.half_size.y - ROTATION_HANDLE_OFFSET)
    }

    /// Check if a point is inside the rotated rectangle
    fn contains_point(&self, point: egui::Pos2) -> bool {
        // Transform point into local (un-rotated) space
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        let cos_r = (-self.rotation_rad).cos();
        let sin_r = (-self.rotation_rad).sin();
        let local_x = dx * cos_r - dy * sin_r;
        let local_y = dx * sin_r + dy * cos_r;
        local_x.abs() <= self.half_size.x && local_y.abs() <= self.half_size.y
    }

    /// Check if a point is near any corner handle; returns true if so
    fn corner_near_point(&self, point: egui::Pos2) -> bool {
        self.corners()
            .iter()
            .any(|c| c.distance(point) < CORNER_HANDLE_RADIUS * 2.0)
    }

    /// Return the index of the nearest corner (0=TL, 1=TR, 2=BR, 3=BL)
    fn nearest_corner_index(&self, point: egui::Pos2) -> usize {
        let corners = self.corners();
        corners
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.distance(point)
                    .partial_cmp(&b.distance(point))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Check if a point is near the rotation handle
    fn rotation_handle_near_point(&self, point: egui::Pos2) -> bool {
        self.rotation_handle_pos().distance(point) < ROTATION_HANDLE_RADIUS * 3.0
    }
}

/// Draw a textured quad rotated around its center
fn paint_rotated_image(
    painter: &egui::Painter,
    texture_id: egui::TextureId,
    center: egui::Pos2,
    size: egui::Vec2,
    rotation_rad: f32,
    tint: egui::Color32,
) {
    let half = size / 2.0;
    let cos_r = rotation_rad.cos();
    let sin_r = rotation_rad.sin();

    let rotate = |dx: f32, dy: f32| -> egui::Pos2 {
        egui::pos2(
            center.x + dx * cos_r - dy * sin_r,
            center.y + dx * sin_r + dy * cos_r,
        )
    };

    let positions = [
        rotate(-half.x, -half.y),
        rotate(half.x, -half.y),
        rotate(half.x, half.y),
        rotate(-half.x, half.y),
    ];
    let uvs = [
        egui::pos2(0.0, 0.0),
        egui::pos2(1.0, 0.0),
        egui::pos2(1.0, 1.0),
        egui::pos2(0.0, 1.0),
    ];

    let mut mesh = egui::Mesh::with_texture(texture_id);
    for i in 0..4 {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: positions[i],
            uv: uvs[i],
            color: tint,
        });
    }
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    painter.add(egui::Shape::mesh(mesh));
}

/// Draw interactive handles (corners + rotation) for a sticker
fn draw_sticker_handles(painter: &egui::Painter, sd: &StickerDisplay, is_active: bool) {
    let color = if is_active {
        egui::Color32::YELLOW
    } else {
        egui::Color32::WHITE
    };
    let outline_color = if is_active {
        egui::Color32::YELLOW
    } else {
        egui::Color32::from_white_alpha(180)
    };

    // Bounding box outline
    let corners = sd.corners();
    for i in 0..4 {
        painter.line_segment([corners[i], corners[(i + 1) % 4]], (1.5, outline_color));
    }

    // Corner resize handles
    for corner in &corners {
        painter.circle_filled(*corner, CORNER_HANDLE_RADIUS, color);
        painter.circle_stroke(*corner, CORNER_HANDLE_RADIUS, (1.0, egui::Color32::BLACK));
    }

    // Rotation handle (line from top-center to handle circle)
    let top_center = egui::pos2(
        (corners[0].x + corners[1].x) / 2.0,
        (corners[0].y + corners[1].y) / 2.0,
    );
    let rot_handle = sd.rotation_handle_pos();
    painter.line_segment([top_center, rot_handle], (1.5, color));
    painter.circle_filled(rot_handle, ROTATION_HANDLE_RADIUS, color);
    painter.circle_stroke(
        rot_handle,
        ROTATION_HANDLE_RADIUS,
        (1.0, egui::Color32::BLACK),
    );
}

impl ChamaOptics {
    /// Start background thread to generate base image texture with color effects applied.
    /// Applies: EXIF orientation → crop/rotate → color adjustments → LUT
    fn start_cheki_base_texture_generation(&mut self) -> Option<()> {
        let idx = self.cheki_selected_index?;
        let packed_image = self.packed_images.get(idx)?;

        // Include crop_rotate, lut_id, and color_adjustments in cache key
        let mut hasher = std::hash::DefaultHasher::new();
        std::hash::Hash::hash(&packed_image.crop_rotate.content_hash(), &mut hasher);
        std::hash::Hash::hash(&packed_image.lut_id, &mut hasher);
        // Hash color_adjustments fields via serialized form
        std::hash::Hash::hash(&self.color_adjustments.enabled, &mut hasher);
        std::hash::Hash::hash(&self.color_adjustments.exposure.to_bits(), &mut hasher);
        std::hash::Hash::hash(&self.color_adjustments.contrast, &mut hasher);
        std::hash::Hash::hash(&self.color_adjustments.highlights, &mut hasher);
        std::hash::Hash::hash(&self.color_adjustments.shadows, &mut hasher);
        std::hash::Hash::hash(&self.color_adjustments.whites, &mut hasher);
        std::hash::Hash::hash(&self.color_adjustments.blacks, &mut hasher);
        std::hash::Hash::hash(&self.color_adjustments.clarity, &mut hasher);
        std::hash::Hash::hash(&self.color_adjustments.vibrance, &mut hasher);
        std::hash::Hash::hash(&self.color_adjustments.saturation, &mut hasher);
        let cache_key = (idx, std::hash::Hasher::finish(&hasher));

        if self.cheki_preview_cache_key.as_ref() == Some(&cache_key) {
            return Some(());
        }

        self.cheki_preview_cache_key = Some(cache_key);
        self.cheki_preview_texture = None;

        let orientation = packed_image.view_exif.orientation;
        let crop_rotate = packed_image.crop_rotate.clone();
        let image_uuid = packed_image.uuid;
        let image_lut_id = packed_image.lut_id;
        let color_adjustments = self.color_adjustments.clone();
        let mut lut_storage = self.lut_storage.clone_for_thread();
        let queue = self.cheki_preview_queue.clone();

        // Shared processing logic (image → preview ColorImage)
        let mut process_image =
            move |mut img: image::DynamicImage| -> Option<(egui::ColorImage, uuid::Uuid)> {
                img.apply_orientation(orientation);

                if !crop_rotate.is_identity() {
                    img = crop_rotate.apply(&img);
                }

                color_adjustments.apply(&mut img);

                if let Some(lut_id) = image_lut_id {
                    lut_storage.apply_lut_to_image(lut_id, &mut img);
                }

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
                Some((color_image, image_uuid))
            };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let image_path = packed_image.path.clone();
            std::thread::spawn(move || {
                let img = match image::open(&image_path) {
                    Ok(img) => img,
                    Err(e) => {
                        log::error!("Failed to load image for cheki base texture: {:?}", e);
                        return;
                    }
                };
                if let Some(result) = process_image(img)
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
                if let Some(result) = process_image(img)
                    && let Ok(mut q) = queue.lock()
                {
                    *q = Some(result);
                }
            } else {
                log::error!("WASM: No image_bytes for cheki preview");
            }
        }

        Some(())
    }

    /// Process base texture from background thread queue
    fn process_cheki_base_texture(&mut self, ctx: &egui::Context) {
        if let Ok(mut queue) = self.cheki_preview_queue.try_lock()
            && let Some((color_image, image_uuid)) = queue.take()
        {
            let still_relevant = self
                .cheki_selected_index
                .and_then(|idx| self.packed_images.get(idx))
                .is_some_and(|pi| pi.uuid == image_uuid);

            if still_relevant {
                let texture = ctx.load_texture(
                    format!("cheki_base_{}", image_uuid),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                self.cheki_preview_texture = Some(texture);
            }
        }
    }

    /// Render the Cheki tab
    pub(crate) fn render_cheki_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("tabs.cheki_heading", default = "Cheki - Polaroid Style"));
        ui.separator();

        if self.packed_images.is_empty() {
            egui::ScrollArea::vertical()
                .id_salt("cheki_empty")
                .show(ui, |ui| {
                    // Info card
                    ui.add_space(10.0);
                    egui::Frame::group(ui.style())
                        .fill(ui.visuals().faint_bg_color)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("ℹ").size(16.0));
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(t!(
                                            "cheki.no_images",
                                            default = "No images loaded"
                                        ))
                                        .size(14.0),
                                    );
                                    ui.label(
                                        egui::RichText::new(t!(
                                            "cheki.no_images_hint",
                                            default = "Go to the Gallery tab to add images, then come back to apply Cheki decoration."
                                        ))
                                        .size(11.0)
                                        .color(ui.visuals().weak_text_color()),
                                    );
                                });
                            });
                        });

                    ui.add_space(8.0);

                    // Greyed-out settings preview
                    ui.scope(|ui| {
                        ui.disable();
                        ui.style_mut().visuals.override_text_color =
                            Some(ui.visuals().weak_text_color());

                        // Border Settings
                        ui.separator();
                        ui.strong(t!("cheki.border_settings", default = "Border Settings"));
                        ui.horizontal(|ui| {
                            ui.label(t!("cheki.border_width", default = "Border"));
                            ui.add(egui::Slider::new(&mut 0.04_f32, 0.01..=0.15));
                        });
                        ui.horizontal(|ui| {
                            ui.label(t!("cheki.bottom_extra", default = "Bottom"));
                            ui.add(egui::Slider::new(&mut 0.15_f32, 0.05..=0.35));
                        });

                        ui.add_space(3.0);

                        // Date Stamp
                        ui.separator();
                        ui.strong(t!("cheki.date_section", default = "Date Stamp"));
                        let mut preview_check = true;
                        ui.checkbox(
                            &mut preview_check,
                            t!("cheki.date_enabled", default = "Enable date stamp"),
                        );

                        ui.add_space(3.0);

                        // Text / Sign
                        ui.separator();
                        ui.strong(t!("cheki.text_section", default = "Text / Sign"));
                        let mut preview_text = String::new();
                        ui.add(
                            egui::TextEdit::singleline(&mut preview_text)
                                .desired_width(ui.available_width()),
                        );

                        ui.add_space(3.0);

                        // Random Character
                        ui.separator();
                        ui.strong(t!("cheki.dice_section", default = "Random Character"));
                        ui.add_space(2.0);
                        let _ = ui.button(
                            egui::RichText::new(t!(
                                "cheki.play_dice",
                                default = "Play Dice!"
                            ))
                            .strong(),
                        );
                    });
                });
            return;
        }

        // Image gallery
        ui.label(t!("cheki.select_image", default = "Select Image"));

        let current_selected = self.cheki_selected_index;
        let image_to_delete = crate::ui_components::render_horizontal_gallery(
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
            Some(|item: &(usize, &crate::packed_image::PackedImage)| {
                self.cheki_decorations.contains_key(&item.1.uuid)
            }),
            None::<fn(&_) -> Option<(bool, bool)>>,
            &mut |idx| {
                self.cheki_selected_index = Some(idx);
                self.cheki_preview_cache_key = None;
            },
            Some(&mut |idx| {
                log::info!("Delete button clicked for image index {} in cheki tab", idx);
            }),
        );

        if let Some(idx) = image_to_delete {
            self.delete_image_by_index(idx);
        }

        ui.separator();

        // Auto-select first if none selected
        if self.cheki_selected_index.is_none() && !self.packed_images.is_empty() {
            self.cheki_selected_index = Some(0);
        }

        let Some(idx) = self.cheki_selected_index else {
            return;
        };

        if idx >= self.packed_images.len() {
            self.cheki_selected_index = None;
            return;
        }

        let image_uuid = self.packed_images[idx].uuid;

        // Ensure decoration exists for this image
        self.cheki_decorations.entry(image_uuid).or_default();

        // Warning: both cheki and theme active
        let theme_name = self
            .export_config
            .theme_reg
            .selected_theme_read()
            .unique_name()
            .to_string();
        if theme_name != "nothing" {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(t!(
                        "cheki.theme_warning",
                        default =
                            "Note: Cheki decoration will be applied ON TOP of the selected theme"
                    ))
                    .size(11.0)
                    .color(ui.visuals().warn_fg_color),
                );
            });
            ui.add_space(3.0);
        }

        // Generate base texture (async) and process results
        self.process_cheki_base_texture(ui.ctx());
        self.start_cheki_base_texture_generation();

        // Vertical layout: full-width Canvas TOP | Controls BOTTOM
        let available = ui.available_size();
        let canvas_height = (available.y * 0.50).max(200.0);

        ui.allocate_ui(egui::vec2(available.x, canvas_height), |ui| {
            self.render_cheki_canvas(ui, image_uuid);
        });

        ui.separator();

        self.render_cheki_controls(ui, image_uuid);
    }

    /// Render the cheki canvas using egui-native drawing.
    /// Stickers are drawn rotated via custom mesh; handles shown for resize/rotate.
    fn render_cheki_canvas(&mut self, ui: &mut egui::Ui, image_uuid: uuid::Uuid) {
        let Some(base_texture) = self.cheki_preview_texture.clone() else {
            ui.centered_and_justified(|ui| {
                ui.label(t!("cheki.loading_preview", default = "Loading preview..."));
            });
            return;
        };
        let base_size = base_texture.size_vec2();
        let ctx = ui.ctx().clone();

        // Clone decoration to avoid borrow conflicts with sticker_storage
        let deco = self
            .cheki_decorations
            .get(&image_uuid)
            .cloned()
            .unwrap_or_default();

        let available_size = ui.available_size();

        // Compute canvas layout
        let (canvas_aspect, border_frac_x, border_frac_y, img_frac_w, img_frac_h) = if deco.enabled
        {
            let shorter = base_size.x.min(base_size.y);
            let border_px = shorter * deco.border_width;
            let bottom_extra_px = base_size.y * deco.bottom_extra;
            let canvas_w = base_size.x + border_px * 2.0;
            let canvas_h = base_size.y + border_px * 2.0 + bottom_extra_px;
            (
                canvas_w / canvas_h,
                border_px / canvas_w,
                border_px / canvas_h,
                base_size.x / canvas_w,
                base_size.y / canvas_h,
            )
        } else {
            (base_size.x / base_size.y, 0.0, 0.0, 1.0, 1.0)
        };

        let display_size = if available_size.x / canvas_aspect > available_size.y {
            egui::vec2(available_size.y * canvas_aspect, available_size.y)
        } else {
            egui::vec2(available_size.x, available_size.x / canvas_aspect)
        };

        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());
        let viewport_rect = response.rect;

        let center_offset = (available_size - display_size) / 2.0;
        let canvas_rect = egui::Rect::from_min_size(
            egui::pos2(
                viewport_rect.min.x + center_offset.x,
                viewport_rect.min.y + center_offset.y,
            ),
            display_size,
        );

        if deco.enabled {
            // 1. Draw border
            painter.rect_filled(canvas_rect, 0.0, deco.border_color);

            // 2. Draw base image inside border
            let img_rect = egui::Rect::from_min_size(
                egui::pos2(
                    canvas_rect.min.x + border_frac_x * canvas_rect.width(),
                    canvas_rect.min.y + border_frac_y * canvas_rect.height(),
                ),
                egui::vec2(
                    img_frac_w * canvas_rect.width(),
                    img_frac_h * canvas_rect.height(),
                ),
            );
            painter.image(
                base_texture.id(),
                img_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );

            // 3. Draw stickers with rotation and collect display info
            // Use a clipped painter if clip_stickers is enabled
            let sticker_painter = if deco.clip_stickers {
                painter.with_clip_rect(img_rect)
            } else {
                painter.clone()
            };

            let mut sticker_displays: Vec<StickerDisplay> = Vec::new();

            for (i, placed) in deco.dice_stickers.iter().enumerate() {
                if let Some(sticker_tex) = self.sticker_storage.get_texture(&ctx, placed.sticker_id)
                {
                    // Compute size preserving aspect ratio
                    let box_w = placed.scale * canvas_rect.width();
                    let box_h = placed.scale * canvas_rect.height();
                    let tex_size = sticker_tex.size_vec2();
                    let sticker_aspect = tex_size.x / tex_size.y;
                    let (sw, sh) = if box_w / sticker_aspect <= box_h {
                        (box_w, box_w / sticker_aspect)
                    } else {
                        (box_h * sticker_aspect, box_h)
                    };

                    let sx = canvas_rect.min.x + placed.x * canvas_rect.width();
                    let sy = canvas_rect.min.y + placed.y * canvas_rect.height();
                    let center = egui::pos2(sx + sw / 2.0, sy + sh / 2.0);
                    let rotation_rad = placed.rotation.to_radians();

                    // Draw rotated sticker image (clipped if enabled)
                    paint_rotated_image(
                        &sticker_painter,
                        sticker_tex.id(),
                        center,
                        egui::vec2(sw, sh),
                        rotation_rad,
                        egui::Color32::WHITE,
                    );

                    sticker_displays.push(StickerDisplay {
                        index: i,
                        center,
                        half_size: egui::vec2(sw / 2.0, sh / 2.0),
                        rotation_rad,
                    });
                }
            }

            // Determine which sticker to show handles on (hovered or active)
            let hover_pos = response.hover_pos();
            let active_sticker_idx = match &self.cheki_interaction_state {
                crate::app::ChekiInteractionState::DraggingSticker { sticker_index, .. }
                | crate::app::ChekiInteractionState::ResizingSticker { sticker_index, .. }
                | crate::app::ChekiInteractionState::RotatingSticker { sticker_index, .. } => {
                    Some(*sticker_index)
                }
                _ => None,
            };
            let hovered_sticker_idx = hover_pos.and_then(|pos| {
                sticker_displays
                    .iter()
                    .rev()
                    .find(|sd| {
                        sd.contains_point(pos)
                            || sd.corner_near_point(pos)
                            || sd.rotation_handle_near_point(pos)
                    })
                    .map(|sd| sd.index)
            });

            for sd in &sticker_displays {
                let is_active = active_sticker_idx == Some(sd.index);
                let is_hovered = hovered_sticker_idx == Some(sd.index);
                if is_active || is_hovered {
                    draw_sticker_handles(&painter, sd, is_active);
                }
            }

            // Clipped painter to prevent text/date from overflowing the canvas
            let clipped_painter = painter.with_clip_rect(canvas_rect);

            // 4. Draw text
            if !deco.text.is_empty() {
                let text_area_top_frac = border_frac_y + img_frac_h + border_frac_y;
                let text_area_h_frac = 1.0 - text_area_top_frac;

                let text_x = canvas_rect.min.x + deco.text_position_x * canvas_rect.width();
                let text_area_top = canvas_rect.min.y + text_area_top_frac * canvas_rect.height();
                let text_area_h = text_area_h_frac * canvas_rect.height();
                let text_y = text_area_top + deco.text_position_y * text_area_h;

                let font_px = (text_area_h * deco.font_size).max(8.0);
                let text_font_family = cheki_font_to_egui_family(&deco.font);
                let font_id = egui::FontId::new(font_px, text_font_family);

                clipped_painter.text(
                    egui::pos2(text_x, text_y),
                    egui::Align2::CENTER_CENTER,
                    &deco.text,
                    font_id,
                    deco.text_color,
                );

                // Text crosshair indicator
                let is_dragging_text = matches!(
                    &self.cheki_interaction_state,
                    crate::app::ChekiInteractionState::DraggingText { .. }
                );
                let indicator_color = if is_dragging_text {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::from_rgba_unmultiplied(0, 200, 200, 180)
                };
                let cross_size = 8.0;
                painter.line_segment(
                    [
                        egui::pos2(text_x - cross_size, text_y),
                        egui::pos2(text_x + cross_size, text_y),
                    ],
                    (2.0, indicator_color),
                );
                painter.line_segment(
                    [
                        egui::pos2(text_x, text_y - cross_size),
                        egui::pos2(text_x, text_y + cross_size),
                    ],
                    (2.0, indicator_color),
                );
            }

            // 5. Draw date stamp
            if deco.date_enabled && !deco.date_text.is_empty() {
                let top_border_h = border_frac_y * canvas_rect.height();
                let text_area_top_frac = border_frac_y + img_frac_h + border_frac_y;
                let text_area_h_frac = 1.0 - text_area_top_frac;
                let bottom_area_h = text_area_h_frac * canvas_rect.height();
                let pad = canvas_rect.width() * 0.02;

                let (date_pos, date_align) = match deco.date_position {
                    DatePosition::TopLeft => (
                        egui::pos2(
                            canvas_rect.min.x + pad,
                            canvas_rect.min.y + top_border_h / 2.0,
                        ),
                        egui::Align2::LEFT_CENTER,
                    ),
                    DatePosition::TopCenter => (
                        egui::pos2(
                            canvas_rect.center().x,
                            canvas_rect.min.y + top_border_h / 2.0,
                        ),
                        egui::Align2::CENTER_CENTER,
                    ),
                    DatePosition::TopRight => (
                        egui::pos2(
                            canvas_rect.max.x - pad,
                            canvas_rect.min.y + top_border_h / 2.0,
                        ),
                        egui::Align2::RIGHT_CENTER,
                    ),
                    DatePosition::BottomLeft => (
                        egui::pos2(
                            canvas_rect.min.x + pad,
                            canvas_rect.min.y
                                + text_area_top_frac * canvas_rect.height()
                                + bottom_area_h / 2.0,
                        ),
                        egui::Align2::LEFT_CENTER,
                    ),
                    DatePosition::BottomCenter => (
                        egui::pos2(
                            canvas_rect.center().x,
                            canvas_rect.min.y
                                + text_area_top_frac * canvas_rect.height()
                                + bottom_area_h / 2.0,
                        ),
                        egui::Align2::CENTER_CENTER,
                    ),
                    DatePosition::BottomRight => (
                        egui::pos2(
                            canvas_rect.max.x - pad,
                            canvas_rect.min.y
                                + text_area_top_frac * canvas_rect.height()
                                + bottom_area_h / 2.0,
                        ),
                        egui::Align2::RIGHT_CENTER,
                    ),
                };

                let date_font_px = match deco.date_position {
                    DatePosition::TopLeft | DatePosition::TopCenter | DatePosition::TopRight => {
                        (top_border_h * deco.date_font_size).max(8.0)
                    }
                    _ => (bottom_area_h * deco.date_font_size).max(8.0),
                };

                let date_font_family = cheki_font_to_egui_family(&deco.date_font);
                let date_font_id = egui::FontId::new(date_font_px, date_font_family);

                clipped_painter.text(
                    date_pos,
                    date_align,
                    &deco.date_text,
                    date_font_id,
                    deco.date_color,
                );
            }

            // Handle interactions (pass sticker geometry)
            self.handle_cheki_interactions(
                response,
                canvas_rect,
                image_uuid,
                base_size,
                &sticker_displays,
            );
        } else {
            // Cheki disabled — just show the base image
            painter.image(
                base_texture.id(),
                canvas_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }

    /// Handle interactive sticker drag/resize/rotate and text drag
    fn handle_cheki_interactions(
        &mut self,
        response: egui::Response,
        canvas_rect: egui::Rect,
        image_uuid: uuid::Uuid,
        base_size: egui::Vec2,
        sticker_displays: &[StickerDisplay],
    ) {
        use crate::app::ChekiInteractionState;

        let mouse_pos = response.hover_pos();

        // On drag stop → finalize
        if response.drag_stopped() {
            self.cheki_interaction_state = ChekiInteractionState::Idle;
            return;
        }

        match self.cheki_interaction_state.clone() {
            ChekiInteractionState::Idle => {
                if response.drag_started()
                    && let Some(pos) = mouse_pos
                {
                    // Priority: rotation handle > corner handle > sticker body > text
                    // Check rotation handles first (reverse for top-most)
                    for sd in sticker_displays.iter().rev() {
                        if sd.rotation_handle_near_point(pos) {
                            let angle = (pos.y - sd.center.y).atan2(pos.x - sd.center.x);
                            let original_rotation = self
                                .cheki_decorations
                                .get(&image_uuid)
                                .and_then(|d| d.dice_stickers.get(sd.index))
                                .map(|s| s.rotation)
                                .unwrap_or(0.0);
                            self.cheki_interaction_state = ChekiInteractionState::RotatingSticker {
                                sticker_index: sd.index,
                                center: sd.center,
                                start_angle: angle,
                                original_rotation,
                            };
                            return;
                        }
                    }

                    // Check corner handles
                    for sd in sticker_displays.iter().rev() {
                        if sd.corner_near_point(pos) {
                            let original_scale = self
                                .cheki_decorations
                                .get(&image_uuid)
                                .and_then(|d| d.dice_stickers.get(sd.index))
                                .map(|s| s.scale)
                                .unwrap_or(0.15);
                            // Determine which corner is grabbed and anchor to the opposite
                            let grabbed_idx = sd.nearest_corner_index(pos);
                            let opposite_idx = (grabbed_idx + 2) % 4;
                            let corners = sd.corners();
                            let anchor_screen_pos = corners[opposite_idx];
                            // Anchor's local offset sign from center
                            // TL=(-1,-1), TR=(1,-1), BR=(1,1), BL=(-1,1)
                            let anchor_sign = match opposite_idx {
                                0 => (-1.0, -1.0), // TL
                                1 => (1.0, -1.0),  // TR
                                2 => (1.0, 1.0),   // BR
                                _ => (-1.0, 1.0),  // BL
                            };
                            self.cheki_interaction_state = ChekiInteractionState::ResizingSticker {
                                sticker_index: sd.index,
                                start_pos: pos,
                                original_scale,
                                anchor_screen_pos,
                                anchor_sign,
                                original_half_size: (sd.half_size.x, sd.half_size.y),
                                rotation_rad: sd.rotation_rad,
                            };
                            return;
                        }
                    }

                    // Check sticker bodies
                    for sd in sticker_displays.iter().rev() {
                        if sd.contains_point(pos) {
                            let (ox, oy) = self
                                .cheki_decorations
                                .get(&image_uuid)
                                .and_then(|d| d.dice_stickers.get(sd.index))
                                .map(|s| (s.x, s.y))
                                .unwrap_or((0.0, 0.0));
                            self.cheki_interaction_state = ChekiInteractionState::DraggingSticker {
                                sticker_index: sd.index,
                                start_pos: pos,
                                original_x: ox,
                                original_y: oy,
                            };
                            return;
                        }
                    }

                    // Check text area
                    if let Some(deco) = self.cheki_decorations.get(&image_uuid)
                        && !deco.text.is_empty()
                        && deco.enabled
                    {
                        let (text_area_top, text_area_h) =
                            Self::compute_text_area(deco, &canvas_rect, base_size);
                        let text_x = canvas_rect.min.x + deco.text_position_x * canvas_rect.width();
                        let text_y = text_area_top + deco.text_position_y * text_area_h;

                        if pos.distance(egui::pos2(text_x, text_y)) < 20.0 {
                            self.cheki_interaction_state = ChekiInteractionState::DraggingText {
                                start_pos: pos,
                                original_x: deco.text_position_x,
                                original_y: deco.text_position_y,
                            };
                            return;
                        }
                    }
                }
            }
            ChekiInteractionState::DraggingSticker {
                sticker_index,
                start_pos,
                original_x,
                original_y,
            } => {
                if response.dragged()
                    && let Some(pos) = mouse_pos
                {
                    let delta_x = (pos.x - start_pos.x) / canvas_rect.width();
                    let delta_y = (pos.y - start_pos.y) / canvas_rect.height();
                    if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid)
                        && let Some(sticker) = deco.dice_stickers.get_mut(sticker_index)
                    {
                        sticker.x = (original_x + delta_x).clamp(0.0, 1.0);
                        sticker.y = (original_y + delta_y).clamp(0.0, 1.0);
                    }
                }
            }
            ChekiInteractionState::ResizingSticker {
                sticker_index,
                start_pos,
                original_scale,
                anchor_screen_pos,
                anchor_sign,
                original_half_size,
                rotation_rad,
            } => {
                if response.dragged()
                    && let Some(pos) = mouse_pos
                {
                    let start_dist = start_pos.distance(anchor_screen_pos).max(1.0);
                    let current_dist = pos.distance(anchor_screen_pos).max(1.0);
                    let scale_ratio = current_dist / start_dist;
                    let new_scale = (original_scale * scale_ratio).max(0.03);

                    // New half-sizes
                    let new_hw = original_half_size.0 * scale_ratio;
                    let new_hh = original_half_size.1 * scale_ratio;

                    // Compute new center so the anchor corner stays fixed
                    let cos_r = rotation_rad.cos();
                    let sin_r = rotation_rad.sin();
                    let local_dx = anchor_sign.0 * new_hw;
                    let local_dy = anchor_sign.1 * new_hh;
                    let rotated_dx = local_dx * cos_r - local_dy * sin_r;
                    let rotated_dy = local_dx * sin_r + local_dy * cos_r;
                    let new_center_x = anchor_screen_pos.x - rotated_dx;
                    let new_center_y = anchor_screen_pos.y - rotated_dy;

                    // Convert new center back to normalized top-left position
                    let new_x = (new_center_x - new_hw - canvas_rect.min.x) / canvas_rect.width();
                    let new_y = (new_center_y - new_hh - canvas_rect.min.y) / canvas_rect.height();

                    if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid)
                        && let Some(sticker) = deco.dice_stickers.get_mut(sticker_index)
                    {
                        sticker.scale = new_scale;
                        sticker.x = new_x.clamp(0.0, 1.0);
                        sticker.y = new_y.clamp(0.0, 1.0);
                    }
                }
            }
            ChekiInteractionState::RotatingSticker {
                sticker_index,
                center,
                start_angle,
                original_rotation,
            } => {
                if response.dragged()
                    && let Some(pos) = mouse_pos
                {
                    let current_angle = (pos.y - center.y).atan2(pos.x - center.x);
                    let delta_angle = current_angle - start_angle;
                    let new_rotation = original_rotation + delta_angle.to_degrees();

                    if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid)
                        && let Some(sticker) = deco.dice_stickers.get_mut(sticker_index)
                    {
                        sticker.rotation = new_rotation;
                    }
                }
            }
            ChekiInteractionState::DraggingText {
                start_pos,
                original_x,
                original_y,
            } => {
                if response.dragged()
                    && let Some(pos) = mouse_pos
                {
                    let delta_x = (pos.x - start_pos.x) / canvas_rect.width();
                    let text_area_h = self
                        .cheki_decorations
                        .get(&image_uuid)
                        .map(|d| {
                            let (_, h) = Self::compute_text_area(d, &canvas_rect, base_size);
                            h
                        })
                        .unwrap_or(canvas_rect.height() * 0.15);

                    let delta_y = if text_area_h > 0.0 {
                        (pos.y - start_pos.y) / text_area_h
                    } else {
                        0.0
                    };

                    if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid) {
                        deco.text_position_x = (original_x + delta_x).clamp(0.0, 1.0);
                        deco.text_position_y = (original_y + delta_y).clamp(0.0, 1.0);
                    }
                }
            }
        }

        // Right-click to delete sticker (use rotated hit-testing)
        if response.secondary_clicked()
            && let Some(pos) = mouse_pos
        {
            let delete_idx = sticker_displays
                .iter()
                .rev()
                .find(|sd| sd.contains_point(pos))
                .map(|sd| sd.index);

            if let Some(i) = delete_idx
                && let Some(deco) = self.cheki_decorations.get_mut(&image_uuid)
            {
                deco.dice_stickers.remove(i);
            }
        }

        // Scroll wheel on sticker to adjust scale
        if let Some(pos) = mouse_pos {
            let scroll_delta = response.ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0
                && let Some(sd) = sticker_displays
                    .iter()
                    .rev()
                    .find(|sd| sd.contains_point(pos))
                && let Some(deco) = self.cheki_decorations.get_mut(&image_uuid)
                && let Some(sticker) = deco.dice_stickers.get_mut(sd.index)
            {
                let scale_step = scroll_delta * 0.001;
                sticker.scale = (sticker.scale + scale_step).max(0.03);
            }
        }
    }

    /// Compute text area position and height in screen coordinates
    fn compute_text_area(
        deco: &ChekiDecoration,
        canvas_rect: &egui::Rect,
        base_size: egui::Vec2,
    ) -> (f32, f32) {
        let shorter = base_size.x.min(base_size.y);
        let border_px = shorter * deco.border_width;
        let bottom_extra_px = base_size.y * deco.bottom_extra;
        let canvas_h = base_size.y + 2.0 * border_px + bottom_extra_px;

        let text_area_top_frac = (2.0 * border_px + base_size.y) / canvas_h;
        let text_area_h_frac = bottom_extra_px / canvas_h;

        let text_area_top = canvas_rect.min.y + text_area_top_frac * canvas_rect.height();
        let text_area_h = text_area_h_frac * canvas_rect.height();

        (text_area_top, text_area_h)
    }

    /// Render cheki control panel
    fn render_cheki_controls(&mut self, ui: &mut egui::Ui, image_uuid: uuid::Uuid) {
        egui::ScrollArea::vertical()
            .id_salt("cheki_controls")
            .show(ui, |ui| {
                // Enable/disable toggle
                if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid) {
                    ui.checkbox(
                        &mut deco.enabled,
                        t!("cheki.enabled", default = "Enable Cheki"),
                    );
                    ui.add_space(5.0);
                }

                // ── 1. Random Character ──
                ui.separator();
                ui.strong(t!("cheki.dice_section", default = "Random Character"));
                ui.add_space(2.0);
                {
                    let total_count = self.sticker_storage.stickers.len();
                    let character_count = self.sticker_storage.character_stickers().len();

                    if total_count == 0 {
                        ui.label(
                            egui::RichText::new(t!(
                                "cheki.no_stickers_message",
                                default = "Add stickers in Settings → Sticker Storage first, then enable the Character toggle to use Play Dice."
                            ))
                            .size(11.0)
                            .color(ui.visuals().weak_text_color()),
                        );
                    } else if character_count == 0 {
                        ui.label(
                            egui::RichText::new(t!(
                                "cheki.no_character_stickers_message",
                                default = "Enable the character toggle on stickers in Settings to use Play Dice."
                            ))
                            .size(11.0)
                            .color(ui.visuals().weak_text_color()),
                        );
                    } else {
                        ui.label(format!(
                            "{}: {}",
                            t!(
                                "cheki.available_characters",
                                default = "Characters"
                            ),
                            character_count
                        ));

                        if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid) {
                            ui.checkbox(
                                &mut deco.clip_stickers,
                                t!("cheki.clip_stickers", default = "Clip stickers to image area"),
                            );
                            ui.checkbox(
                                &mut deco.allow_rotation,
                                t!("cheki.allow_rotation", default = "Allow rotation"),
                            );
                        }

                        ui.add_space(3.0);

                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    egui::RichText::new(t!(
                                        "cheki.play_dice",
                                        default = "Play Dice!"
                                    ))
                                    .strong(),
                                )
                                .clicked()
                            {
                                self.roll_cheki_dice(image_uuid);
                            }

                            if ui
                                .button(t!(
                                    "cheki.clear_stickers",
                                    default = "Clear"
                                ))
                                .clicked()
                                && let Some(deco) =
                                    self.cheki_decorations.get_mut(&image_uuid)
                                {
                                    deco.dice_stickers.clear();
                                }
                        });

                        ui.label(
                            egui::RichText::new(t!(
                                "cheki.play_dice_hint",
                                default = "Randomly places character stickers on the image"
                            ))
                            .size(10.0)
                            .color(ui.visuals().weak_text_color()),
                        );

                        if let Some(deco) = self.cheki_decorations.get(&image_uuid)
                            && !deco.dice_stickers.is_empty() {
                                ui.label(t!(
                                    "cheki.placed_stickers",
                                    default = "Placed"
                                ));
                            }

                        ui.add_space(3.0);
                        ui.label(
                            egui::RichText::new(t!(
                                "cheki.sticker_drag_hint",
                                default = "Drag to move. Corners to resize. Top handle to rotate. Right-click to delete. Scroll to scale."
                            ))
                            .size(10.0)
                            .color(ui.visuals().weak_text_color()),
                        );
                    }
                }

                ui.add_space(3.0);

                // ── 2. Border Settings ──
                ui.separator();
                ui.strong(t!("cheki.border_settings", default = "Border Settings"));
                ui.add_space(2.0);
                if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid) {
                    ui.horizontal(|ui| {
                        ui.label(t!("cheki.border_width", default = "Border"));
                        ui.add(
                            egui::Slider::new(&mut deco.border_width, 0.01..=0.15),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(t!("cheki.bottom_extra", default = "Bottom"));
                        ui.add(
                            egui::Slider::new(&mut deco.bottom_extra, 0.05..=0.35),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(t!("cheki.border_color", default = "Color"));
                        egui::color_picker::color_edit_button_srgba(
                            ui,
                            &mut deco.border_color,
                            egui::color_picker::Alpha::Opaque,
                        );
                    });
                }

                ui.add_space(3.0);

                // ── 3. Date Stamp ──
                ui.separator();
                ui.strong(t!("cheki.date_section", default = "Date Stamp"));
                ui.add_space(2.0);
                if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid) {
                    ui.checkbox(
                        &mut deco.date_enabled,
                        t!("cheki.date_enabled", default = "Enable date stamp"),
                    );

                    if deco.date_enabled {
                        ui.add(
                            egui::TextEdit::singleline(&mut deco.date_text)
                                .desired_width(ui.available_width())
                                .hint_text("2025.01.01"),
                        );

                        if ui
                            .button(t!(
                                "cheki.date_autofill",
                                default = "Auto-fill from EXIF"
                            ))
                            .clicked()
                            && let Some(pi) = self
                                .cheki_selected_index
                                .and_then(|idx| self.packed_images.get(idx))
                                && let Some(dt) = pi.view_exif.datetime {
                                    deco.date_text = dt.format("%Y.%m.%d").to_string();
                                }

                        ui.horizontal(|ui| {
                            use crate::effect::variable_text::VariableOrNot;
                            use crate::fonts::variable_font::BuiltinVariableFontIndex;

                            ui.label(t!("cheki.font", default = "Font"));

                            let is_variable =
                                matches!(deco.date_font, VariableOrNot::Variable(_));
                            if ui
                                .selectable_label(
                                    is_variable,
                                    t!("fonts_selector.variable.label"),
                                )
                                .on_hover_text(t!("fonts_selector.variable.hint"))
                                .clicked()
                                && !is_variable
                            {
                                deco.date_font = VariableOrNot::Variable(
                                    BuiltinVariableFontIndex::Barlow,
                                );
                                deco.date_font_weight = 300;
                            }

                            let is_others =
                                matches!(deco.date_font, VariableOrNot::Others(_));
                            if ui
                                .selectable_label(
                                    is_others,
                                    t!("fonts_selector.others.label"),
                                )
                                .on_hover_text(t!("fonts_selector.others.hint"))
                                .clicked()
                                && !is_others
                            {
                                deco.date_font = VariableOrNot::Others(
                                    crate::fonts::FONTS_UNIFY.builtin_select(
                                        crate::fonts::font_unify::BuiltinFontIndex::DynaPuff,
                                    ),
                                );
                            }

                            if let VariableOrNot::Variable(ref mut variable_select) =
                                deco.date_font
                            {
                                variable_select.update_ui(ui, "cheki_date_font");
                                let (start, end) = variable_select.get_font().range();
                                ui.add(
                                    egui::Slider::new(
                                        &mut deco.date_font_weight,
                                        start..=end,
                                    )
                                    .step_by(100.0),
                                );
                            } else if let VariableOrNot::Others(ref mut font_select) =
                                deco.date_font
                            {
                                font_select.update_ui(ui, "cheki_date_font");
                            }

                            if ui.button("↺").clicked() {
                                let default = ChekiDecoration::default();
                                deco.date_font = default.date_font;
                                deco.date_font_weight = default.date_font_weight;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label(t!("cheki.font_size", default = "Size"));
                            ui.add(egui::Slider::new(&mut deco.date_font_size, 0.1..=1.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label(t!("cheki.text_color", default = "Color"));
                            egui::color_picker::color_edit_button_srgba(
                                ui,
                                &mut deco.date_color,
                                egui::color_picker::Alpha::Opaque,
                            );
                        });

                        ui.add_space(3.0);
                        ui.label(t!("cheki.date_position", default = "Position"));
                        // 2x3 position grid
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut deco.date_position, DatePosition::TopLeft, "↖");
                            ui.selectable_value(&mut deco.date_position, DatePosition::TopCenter, "↑");
                            ui.selectable_value(&mut deco.date_position, DatePosition::TopRight, "↗");
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut deco.date_position, DatePosition::BottomLeft, "↙");
                            ui.selectable_value(&mut deco.date_position, DatePosition::BottomCenter, "↓");
                            ui.selectable_value(&mut deco.date_position, DatePosition::BottomRight, "↘");
                        });
                    }
                }

                ui.add_space(3.0);

                // ── 4. Text / Sign ──
                ui.separator();
                ui.strong(t!("cheki.text_section", default = "Text / Sign"));
                ui.add_space(2.0);
                if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid) {
                    ui.add(
                        egui::TextEdit::multiline(&mut deco.text)
                            .desired_width(ui.available_width()),
                    );
                    ui.horizontal(|ui| {
                        use crate::effect::variable_text::VariableOrNot;
                        use crate::fonts::variable_font::BuiltinVariableFontIndex;

                        ui.label(t!("cheki.font", default = "Font"));

                        let is_variable = matches!(deco.font, VariableOrNot::Variable(_));
                        if ui
                            .selectable_label(
                                is_variable,
                                t!("fonts_selector.variable.label"),
                            )
                            .on_hover_text(t!("fonts_selector.variable.hint"))
                            .clicked()
                            && !is_variable
                        {
                            deco.font =
                                VariableOrNot::Variable(BuiltinVariableFontIndex::Barlow);
                            deco.font_weight = 300;
                        }

                        let is_others = matches!(deco.font, VariableOrNot::Others(_));
                        if ui
                            .selectable_label(is_others, t!("fonts_selector.others.label"))
                            .on_hover_text(t!("fonts_selector.others.hint"))
                            .clicked()
                            && !is_others
                        {
                            deco.font = VariableOrNot::Others(
                                crate::fonts::FONTS_UNIFY.builtin_select(
                                    crate::fonts::font_unify::BuiltinFontIndex::default(),
                                ),
                            );
                        }

                        if let VariableOrNot::Variable(ref mut variable_select) = deco.font {
                            variable_select.update_ui(ui, "cheki_text_font");
                            let (start, end) = variable_select.get_font().range();
                            ui.add(
                                egui::Slider::new(&mut deco.font_weight, start..=end)
                                    .step_by(100.0),
                            );
                        } else if let VariableOrNot::Others(ref mut font_select) = deco.font {
                            font_select.update_ui(ui, "cheki_text_font");
                        }

                        if ui.button("↺").clicked() {
                            let default = ChekiDecoration::default();
                            deco.font = default.font;
                            deco.font_weight = default.font_weight;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(t!("cheki.font_size", default = "Size"));
                        ui.add(egui::Slider::new(&mut deco.font_size, 0.1..=1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label(t!("cheki.text_color", default = "Color"));
                        egui::color_picker::color_edit_button_srgba(
                            ui,
                            &mut deco.text_color,
                            egui::color_picker::Alpha::Opaque,
                        );
                    });

                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new(t!(
                            "cheki.text_drag_hint",
                            default = "Drag the crosshair on the preview to move text"
                        ))
                        .size(10.0)
                        .color(ui.visuals().weak_text_color()),
                    );
                }

                ui.add_space(5.0);

                // Remove decoration button
                ui.separator();
                if self.cheki_decorations.contains_key(&image_uuid)
                    && ui
                        .button(t!(
                            "cheki.remove_decoration",
                            default = "Remove Cheki Decoration"
                        ))
                        .clicked()
                {
                    self.cheki_decorations.remove(&image_uuid);
                }
            });
    }

    /// Roll dice to place random character stickers on a cheki image
    fn roll_cheki_dice(&mut self, image_uuid: uuid::Uuid) {
        let character_stickers = self.sticker_storage.character_stickers();
        if character_stickers.is_empty() {
            return;
        }

        let face_areas: Vec<crate::effect::sticker_storage::FaceArea> = self
            .packed_images
            .iter()
            .find(|pi| pi.uuid == image_uuid)
            .map(|pi| pi.configured_faces.clone())
            .unwrap_or_default();

        // Read options before clearing
        let allow_rotation = self
            .cheki_decorations
            .get(&image_uuid)
            .map(|d| d.allow_rotation)
            .unwrap_or(false);

        // Clear previous dice stickers before placing new ones
        if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid) {
            deco.dice_stickers.clear();
        }

        let config = crate::effect::dice::DicePlacementConfig {
            min_scale: 0.21,
            max_scale: 0.63,
            max_rotation: if allow_rotation { 15.0 } else { 0.0 },
            ..Default::default()
        };
        let mut rng = rand::rng();

        let new_stickers = crate::effect::dice::place_character_stickers(
            1000,
            1000,
            &face_areas,
            &[],
            &character_stickers,
            1,
            &config,
            &mut rng,
        );

        if let Some(deco) = self.cheki_decorations.get_mut(&image_uuid) {
            deco.dice_stickers = new_stickers;
        }
    }
}

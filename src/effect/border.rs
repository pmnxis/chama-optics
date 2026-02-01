/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Border {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
    pub color: egui::Color32,
    pub is_relative: bool,
}

const PREVIEW_SCALE: f32 = 0.05;

impl Border {
    #[allow(unused)]
    pub const fn uniform(size: u32, color: egui::Color32) -> Self {
        Self {
            left: size,
            right: size,
            top: size,
            bottom: size,
            color,
            is_relative: true,
        }
    }

    #[allow(unused)]
    pub const fn bottom(size: u32, color: egui::Color32) -> Self {
        Self {
            left: 0,
            right: 0,
            top: 0,
            bottom: size,
            color,
            is_relative: true,
        }
    }

    #[allow(unused)]
    pub const fn top_and_bottom(size: u32, color: egui::Color32) -> Self {
        Self {
            left: 0,
            right: 0,
            top: size,
            bottom: size,
            color,
            is_relative: true,
        }
    }

    fn __rel_size(value: u32, dyn_wh: u32) -> u32 {
        if value == 0 {
            0
        } else {
            ((value as f32) * ((dyn_wh as f32) / 2000.0)) as u32
        }
    }

    pub fn border_size(&self, dyn_wh: u32) -> (u32, u32, u32, u32) {
        if self.is_relative {
            (
                Self::__rel_size(self.left, dyn_wh),
                Self::__rel_size(self.right, dyn_wh),
                Self::__rel_size(self.top, dyn_wh),
                Self::__rel_size(self.bottom, dyn_wh),
            )
        } else {
            (self.left, self.right, self.top, self.bottom)
        }
    }

    pub fn interactive_watermark_padding(&self, w: u32, h: u32) -> u32 {
        let (left, right, top, bottom) = self.border_size(w.max(h));
        left.max(right).max(top.max(bottom)).min(w.min(h) / 2)
    }

    pub fn take_from_exist(&self, img: &image::DynamicImage, dyn_wh: u32) -> image::DynamicImage {
        use image::GenericImage;
        use imageproc::drawing::Canvas;

        let (w, h) = img.dimensions();

        let (left, right, top, bottom) = self.border_size(dyn_wh);
        let new_w = w + left + right;
        let new_h = h + top + bottom;
        let color = crate::theme::color32_to_rgba(self.color);
        let mut bordered = image::DynamicImage::new_rgba8(new_w, new_h);
        let inner = bordered.as_mut_rgba8().unwrap();

        unsafe {
            // more dangerous
            let color_u32 = u32::from_le_bytes(color.0);
            let buf = inner.as_flat_samples_mut().samples.as_mut_ptr();
            let len = inner.as_flat_samples_mut().samples.len();

            let pixel_count = len / 4;
            let buf32 = buf as *mut u32;

            for i in 0..pixel_count {
                core::ptr::write(buf32.add(i), color_u32);
            }
        }

        // unsafe {
        //     let buf = inner.as_flat_samples_mut().samples;
        //     let len = buf.len();
        //     let mut i = 0;
        //     while i + 3 < len {
        //         *buf.get_unchecked_mut(i) = color[0];
        //         *buf.get_unchecked_mut(i + 1) = color[1];
        //         *buf.get_unchecked_mut(i + 2) = color[2];
        //         *buf.get_unchecked_mut(i + 3) = color[3];
        //         i += 4;
        //     }
        // }

        bordered.copy_from(img, left, top).unwrap();
        bordered
    }

    pub fn ui_config(
        &mut self,
        ui: &mut egui::Ui,
        default: &'static Self,
        limit: &'static BorderLimit,
    ) {
        macro_rules! side_show {
            ($ui: expr, $side:ident, $px_word: expr) => {
                $ui.label(t!(concat!("effects.border.", stringify!($side))));
                $ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.$side)
                            .speed(1)
                            .range(limit.$side.0..=limit.$side.1),
                    );
                    ui.label($px_word)
                        .on_hover_text(t!("effects.border.padding_description"))
                });
            };
        }

        let px_word = if self.is_relative {
            t!("effects.border.padding")
        } else {
            t!("scale_config.px_std")
        };

        ui.horizontal(|ui| {
            egui::Grid::new("border_config_grid")
                .num_columns(2)
                .spacing([4.0, 3.0])
                .striped(true)
                .show(ui, |ui| {
                    side_show!(ui, top, px_word.clone());
                    ui.end_row();

                    side_show!(ui, right, px_word.clone());
                    ui.end_row();

                    side_show!(ui, bottom, px_word.clone());
                    ui.end_row();

                    side_show!(ui, left, px_word);
                    ui.end_row();

                    ui.label(t!("effects.border.color"));
                    egui::widgets::color_picker::color_edit_button_srgba(
                        ui,
                        &mut self.color,
                        egui::color_picker::Alpha::Opaque,
                    );
                    ui.end_row();

                    ui.label(t!("effects.border.padding_mode.label"));
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.is_relative,
                            false,
                            t!("effects.border.padding_mode.absolute"),
                        )
                        .on_hover_text(t!("effects.border.padding_mode.absolute_hint"));
                        ui.selectable_value(
                            &mut self.is_relative,
                            true,
                            t!("effects.border.padding_mode.relative"),
                        )
                        .on_hover_text(t!("effects.border.padding_mode.relative_hint"));
                    });
                    ui.end_row();
                });

            ui.separator();

            // Show Preview
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                let (rect, _response) =
                    ui.allocate_exact_size(egui::vec2(150.0, 100.0), egui::Sense::hover());

                let painter = ui.painter();
                // Rescaled border size
                let left = self.left as f32 * PREVIEW_SCALE;
                let right = self.right as f32 * PREVIEW_SCALE;
                let top = self.top as f32 * PREVIEW_SCALE;
                let bottom = self.bottom as f32 * PREVIEW_SCALE;

                // Inner rect
                let inner = rect;
                // Outer border
                let outer = egui::Rect::from_min_max(
                    inner.min - egui::vec2(left, top),
                    inner.max + egui::vec2(right, bottom),
                );

                // Fill Outer
                painter.rect_filled(outer, 0.0, self.color);
                painter.rect_filled(inner, 0.0, egui::Color32::DARK_BLUE);

                // Inner Box
                painter.rect_stroke(
                    inner,
                    0.0,
                    egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
                    egui::StrokeKind::Inside,
                );

                painter.rect_stroke(
                    outer,
                    0.0,
                    egui::Stroke::new(2.0, self.color),
                    egui::StrokeKind::Inside,
                );

                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Preview",
                    egui::FontId::proportional(12.0),
                    egui::Color32::LIGHT_BLUE,
                );

                painter.text(
                    {
                        let mut ret = rect.center();
                        ret.y += 10.0;
                        ret
                    },
                    egui::Align2::CENTER_TOP,
                    t!("effects.border.preview_desc"),
                    egui::FontId::proportional(6.0),
                    egui::Color32::LIGHT_BLUE,
                );

                // right-bottom aligned
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                    if ui
                        .button(t!("effects.border.default"))
                        .on_hover_text(t!("effects.border.default_desc"))
                        .clicked()
                    {
                        *self = default.clone();
                    }
                });
            });
        });
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct BorderLimit {
    pub left: (u32, u32),
    pub right: (u32, u32),
    pub top: (u32, u32),
    pub bottom: (u32, u32),
}

impl BorderLimit {
    #[allow(unused)]
    pub const fn uniform(start: u32, end: u32) -> Self {
        Self {
            left: (start, end),
            right: (start, end),
            top: (start, end),
            bottom: (start, end),
        }
    }

    #[allow(unused)]
    pub const fn bottom(start: u32, end: u32) -> Self {
        Self {
            left: (0, end),
            right: (0, end),
            top: (0, end),
            bottom: (start, end),
        }
    }

    #[allow(unused)]
    pub const fn top_and_bottom(start: u32, end: u32) -> Self {
        Self {
            left: (0, end),
            right: (0, end),
            top: (start, end),
            bottom: (start, end),
        }
    }
}

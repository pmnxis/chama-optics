/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

// todo - iOS version doesn't have watermark feature yet, need to fix cfg-hell later

#[cfg(not(feature = "ios_integration"))]
use crate::effect::draw_with_transparency::{
    draw_text_screen_transparency_mut, draw_text_transparency_mut,
};
#[cfg(not(feature = "ios_integration"))]
use crate::theme::text_dimensions;

#[cfg(not(feature = "ios_integration"))]
use rust_i18n::t;

// Desktop version with full font selection support
#[cfg(not(feature = "ios_integration"))]
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Watermark {
    pub is_enabled: bool,
    pub is_screen_overlay: bool,
    font_color: egui::Color32,
    font_size: f32,
    font: crate::FontSelection,
    label: String,
    position: u8,
}

// iOS version - watermark disabled (no font selection system)
#[cfg(feature = "ios_integration")]
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct Watermark {
    pub is_enabled: bool,
}

#[cfg(not(feature = "ios_integration"))]
const DEFAULT_COLOR: image::Rgba<u8> = image::Rgba([232, 232, 232, 255]);
#[cfg(not(feature = "ios_integration"))]
const DEFAULT_FONT_SIZE: u32 = 25;
#[cfg(not(feature = "ios_integration"))]
const POSITION_ICONS: [&str; 9] = ["↖", "↑", "↗", "←", "●", "→", "↙", "↓", "↘"];

#[cfg(not(feature = "ios_integration"))]
impl core::default::Default for Watermark {
    fn default() -> Self {
        use imageproc::integral_image::ArrayData;
        let [r, g, b, a] = DEFAULT_COLOR.data();

        Self {
            is_enabled: false,
            is_screen_overlay: false,
            font_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
            font_size: DEFAULT_FONT_SIZE as f32,
            font: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::NtSansMed),
            label: "".to_owned(),
            position: 8,
        }
    }
}

#[cfg(feature = "ios_integration")]
impl core::default::Default for Watermark {
    fn default() -> Self {
        Self { is_enabled: false }
    }
}

// Desktop implementation with full functionality
#[cfg(not(feature = "ios_integration"))]
impl Watermark {
    fn rel_size<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> f32 {
        size.as_() * (self.font_size / (DEFAULT_FONT_SIZE as f32)) * (dyn_wh.as_() / 4000.0)
    }

    fn rel_scale<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> ab_glyph::PxScale {
        ab_glyph::PxScale::from(self.rel_size(size, dyn_wh))
    }

    fn position(
        &self,
        (dyn_w, dyn_h): (u32, u32),
        (txt_w, txt_h): (f32, f32),
        (ascent, descent): (f32, f32),
        margin: i32,
    ) -> (i32, i32) {
        let (dyn_w, dyn_h) = (dyn_w as f32, dyn_h as f32);
        let margin = margin as f32;

        match self.position {
            // Top Line
            1 => (margin as i32, margin as i32),
            2 => (((dyn_w - txt_w) / 2.0) as i32, margin as i32),
            3 => ((dyn_w - txt_w - margin) as i32, margin as i32),

            // Center Line
            4 => (
                margin as i32,
                ((dyn_h / 2.0) - (txt_h / 2.0) - ((ascent - descent) / 2.0)) as i32,
            ),
            5 => (
                ((dyn_w - txt_w) / 2.0) as i32,
                ((dyn_h / 2.0) - (txt_h / 2.0) - ((ascent - descent) / 2.0)) as i32,
            ),
            6 => (
                (dyn_w - txt_w - margin) as i32,
                ((dyn_h / 2.0) - (txt_h / 2.0) - ((ascent - descent) / 2.0)) as i32,
            ),

            // Bottom line
            7 => (margin as i32, (dyn_h - txt_h - margin - descent) as i32),
            8 => (
                ((dyn_w - txt_w) / 2.0) as i32,
                (dyn_h - margin - ascent) as i32,
            ),
            9 => (
                (dyn_w - txt_w - margin) as i32,
                (dyn_h - margin - ascent) as i32,
            ),

            // Same as position 8
            _ => (
                ((dyn_w - txt_w) / 2.0) as i32,
                (dyn_h - margin - ascent) as i32,
            ),
        }
    }

    pub fn apply(
        &self,
        dyn_image: &mut image::DynamicImage,
        margin: Option<i32>,
    ) -> Result<(), image::ImageError> {
        #[allow(unused)]
        use {ab_glyph::Font, ab_glyph::ScaleFont};

        let color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);

        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);
        let font = crate::FONTS_UNIFY.search(&self.font)?;

        let margin = margin.unwrap_or(self.rel_size(120, dyn_wh).trunc() as i32);
        let tp = self.font_color[3];

        let scale = self.rel_scale(75, dyn_wh);
        let (txt_w, txt_h) = text_dimensions(scale, &font, &self.label);

        let (xxx, yyy) = self.position(
            (dyn_w, dyn_h),
            (txt_w, txt_h),
            (
                font.as_scaled(scale).ascent(),
                font.as_scaled(scale).descent(),
            ),
            margin,
        );

        // // todo - Supports transparent watermarks to suit transparency
        // imageproc::drawing::draw_text_mut(dyn_image, color, xxx, yyy, scale, &font, &self.label);

        if self.is_screen_overlay {
            #[rustfmt::skip]
            draw_text_screen_transparency_mut(dyn_image, color, xxx, yyy, scale, &font, tp, &self.label);
        } else {
            #[rustfmt::skip]
            draw_text_transparency_mut(dyn_image, color, xxx, yyy, scale, &font, tp, &self.label);
        }

        Ok(())
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // left
            ui.checkbox(&mut self.is_enabled, t!("watermark.is_enabled"));

            self.font.update_ui_with_default_label(ui);

            ui.horizontal(|ui| {
                egui::color_picker::color_edit_button_srgba(
                    ui,
                    &mut self.font_color,
                    egui::color_picker::Alpha::BlendOrAdditive,
                );
                ui.add_space(1.0);
                ui.add(
                    egui::Slider::new(&mut self.font_size, 1.0..=100.0).text(t!("theme.font_size")),
                )
                .on_hover_text(t!(
                    "theme.font_size_description",
                    default = DEFAULT_FONT_SIZE
                ));
            });
            ui.label(t!("watermark.text"));
            ui.add(egui::TextEdit::singleline(&mut self.label).desired_width(200.0));
            ui.label(t!("watermark.position"));

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    for row in 0..3 {
                        ui.horizontal(|ui| {
                            for col in 0..3 {
                                let i = (row * 3 + col + 1) as u8;
                                let selected = self.position == i;
                                let label = POSITION_ICONS[(i - 1) as usize];
                                let hover = t!(format!("watermark.position.{}", i));
                                if ui
                                    .selectable_label(selected, label)
                                    .on_hover_text(hover)
                                    .clicked()
                                {
                                    self.position = i;
                                }
                            }
                        });
                    }
                });

                // corner
                ui.vertical(|ui| {
                    ui.label(t!("watermark.blend_mode.label"));

                    ui.horizontal(|ui| {
                        let normal_selected = !self.is_screen_overlay;
                        let screen_selected = self.is_screen_overlay;

                        if ui
                            .selectable_label(normal_selected, t!("watermark.blend_mode.normal"))
                            .on_hover_text(t!("watermark.blend_mode.normal_hint"))
                            .clicked()
                        {
                            self.is_screen_overlay = false;
                        }

                        if ui
                            .selectable_label(screen_selected, t!("watermark.blend_mode.screen"))
                            .on_hover_text(t!("watermark.blend_mode.screen_hint"))
                            .clicked()
                        {
                            self.is_screen_overlay = true;
                        }
                    });
                });
            });
        });
    }
}

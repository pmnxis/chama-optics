/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::text_dimensions;
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Watermark {
    is_enabled: bool,
    font_color: egui::Color32,
    font_size: f32,
    font: crate::FontSelection,
    label: String,
}

const DEFAULT_COLOR: image::Rgba<u8> = image::Rgba([232, 232, 232, 255]);
const DEFAULT_FONT_SIZE: u32 = 10;

impl core::default::Default for Watermark {
    fn default() -> Self {
        use imageproc::integral_image::ArrayData;
        let [r, g, b, a] = DEFAULT_COLOR.data();

        Self {
            is_enabled: false,
            font_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
            font_size: DEFAULT_FONT_SIZE as f32,
            font: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::NtSansMed),
            label: "".to_owned(),
        }
    }
}

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

    pub fn apply(&self, dyn_image: &mut image::DynamicImage) -> Result<(), image::ImageError> {
        #[allow(unused)]
        use {ab_glyph::Font, ab_glyph::ScaleFont};

        let color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);

        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);
        let font = crate::FONTS_UNIFY.search(&self.font)?;

        let margin = self.rel_size(120, dyn_wh).trunc() as i32;
        let base_y = dyn_h as i32 - margin;
        let scale = self.rel_scale(75, dyn_wh);
        let (txt_w, _txt_h) = text_dimensions(scale, &font, &self.label);

        let yyy = base_y;
        let mut xxx = (dyn_w as f32) / 2.0;
        // todo - need to select where the position
        xxx -= (txt_w / 2.0).round();

        // todo - Supports transparent watermarks to suit transparency
        // todo - ascent position pollution
        imageproc::drawing::draw_text_mut(
            dyn_image,
            color,
            xxx as i32,
            (yyy as f32 - font.as_scaled(scale).ascent()) as i32,
            scale,
            &font,
            &self.label,
        );

        Ok(())
    }

    pub fn update_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                // left
                ui.checkbox(&mut self.is_enabled, t!("watermark.is_enabled"));

                self.font.update_ui_with_default_label(ctx, ui);
                ui.add(
                    egui::Slider::new(&mut self.font_size, 1.0..=100.0).text(t!("theme.font_size")),
                )
                .on_hover_text(t!(
                    "theme.font_size_description",
                    default = DEFAULT_FONT_SIZE
                ));
                ui.label(t!("watermark.text"));
                ui.add(egui::TextEdit::singleline(&mut self.label).desired_width(200.0));
            });

            ui.vertical(|ui| {
                // right
                ui.add_space(1.0);
                egui::color_picker::color_picker_color32(
                    ui,
                    &mut self.font_color,
                    egui::color_picker::Alpha::BlendOrAdditive,
                );
            });
        });
    }
}

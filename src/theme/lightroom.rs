/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::effect::variable_text::{VariableTextSlot, VariableTextSlotDefault};
use crate::theme::Theme;
use ab_glyph::{Font, ScaleFont};
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Lightroom {
    border: crate::effect::border::Border,
    pub font_color: egui::Color32,
    font_height: u32,
    pub left: VariableTextSlot,
    pub center: VariableTextSlot,
    pub right: VariableTextSlot,
    // pub width_aligned: bool,
    show_hint: bool,
}

const DEFAULT_FONT_HEIGHT: u32 = 60;

const DEFAULT_LEFT: VariableTextSlotDefault = VariableTextSlotDefault::with_digital7(
    "[ISO{iso_speed}]    [{exposure}s]    [F{fnumber}]    [{focal}mm]",
);

const DEFAULT_CENTER: VariableTextSlotDefault =
    VariableTextSlotDefault::with_digital7("[{camera_mnf}  ][  {camera_model}]    [{lens_model}]");

const DEFAULT_RIGHT: VariableTextSlotDefault = VariableTextSlotDefault::with_digital7("");

const DEFAULT_BORDER_DEFAULT_SIZE: u32 = 90;
const DEFAULT_BORDER_MIN_SIZE: u32 = 50;
const DEFAULT_LIMIT: crate::effect::border::BorderLimit =
    crate::effect::border::BorderLimit::bottom(DEFAULT_BORDER_MIN_SIZE, 900);
const DEFAULT_BORDER: crate::effect::border::Border =
    crate::effect::border::Border::bottom(DEFAULT_BORDER_DEFAULT_SIZE, egui::Color32::BLACK);
const FILM_COLOR: image::Rgba<u8> = image::Rgba([255, 153, 0, 255]);

impl core::default::Default for Lightroom {
    fn default() -> Self {
        use imageproc::integral_image::ArrayData;
        let [r, g, b, a] = FILM_COLOR.data();

        Self {
            border: DEFAULT_BORDER,
            font_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
            font_height: DEFAULT_FONT_HEIGHT,
            left: VariableTextSlot::from_default(&DEFAULT_LEFT),
            center: VariableTextSlot::from_default(&DEFAULT_CENTER),
            right: VariableTextSlot::from_default(&DEFAULT_RIGHT),
            // width_aligned: true,
            show_hint: false,
        }
    }
}

impl Lightroom {
    // 0.0~1.0
    fn rel_size<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> f32 {
        size.as_() * (dyn_wh.as_())
    }

    fn rel_scale<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> ab_glyph::PxScale {
        ab_glyph::PxScale::from(self.rel_size(size, dyn_wh))
    }
}

impl Theme for Lightroom {
    fn unique_name(&self) -> &'static str {
        "lightroom"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        t!("theme.lightroom.title")
    }

    fn apply_to_image(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        let scale_config = &export_config.scale_config;
        let font_color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);
        let dyn_image: image::DynamicImage = pi.with_scale_and_orientation(*scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);

        let (ll, rr, _tt, bb) = self.border.border_size(dyn_wh);
        let font_height_ratio = self.font_height.clamp(10, 80) as f32 / 100.0;
        let txt_scale = self.rel_scale(font_height_ratio, bb);
        let mut new_image = self.border.take_from_exist(&dyn_image, dyn_wh);

        // TODO - Need more profer way
        let y = new_image.height() - (bb / 2);

        // left
        let left_font = &self.left.get_font();
        let left_txt = self.left.format_custom(&pi.view_exif);
        let left_x = ((bb / 10).min(2) + ll) as i32;
        let (left_www, _) = crate::theme::text_dimensions_with_fallback(
            txt_scale,
            left_font,
            self.left.weight,
            &left_txt,
        );
        crate::theme::draw_text_with_fallback(
            &mut new_image,
            font_color,
            left_x,
            (y as f32
                - ((left_font.as_scaled(txt_scale).ascent()
                    + left_font.as_scaled(txt_scale).descent().abs())
                    * 0.55)) as i32,
            txt_scale,
            left_font,
            self.left.weight,
            &left_txt,
        );

        // center
        let y = new_image.height() - (bb / 2);
        let center_font = &self.center.get_font();
        let center_txt = self.center.format_custom(&pi.view_exif);
        let (center_www, _) = crate::theme::text_dimensions_with_fallback(
            txt_scale,
            center_font,
            self.center.weight,
            &center_txt,
        );

        let center_x = {
            let (min_spacing, _) = crate::theme::text_dimensions_with_fallback(
                txt_scale,
                center_font,
                self.center.weight,
                "      ",
            );
            let center_x = ((dyn_w as f32 - center_www) / 2.0) + ll as f32;
            let left_max = left_www + left_x as f32;
            center_x.max(left_max + min_spacing)
        }
        .floor() as i32;

        crate::theme::draw_text_with_fallback(
            &mut new_image,
            font_color,
            center_x,
            (y as f32
                - ((center_font.as_scaled(txt_scale).ascent()
                    + center_font.as_scaled(txt_scale).descent().abs())
                    * 0.55)) as i32,
            txt_scale,
            center_font,
            self.center.weight,
            &center_txt,
        );

        // right
        let right_font = &self.right.get_font();
        let right_txt = self.right.format_custom(&pi.view_exif);
        let (right_www, _) = crate::theme::text_dimensions_with_fallback(
            txt_scale,
            right_font,
            self.right.weight,
            &right_txt,
        );
        let right_x = (new_image.width() - rr - (bb / 10).min(2)) as i32 - (right_www as i32);
        crate::theme::draw_text_with_fallback(
            &mut new_image,
            font_color,
            right_x,
            (y as f32
                - ((right_font.as_scaled(txt_scale).ascent()
                    + right_font.as_scaled(txt_scale).descent().abs())
                    * 0.55)) as i32,
            txt_scale,
            right_font,
            self.right.weight,
            &right_txt,
        );

        Ok(new_image)
    }

    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        let scale_config = &export_config.scale_config;
        let dyn_image = pi.with_scale_and_orientation(*scale_config)?;
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh = dyn_w.max(dyn_h);
        let (_ll, _rr, _tt, bb) = self.border.border_size(dyn_wh);

        let temp_margin = self.border.interactive_watermark_padding(dyn_w, dyn_h) / 6;
        let border_margin = (temp_margin * 5).max(bb) + temp_margin;

        let mut themed_image = self.apply_to_image(pi, export_config)?;
        export_config.save_image(&mut themed_image, Some(border_margin as i32), output_path)
    }

    fn ui_config(&mut self, ui: &mut egui::Ui) {
        self.border.ui_config(ui, &DEFAULT_BORDER, &DEFAULT_LIMIT);

        ui.vertical(|ui| {
            // Padding configuration
            ui.add_space(4.0);

            // Own configuration
            egui::Grid::new("lightroom_config_grid")
                .num_columns(2)
                .spacing([4.0, 3.0])
                .show(ui, |ui| {
                    ui.label(t!("theme.font_color"));
                    egui::widgets::color_picker::color_edit_button_srgba(
                        ui,
                        &mut self.font_color,
                        egui::color_picker::Alpha::Opaque,
                    );
                    ui.end_row();

                    ui.label(t!("theme.font_height_ratio.label"))
                        .on_hover_text(t!("theme.font_height_ratio.hint"));
                    ui.horizontal(|ui| {
                        ui.add(
                            // [slider_width, 23.0],
                            egui::Slider::new(&mut self.font_height, 50..=80).step_by(0.01),
                        );
                        ui.label("% ");
                        if ui.button("↺").clicked() {
                            self.font_height = DEFAULT_FONT_HEIGHT;
                        }
                    });
                    ui.end_row();

                    self.left.ui(ui, t!("theme.exif_left_bot"), &DEFAULT_LEFT);
                    ui.end_row();

                    self.center
                        .ui(ui, t!("theme.exif_center_bot"), &DEFAULT_CENTER);
                    ui.end_row();

                    self.right
                        .ui(ui, t!("theme.exif_right_bot"), &DEFAULT_RIGHT);
                    ui.end_row();
                });

            ui.horizontal(|ui| {
                ui.label(t!("theme.template_format_hint.title"));
                if ui.button("?").clicked() {
                    self.show_hint = !self.show_hint;
                }
            });

            if self.show_hint {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(t!("theme.template_format_hint.description"));
                });
            }
        });
    }

    fn is_ui_config_available(&self) -> bool {
        true
    }
}

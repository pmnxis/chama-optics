/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::theme::Theme;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
use crate::update_param;
use ab_glyph::{Font, ScaleFont};
use imageproc::integral_image::ArrayData;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
use rust_i18n::t;

#[derive(serde::Deserialize, serde::Serialize)]
#[cfg_attr(
    not(any(feature = "ios_integration", feature = "android_integration")),
    derive(chama_optics_macros::ThemeParameters)
)]
pub struct FilmDate {
    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub font: crate::FontSelection,
    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub font_italic: crate::FontSelection,
    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    pub font_file: String,
    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    pub font_italic_file: String,

    #[cfg_attr(
        not(any(feature = "ios_integration", feature = "android_integration")),
        param(color, label_key = "theme.font_color", default = "rgb(255, 138, 0)")
    )]
    pub font_color: egui::Color32,

    #[cfg_attr(
        not(any(feature = "ios_integration", feature = "android_integration")),
        param(color, label_key = "theme.glow_color", default = "rgb(238, 140, 128)")
    )]
    pub glow_color: egui::Color32,

    #[cfg_attr(
        not(any(feature = "ios_integration", feature = "android_integration")),
        param(
            slider,
            label_key = "theme.font_size",
            min = 10,
            max = 100,
            default_const = "DEFAULT_FONT_SIZE"
        )
    )]
    pub font_size: u32,

    #[cfg_attr(
        not(any(feature = "ios_integration", feature = "android_integration")),
        param(
            slider,
            label_key = "theme.glow_range",
            min = 0,
            max = 20,
            default_const = "DEFAULT_GLOW_GAIN"
        )
    )]
    pub glow_gain: u32,

    pub hide_camera_exif: bool,
    pub show_ps: bool,
}

const FILM_COLOR: image::Rgba<u8> = image::Rgba([255, 138, 0, 255]);
const FILM_COLOR_GLOW: image::Rgba<u8> = image::Rgba([238, 140, 128, 255]);
const DEFAULT_FONT_SIZE: u32 = 25;
const DEFAULT_GLOW_GAIN: u32 = 8;
#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
use super::{DEFAULT_DIGITAL7_FONT_FILE, DEFAULT_DIGITAL7_ITALIC_FILE};

impl core::default::Default for FilmDate {
    fn default() -> Self {
        let [r, g, b, a] = FILM_COLOR.data();
        let [gr, gg, gb, ga] = FILM_COLOR_GLOW.data();
        Self {
            #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
            font: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::Digital7),
            #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
            font_italic: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::Digital7Italic),
            #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
            font_file: DEFAULT_DIGITAL7_FONT_FILE.to_string(),
            #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
            font_italic_file: DEFAULT_DIGITAL7_ITALIC_FILE.to_string(),
            font_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
            glow_color: egui::Color32::from_rgba_unmultiplied_const(gr, gg, gb, ga),
            font_size: DEFAULT_FONT_SIZE,
            glow_gain: DEFAULT_GLOW_GAIN,
            hide_camera_exif: true,
            show_ps: false,
        }
    }
}

#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
impl crate::theme::parameter_schema::ThemeParameters for FilmDate {
    fn schema(&self) -> crate::theme::parameter_schema::ThemeSchema {
        crate::theme::parameter_schema::ThemeSchema {
            theme_name: "film_date".to_string(),
            theme_label: "Film Date".to_string(),
            parameters: vec![],
        }
    }

    fn update_from_json(
        &mut self,
        _updates: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), String> {
        Ok(())
    }
}

impl FilmDate {
    fn rel_size<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> f32 {
        size.as_() * (self.font_size as f32 / (DEFAULT_FONT_SIZE as f32)) * (dyn_wh.as_() / 4000.0)
    }

    fn rel_scale<F: Copy + num_traits::AsPrimitive<f32>, G: Copy + num_traits::AsPrimitive<f32>>(
        &self,
        size: F,
        dyn_wh: G,
    ) -> ab_glyph::PxScale {
        ab_glyph::PxScale::from(self.rel_size(size, dyn_wh))
    }
}

// iOS helper for loading font from file
#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
impl FilmDate {
    fn load_font(font_file: &str) -> Result<ab_glyph::FontArc, image::ImageError> {
        use crate::effect::variable_text::get_fonts_base_directory;
        let base_dir = get_fonts_base_directory();
        let full_path = if base_dir.is_empty() {
            std::path::PathBuf::from(font_file)
        } else {
            std::path::PathBuf::from(&base_dir).join(font_file)
        };
        let data = std::fs::read(&full_path).map_err(|e| {
            image::ImageError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, e))
        })?;
        ab_glyph::FontArc::try_from_vec(data).map_err(|_| {
            image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse font",
            ))
        })
    }
}

impl Theme for FilmDate {
    fn unique_name(&self) -> &'static str {
        "film_date"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        {
            t!("theme.film_date")
        }
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        {
            std::borrow::Cow::Borrowed("Film Date")
        }
    }

    fn apply_to_image(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        let exif = &pi.view_exif;
        let color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.font_color);
        let glow_color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.glow_color);
        let scale_config = &export_config.scale_config;
        let mut dyn_image: image::DynamicImage = pi.with_scale_and_orientation(*scale_config)?;
        let mut luma_text = image::GrayImage::new(dyn_image.width(), dyn_image.height());
        let (dyn_w, dyn_h) = (dyn_image.width(), dyn_image.height());
        let dyn_wh: f32 = (dyn_w as f32).max(dyn_h as f32);

        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        let font = crate::FONTS_UNIFY.search(&self.font)?;
        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        let font_italic = crate::FONTS_UNIFY.search(&self.font_italic)?;
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        let font = Self::load_font(&self.font_file)?;
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        let font_italic = Self::load_font(&self.font_italic_file)?;

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $font:expr, $scale:expr, $text:expr) => {
                crate::theme::draw_text_with_fallback_luma(&mut luma_text, image::Luma([255]), ($xxx) as i32, ($yyy as f32 - $font.as_scaled($scale).ascent()) as i32, $scale, $font, 400, $text);
            };
        }

        let margin = self.rel_size(120, dyn_wh).trunc() as i32;
        let base_y = dyn_h as i32 - margin;

        // Left
        let mut y = base_y as f32;
        let cam_scale = self.rel_scale(75, dyn_wh);
        let left_list = {
            let mut list = Vec::new();
            if !self.hide_camera_exif {
                if !(exif.camera_mnf.is_empty() || exif.camera_model.is_empty()) {
                    list.push(format!("{}  {}", exif.camera_mnf, exif.camera_model));
                }
                if !exif.lens_model.is_empty() {
                    list.push(exif.lens_model.clone());
                }
            }

            if let Some(ps_main) = exif.get_ps_main()
                && self.show_ps
            {
                list.push(if let Some(ps_sub) = exif.get_lut_detail() {
                    if !ps_sub.is_empty() {
                        format!("{ps_main} = {ps_sub}")
                    } else {
                        ps_main.to_string()
                    }
                } else {
                    ps_main.to_string()
                })
            }

            list
        };
        for left_str in left_list.iter().rev() {
            draw!(margin, y, &font, cam_scale, left_str);
            y -= cam_scale.y;
        }

        // Right - format: YY  MM  DD
        let date = exif
            .datetime
            .map(|dt| dt.format("%y.%m.%d").to_string())
            .unwrap_or_default();

        let datetime_scale = self.rel_scale(105, dyn_wh);
        let y: f32 = base_y as f32;

        let (datetime_w, _) =
            crate::theme::text_dimensions_with_fallback(datetime_scale, &font_italic, 400, &date);

        let x_right = dyn_w as f32 - margin as f32;
        let x_datetime = (x_right - datetime_w).round() as i32;
        draw!(x_datetime, y, &font_italic, datetime_scale, &date);

        let rgba_image = dyn_image
            .as_mut_rgba8()
            .ok_or(image::ImageError::Parameter(
                image::error::ParameterError::from_kind(image::error::ParameterErrorKind::Generic(
                    "Mismatch RGBA channel internally, it should not happened".to_owned(),
                )),
            ))?;

        crate::effect::glow::final_glow_effect(
            rgba_image,
            &luma_text,
            color,
            glow_color,
            self.rel_size(self.glow_gain as f32, dyn_wh),
        );

        Ok(dyn_image)
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
        let dyn_wh: f32 = (dyn_w as f32).max(dyn_h as f32);

        let margin = self.rel_size(120, dyn_wh).trunc() as i32;

        let mut themed_image = self.apply_to_image(pi, export_config)?;
        export_config.save_image(&mut themed_image, Some(margin), output_path)
    }

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    fn ui_config(&mut self, ui: &mut egui::Ui) {
        self.auto_ui_config(ui);

        // Custom UI for font selection and checkboxes
        ui.vertical(|ui| {
            ui.checkbox(
                &mut self.hide_camera_exif,
                t!("theme.film_config.hide_camera_exif.label"),
            )
            .on_hover_text(t!("theme.film_config.hide_camera_exif.description"));

            ui.add_space(1.0);
            ui.checkbox(
                &mut self.show_ps,
                t!("theme.film_config.show_photo_style.label"),
            )
            .on_hover_text(t!("theme.film_config.show_photo_style.description"));

            self.font.update_ui_with_default_label(ui);

            self.font_italic
                .update_ui_with_label(ui, t!("theme.date_config.date_font_select"));
        });
    }

    fn is_ui_config_available(&self) -> bool {
        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        {
            true
        }
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        {
            false
        }
    }

    fn get_parameters_json(&self) -> String {
        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        {
            self.auto_get_parameters_json()
        }
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        {
            r#"{"parameters": []}"#.to_string()
        }
    }
}

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
pub struct FilmGlow {
    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub font: crate::FontSelection,
    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    pub font_file: String,

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

    pub show_ps: bool,
}

const FILM_COLOR: image::Rgba<u8> = image::Rgba([255, 138, 0, 255]);
const FILM_COLOR_GLOW: image::Rgba<u8> = image::Rgba([238, 140, 128, 255]);
const DEFAULT_FONT_SIZE: u32 = 25;
const DEFAULT_GLOW_GAIN: u32 = 8;
#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
use super::DEFAULT_DIGITAL7_FONT_FILE;

impl core::default::Default for FilmGlow {
    fn default() -> Self {
        let [r, g, b, a] = FILM_COLOR.data();
        let [gr, gg, gb, ga] = FILM_COLOR_GLOW.data();
        Self {
            #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
            font: crate::FONTS_UNIFY.builtin_select(crate::BuiltinFontIndex::Digital7),
            #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
            font_file: DEFAULT_DIGITAL7_FONT_FILE.to_string(),
            font_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
            glow_color: egui::Color32::from_rgba_unmultiplied_const(gr, gg, gb, ga),
            font_size: DEFAULT_FONT_SIZE,
            glow_gain: DEFAULT_GLOW_GAIN,
            show_ps: false,
        }
    }
}

#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
impl crate::theme::parameter_schema::ThemeParameters for FilmGlow {
    fn schema(&self) -> crate::theme::parameter_schema::ThemeSchema {
        use crate::theme::parameter_schema::{ParameterMeta, ParameterType};
        let fc = self.font_color;
        let gc = self.glow_color;
        crate::theme::parameter_schema::ThemeSchema {
            theme_name: "film_glow".to_string(),
            theme_label: "Film Glow".to_string(),
            parameters: vec![
                ParameterMeta {
                    name: "font_color".to_string(),
                    label: "Font Color".to_string(),
                    hint: None,
                    param_type: ParameterType::Color,
                    min: None,
                    max: None,
                    default: serde_json::json!([255u8, 138u8, 0u8, 255u8]),
                    current: serde_json::json!([fc.r(), fc.g(), fc.b(), fc.a()]),
                    exif_fields: None,
                },
                ParameterMeta {
                    name: "glow_color".to_string(),
                    label: "Glow Color".to_string(),
                    hint: None,
                    param_type: ParameterType::Color,
                    min: None,
                    max: None,
                    default: serde_json::json!([238u8, 140u8, 128u8, 255u8]),
                    current: serde_json::json!([gc.r(), gc.g(), gc.b(), gc.a()]),
                    exif_fields: None,
                },
                ParameterMeta {
                    name: "font_size".to_string(),
                    label: "Font Size".to_string(),
                    hint: None,
                    param_type: ParameterType::Slider,
                    min: Some(10.0),
                    max: Some(100.0),
                    default: serde_json::json!(DEFAULT_FONT_SIZE),
                    current: serde_json::json!(self.font_size),
                    exif_fields: None,
                },
                ParameterMeta {
                    name: "glow_gain".to_string(),
                    label: "Glow Range".to_string(),
                    hint: None,
                    param_type: ParameterType::Slider,
                    min: Some(0.0),
                    max: Some(20.0),
                    default: serde_json::json!(DEFAULT_GLOW_GAIN),
                    current: serde_json::json!(self.glow_gain),
                    exif_fields: None,
                },
                ParameterMeta {
                    name: "show_ps".to_string(),
                    label: "Show Photo Style".to_string(),
                    hint: None,
                    param_type: ParameterType::Boolean,
                    min: None,
                    max: None,
                    default: serde_json::json!(false),
                    current: serde_json::json!(self.show_ps),
                    exif_fields: None,
                },
            ],
        }
    }

    fn update_from_json(
        &mut self,
        updates: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), String> {
        if let Some(v) = updates.get("font_color") {
            if let Ok(arr) = serde_json::from_value::<[u8; 4]>(v.clone()) {
                self.font_color =
                    egui::Color32::from_rgba_unmultiplied(arr[0], arr[1], arr[2], arr[3]);
            }
        }
        if let Some(v) = updates.get("glow_color") {
            if let Ok(arr) = serde_json::from_value::<[u8; 4]>(v.clone()) {
                self.glow_color =
                    egui::Color32::from_rgba_unmultiplied(arr[0], arr[1], arr[2], arr[3]);
            }
        }
        if let Some(v) = updates.get("font_size") {
            if let Some(n) = v.as_f64() {
                self.font_size = (n as u32).clamp(10, 100);
            }
        }
        if let Some(v) = updates.get("glow_gain") {
            if let Some(n) = v.as_f64() {
                self.glow_gain = (n as u32).clamp(0, 20);
            }
        }
        if let Some(v) = updates.get("show_ps") {
            if let Some(b) = v.as_bool() {
                self.show_ps = b;
            }
        }
        Ok(())
    }
}

#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
impl FilmGlow {
    fn get_parameters_json_mobile(&self) -> String {
        use crate::theme::parameter_schema::ThemeParameters;
        let schema = self.schema();
        match serde_json::to_string(&serde_json::json!({
            "theme_name": schema.theme_name,
            "parameters": schema.parameters,
        })) {
            Ok(s) => s,
            Err(_) => r#"{"parameters":[]}"#.to_string(),
        }
    }
}

impl FilmGlow {
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

#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
impl FilmGlow {
    fn load_font(&self) -> Result<ab_glyph::FontArc, image::ImageError> {
        use crate::effect::variable_text::get_fonts_base_directory;
        let base_dir = get_fonts_base_directory();
        let full_path = if base_dir.is_empty() {
            std::path::PathBuf::from(&self.font_file)
        } else {
            std::path::PathBuf::from(&base_dir).join(&self.font_file)
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

impl Theme for FilmGlow {
    fn unique_name(&self) -> &'static str {
        "film_glow"
    }

    fn label(&self) -> std::borrow::Cow<'static, str> {
        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        {
            t!("theme.film_glow")
        }
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        {
            std::borrow::Cow::Borrowed("Film Glow")
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
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        let font = self.load_font()?;

        #[rustfmt::skip]
        macro_rules! draw {
            ($xxx:expr, $yyy:expr, $scale:expr, $text:expr) => {
                crate::theme::draw_text_with_fallback_luma(&mut luma_text, image::Luma([255]), ($xxx) as i32, ($yyy as f32 - font.as_scaled($scale).ascent()) as i32, $scale, &font, 400, $text);
            };
        }

        let margin = self.rel_size(120, dyn_wh).trunc() as i32;
        let base_y = dyn_h as i32 - margin;

        // Left
        let mut y = base_y as f32;
        let cam_scale = self.rel_scale(75, dyn_wh);
        let left_list = {
            let mut list: Vec<String> = Vec::new();
            if !(exif.camera_mnf.is_empty() || exif.camera_model.is_empty()) {
                list.push(format!("{}  {}", exif.camera_mnf, exif.camera_model));
            }
            if !exif.lens_model.is_empty() {
                list.push(exif.lens_model.clone());
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
            draw!(margin, y, cam_scale, left_str);
            y -= cam_scale.y;
        }

        // Right
        let pairs = {
            let mut list: Vec<(&'static str, String)> = Vec::new();
            if let Some(f) = exif.get_fnumber() {
                list.push(("F", f));
            }
            if let Some(sec) = exif.get_exposure() {
                list.push(("SEC", sec));
            }
            if let Some(iso) = exif.get_iso() {
                list.push(("ISO", iso));
            }
            list
        };

        let prefix_scale = self.rel_scale(65, dyn_wh);
        let number_scale = self.rel_scale(100, dyn_wh);
        let spacing = self.rel_size(8.0, dyn_wh);
        let mut y: f32 = base_y as f32;

        for (prefix, number) in pairs.iter().rev() {
            let (prefix_w, prefix_h) =
                crate::theme::text_dimensions_with_fallback(prefix_scale, &font, 400, prefix);
            let (number_w, number_h) =
                crate::theme::text_dimensions_with_fallback(number_scale, &font, 400, number);
            let line_h = number_h.max(prefix_h);
            let total_w = prefix_w + spacing + number_w;

            // For right alignment
            let x_right = dyn_w as f32 - margin as f32;
            let x_prefix = (x_right - total_w).round() as i32;
            let x_number = (x_right - number_w).round() as i32;

            draw!(x_prefix, y, prefix_scale, prefix);
            draw!(x_number, y, number_scale, number);

            y -= line_h;
        }

        // JPEG images load as RGB8; convert to RGBA8 for the glow compositing step
        if dyn_image.as_rgba8().is_none() {
            dyn_image = image::DynamicImage::ImageRgba8(dyn_image.to_rgba8());
        }

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

        // Custom UI for font selection and show_ps checkbox
        ui.vertical(|ui| {
            self.font.update_ui_with_default_label(ui);

            ui.add_space(1.0);
            ui.checkbox(
                &mut self.show_ps,
                t!("theme.film_config.show_photo_style.label"),
            )
            .on_hover_text(t!("theme.film_config.show_photo_style.description"));
        });
    }

    fn is_ui_config_available(&self) -> bool {
        true
    }

    fn get_parameters_json(&self) -> String {
        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        {
            self.auto_get_parameters_json()
        }
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        {
            self.get_parameters_json_mobile()
        }
    }
}

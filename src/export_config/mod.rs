/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

#[cfg(all(feature = "desktop", not(feature = "ios_integration")))]
pub(crate) mod open_explorer;
pub(crate) mod output_format;
pub(crate) mod output_name;
pub(crate) mod scale_config;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct ExportConfig {
    pub scale_config: scale_config::ScaleConfig,
    pub output_format: output_format::OutputFormat,
    pub output_name: output_name::OutputName,
    pub theme_reg: crate::theme::ThemeRegistry,
    pub watermark: crate::effect::watermark::Watermark,
    pub face_detection: crate::effect::face_detection::FaceDetection,
}

impl core::default::Default for ExportConfig {
    fn default() -> Self {
        #[cfg(not(test))]
        {
            Self {
                scale_config: scale_config::SCALE_NEAR_COMMON_4K,
                output_format: output_format::OutputFormat::default(),
                output_name: output_name::OutputName::default(),
                theme_reg: crate::theme::ThemeRegistry::new(),
                watermark: crate::effect::watermark::Watermark::default(),
                face_detection: crate::effect::face_detection::FaceDetection::default(),
            }
        }
        #[cfg(test)]
        {
            Self::testkit_default()
        }
    }
}

impl ExportConfig {
    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub fn update_ui(&mut self, ui: &mut egui::Ui, show_theme_name_in_english: bool) {
        ui.group(|ui| {
            ui.heading(rust_i18n::t!("export_config.label"));
            ui.separator();
            self.scale_config.update_ui(ui);
            ui.collapsing(rust_i18n::t!("export_config.detail_of_export"), |ui| {
                ui.separator();
                self.output_format.update_ui(ui);
                ui.separator();
                self.output_name.update_ui(ui);
                ui.separator();
                self.watermark.update_ui(ui);
            });
            ui.separator();
            self.theme_reg.update_ui(ui, show_theme_name_in_english);
        });
    }

    pub fn testkit_default() -> Self {
        use std::str::FromStr;
        Self {
            scale_config: scale_config::SCALE_HALF,
            output_format: output_format::OutputFormat::default(),
            output_name: output_name::OutputName {
                prefix: "".to_owned(),
                postfix: "-testcase".to_owned(),
                folder: std::path::PathBuf::from_str("test_image/export").unwrap(),
                remove_after_bulk_save: false,
                ..Default::default()
            },
            theme_reg: crate::theme::ThemeRegistry::new(),
            watermark: crate::effect::watermark::Watermark::default(),
            face_detection: crate::effect::face_detection::FaceDetection::default(),
        }
    }

    pub fn save_image<P: AsRef<std::path::Path>>(
        &self,
        dyn_image: &mut image::DynamicImage,
        margin: Option<i32>,
        path: P,
    ) -> Result<(), image::ImageError> {
        self.save_image_with_faces(dyn_image, margin, path, None)
    }

    pub fn save_image_with_faces<P: AsRef<std::path::Path>>(
        &self,
        dyn_image: &mut image::DynamicImage,
        #[allow(unused_variables)] margin: Option<i32>,
        path: P,
        #[allow(unused_variables)] pre_detected_faces: Option<Vec<(i32, i32, u32, u32)>>,
    ) -> Result<(), image::ImageError> {
        // Apply watermark first (desktop only - iOS watermark is disabled)
        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        if self.watermark.is_enabled {
            self.watermark.apply(dyn_image, margin)?;
        }

        // Apply face detection
        #[cfg(target_os = "ios")]
        {
            // iOS: Handled via FFI bridge in Swift code
        }

        #[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
        {
            // Face effects are applied to dyn_image before this call (in app.rs __save_bulk_each).
            // pre_detected_faces is used only for logging — no re-detection on export.
            let face_count = pre_detected_faces.as_ref().map(|f| f.len()).unwrap_or(0);
            if face_count > 0 {
                log::info!(
                    "[PASS] [Face Detection] {} pre-detected face(s) applied",
                    face_count
                );
            }
        }

        // Save with output format
        self.output_format.save_image(dyn_image, path)
    }

    #[cfg(test)]
    pub fn insert_or_replace_theme<T: crate::theme::Theme + 'static>(&mut self, theme: T) {
        self.theme_reg.insert_or_replace_theme(theme);
    }
}

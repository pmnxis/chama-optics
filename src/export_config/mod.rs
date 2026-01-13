/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use rust_i18n::t;

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
    pub fn update_ui(&mut self, ui: &mut egui::Ui, show_theme_name_in_english: bool) {
        ui.group(|ui| {
            ui.heading(t!("export_config.label"));
            ui.separator();
            self.scale_config.update_ui(ui);
            ui.collapsing(t!("export_config.detail_of_export"), |ui| {
                ui.separator();
                self.output_format.update_ui(ui);
                ui.separator();
                self.output_name.update_ui(ui);
                ui.separator();
                self.watermark.update_ui(ui);
                ui.separator();
                self.face_detection.update_ui(ui);
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
        margin: Option<i32>,
        path: P,
        pre_detected_faces: Option<Vec<(i32, i32, u32, u32)>>,
    ) -> Result<(), image::ImageError> {
        // Apply watermark first
        if self.watermark.is_enabled {
            self.watermark.apply(dyn_image, margin)?;
        }

        // Apply face detection
        #[cfg(target_os = "ios")]
        {
            // iOS: Handled via FFI bridge in Swift code
        }

        #[cfg(target_os = "macos")]
        {
            // macOS: Support multiple detection engines
            if self.face_detection.is_enabled {
                // Use pre-detected faces if available, otherwise detect on themed image
                let faces = if let Some(pre_faces) = pre_detected_faces {
                    log::info!(
                        "[PASS] [Face Detection] Using {} pre-detected face(s) from original image",
                        pre_faces.len()
                    );
                    pre_faces
                } else {
                    log::info!(
                        "🎯 [Face Detection] Engine: {}, Detecting faces on themed image: {:?}",
                        self.face_detection.engine,
                        path.as_ref()
                    );

                    // Get image dimensions for recursive detection
                    let _img_width = dyn_image.width();
                    let _img_height = dyn_image.height();

                    match &self.face_detection.engine {
                        #[cfg(feature = "face_detection_visionkit")]
                        crate::effect::face_detection::FaceDetectionEngine::VisionKit => {
                            self.detect_visionkit(path.as_ref())
                        }

                        #[cfg(feature = "face_detection_insightface")]
                        crate::effect::face_detection::FaceDetectionEngine::InsightFace => {
                            let detector =
                                crate::effect::insightface_detector::InsightFaceDetector::new(
                                    self.face_detection.speed_mode,
                                    self.face_detection.provider,
                                );
                            self.run_detection(&detector, path.as_ref(), _img_width, _img_height)
                        }
                    }
                };

                // Apply faces if any were detected
                if !faces.is_empty() {
                    let face_count = faces.len();
                    self.face_detection.apply(dyn_image, faces)?;
                    log::info!(
                        "[PASS] [Face Detection] Successfully applied face detection using {} - {} face(s) detected and processed",
                        self.face_detection.engine,
                        face_count
                    );
                } else {
                    log::info!(
                        "[INFO][Face Detection] No faces detected using {}",
                        self.face_detection.engine
                    );
                }
            }
        }

        // Save with output format
        self.output_format.save_image(dyn_image, path)
    }

    #[cfg(feature = "face_detection_insightface")]
    pub fn run_detection<D: crate::effect::face_detectors::FaceDetector>(
        &self,
        detector: &D,
        image_path: &std::path::Path,
        _img_width: u32,
        _img_height: u32,
    ) -> Vec<(i32, i32, u32, u32)> {
        if self.face_detection.recursive_detection {
            log::info!(
                "🔄 [Face Detection] Running recursive face detection with {} (min size: {}px)",
                detector.engine_name(),
                self.face_detection.recursive_min_size
            );

            let faces = self.face_detection.detect_faces_recursive(
                detector,
                image_path,
                _img_width,
                _img_height,
            );

            log::info!(
                "Recursive detection complete: {} unique faces found",
                faces.len()
            );

            faces
        } else {
            log::info!(
                "🎯 [Face Detection] Running standard detection with {}",
                detector.engine_name()
            );

            let faces = detector.detect_faces(image_path);

            log::info!("Standard detection complete: {} face(s) found", faces.len());

            faces
        }
    }

    #[cfg(all(target_os = "macos", feature = "face_detection_visionkit"))]
    pub fn detect_visionkit(&self, path: &std::path::Path) -> Vec<(i32, i32, u32, u32)> {
        use std::process::Command;

        let Some(path_str) = path.to_str() else {
            log::warn!("Invalid UTF-8 path");
            return vec![];
        };

        // Create JSON input for Swift detector
        let input_json = serde_json::json!({
            "image_path": path_str
        });

        // Run Swift face detector
        let detector_path = std::path::PathBuf::from("macos/face_detector.swift");

        Command::new("swift")
            .arg(&detector_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = writeln!(stdin, "{}", input_json);
                }
                child.wait_with_output()
            })
            .map(|output| {
                if !output.status.success() {
                    log::warn!(
                        "Face detector failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }

                // Parse output JSON
                String::from_utf8(output.stdout)
                    .ok()
                    .and_then(|json_str| serde_json::from_str::<serde_json::Value>(&json_str).ok())
                    .and_then(|result| {
                        result
                            .get("faces")
                            .and_then(|v| v.as_array())
                            .map(|faces_array| {
                                faces_array
                                    .iter()
                                    .filter_map(|face_obj| {
                                        let (x, y, width, height) = (
                                            face_obj
                                                .get("x")
                                                .and_then(|v| v.as_i64())
                                                .map(|v| v as i32),
                                            face_obj
                                                .get("y")
                                                .and_then(|v| v.as_i64())
                                                .map(|v| v as i32),
                                            face_obj
                                                .get("width")
                                                .and_then(|v| v.as_i64())
                                                .map(|v| v as u32),
                                            face_obj
                                                .get("height")
                                                .and_then(|v| v.as_i64())
                                                .map(|v| v as u32),
                                        );
                                        Some((x?, y?, width?, height?))
                                    })
                                    .collect()
                            })
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_else(|e| {
                log::error!(
                    "[FAIL][Face Detection] Failed to spawn {} detector: {}",
                    self.face_detection.engine,
                    e
                );
                vec![]
            })
    }

    #[cfg(test)]
    pub fn insert_or_replace_theme<T: crate::theme::Theme + 'static>(&mut self, theme: T) {
        self.theme_reg.insert_or_replace_theme(theme);
    }
}

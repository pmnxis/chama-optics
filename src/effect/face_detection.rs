/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use rust_i18n::t;
use strum::Display;

/// Detection speed modes — controls sliding window strategy for large images.
/// Shared by InsightFace (ort), Candle, and VisionKit (macOS pyramid) detectors.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg(any(
    feature = "face_detection_insightface",
    feature = "face_detection_candle",
    feature = "face_detection_visionkit",
    target_os = "macos"
))]
pub enum SpeedMode {
    /// Fastest: No sliding window (whole image resized to 640×640), ~0.5s avg
    Fastest,
    /// Fast: min(w,h) sliding windows only, ~0.6s avg
    Fast,
    /// Normal: 1 depth level from m_max window, ~7s avg
    Normal,
    /// Slow: 2 depth levels from m_max down, ~13s avg
    Slow,
    /// Slowest: 3 depth levels from m_max down, ~28s avg.
    /// For professional ILC cameras (Panasonic/Sony/Canon/Sigma/Fuji/Hasselblad/Nikon/Leica)
    /// the depth extends to m_max+1 levels, reaching down to 640 px.
    Slowest,
}

#[cfg(any(
    feature = "face_detection_insightface",
    feature = "face_detection_candle",
    feature = "face_detection_visionkit",
    target_os = "macos"
))]
impl SpeedMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpeedMode::Fastest => "fastest",
            SpeedMode::Fast => "fast",
            SpeedMode::Normal => "normal",
            SpeedMode::Slow => "slow",
            SpeedMode::Slowest => "slowest",
        }
    }

    /// Integer 0-4 encoding passed to the macOS face_detector.swift subprocess.
    /// Matches iOS `FaceDetectionSpeedMode.intValue` semantics.
    pub fn as_u8(&self) -> u8 {
        match self {
            SpeedMode::Fastest => 0,
            SpeedMode::Fast => 1,
            SpeedMode::Normal => 2,
            SpeedMode::Slow => 3,
            SpeedMode::Slowest => 4,
        }
    }

    pub fn max_depth(&self) -> u32 {
        match self {
            SpeedMode::Fastest => 0,
            SpeedMode::Fast => 1,
            SpeedMode::Normal => 1,
            SpeedMode::Slow => 2,
            SpeedMode::Slowest => 3,
        }
    }
}

/// Returns `true` if the EXIF Make string belongs to a professional ILC camera brand.
///
/// When `Slowest` mode is active for an ILC camera, the sliding-window pyramid is
/// extended by one extra level (m_max + 1), reaching down to the 640 px base window.
pub fn is_ilc_camera_make(make: &str) -> bool {
    let m = make.to_lowercase();
    m.contains("panasonic")
        || m.contains("sony")
        || m.contains("canon")
        || m.contains("sigma")
        || m.contains("fuji")
        || m.contains("hasselblad")
        || m.contains("nikon")
        || m.contains("leica")
}

/// Face effect mode - what effect to apply to detected faces
/// This enum matches the iOS FaceEffectType for consistency
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, Default)]
pub enum FaceEffectMode {
    /// No effect applied to faces
    #[default]
    None,
    /// Mosaic/pixelate effect
    Mosaic,
    /// Stroke/border around face
    Stroke,
    /// Combined mosaic inside + stroke border
    MosaicStroke,
    /// Sticker overlay (handled separately)
    Sticker,
}

impl FaceEffectMode {
    /// Get display name for the effect mode
    pub fn display_name(&self) -> &'static str {
        match self {
            FaceEffectMode::None => "None",
            FaceEffectMode::Mosaic => "Mosaic",
            FaceEffectMode::Stroke => "Stroke",
            FaceEffectMode::MosaicStroke => "Mosaic+Stroke",
            FaceEffectMode::Sticker => "Sticker",
        }
    }

    /// Get all available modes for UI
    pub fn all_modes() -> &'static [FaceEffectMode] {
        &[
            FaceEffectMode::None,
            FaceEffectMode::Mosaic,
            FaceEffectMode::Stroke,
            FaceEffectMode::MosaicStroke,
            FaceEffectMode::Sticker,
        ]
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Display, PartialEq, Eq, Debug)]
pub enum FaceDetectionEngine {
    #[cfg(any(feature = "face_detection_visionkit", target_os = "macos"))]
    VisionKit,
    #[cfg(feature = "face_detection_insightface")]
    InsightFace,
    #[cfg(feature = "face_detection_candle")]
    Candle,
    #[cfg(not(any(
        feature = "face_detection_visionkit",
        target_os = "macos",
        feature = "face_detection_insightface",
        feature = "face_detection_candle"
    )))]
    NoOp,
}

impl FaceDetectionEngine {
    /// Get display name for engine
    pub fn display_name(&self) -> &'static str {
        match self {
            #[cfg(any(feature = "face_detection_visionkit", target_os = "macos"))]
            Self::VisionKit => "VisionKit",
            #[cfg(feature = "face_detection_insightface")]
            Self::InsightFace => "InsightFace",
            #[cfg(feature = "face_detection_candle")]
            Self::Candle => "Candle",
            #[cfg(not(any(
                feature = "face_detection_visionkit",
                target_os = "macos",
                feature = "face_detection_insightface",
                feature = "face_detection_candle"
            )))]
            Self::NoOp => "NoOp",
        }
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct FaceDetectionConfig {
    pub engine: FaceDetectionEngine,
    #[cfg(any(
        feature = "face_detection_insightface",
        feature = "face_detection_candle",
        feature = "face_detection_visionkit",
        target_os = "macos"
    ))]
    pub speed_mode: SpeedMode,
    #[cfg(feature = "face_detection_insightface")]
    pub provider: crate::effect::insightface_detector::ExecutionProvider,
}

impl core::default::Default for FaceDetectionConfig {
    fn default() -> Self {
        // ── macOS: VisionKit always available ──
        #[cfg(all(target_os = "macos", feature = "face_detection_insightface"))]
        {
            FaceDetectionConfig {
                engine: FaceDetectionEngine::VisionKit,
                speed_mode: SpeedMode::Normal,
                provider:
                    crate::effect::insightface_detector::ExecutionProvider::CPUExecutionProvider,
            }
        }

        #[cfg(all(target_os = "macos", not(feature = "face_detection_insightface")))]
        {
            FaceDetectionConfig {
                engine: FaceDetectionEngine::VisionKit,
                speed_mode: SpeedMode::Normal,
            }
        }

        // ── Non-Apple desktop (Linux / Windows): feature-gated defaults ──
        #[cfg(all(
            not(any(target_os = "ios", target_os = "android", target_os = "macos")),
            feature = "face_detection_visionkit",
            feature = "face_detection_insightface"
        ))]
        {
            FaceDetectionConfig {
                engine: FaceDetectionEngine::VisionKit,
                speed_mode: SpeedMode::Normal,
                provider:
                    crate::effect::insightface_detector::ExecutionProvider::CPUExecutionProvider,
            }
        }

        #[cfg(all(
            not(any(target_os = "ios", target_os = "android", target_os = "macos")),
            feature = "face_detection_visionkit",
            not(feature = "face_detection_insightface")
        ))]
        {
            FaceDetectionConfig {
                engine: FaceDetectionEngine::VisionKit,
                speed_mode: SpeedMode::Normal,
            }
        }

        #[cfg(all(
            not(any(target_os = "ios", target_os = "android", target_os = "macos")),
            feature = "face_detection_insightface",
            not(feature = "face_detection_visionkit")
        ))]
        {
            FaceDetectionConfig {
                engine: FaceDetectionEngine::InsightFace,
                speed_mode: SpeedMode::Normal,
                provider: crate::effect::insightface_detector::ExecutionProvider::OnnxAuto,
            }
        }

        #[cfg(all(
            not(any(target_os = "ios", target_os = "android", target_os = "macos")),
            feature = "face_detection_candle",
            not(feature = "face_detection_visionkit"),
            not(feature = "face_detection_insightface")
        ))]
        {
            FaceDetectionConfig {
                engine: FaceDetectionEngine::Candle,
                speed_mode: SpeedMode::Normal,
            }
        }

        #[cfg(all(
            not(any(
                feature = "face_detection_visionkit",
                feature = "face_detection_insightface",
                feature = "face_detection_candle"
            )),
            not(any(target_os = "ios", target_os = "android", target_os = "macos"))
        ))]
        {
            FaceDetectionConfig {
                engine: FaceDetectionEngine::NoOp,
            }
        }

        // ── iOS ──
        #[cfg(all(target_os = "ios", feature = "face_detection_visionkit"))]
        {
            Self {
                engine: FaceDetectionEngine::VisionKit,
                speed_mode: SpeedMode::Normal,
            }
        }

        #[cfg(all(
            any(target_os = "ios", target_os = "android"),
            not(feature = "face_detection_visionkit"),
            not(feature = "face_detection_candle")
        ))]
        {
            Self {
                engine: FaceDetectionEngine::NoOp,
            }
        }
    }
}

impl FaceDetectionConfig {
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.collapsing(t!("face_detection.detail_of_detection_engine"), |ui| {
                // Engine selection
                ui.label(t!("face_detection.engine"));
                egui::ComboBox::from_label(t!("face_detection.engine"))
                    .selected_text(format!("{}", self.engine))
                    .show_ui(ui, |ui| {
                        // Show VisionKit on macOS (always) or when feature is enabled
                        #[cfg(any(feature = "face_detection_visionkit", target_os = "macos"))]
                        {
                            ui.selectable_value(
                                &mut self.engine,
                                FaceDetectionEngine::VisionKit,
                                "VisionKit",
                            );
                        }
                        // Show InsightFace (enabled if feature is active, otherwise disabled)
                        #[cfg(feature = "face_detection_insightface")]
                        {
                            ui.selectable_value(
                                &mut self.engine,
                                FaceDetectionEngine::InsightFace,
                                "InsightFace",
                            );
                        }
                        #[cfg(not(feature = "face_detection_insightface"))]
                        {
                            ui.colored_label(
                                ui.visuals().weak_text_color(),
                                "InsightFace (requires feature flag)",
                            );
                        }
                        // Show Candle (pure Rust ONNX, works on WASM + desktop)
                        #[cfg(feature = "face_detection_candle")]
                        {
                            ui.selectable_value(
                                &mut self.engine,
                                FaceDetectionEngine::Candle,
                                "Candle",
                            );
                        }
                    });

                ui.separator();

                // Show speed mode options for VisionKit, InsightFace, and Candle
                #[cfg(any(
                    feature = "face_detection_insightface",
                    feature = "face_detection_candle",
                    feature = "face_detection_visionkit",
                    target_os = "macos"
                ))]
                {
                    let mut show_speed_mode = false;
                    #[cfg(any(feature = "face_detection_visionkit", target_os = "macos"))]
                    {
                        show_speed_mode |= matches!(self.engine, FaceDetectionEngine::VisionKit);
                    }
                    #[cfg(feature = "face_detection_insightface")]
                    {
                        show_speed_mode |= matches!(self.engine, FaceDetectionEngine::InsightFace);
                    }
                    #[cfg(feature = "face_detection_candle")]
                    {
                        show_speed_mode |= matches!(self.engine, FaceDetectionEngine::Candle);
                    }
                    if show_speed_mode {
                        // Speed mode selection
                        ui.label(t!("face_detection.speed_mode"))
                            .on_hover_text(t!("face_detection.speed_mode_hint"));

                        egui::ComboBox::from_label(t!("face_detection.speed_mode"))
                            .selected_text(self.speed_mode.as_str().to_string())
                            .show_ui(ui, |ui| {
                                let modes = [
                                    (
                                        SpeedMode::Fastest,
                                        t!("face_detection.speed_mode_fastest"),
                                        "Single or two person photo",
                                    ),
                                    (
                                        SpeedMode::Fast,
                                        t!("face_detection.speed_mode_fast"),
                                        "Single or two person photo with an unusual aspect ratio",
                                    ),
                                    (
                                        SpeedMode::Normal,
                                        t!("face_detection.speed_mode_normal"),
                                        "Group photo of around 10 people",
                                    ),
                                    (
                                        SpeedMode::Slow,
                                        t!("face_detection.speed_mode_slow"),
                                        "Group photo of 40~50 people",
                                    ),
                                    (
                                        SpeedMode::Slowest,
                                        t!("face_detection.speed_mode_slowest"),
                                        "Large group photo of more than 50 people (ILC cameras: extends to 640px)",
                                    ),
                                ];

                                for (mode, label, hint) in modes {
                                    ui.selectable_value(&mut self.speed_mode, mode, label)
                                        .on_hover_text(hint);
                                }
                            });

                        ui.separator();
                    }
                } // #[cfg(any(face_detection_insightface, face_detection_candle))]

                // Execution provider selection (InsightFace only)
                #[cfg(feature = "face_detection_insightface")]
                if matches!(self.engine, FaceDetectionEngine::InsightFace) {
                    ui.label(t!("face_detection.execution_provider"))
                        .on_hover_text(t!("face_detection.execution_provider_hint"));

                    egui::ComboBox::from_label(t!("face_detection.execution_provider"))
                        .selected_text(self.provider.as_str().to_string())
                        .show_ui(ui, |ui| {
                            use crate::effect::insightface_detector::ExecutionProvider;

                            ui.selectable_value(
                                &mut self.provider,
                                ExecutionProvider::CPUExecutionProvider,
                                t!("face_detection.provider_cpu"),
                            )
                            .on_hover_text(t!("face_detection.provider_cpu_hint"));

                            ui.selectable_value(
                                &mut self.provider,
                                ExecutionProvider::OnnxAuto,
                                t!("face_detection.provider_onnx_auto"),
                            )
                            .on_hover_text(t!("face_detection.provider_onnx_auto_hint"));

                            #[cfg(target_os = "macos")]
                            ui.selectable_value(
                                &mut self.provider,
                                ExecutionProvider::CoreMLExecutionProvider,
                                t!("face_detection.provider_coreml"),
                            )
                            .on_hover_text(t!("face_detection.provider_coreml_hint"));
                        });

                    ui.separator();
                }
            });
        });
    }

    pub fn get_current_engine_name(&self) -> String {
        #[cfg(feature = "face_detection_insightface")]
        {
            if matches!(self.engine, FaceDetectionEngine::InsightFace) {
                return format!(
                    "{} {} {}",
                    self.engine.display_name(),
                    self.provider.as_str(),
                    self.speed_mode.as_str()
                );
            }
        }
        #[cfg(any(
            feature = "face_detection_insightface",
            feature = "face_detection_candle"
        ))]
        {
            format!(
                "{} {}",
                self.engine.display_name(),
                self.speed_mode.as_str()
            )
        }
        #[cfg(not(any(
            feature = "face_detection_insightface",
            feature = "face_detection_candle"
        )))]
        {
            self.engine.display_name().to_string()
        }
    }
}

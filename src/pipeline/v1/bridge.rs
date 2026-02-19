/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Bridge utilities for converting legacy FFI types to Pipeline V1 types.
//!
//! This module provides conversion functions from the old C-struct based
//! `CombinedExportConfig` / `CFaceEffectConfig` / `CScaleConfig` world
//! into the new serde-enabled `PipelineConfig` / `PipelineStage` world.
//!
//! The goal is gradual migration: native callers can continue to use the
//! existing C-struct FFI while Rust-side execution routes through the pipeline.

use crate::effect::face_detection::FaceEffectMode;
use crate::effect::sticker_storage::FaceArea;
use crate::export_config::output_format::{OutputExtension, OutputFormat};
use crate::export_config::scale_config::{ScaleConfig, ScaleMode};
use crate::image::exif_impl::SimplifiedExif;

use super::config::PipelineConfig;
use super::stages::*;

// ─── EXIF helpers ───

/// Extract `SimplifiedExif` from an image file path.
///
/// Reads raw EXIF, converts to `SimplifiedExif`, then optionally applies
/// alt-fnumber and 35mm focal length adjustments.
pub fn extract_simplified_exif(
    image_path: &str,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
) -> SimplifiedExif {
    let exif = {
        let file = match std::fs::File::open(image_path) {
            Ok(f) => f,
            Err(_) => return SimplifiedExif::default(),
        };
        let mut buf_reader = std::io::BufReader::new(file);
        exif::Reader::new()
            .read_from_container(&mut buf_reader)
            .ok()
    };

    let original_exif = crate::image::exif_impl::OriginalExif::new(exif);
    let mut view_exif = SimplifiedExif::from(&original_exif);

    if get_alt_fnumber {
        view_exif.replace_with_fnumber_alt_when_invalid();
    }
    if use_35mm_focal_length {
        view_exif.use_35mm_focal_length(&original_exif);
    }

    view_exif
}

/// Apply EXIF override JSON (user edits) onto a `SimplifiedExif`.
///
/// Non-empty fields in the override replace the corresponding fields.
pub fn apply_exif_overrides(exif: &mut SimplifiedExif, override_json: &str) {
    if override_json.is_empty() || override_json == "{}" {
        return;
    }
    match serde_json::from_str::<SimplifiedExif>(override_json) {
        Ok(ov) => {
            if !ov.camera_mnf.is_empty() {
                exif.camera_mnf = ov.camera_mnf;
            }
            if !ov.camera_model.is_empty() {
                exif.camera_model = ov.camera_model;
            }
            if !ov.lens_mnf.is_empty() {
                exif.lens_mnf = ov.lens_mnf;
            }
            if !ov.lens_model.is_empty() {
                exif.lens_model = ov.lens_model;
            }
            if !ov.focal.is_empty() {
                exif.focal = ov.focal;
            }
            if !ov.fnumber.is_empty() {
                exif.fnumber = ov.fnumber;
            }
            if !ov.exposure.is_empty() {
                exif.exposure = ov.exposure;
            }
            if ov.iso_speed.is_some() {
                exif.iso_speed = ov.iso_speed;
            }
            if ov.datetime.is_some() {
                exif.datetime = ov.datetime;
            }
        }
        Err(e) => {
            log::warn!("Failed to parse EXIF override JSON: {}", e);
        }
    }
}

// ─── Face effect conversion ───

/// Describes which face effect type to apply uniformly.
///
/// This mirrors `CFaceEffectType` but is a Rust-native enum
/// that doesn't require FFI types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeFaceEffectType {
    None,
    Mosaic,
    Stroke,
    Sticker,
    MosaicStroke,
}

/// Parameters for building a FaceEffect pipeline stage from legacy FFI data.
///
/// Note: Sticker effects through the bridge only support Mosaic/Stroke/MosaicStroke.
/// Sticker overlays require `FaceArea.sticker_id` (UUID) mapped to a pre-populated
/// `StickerStorage` — the legacy `sticker_path`/`sticker_id` string approach is not
/// bridged. For sticker support, use the JSON-based `chama_pipeline_execute` API
/// where the native caller populates sticker UUIDs directly in `PipelineConfig`.
pub struct BridgeFaceEffectParams {
    pub effect_type: BridgeFaceEffectType,
    pub mosaic_block_size: u32,
    pub mosaic_intensity: f32,
    pub stroke_color: [u8; 4],
    pub stroke_thickness: u32,
    pub sticker_scale: f32,
    pub sticker_offset_x: i32,
    pub sticker_offset_y: i32,
}

/// Build a `PipelineStage::FaceEffect` from legacy C-struct face parameters.
///
/// Converts `CFaceRect` tuples + `CFaceEffectConfig` equivalent into a single
/// `PipelineStage::FaceEffect` with per-face `FaceEffectMode` set uniformly.
///
/// Returns `None` if effect type is None or there are no faces.
pub fn build_face_effect_stage(
    face_rects: &[(i32, i32, u32, u32)],
    params: &BridgeFaceEffectParams,
) -> Option<PipelineStage> {
    if face_rects.is_empty() || params.effect_type == BridgeFaceEffectType::None {
        return None;
    }

    let mode = match params.effect_type {
        BridgeFaceEffectType::None => return None,
        BridgeFaceEffectType::Mosaic => FaceEffectMode::Mosaic,
        BridgeFaceEffectType::Stroke => FaceEffectMode::Stroke,
        BridgeFaceEffectType::MosaicStroke => FaceEffectMode::MosaicStroke,
        BridgeFaceEffectType::Sticker => FaceEffectMode::Sticker,
    };

    let faces: Vec<FaceArea> = face_rects
        .iter()
        .map(|&(x, y, w, h)| {
            let mut face = FaceArea::new(x, y, w, h);
            face.effect_mode = mode;
            face
        })
        .collect();

    Some(PipelineStage::FaceEffect {
        faces,
        mosaic: MosaicEffectConfig {
            block_size: params.mosaic_block_size,
            intensity: params.mosaic_intensity,
        },
        stroke: StrokeEffectConfig {
            thickness: params.stroke_thickness,
            color: params.stroke_color,
        },
        sticker: StickerEffectConfig {
            scale: params.sticker_scale,
            offset_x: params.sticker_offset_x,
            offset_y: params.sticker_offset_y,
        },
    })
}

// ─── Scale config conversion ───

/// Convert the FFI-level CScaleMode/CScaleConfig values into
/// pipeline-level `ScaleConfig`.
///
/// This duplicates the mapping in `ffi_mobile/mod.rs::convert_c_scale_config`
/// but operates on raw mode/value tuples, avoiding `#[repr(C)]` dependency.
pub fn build_scale_config(mode: u8, value: u32, sub_value: u32, scale_value: f64) -> ScaleConfig {
    let scale_mode = match mode {
        1 => ScaleMode::MaxWidth,
        2 => ScaleMode::MaxHeight,
        3 => ScaleMode::Longside,
        4 => ScaleMode::Divide,
        5 => ScaleMode::NearCommonDivisorConsiderWidth,
        6 => ScaleMode::NearCommonDivisorConsiderHeight,
        7 => ScaleMode::ResizeAndCrop,
        _ => ScaleMode::None,
    };

    ScaleConfig {
        mode: scale_mode,
        value,
        sub_value,
        scale_value: scale_value as f32,
    }
}

// ─── Image scaling ───

/// Apply `ScaleConfig` to a `DynamicImage`, returning the scaled result.
///
/// Uses `fast_image_resize` for high-quality Lanczos3 downscaling.
/// Returns the original image unchanged if mode is `None` or dimensions match.
pub fn apply_scale(
    image: image::DynamicImage,
    scale: &ScaleConfig,
) -> Result<image::DynamicImage, image::ImageError> {
    if scale.mode == ScaleMode::None {
        return Ok(image);
    }

    let (src_w, src_h) = (image.width(), image.height());
    let (new_w, new_h) = scale.apply(src_w, src_h, false);

    if new_w == src_w && new_h == src_h {
        return Ok(image);
    }

    if new_w == 0 || new_h == 0 {
        return Ok(image);
    }

    log::info!(
        "Pipeline scale: {}x{} → {}x{} (mode={:?})",
        src_w,
        src_h,
        new_w,
        new_h,
        scale.mode
    );

    let resized = crate::image::common::resize_image(image, new_w, new_h)?;
    let buffer =
        image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(new_w, new_h, resized.into_vec())
            .ok_or_else(|| {
                image::ImageError::Encoding(image::error::EncodingError::new(
                    image::error::ImageFormatHint::Unknown,
                    "Failed to create ImageBuffer from resized data",
                ))
            })?;

    Ok(image::DynamicImage::ImageRgba8(buffer))
}

// ─── Output format conversion ───

/// Convert FFI output format enum value + quality into `OutputFormat`.
pub fn build_output_format(format: u32, quality: u8) -> OutputFormat {
    let ext = match format {
        1 => OutputExtension::PngOptimized,
        2 => OutputExtension::Webp,
        _ => OutputExtension::Jpeg,
    };
    OutputFormat { ext, quality }
}

// ─── Full PipelineConfig builder ───

/// Parameters for building a complete `PipelineConfig` from legacy export data.
///
/// This is the Rust-native equivalent of `CombinedExportConfig` — no raw pointers,
/// no `*const c_char`. Designed to be populated from parsed FFI strings.
pub struct BridgeExportParams<'a> {
    /// Face rectangles (x, y, w, h)
    pub face_rects: &'a [(i32, i32, u32, u32)],
    /// Face effect configuration
    pub face_effect: BridgeFaceEffectParams,
    /// Theme name (empty string = no theme)
    pub theme_name: &'a str,
    /// Theme parameters JSON
    pub theme_params_json: &'a str,
    /// Scale mode (0=None, 1=MaxWidth, ...)
    pub scale_mode: u8,
    /// Scale primary value
    pub scale_value: u32,
    /// Scale secondary value
    pub scale_sub_value: u32,
    /// Scale divisor
    pub scale_divisor: f64,
    /// Output format (0=JPEG, 1=PNG, 2=WebP)
    pub output_format: u32,
    /// Quality (1-100)
    pub quality: u8,
}

/// Build a complete `PipelineConfig` from legacy combined export parameters.
///
/// The resulting config reproduces the same execution order as `chama_export_combined`:
/// 1. Face effects (if any faces + effect != None)
/// 2. Theme decoration (if theme_name is non-empty)
///
/// Scale and output format are set on PipelineConfig directly.
pub fn build_pipeline_config(params: &BridgeExportParams) -> PipelineConfig {
    let mut stages = Vec::new();

    // Stage: Face effects
    if let Some(face_stage) = build_face_effect_stage(params.face_rects, &params.face_effect) {
        stages.push(StageEntry::enabled(face_stage));
    }

    // Decoration: Theme
    let decoration = if !params.theme_name.is_empty() {
        let theme_params: Option<serde_json::Value> =
            if params.theme_params_json.is_empty() || params.theme_params_json == "{}" {
                None
            } else {
                serde_json::from_str(params.theme_params_json).ok()
            };

        Some(DecorationEntry::enabled(Decoration::Theme(ThemeConfig {
            name: params.theme_name.to_string(),
            params: theme_params,
        })))
    } else {
        None
    };

    PipelineConfig {
        stages,
        decoration,
        scale: build_scale_config(
            params.scale_mode,
            params.scale_value,
            params.scale_sub_value,
            params.scale_divisor,
        ),
        output_format: build_output_format(params.output_format, params.quality),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_face_effect_stage_none_type_returns_none() {
        let rects = vec![(10, 20, 100, 100)];
        let params = BridgeFaceEffectParams {
            effect_type: BridgeFaceEffectType::None,
            mosaic_block_size: 10,
            mosaic_intensity: 1.0,
            stroke_color: [255, 0, 0, 255],
            stroke_thickness: 4,
            sticker_scale: 1.0,
            sticker_offset_x: 0,
            sticker_offset_y: 0,
        };
        assert!(build_face_effect_stage(&rects, &params).is_none());
    }

    #[test]
    fn test_build_face_effect_stage_empty_faces_returns_none() {
        let rects: Vec<(i32, i32, u32, u32)> = vec![];
        let params = BridgeFaceEffectParams {
            effect_type: BridgeFaceEffectType::Mosaic,
            mosaic_block_size: 10,
            mosaic_intensity: 1.0,
            stroke_color: [255, 0, 0, 255],
            stroke_thickness: 4,
            sticker_scale: 1.0,
            sticker_offset_x: 0,
            sticker_offset_y: 0,
        };
        assert!(build_face_effect_stage(&rects, &params).is_none());
    }

    #[test]
    fn test_build_face_effect_stage_mosaic() {
        let rects = vec![(10, 20, 100, 100), (200, 300, 50, 50)];
        let params = BridgeFaceEffectParams {
            effect_type: BridgeFaceEffectType::Mosaic,
            mosaic_block_size: 15,
            mosaic_intensity: 0.8,
            stroke_color: [0; 4],
            stroke_thickness: 0,
            sticker_scale: 1.0,
            sticker_offset_x: 0,
            sticker_offset_y: 0,
        };
        let stage = build_face_effect_stage(&rects, &params).unwrap();
        assert_eq!(stage.kind(), StageKind::FaceEffect);

        if let PipelineStage::FaceEffect { faces, mosaic, .. } = &stage {
            assert_eq!(faces.len(), 2);
            assert_eq!(faces[0].effect_mode, FaceEffectMode::Mosaic);
            assert_eq!(mosaic.block_size, 15);
        } else {
            panic!("Expected FaceEffect stage");
        }
    }

    #[test]
    fn test_build_scale_config_modes() {
        let none = build_scale_config(0, 0, 0, 0.0);
        assert_eq!(none.mode, ScaleMode::None);

        let max_w = build_scale_config(1, 4000, 0, 0.0);
        assert_eq!(max_w.mode, ScaleMode::MaxWidth);
        assert_eq!(max_w.value, 4000);

        let divide = build_scale_config(4, 0, 0, 2.0);
        assert_eq!(divide.mode, ScaleMode::Divide);
        assert_eq!(divide.scale_value, 2.0);

        let crop = build_scale_config(7, 1920, 1080, 0.0);
        assert_eq!(crop.mode, ScaleMode::ResizeAndCrop);
        assert_eq!(crop.value, 1920);
        assert_eq!(crop.sub_value, 1080);
    }

    #[test]
    fn test_build_output_format() {
        let jpeg = build_output_format(0, 95);
        assert_eq!(jpeg.ext, OutputExtension::Jpeg);
        assert_eq!(jpeg.quality, 95);

        let png = build_output_format(1, 0);
        assert_eq!(png.ext, OutputExtension::PngOptimized);

        let webp = build_output_format(2, 80);
        assert_eq!(webp.ext, OutputExtension::Webp);
        assert_eq!(webp.quality, 80);
    }

    #[test]
    fn test_build_pipeline_config_no_effects_no_theme() {
        let params = BridgeExportParams {
            face_rects: &[],
            face_effect: BridgeFaceEffectParams {
                effect_type: BridgeFaceEffectType::None,
                mosaic_block_size: 10,
                mosaic_intensity: 1.0,
                stroke_color: [0; 4],
                stroke_thickness: 0,
                sticker_scale: 1.0,
                sticker_offset_x: 0,
                sticker_offset_y: 0,
            },
            theme_name: "",
            theme_params_json: "{}",
            scale_mode: 0,
            scale_value: 0,
            scale_sub_value: 0,
            scale_divisor: 0.0,
            output_format: 0,
            quality: 95,
        };

        let config = build_pipeline_config(&params);
        assert!(config.stages.is_empty());
        assert!(config.decoration.is_none());
        assert_eq!(config.scale.mode, ScaleMode::None);
        assert_eq!(config.output_format.ext, OutputExtension::Jpeg);
    }

    #[test]
    fn test_build_pipeline_config_with_faces_and_theme() {
        let faces = vec![(10, 20, 100, 100)];
        let params = BridgeExportParams {
            face_rects: &faces,
            face_effect: BridgeFaceEffectParams {
                effect_type: BridgeFaceEffectType::MosaicStroke,
                mosaic_block_size: 12,
                mosaic_intensity: 0.9,
                stroke_color: [255, 0, 0, 255],
                stroke_thickness: 3,
                sticker_scale: 1.0,
                sticker_offset_x: 0,
                sticker_offset_y: 0,
            },
            theme_name: "film",
            theme_params_json: "{\"border.bottom\": 100}",
            scale_mode: 5, // NearCommonWidth
            scale_value: 4072,
            scale_sub_value: 3054,
            scale_divisor: 2.0,
            output_format: 2, // WebP
            quality: 90,
        };

        let config = build_pipeline_config(&params);

        // One stage: FaceEffect
        assert_eq!(config.stages.len(), 1);
        assert!(config.stages[0].enabled);
        assert_eq!(config.stages[0].stage.kind(), StageKind::FaceEffect);

        // Decoration: Theme "film"
        let deco = config.decoration.as_ref().unwrap();
        assert!(deco.enabled);
        if let Decoration::Theme(ref tc) = deco.decoration {
            assert_eq!(tc.name, "film");
            assert!(tc.params.is_some());
        } else {
            panic!("Expected Theme decoration");
        }

        // Scale
        assert_eq!(config.scale.mode, ScaleMode::NearCommonDivisorConsiderWidth);
        assert_eq!(config.scale.value, 4072);

        // Output format
        assert_eq!(config.output_format.ext, OutputExtension::Webp);
        assert_eq!(config.output_format.quality, 90);
    }

    #[test]
    fn test_build_pipeline_config_serde_roundtrip() {
        let faces = vec![(50, 50, 200, 200)];
        let params = BridgeExportParams {
            face_rects: &faces,
            face_effect: BridgeFaceEffectParams {
                effect_type: BridgeFaceEffectType::Stroke,
                mosaic_block_size: 10,
                mosaic_intensity: 1.0,
                stroke_color: [0, 255, 0, 128],
                stroke_thickness: 5,
                sticker_scale: 1.0,
                sticker_offset_x: 0,
                sticker_offset_y: 0,
            },
            theme_name: "monitor",
            theme_params_json: "{}",
            scale_mode: 3, // Longside
            scale_value: 2048,
            scale_sub_value: 0,
            scale_divisor: 0.0,
            output_format: 0, // JPEG
            quality: 85,
        };

        let config = build_pipeline_config(&params);
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: PipelineConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.stages.len(), config.stages.len());
        assert_eq!(
            deserialized.decoration.is_some(),
            config.decoration.is_some()
        );
        assert_eq!(deserialized.scale.mode, config.scale.mode);
        assert_eq!(deserialized.output_format.ext, config.output_format.ext);
    }

    #[test]
    fn test_apply_exif_overrides_partial() {
        let mut exif = SimplifiedExif::default();
        exif.camera_mnf = "Canon".to_string();
        exif.camera_model = "EOS R5".to_string();
        exif.focal = "50".to_string();

        // Only override camera_model
        apply_exif_overrides(&mut exif, r#"{"camera_model": "EOS R6 II"}"#);

        assert_eq!(exif.camera_mnf, "Canon"); // unchanged
        assert_eq!(exif.camera_model, "EOS R6 II"); // overridden
        assert_eq!(exif.focal, "50"); // unchanged
    }

    #[test]
    fn test_apply_exif_overrides_empty_noop() {
        let mut exif = SimplifiedExif::default();
        exif.camera_mnf = "Sony".to_string();

        apply_exif_overrides(&mut exif, "{}");
        assert_eq!(exif.camera_mnf, "Sony");

        apply_exif_overrides(&mut exif, "");
        assert_eq!(exif.camera_mnf, "Sony");
    }
}

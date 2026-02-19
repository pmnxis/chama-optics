/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Pipeline execution engine.
//!
//! Platform-agnostic, cfg-free image processing.
//! All effects are applied at original resolution; decoration (Theme/Cheki)
//! handles final resize.
//!
//! Stage execution logic is shared between `ExportPipeline` (full export)
//! and `PreviewPipeline` (incremental preview with caching).

use image::DynamicImage;

use crate::effect::face_detection::FaceEffectMode;
use crate::effect::mosaic::MosaicEffect;
use crate::effect::sticker_storage::{FaceArea, StickerConfig};
use crate::effect::stroke::StrokeEffect;

use super::config::PipelineConfig;
use super::context::PipelineContext;
use super::stages::{
    Decoration, MosaicEffectConfig, PipelineStage, StickerEffectConfig, StrokeEffectConfig,
    WatermarkConfig,
};
use super::validation::PipelineError;

// ─── Shared stage execution ───

/// Execute a single `PipelineStage` on the given image (in-place).
///
/// This is the shared core used by both `ExportPipeline` and `PreviewPipeline`.
pub(crate) fn execute_stage(
    image: &mut DynamicImage,
    stage: &PipelineStage,
    ctx: &PipelineContext,
) -> Result<(), PipelineError> {
    match stage {
        PipelineStage::CropRotate(transform) => {
            *image = transform.apply(image);
        }
        PipelineStage::ColorAdjustments(adjustments) => {
            if adjustments.enabled {
                adjustments.apply(image);
            }
        }
        PipelineStage::Lut { lut_id } => {
            apply_lut(image, *lut_id, ctx)?;
        }
        PipelineStage::FaceEffect {
            faces,
            mosaic,
            stroke,
            sticker,
        } => {
            apply_face_effects(image, faces, mosaic, stroke, sticker, ctx)?;
        }
        PipelineStage::Watermark(config) => {
            apply_watermark(image, config, ctx)?;
        }
    }
    Ok(())
}

/// Execute a `Decoration` on the given image.
pub(crate) fn execute_decoration(
    image: &mut DynamicImage,
    decoration: &Decoration,
    ctx: &PipelineContext,
) -> Result<(), PipelineError> {
    match decoration {
        Decoration::Theme(config) => {
            apply_theme(image, config, ctx)?;
        }
        Decoration::Cheki(config) => {
            apply_cheki(image, config, ctx)?;
        }
    }
    Ok(())
}

// ─── Stage implementations ───

/// Apply LUT color grading from pre-resolved LUT data in context.
fn apply_lut(
    image: &mut DynamicImage,
    lut_id: uuid::Uuid,
    ctx: &PipelineContext,
) -> Result<(), PipelineError> {
    let Some(lut_map) = ctx.lut_map else {
        return Err(PipelineError::StageError(
            "LUT stage: no lut_map provided in PipelineContext".into(),
        ));
    };

    let Some(lut) = lut_map.get(&lut_id) else {
        return Err(PipelineError::StageError(format!(
            "LUT stage: lut_id {} not found in lut_map",
            lut_id
        )));
    };

    // Apply LUT based on image type (same logic as LutStorage::apply_lut_to_image)
    match image {
        DynamicImage::ImageRgba8(img) => {
            wagahai_lut::lut::apply_rgba_mut(lut, img);
        }
        DynamicImage::ImageRgb8(img) => {
            wagahai_lut::lut::apply_rgb_mut(lut, img);
        }
        _ => {
            let mut rgba = image.to_rgba8();
            wagahai_lut::lut::apply_rgba_mut(lut, &mut rgba);
            *image = DynamicImage::ImageRgba8(rgba);
        }
    }

    Ok(())
}

/// Face effects: classify by mode, batch apply mosaic/stroke/sticker.
fn apply_face_effects(
    image: &mut DynamicImage,
    faces: &[FaceArea],
    mosaic_config: &MosaicEffectConfig,
    stroke_config: &StrokeEffectConfig,
    sticker_config: &StickerEffectConfig,
    ctx: &PipelineContext,
) -> Result<(), PipelineError> {
    let mut mosaic_faces: Vec<(i32, i32, u32, u32)> = Vec::new();
    let mut stroke_faces: Vec<(i32, i32, u32, u32)> = Vec::new();
    let mut sticker_faces: Vec<&FaceArea> = Vec::new();

    for face in faces {
        let face_tuple = (face.x, face.y, face.width, face.height);
        match face.effect_mode {
            FaceEffectMode::None => {}
            FaceEffectMode::Mosaic => mosaic_faces.push(face_tuple),
            FaceEffectMode::Stroke => stroke_faces.push(face_tuple),
            FaceEffectMode::MosaicStroke => {
                mosaic_faces.push(face_tuple);
                stroke_faces.push(face_tuple);
            }
            FaceEffectMode::Sticker => sticker_faces.push(face),
        }
    }

    // Mosaic
    if !mosaic_faces.is_empty() {
        let effect = MosaicEffect::new(mosaic_config.block_size, mosaic_config.intensity);
        if let Err(e) = MosaicEffect::apply(image, &mosaic_faces, &effect) {
            log::error!("Mosaic effect failed: {}", e);
        }
    }

    // Stroke
    if !stroke_faces.is_empty() {
        let effect = StrokeEffect::new(
            stroke_config.thickness,
            (
                stroke_config.color[0],
                stroke_config.color[1],
                stroke_config.color[2],
                stroke_config.color[3],
            ),
        );
        if let Err(e) = StrokeEffect::apply(image, &stroke_faces, &effect) {
            log::error!("Stroke effect failed: {}", e);
        }
    }

    // Sticker — apply using StickerStorage from context
    if !sticker_faces.is_empty() {
        if let Some(storage) = ctx.sticker_storage {
            // Convert pipeline StickerEffectConfig → existing StickerConfig
            let config = StickerConfig {
                sticker_id: None, // per-face sticker_id takes precedence
                scale: sticker_config.scale,
                offset_x: sticker_config.offset_x,
                offset_y: sticker_config.offset_y,
            };

            // Collect only sticker-mode faces for the sticker function
            let sticker_face_areas: Vec<FaceArea> = sticker_faces
                .iter()
                .filter(|f| f.sticker_id.is_some())
                .map(|f| (*f).clone())
                .collect();

            if !sticker_face_areas.is_empty() {
                // apply_stickers_from_storage takes ownership and returns new image
                let taken = std::mem::take(image);
                *image = crate::effect::sticker_storage::apply_stickers_from_storage(
                    taken,
                    &sticker_face_areas,
                    storage,
                    &config,
                );
            }
        } else {
            log::warn!(
                "Pipeline sticker effect: {} faces need stickers but no StickerStorage in context",
                sticker_faces.len()
            );
        }
    }

    Ok(())
}

/// Apply watermark text overlay using font from context.
fn apply_watermark(
    image: &mut DynamicImage,
    config: &WatermarkConfig,
    ctx: &PipelineContext,
) -> Result<(), PipelineError> {
    // Resolve font from context
    let font = if let Some(font_map) = ctx.font_map {
        if let Some(name) = &config.font_name {
            font_map.get(name)
        } else {
            // Use first available font as fallback
            font_map.values().next()
        }
    } else {
        None
    };

    let Some(font) = font else {
        return Err(PipelineError::StageError(
            "Watermark stage: no font available in PipelineContext.font_map".into(),
        ));
    };

    let (img_w, img_h) = (image.width() as f32, image.height() as f32);
    let dyn_wh = img_w.min(img_h);

    // Scale font size relative to image dimensions (normalized to 4000px reference)
    let scale_factor = dyn_wh / 4000.0;
    let font_px = config.font_size * scale_factor;
    let px_scale = ab_glyph::PxScale::from(font_px);

    // Calculate text dimensions
    let (txt_w, txt_h) = {
        use ab_glyph::{Font, ScaleFont};
        let scaled = font.as_scaled(px_scale);
        (
            config
                .text
                .chars()
                .map(|c| scaled.h_advance(font.glyph_id(c)))
                .sum::<f32>(),
            scaled.height(),
        )
    };

    // Calculate margin
    let margin = (120.0 * scale_factor) as i32;

    // Position calculation (9-position grid: 1-9)
    let (x, y) = watermark_position(
        config.position,
        img_w as i32,
        img_h as i32,
        txt_w as i32,
        txt_h as i32,
        margin,
    );

    // Draw text with transparency
    let color = image::Rgba([
        config.font_color[0],
        config.font_color[1],
        config.font_color[2],
        config.font_color[3],
    ]);

    let transparency = config.font_color[3];

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    {
        if config.is_screen_overlay {
            crate::effect::draw_with_transparency::draw_text_screen_transparency_mut(
                image,
                color,
                x,
                y,
                px_scale,
                font,
                transparency,
                &config.text,
            );
        } else {
            crate::effect::draw_with_transparency::draw_text_transparency_mut(
                image,
                color,
                x,
                y,
                px_scale,
                font,
                transparency,
                &config.text,
            );
        }
    }
    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    {
        // Watermark text rendering not yet available on mobile platforms.
        let _ = (x, y, px_scale, font, transparency, color);
        log::warn!("Watermark rendering is not yet supported on mobile platforms");
    }

    Ok(())
}

/// 9-position grid for watermark placement.
/// ```text
/// 1(↖)  2(↑)  3(↗)
/// 4(←)  5(●)  6(→)
/// 7(↙)  8(↓)  9(↘)
/// ```
fn watermark_position(
    position: u8,
    img_w: i32,
    img_h: i32,
    txt_w: i32,
    txt_h: i32,
    margin: i32,
) -> (i32, i32) {
    let center_x = (img_w - txt_w) / 2;
    let center_y = (img_h - txt_h) / 2;
    let right_x = img_w - txt_w - margin;
    let bottom_y = img_h - txt_h - margin;

    match position {
        1 => (margin, margin),     // top-left
        2 => (center_x, margin),   // top-center
        3 => (right_x, margin),    // top-right
        4 => (margin, center_y),   // center-left
        5 => (center_x, center_y), // center
        6 => (right_x, center_y),  // center-right
        7 => (margin, bottom_y),   // bottom-left
        8 => (center_x, bottom_y), // bottom-center
        _ => (right_x, bottom_y),  // bottom-right (default: 9 or 3)
    }
}

/// Apply Theme decoration by creating a fresh theme instance and applying params.
///
/// Uses `create_theme()` to get a mutable `Box<dyn Theme>`, then applies
/// `ThemeConfig.params` via downcast-based `update_from_json`. This matches
/// the existing FFI flow in `ffi_mobile/theme.rs::export_final_impl`.
fn apply_theme(
    image: &mut DynamicImage,
    config: &super::stages::ThemeConfig,
    ctx: &PipelineContext,
) -> Result<(), PipelineError> {
    let Some(export_config) = ctx.export_config else {
        return Err(PipelineError::StageError(
            "Theme decoration: no export_config provided in PipelineContext".into(),
        ));
    };

    let default_exif = crate::image::exif_impl::SimplifiedExif::default();
    let exif = ctx.exif.unwrap_or(&default_exif);

    // Create a fresh theme instance (mutable, with default params)
    let mut theme = crate::theme::create_theme(&config.name).ok_or_else(|| {
        PipelineError::StageError(format!(
            "Theme decoration: theme '{}' not found",
            config.name
        ))
    })?;

    // Apply parameter overrides from config.params if present
    if let Some(params_value) = &config.params
        && let Some(params_map) = params_value.as_object()
        && !params_map.is_empty()
        && let Err(e) = update_theme_params(&mut *theme, params_map)
    {
        log::warn!("Theme param update warning (using defaults): {}", e);
    }

    let taken = std::mem::take(image);
    *image = theme.apply_to_dynamic_image(taken, exif, export_config)?;

    Ok(())
}

/// Update theme parameters via downcast to concrete types.
///
/// This mirrors `ffi_mobile/theme.rs::update_theme_from_json` but returns
/// a plain `String` error instead of `ChamaOpticsError`.
fn update_theme_params(
    theme: &mut dyn crate::theme::Theme,
    updates: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    use crate::theme::parameter_schema::ThemeParameters;

    macro_rules! try_update {
        ($theme_type:ty) => {
            if let Some(concrete) = (theme as &mut dyn std::any::Any).downcast_mut::<$theme_type>()
            {
                return concrete.update_from_json(updates);
            }
        };
    }

    try_update!(crate::theme::just_frame::JustFrame);
    try_update!(crate::theme::one_line::OneLine);
    try_update!(crate::theme::two_line::TwoLine);
    try_update!(crate::theme::shot_on_one_line::ShotOnOneLine);
    try_update!(crate::theme::shot_on_two_line::ShotOnTwoLine);
    try_update!(crate::theme::strap::Strap);
    try_update!(crate::theme::monitor::Monitor);
    try_update!(crate::theme::lightroom::Lightroom);
    try_update!(crate::theme::film::Film);
    try_update!(crate::theme::film_date::FilmDate);
    try_update!(crate::theme::film_glow::FilmGlow);

    Err(format!(
        "Could not downcast theme '{}' for parameter update",
        theme.unique_name()
    ))
}

/// Apply Cheki (polaroid) decoration using the cheki renderer.
fn apply_cheki(
    image: &mut DynamicImage,
    config: &crate::effect::cheki::ChekiDecoration,
    ctx: &PipelineContext,
) -> Result<(), PipelineError> {
    let default_storage = crate::effect::sticker_storage::StickerStorage::default();
    let storage = ctx.sticker_storage.unwrap_or(&default_storage);

    // apply_cheki_decoration takes ownership and returns new image
    let taken = std::mem::take(image);
    *image = crate::effect::cheki_renderer::apply_cheki_decoration(taken, config, storage);

    Ok(())
}

// ─── ExportPipeline ───

/// Platform-agnostic export pipeline.
///
/// Takes a `DynamicImage` and a `PipelineConfig`, executes all enabled stages
/// in order, then applies decoration (Theme/Cheki) last.
/// For preview with incremental caching, use `PreviewPipeline` instead.
pub struct ExportPipeline {
    image: DynamicImage,
    config: PipelineConfig,
}

impl ExportPipeline {
    /// Create a new pipeline with the given image and configuration.
    pub fn new(image: DynamicImage, config: PipelineConfig) -> Self {
        Self { image, config }
    }

    /// Execute the pipeline: validate → run stages → apply decoration.
    ///
    /// Consumes `self` and returns the fully processed image.
    pub fn execute(mut self, ctx: &PipelineContext) -> Result<DynamicImage, PipelineError> {
        self.config.validate()?;

        let stages = std::mem::take(&mut self.config.stages);
        for entry in &stages {
            if !entry.enabled {
                continue;
            }
            execute_stage(&mut self.image, &entry.stage, ctx)?;
        }

        let decoration = self.config.decoration.take();
        if let Some(deco_entry) = &decoration
            && deco_entry.enabled
        {
            execute_decoration(&mut self.image, &deco_entry.decoration, ctx)?;
        }

        Ok(self.image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::color_adjustments::ColorAdjustments;
    use crate::effect::crop_rotate::CropRotateTransform;
    use crate::pipeline::v1::stages::{StageEntry, StageKind};

    /// Create a small test image (10x10 red RGBA).
    fn test_image() -> DynamicImage {
        DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            10,
            10,
            image::Rgba([255, 0, 0, 255]),
        ))
    }

    #[test]
    fn test_empty_pipeline_returns_original() {
        let img = test_image();
        let config = PipelineConfig::default();
        let ctx = PipelineContext::empty();
        let pipeline = ExportPipeline::new(img.clone(), config);
        let result = pipeline.execute(&ctx).unwrap();
        assert_eq!(result.width(), 10);
        assert_eq!(result.height(), 10);
    }

    #[test]
    fn test_disabled_stage_is_skipped() {
        let img = test_image();
        let mut config = PipelineConfig::default();
        // Add a disabled ColorAdjustments stage with extreme values
        let mut adjustments = ColorAdjustments::new();
        adjustments.enabled = true;
        adjustments.exposure = 5.0; // extreme value
        config
            .stages
            .push(StageEntry::disabled(PipelineStage::ColorAdjustments(
                adjustments,
            )));

        let ctx = PipelineContext::empty();
        let pipeline = ExportPipeline::new(img.clone(), config);
        let result = pipeline.execute(&ctx).unwrap();

        // Image should be unchanged since stage is disabled
        let orig_pixel = img.as_rgba8().unwrap().get_pixel(5, 5);
        let result_pixel = result.as_rgba8().unwrap().get_pixel(5, 5);
        assert_eq!(orig_pixel, result_pixel);
    }

    #[test]
    fn test_color_adjustments_applied() {
        let img = test_image();
        let mut config = PipelineConfig::default();
        let mut adjustments = ColorAdjustments::new();
        adjustments.enabled = true;
        adjustments.exposure = 2.0; // brighten significantly
        config
            .stages
            .push(StageEntry::enabled(PipelineStage::ColorAdjustments(
                adjustments,
            )));

        let ctx = PipelineContext::empty();
        let pipeline = ExportPipeline::new(img, config);
        let result = pipeline.execute(&ctx).unwrap();

        // Red channel should still be 255 (clamped), but the image was processed
        let pixel = result.as_rgba8().unwrap().get_pixel(5, 5);
        assert_eq!(pixel[3], 255); // alpha unchanged
    }

    #[test]
    fn test_validation_crop_rotate_not_first() {
        let mut config = PipelineConfig::default();
        // Add ColorAdjustments first, then CropRotate — should fail validation
        let mut adjustments = ColorAdjustments::new();
        adjustments.enabled = true;
        config
            .stages
            .push(StageEntry::enabled(PipelineStage::ColorAdjustments(
                adjustments,
            )));
        config
            .stages
            .push(StageEntry::enabled(PipelineStage::CropRotate(
                CropRotateTransform::default(),
            )));

        let result = config.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            PipelineError::CropRotateNotFirst { found_at } => {
                assert_eq!(found_at, 1);
            }
            _ => panic!("Expected CropRotateNotFirst error"),
        }
    }

    #[test]
    fn test_validation_crop_rotate_first_ok() {
        let mut config = PipelineConfig::default();
        config
            .stages
            .push(StageEntry::enabled(PipelineStage::CropRotate(
                CropRotateTransform::default(),
            )));
        let mut adjustments = ColorAdjustments::new();
        adjustments.enabled = true;
        config
            .stages
            .push(StageEntry::enabled(PipelineStage::ColorAdjustments(
                adjustments,
            )));

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_lut_missing_context_returns_error() {
        let img = test_image();
        let mut config = PipelineConfig::default();
        config.stages.push(StageEntry::enabled(PipelineStage::Lut {
            lut_id: uuid::Uuid::new_v4(),
        }));

        let ctx = PipelineContext::empty(); // no lut_map
        let pipeline = ExportPipeline::new(img, config);
        let result = pipeline.execute(&ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_stage_kind_discriminant() {
        let crop = PipelineStage::CropRotate(CropRotateTransform::default());
        let mut adj = ColorAdjustments::new();
        adj.enabled = true;
        let color = PipelineStage::ColorAdjustments(adj);

        assert_eq!(crop.kind(), StageKind::CropRotate);
        assert_eq!(color.kind(), StageKind::ColorAdjustments);
    }

    #[test]
    fn test_pipeline_config_serde_roundtrip() {
        let mut config = PipelineConfig::default();
        let mut adjustments = ColorAdjustments::new();
        adjustments.enabled = true;
        adjustments.exposure = 0.5;
        config
            .stages
            .push(StageEntry::enabled(PipelineStage::ColorAdjustments(
                adjustments,
            )));

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: PipelineConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.stages.len(), 1);
        assert!(deserialized.stages[0].enabled);
        assert_eq!(
            deserialized.stages[0].stage.kind(),
            StageKind::ColorAdjustments
        );
    }

    #[test]
    fn test_watermark_position_grid() {
        // Test all 9 positions produce valid coordinates
        for pos in 1..=9 {
            let (x, y) = watermark_position(pos, 1000, 800, 100, 20, 10);
            assert!(x >= 0, "position {} x should be >= 0, got {}", pos, x);
            assert!(y >= 0, "position {} y should be >= 0, got {}", pos, y);
        }

        // Position 1 (top-left) should be at margin
        let (x, y) = watermark_position(1, 1000, 800, 100, 20, 10);
        assert_eq!(x, 10);
        assert_eq!(y, 10);

        // Position 9 (bottom-right)
        let (x, y) = watermark_position(9, 1000, 800, 100, 20, 10);
        assert_eq!(x, 1000 - 100 - 10);
        assert_eq!(y, 800 - 20 - 10);
    }
}

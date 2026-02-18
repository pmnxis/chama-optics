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
use crate::effect::sticker_storage::FaceArea;
use crate::effect::stroke::StrokeEffect;

use super::config::PipelineConfig;
use super::context::PipelineContext;
use super::stages::{Decoration, MosaicEffectConfig, PipelineStage, StrokeEffectConfig};
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
        PipelineStage::Lut { lut_id: _ } => {
            // TODO(Phase 2): Resolve LUT data from ctx.lut_data and apply
            let _ = ctx;
            log::warn!("Pipeline LUT stage: not yet implemented, skipping");
        }
        PipelineStage::FaceEffect {
            faces,
            mosaic,
            stroke,
            sticker: _,
        } => {
            apply_face_effects(image, faces, mosaic, stroke, ctx)?;
        }
        PipelineStage::Watermark(_config) => {
            // TODO(Phase 2): Implement watermark rendering
            log::warn!("Pipeline watermark stage: not yet implemented, skipping");
        }
    }
    Ok(())
}

/// Execute a `Decoration` on the given image.
pub(crate) fn execute_decoration(
    _image: &mut DynamicImage,
    decoration: &Decoration,
    _ctx: &PipelineContext,
) -> Result<(), PipelineError> {
    match decoration {
        Decoration::Theme(_config) => {
            // TODO(Phase 3): Resolve theme from ctx.theme_registry and apply
            log::warn!("Pipeline theme decoration: not yet implemented, skipping");
        }
        Decoration::Cheki(_config) => {
            // TODO(Phase 3): Apply cheki decoration
            log::warn!("Pipeline cheki decoration: not yet implemented, skipping");
        }
    }
    Ok(())
}

/// Face effects sub-pipeline: classify by mode, then batch apply.
fn apply_face_effects(
    image: &mut DynamicImage,
    faces: &[FaceArea],
    mosaic_config: &MosaicEffectConfig,
    stroke_config: &StrokeEffectConfig,
    _ctx: &PipelineContext,
) -> Result<(), PipelineError> {
    let mut mosaic_faces: Vec<(i32, i32, u32, u32)> = Vec::new();
    let mut stroke_faces: Vec<(i32, i32, u32, u32)> = Vec::new();

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
            FaceEffectMode::Sticker => {} // handled below
        }
    }

    if !mosaic_faces.is_empty() {
        let effect = MosaicEffect::new(mosaic_config.block_size, mosaic_config.intensity);
        if let Err(e) = MosaicEffect::apply(image, &mosaic_faces, &effect) {
            log::error!("Mosaic effect failed: {}", e);
        }
    }

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

    // TODO(Phase 2): Implement sticker application using ctx.sticker_storage
    let sticker_faces: Vec<&FaceArea> = faces
        .iter()
        .filter(|f| f.effect_mode == FaceEffectMode::Sticker && f.sticker_id.is_some())
        .collect();
    if !sticker_faces.is_empty() {
        log::warn!(
            "Pipeline sticker effect: {} faces with stickers, not yet implemented",
            sticker_faces.len()
        );
    }

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
    pub fn execute(
        mut self,
        ctx: &PipelineContext,
    ) -> Result<DynamicImage, PipelineError> {
        self.config.validate()?;

        let stages = std::mem::take(&mut self.config.stages);
        for entry in &stages {
            if !entry.enabled {
                continue;
            }
            execute_stage(&mut self.image, &entry.stage, ctx)?;
        }

        let decoration = self.config.decoration.take();
        if let Some(deco_entry) = &decoration {
            if deco_entry.enabled {
                execute_decoration(&mut self.image, &deco_entry.decoration, ctx)?;
            }
        }

        Ok(self.image)
    }
}

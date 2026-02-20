/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Preview pipeline with incremental stage caching.
//!
//! Shares the same execution logic as `ExportPipeline` but caches intermediate
//! results between stages. When a single parameter changes, only the dirty
//! stage and everything after it is re-executed.
//!
//! # Usage
//! ```rust,ignore
//! let mut preview = PreviewPipeline::new(thumbnail, config, &ctx);
//! preview.render()?; // initial full render
//!
//! // User adjusts contrast — only ColorAdjustments onward re-executes
//! preview.update_stage(StageKind::ColorAdjustments, new_color_adj);
//! preview.render()?;
//! ```

use image::DynamicImage;

use super::config::PipelineConfig;
use super::context::PipelineContext;
use super::execute::{execute_decoration, execute_stage};
use super::stages::{PipelineStage, StageKind};
use super::validation::PipelineError;

/// Preview pipeline with intermediate snapshot caching.
///
/// Each stage's output is cached. When a stage is updated via `update_stage`,
/// only that stage and subsequent stages are re-executed — earlier snapshots
/// are reused.
///
/// Designed for downscaled preview images. For full-resolution export,
/// use `ExportPipeline` instead (via `to_export_pipeline`).
pub struct PreviewPipeline {
    /// Original (typically downscaled) base image before any processing.
    base_image: DynamicImage,
    /// Pipeline configuration (shared with export).
    config: PipelineConfig,
    /// `snapshots[i]` = image after executing `stages[0..=i]`.
    /// `None` means the cache is invalid (dirty) for that stage.
    snapshots: Vec<Option<DynamicImage>>,
    /// First stage index that needs re-execution.
    /// Everything from this index onward is dirty.
    dirty_from: usize,
}

impl PreviewPipeline {
    /// Create a new preview pipeline.
    ///
    /// `base_image` should be a downscaled version of the original for fast preview.
    /// All snapshots start as dirty (initial `render()` will execute all stages).
    pub fn new(base_image: DynamicImage, config: PipelineConfig) -> Self {
        let num_stages = config.stages.len();
        Self {
            base_image,
            config,
            snapshots: vec![None; num_stages],
            dirty_from: 0,
        }
    }

    /// Update a stage's configuration by `StageKind`.
    ///
    /// Finds the stage matching `kind`, replaces its config, and invalidates
    /// all snapshots from that stage onward.
    ///
    /// Returns `true` if the stage was found and updated, `false` if no stage
    /// of that kind exists in the pipeline.
    pub fn update_stage(&mut self, kind: StageKind, new_stage: PipelineStage) -> bool {
        if let Some(index) = self.find_stage(kind) {
            self.config.stages[index].stage = new_stage;
            self.invalidate_from(index);
            true
        } else {
            false
        }
    }

    /// Toggle a stage's enabled flag by `StageKind`.
    ///
    /// Invalidates from that stage onward.
    pub fn toggle_stage(&mut self, kind: StageKind, enabled: bool) -> bool {
        if let Some(index) = self.find_stage(kind) {
            self.config.stages[index].enabled = enabled;
            self.invalidate_from(index);
            true
        } else {
            false
        }
    }

    /// Reorder stages. Invalidates all snapshots.
    ///
    /// `new_order` specifies the desired stage order by kind.
    /// Stages not in `new_order` are removed. Stages in `new_order`
    /// but not in the current config are ignored.
    pub fn reorder_stages(&mut self, new_order: &[StageKind]) {
        let mut reordered = Vec::with_capacity(new_order.len());
        for kind in new_order {
            if let Some(index) = self.find_stage(*kind) {
                reordered.push(self.config.stages[index].clone());
            }
        }
        self.config.stages = reordered;
        self.snapshots = vec![None; self.config.stages.len()];
        self.dirty_from = 0;
    }

    /// Render the preview, re-executing only dirty stages.
    ///
    /// Returns a reference to the final preview image (after all stages,
    /// before decoration).
    pub fn render(&mut self, ctx: &PipelineContext) -> Result<&DynamicImage, PipelineError> {
        self.config.validate()?;

        if self.config.stages.is_empty() {
            return Ok(&self.base_image);
        }

        // Ensure snapshots vec matches stages count
        self.snapshots
            .resize_with(self.config.stages.len(), || None);

        // Find the starting image: snapshot before dirty_from, or base_image
        let mut image = if self.dirty_from == 0 {
            self.base_image.clone()
        } else if let Some(ref snapshot) = self.snapshots[self.dirty_from - 1] {
            snapshot.clone()
        } else {
            // Fallback: no valid snapshot found, re-execute from start
            self.dirty_from = 0;
            self.base_image.clone()
        };

        // Execute from dirty_from onward
        for i in self.dirty_from..self.config.stages.len() {
            let entry = &self.config.stages[i];
            if entry.enabled {
                execute_stage(&mut image, &entry.stage, ctx)?;
            }
            self.snapshots[i] = Some(image.clone());
        }

        // All clean now
        self.dirty_from = self.config.stages.len();

        // Return last snapshot
        self.snapshots
            .last()
            .and_then(|s| s.as_ref())
            .ok_or_else(|| {
                PipelineError::StageError("Preview render produced no output".into())
            })
    }

    /// Render preview with decoration applied.
    ///
    /// Returns a new image (cloned from cached stages + decoration).
    /// Decoration is not cached since it may resize the image.
    pub fn render_with_decoration(
        &mut self,
        ctx: &PipelineContext,
    ) -> Result<DynamicImage, PipelineError> {
        let stages_result = self.render(ctx)?.clone();
        let mut image = stages_result;

        if let Some(deco_entry) = &self.config.decoration
            && deco_entry.enabled
        {
            execute_decoration(&mut image, &deco_entry.decoration, ctx)?;
        }

        Ok(image)
    }

    /// Build an `ExportPipeline` from the current config for full-resolution export.
    ///
    /// `full_image` is the original full-resolution image (not the preview thumbnail).
    pub fn to_export_pipeline(&self, full_image: DynamicImage) -> super::execute::ExportPipeline {
        super::execute::ExportPipeline::new(full_image, self.config.clone())
    }

    /// Get a reference to the current pipeline config.
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    // ─── Internal helpers ───

    fn find_stage(&self, kind: StageKind) -> Option<usize> {
        self.config
            .stages
            .iter()
            .position(|e| e.stage.kind() == kind)
    }

    fn invalidate_from(&mut self, index: usize) {
        self.dirty_from = self.dirty_from.min(index);
        for i in index..self.snapshots.len() {
            self.snapshots[i] = None;
        }
    }
}

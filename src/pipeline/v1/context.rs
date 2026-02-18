/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Pipeline runtime context — immutable references to platform resources.
//!
//! All references are immutable. LUT data must be resolved (loaded) before
//! pipeline execution to avoid borrow conflicts.

use crate::effect::sticker_storage::StickerStorage;
use crate::export_config::ExportConfig;
use crate::theme::ThemeRegistry;

/// Runtime dependencies for pipeline execution.
///
/// All fields are `Option` — stages that need a missing dependency will
/// return a descriptive error rather than panicking.
///
/// # Design decision: all references are immutable
///
/// LUT data is pre-resolved before `execute()` to avoid `&mut` borrow conflicts.
/// This also enables safe sharing across threads if parallelization is needed later.
pub struct PipelineContext<'a> {
    /// Sticker image storage (for FaceEffect with Sticker mode)
    pub sticker_storage: Option<&'a StickerStorage>,

    /// Pre-resolved LUT data (loaded before pipeline execution)
    /// Using a type-erased approach for now; concrete type TBD during Phase 2
    pub lut_data: Option<&'a dyn std::any::Any>,

    /// Theme registry (for Decoration::Theme)
    pub theme_registry: Option<&'a ThemeRegistry>,

    /// Export config (for Theme rendering parameters)
    pub export_config: Option<&'a ExportConfig>,
}

impl<'a> PipelineContext<'a> {
    /// Create an empty context (no resources available).
    /// Useful for testing stages that don't need external resources.
    pub fn empty() -> Self {
        Self {
            sticker_storage: None,
            lut_data: None,
            theme_registry: None,
            export_config: None,
        }
    }
}

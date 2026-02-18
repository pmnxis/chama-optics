/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Pipeline runtime context — immutable references to platform resources.
//!
//! All references are immutable. Resources (LUT, fonts, stickers) must be
//! resolved/loaded before pipeline execution to avoid borrow conflicts.

use std::collections::HashMap;

use crate::effect::sticker_storage::StickerStorage;
use crate::export_config::ExportConfig;
use crate::image::exif_impl::SimplifiedExif;
use crate::theme::ThemeRegistry;

/// Runtime dependencies for pipeline execution.
///
/// All fields are `Option` — stages that need a missing dependency will
/// return a descriptive error rather than panicking.
///
/// # Design decision: all references are immutable
///
/// LUT data and fonts are pre-resolved before `execute()` to avoid
/// `&mut` borrow conflicts. This also enables safe sharing across
/// threads if parallelization is needed later.
///
/// # Example
/// ```rust,ignore
/// let ctx = PipelineContext {
///     lut_map: Some(&lut_cache),
///     sticker_storage: Some(&sticker_storage),
///     font_map: Some(&fonts),
///     ..PipelineContext::empty()
/// };
/// ```
pub struct PipelineContext<'a> {
    /// Sticker image storage (for FaceEffect with Sticker mode, and Cheki decoration)
    pub sticker_storage: Option<&'a StickerStorage>,

    /// Pre-resolved LUT data, keyed by UUID.
    /// Caller must load/parse CubeLut before pipeline execution.
    pub lut_map: Option<&'a HashMap<uuid::Uuid, wagahai_lut::CubeLut>>,

    /// Pre-resolved fonts, keyed by font name.
    /// Used by Watermark and other text-rendering stages.
    pub font_map: Option<&'a HashMap<String, ab_glyph::FontArc>>,

    /// Theme registry (for Decoration::Theme)
    pub theme_registry: Option<&'a ThemeRegistry>,

    /// Export config (for Theme rendering parameters)
    pub export_config: Option<&'a ExportConfig>,

    /// EXIF metadata for Theme decoration text overlays.
    /// Themes use this to render camera info (model, lens, ISO, etc.).
    pub exif: Option<&'a SimplifiedExif>,
}

impl<'a> PipelineContext<'a> {
    /// Create an empty context (no resources available).
    /// Useful for testing stages that don't need external resources.
    pub fn empty() -> Self {
        Self {
            sticker_storage: None,
            lut_map: None,
            font_map: None,
            theme_registry: None,
            export_config: None,
            exif: None,
        }
    }
}

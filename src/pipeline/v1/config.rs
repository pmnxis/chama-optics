/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Pipeline configuration — the top-level config for export pipeline execution.

use serde::{Deserialize, Serialize};

use crate::export_config::output_format::OutputFormat;
use crate::export_config::scale_config::ScaleConfig;

use super::stages::{DecorationEntry, StageEntry};

/// Export pipeline configuration.
///
/// `stages` are executed in order, then `decoration` (if enabled) is applied last.
/// The entire config is JSON-serializable for FFI, CLI, presets, and debugging.
///
/// # Desktop usage (no JSON):
/// ```rust,ignore
/// let config = PipelineConfig::builder()
///     .stage(PipelineStage::ColorAdjustments(color))
///     .stage(PipelineStage::Lut { lut_id })
///     .decoration(Decoration::Theme(theme_config))
///     .build();
/// ```
///
/// # FFI usage (JSON):
/// ```json
/// {
///   "stages": [
///     { "type": "ColorAdjustments", "exposure": 0.5 },
///     { "type": "Lut", "lut_id": "abc-123" }
///   ],
///   "decoration": { "enabled": true, "type": "Theme", "name": "classic_border" },
///   "scale": { "mode": "None" },
///   "output_format": { "ext": "Jpeg", "quality": 95 }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineConfig {
    /// Ordered queue of processing stages.
    /// Each `StageEntry` has an `enabled` flag — disabled stages are skipped.
    #[serde(default)]
    pub stages: Vec<StageEntry>,

    /// Final decoration (Theme or Cheki, mutually exclusive).
    /// Applied after all stages. `None` means no decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoration: Option<DecorationEntry>,

    /// Output scaling configuration.
    pub scale: ScaleConfig,

    /// Output format and quality.
    pub output_format: OutputFormat,
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Pipeline stages and decoration types.
//!
//! All types are cfg-free and Serialize/Deserialize for JSON FFI/CLI/preset support.
//! Platform-specific types (egui::Color32, PathBuf-based configs) are replaced with
//! platform-agnostic equivalents ([u8; 4] for colors, String for font names).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::effect::color_adjustments::ColorAdjustments;
use crate::effect::crop_rotate::CropRotateTransform;
use crate::effect::sticker_storage::FaceArea;

// ─── Pipeline-specific effect configs (cfg-free, serde-enabled) ───

/// Mosaic effect configuration for pipeline.
/// Mirrors `crate::effect::mosaic::MosaicEffect` but with Serialize/Deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MosaicEffectConfig {
    /// Block size for mosaic (in pixels)
    pub block_size: u32,
    /// Blend intensity (0.0 = no effect, 1.0 = full mosaic)
    pub intensity: f32,
}

impl Default for MosaicEffectConfig {
    fn default() -> Self {
        Self {
            block_size: 10,
            intensity: 1.0,
        }
    }
}

/// Stroke/border effect configuration for pipeline.
/// Uses `[u8; 4]` instead of `(u8, u8, u8, u8)` for JSON compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StrokeEffectConfig {
    /// Thickness of stroke border in pixels
    pub thickness: u32,
    /// Color of stroke border (RGBA)
    pub color: [u8; 4],
}

impl Default for StrokeEffectConfig {
    fn default() -> Self {
        Self {
            thickness: 4,
            color: [255, 0, 0, 255],
        }
    }
}

/// Sticker effect configuration for pipeline.
/// Platform-agnostic — no PathBuf, no legacy sticker_id string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StickerEffectConfig {
    /// Scale factor for sticker overlay
    pub scale: f32,
    /// Horizontal offset in pixels
    pub offset_x: i32,
    /// Vertical offset in pixels
    pub offset_y: i32,
}

impl Default for StickerEffectConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset_x: 0,
            offset_y: 0,
        }
    }
}

/// Watermark configuration for pipeline.
/// Platform-agnostic — uses `[u8; 4]` for color, `String` for font name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkConfig {
    /// Watermark text
    pub text: String,
    /// Font size in points
    #[serde(default = "default_watermark_font_size")]
    pub font_size: f32,
    /// Font color (RGBA)
    #[serde(default = "default_watermark_font_color")]
    pub font_color: [u8; 4],
    /// Font name (resolved to path at runtime)
    #[serde(default)]
    pub font_name: Option<String>,
    /// Position (0=top-left, 1=top-right, 2=bottom-left, 3=bottom-right, etc.)
    #[serde(default = "default_watermark_position")]
    pub position: u8,
    /// Whether to also show as screen overlay (desktop only)
    #[serde(default)]
    pub is_screen_overlay: bool,
}

fn default_watermark_font_size() -> f32 {
    24.0
}

fn default_watermark_font_color() -> [u8; 4] {
    [255, 255, 255, 200]
}

fn default_watermark_position() -> u8 {
    3 // bottom-right
}

// ─── Theme / Cheki config ───

/// Theme configuration for pipeline.
/// The `name` maps to a registered theme in ThemeRegistry at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Theme unique name (e.g. "classic_border", "film_date")
    pub name: String,
    /// Theme-specific parameters (JSON object, interpreted by each theme)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

// Note: ChekiDecoration is imported from crate::effect::cheki::ChekiDecoration
// It already has Serialize/Deserialize derives.

// ─── StageKind discriminant ───

/// Stage type identifier — used to find/track stages regardless of queue position.
///
/// When users reorder stages (e.g. LUT before ColorAdjustments),
/// `StageKind` allows lookup by type rather than by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageKind {
    CropRotate,
    ColorAdjustments,
    Lut,
    FaceEffect,
    Watermark,
}

// ─── PipelineStage enum ───

/// A single processing stage in the pipeline queue.
///
/// Wrapped by `StageEntry` which adds an `enabled` flag.
/// Stages are executed in order; CropRotate must be first if present.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PipelineStage {
    /// Image crop/rotation — must be first stage if present.
    CropRotate(CropRotateTransform),

    /// Lightroom-style color adjustments (exposure, contrast, etc.)
    ColorAdjustments(ColorAdjustments),

    /// LUT color grading by UUID (resolved from LUT storage at runtime)
    Lut {
        lut_id: Uuid,
    },

    /// Face effects (mosaic, stroke, sticker) applied to detected faces.
    /// Each FaceArea carries its own effect_mode, enabling per-face effects.
    /// The same FaceEffect stage can appear multiple times for different face groups.
    FaceEffect {
        faces: Vec<FaceArea>,
        #[serde(default)]
        mosaic: MosaicEffectConfig,
        #[serde(default)]
        stroke: StrokeEffectConfig,
        #[serde(default)]
        sticker: StickerEffectConfig,
    },

    /// Watermark text overlay
    Watermark(WatermarkConfig),
}

impl PipelineStage {
    /// Returns the discriminant identifying this stage's type.
    pub fn kind(&self) -> StageKind {
        match self {
            PipelineStage::CropRotate(_) => StageKind::CropRotate,
            PipelineStage::ColorAdjustments(_) => StageKind::ColorAdjustments,
            PipelineStage::Lut { .. } => StageKind::Lut,
            PipelineStage::FaceEffect { .. } => StageKind::FaceEffect,
            PipelineStage::Watermark(_) => StageKind::Watermark,
        }
    }
}

// ─── Decoration enum (Theme or Cheki, mutually exclusive) ───

/// Final decoration layer — always applied last, after all stages.
///
/// Theme and Cheki are mutually exclusive (enforced by enum).
/// Both add borders/decorations and may resize the image.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Decoration {
    /// Theme: border/logo/text overlay with final resize
    Theme(ThemeConfig),
    /// Cheki: polaroid-style border, text, dice stickers
    Cheki(crate::effect::cheki::ChekiDecoration),
}

// ─── StageEntry / DecorationEntry wrappers ───

/// Wrapper around `PipelineStage` with an `enabled` flag.
///
/// When `enabled` is false, the stage is skipped but its configuration is preserved.
/// This allows UI toggles without losing settings.
///
/// JSON: `{ "enabled": true, "type": "ColorAdjustments", "exposure": 0.5, ... }`
/// The `enabled` field defaults to `true` when omitted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageEntry {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(flatten)]
    pub stage: PipelineStage,
}

fn default_true() -> bool {
    true
}

impl StageEntry {
    /// Create an enabled stage entry.
    pub fn enabled(stage: PipelineStage) -> Self {
        Self {
            enabled: true,
            stage,
        }
    }

    /// Create a disabled stage entry (configuration preserved, skipped during execution).
    pub fn disabled(stage: PipelineStage) -> Self {
        Self {
            enabled: false,
            stage,
        }
    }
}

/// Wrapper around `Decoration` with an `enabled` flag.
///
/// - `None` → no decoration at all
/// - `Some({ enabled: false, ... })` → settings preserved, decoration skipped
/// - `Some({ enabled: true, Theme(...) })` → theme applied
/// - `Some({ enabled: true, Cheki(...) })` → cheki applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecorationEntry {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(flatten)]
    pub decoration: Decoration,
}

impl DecorationEntry {
    pub fn enabled(decoration: Decoration) -> Self {
        Self {
            enabled: true,
            decoration,
        }
    }

    pub fn disabled(decoration: Decoration) -> Self {
        Self {
            enabled: false,
            decoration,
        }
    }
}

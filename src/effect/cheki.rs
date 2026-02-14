// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cheki (Japanese polaroid) decoration data model
//!
//! Stores per-image cheki decoration state including text, font selection,
//! and randomly placed character stickers. This is NOT a theme - it is
//! a decoration layer applied on top of any selected theme.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A sticker placed at a specific position on the cheki
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedSticker {
    /// References a sticker in StickerStorage
    pub sticker_id: Uuid,
    /// Normalized x position (0.0-1.0 relative to image width)
    pub x: f32,
    /// Normalized y position (0.0-1.0 relative to image height)
    pub y: f32,
    /// Scale factor relative to image dimension (e.g., 0.1 = 10% of image)
    pub scale: f32,
    /// Rotation in degrees (slight random tilt for natural feel)
    pub rotation: f32,
}

/// Font selection for cheki text rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ChekiFontSelection {
    #[default]
    Barlow,
    BarlowNarrow,
    SourceHanSans,
}

impl ChekiFontSelection {
    pub fn all() -> &'static [Self] {
        &[Self::Barlow, Self::BarlowNarrow, Self::SourceHanSans]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Barlow => "Barlow",
            Self::BarlowNarrow => "Barlow Narrow",
            Self::SourceHanSans => "Source Han Sans",
        }
    }
}

/// Per-image cheki decoration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChekiDecoration {
    /// Whether cheki decoration is enabled for this image
    pub enabled: bool,
    /// Text to render on the cheki (bottom border area)
    pub text: String,
    /// Font selection for text rendering
    pub font: ChekiFontSelection,
    /// Font size relative to border height (0.1 to 1.0)
    pub font_size: f32,
    /// Text color
    #[cfg(feature = "egui")]
    pub text_color: egui::Color32,
    #[cfg(not(feature = "egui"))]
    pub text_color: [u8; 4],
    /// Normalized text position within bottom border area (0.0-1.0)
    pub text_position_x: f32,
    pub text_position_y: f32,
    /// Randomly placed character stickers (via dice)
    pub dice_stickers: Vec<PlacedSticker>,
    /// Border width as fraction of image shorter dimension (e.g., 0.05 = 5%)
    pub border_width: f32,
    /// Extra bottom border height as fraction of image height (for text area)
    pub bottom_extra: f32,
    /// Border color
    #[cfg(feature = "egui")]
    pub border_color: egui::Color32,
    #[cfg(not(feature = "egui"))]
    pub border_color: [u8; 4],
}

impl ChekiDecoration {
    /// Compute a content hash for cache invalidation.
    /// Changes to any visual parameter will produce a different hash.
    pub fn content_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.enabled.hash(&mut hasher);
        self.text.hash(&mut hasher);
        (self.font as u8).hash(&mut hasher);
        self.font_size.to_bits().hash(&mut hasher);
        self.text_position_x.to_bits().hash(&mut hasher);
        self.text_position_y.to_bits().hash(&mut hasher);
        self.border_width.to_bits().hash(&mut hasher);
        self.bottom_extra.to_bits().hash(&mut hasher);
        self.dice_stickers.len().hash(&mut hasher);
        for s in &self.dice_stickers {
            s.sticker_id.hash(&mut hasher);
            s.x.to_bits().hash(&mut hasher);
            s.y.to_bits().hash(&mut hasher);
            s.scale.to_bits().hash(&mut hasher);
        }
        #[cfg(feature = "egui")]
        {
            self.text_color.to_array().hash(&mut hasher);
            self.border_color.to_array().hash(&mut hasher);
        }
        #[cfg(not(feature = "egui"))]
        {
            self.text_color.hash(&mut hasher);
            self.border_color.hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl Default for ChekiDecoration {
    fn default() -> Self {
        Self {
            enabled: true,
            text: String::new(),
            font: ChekiFontSelection::default(),
            font_size: 0.5,
            #[cfg(feature = "egui")]
            text_color: egui::Color32::from_rgb(0, 180, 180), // Teal like the examples
            #[cfg(not(feature = "egui"))]
            text_color: [0, 180, 180, 255],
            text_position_x: 0.5,
            text_position_y: 0.5,
            dice_stickers: Vec::new(),
            border_width: 0.04,
            bottom_extra: 0.15,
            #[cfg(feature = "egui")]
            border_color: egui::Color32::from_rgb(240, 245, 240), // Slightly off-white
            #[cfg(not(feature = "egui"))]
            border_color: [240, 245, 240, 255],
        }
    }
}

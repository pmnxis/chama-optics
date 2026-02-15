// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cheki (Japanese polaroid) decoration data model
//!
//! Stores per-image cheki decoration state including text, font selection,
//! and randomly placed character stickers. This is NOT a theme - it is
//! a decoration layer applied on top of any selected theme.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
use crate::effect::variable_text::VariableOrNot;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
use crate::fonts::variable_font::BuiltinVariableFontIndex;

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

/// Date stamp position within the cheki border area (2x3 grid)
/// Top row = top border (above image), Bottom row = bottom border (text area)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DatePosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    #[default]
    BottomRight,
}

/// Per-image cheki decoration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChekiDecoration {
    /// Whether cheki decoration is enabled for this image
    pub enabled: bool,
    /// Text to render on the cheki (bottom border area)
    pub text: String,
    /// Font selection for text rendering (variable or fixed)
    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub font: VariableOrNot,
    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    pub font_file: String,
    /// Font weight for variable font rendering
    pub font_weight: u16,
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
    /// Clip stickers to the image area (hide overflow beyond border)
    pub clip_stickers: bool,
    /// Allow rotation when placing stickers via dice (default: false)
    pub allow_rotation: bool,
    /// Date stamp text (editable, auto-populated from EXIF)
    pub date_text: String,
    /// Whether date stamp is enabled
    pub date_enabled: bool,
    /// Date stamp font (default: DynaPuff)
    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub date_font: VariableOrNot,
    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    pub date_font_file: String,
    /// Date stamp font weight
    pub date_font_weight: u16,
    /// Date stamp font size relative to border height
    pub date_font_size: f32,
    /// Date stamp color
    #[cfg(feature = "egui")]
    pub date_color: egui::Color32,
    #[cfg(not(feature = "egui"))]
    pub date_color: [u8; 4],
    /// Date stamp position in 2x3 grid
    pub date_position: DatePosition,
}

impl ChekiDecoration {
    /// Compute a content hash for cache invalidation.
    /// Changes to any visual parameter will produce a different hash.
    pub fn content_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.enabled.hash(&mut hasher);
        self.text.hash(&mut hasher);
        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        match &self.font {
            VariableOrNot::Variable(idx) => {
                0u8.hash(&mut hasher);
                (*idx as usize).hash(&mut hasher);
            }
            #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
            VariableOrNot::Others(fs) => {
                1u8.hash(&mut hasher);
                fs.name.hash(&mut hasher);
            }
        }
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        self.font_file.hash(&mut hasher);
        self.font_weight.hash(&mut hasher);
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
        self.date_text.hash(&mut hasher);
        self.date_enabled.hash(&mut hasher);
        #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
        match &self.date_font {
            VariableOrNot::Variable(idx) => {
                0u8.hash(&mut hasher);
                (*idx as usize).hash(&mut hasher);
            }
            #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
            VariableOrNot::Others(fs) => {
                1u8.hash(&mut hasher);
                fs.name.hash(&mut hasher);
            }
        }
        #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
        self.date_font_file.hash(&mut hasher);
        self.date_font_weight.hash(&mut hasher);
        self.date_font_size.to_bits().hash(&mut hasher);
        (self.date_position as u8).hash(&mut hasher);
        #[cfg(feature = "egui")]
        {
            self.text_color.to_array().hash(&mut hasher);
            self.border_color.to_array().hash(&mut hasher);
            self.date_color.to_array().hash(&mut hasher);
        }
        #[cfg(not(feature = "egui"))]
        {
            self.text_color.hash(&mut hasher);
            self.border_color.hash(&mut hasher);
            self.date_color.hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl Default for ChekiDecoration {
    fn default() -> Self {
        Self {
            enabled: true,
            text: String::new(),
            #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
            font: VariableOrNot::Variable(BuiltinVariableFontIndex::Barlow),
            #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
            font_file: String::new(),
            font_weight: 300,
            font_size: 0.5,
            #[cfg(feature = "egui")]
            text_color: egui::Color32::from_rgb(0, 180, 180),
            #[cfg(not(feature = "egui"))]
            text_color: [0, 180, 180, 255],
            text_position_x: 0.5,
            text_position_y: 0.5,
            dice_stickers: Vec::new(),
            border_width: 0.04,
            bottom_extra: 0.15,
            #[cfg(feature = "egui")]
            border_color: egui::Color32::from_rgb(240, 245, 240),
            #[cfg(not(feature = "egui"))]
            border_color: [240, 245, 240, 255],
            clip_stickers: false,
            allow_rotation: false,
            date_text: String::new(),
            date_enabled: false,
            #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
            date_font: VariableOrNot::Others(
                crate::fonts::FONTS_UNIFY
                    .builtin_select(crate::fonts::font_unify::BuiltinFontIndex::DynaPuff),
            ),
            #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
            date_font_file: String::new(),
            date_font_weight: 400,
            date_font_size: 0.4,
            #[cfg(feature = "egui")]
            date_color: egui::Color32::from_rgb(255, 140, 0),
            #[cfg(not(feature = "egui"))]
            date_color: [255, 140, 0, 255],
            date_position: DatePosition::default(),
        }
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Headless scale configuration (no GUI dependencies)
//! This is a simplified version of export_config/scale_config.rs for iOS builds

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScaleMode {
    None,
    MaxWidth,
    MaxHeight,
    Longside,
    Divide,
    NearCommonDivisorConsiderWidth,
    NearCommonDivisorConsiderHeight,
    ResizeAndCrop,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct ScaleConfig {
    pub mode: ScaleMode,
    pub value: u32,
    pub sub_value: u32,
    pub scale_value: f64,
}

impl Default for ScaleConfig {
    fn default() -> Self {
        Self {
            mode: ScaleMode::Longside,
            value: 3840,
            sub_value: 2160,
            scale_value: 1.0,
        }
    }
}

impl ScaleConfig {
    pub fn apply(&self, src_width: u32, src_height: u32, is_vertical_rotated: bool) -> (u32, u32) {
        match self.mode {
            ScaleMode::None => (src_width, src_height),
            ScaleMode::MaxWidth => {
                let target_width = self.value;
                if src_width <= target_width {
                    (src_width, src_height)
                } else {
                    let ratio = target_width as f64 / src_width as f64;
                    (target_width, (src_height as f64 * ratio).round() as u32)
                }
            }
            ScaleMode::MaxHeight => {
                let target_height = self.value;
                if src_height <= target_height {
                    (src_width, src_height)
                } else {
                    let ratio = target_height as f64 / src_height as f64;
                    ((src_width as f64 * ratio).round() as u32, target_height)
                }
            }
            ScaleMode::Longside => {
                let longside_value = self.value;
                if (is_vertical_rotated && src_height >= src_width)
                    || (!is_vertical_rotated && src_width >= src_height)
                {
                    // Width is longside
                    if src_width <= longside_value {
                        (src_width, src_height)
                    } else {
                        let ratio = longside_value as f64 / src_width as f64;
                        (longside_value, (src_height as f64 * ratio).round() as u32)
                    }
                } else {
                    // Height is longside
                    if src_height <= longside_value {
                        (src_width, src_height)
                    } else {
                        let ratio = longside_value as f64 / src_height as f64;
                        ((src_width as f64 * ratio).round() as u32, longside_value)
                    }
                }
            }
            ScaleMode::Divide => {
                let divider = self.scale_value;
                (
                    (src_width as f64 / divider).round() as u32,
                    (src_height as f64 / divider).round() as u32,
                )
            }
            ScaleMode::NearCommonDivisorConsiderWidth => {
                // Simplified version
                let target_width = self.value;
                let ratio = target_width as f64 / src_width as f64;
                (target_width, (src_height as f64 * ratio).round() as u32)
            }
            ScaleMode::NearCommonDivisorConsiderHeight => {
                // Simplified version
                let target_height = self.value;
                let ratio = target_height as f64 / src_height as f64;
                ((src_width as f64 * ratio).round() as u32, target_height)
            }
            ScaleMode::ResizeAndCrop => {
                // For thumbnails - resize to fit and crop center
                let target_width = self.value;
                let target_height = self.sub_value;

                let width_ratio = target_width as f64 / src_width as f64;
                let height_ratio = target_height as f64 / src_height as f64;
                let ratio = width_ratio.max(height_ratio);

                (
                    (src_width as f64 * ratio).round() as u32,
                    (src_height as f64 * ratio).round() as u32,
                )
            }
        }
    }
}

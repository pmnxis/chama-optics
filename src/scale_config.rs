/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use rust_i18n::t;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

#[derive(EnumIter, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScaleMode {
    /// No scale
    None,

    /// Resize with width with aspect ratio
    MaxWidth,

    /// Resize with height with aspect ratio
    MaxHeight,

    /// Resize with dividing width and height
    Divide,

    /// Choose nearest width by considering common divisor
    NearCommonDivisorConsiderWidth,

    /// Choose nearest height by considering common divisor
    NearCommonDivisorConsiderHeight,

    /// Resize and crop fill to specific size
    /// This option mostly used for thumbnail
    ResizeAndCrop,
}

impl ScaleMode {
    fn label(&self) -> std::borrow::Cow<'static, str> {
        match self {
            ScaleMode::None => t!("scale_config.none"),
            ScaleMode::MaxWidth => t!("scale_config.max_width"),
            ScaleMode::MaxHeight => t!("scale_config.max_height"),
            ScaleMode::Divide => t!("scale_config.divide"),
            ScaleMode::NearCommonDivisorConsiderWidth => {
                t!("scale_config.near_common_divisor_width")
            }
            ScaleMode::NearCommonDivisorConsiderHeight => {
                t!("scale_config.near_common_divisor_height")
            }
            ScaleMode::ResizeAndCrop => {
                t!("scale_config.resize_and_crop")
            }
        }
    }

    fn field_label(&self) -> std::borrow::Cow<'static, str> {
        match self {
            ScaleMode::None => t!("scale_config.field.none"),
            ScaleMode::MaxWidth => t!("scale_config.field.max_width"),
            ScaleMode::MaxHeight => t!("scale_config.field.max_height"),
            ScaleMode::Divide => t!("scale_config.field.divide"),
            ScaleMode::NearCommonDivisorConsiderWidth => {
                t!("scale_config.field.near_common_divisor_width")
            }
            ScaleMode::NearCommonDivisorConsiderHeight => {
                t!("scale_config.field.near_common_divisor_height")
            }
            ScaleMode::ResizeAndCrop => {
                t!("scale_config.field.resize_and_crop")
            }
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]

pub struct ScaleConfig {
    pub mode: ScaleMode,
    pub value: u32,
    pub sub_value: u32,
}

impl core::default::Default for ScaleConfig {
    fn default() -> Self {
        Self {
            mode: ScaleMode::NearCommonDivisorConsiderWidth,
            value: 4072,
            sub_value: 3054,
        }
    }
}

pub const SCALE_NEAR_COMMON_4K: ScaleConfig = ScaleConfig {
    mode: ScaleMode::NearCommonDivisorConsiderWidth,
    value: 4072,
    sub_value: 3054,
};

impl ScaleConfig {
    fn __apply(&self, width: u32, height: u32) -> (u32, u32) {
        match *self {
            Self {
                mode: ScaleMode::None,
                ..
            } => (width, height),

            Self {
                mode: ScaleMode::MaxWidth,
                value: target_w,
                ..
            } => {
                if width == 0 || height == 0 || width < target_w {
                    return (width, height);
                }

                let ratio = target_w as f64 / width as f64;
                let new_h = (height as f64 * ratio).round() as u32;
                (target_w, new_h)
            }

            Self {
                mode: ScaleMode::MaxHeight,
                value: target_h,
                ..
            } => {
                if width == 0 || height == 0 || height < target_h {
                    return (width, height);
                }
                let ratio = target_h as f64 / height as f64;
                let new_w = (width as f64 * ratio).round() as u32;
                (new_w, target_h)
            }

            Self {
                mode: ScaleMode::Divide,
                value: div,
                ..
            } => {
                if div == 0 {
                    return (width, height);
                }
                (width / div, height / div)
            }

            Self {
                mode: ScaleMode::NearCommonDivisorConsiderWidth,
                value: target_w,
                ..
            } => {
                let gcd = gcd::euclid_nonzero_u32(
                    std::num::NonZero::new(width).unwrap(),
                    std::num::NonZero::new(height).unwrap(),
                );
                let w_unit = width / gcd;
                let h_unit = height / gcd;

                let k = (target_w as f64 / w_unit as f64).round() as u32;
                (w_unit * k, h_unit * k)
            }

            Self {
                mode: ScaleMode::NearCommonDivisorConsiderHeight,
                value: target_h,
                ..
            } => {
                let gcd = gcd::euclid_nonzero_u32(
                    std::num::NonZero::new(width).unwrap(),
                    std::num::NonZero::new(height).unwrap(),
                );
                let w_unit = width / gcd;
                let h_unit = height / gcd;

                let k = (target_h as f64 / h_unit as f64).round() as u32;
                (w_unit * k, h_unit * k)
            }
            Self {
                mode: ScaleMode::ResizeAndCrop,
                value: target_w,
                sub_value: target_h,
            } => {
                if width == 0 || height == 0 {
                    return (width, height);
                }

                let src_ratio = width as f64 / height as f64;
                let dst_ratio = target_w as f64 / target_h as f64;

                if src_ratio > dst_ratio {
                    let new_h = target_h;
                    let ratio = new_h as f64 / height as f64;
                    let new_w = (width as f64 * ratio).round() as u32;
                    (new_w, new_h)
                } else {
                    let new_w = target_w;
                    let ratio = new_w as f64 / width as f64;
                    let new_h = (height as f64 * ratio).round() as u32;
                    (new_w, new_h)
                }
            }
        }
    }

    pub fn apply(&self, width: u32, height: u32, is_vert_rot: bool) -> (u32, u32) {
        if !is_vert_rot {
            self.__apply(width, height)
        } else {
            Self {
                mode: match self.mode {
                    ScaleMode::MaxWidth => ScaleMode::MaxHeight,
                    ScaleMode::MaxHeight => ScaleMode::MaxWidth,
                    ScaleMode::NearCommonDivisorConsiderWidth => {
                        ScaleMode::NearCommonDivisorConsiderHeight
                    }
                    ScaleMode::NearCommonDivisorConsiderHeight => {
                        ScaleMode::NearCommonDivisorConsiderWidth
                    }
                    others => others,
                },
                value: if self.mode != ScaleMode::ResizeAndCrop {
                    self.value
                } else {
                    self.sub_value
                },
                sub_value: if self.mode != ScaleMode::ResizeAndCrop {
                    self.sub_value
                } else {
                    self.value
                },
            }
            .__apply(width, height)
        }
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                ui.label(t!("scale_config.label"));
                egui::ComboBox::from_id_salt("scale_mode")
                    .selected_text(self.mode.label())
                    .show_ui(ui, |ui| {
                        for scale in ScaleMode::iter() {
                            ui.selectable_value(&mut self.mode, scale, scale.label());
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label(self.mode.field_label());
                if self.mode == ScaleMode::Divide {
                    ui.add(egui::DragValue::new(&mut self.value).range(1..=1024));
                    ui.label(t!("scale_config.px_std"));
                } else if self.mode != ScaleMode::None {
                    ui.add(egui::DragValue::new(&mut self.value).range(1..=20000));
                    ui.label(t!("scale_config.px_std"));
                }
            });
        });
    }
}

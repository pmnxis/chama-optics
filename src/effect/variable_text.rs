/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::fonts::variable_font::*;
use egui::{self, Align, Ui};
use std::borrow::Cow;

pub struct VariableTextSlotDefault {
    pub text: &'static str,
    pub weight: u16,
    pub variable_font_index: BuiltinVariableFontIndex,
}

impl VariableTextSlotDefault {
    pub const fn with_barlow(default: &'static str) -> Self {
        Self {
            text: default,
            weight: 300, // todo - get method with const gently
            variable_font_index: BuiltinVariableFontIndex::Barlow,
        }
    }
}

impl From<VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: VariableTextSlotDefault) -> Self {
        Self {
            text: value.text.into(),
            weight: value.weight,
            variable_font_index: value.variable_font_index,
        }
    }
}
impl From<&VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: &VariableTextSlotDefault) -> Self {
        Self {
            text: value.text.into(),
            weight: value.weight,
            variable_font_index: value.variable_font_index,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct VariableTextSlot {
    pub text: String,
    pub weight: u16,
    // todo - future selection variable fonts
    pub variable_font_index: BuiltinVariableFontIndex,
}

impl VariableTextSlot {
    #[allow(dead_code)]
    pub fn new(default: &'static str, weight: u16) -> Self {
        Self {
            text: default.to_string(),
            weight,
            variable_font_index: BuiltinVariableFontIndex::default(),
        }
    }

    pub fn from_default(default: &'static VariableTextSlotDefault) -> Self {
        default.into()
    }

    pub fn format_custom(&self, exif: &crate::image::exif_impl::SimplifiedExif) -> String {
        exif.format_custom(self.text.clone())
    }

    pub fn get_font(&self) -> &'static ab_glyph::FontArc {
        self.variable_font_index.get_font_by_weight(self.weight)
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        label: Cow<'static, str>,
        default: &'static VariableTextSlotDefault,
    ) {
        ui.label(label);

        ui.horizontal(|ui| {
            let font_pack = self.variable_font_index.get_font();
            let (start, end, step) = font_pack.range();
            let total_width = ui.available_width();
            let slider_width = (total_width * 0.30).min(195.0);
            let text_width = (total_width * 0.60).max(total_width - 195.0);

            ui.add_sized(
                [text_width, 23.0],
                egui::TextEdit::singleline(&mut self.text).vertical_align(Align::Center),
            );
            ui.add_sized(
                [slider_width, 23.0],
                egui::Slider::new(&mut self.weight, start..=end).step_by(step.into()),
            );
            if ui.button("↺").clicked() {
                *self = default.into();
            }

            // todo - font selection with variable_font_index
        });
    }
}

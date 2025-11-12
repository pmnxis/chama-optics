/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::fonts::variable_font::*;
use egui::{self, Align, Ui};
use std::borrow::Cow;

#[rustfmt::skip]
#[repr(usize)]
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub enum VarialbeOrNot {
    Variable(BuiltinVariableFontIndex),
    Others(crate::fonts::font_unify::FontSelection),
}

impl std::default::Default for VarialbeOrNot {
    fn default() -> Self {
        Self::Variable(BuiltinVariableFontIndex::Barlow)
    }
}

pub struct VariableTextSlotDefault {
    pub text: &'static str,
    pub weight: u16,
    pub font_index: BuiltinVariableFontIndex,
}

impl VariableTextSlotDefault {
    pub const fn with_barlow(default: &'static str) -> Self {
        Self {
            text: default,
            weight: 300, // todo - get method with const gently
            font_index: BuiltinVariableFontIndex::Barlow,
        }
    }
}

impl From<VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: VariableTextSlotDefault) -> Self {
        Self {
            text: value.text.into(),
            weight: value.weight,
            font_index: VarialbeOrNot::Variable(value.font_index),
        }
    }
}
impl From<&VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: &VariableTextSlotDefault) -> Self {
        Self {
            text: value.text.into(),
            weight: value.weight,
            font_index: VarialbeOrNot::Variable(value.font_index),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct VariableTextSlot {
    pub text: String,
    pub weight: u16,
    // todo - future selection variable fonts
    pub font_index: VarialbeOrNot,
}

impl VariableTextSlot {
    #[allow(dead_code)]
    pub fn new(default: &'static str, weight: u16) -> Self {
        Self {
            text: default.to_string(),
            weight,
            font_index: VarialbeOrNot::default(),
        }
    }

    pub fn from_default(default: &'static VariableTextSlotDefault) -> Self {
        default.into()
    }

    pub fn format_custom(&self, exif: &crate::image::exif_impl::SimplifiedExif) -> String {
        exif.format_custom(self.text.clone())
    }

    pub fn get_font(&self) -> ab_glyph::FontArc {
        // todo - resolve &'static, &, * hell
        // todo - Result<T,E>
        match &self.font_index {
            VarialbeOrNot::Variable(var) => var.get_font_by_weight(self.weight).clone(),
            VarialbeOrNot::Others(others) => match crate::FONTS_UNIFY.search(&others) {
                Ok(x) => x,
                Err(e) => {
                    log::error!("{}", e);
                    BuiltinVariableFontIndex::Barlow
                        .get_font_by_weight(self.weight)
                        .clone()
                }
            },
        }
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        ui: &mut Ui,
        label: Cow<'static, str>,
        default: &'static VariableTextSlotDefault,
    ) {
        // ensure outside is grid ui

        ui.label(label.clone());
        ui.horizontal(|ui| {
            let total_width = ui.available_width();

            if let VarialbeOrNot::Others(font_select) = &mut self.font_index {
                font_select.update_ui_with_label(ctx, ui, label.clone());
            } else if let VarialbeOrNot::Variable(mut variable_select) = self.font_index {
                variable_select.update_ui_with_label(ui, label.clone());
            }
        });

        ui.end_row();

        ui.label(label.clone());

        ui.horizontal(|ui| {
            // let font_pack = self.font_index.get_font();
            // let (start, end, step) = font_pack.range();
            let total_width = ui.available_width();
            let slider_width = (total_width * 0.30).min(195.0);
            let text_width = (total_width * 0.60).max(total_width - 195.0);

            ui.add_sized(
                [text_width, 23.0],
                egui::TextEdit::singleline(&mut self.text).vertical_align(Align::Center),
            );
            // ui.add_sized(
            //     [slider_width, 23.0],
            //     egui::Slider::new(&mut self.weight, start..=end).step_by(step.into()),
            // );
            if ui.button("↺").clicked() {
                *self = default.into();
            }

            // todo - font selection with variable_font_index
        });
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::fonts::variable_font::*;
use egui::{self, Align, Ui};
use rust_i18n::t;
use std::borrow::Cow;

#[rustfmt::skip]
#[repr(usize)]
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub enum VariableOrNot {
    Variable(BuiltinVariableFontIndex),
    Others(crate::fonts::font_unify::FontSelection),
}

impl std::default::Default for VariableOrNot {
    fn default() -> Self {
        Self::Variable(BuiltinVariableFontIndex::Barlow)
    }
}

pub struct VariableTextSlotDefault {
    pub text: &'static str,
    pub weight: u16,
    // variable index
    pub font_index: BuiltinVariableFontIndex,
    // fixed index
    pub fixed_index: Option<crate::BuiltinFontIndex>,
    pub prefer_fixed: bool,
}

impl VariableTextSlotDefault {
    pub const fn with_barlow(default: &'static str) -> Self {
        Self {
            text: default,
            weight: 300, // todo - get method with const gently
            font_index: BuiltinVariableFontIndex::Barlow,
            fixed_index: None,
            prefer_fixed: false,
        }
    }

    pub const fn with_barlow_weight(default: &'static str, weight: u16) -> Self {
        Self {
            text: default,
            weight,
            font_index: BuiltinVariableFontIndex::Barlow,
            fixed_index: None,
            prefer_fixed: false,
        }
    }

    pub const fn with_digital7(default: &'static str) -> Self {
        Self {
            text: default,
            weight: 300, // todo - get method with const gently
            font_index: BuiltinVariableFontIndex::Barlow, // don't need for default
            fixed_index: Some(crate::BuiltinFontIndex::Digital7),
            prefer_fixed: true, // will select fixed_index
        }
    }
}

impl From<VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: VariableTextSlotDefault) -> Self {
        if let Some(Some(x)) = value.prefer_fixed.then_some(value.fixed_index) {
            Self {
                text: value.text.into(),
                weight: value.weight,
                font_index: VariableOrNot::Others(crate::FONTS_UNIFY.builtin_select(x)),
            }
        } else {
            Self {
                text: value.text.into(),
                weight: value.weight,
                font_index: VariableOrNot::Variable(value.font_index),
            }
        }
    }
}
impl From<&VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: &VariableTextSlotDefault) -> Self {
        if let Some(Some(x)) = value.prefer_fixed.then_some(value.fixed_index) {
            Self {
                text: value.text.into(),
                weight: value.weight,
                font_index: VariableOrNot::Others(crate::FONTS_UNIFY.builtin_select(x)),
            }
        } else {
            Self {
                text: value.text.into(),
                weight: value.weight,
                font_index: VariableOrNot::Variable(value.font_index),
            }
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct VariableTextSlot {
    pub text: String,
    pub weight: u16,
    // todo - future selection variable fonts
    pub font_index: VariableOrNot,
}

impl VariableTextSlot {
    #[allow(dead_code)]
    pub fn new(default: &'static str, weight: u16) -> Self {
        Self {
            text: default.to_string(),
            weight,
            font_index: VariableOrNot::default(),
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
            VariableOrNot::Variable(var) => var.get_font_by_weight(self.weight).clone(),
            VariableOrNot::Others(others) => match crate::FONTS_UNIFY.search(others) {
                Ok(x) => x,
                Err(e) => {
                    log::error!("{e}");
                    BuiltinVariableFontIndex::Barlow
                        .get_font_by_weight(self.weight)
                        .clone()
                }
            },
        }
    }

    /// return false on bool tuple when it's not variable font
    pub fn get_font_with_new_weight(&self, weight: u16) -> (ab_glyph::FontArc, bool) {
        // todo - resolve &'static, &, * hell
        // todo - Result<T,E>
        match &self.font_index {
            VariableOrNot::Variable(var) => (var.get_font_by_weight(weight).clone(), true),
            VariableOrNot::Others(others) => (
                match crate::FONTS_UNIFY.search(others) {
                    Ok(x) => x,
                    Err(e) => {
                        log::error!("{e}");
                        BuiltinVariableFontIndex::Barlow
                            .get_font_by_weight(self.weight)
                            .clone()
                    }
                },
                false,
            ),
        }
    }

    pub fn text_dimensions(&self, scale: ab_glyph::PxScale, txt: impl AsRef<str>) -> (f32, f32) {
        crate::theme::text_dimensions(scale, &self.get_font(), txt.as_ref())
    }

    #[allow(unused)]
    pub fn format_dimensions(
        &self,
        scale: ab_glyph::PxScale,
        exif: &crate::image::exif_impl::SimplifiedExif,
        additional_text: impl AsRef<str> + std::fmt::Display,
    ) -> (f32, f32) {
        let txt = exif.format_custom(format!("{}{}", self.text, additional_text));

        self.text_dimensions(scale, txt)
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        ui: &mut Ui,
        label: Cow<'static, str>,
        default: &'static VariableTextSlotDefault,
    ) {
        // ensure outside is grid ui
        ui.vertical(|ui| {
            ui.add_space(2.0);
            ui.label(label.clone());
            ui.add_space(27.0);
        });
        // let total_width = ui.available_width();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add_space(2.0);

                // Use radio button style for variant selection
                let is_variable = matches!(self.font_index, VariableOrNot::Variable(_));
                if ui
                    .selectable_label(is_variable, t!("fonts_selector.variable.label"))
                    .on_hover_text(t!("fonts_selector.variable.hint"))
                    .clicked()
                    && !is_variable
                {
                    self.font_index = VariableOrNot::Variable(default.font_index);
                }

                let is_others = matches!(self.font_index, VariableOrNot::Others(_));
                if ui
                    .selectable_label(is_others, t!("fonts_selector.others.label"))
                    .on_hover_text(t!("fonts_selector.others.hint"))
                    .clicked()
                    && !is_others
                {
                    self.font_index = VariableOrNot::Others(
                        crate::FONTS_UNIFY.builtin_select(default.fixed_index.unwrap_or_default()),
                    );
                }

                if let VariableOrNot::Variable(ref mut variable_select) = self.font_index {
                    // variable_select.update_ui_with_label(ui, label.clone());
                    variable_select.update_ui(ui, label.clone());

                    let (start, end) = variable_select.get_font().range();
                    ui.add(egui::Slider::new(&mut self.weight, start..=end).step_by(100.0));
                } else if let VariableOrNot::Others(ref mut font_select) = self.font_index {
                    // font_select.update_ui_with_label(ctx, ui, label.clone());
                    font_select.update_ui(ctx, ui, label.clone());
                };

                if ui.button("↺").clicked() {
                    *self = default.into();
                }
            });

            ui.horizontal(|ui| {
                ui.add_sized(
                    [(ui.available_width() - 8.0).max(16.0), 23.0],
                    egui::TextEdit::singleline(&mut self.text).vertical_align(Align::Center),
                );

                // todo - font selection with variable_font_index
            });
            ui.add_space(4.0);
        });
    }
}

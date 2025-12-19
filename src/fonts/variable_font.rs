/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use ab_glyph::{Font, VariableFont};
use rust_i18n::t;

#[allow(dead_code)]
pub struct VariableFontPack {
    pub label: &'static str,
    pub font: ab_glyph::FontRef<'static>,

    // weight axis range
    pub default: u16,
    pub start: u16,
    pub end_include: u16,
}

#[allow(dead_code)]
impl VariableFontPack {
    pub const fn label(&self) -> &'static str {
        self.label
    }

    pub const fn get_default_weight(&self) -> u16 {
        self.default
    }

    /// (start_include: u16, end_include: u16)
    pub const fn range(&self) -> (u16, u16) {
        (self.start, self.end_include)
    }

    pub const fn default_weight(&self) -> u16 {
        self.default
    }

    pub fn get_font_by_weight(&self, weight: u16) -> ab_glyph::FontArc {
        let clamped_weight = weight.clamp(self.start, self.end_include);
        let mut font = self.font.clone();
        font.set_variation(b"wght", clamped_weight as f32);
        font.into()
    }

    pub fn get_near_weight(&self, weight: u16) -> u16 {
        // assume all fonts has weigt with 100 step
        weight.clamp(self.start, self.end_include)
    }
}

lazy_static::lazy_static! {
    static ref BARLOW: VariableFontPack = VariableFontPack {
        label: crate::fonts::FONT_BARLOW.name,
        font: ab_glyph::FontRef::try_from_slice(
             crate::fonts::FONT_BARLOW.data
        ).expect("Failed to load Barlow variable font"),
        default: 300,
        start: 100,
        end_include: 900,
    };

    static ref SOURCE_HAN_SANS: VariableFontPack = VariableFontPack {
        label: crate::fonts::FONT_SHSANS.name,
        font: ab_glyph::FontRef::try_from_slice(
        crate::fonts::FONT_SHSANS.data).expect("Failed to load Source Han Sans variable font"),
        default: 300,
        start: 200,
        end_include: 800,
    };

    pub static ref BUILTIN_VARIABLE_FONTS: [&'static VariableFontPack; 2] = [
        &*BARLOW,
        &*SOURCE_HAN_SANS,
    ];
}

#[rustfmt::skip]
#[repr(usize)]
#[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, Copy, PartialEq, Eq, strum::FromRepr)]
pub enum BuiltinVariableFontIndex {
    #[default]
    Barlow,
    SourceHanSans,
}

impl BuiltinVariableFontIndex {
    pub fn get_font(&self) -> &'static VariableFontPack {
        BUILTIN_VARIABLE_FONTS[*self as usize]
    }

    pub fn get_font_by_weight(&self, weight: u16) -> ab_glyph::FontArc {
        self.get_font().get_font_by_weight(weight)
    }

    pub fn update_ui<S: Into<egui::WidgetText>>(
        &mut self,
        // ctx: &egui::Context,
        ui: &mut egui::Ui,
        label: S,
    ) {
        egui::ComboBox::from_id_salt(label.into().text())
            .selected_text(self.get_font().label())
            .show_ui(ui, |ui| {
                ui.label(t!("fonts_selector.variable_fonts"));
                for (i, font) in BUILTIN_VARIABLE_FONTS.iter().enumerate() {
                    let selected = *self as usize == i;

                    if ui.selectable_label(selected, font.label()).clicked() {
                        if let Some(new_value) = Self::from_repr(i) {
                            *self = new_value;
                        } else {
                            log::error!("Failed to convert index {i} to BuiltinVariableFontIndex");
                        }
                    }
                }
            });
    }

    #[allow(dead_code)]
    pub fn update_ui_with_label<S: Into<egui::WidgetText> + Clone>(
        &mut self,
        ui: &mut egui::Ui,
        label: S,
    ) {
        ui.horizontal(|ui| {
            ui.label(label.clone());
            self.update_ui(ui, label);
        });
    }

    #[allow(dead_code)]
    pub fn update_ui_with_default_label(&mut self, ui: &mut egui::Ui) {
        let label = rust_i18n::t!("fonts_selector.select_a_font");
        ui.horizontal(|ui| {
            ui.label(label.clone());
            self.update_ui(ui, label);
        });
    }
}

/// Get appropriate font for the given character with fallback support.
/// Returns Barlow font by default, but falls back to SourceHanSans if the character is not supported.
#[allow(dead_code)]
pub fn get_font_with_fallback(ch: char, weight: u16) -> ab_glyph::FontArc {
    let barlow = BuiltinVariableFontIndex::Barlow.get_font_by_weight(weight);

    // Check if Barlow supports this character
    if barlow.glyph_id(ch) != ab_glyph::GlyphId(0) {
        barlow
    } else {
        // Fallback to Source Han Sans for CJK and other characters
        BuiltinVariableFontIndex::SourceHanSans.get_font_by_weight(weight)
    }
}

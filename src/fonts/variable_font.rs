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

// Variable fonts only loaded for desktop/web builds
// iOS loads fonts from app bundle via FFI, not from embedded data

// ===== EMBEDDED VERSION (no ext_res) =====
#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    not(feature = "ext_res")
))]
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

    static ref BARLOW_NARROW: VariableFontPack = VariableFontPack {
        label: crate::fonts::FONT_BARLOW_NARROW.name,
        font: ab_glyph::FontRef::try_from_slice(
             crate::fonts::FONT_BARLOW_NARROW.data
        ).expect("Failed to load Barlow narrow variable font"),
        default: 400, // too narrow, add 100 from 300
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

    static ref DYNAPUFF: VariableFontPack = VariableFontPack {
        label: crate::fonts::FONT_DYNAPUFF.name,
        font: ab_glyph::FontRef::try_from_slice(
             crate::fonts::FONT_DYNAPUFF.data
        ).expect("Failed to load DynaPuff variable font"),
        default: 400,
        start: 400,
        end_include: 700,
    };

    pub static ref BUILTIN_VARIABLE_FONTS: [&'static VariableFontPack; 4] = [
        &*BARLOW,
        &*BARLOW_NARROW,
        &*SOURCE_HAN_SANS,
        &*DYNAPUFF,
    ];
}

// ===== EXTERNAL RESOURCES VERSION (ext_res enabled) =====
// Fonts are loaded at runtime from Resources directory
// Uses leaked Box to create 'static lifetime for FontRef
#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    feature = "ext_res"
))]
lazy_static::lazy_static! {
    // Load and leak font data to get 'static lifetime
    static ref BARLOW_DATA: &'static [u8] = {
        let data = crate::resources::load_font(crate::fonts::FONT_BARLOW.filename)
            .expect("Failed to load Barlow font from Resources");
        Box::leak(data.into_boxed_slice())
    };

    static ref BARLOW_NARROW_DATA: &'static [u8] = {
        let data = crate::resources::load_font(crate::fonts::FONT_BARLOW_NARROW.filename)
            .expect("Failed to load Barlow Narrow font from Resources");
        Box::leak(data.into_boxed_slice())
    };

    static ref SOURCE_HAN_SANS_DATA: &'static [u8] = {
        let data = crate::resources::load_font(crate::fonts::FONT_SHSANS.filename)
            .expect("Failed to load Source Han Sans font from Resources");
        Box::leak(data.into_boxed_slice())
    };

    static ref BARLOW: VariableFontPack = VariableFontPack {
        label: crate::fonts::FONT_BARLOW.name,
        font: ab_glyph::FontRef::try_from_slice(&BARLOW_DATA)
            .expect("Failed to parse Barlow variable font"),
        default: 300,
        start: 100,
        end_include: 900,
    };

    static ref BARLOW_NARROW: VariableFontPack = VariableFontPack {
        label: crate::fonts::FONT_BARLOW_NARROW.name,
        font: ab_glyph::FontRef::try_from_slice(&BARLOW_NARROW_DATA)
            .expect("Failed to parse Barlow narrow variable font"),
        default: 400,
        start: 100,
        end_include: 900,
    };

    static ref SOURCE_HAN_SANS: VariableFontPack = VariableFontPack {
        label: crate::fonts::FONT_SHSANS.name,
        font: ab_glyph::FontRef::try_from_slice(&SOURCE_HAN_SANS_DATA)
            .expect("Failed to parse Source Han Sans variable font"),
        default: 300,
        start: 200,
        end_include: 800,
    };

    static ref DYNAPUFF_DATA: &'static [u8] = {
        let data = crate::resources::load_font(crate::fonts::FONT_DYNAPUFF.filename)
            .expect("Failed to load DynaPuff font from Resources");
        Box::leak(data.into_boxed_slice())
    };

    static ref DYNAPUFF: VariableFontPack = VariableFontPack {
        label: crate::fonts::FONT_DYNAPUFF.name,
        font: ab_glyph::FontRef::try_from_slice(&DYNAPUFF_DATA)
            .expect("Failed to parse DynaPuff variable font"),
        default: 400,
        start: 400,
        end_include: 700,
    };

    pub static ref BUILTIN_VARIABLE_FONTS: [&'static VariableFontPack; 4] = [
        &*BARLOW,
        &*BARLOW_NARROW,
        &*SOURCE_HAN_SANS,
        &*DYNAPUFF,
    ];
}

// For iOS, create empty placeholder
#[cfg(any(feature = "ios_integration", feature = "android_integration"))]
lazy_static::lazy_static! {
    pub static ref BUILTIN_VARIABLE_FONTS: [&'static VariableFontPack; 0] = [];
}

#[rustfmt::skip]
#[repr(usize)]
#[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, Copy, PartialEq, Eq, strum::FromRepr)]
pub enum BuiltinVariableFontIndex {
    #[default]
    Barlow,
    BarlowNarrow,
    SourceHanSans,
    DynaPuff,
}

impl BuiltinVariableFontIndex {
    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub fn get_font(&self) -> &'static VariableFontPack {
        BUILTIN_VARIABLE_FONTS[*self as usize]
    }

    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    pub fn get_font(&self) -> &'static VariableFontPack {
        panic!(
            "Builtin fonts not available on iOS - fonts must be loaded via FFI parameters. Theme should receive font paths through parameter JSON."
        )
    }

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub fn get_font_by_weight(&self, weight: u16) -> ab_glyph::FontArc {
        self.get_font().get_font_by_weight(weight)
    }

    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
    pub fn get_font_by_weight(&self, _weight: u16) -> ab_glyph::FontArc {
        panic!(
            "Builtin fonts not available on iOS - fonts must be loaded via FFI parameters. Theme should receive font paths through parameter JSON."
        )
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

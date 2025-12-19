/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Add or replace fonts from this code

use eframe::egui;

pub(crate) mod align;
pub(crate) mod builtin_fonts;
pub(crate) mod font_unify;
pub(crate) mod variable_font;

struct BuiltInFonts {
    pub(crate) name: &'static str,
    pub(crate) data: &'static [u8],
}

const FONT_D2CODING: BuiltInFonts = BuiltInFonts {
    name: "D2Coding-Nerd",
    data: include_bytes!("../../assets/fonts/D2Coding-Ver1.3.2-20180524-all.ttc"),
};

const FONT_SHSANS: BuiltInFonts = BuiltInFonts {
    name: "Source Han Sans",
    data: include_bytes!("../../assets/fonts/SourceHanSansVF-remapped.otf"),
};

const FONT_BARLOW: BuiltInFonts = BuiltInFonts {
    name: "Barlow",
    data: include_bytes!("../../assets/fonts/Barlow-Variable-Remapped.ttf"),
};

const FONT_DIGITAL_7: BuiltInFonts = BuiltInFonts {
    name: "Digital 7",
    data: include_bytes!(env!("DIGITAL_7_FONT_PATH")),
};
const FONT_DIGITAL_7_ITALIC: BuiltInFonts = BuiltInFonts {
    name: "Digital 7 Italic",
    data: include_bytes!(env!("DIGITAL_7_ITALIC_FONT_PATH")),
};

lazy_static::lazy_static! {
    pub static ref FONTS_UNIFY: crate::fonts::font_unify::FontsUnify = crate::fonts::font_unify::FontsUnify::new();

    pub static ref FONT_DIGITS: ab_glyph::FontArc = ab_glyph::FontArc::try_from_slice(include_bytes!(env!("DIGITAL_7_FONT_PATH"))).expect("Cannot init font.");
    pub static ref FONT_DIGITS_ITALIC: ab_glyph::FontArc = ab_glyph::FontArc::try_from_slice(include_bytes!(env!("DIGITAL_7_ITALIC_FONT_PATH"))).expect("Cannot init font.");
}

// Demonstrates how to replace all fonts.
pub(crate) fn replace_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        FONT_D2CODING.name.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(FONT_D2CODING.data)),
    );

    // Note: Variable font weight is controlled by the font's default weight
    fonts.font_data.insert(
        FONT_SHSANS.name.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(FONT_SHSANS.data)),
    );

    // proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, FONT_SHSANS.name.to_owned());

    // monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, FONT_D2CODING.name.to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(FONT_SHSANS.name.to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Add or replace fonts from this code

use eframe::egui;

struct BuiltInFonts {
    pub(crate) name: &'static str,
    pub(crate) data: &'static [u8],
}

const FONT_D2CODING: BuiltInFonts = BuiltInFonts {
    name: "D2Coding-Nerd",
    data: include_bytes!("../assets/fonts/D2Coding-Ver1.3.2-20180524-all.ttc"),
};

const FONT_NTSANS_MED: BuiltInFonts = BuiltInFonts {
    name: "NotoSans-Medium",
    data: include_bytes!("../assets/fonts/NotoSansKR-Medium.ttf"),
};

lazy_static::lazy_static! {
    pub static ref FONT_DIGITS: ab_glyph::FontArc = ab_glyph::FontArc::try_from_slice(include_bytes!(env!("DIGITAL_7_FONT_PATH"))).expect("Cannot init font.");
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

    fonts.font_data.insert(
        FONT_NTSANS_MED.name.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(FONT_NTSANS_MED.data)),
    );

    // proportional text:
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, FONT_NTSANS_MED.name.to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push(FONT_D2CODING.name.to_owned());

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
        .push(FONT_NTSANS_MED.name.to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

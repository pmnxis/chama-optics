/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Add or replace fonts from this code

// #[cfg(feature = "ios_integration")]
pub(crate) mod align;
pub(crate) mod builtin_fonts;
#[cfg(not(feature = "ios_integration"))]
pub(crate) mod font_unify;
pub(crate) mod variable_font;

#[cfg(not(feature = "ios_integration"))]
pub struct BuiltInFonts {
    pub name: &'static str,
    pub data: &'static [u8],
    // For iOS, data field is not included - fonts loaded from app bundle via FFI
}

// Font definitions - data loaded based on feature flags
// iOS loads fonts from app bundle via file paths passed through FFI
// ext_res: Load from Resources/Fonts/ directory (for egui fonts)
// Default (no ext_res): Embed in binary using include_bytes!() (for egui fonts)
//
// Note: These constants are used by font_unify.rs for the builtin font system
// They are always embedded for non-iOS builds to support ab_glyph font rendering

#[cfg(not(feature = "ios_integration"))]
const FONT_D2CODING: BuiltInFonts = BuiltInFonts {
    name: "D2Coding-Nerd",
    data: include_bytes!("../../assets/fonts/D2Coding-Ver1.3.2-20180524-all.ttc"),
};

#[cfg(not(feature = "ios_integration"))]
const FONT_SHSANS: BuiltInFonts = BuiltInFonts {
    name: "Source Han Sans",
    data: include_bytes!("../../assets/fonts/SourceHanSansVF-remapped.otf"),
};

#[cfg(not(feature = "ios_integration"))]
const FONT_BARLOW: BuiltInFonts = BuiltInFonts {
    name: "Barlow",
    data: include_bytes!("../../assets/fonts/Barlow-Variable-Remapped.ttf"),
};

#[cfg(not(feature = "ios_integration"))]
const FONT_BARLOW_NARROW: BuiltInFonts = BuiltInFonts {
    name: "Barlow Narrow",
    data: include_bytes!("../../assets/fonts/Barlow-Variable-Remapped-Narrow.ttf"),
};

#[cfg(not(feature = "ios_integration"))]
const FONT_DIGITAL_7: BuiltInFonts = BuiltInFonts {
    name: "Digital 7",
    data: include_bytes!(env!("DIGITAL_7_FONT_PATH")),
};

#[cfg(not(feature = "ios_integration"))]
const FONT_DIGITAL_7_ITALIC: BuiltInFonts = BuiltInFonts {
    name: "Digital 7 Italic",
    data: include_bytes!(env!("DIGITAL_7_ITALIC_FONT_PATH")),
};

#[cfg(not(feature = "ios_integration"))]
lazy_static::lazy_static! {
    pub static ref FONTS_UNIFY: crate::fonts::font_unify::FontsUnify = crate::fonts::font_unify::FontsUnify::new();

    // Digital 7 fonts not currently available
    // pub static ref FONT_DIGITS: ab_glyph::FontArc = ab_glyph::FontArc::try_from_slice(include_bytes!(env!("DIGITAL_7_FONT_PATH"))).expect("Cannot init font.");
    // pub static ref FONT_DIGITS_ITALIC: ab_glyph::FontArc = ab_glyph::FontArc::try_from_slice(include_bytes!(env!("DIGITAL_7_ITALIC_FONT_PATH"))).expect("Cannot init font.");
}

// Load and setup fonts - supports both external and embedded
// Not used for iOS - fonts are loaded from app bundle via FFI
#[cfg(not(feature = "ios_integration"))]
pub(crate) fn replace_fonts(ctx: &egui::Context) {
    use crate::resources;

    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Load fonts using the resources module
    // This will try external first, then embedded fallback

    // D2Coding (Korean monospace)
    if let Some(d2coding_data) = resources::load_font("D2Coding-Ver1.3.2-20180524-all.ttc") {
        fonts.font_data.insert(
            "D2Coding-Nerd".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(d2coding_data)),
        );

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "D2Coding-Nerd".to_owned());
    } else {
        log::warn!("Failed to load D2Coding font");
    }

    // Source Han Sans (CJK)
    if let Some(shsans_data) = resources::load_font("SourceHanSansVF-remapped.otf") {
        fonts.font_data.insert(
            "Source Han Sans".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(shsans_data)),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "Source Han Sans".to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("Source Han Sans".to_owned());
    } else {
        log::warn!("Failed to load Source Han Sans font");
    }

    // Barlow (variable font)
    if let Some(barlow_data) = resources::load_font("Barlow-Variable-Remapped.ttf") {
        fonts.font_data.insert(
            "Barlow".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(barlow_data)),
        );
    }

    // Barlow Narrow
    if let Some(barlow_narrow_data) = resources::load_font("Barlow-Variable-Remapped-Narrow.ttf") {
        fonts.font_data.insert(
            "Barlow Narrow".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(barlow_narrow_data)),
        );
    }

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);

    log::info!("Fonts loaded successfully");
}

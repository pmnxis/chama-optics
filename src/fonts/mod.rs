/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Add or replace fonts from this code

// #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
pub(crate) mod align;
pub(crate) mod builtin_fonts;
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub(crate) mod font_unify;
pub(crate) mod variable_font;

// Font definitions - data loaded based on feature flags
// iOS loads fonts from app bundle via file paths passed through FFI
// ext_res: Load from Resources/Fonts/ directory at runtime
// Default (no ext_res): Embed in binary using include_bytes!()
//
// Note: These constants are used by font_unify.rs for the builtin font system

// ===== EMBEDDED FONTS (when ext_res is NOT enabled) =====
#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    not(feature = "ext_res")
))]
pub struct BuiltInFonts {
    pub name: &'static str,
    pub data: &'static [u8],
}

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    not(feature = "ext_res")
))]
pub(crate) const FONT_D2CODING: BuiltInFonts = BuiltInFonts {
    name: "D2Coding-Nerd",
    data: include_bytes!("../../assets/fonts/D2Coding-Ver1.3.2-20180524-all.ttc"),
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    not(feature = "ext_res")
))]
pub(crate) const FONT_SHSANS: BuiltInFonts = BuiltInFonts {
    name: "Source Han Sans",
    data: include_bytes!("../../assets/fonts/SourceHanSansVF-remapped.otf"),
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    not(feature = "ext_res")
))]
pub(crate) const FONT_BARLOW: BuiltInFonts = BuiltInFonts {
    name: "Barlow",
    data: include_bytes!("../../assets/fonts/Barlow-Variable-Remapped.ttf"),
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    not(feature = "ext_res")
))]
pub(crate) const FONT_BARLOW_NARROW: BuiltInFonts = BuiltInFonts {
    name: "Barlow Narrow",
    data: include_bytes!("../../assets/fonts/Barlow-Variable-Remapped-Narrow.ttf"),
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    not(feature = "ext_res")
))]
pub(crate) const FONT_DIGITAL_7: BuiltInFonts = BuiltInFonts {
    name: "Digital 7",
    data: include_bytes!(env!("DIGITAL_7_FONT_PATH")),
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    not(feature = "ext_res")
))]
pub(crate) const FONT_DIGITAL_7_ITALIC: BuiltInFonts = BuiltInFonts {
    name: "Digital 7 Italic",
    data: include_bytes!(env!("DIGITAL_7_ITALIC_FONT_PATH")),
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    not(feature = "ext_res")
))]
pub(crate) const FONT_DYNAPUFF: BuiltInFonts = BuiltInFonts {
    name: "DynaPuff",
    data: include_bytes!(env!("DYNAPUFF_FONT_PATH")),
};

// ===== EXTERNAL FONTS (when ext_res IS enabled) =====
// Fonts are loaded at runtime from Resources/Fonts/ directory
#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    feature = "ext_res"
))]
pub struct BuiltInFontsExt {
    pub name: &'static str,
    pub filename: &'static str,
}

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    feature = "ext_res"
))]
pub(crate) const FONT_D2CODING: BuiltInFontsExt = BuiltInFontsExt {
    name: "D2Coding-Nerd",
    filename: "D2Coding-Ver1.3.2-20180524-all.ttc",
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    feature = "ext_res"
))]
pub(crate) const FONT_SHSANS: BuiltInFontsExt = BuiltInFontsExt {
    name: "Source Han Sans",
    filename: "SourceHanSansVF-remapped.otf",
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    feature = "ext_res"
))]
pub(crate) const FONT_BARLOW: BuiltInFontsExt = BuiltInFontsExt {
    name: "Barlow",
    filename: "Barlow-Variable-Remapped.ttf",
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    feature = "ext_res"
))]
pub(crate) const FONT_BARLOW_NARROW: BuiltInFontsExt = BuiltInFontsExt {
    name: "Barlow Narrow",
    filename: "Barlow-Variable-Remapped-Narrow.ttf",
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    feature = "ext_res"
))]
pub(crate) const FONT_DIGITAL_7: BuiltInFontsExt = BuiltInFontsExt {
    name: "Digital 7",
    filename: "digital-7.ttf",
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    feature = "ext_res"
))]
pub(crate) const FONT_DIGITAL_7_ITALIC: BuiltInFontsExt = BuiltInFontsExt {
    name: "Digital 7 Italic",
    filename: "digital-7-italic.ttf",
};

#[cfg(all(
    not(any(feature = "ios_integration", feature = "android_integration")),
    feature = "ext_res"
))]
pub(crate) const FONT_DYNAPUFF: BuiltInFontsExt = BuiltInFontsExt {
    name: "DynaPuff",
    filename: "DynaPuff-Variable.ttf",
};

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
lazy_static::lazy_static! {
    pub static ref FONTS_UNIFY: crate::fonts::font_unify::FontsUnify = crate::fonts::font_unify::FontsUnify::new();

    // Digital 7 fonts not currently available
    // pub static ref FONT_DIGITS: ab_glyph::FontArc = ab_glyph::FontArc::try_from_slice(include_bytes!(env!("DIGITAL_7_FONT_PATH"))).expect("Cannot init font.");
    // pub static ref FONT_DIGITS_ITALIC: ab_glyph::FontArc = ab_glyph::FontArc::try_from_slice(include_bytes!(env!("DIGITAL_7_ITALIC_FONT_PATH"))).expect("Cannot init font.");
}

// Load and setup fonts - supports both external and embedded
// Not used for iOS - fonts are loaded from app bundle via FFI
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
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

    // DynaPuff (playful variable font for date stamps)
    if let Some(dynapuff_data) = resources::load_font("DynaPuff-Variable.ttf") {
        fonts.font_data.insert(
            "DynaPuff".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(dynapuff_data)),
        );
    }

    // Register named font families for explicit selection (e.g. cheki date stamp)
    for name in ["Barlow", "Barlow Narrow", "DynaPuff", "Source Han Sans"] {
        if fonts.font_data.contains_key(name) {
            fonts
                .families
                .entry(egui::FontFamily::Name(name.into()))
                .or_default()
                .push(name.to_owned());
        }
    }

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);

    log::info!("Fonts loaded successfully");
}

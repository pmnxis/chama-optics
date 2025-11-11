/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[allow(dead_code)]
pub struct VariableFontPack {
    pub label: &'static str,
    pub font: &'static [ab_glyph::FontArc],

    // weight
    pub default: u16,
    pub start: u16,
    pub end_include: u16,
    pub step: u16,
}

#[allow(dead_code)]
impl VariableFontPack {
    pub const fn label(&self) -> &'static str {
        self.label
    }

    pub const fn get_default_weight(&self) -> u16 {
        self.default
    }
    // fn range() -> std::range::legacy::RangeInclusive<Num>;
    /// (start_include: u16, end_include: u16, step: u16)
    /// (... start_include..=end_include).step_by(step as f32)
    pub const fn range(&self) -> (u16, u16, u16) {
        (self.start, self.end_include, self.step)
    }

    pub const fn default_weight(&self) -> u16 {
        self.default
    }

    fn weight_to_index(&self, weight: u16) -> usize {
        ((weight / self.step).saturating_sub(self.start / self.step))
            .clamp(0, self.font.len() as u16) as usize
    }

    pub fn get_font_by_weight(&'static self, weight: u16) -> &'static ab_glyph::FontArc {
        &self.font[self.weight_to_index(weight)]
    }
}

lazy_static::lazy_static! {
    static ref FONT_PACK_BARLOW: [ab_glyph::FontArc; 9] = [
        ab_glyph::FontArc::try_from_slice(include_bytes!(env!("BARLOW_100_FONT_PATH"))).unwrap(),
        ab_glyph::FontArc::try_from_slice(include_bytes!(env!("BARLOW_200_FONT_PATH"))).unwrap(),
        ab_glyph::FontArc::try_from_slice(include_bytes!(env!("BARLOW_300_FONT_PATH"))).unwrap(),
        ab_glyph::FontArc::try_from_slice(include_bytes!(env!("BARLOW_400_FONT_PATH"))).unwrap(),
        ab_glyph::FontArc::try_from_slice(include_bytes!(env!("BARLOW_500_FONT_PATH"))).unwrap(),
        ab_glyph::FontArc::try_from_slice(include_bytes!(env!("BARLOW_600_FONT_PATH"))).unwrap(),
        ab_glyph::FontArc::try_from_slice(include_bytes!(env!("BARLOW_700_FONT_PATH"))).unwrap(),
        ab_glyph::FontArc::try_from_slice(include_bytes!(env!("BARLOW_800_FONT_PATH"))).unwrap(),
        ab_glyph::FontArc::try_from_slice(include_bytes!(env!("BARLOW_900_FONT_PATH"))).unwrap(),
    ];

    static ref BARLOW : VariableFontPack = VariableFontPack {
        label: "Barlow",
        font: &*FONT_PACK_BARLOW,
        default: 300,
        start: 100,
        end_include: 900,
        step: 100
    };

    pub static ref BUILTIN_VARIABLE_FONTS : [&'static VariableFontPack; 1] = [
        &*BARLOW,
    ];
}

#[rustfmt::skip]
#[repr(usize)]
#[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinVariableFontIndex {
    #[default]
    Barlow,
}

impl BuiltinVariableFontIndex {
    pub fn get_font(&self) -> &'static VariableFontPack {
        BUILTIN_VARIABLE_FONTS[*self as usize]
    }

    pub fn get_font_by_weight(&self, weight: u16) -> &'static ab_glyph::FontArc {
        self.get_font().get_font_by_weight(weight)
    }
}

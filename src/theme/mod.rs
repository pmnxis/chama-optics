/*

* SPDX-FileCopyrightText: © 2025 Jinwoo Park ([pmnxis@gmail.com](mailto:pmnxis@gmail.com))
*
* SPDX-License-Identifier: LicenseRef-Non-AI-MIT
  */

//! collection of themes

pub(crate) mod film;
pub(crate) mod film_date;
pub(crate) mod film_glow;
pub(crate) mod just_frame;
pub(crate) mod lightroom;
pub(crate) mod monitor;
pub(crate) mod nothing;
pub(crate) mod one_line;
pub(crate) mod shot_on_one_line;
pub(crate) mod shot_on_two_line;
pub(crate) mod strap;
pub(crate) mod two_line;

use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

pub fn color32_to_rgba(color: egui::Color32) -> image::Rgba<u8> {
    let [r, g, b, a] = color.to_array();
    image::Rgba([r, g, b, a])
}

pub(crate) fn text_dimensions(
    scale: ab_glyph::PxScale,
    font: &impl ab_glyph::Font,
    text: &str,
) -> (f32, f32) {
    use ab_glyph::ScaleFont;
    let scaled = font.as_scaled(scale);
    (
        text.chars()
            .map(|c| scaled.h_advance(font.glyph_id(c)))
            .sum::<f32>(),
        scaled.height(),
    )
}

/// Calculate text dimensions with automatic fallback to SourceHanSans for unsupported characters
/// This function calculates width character by character, using the appropriate font for each character.
pub(crate) fn text_dimensions_with_fallback(
    scale: ab_glyph::PxScale,
    primary_font: &ab_glyph::FontArc,
    weight: u16,
    text: &str,
) -> (f32, f32) {
    use crate::fonts::variable_font::{BUILTIN_VARIABLE_FONTS, BuiltinVariableFontIndex};
    use ab_glyph::{Font, ScaleFont};

    let fallback_weight = BUILTIN_VARIABLE_FONTS[BuiltinVariableFontIndex::SourceHanSans as usize]
        .get_near_weight(weight);
    let fallback_font = BuiltinVariableFontIndex::SourceHanSans.get_font_by_weight(fallback_weight);

    let scaled_primary = primary_font.as_scaled(scale);
    let max_height = scaled_primary.height();

    let total_width: f32 = text
        .chars()
        .map(|ch| {
            // Check if primary font supports this character
            let glyph_id = primary_font.glyph_id(ch);
            if glyph_id != ab_glyph::GlyphId(0) {
                scaled_primary.h_advance(glyph_id)
            } else {
                // Fallback to SourceHanSans
                let scaled_fallback = fallback_font.as_scaled(scale);
                scaled_fallback.h_advance(fallback_font.glyph_id(ch))
            }
        })
        .sum();

    (total_width, max_height)
}

/// Draw text with automatic fallback to SourceHanSans for unsupported characters
/// This function draws text character by character, using the primary font when possible
/// and falling back to SourceHanSans for CJK and other unsupported characters.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_text_with_fallback<I>(
    image: &mut I,
    color: image::Rgba<u8>,
    x: i32,
    y: i32,
    scale: ab_glyph::PxScale,
    primary_font: &ab_glyph::FontArc,
    weight: u16,
    text: &str,
) where
    I: image::GenericImage<Pixel = image::Rgba<u8>>,
{
    use crate::fonts::variable_font::{BUILTIN_VARIABLE_FONTS, BuiltinVariableFontIndex};
    use ab_glyph::{Font, ScaleFont};

    let fallback_weight = BUILTIN_VARIABLE_FONTS[BuiltinVariableFontIndex::SourceHanSans as usize]
        .get_near_weight(weight);
    let fallback_font = BuiltinVariableFontIndex::SourceHanSans.get_font_by_weight(fallback_weight);

    let mut current_x = x as f32;
    let scaled_primary = primary_font.as_scaled(scale);

    for ch in text.chars() {
        // Check if primary font supports this character
        let glyph_id = primary_font.glyph_id(ch);
        let (font_to_use, scaled_font): (&ab_glyph::FontArc, _) =
            if glyph_id != ab_glyph::GlyphId(0) {
                (primary_font, scaled_primary)
            } else {
                // Fallback to SourceHanSans
                // log::debug!("Falling back to SourceHanSans for character: {ch}");
                (&fallback_font, fallback_font.as_scaled(scale))
            };

        // Draw single character
        imageproc::drawing::draw_text_mut(
            image,
            color,
            current_x as i32,
            y,
            scale,
            font_to_use,
            &ch.to_string(),
        );

        // Advance x position
        current_x += scaled_font.h_advance(font_to_use.glyph_id(ch));
    }
}

/// Draw text with automatic fallback to SourceHanSans for unsupported characters (Luma version)
/// This function is for grayscale images used in glow effects.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_text_with_fallback_luma<I>(
    image: &mut I,
    color: image::Luma<u8>,
    x: i32,
    y: i32,
    scale: ab_glyph::PxScale,
    primary_font: &ab_glyph::FontArc,
    weight: u16,
    text: &str,
) where
    I: image::GenericImage<Pixel = image::Luma<u8>>,
{
    use crate::fonts::variable_font::{BUILTIN_VARIABLE_FONTS, BuiltinVariableFontIndex};
    use ab_glyph::{Font, ScaleFont};

    let fallback_weight = BUILTIN_VARIABLE_FONTS[BuiltinVariableFontIndex::SourceHanSans as usize]
        .get_near_weight(weight);
    let fallback_font = BuiltinVariableFontIndex::SourceHanSans.get_font_by_weight(fallback_weight);

    let mut current_x = x as f32;
    let scaled_primary = primary_font.as_scaled(scale);

    for ch in text.chars() {
        // Check if primary font supports this character
        let glyph_id = primary_font.glyph_id(ch);
        let (font_to_use, scaled_font): (&ab_glyph::FontArc, _) =
            if glyph_id != ab_glyph::GlyphId(0) {
                (primary_font, scaled_primary)
            } else {
                // Fallback to SourceHanSans
                // log::debug!("Falling back to SourceHanSans for character: {ch}");
                (&fallback_font, fallback_font.as_scaled(scale))
            };

        // Draw single character
        imageproc::drawing::draw_text_mut(
            image,
            color,
            current_x as i32,
            y,
            scale,
            font_to_use,
            &ch.to_string(),
        );

        // Advance x position
        current_x += scaled_font.h_advance(font_to_use.glyph_id(ch));
    }
}

pub trait Theme {
    /// return unique name of theme
    fn unique_name(&self) -> &'static str;

    /// return label to show on UI
    fn label(&self) -> std::borrow::Cow<'static, str>;

    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError>;

    fn ui_config(&mut self, ui: &mut egui::Ui);

    fn is_ui_config_available(&self) -> bool;

    // todo - the trait reset some value when UI is selected
}
/// Serializable state used for saving/loading preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeRegistryState {
    pub names: Vec<String>,
    pub selected: usize,
}

/// Runtime registry that holds real Theme trait objects.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeRegistry {
    #[serde(skip)]
    pub themes: Vec<Arc<RwLock<dyn Theme>>>,
    pub selected: usize,
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeRegistry {
    pub fn default_vector() -> Vec<Arc<RwLock<dyn Theme>>> {
        vec![
            Arc::new(RwLock::new(film::Film::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(film_glow::FilmGlow::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(film_date::FilmDate::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(nothing::Nothing::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(just_frame::JustFrame::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(one_line::OneLine::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(two_line::TwoLine::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(shot_on_one_line::ShotOnOneLine::default()))
                as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(shot_on_two_line::ShotOnTwoLine::default()))
                as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(strap::Strap::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(monitor::Monitor::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(lightroom::Lightroom::default())) as Arc<RwLock<dyn Theme>>,
        ]
    }

    pub fn new() -> Self {
        Self {
            themes: Self::default_vector(),
            selected: 0,
        }
    }

    pub fn find(&self, unique: &str) -> Option<std::sync::RwLockReadGuard<'_, dyn Theme>> {
        self.themes
            .iter()
            .position(|t| {
                t.read()
                    .map(|tt| tt.unique_name() == unique)
                    .unwrap_or(false)
            })
            .map(|idx| self.themes[idx].read().unwrap())
    }

    pub fn from_state(state: ThemeRegistryState) -> Self {
        let available: Vec<Arc<RwLock<dyn Theme>>> = Self::default_vector();

        let mut ordered = Vec::new();
        let mut remaining = available.clone();

        for saved_name in &state.names {
            if let Some(pos) = remaining.iter().position(|t: &Arc<RwLock<dyn Theme>>| {
                t.read().unwrap().unique_name() == saved_name
            }) {
                ordered.push(remaining.remove(pos));
            }
        }

        ordered.extend(remaining);

        let selected = state.selected.min(ordered.len().saturating_sub(1));
        Self {
            themes: ordered,
            selected,
        }
    }

    pub fn to_state(&self) -> ThemeRegistryState {
        ThemeRegistryState {
            names: self
                .themes
                .iter()
                .map(|t| t.read().unwrap().unique_name().to_string())
                .collect(),
            selected: self.selected.min(self.themes.len().saturating_sub(1)),
        }
    }

    pub fn selected_theme_read(&self) -> std::sync::RwLockReadGuard<'_, dyn Theme> {
        self.themes[self.selected].read().unwrap()
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(t!("theme.selector"));
            egui::ComboBox::from_id_salt("theme_selector")
                .selected_text(self.themes[self.selected].read().unwrap().label())
                .show_ui(ui, |ui| {
                    for (i, theme) in self.themes.iter().enumerate() {
                        if ui
                            .selectable_label(i == self.selected, theme.read().unwrap().label())
                            .clicked()
                        {
                            self.selected = i;
                        }
                    }
                });

            let mut theme = self.themes[self.selected].write().unwrap();
            if theme.is_ui_config_available() {
                ui.collapsing(t!("theme.settings"), |ui| {
                    theme.ui_config(ui);
                });
            }
        });
    }

    #[cfg(test)]
    pub fn insert_or_replace_theme<T: Theme + 'static>(&mut self, theme: T) {
        let unique = theme.unique_name();
        let arc_theme = Arc::new(RwLock::new(theme));

        if let Some(idx) = self.themes.iter().position(|t| {
            t.read()
                .map(|tt| tt.unique_name() == unique)
                .unwrap_or(false)
        }) {
            self.themes[idx] = arc_theme;
            self.selected = idx;
        } else {
            self.themes.push(arc_theme);
            self.selected = self.themes.len() - 1;
        }
    }
}

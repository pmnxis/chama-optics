/*

* SPDX-FileCopyrightText: © 2025 Jinwoo Park ([pmnxis@gmail.com](mailto:pmnxis@gmail.com))
*
* SPDX-License-Identifier: LicenseRef-Non-AI-MIT
  */

//! collection of themes

pub mod parameter_schema;

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
#[cfg(not(feature = "ios_integration"))]
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

/// Calculate text dimensions with automatic fallback (iOS version - no fallback)
/// iOS version uses only the primary font. Users select appropriate fonts including CJK support.
#[cfg(feature = "ios_integration")]
pub(crate) fn text_dimensions_with_fallback(
    scale: ab_glyph::PxScale,
    primary_font: &ab_glyph::FontArc,
    _weight: u16,
    text: &str,
) -> (f32, f32) {
    // iOS: no fallback, just use primary font
    text_dimensions(scale, primary_font, text)
}

/// Draw text with automatic fallback to SourceHanSans for unsupported characters
/// This function draws text character by character, using the primary font when possible
/// and falling back to SourceHanSans for CJK and other unsupported characters.
#[cfg(not(feature = "ios_integration"))]
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

/// Draw text with automatic fallback (iOS version - no fallback)
/// iOS version uses only the primary font directly via imageproc.
#[cfg(feature = "ios_integration")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_text_with_fallback<I>(
    image: &mut I,
    color: image::Rgba<u8>,
    x: i32,
    y: i32,
    scale: ab_glyph::PxScale,
    primary_font: &ab_glyph::FontArc,
    _weight: u16,
    text: &str,
) where
    I: image::GenericImage<Pixel = image::Rgba<u8>>,
{
    // iOS: no fallback, just use primary font directly
    log::debug!(
        "Drawing text at ({}, {}) with scale {:?}, color {:?}: '{}'",
        x,
        y,
        scale,
        color,
        text
    );
    if text.is_empty() {
        log::warn!("Attempting to draw empty text at ({}, {})", x, y);
    }
    imageproc::drawing::draw_text_mut(image, color, x, y, scale, primary_font, text);
}

/// Draw text with automatic fallback to SourceHanSans for unsupported characters (Luma version)
/// This function is for grayscale images used in glow effects.
#[cfg(not(feature = "ios_integration"))]
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

/// Draw text with automatic fallback (iOS version - no fallback, Luma)
/// iOS version uses only the primary font directly via imageproc for grayscale images.
#[cfg(feature = "ios_integration")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_text_with_fallback_luma<I>(
    image: &mut I,
    color: image::Luma<u8>,
    x: i32,
    y: i32,
    scale: ab_glyph::PxScale,
    primary_font: &ab_glyph::FontArc,
    _weight: u16,
    text: &str,
) where
    I: image::GenericImage<Pixel = image::Luma<u8>>,
{
    // iOS: no fallback, just use primary font directly
    imageproc::drawing::draw_text_mut(image, color, x, y, scale, primary_font, text);
}

pub trait Theme: Send + Sync + std::any::Any {
    /// return unique name of theme
    fn unique_name(&self) -> &'static str;

    /// return label to show on UI
    fn label(&self) -> std::borrow::Cow<'static, str>;

    /// Apply theme and return the resulting DynamicImage (for preview)
    fn apply_to_image(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
    ) -> Result<image::DynamicImage, image::ImageError>;

    /// Apply theme and save to file
    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError> {
        self.apply_with_faces(pi, export_config, output_path, None)
    }

    /// Apply theme with pre-detected faces and save to file
    fn apply_with_faces(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
        pre_detected_faces: Option<Vec<(i32, i32, u32, u32)>>,
    ) -> Result<(), image::ImageError> {
        // Get original image dimensions before theming
        let orig_img = image::open(&pi.path)?;
        let orig_width = orig_img.width();
        let orig_height = orig_img.height();

        // Apply theme (may resize the image)
        let mut dyn_image = self.apply_to_image(pi, export_config)?;

        // Scale face coordinates if the image was resized
        let scaled_faces = if let Some(faces) = pre_detected_faces {
            let new_width = dyn_image.width();
            let new_height = dyn_image.height();

            if new_width != orig_width || new_height != orig_height {
                let scale_x = new_width as f32 / orig_width as f32;
                let scale_y = new_height as f32 / orig_height as f32;

                log::info!(
                    "Scaling face coordinates from {}×{} to {}×{} (scale: {:.3}×{:.3})",
                    orig_width,
                    orig_height,
                    new_width,
                    new_height,
                    scale_x,
                    scale_y
                );

                Some(
                    faces
                        .into_iter()
                        .map(|(x, y, w, h)| {
                            let scaled_x = (x as f32 * scale_x) as i32;
                            let scaled_y = (y as f32 * scale_y) as i32;
                            let scaled_w = (w as f32 * scale_x) as u32;
                            let scaled_h = (h as f32 * scale_y) as u32;
                            (scaled_x, scaled_y, scaled_w, scaled_h)
                        })
                        .collect(),
                )
            } else {
                Some(faces)
            }
        } else {
            None
        };

        // Get margin from the theme (if applicable)
        // For now, use None as default - themes can override this method if needed
        export_config.save_image_with_faces(&mut dyn_image, None, output_path, scaled_faces)
    }

    #[cfg(not(feature = "ios_integration"))]
    fn ui_config(&mut self, ui: &mut egui::Ui);

    fn is_ui_config_available(&self) -> bool;

    /// Get parameters as JSON string for FFI
    /// Returns JSON describing available parameters and their current values
    /// Default implementation returns empty parameters
    fn get_parameters_json(&self) -> String {
        r#"{"parameters": []}"#.to_string()
    }

    // todo - the trait reset some value when UI is selected
}
/// Serializable state used for saving/loading preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeRegistryState {
    pub names: Vec<String>,
    pub selected: usize,
}

/// Runtime registry that holds real Theme trait objects.
#[derive(Clone, Serialize, Deserialize)]
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

    #[cfg(not(feature = "ios_integration"))]
    pub fn update_ui(&mut self, ui: &mut egui::Ui, show_english_name: bool) {
        ui.vertical(|ui| {
            ui.label(rust_i18n::t!("theme.selector"));

            let selected_text = if show_english_name {
                // Temporarily switch to English locale to get English label
                let current_locale = rust_i18n::locale();
                rust_i18n::set_locale("en");
                let label = self.themes[self.selected].read().unwrap().label();
                rust_i18n::set_locale(&current_locale);
                label
            } else {
                self.themes[self.selected].read().unwrap().label()
            };

            egui::ComboBox::from_id_salt("theme_selector")
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    for (i, theme) in self.themes.iter().enumerate() {
                        let theme_guard = theme.read().unwrap();
                        let display_name = if show_english_name {
                            // Temporarily switch to English locale to get English label
                            let current_locale = rust_i18n::locale();
                            rust_i18n::set_locale("en");
                            let label = theme_guard.label();
                            rust_i18n::set_locale(&current_locale);
                            label
                        } else {
                            theme_guard.label()
                        };

                        if ui
                            .selectable_label(i == self.selected, display_name)
                            .clicked()
                        {
                            self.selected = i;
                        }
                    }
                });

            let mut theme = self.themes[self.selected].write().unwrap();
            if theme.is_ui_config_available() {
                ui.collapsing(rust_i18n::t!("theme.settings"), |ui| {
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

/// Create a theme by name (for FFI)
///
/// This function is used by FFI layer to create theme instances.
pub fn create_theme(name: &str) -> Option<Box<dyn Theme>> {
    match name {
        "just_frame" => Some(Box::new(just_frame::JustFrame::default())),
        "one_line" => Some(Box::new(one_line::OneLine::default())),
        "two_line" => Some(Box::new(two_line::TwoLine::default())),
        "shot_on_one_line" => Some(Box::new(shot_on_one_line::ShotOnOneLine::default())),
        "shot_on_two_line" => Some(Box::new(shot_on_two_line::ShotOnTwoLine::default())),
        "strap" => Some(Box::new(strap::Strap::default())),
        "monitor" => Some(Box::new(monitor::Monitor::default())),
        "lightroom" => Some(Box::new(lightroom::Lightroom::default())),
        "film" => Some(Box::new(film::Film::default())),
        "film_date" => Some(Box::new(film_date::FilmDate::default())),
        "film_glow" => Some(Box::new(film_glow::FilmGlow::default())),
        _ => None,
    }
}

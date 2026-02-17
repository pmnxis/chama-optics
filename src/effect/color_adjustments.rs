// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Color adjustments module - Lightroom-style color adjustments
//!
//! This module provides Lightroom-style color adjustment features:
//! - Exposure, Contrast, Highlights, Shadows, Whites, Blacks
//! - Clarity, Vibrance, Saturation
//!
//! Performance optimizations:
//! - Direct byte slice access (no get_pixel/put_pixel overhead)
//! - Pre-computed constants outside loops
//! - Parallel processing with rayon for large images
//! - Inline functions for critical paths

#[cfg(feature = "threading")]
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "desktop", feature = "web"))]
use rust_i18n::t;

/// Color adjustment parameters (Lightroom-style)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorAdjustments {
    /// Master enable/disable for all adjustments
    pub enabled: bool,

    // Basic adjustments
    /// Exposure adjustment in EV (-5.0 to +5.0)
    pub exposure: f32,
    /// Contrast adjustment (-100 to +100)
    pub contrast: i32,
    /// Highlights adjustment (-100 to +100)
    pub highlights: i32,
    /// Shadows adjustment (-100 to +100)
    pub shadows: i32,
    /// Whites adjustment (-100 to +100)
    pub whites: i32,
    /// Blacks adjustment (-100 to +100)
    pub blacks: i32,

    // Presence adjustments
    /// Clarity adjustment (-100 to +100)
    pub clarity: i32,
    /// Vibrance adjustment (-100 to +100)
    pub vibrance: i32,
    /// Saturation adjustment (-100 to +100)
    pub saturation: i32,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            enabled: false,
            exposure: 0.0,
            contrast: 0,
            highlights: 0,
            shadows: 0,
            whites: 0,
            blacks: 0,
            clarity: 0,
            vibrance: 0,
            saturation: 0,
        }
    }
}

/// Smooth interpolation function (Hermite interpolation)
/// Returns 0.0 when x <= edge0, 1.0 when x >= edge1, smooth transition between
#[inline(always)]
fn smooth_step(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Pre-computed adjustment parameters for fast pixel processing
#[derive(Clone, Copy)]
struct AdjustmentParams {
    inv_255: f32,
    exposure_mult: f32,
    contrast_factor: f32,
    highlights_factor: f32,
    shadows_factor: f32,
    whites_factor: f32,
    blacks_factor: f32,
    saturation_factor: f32,
    vibrance_factor: f32,
    clarity_factor: f32,
    // Flags to skip unused adjustments
    apply_exposure: bool,
    apply_contrast: bool,
    apply_highlights: bool,
    apply_shadows: bool,
    apply_whites: bool,
    apply_blacks: bool,
    apply_saturation: bool,
    apply_vibrance: bool,
    apply_clarity: bool,
}

impl AdjustmentParams {
    #[inline]
    fn from_adjustments(adj: &ColorAdjustments) -> Self {
        Self {
            inv_255: 1.0 / 255.0,
            exposure_mult: 2.0_f32.powf(adj.exposure),
            contrast_factor: 1.0 + (adj.contrast as f32 / 100.0),
            highlights_factor: adj.highlights as f32 / 100.0,
            shadows_factor: adj.shadows as f32 / 100.0,
            whites_factor: adj.whites as f32 / 100.0 * 0.5,
            blacks_factor: adj.blacks as f32 / 100.0 * 0.5,
            saturation_factor: 1.0 + (adj.saturation as f32 / 100.0),
            vibrance_factor: adj.vibrance as f32 / 100.0,
            clarity_factor: 1.0 + (adj.clarity as f32 / 200.0),
            apply_exposure: adj.exposure != 0.0,
            apply_contrast: adj.contrast != 0,
            apply_highlights: adj.highlights != 0,
            apply_shadows: adj.shadows != 0,
            apply_whites: adj.whites != 0,
            apply_blacks: adj.blacks != 0,
            apply_saturation: adj.saturation != 0,
            apply_vibrance: adj.vibrance != 0,
            apply_clarity: adj.clarity != 0,
        }
    }
}

/// Process a single pixel with all adjustments (inline for performance)
#[inline(always)]
fn process_pixel(r: u8, g: u8, b: u8, params: &AdjustmentParams) -> (u8, u8, u8) {
    // Convert to f32 normalized
    let mut rf = r as f32 * params.inv_255;
    let mut gf = g as f32 * params.inv_255;
    let mut bf = b as f32 * params.inv_255;

    // 1. Exposure
    if params.apply_exposure {
        rf *= params.exposure_mult;
        gf *= params.exposure_mult;
        bf *= params.exposure_mult;
    }

    // 2. Contrast (S-curve centered at 0.5)
    if params.apply_contrast {
        rf = ((rf - 0.5) * params.contrast_factor + 0.5).clamp(0.0, 1.0);
        gf = ((gf - 0.5) * params.contrast_factor + 0.5).clamp(0.0, 1.0);
        bf = ((bf - 0.5) * params.contrast_factor + 0.5).clamp(0.0, 1.0);
    }

    // Calculate luminance once for tone-based adjustments
    let luma = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;

    // 3. Highlights (bright areas)
    if params.apply_highlights {
        let weight = smooth_step(0.5, 1.0, luma);
        let adj = params.highlights_factor * weight;
        rf = (rf + adj * rf).clamp(0.0, 1.0);
        gf = (gf + adj * gf).clamp(0.0, 1.0);
        bf = (bf + adj * bf).clamp(0.0, 1.0);
    }

    // 4. Shadows (dark areas)
    if params.apply_shadows {
        let weight = 1.0 - smooth_step(0.0, 0.5, luma);
        let adj = params.shadows_factor * weight;
        rf = (rf + adj * (1.0 - rf)).clamp(0.0, 1.0);
        gf = (gf + adj * (1.0 - gf)).clamp(0.0, 1.0);
        bf = (bf + adj * (1.0 - bf)).clamp(0.0, 1.0);
    }

    // 5. Whites (brightest areas)
    if params.apply_whites {
        let weight = smooth_step(0.75, 1.0, luma);
        let adj = params.whites_factor * weight;
        rf = (rf + adj).clamp(0.0, 1.0);
        gf = (gf + adj).clamp(0.0, 1.0);
        bf = (bf + adj).clamp(0.0, 1.0);
    }

    // 6. Blacks (darkest areas)
    if params.apply_blacks {
        let weight = 1.0 - smooth_step(0.0, 0.25, luma);
        let adj = params.blacks_factor * weight;
        rf = (rf - adj).clamp(0.0, 1.0);
        gf = (gf - adj).clamp(0.0, 1.0);
        bf = (bf - adj).clamp(0.0, 1.0);
    }

    // 7. Saturation
    if params.apply_saturation {
        let gray = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
        rf = (gray + (rf - gray) * params.saturation_factor).clamp(0.0, 1.0);
        gf = (gray + (gf - gray) * params.saturation_factor).clamp(0.0, 1.0);
        bf = (gray + (bf - gray) * params.saturation_factor).clamp(0.0, 1.0);
    }

    // 8. Vibrance (selective saturation)
    if params.apply_vibrance {
        let gray = 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
        let current_sat = if gray > 0.001 {
            let max_c = rf.max(gf).max(bf);
            let min_c = rf.min(gf).min(bf);
            (max_c - min_c) / max_c.max(0.001)
        } else {
            0.0
        };
        let vib_factor = 1.0 + params.vibrance_factor * (1.0 - current_sat);
        rf = (gray + (rf - gray) * vib_factor).clamp(0.0, 1.0);
        gf = (gray + (gf - gray) * vib_factor).clamp(0.0, 1.0);
        bf = (gray + (bf - gray) * vib_factor).clamp(0.0, 1.0);
    }

    // 9. Clarity (mid-tone contrast)
    if params.apply_clarity {
        let mid_weight = 1.0 - (2.0 * luma - 1.0).abs();
        let adj_factor = 1.0 + (params.clarity_factor - 1.0) * mid_weight;
        rf = ((rf - 0.5) * adj_factor + 0.5).clamp(0.0, 1.0);
        gf = ((gf - 0.5) * adj_factor + 0.5).clamp(0.0, 1.0);
        bf = ((bf - 0.5) * adj_factor + 0.5).clamp(0.0, 1.0);
    }

    // Convert back to u8
    (
        (rf * 255.0 + 0.5) as u8,
        (gf * 255.0 + 0.5) as u8,
        (bf * 255.0 + 0.5) as u8,
    )
}

impl ColorAdjustments {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if all adjustments are at neutral (identity) values
    pub fn is_identity(&self) -> bool {
        !self.enabled
            || (self.exposure == 0.0
                && self.contrast == 0
                && self.highlights == 0
                && self.shadows == 0
                && self.whites == 0
                && self.blacks == 0
                && self.clarity == 0
                && self.vibrance == 0
                && self.saturation == 0)
    }

    /// Reset all adjustments to neutral values
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Apply color adjustments to an image
    ///
    /// Applies Lightroom-style adjustments: exposure, contrast, highlights,
    /// shadows, whites, blacks, clarity, vibrance, and saturation.
    ///
    /// Performance optimizations:
    /// - Pre-computed adjustment parameters
    /// - Direct byte slice access (no get_pixel/put_pixel overhead)
    /// - Parallel processing with rayon for images > 100K pixels
    pub fn apply(&self, image: &mut image::DynamicImage) {
        if !self.enabled || self.is_identity() {
            return;
        }

        let mut rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        let total_pixels = (width as usize) * (height as usize);

        // Pre-compute all adjustment parameters once
        let params = AdjustmentParams::from_adjustments(self);

        // Get mutable access to raw pixel data
        let pixels: &mut [u8] = rgba.as_mut();

        // Use parallel processing for large images (> 100K pixels)
        const PARALLEL_THRESHOLD: usize = 100_000;

        #[cfg(feature = "threading")]
        if total_pixels >= PARALLEL_THRESHOLD {
            // Process in chunks of 4 bytes (RGBA) using rayon
            pixels.par_chunks_exact_mut(4).for_each(|chunk| {
                let (r, g, b) = process_pixel(chunk[0], chunk[1], chunk[2], &params);
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
                // chunk[3] (alpha) is preserved
            });
        } else {
            // Sequential processing for small images (less overhead)
            let mut idx = 0;
            for _ in 0..total_pixels {
                let (r, g, b) =
                    process_pixel(pixels[idx], pixels[idx + 1], pixels[idx + 2], &params);
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                // pixels[idx + 3] (alpha) is preserved
                idx += 4;
            }
        }

        // Sequential fallback when threading is not available (WASM)
        #[cfg(not(feature = "threading"))]
        {
            let mut idx = 0;
            for _ in 0..total_pixels {
                let (r, g, b) =
                    process_pixel(pixels[idx], pixels[idx + 1], pixels[idx + 2], &params);
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                idx += 4;
            }
        }

        *image = image::DynamicImage::ImageRgba8(rgba);
    }

    /// Apply color adjustments to an RGBA image buffer in-place
    ///
    /// This is the fastest method as it avoids DynamicImage conversion overhead.
    /// Use this when you already have an RgbaImage.
    #[inline]
    pub fn apply_to_rgba_mut(&self, rgba: &mut image::RgbaImage) {
        if !self.enabled || self.is_identity() {
            return;
        }

        let (width, height) = rgba.dimensions();
        let total_pixels = (width as usize) * (height as usize);

        // Pre-compute all adjustment parameters once
        let params = AdjustmentParams::from_adjustments(self);

        // Get mutable access to raw pixel data
        let pixels: &mut [u8] = rgba.as_mut();

        // Use parallel processing for large images
        const PARALLEL_THRESHOLD: usize = 100_000;

        #[cfg(feature = "threading")]
        if total_pixels >= PARALLEL_THRESHOLD {
            pixels.par_chunks_exact_mut(4).for_each(|chunk| {
                let (r, g, b) = process_pixel(chunk[0], chunk[1], chunk[2], &params);
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
            });
        } else {
            let mut idx = 0;
            for _ in 0..total_pixels {
                let (r, g, b) =
                    process_pixel(pixels[idx], pixels[idx + 1], pixels[idx + 2], &params);
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                idx += 4;
            }
        }

        #[cfg(not(feature = "threading"))]
        {
            let mut idx = 0;
            for _ in 0..total_pixels {
                let (r, g, b) =
                    process_pixel(pixels[idx], pixels[idx + 1], pixels[idx + 2], &params);
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                idx += 4;
            }
        }
    }

    /// Apply color adjustments to an RGB image buffer in-place
    #[inline]
    pub fn apply_to_rgb_mut(&self, rgb: &mut image::RgbImage) {
        if !self.enabled || self.is_identity() {
            return;
        }

        let (width, height) = rgb.dimensions();
        let total_pixels = (width as usize) * (height as usize);

        // Pre-compute all adjustment parameters once
        let params = AdjustmentParams::from_adjustments(self);

        // Get mutable access to raw pixel data
        let pixels: &mut [u8] = rgb.as_mut();

        // Use parallel processing for large images
        const PARALLEL_THRESHOLD: usize = 100_000;

        #[cfg(feature = "threading")]
        if total_pixels >= PARALLEL_THRESHOLD {
            pixels.par_chunks_exact_mut(3).for_each(|chunk| {
                let (r, g, b) = process_pixel(chunk[0], chunk[1], chunk[2], &params);
                chunk[0] = r;
                chunk[1] = g;
                chunk[2] = b;
            });
        } else {
            let mut idx = 0;
            for _ in 0..total_pixels {
                let (r, g, b) =
                    process_pixel(pixels[idx], pixels[idx + 1], pixels[idx + 2], &params);
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                idx += 3;
            }
        }

        #[cfg(not(feature = "threading"))]
        {
            let mut idx = 0;
            for _ in 0..total_pixels {
                let (r, g, b) =
                    process_pixel(pixels[idx], pixels[idx + 1], pixels[idx + 2], &params);
                pixels[idx] = r;
                pixels[idx + 1] = g;
                pixels[idx + 2] = b;
                idx += 3;
            }
        }
    }

    /// Render UI for color adjustments
    #[cfg(any(feature = "desktop", feature = "web"))]
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.checkbox(&mut self.enabled, t!("color.adjustments_enabled"));

        // Sliders enabled when color adjustments are enabled
        ui.add_enabled_ui(self.enabled, |ui| {
            egui::Grid::new("color_adjustments_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    // Basic adjustments
                    ui.label(t!("color.exposure"));
                    ui.add(egui::Slider::new(&mut self.exposure, -5.0..=5.0).suffix(" EV"));
                    ui.end_row();

                    ui.label(t!("color.contrast"));
                    ui.add(egui::Slider::new(&mut self.contrast, -100..=100));
                    ui.end_row();

                    ui.label(t!("color.highlights"));
                    ui.add(egui::Slider::new(&mut self.highlights, -100..=100));
                    ui.end_row();

                    ui.label(t!("color.shadows"));
                    ui.add(egui::Slider::new(&mut self.shadows, -100..=100));
                    ui.end_row();

                    ui.label(t!("color.whites"));
                    ui.add(egui::Slider::new(&mut self.whites, -100..=100));
                    ui.end_row();

                    ui.label(t!("color.blacks"));
                    ui.add(egui::Slider::new(&mut self.blacks, -100..=100));
                    ui.end_row();

                    ui.separator();
                    ui.separator();
                    ui.end_row();

                    // Presence adjustments
                    ui.label(t!("color.clarity"));
                    ui.add(egui::Slider::new(&mut self.clarity, -100..=100));
                    ui.end_row();

                    ui.label(t!("color.vibrance"));
                    ui.add(egui::Slider::new(&mut self.vibrance, -100..=100));
                    ui.end_row();

                    ui.label(t!("color.saturation"));
                    ui.add(egui::Slider::new(&mut self.saturation, -100..=100));
                    ui.end_row();
                });
        });

        // Reset button (enabled)
        if ui.button(t!("color.reset_adjustments")).clicked() {
            self.reset();
        }
    }
}

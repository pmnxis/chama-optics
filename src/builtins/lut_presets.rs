// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Built-in LUT CUBE file generation.
//!
//! Generates CUBE-format LUT files algorithmically for each preset.
//! All LUTs use a 17x17x17 grid for reasonable quality with modest file size.

use super::*;
use crate::effect::lut_storage::{LutItem, LutStorage};

const LUT_SIZE: usize = 17;

/// Generate a CUBE format 3D LUT string using the given color transform.
///
/// The transform receives (r, g, b) in [0.0, 1.0] and returns (r, g, b) in [0.0, 1.0].
fn generate_cube<F>(title: &str, transform: F) -> String
where
    F: Fn(f32, f32, f32) -> (f32, f32, f32),
{
    let n = LUT_SIZE;
    let step = 1.0 / (n as f32 - 1.0);

    let mut out = String::with_capacity(n * n * n * 20 + 256);
    out.push_str(&format!("TITLE \"{}\"\n", title));
    out.push_str(&format!("LUT_3D_SIZE {}\n", n));
    out.push_str("DOMAIN_MIN 0.0 0.0 0.0\n");
    out.push_str("DOMAIN_MAX 1.0 1.0 1.0\n");
    out.push('\n');

    // CUBE order: B outer, G middle, R inner
    for bi in 0..n {
        for gi in 0..n {
            for ri in 0..n {
                let r_in = ri as f32 * step;
                let g_in = gi as f32 * step;
                let b_in = bi as f32 * step;

                let (r_out, g_out, b_out) = transform(r_in, g_in, b_in);

                // Clamp to valid range
                let r_out = r_out.clamp(0.0, 1.0);
                let g_out = g_out.clamp(0.0, 1.0);
                let b_out = b_out.clamp(0.0, 1.0);

                out.push_str(&format!("{:.6} {:.6} {:.6}\n", r_out, g_out, b_out));
            }
        }
    }

    out
}

/// Warm Sunrise: lifts shadows, slightly warm (+R, +G, -B)
fn warm_sunrise_transform(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let shadow_lift = 0.03;
    // Lift shadows
    let r2 = r + shadow_lift * (1.0 - r);
    let g2 = g + shadow_lift * (1.0 - g) * 0.7;
    let b2 = b + shadow_lift * (1.0 - b) * 0.3;
    // Warm shift
    let r3 = (r2 + 0.05 * (1.0 - r2)).min(1.0);
    let g3 = (g2 + 0.02 * (1.0 - g2)).min(1.0);
    let b3 = (b2 - 0.04 * b2).max(0.0);
    (r3, g3, b3)
}

/// Cool Dusk: slightly blue-shifted, slight highlight roll-off
fn cool_dusk_transform(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let r2 = (r - 0.04 * r).max(0.0);
    let g2 = (g - 0.01 * g).max(0.0);
    let b2 = (b + 0.06 * (1.0 - b)).min(1.0);
    // Slight contrast
    let contrast = |v: f32| ((v - 0.5) * 1.05 + 0.5).clamp(0.0, 1.0);
    (contrast(r2), contrast(g2), contrast(b2))
}

/// Cinematic B&W: desaturate using luminance weights, add slight contrast S-curve
fn cinematic_bw_transform(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    // Slight S-curve for cinematic look
    let s_curve = |v: f32| {
        // Simple approximation: steeper in midtones
        let v2 = if v < 0.5 {
            0.5 * (2.0 * v).powf(1.2)
        } else {
            1.0 - 0.5 * (2.0 * (1.0 - v)).powf(1.2)
        };
        v2.clamp(0.0, 1.0)
    };
    let out = s_curve(luma);
    // Slight sepia tint for warmth
    let r_out = (out * 1.08).min(1.0);
    let g_out = out;
    let b_out = (out * 0.90).max(0.0);
    (r_out, g_out, b_out)
}

/// Vivid: boosted saturation and contrast
fn vivid_transform(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    // Boost saturation by 30%
    let sat_factor = 1.3;
    let r2 = (luma + (r - luma) * sat_factor).clamp(0.0, 1.0);
    let g2 = (luma + (g - luma) * sat_factor).clamp(0.0, 1.0);
    let b2 = (luma + (b - luma) * sat_factor).clamp(0.0, 1.0);
    // Slight contrast boost
    let contrast = |v: f32| ((v - 0.5) * 1.1 + 0.5).clamp(0.0, 1.0);
    (contrast(r2), contrast(g2), contrast(b2))
}

/// Definition of a single built-in LUT preset
pub struct BuiltinLutDef {
    pub id: Uuid,
    pub name: &'static str,
    pub filename: &'static str,
}

/// All built-in LUT definitions in display order
pub fn builtin_lut_defs() -> Vec<BuiltinLutDef> {
    vec![
        BuiltinLutDef {
            id: BUILTIN_LUT_WARM_SUNRISE_ID,
            name: "Warm Sunrise",
            filename: "builtin_warm_sunrise.cube",
        },
        BuiltinLutDef {
            id: BUILTIN_LUT_COOL_DUSK_ID,
            name: "Cool Dusk",
            filename: "builtin_cool_dusk.cube",
        },
        BuiltinLutDef {
            id: BUILTIN_LUT_CINEMATIC_BW_ID,
            name: "Cinematic B&W",
            filename: "builtin_cinematic_bw.cube",
        },
        BuiltinLutDef {
            id: BUILTIN_LUT_VIVID_ID,
            name: "Vivid",
            filename: "builtin_vivid.cube",
        },
    ]
}

/// Generate CUBE content for a given built-in LUT UUID.
/// Returns None if the UUID is not a known built-in LUT.
pub fn generate_builtin_lut_content(id: Uuid) -> Option<String> {
    if id == BUILTIN_LUT_WARM_SUNRISE_ID {
        Some(generate_cube("Warm Sunrise", warm_sunrise_transform))
    } else if id == BUILTIN_LUT_COOL_DUSK_ID {
        Some(generate_cube("Cool Dusk", cool_dusk_transform))
    } else if id == BUILTIN_LUT_CINEMATIC_BW_ID {
        Some(generate_cube("Cinematic B&W", cinematic_bw_transform))
    } else if id == BUILTIN_LUT_VIVID_ID {
        Some(generate_cube("Vivid", vivid_transform))
    } else {
        None
    }
}

/// Initialize built-in LUTs in the given storage directory.
///
/// For each built-in LUT:
/// - If not present in storage, generate the CUBE file and add it to storage.
/// - If already present (any visibility state), leave it alone.
///
/// Returns the number of new built-in LUTs added.
pub fn init_builtin_luts(storage: &mut LutStorage) -> usize {
    let mut added = 0;

    for def in builtin_lut_defs() {
        // Already registered (visible or hidden)
        if storage.luts.iter().any(|l| l.id == def.id) {
            continue;
        }

        let file_path = storage.storage_directory.join(def.filename);

        // Generate CUBE content
        let Some(content) = generate_builtin_lut_content(def.id) else {
            log::error!("Failed to generate built-in LUT content for {}", def.name);
            continue;
        };

        // Write CUBE file to storage directory
        if let Err(e) = std::fs::write(&file_path, &content) {
            log::error!("Failed to write built-in LUT {}: {}", def.name, e);
            continue;
        }

        // Parse to get metadata (wrap string in Cursor to provide Read impl)
        let parsed = match wagahai_lut::CubeParser::parse(std::io::Cursor::new(content.as_bytes()))
        {
            Ok(lut) => lut,
            Err(e) => {
                log::error!(
                    "Failed to parse generated built-in LUT {}: {:?}",
                    def.name,
                    e
                );
                continue;
            }
        };

        // Create LutItem with fixed UUID and builtin flag
        let mut item = LutItem::new(def.name.to_string(), file_path, &parsed);
        item.id = def.id;
        item.is_builtin = true;
        item.is_hidden = false;

        storage.luts.push(item);
        added += 1;

        log::info!("Initialized built-in LUT: {}", def.name);
    }

    added
}

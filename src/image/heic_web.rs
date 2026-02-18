/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! HEIF/HEIC decoding for WASM via libheif-js (JavaScript interop).
//!
//! Unlike desktop (C FFI -> libheif with direct memory access), WASM goes through JS:
//!   Rust WASM -> JS (copy HEIF bytes) -> libheif-js decode -> JS -> Rust WASM (copy RGBA pixels)
//! This involves two data copies across the WASM<->JS boundary.
//! For a 24MP image: ~10-30MB out (HEIF) + ~96MB back (RGBA) as temporary allocations.
//! The RGBA buffer is freed immediately after thumbnail generation.

use crate::exif_impl::{OriginalExif, SimplifiedExif};
use crate::image::loader::LoadedImageData;
use js_sys::{Promise, Reflect, Uint8Array};
use std::path::PathBuf;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/heif_helper.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn decode_heif_from_bytes(data: &[u8]) -> Result<Promise, JsValue>;
}

/// Check if a file appears to be HEIF/HEIC format based on extension or magic bytes.
pub fn is_heif(filename: &str, bytes: &[u8]) -> bool {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    if matches!(ext.as_str(), "heic" | "heif" | "hif") {
        return true;
    }
    // HEIF ISO BMFF ftyp box: bytes 4..8 == "ftyp", bytes 8..12 == brand
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        return matches!(
            brand,
            b"heic" | b"heix" | b"hevc" | b"hevx" | b"heim" | b"heis" | b"mif1" | b"msf1"
        );
    }
    false
}

/// Decode HEIF/HEIC bytes via libheif-js and return (width, height, rgba_pixels).
///
/// The returned RGBA buffer is full-resolution (width * height * 4 bytes).
/// Caller should generate thumbnail and drop the large buffer promptly.
async fn decode_heif_rgba(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let promise = decode_heif_from_bytes(bytes).map_err(|e| format!("JS call failed: {:?}", e))?;

    // 30-second timeout to prevent hanging on corrupt/huge HEIF files
    let js_result = crate::util::web_helper::race_with_timeout(promise, 30_000)
        .await
        .map_err(|e| format!("HEIF decode failed: {}", e))?;

    let width = Reflect::get(&js_result, &"width".into())
        .map_err(|_| "Missing width".to_string())?
        .as_f64()
        .ok_or("Invalid width")? as u32;

    let height = Reflect::get(&js_result, &"height".into())
        .map_err(|_| "Missing height".to_string())?
        .as_f64()
        .ok_or("Invalid height")? as u32;

    let data_js = Reflect::get(&js_result, &"data".into())
        .map_err(|_| "Missing data".to_string())?
        .dyn_into::<Uint8Array>()
        .map_err(|_| "Invalid data type".to_string())?;

    // Single copy from JS heap to WASM linear memory
    let mut rgba = vec![0u8; data_js.length() as usize];
    data_js.copy_to(&mut rgba);

    let expected = (width as usize) * (height as usize) * 4;
    if rgba.len() != expected {
        return Err(format!(
            "Buffer size mismatch: got {} expected {} ({}x{}x4)",
            rgba.len(),
            expected,
            width,
            height
        ));
    }

    Ok((width, height, rgba))
}

/// Full HEIF loading pipeline for WASM: EXIF parse + JS decode + thumbnail generation.
///
/// Returns LoadedImageData ready to be converted to PackedImage in the UI thread.
/// The original HEIF bytes are stored in image_bytes for later re-decode (effects/export).
pub async fn load_heif_image_data(
    bytes: &[u8],
    filename: &str,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
    simplify_lens_model: bool,
) -> Result<LoadedImageData, String> {
    // 1. Parse EXIF from original HEIF bytes (synchronous, fast)
    let original_exif = {
        let mut cursor = std::io::Cursor::new(bytes);
        OriginalExif::new(match exif::Reader::new().read_from_container(&mut cursor) {
            Ok(exif) => Some(exif),
            Err(e) => {
                log::warn!("Failed to parse EXIF from HEIF {}: {e:?}", filename);
                None
            }
        })
    };

    let mut view_exif = SimplifiedExif::from(&original_exif);
    if get_alt_fnumber {
        view_exif.replace_with_fnumber_alt_when_invalid();
    }
    if use_35mm_focal_length {
        view_exif.use_35mm_focal_length(&original_exif);
    }
    if simplify_lens_model {
        view_exif.apply_simplify_lens_model();
    }

    let orientation = original_exif.orientation();

    // 2. Decode HEIF via JS interop (async, involves WASM<->JS data copies)
    log::info!(
        "HEIF web decode: {} ({} bytes) — sending to libheif-js",
        filename,
        bytes.len()
    );
    let (width, height, rgba) = decode_heif_rgba(bytes).await?;
    log::info!(
        "HEIF decoded: {}x{} ({} bytes RGBA)",
        width,
        height,
        rgba.len()
    );

    // 3. Create DynamicImage from decoded RGBA pixels
    let image_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, rgba)
        .ok_or("Failed to create ImageBuffer from HEIF RGBA")?;
    let dyn_image = image::DynamicImage::ImageRgba8(image_buffer);

    // 4. Re-encode as high-quality JPEG for image_bytes storage.
    //    The image crate can't decode HEIF, so tabs that reload via
    //    image::load_from_memory() need a format it understands.
    let jpeg_bytes = {
        let mut buf = std::io::Cursor::new(Vec::new());
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 95);
        dyn_image
            .to_rgb8()
            .write_with_encoder(encoder)
            .map_err(|e| format!("JPEG re-encode failed: {:?}", e))?;
        buf.into_inner()
    };
    log::warn!(
        "⚠ HEIF image '{}' re-encoded as JPEG (quality 95%) for WASM compatibility. \
         Original: {} bytes → JPEG: {} bytes. \
         Slight quality loss may occur in processing/export.",
        filename,
        bytes.len(),
        jpeg_bytes.len()
    );

    // 5. Generate thumbnail using existing pipeline (reuse Rust resize/crop/orientation)
    // dyn_image is consumed here — only thumbnail + jpeg_bytes remain
    let thumbnail = crate::image::common::gen_thumbnail(dyn_image, orientation)
        .map_err(|e| format!("Thumbnail generation failed: {:?}", e))?;

    let perceptual_hash =
        crate::image::loader::calculate_perceptual_hash_from_thumbnail(&thumbnail);

    // Use .jpg extension since image_bytes now contains JPEG data (re-encoded from HEIF).
    // The image crate uses path extension for format detection, and .HIF/.HEIC aren't recognized.
    let pseudo_path = PathBuf::from(format!("web://{}.jpg", filename));

    Ok(LoadedImageData {
        path: pseudo_path,
        view_exif,
        thumbnail: Some(thumbnail),
        orientation,
        image_bytes: Some(jpeg_bytes),
        perceptual_hash,
    })
}

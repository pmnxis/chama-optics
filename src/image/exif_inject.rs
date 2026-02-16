/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! EXIF metadata injection for exported images.
//!
//! After Rust saves pixel-only output (JPEG/WebP), this module reads EXIF from
//! the original image, filters out MakerNote and GPS, applies user overrides,
//! serializes via `exif::experimental::Writer`, and injects into the output
//! file using `img-parts`.

use std::collections::HashSet;
use std::io::{BufReader, Cursor};
use std::path::Path;

use exif::experimental::Writer;
use exif::{Context, Field, In, Tag, Value};

use super::exif_impl::SimplifiedExif;

/// Inject EXIF metadata from `original_path` into the already-saved `output_path`.
///
/// - Reads EXIF from `original_path`
/// - Filters out MakerNote, GPS, orientation, and maker note sub-IFDs
/// - Applies user override fields from `exif_override_json` (SimplifiedExif JSON)
/// - Serializes to raw TIFF bytes via `exif::experimental::Writer`
/// - Injects into JPEG or WebP container via `img-parts`
///
/// Returns Ok(()) on success. Errors are non-fatal for the export pipeline.
pub fn inject_exif_to_output(
    original_path: &str,
    output_path: &str,
    exif_override_json: Option<&str>,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
) -> Result<(), anyhow::Error> {
    // Step 1: Read original EXIF
    let file = std::fs::File::open(original_path)?;
    let mut buf_reader = BufReader::new(file);
    let original_exif = exif::Reader::new()
        .read_from_container(&mut buf_reader)
        .map_err(|e| anyhow::anyhow!("Failed to read EXIF from original: {}", e))?;

    // Step 2: Parse override JSON (if provided)
    let override_exif: Option<SimplifiedExif> = exif_override_json
        .filter(|s| !s.is_empty())
        .and_then(|json| serde_json::from_str(json).ok());

    // Step 3: Build override fields and tag set
    let override_fields = build_override_fields(
        &override_exif,
        &original_exif,
        get_alt_fnumber,
        use_35mm_focal_length,
    );
    let overridden_tags: HashSet<Tag> = override_fields.iter().map(|f| f.tag).collect();

    // Step 4: Create Writer and push filtered fields
    let mut writer = Writer::new();

    for field in original_exif.fields() {
        // Skip MakerNote tag
        if field.tag == Tag::MakerNote {
            continue;
        }
        // Skip GPS context entirely
        if field.tag.context() == Context::Gps {
            continue;
        }
        // Skip orientation (image is already corrected)
        if field.tag == Tag::Orientation {
            continue;
        }
        // Skip IFD >= 2 (maker note sub-IFDs)
        if matches!(field.ifd_num, In(n) if n >= 2) {
            continue;
        }
        // Skip dimension tags (image dimensions may have changed)
        if field.tag == Tag::ImageWidth
            || field.tag == Tag::ImageLength
            || field.tag == Tag::PixelXDimension
            || field.tag == Tag::PixelYDimension
        {
            continue;
        }
        // Skip if overridden by user edits
        if overridden_tags.contains(&field.tag) {
            continue;
        }
        // Skip Unknown values (Writer cannot serialize them)
        if matches!(field.value, Value::Unknown(_, _, _)) {
            continue;
        }

        writer.push_field(field);
    }

    // Push orientation = 1 (Normal, image already corrected)
    let orientation_field = Field {
        tag: Tag::Orientation,
        ifd_num: In::PRIMARY,
        value: Value::Short(vec![1]),
    };
    writer.push_field(&orientation_field);

    // Push override fields
    for field in &override_fields {
        writer.push_field(field);
    }

    // Step 5: Serialize to raw TIFF bytes
    let mut cursor = Cursor::new(Vec::new());
    writer
        .write(&mut cursor, original_exif.little_endian())
        .map_err(|e| anyhow::anyhow!("Failed to write EXIF: {}", e))?;
    let exif_bytes = cursor.into_inner();

    log::info!(
        "EXIF serialized: {} bytes (from {})",
        exif_bytes.len(),
        original_path
    );

    // Step 6: Detect output format and inject
    let output_ext = Path::new(output_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match output_ext.as_str() {
        "jpg" | "jpeg" => inject_exif_jpeg(output_path, &exif_bytes),
        "webp" => inject_exif_webp(output_path, &exif_bytes),
        _ => {
            log::info!("EXIF injection skipped for format: {}", output_ext);
            Ok(())
        }
    }
}

/// Inject EXIF bytes into a JPEG file using img-parts.
fn inject_exif_jpeg(output_path: &str, exif_bytes: &[u8]) -> Result<(), anyhow::Error> {
    use img_parts::ImageEXIF;
    use img_parts::jpeg::Jpeg;

    let file_data = std::fs::read(output_path)?;
    let mut jpeg = Jpeg::from_bytes(file_data.into())
        .map_err(|e| anyhow::anyhow!("Failed to parse JPEG: {}", e))?;

    jpeg.set_exif(Some(exif_bytes.to_vec().into()));

    let output_file = std::fs::File::create(output_path)?;
    jpeg.encoder()
        .write_to(output_file)
        .map_err(|e| anyhow::anyhow!("Failed to write JPEG with EXIF: {}", e))?;

    log::info!("EXIF injected into JPEG: {}", output_path);
    Ok(())
}

/// Inject EXIF bytes into a WebP file using img-parts.
fn inject_exif_webp(output_path: &str, exif_bytes: &[u8]) -> Result<(), anyhow::Error> {
    use img_parts::ImageEXIF;
    use img_parts::webp::WebP;

    let file_data = std::fs::read(output_path)?;
    let mut webp = WebP::from_bytes(file_data.into())
        .map_err(|e| anyhow::anyhow!("Failed to parse WebP: {}", e))?;

    webp.set_exif(Some(exif_bytes.to_vec().into()));

    let output_file = std::fs::File::create(output_path)?;
    webp.encoder()
        .write_to(output_file)
        .map_err(|e| anyhow::anyhow!("Failed to write WebP with EXIF: {}", e))?;

    log::info!("EXIF injected into WebP: {}", output_path);
    Ok(())
}

/// Build override EXIF fields from SimplifiedExif.
/// Only non-empty fields are included. Returns owned Field values.
fn build_override_fields(
    override_exif: &Option<SimplifiedExif>,
    original_exif: &exif::Exif,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
) -> Vec<Field> {
    let mut fields = Vec::new();

    let Some(exif) = override_exif else {
        return fields;
    };

    // Camera Make
    if !exif.camera_mnf.is_empty() {
        fields.push(Field {
            tag: Tag::Make,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![exif.camera_mnf.as_bytes().to_vec()]),
        });
    }

    // Camera Model
    if !exif.camera_model.is_empty() {
        fields.push(Field {
            tag: Tag::Model,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![exif.camera_model.as_bytes().to_vec()]),
        });
    }

    // Lens Make
    if !exif.lens_mnf.is_empty() {
        fields.push(Field {
            tag: Tag::LensMake,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![exif.lens_mnf.as_bytes().to_vec()]),
        });
    }

    // Lens Model
    if !exif.lens_model.is_empty() {
        fields.push(Field {
            tag: Tag::LensModel,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![exif.lens_model.as_bytes().to_vec()]),
        });
    }

    // Focal Length
    if !exif.focal.is_empty()
        && let Some(rational) = parse_focal_length(&exif.focal)
    {
        fields.push(Field {
            tag: Tag::FocalLength,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![rational]),
        });
    }

    // F-Number
    let fnumber_str = if get_alt_fnumber {
        exif.get_fnumber_alt().unwrap_or_default()
    } else {
        exif.get_fnumber().unwrap_or_default()
    };
    if !fnumber_str.is_empty()
        && let Some(rational) = parse_fnumber(&fnumber_str)
    {
        fields.push(Field {
            tag: Tag::FNumber,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![rational]),
        });
    }

    // Exposure Time
    if !exif.exposure.is_empty()
        && let Some(rational) = parse_exposure_time(&exif.exposure)
    {
        fields.push(Field {
            tag: Tag::ExposureTime,
            ifd_num: In::PRIMARY,
            value: Value::Rational(vec![rational]),
        });
    }

    // ISO Speed
    if let Some(iso) = exif.iso_speed {
        fields.push(Field {
            tag: Tag::PhotographicSensitivity,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![iso.min(65535) as u16]),
        });
    }

    // DateTime Original
    if let Some(dt) = exif.datetime {
        let dt_str = dt.format("%Y:%m:%d %H:%M:%S").to_string();
        fields.push(Field {
            tag: Tag::DateTimeOriginal,
            ifd_num: In::PRIMARY,
            value: Value::Ascii(vec![dt_str.as_bytes().to_vec()]),
        });
    }

    // 35mm Focal Length (if enabled and available from original)
    if use_35mm_focal_length
        && let Some(f35) = original_exif
            .get_field(Tag::FocalLengthIn35mmFilm, In::PRIMARY)
            .and_then(|f| f.value.get_uint(0))
    {
        fields.push(Field {
            tag: Tag::FocalLengthIn35mmFilm,
            ifd_num: In::PRIMARY,
            value: Value::Short(vec![f35.min(65535) as u16]),
        });
    }

    fields
}

/// Parse focal length string (e.g., "50", "50mm", "50 mm") to Rational.
fn parse_focal_length(s: &str) -> Option<exif::Rational> {
    let cleaned = s
        .trim()
        .trim_end_matches("mm")
        .trim_end_matches("MM")
        .trim();
    let val: f64 = cleaned.parse().ok()?;
    // Convert to rational: multiply by 100 for precision
    let num = (val * 100.0).round() as u32;
    Some(exif::Rational { num, denom: 100 })
}

/// Parse f-number string (e.g., "2.8", "f/2.8", "F2.8") to Rational.
fn parse_fnumber(s: &str) -> Option<exif::Rational> {
    let cleaned = s
        .trim()
        .trim_start_matches("f/")
        .trim_start_matches("F/")
        .trim_start_matches("f")
        .trim_start_matches("F")
        .trim();
    let val: f64 = cleaned.parse().ok()?;
    let num = (val * 100.0).round() as u32;
    Some(exif::Rational { num, denom: 100 })
}

/// Parse exposure time string (e.g., "1/250", "1/250s", "0.004") to Rational.
fn parse_exposure_time(s: &str) -> Option<exif::Rational> {
    let cleaned = s.trim().trim_end_matches('s').trim_end_matches('S').trim();
    if let Some((num_str, denom_str)) = cleaned.split_once('/') {
        let num: u32 = num_str.trim().parse().ok()?;
        let denom: u32 = denom_str.trim().parse().ok()?;
        Some(exif::Rational { num, denom })
    } else {
        // Decimal format like "0.004"
        let val: f64 = cleaned.parse().ok()?;
        if val > 0.0 {
            // Convert to rational
            let denom = (1.0 / val).round() as u32;
            Some(exif::Rational { num: 1, denom })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_focal_length() {
        let r = parse_focal_length("50").unwrap();
        assert_eq!(r.num, 5000);
        assert_eq!(r.denom, 100);

        let r = parse_focal_length("50mm").unwrap();
        assert_eq!(r.num, 5000);
        assert_eq!(r.denom, 100);

        let r = parse_focal_length("24.5 mm").unwrap();
        assert_eq!(r.num, 2450);
        assert_eq!(r.denom, 100);
    }

    #[test]
    fn test_parse_fnumber() {
        let r = parse_fnumber("2.8").unwrap();
        assert_eq!(r.num, 280);
        assert_eq!(r.denom, 100);

        let r = parse_fnumber("f/1.4").unwrap();
        assert_eq!(r.num, 140);
        assert_eq!(r.denom, 100);

        let r = parse_fnumber("F2.8").unwrap();
        assert_eq!(r.num, 280);
        assert_eq!(r.denom, 100);
    }

    #[test]
    fn test_parse_exposure_time() {
        let r = parse_exposure_time("1/250").unwrap();
        assert_eq!(r.num, 1);
        assert_eq!(r.denom, 250);

        let r = parse_exposure_time("1/250s").unwrap();
        assert_eq!(r.num, 1);
        assert_eq!(r.denom, 250);

        let r = parse_exposure_time("1/60").unwrap();
        assert_eq!(r.num, 1);
        assert_eq!(r.denom, 60);
    }
}

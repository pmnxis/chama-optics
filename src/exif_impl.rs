/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use exif::{In, Tag};

#[allow(unused)]
pub struct SimplifiedExif {
    pub camera_model: String,
    pub camera_mnf: String,
    pub lens_model: String,
}

/// Remove trash chars from exif string field
fn simplify_exif_string(input: &str) -> String {
    let mut parts = Vec::new();
    let mut inside = false;
    let mut current = String::new();

    for c in input.chars() {
        match c {
            '"' => {
                if inside {
                    // " closed
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        parts.push(trimmed.to_string());
                    }
                    current.clear();
                    inside = false;
                } else {
                    // " opened
                    inside = true;
                }
            }
            _ if inside => current.push(c),
            _ => {}
        }
    }

    parts.join(" | ")
}

impl crate::packed_image::PackedImage {
    fn get_exif_value(&self, tag: Tag) -> String {
        self.src_exif
            .as_ref()
            .and_then(|exif| {
                exif.get_field(tag, In::PRIMARY)
                    .map(|f| f.display_value().to_string())
            })
            .unwrap_or_default()
    }

    fn get_exif_trim_string(&self, tag: Tag) -> String {
        self.src_exif
            .as_ref()
            .and_then(|exif| {
                exif.get_field(tag, In::PRIMARY)
                    .map(|f| simplify_exif_string(f.display_value().to_string().as_str()))
            })
            .unwrap_or_default()
    }

    // todo - Make orientation enum and rotated thumbnail as well
    pub fn orientation(&self) -> u32 {
        // Orientation (TIFF 0x112)
        let value = self
            .src_exif
            .as_ref()
            .and_then(|exif| exif.get_field(Tag::Orientation, In::PRIMARY))
            .and_then(|field| field.value.get_uint(0));
        value.unwrap_or(0)
    }

    /// Manufacturer of the image input equipment.
    pub fn camera_mnf(&self) -> String {
        self.get_exif_trim_string(Tag::Make)
    }

    /// Camera model
    pub fn camera_model(&self) -> String {
        // hex_dump(value.as_str());
        self.get_exif_trim_string(Tag::Model)
    }

    /// Lens manufacturer
    pub fn lens_mnf(&self) -> String {
        self.get_exif_trim_string(Tag::LensMake)
    }

    /// Lens Model
    pub fn lens_model(&self) -> String {
        // hex_dump(value.as_str());
        self.get_exif_trim_string(Tag::LensModel)
    }

    /// Focal length with mm
    pub fn focal(&self) -> String {
        self.get_exif_value(Tag::FocalLength)
    }

    /// Lens aperture (F-number)
    pub fn fnumber(&self) -> String {
        self.get_exif_value(Tag::FNumber)
    }

    /// Exposure time
    pub fn exposure(&self) -> String {
        self.get_exif_value(Tag::ExposureTime)
    }

    /// ISO Speed
    pub fn iso_speed(&self) -> Option<u32> {
        self.src_exif
            .as_ref()
            .and_then(|exif| {
                exif.get_field(Tag::ISOSpeed, In::PRIMARY)
                    .or_else(|| exif.get_field(Tag::StandardOutputSensitivity, In::PRIMARY))
                    .or_else(|| exif.get_field(Tag::PhotographicSensitivity, In::PRIMARY))
            })
            .and_then(|field| field.value.get_uint(0))
    }

    /// Datetime
    pub fn datetime(&self) -> String {
        self.get_exif_value(Tag::DateTime)
    }
}

#[allow(dead_code)]
fn hex_dump(s: &str) {
    for (i, b) in s.as_bytes().iter().enumerate() {
        print!("{:02X} ", b);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();
}

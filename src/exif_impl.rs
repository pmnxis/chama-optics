/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use exif::{In, Tag};

pub(crate) const _MAX_FIELD_WIDTH: f32 = 140.0;
pub(crate) const _LABEL_SPACING: f32 = 3.0;

#[derive(Default)]
pub struct OriginalExif(Option<exif::Exif>);

impl OriginalExif {
    pub fn new(exif_or_none: Option<exif::Exif>) -> Self {
        Self(exif_or_none)
    }

    pub fn new_with_exif(exif: exif::Exif) -> Self {
        Self(Some(exif))
    }

    pub fn none() -> Self {
        Self(None)
    }

    pub fn get_exif_value(&self, tag: Tag) -> String {
        self.0
            .as_ref()
            .and_then(|exif| {
                exif.get_field(tag, In::PRIMARY)
                    .map(|f| f.display_value().to_string())
            })
            .unwrap_or_default()
    }

    pub fn get_exif_trim_string(&self, tag: Tag) -> String {
        self.0
            .as_ref()
            .and_then(|exif| {
                exif.get_field(tag, In::PRIMARY)
                    .map(|f| simplify_exif_string(f.display_value().to_string().as_str()))
            })
            .unwrap_or_default()
    }

    pub fn orientation(&self) -> crate::orientation::Orientation {
        // Orientation (TIFF 0x112)
        let value = self
            .0
            .as_ref()
            .and_then(|exif| exif.get_field(Tag::Orientation, In::PRIMARY))
            .and_then(|field| field.value.get_uint(0));
        crate::orientation::Orientation::from_tiff(value.unwrap_or(0))
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
        self.0
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

#[derive(serde::Deserialize, serde::Serialize, Default, Clone, PartialEq, PartialOrd)]
#[serde(default)]
pub struct SimplifiedExif {
    pub camera_mnf: String,
    pub camera_model: String,
    pub lens_model: String,
    pub focal: String,
    pub fnumber: String,
    pub exposure: String,
    pub iso_speed: Option<u32>,
    pub datetime: String, // Option<DateTime>,

    #[serde(skip)]
    pub orientation: crate::orientation::Orientation,
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

impl From<&OriginalExif> for SimplifiedExif {
    fn from(value: &OriginalExif) -> Self {
        Self {
            camera_mnf: value.camera_mnf(),
            camera_model: value.camera_model(),
            lens_model: value.lens_model(),
            focal: value.focal(),
            fnumber: value.fnumber(),
            exposure: value.exposure(),
            iso_speed: value.iso_speed(),
            datetime: value.datetime(),
            orientation: value.orientation(),
        }
    }
}
use egui::{RichText, TextEdit, TextStyle};

impl SimplifiedExif {
    pub fn update_ui(&mut self, ui: &mut egui::Ui, editable: bool) {
        let small_text = |text: &str| RichText::new(text).text_style(TextStyle::Small);

        ui.spacing_mut().item_spacing.y = 2.0;
        // ui.style_mut().interaction.selectable_labels = false;

        // Camera
        ui.label(small_text("Camera"));
        if editable {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut self.camera_mnf)
                        .font(TextStyle::Small)
                        .desired_width(60.0),
                );

                ui.label(small_text("\t\tModel"));
                ui.add(
                    TextEdit::singleline(&mut self.camera_model)
                        .font(TextStyle::Small)
                        .desired_width(140.0),
                );
            });
        } else {
            ui.label(small_text(&format!(
                "{}  {}",
                self.camera_mnf, self.camera_model
            )));
        }

        ui.end_row();

        // Lens
        ui.label(small_text("Lens"));
        if editable {
            ui.add(
                TextEdit::singleline(&mut self.lens_model)
                    .font(TextStyle::Small)
                    .desired_width(280.0),
            );
        } else {
            ui.label(small_text(&self.lens_model));
        }

        ui.end_row();

        // Focal
        ui.label(small_text("Focal"));
        if editable {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut self.focal)
                        .font(TextStyle::Small)
                        .desired_width(40.0),
                );
                ui.label(small_text("mm"));
            });
        } else {
            ui.label(small_text(&format!("{} mm", self.focal)));
        }
        ui.end_row();

        // F-number
        ui.label(small_text("F"));
        if editable {
            ui.add(
                TextEdit::singleline(&mut self.fnumber)
                    .font(TextStyle::Small)
                    .desired_width(40.0),
            );
        } else {
            ui.label(small_text(&self.fnumber));
        }

        ui.end_row();

        // Shutter + ISO
        ui.label(small_text("Shutter"));
        if editable {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut self.exposure)
                        .font(TextStyle::Small)
                        .desired_width(40.0),
                );
                ui.label(small_text("sec"));
            });
            ui.end_row();

            ui.label(small_text("ISO"));
            let mut iso_str = self.iso_speed.map_or(String::new(), |v| v.to_string());
            if ui
                .add(
                    TextEdit::singleline(&mut iso_str)
                        .font(TextStyle::Small)
                        .desired_width(40.0),
                )
                .changed()
                && let Ok(v) = iso_str.parse::<u32>() {
                    self.iso_speed = Some(v);
                }
        } else {
            ui.horizontal(|ui| {
                ui.label(small_text(&self.exposure));
                let iso = self.iso_speed.map_or(String::from("-"), |v| v.to_string());
                ui.label(small_text(&format!("\tISO {}", iso)));
            });
        }

        ui.end_row();

        // DateTime
        ui.label(small_text("DateTime"));
        if editable {
            ui.add(
                TextEdit::singleline(&mut self.datetime)
                    .font(TextStyle::Small)
                    .desired_width(80.0),
            );
        } else {
            ui.label(small_text(&self.datetime));
        }

        ui.end_row();
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

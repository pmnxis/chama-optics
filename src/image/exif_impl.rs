/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use chrono::{Datelike, Timelike};
use exif::{In, Tag};

mod hotfix {
    include!("exif_hotfix.rs");
}

/// Custom serialization/deserialization for Option<NaiveDateTime>
/// Serializes as ISO 8601 string (e.g., "2024-12-27T13:45:30")
mod datetime_format {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        datetime: &Option<chrono::NaiveDateTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match datetime {
            Some(dt) => serializer.serialize_some(&dt.format("%Y-%m-%d %H:%M:%S").to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<chrono::NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: Option<String> = Option::deserialize(deserializer)?;
        match s {
            Some(s) if !s.is_empty() => {
                chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
            _ => Ok(None),
        }
    }
}

pub(crate) const _MAX_FIELD_WIDTH: f32 = 140.0;
pub(crate) const _LABEL_SPACING: f32 = 3.0;

#[derive(Default)]
pub struct OriginalExif(Option<exif::Exif>);

pub fn __is_vertical_rotated(ort: image::metadata::Orientation) -> bool {
    matches!(
        ort,
        image::metadata::Orientation::Rotate90 | image::metadata::Orientation::Rotate270
    )
}

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

    pub fn get_exif_decimal_string(&self, tag: Tag, round_up: Option<i32>) -> String {
        self.0
            .as_ref()
            .and_then(|exif| {
                exif.get_field(tag, In::PRIMARY).map(|f| {
                    let mut s = String::new();
                    let _ = hotfix::d_decimal(&mut s, &f.value, round_up.unwrap_or(2));
                    s
                })
            })
            .unwrap_or_default()
    }

    pub fn orientation(&self) -> image::metadata::Orientation {
        // Orientation (TIFF 0x112)
        let value = self
            .0
            .as_ref()
            .and_then(|exif| exif.get_field(Tag::Orientation, In::PRIMARY))
            .and_then(|field| field.value.get_uint(0));
        image::metadata::Orientation::from_exif(value.unwrap_or(0) as u8)
            .unwrap_or(image::metadata::Orientation::NoTransforms)
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
    pub fn lens_maker(&self) -> String {
        // hex_dump(value.as_str());
        self.get_exif_trim_string(Tag::LensMake)
    }

    /// Lens Model
    pub fn lens_model(&self) -> String {
        // hex_dump(value.as_str());
        self.get_exif_trim_string(Tag::LensModel)
    }

    /// Physical focal length (actual lens focal length)
    /// This returns the physical focal length of the lens
    pub fn focal(&self) -> String {
        self.get_exif_value(Tag::FocalLength)
    }

    /// Get 35mm equivalent focal length if available
    /// Returns None if FocalLengthIn35mmFilm is not present in EXIF
    pub fn focal_35mm(&self) -> Option<u32> {
        self.0.as_ref().and_then(|exif| {
            exif.get_field(Tag::FocalLengthIn35mmFilm, In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
        })
    }

    /// Lens aperture (F-number)
    pub fn fnumber(&self) -> String {
        self.get_exif_decimal_string(Tag::FNumber, None)
    }

    // this is initial implement

    pub fn make_note(&self) -> Option<SimplifiedMakeNote> {
        self.0.as_ref().map(|exif| SimplifiedMakeNote {
            photo_style: crate::image::make_note::MakePhotoStyle::from_exif(exif),
        })
    }

    /// Exposure time
    pub fn exposure(&self) -> String {
        self.0
            .as_ref()
            .and_then(|exif| {
                exif.get_field(Tag::ExposureTime, In::PRIMARY).map(|f| {
                    let mut s = String::new();
                    let _ = hotfix::d_exptime(&mut s, &f.value);
                    s
                })
            })
            .unwrap_or_default()
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

    /// Datetime as parsed NaiveDateTime
    pub fn datetime(&self) -> Option<chrono::NaiveDateTime> {
        let exif = self.0.as_ref()?;

        // Try to get DateTimeOriginal first
        let field = exif.get_field(Tag::DateTimeOriginal, In::PRIMARY);

        if let Some(field) = field {
            let datetime_str = field.display_value().to_string();
            log::debug!(
                "DEBUG: EXIF DateTimeOriginal field found: '{}'",
                datetime_str
            );

            // EXIF datetime format can be either:
            // 1. "YYYY-MM-DD HH:MM:SS" (common in many cameras)
            // 2. "YYYY:MM:DD HH:MM:SS" (EXIF standard)
            // Try both formats
            let parsed = chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S")
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y:%m:%d %H:%M:%S")
                });

            match parsed {
                Ok(dt) => Some(dt),
                Err(e) => {
                    log::warn!(
                        "DEBUG: Failed to parse datetime string '{}': {}",
                        datetime_str,
                        e
                    );
                    None
                }
            }
        } else {
            // Try DateTime as fallback
            let field = exif.get_field(Tag::DateTime, In::PRIMARY);
            if let Some(field) = field {
                let datetime_str = field.display_value().to_string();

                // Try both formats for fallback too
                let parsed =
                    chrono::NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%d %H:%M:%S")
                        .or_else(|_| {
                            chrono::NaiveDateTime::parse_from_str(
                                &datetime_str,
                                "%Y:%m:%d %H:%M:%S",
                            )
                        });

                parsed.ok()
            } else {
                None
            }
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq)]
#[serde(default)]
pub struct SimplifiedExif {
    pub camera_mnf: String,
    pub camera_model: String,
    pub lens_mnf: String,
    pub lens_model: String,
    /// Original lens model from EXIF (before simplification). Used for dynamic toggle.
    pub raw_lens_model: String,
    pub focal: String,
    pub fnumber: String,
    pub exposure: String,
    pub iso_speed: Option<u32>,
    #[serde(with = "datetime_format")]
    pub datetime: Option<chrono::NaiveDateTime>,

    pub make_note: Option<SimplifiedMakeNote>,

    #[serde(skip)]
    pub orientation: image::metadata::Orientation,

    #[serde(skip)]
    datetime_edit_state: crate::image::datetime_edit::DatetimeEditState,
}

impl core::default::Default for SimplifiedExif {
    fn default() -> Self {
        Self {
            camera_mnf: String::new(),
            camera_model: String::new(),
            lens_mnf: String::new(),
            lens_model: String::new(),
            raw_lens_model: String::new(),
            focal: String::new(),
            fnumber: String::new(),
            exposure: String::new(),
            iso_speed: None,
            datetime: None,

            make_note: None,
            orientation: image::metadata::Orientation::NoTransforms,
            datetime_edit_state: crate::image::datetime_edit::DatetimeEditState::new(),
        }
    }
}

/// Remove trash chars from exif string field
pub(crate) fn simplify_exif_string(input: &str) -> String {
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
        let lens_model = value.lens_model();
        Self {
            camera_mnf: value.camera_mnf(),
            camera_model: value.camera_model(),
            lens_mnf: value.lens_mnf(),
            raw_lens_model: lens_model.clone(),
            lens_model,
            focal: value.focal(),
            fnumber: value.fnumber(),
            exposure: value.exposure(),
            iso_speed: value.iso_speed(),
            datetime: value.datetime(),

            make_note: value.make_note(),
            orientation: value.orientation(),
            datetime_edit_state: crate::image::datetime_edit::DatetimeEditState::new(),
        }
    }
}

// UI dependencies - only needed for desktop

use egui::{RichText, TextEdit, TextStyle};

use crate::image::make_note::SimplifiedMakeNote;

impl SimplifiedExif {
    pub fn get_fnumber(&self) -> Option<String> {
        match self.fnumber.as_str() {
            "0" | "" | "F0" | "0.0" | "0.1" | "0.2" | "0.00" => None,
            others => Some(others.to_string()),
        }
    }

    pub fn extract_fnumber_from_lens(&self) -> Option<String> {
        let bytes = self.lens_model.as_bytes();
        let len = bytes.len();

        let mut i = 0;
        while i < len {
            let c = bytes[i];
            if c == b'F' || c == b'f' {
                let mut j = i + 1;
                while j < len && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }

                if j < len && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                    let start = j;
                    while j < len
                        && (bytes[j].is_ascii_digit() || bytes[j] == b'.' || bytes[j] == b'-')
                    {
                        j += 1;
                    }

                    let num = &self.lens_model[start..j];

                    // consider F3.5-5.6
                    let num = num.split('-').next().unwrap_or(num);
                    return Some(num.to_owned());
                }
            }
            i += 1;
        }
        None
    }

    pub fn get_fnumber_alt(&self) -> Option<String> {
        match self.get_fnumber() {
            None => self.extract_fnumber_from_lens(),
            x => x,
        }
    }

    pub fn replace_with_fnumber_alt_when_invalid(&mut self) -> bool {
        if self.get_fnumber().is_none()
            && let Some(x) = self.get_fnumber_alt()
        {
            self.fnumber = x;
            return true;
        }
        false
    }

    /// Replace focal length with 35mm equivalent if available
    /// Returns true if replacement was successful
    pub fn use_35mm_focal_length(&mut self, original_exif: &OriginalExif) -> bool {
        if let Some(focal_35mm) = original_exif.focal_35mm() {
            self.focal = format!("{}", focal_35mm);
            return true;
        }
        false
    }

    /// Strip redundant camera model prefix from lens model.
    /// e.g. "iPhone 17 Pro back triple camera 2.22mm f/2.2" with camera_model "iPhone 17 Pro"
    ///   -> "back triple camera 2.22mm f/2.2"
    pub fn simplify_lens_model_value(lens_model: &str, camera_model: &str) -> String {
        let trimmed_lens = lens_model.trim();
        let trimmed_camera = camera_model.trim();
        if trimmed_camera.is_empty() || trimmed_lens.is_empty() {
            return lens_model.to_string();
        }
        if trimmed_lens
            .to_lowercase()
            .starts_with(&trimmed_camera.to_lowercase())
        {
            let remainder = trimmed_lens[trimmed_camera.len()..].trim();
            if remainder.is_empty() {
                trimmed_lens.to_string()
            } else {
                remainder.to_string()
            }
        } else {
            trimmed_lens.to_string()
        }
    }

    /// Apply lens model simplification, storing original in raw_lens_model.
    pub fn apply_simplify_lens_model(&mut self) {
        if self.raw_lens_model.is_empty() {
            self.raw_lens_model = self.lens_model.clone();
        }
        self.lens_model = Self::simplify_lens_model_value(&self.raw_lens_model, &self.camera_model);
    }

    /// Reapply or restore lens model simplification based on enabled flag.
    /// Respects manual user edits.
    pub fn reapply_simplify_lens_model(&mut self, enabled: bool) {
        // Backwards compat: if raw_lens_model is empty, treat current as raw
        if self.raw_lens_model.is_empty() {
            self.raw_lens_model = self.lens_model.clone();
        }
        let simplified = Self::simplify_lens_model_value(&self.raw_lens_model, &self.camera_model);
        if enabled {
            // Apply simplification if user hasn't manually edited
            if self.lens_model == self.raw_lens_model {
                self.lens_model = simplified;
            }
        } else {
            // Restore raw if user hasn't manually edited the simplified version
            if self.lens_model == simplified {
                self.lens_model = self.raw_lens_model.clone();
            }
        }
    }

    pub fn get_exposure(&self) -> Option<String> {
        match self.exposure.as_str() {
            "" | "0" | "0.0" | "0.00" | "1/0" | "0/1" => None,
            others => Some(others.to_string()),
        }
    }

    pub fn get_iso(&self) -> Option<String> {
        self.iso_speed.map(|x| x.to_string())
    }

    pub fn get_ps_main(&self) -> Option<String> {
        self.make_note.as_ref()?.photo_style.main_name()
    }

    pub fn get_lut_detail(&self) -> Option<String> {
        self.make_note.as_ref()?.photo_style.lut_detail()
    }

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
                && let Ok(v) = iso_str.parse::<u32>()
            {
                self.iso_speed = Some(v);
            }
        } else {
            ui.horizontal(|ui| {
                ui.label(small_text(&self.exposure));
                let iso = self.iso_speed.map_or(String::from("-"), |v| v.to_string());
                ui.label(small_text(&format!("\tISO {iso}")));
            });
        }

        ui.end_row();

        // DateTime
        crate::image::datetime_edit::render_datetime_editor(
            &mut self.datetime_edit_state,
            &mut self.datetime,
            editable,
            ui,
        );
    }

    pub fn is_vertical_rotated(&self) -> bool {
        __is_vertical_rotated(self.orientation)
    }

    fn format_key(
        &self,
        map: &serde_json::Map<String, serde_json::Value>,
        key: impl AsRef<str>,
    ) -> String {
        match key.as_ref() {
            "fnumber" => self.get_fnumber().unwrap_or_default(),
            "exposure" => self.get_exposure().unwrap_or_default(),
            "datetime" => self
                .datetime
                .map(|dt| dt.format("%Y.%m.%d  %H:%M:%S").to_string())
                .unwrap_or_default(),
            // Date parts
            "date" => self
                .datetime
                .map(|dt| dt.format("%Y.%m.%d").to_string())
                .unwrap_or_default(),
            "YYYY" => self
                .datetime
                .map(|dt| dt.format("%Y").to_string())
                .unwrap_or_default(),
            "YY" => self
                .datetime
                .map(|dt| dt.format("%y").to_string())
                .unwrap_or_default(),
            "MM" => self
                .datetime
                .map(|dt| dt.format("%m").to_string())
                .unwrap_or_default(),
            "%M" => self
                .datetime
                .map(|dt| dt.month().to_string())
                .unwrap_or_default(),
            "DD" => self
                .datetime
                .map(|dt| dt.format("%d").to_string())
                .unwrap_or_default(),
            "%D" => self
                .datetime
                .map(|dt| dt.day().to_string())
                .unwrap_or_default(),
            // Time parts
            "time" => self
                .datetime
                .map(|dt| dt.format("%H:%M:%S").to_string())
                .unwrap_or_default(),
            "hh" => self
                .datetime
                .map(|dt| dt.format("%H").to_string())
                .unwrap_or_default(),
            "%h" => self
                .datetime
                .map(|dt| dt.hour().to_string())
                .unwrap_or_default(),
            "mm" => self
                .datetime
                .map(|dt| dt.format("%M").to_string())
                .unwrap_or_default(),
            "%m" => self
                .datetime
                .map(|dt| dt.minute().to_string())
                .unwrap_or_default(),
            "ss" => self
                .datetime
                .map(|dt| dt.format("%S").to_string())
                .unwrap_or_default(),
            "%s" => self
                .datetime
                .map(|dt| dt.second().to_string())
                .unwrap_or_default(),
            // Other fields
            "photo_style" => self.get_ps_main().unwrap_or_default(),
            "lut_detail" => self.get_lut_detail().unwrap_or_default(),
            // default
            default => match map.get(default) {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(serde_json::Value::Number(n)) => n.to_string(),
                Some(serde_json::Value::Null) => "".to_string(),
                _ => "".to_string(),
            },
        }
    }

    pub fn format_custom(&self, fmt: impl AsRef<str>) -> String {
        let fmt = fmt.as_ref();
        let json_value = match serde_json::to_value(self) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to serialize EXIF for format_custom: {}", e);
                return String::new();
            }
        };
        let Some(map) = json_value.as_object() else {
            log::error!("EXIF serialization did not produce a JSON object");
            return String::new();
        };

        // Debug logging to see what EXIF data we have
        log::debug!("format_custom called with template: '{}'", fmt);
        log::debug!(
            "EXIF data: camera_mnf='{}', camera_model='{}', lens_mnf='{}', lens_model='{}', focal='{}', iso_speed={:?}",
            self.camera_mnf,
            self.camera_model,
            self.lens_mnf,
            self.lens_model,
            self.focal,
            self.iso_speed
        );

        let mut result = String::new();
        let mut chars = fmt.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                let mut key = String::new();
                while let Some(&next_ch) = chars.peek() {
                    chars.next();
                    if next_ch == '}' {
                        break;
                    }
                    key.push(next_ch);
                }

                let val = self.format_key(map, key);
                result.push_str(&val);
            } else if ch == '[' {
                let mut block_content = String::new();
                for c in chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                    block_content.push(c);
                }

                // Extract variable values from block content to check if any are non-empty
                let mut has_content = false;
                let mut temp_chars = block_content.chars().peekable();
                while let Some(c) = temp_chars.next() {
                    if c == '{' {
                        let mut key = String::new();
                        while let Some(&next_ch) = temp_chars.peek() {
                            temp_chars.next();
                            if next_ch == '}' {
                                break;
                            }
                            key.push(next_ch);
                        }
                        let val = self.format_key(map, key);
                        if !val.is_empty() {
                            has_content = true;
                            break;
                        }
                    }
                }

                // Only include block if it contains non-empty variable values
                if has_content {
                    let expanded = self.format_custom(block_content.as_str());
                    result.push_str(&expanded);
                }
            } else {
                result.push(ch);
            }
        }

        result
    }
}

/// Extract verbose EXIF data including all fields and maker notes
/// Returns a JSON string with all EXIF fields
#[cfg(any(
    target_os = "ios",
    target_os = "macos",
    feature = "ios_integration",
    feature = "android_integration"
))]
#[allow(dead_code)] // Called via FFI from ffi_apple.rs
pub fn extract_verbose_exif(path: &str) -> Result<String, String> {
    use std::io::BufReader;

    // Read EXIF data from file
    let file =
        std::fs::File::open(path).map_err(|e| format!("Failed to open image file: {}", e))?;

    let mut bufreader = BufReader::new(&file);
    let exifreader = exif::Reader::new()
        .read_from_container(&mut bufreader)
        .map_err(|e| format!("Failed to read EXIF data: {}", e))?;

    // Check maker note vendor before processing fields
    log::info!("📝 Checking MakerNote vendor...");
    match exifreader.maker_note_vendor() {
        Ok(vendor) => log::info!("[OKAY] MakerNote vendor detected: {:?}", vendor),
        Err(e) => log::info!("[WARN] No MakerNote vendor detected: {:?}", e),
    }

    // Collect all EXIF fields into a vector of objects
    let mut fields = Vec::new();
    let mut maker_note_tag_found = false;
    let mut ifd2_count = 0;

    for field in exifreader.fields() {
        let tag_name = format!("{}", field.tag);

        // Format IFD name - prefix maker note IFDs with "MakeNote-"
        // Standard IFDs are In(0) and In(1), maker notes are typically In(2+)
        let ifd_base = format!("{:?}", field.ifd_num);
        let is_maker_note_ifd = matches!(field.ifd_num, exif::In(n) if n >= 2);
        if is_maker_note_ifd {
            ifd2_count += 1;
        }
        let ifd_name = if is_maker_note_ifd {
            let formatted = format!("MakeNote-{}", ifd_base);
            log::info!(
                "🏷️ Field '{}' has IFD {:?}, formatted as '{}'",
                tag_name,
                field.ifd_num,
                formatted
            );
            formatted
        } else {
            ifd_base
        };

        // Get display value for this field
        let value_display = field.display_value().to_string();

        // Handle MakerNote tag specially
        if field.tag == exif::Tag::MakerNote {
            maker_note_tag_found = true;
            log::info!("Found MakerNote tag in IFD {:?}", field.ifd_num);

            // Force MakeNote- prefix for the MakerNote tag IFD
            let maker_note_ifd = format!("MakeNote-{:?}", field.ifd_num);

            // Show vendor if detected, otherwise show raw data
            let maker_note_value = if let Ok(vendor) = exifreader.maker_note_vendor() {
                log::info!("Vendor detected: {:?}", vendor);
                format!("{:?} (Raw data: {} bytes)", vendor, value_display.len())
            } else {
                format!("Unknown vendor (Raw data: {} bytes)", value_display.len())
            };

            let vendor_field = serde_json::json!({
                "tag": "MakerNote",
                "ifd": maker_note_ifd,
                "value": maker_note_value,
            });
            fields.push(vendor_field);
            continue;
        }

        // Create a field object
        let field_obj = serde_json::json!({
            "tag": tag_name,
            "ifd": ifd_name,
            "value": value_display,
        });

        fields.push(field_obj);
    }

    // Extract ALL maker note fields using the maker_note_fields() iterator
    if let Ok(vendor) = exifreader.maker_note_vendor() {
        log::info!("Extracting ALL maker note fields for vendor: {:?}", vendor);

        for maker_field in exifreader.maker_note_fields() {
            let tag_name = format!("{}", maker_field.tag);
            let value_display = maker_field.display_value().to_string();
            let ifd_name = format!("MakeNote-{:?}", vendor);

            let field_obj = serde_json::json!({
                "tag": tag_name,
                "ifd": ifd_name,
                "value": value_display,
            });
            fields.push(field_obj);
            ifd2_count += 1;
            log::debug!(
                "MakerNote field: {} = {}",
                tag_name,
                value_display.chars().take(50).collect::<String>()
            );
        }

        log::info!("Extracted {} maker note fields", ifd2_count);
    }

    // Summary logging
    log::info!("EXIF Summary:");
    log::info!("  Total fields: {}", fields.len());
    log::info!("  MakerNote tag found: {}", maker_note_tag_found);
    log::info!("  Fields with IFD >= 2: {}", ifd2_count);

    // Create JSON object with all fields
    let json = serde_json::json!({
        "fields": fields,
    });

    // Convert to JSON string
    serde_json::to_string_pretty(&json).map_err(|e| format!("Failed to serialize EXIF data: {}", e))
}

#[allow(dead_code)]
fn hex_dump(s: &str) {
    for (i, b) in s.as_bytes().iter().enumerate() {
        print!("{b:02X} ");
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::datetime_edit::DatetimeEditState;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_exif_datetime_parsing_with_dashes() {
        // Test YYYY-MM-DD format (your camera's format)
        let exif_data = "2025-11-30 21:41:15";
        let parsed = chrono::NaiveDateTime::parse_from_str(exif_data, "%Y-%m-%d %H:%M:%S");
        assert!(parsed.is_ok());
        let dt = parsed.unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 11);
        assert_eq!(dt.day(), 30);
        assert_eq!(dt.hour(), 21);
        assert_eq!(dt.minute(), 41);
        assert_eq!(dt.second(), 15);
    }

    #[test]
    fn test_exif_datetime_parsing_with_colons() {
        // Test YYYY:MM:DD format (EXIF standard)
        let exif_data = "2025:11:30 21:41:15";
        let parsed = chrono::NaiveDateTime::parse_from_str(exif_data, "%Y:%m:%d %H:%M:%S");
        assert!(parsed.is_ok());
        let dt = parsed.unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 11);
        assert_eq!(dt.day(), 30);
        assert_eq!(dt.hour(), 21);
        assert_eq!(dt.minute(), 41);
        assert_eq!(dt.second(), 15);
    }

    #[test]
    fn test_partial_hinting_12_digits_with_padding() {
        // Test: "20240409144137" (12 digits) → minutes are at positions 10-11
        // With 12+ digits (substantial input), if we have both minutes digits, show them
        let cleaned = "20240409144137";
        let n = cleaned.len();

        // Get minute at positions 10-11
        let minute_text = if n >= 12 {
            cleaned[10..12].to_string()
        } else if n >= 11 {
            if n >= 12 {
                format!("0{}", &cleaned[10..11])
            } else {
                cleaned[10..11].to_string()
            }
        } else if n > 10 {
            cleaned[10..].to_string()
        } else {
            "mm".to_string()
        };

        // With 12 digits (20240409144137), minutes at 10-11 are "41"
        assert_eq!(minute_text, "41");
    }

    #[test]
    fn test_partial_hinting_13_digits_no_padding() {
        // Test: "2024040914415" (13 digits) → second is at position 12 (single digit)
        // With 13 digits (not yet complete 14), we have single second digit "5"
        let cleaned = "2024040914415";
        let n = cleaned.len();

        // Get second at position 12 (single digit with 13 total)
        let second_text = if n >= 14 {
            cleaned[12..14].to_string()
        } else if n >= 13 {
            if n >= 12 {
                format!("0{}", &cleaned[12..13])
            } else {
                cleaned[12..13].to_string()
            }
        } else if n > 12 {
            cleaned[12..].to_string()
        } else {
            "ss".to_string()
        };

        // With 13 digits, we enter the n >= 13 branch, which tries to format with padding
        // Since n >= 12, it formats "0" + "5" = "05"
        assert_eq!(second_text, "05");
    }

    #[test]
    fn test_partial_hinting_14_digits_complete() {
        // Test: "202404091441537" (14 digits) → "2024.04.09  14:41:53" (complete)
        let cleaned = "202404091441537";
        let n = cleaned.len();

        // Get second with complete input
        let second_text = if n >= 14 {
            cleaned[12..14].to_string()
        } else if n >= 13 {
            if n >= 12 {
                format!("0{}", &cleaned[12..13])
            } else {
                cleaned[12..13].to_string()
            }
        } else if n > 12 {
            cleaned[12..].to_string()
        } else {
            "ss".to_string()
        };

        assert_eq!(second_text, "53");
    }

    #[test]
    fn test_color_validation_month() {
        // Test month validation
        let valid_month = "11";
        let invalid_month = "55";

        let valid: u32 = valid_month.parse().unwrap();
        let invalid: u32 = invalid_month.parse().unwrap();

        assert!((1..=12).contains(&valid));
        assert!(!(1..=12).contains(&invalid));
    }

    #[test]
    fn test_color_validation_hour() {
        // Test hour validation
        let valid_hour = "23";
        let invalid_hour = "25";

        let valid: u32 = valid_hour.parse().unwrap();
        let invalid: u32 = invalid_hour.parse().unwrap();

        assert!((0..=23).contains(&valid));
        assert!(!(0..=23).contains(&invalid));
    }

    #[test]
    fn test_datetime_serialization() {
        // Test serialization/deserialization
        use serde_json;

        let dt = chrono::NaiveDateTime::parse_from_str("2025-11-30 21:41:15", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let exif = SimplifiedExif {
            camera_mnf: "Test".to_string(),
            camera_model: "Model".to_string(),
            lens_mnf: "Lens".to_string(),
            lens_model: "LensModel".to_string(),
            raw_lens_model: "LensModel".to_string(),
            focal: "50mm".to_string(),
            fnumber: "2.8".to_string(),
            exposure: "1/60".to_string(),
            iso_speed: Some(100),
            datetime: Some(dt),
            make_note: None,
            orientation: image::metadata::Orientation::NoTransforms,
            datetime_edit_state: crate::image::datetime_edit::DatetimeEditState::new(),
        };

        // Serialize
        let json = serde_json::to_string(&exif).unwrap();
        assert!(json.contains("2025-11-30 21:41:15"));

        // Deserialize
        let deserialized: SimplifiedExif = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.datetime, Some(dt));
    }

    #[test]
    fn test_datetime_format_custom() {
        // Test format_custom with {datetime} variable
        let dt = chrono::NaiveDateTime::parse_from_str("2025-11-30 21:41:15", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let exif = SimplifiedExif {
            camera_mnf: "Test".to_string(),
            camera_model: "Model".to_string(),
            lens_mnf: "Lens".to_string(),
            lens_model: "LensModel".to_string(),
            raw_lens_model: "LensModel".to_string(),
            focal: "50mm".to_string(),
            fnumber: "2.8".to_string(),
            exposure: "1/60".to_string(),
            iso_speed: Some(100),
            datetime: Some(dt),
            make_note: None,
            orientation: image::metadata::Orientation::NoTransforms,
            datetime_edit_state: DatetimeEditState::new(),
        };

        // Test {datetime} variable
        let result = exif.format_custom("{datetime}");
        assert_eq!(result, "2025.11.30  21:41:15");

        // Test mixed variables
        let result = exif.format_custom("Shot on {datetime} at {fnumber}");
        assert_eq!(result, "Shot on 2025.11.30  21:41:15 at 2.8");
    }

    #[test]
    fn test_new_datetime_placeholders_date() {
        // Test new date placeholders
        let dt = chrono::NaiveDateTime::parse_from_str("2025-11-30 21:41:15", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let exif = SimplifiedExif {
            camera_mnf: "Test".to_string(),
            camera_model: "Model".to_string(),
            lens_mnf: "Lens".to_string(),
            lens_model: "LensModel".to_string(),
            raw_lens_model: "LensModel".to_string(),
            focal: "50mm".to_string(),
            fnumber: "2.8".to_string(),
            exposure: "1/60".to_string(),
            iso_speed: Some(100),
            datetime: Some(dt),
            make_note: None,
            orientation: image::metadata::Orientation::NoTransforms,
            datetime_edit_state: DatetimeEditState::new(),
        };

        // Test {date}
        let result = exif.format_custom("{date}");
        assert_eq!(result, "2025.11.30");

        // Test {YYYY}
        let result = exif.format_custom("{YYYY}");
        assert_eq!(result, "2025");

        // Test {YY}
        let result = exif.format_custom("{YY}");
        assert_eq!(result, "25");

        // Test {MM} (month with leading zero)
        let result = exif.format_custom("{MM}");
        assert_eq!(result, "11");

        // Test {%M} (month without leading zero)
        let result = exif.format_custom("{%M}");
        assert_eq!(result, "11");

        // Test month 9 with leading zero
        let dt = chrono::NaiveDateTime::parse_from_str("2025-09-15 21:41:15", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let exif = SimplifiedExif {
            camera_mnf: "Test".to_string(),
            camera_model: "Model".to_string(),
            lens_mnf: "Lens".to_string(),
            lens_model: "LensModel".to_string(),
            raw_lens_model: "LensModel".to_string(),
            focal: "50mm".to_string(),
            fnumber: "2.8".to_string(),
            exposure: "1/60".to_string(),
            iso_speed: Some(100),
            datetime: Some(dt),
            make_note: None,
            orientation: image::metadata::Orientation::NoTransforms,
            datetime_edit_state: DatetimeEditState::new(),
        };

        let result = exif.format_custom("{MM}");
        assert_eq!(result, "09");

        let result = exif.format_custom("{%M}");
        assert_eq!(result, "9");
    }

    #[test]
    fn test_new_datetime_placeholders_time() {
        // Test new time placeholders
        let dt = chrono::NaiveDateTime::parse_from_str("2025-11-30 21:41:15", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let exif = SimplifiedExif {
            camera_mnf: "Test".to_string(),
            camera_model: "Model".to_string(),
            lens_mnf: "Lens".to_string(),
            lens_model: "LensModel".to_string(),
            raw_lens_model: "LensModel".to_string(),
            focal: "50mm".to_string(),
            fnumber: "2.8".to_string(),
            exposure: "1/60".to_string(),
            iso_speed: Some(100),
            datetime: Some(dt),
            make_note: None,
            orientation: image::metadata::Orientation::NoTransforms,
            datetime_edit_state: DatetimeEditState::new(),
        };

        // Test {time}
        let result = exif.format_custom("{time}");
        assert_eq!(result, "21:41:15");

        // Test {hh} (hour with leading zero)
        let result = exif.format_custom("{hh}");
        assert_eq!(result, "21");

        // Test {%h} (hour without leading zero)
        let result = exif.format_custom("{%h}");
        assert_eq!(result, "21");

        // Test {mm} (minute with leading zero)
        let result = exif.format_custom("{mm}");
        assert_eq!(result, "41");

        // Test {%m} (minute without leading zero)
        let result = exif.format_custom("{%m}");
        assert_eq!(result, "41");

        // Test {ss} (second with leading zero)
        let result = exif.format_custom("{ss}");
        assert_eq!(result, "15");

        // Test {%s} (second without leading zero)
        let result = exif.format_custom("{%s}");
        assert_eq!(result, "15");
    }

    #[test]
    fn test_new_datetime_placeholders_mixed() {
        // Test mixed format with all new placeholders
        let dt = chrono::NaiveDateTime::parse_from_str("2025-11-30 09:05:07", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let exif = SimplifiedExif {
            camera_mnf: "Test".to_string(),
            camera_model: "Model".to_string(),
            lens_mnf: "Lens".to_string(),
            lens_model: "LensModel".to_string(),
            raw_lens_model: "LensModel".to_string(),
            focal: "50mm".to_string(),
            fnumber: "2.8".to_string(),
            exposure: "1/60".to_string(),
            iso_speed: Some(100),
            datetime: Some(dt),
            make_note: None,
            orientation: image::metadata::Orientation::NoTransforms,
            datetime_edit_state: DatetimeEditState::new(),
        };

        // Test custom format with leading zeros
        let result = exif.format_custom("{YYYY}.{MM}.{DD} {hh}:{mm}:{ss}");
        assert_eq!(result, "2025.11.30 09:05:07");

        // Test custom format without leading zeros
        let result = exif.format_custom("{YY}.{%M}.{%D} {%h}:{%m}:{%s}");
        assert_eq!(result, "25.11.30 9:5:7");

        // Test mixed format
        let result = exif.format_custom("{date} at {time}");
        assert_eq!(result, "2025.11.30 at 09:05:07");
    }
}

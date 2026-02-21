/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use exif::StructuredMakerNoteData;

/// Extract color temperature (in Kelvin) from vendor-specific MakerNote fields.
/// Returns `None` if the vendor is unsupported or the value is zero/absent.
pub(crate) fn color_temperature_from_exif(exif: &exif::Exif) -> Option<u16> {
    match exif.maker_note_vendor() {
        Ok(exif::MakerNoteVendor::Canon) => exif
            .get_maker_note_field(&exif::canon::tags::ColorTemperature)
            .and_then(|v| v.value.get_uint(0))
            .filter(|&v| v > 0)
            .map(|v| v as u16),
        Ok(exif::MakerNoteVendor::Nikon) => exif
            .get_maker_note_field(&exif::nikon::tags::ColorTemperatureAuto)
            .and_then(|v| v.value.get_uint(0))
            .filter(|&v| v > 0)
            .map(|v| v as u16),
        Ok(exif::MakerNoteVendor::Panasonic) => exif
            .get_maker_note_field(&exif::panasonic::tags::ColorTempKelvin)
            .and_then(|v| v.value.get_uint(0))
            .filter(|&v| v > 0)
            .map(|v| v as u16),
        Ok(exif::MakerNoteVendor::Apple) => exif
            .get_maker_note_field(&exif::apple::tags::ColorTemperature)
            .and_then(|v| v.value.get_uint(0))
            .filter(|&v| v > 0)
            .map(|v| v as u16),
        Ok(exif::MakerNoteVendor::Sony) => exif
            .get_maker_note_field(&exif::sony::tags::ColorTemperature)
            .and_then(|v| v.value.get_uint(0))
            .filter(|&v| v > 0)
            .map(|v| v as u16),
        Ok(exif::MakerNoteVendor::Fujifilm) => exif
            .get_maker_note_field(&exif::fujifilm::tags::ColorTemperature)
            .and_then(|v| v.value.get_uint(0))
            .filter(|&v| v > 0)
            .map(|v| v as u16),
        Ok(exif::MakerNoteVendor::Samsung) => exif
            .get_maker_note_field(&exif::samsung::tags::ColorTemperature)
            .and_then(|v| v.value.get_uint(0))
            .filter(|&v| v > 0)
            .map(|v| v as u16),
        Ok(exif::MakerNoteVendor::Pentax) | Ok(exif::MakerNoteVendor::Ricoh) => exif
            .get_maker_note_field(&exif::make_note::pentax::tags::ColorTemperature)
            .and_then(|v| v.value.get_uint(0))
            .filter(|&v| v > 0)
            .map(|v| v as u16),
        // Olympus/OMSystem: WhiteBalanceTemperature2 (0x0501) lives in the CameraSettings
        // subdirectory, parsed under the OlympusCameraSettings vendor key. Construct the
        // MakerTag directly since camera_settings::tags is pub(crate) in the exif crate.
        // Value is 0 when WB is set to Auto.
        Ok(exif::MakerNoteVendor::Olympus) | Ok(exif::MakerNoteVendor::OMSystem) => {
            let tag = exif::make_note::maker_tag::MakerTag::new(
                exif::MakerNoteVendor::OlympusCameraSettings,
                0x0501,
            );
            exif.get_maker_note_field(&tag)
                .and_then(|v| v.value.get_uint(0))
                .filter(|&v| v > 0)
                .map(|v| v as u16)
        }
        // Leica: no leica.rs MakerNote module; no Kelvin field parsed.
        // Sigma: WhiteBalance is a string enum (e.g. "Auto(Natural)"), no K value.
        _ => None,
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Default, Debug)]
pub enum MakePhotoStyle {
    Nikon {
        main: String, // e.g) VitalityFilm_Pmango
    },
    Panasonic {
        main: String,       // e.g) NostalgicKintex
        primary: String,    // e.g) KintexYellow33.CUBE (lumix only)
        primary_gain: u8,   // e.g) 64 (lumix only)
        secondary: String,  // e.g) NostalgicFLAT.CUBE (lumix only)
        secondary_gain: u8, // e.g) 20 (lumix only)
    },
    Sony {
        main: String, // e.g) Standard, Vivid, Portrait, etc.
    },
    #[default]
    None,
}

impl MakePhotoStyle {
    pub fn from_exif(exif: &exif::Exif) -> MakePhotoStyle {
        let le = Some(exif.little_endian());

        match exif.maker_note_vendor() {
            Ok(exif::MakerNoteVendor::Nikon) => {
                // PictureControlData
                if let Some(main) = exif
                    .get_maker_note_field(&exif::nikon::tags::PictureControlData)
                    .and_then(|v| exif::nikon::NikonPictureControl::from_value(&v.value, le))
                    .map(|d| d.name)
                {
                    MakePhotoStyle::Nikon { main }
                }
                // PictureControlData2
                else if let Some(main) = exif
                    .get_maker_note_field(&exif::nikon::tags::PictureControlData2)
                    .and_then(|v| exif::nikon::NikonPictureControl::from_value(&v.value, le))
                    .map(|d| d.name)
                {
                    MakePhotoStyle::Nikon { main }
                } else {
                    MakePhotoStyle::None
                }
            }
            Ok(exif::MakerNoteVendor::Panasonic) => {
                // Looking for Panasonic PhotoStyle
                let main = match exif.get_maker_note_field(&exif::panasonic::tags::PhotoStyleName) {
                    Some(v) => v.display_value().to_string(),
                    None => return MakePhotoStyle::None,
                };

                MakePhotoStyle::Panasonic {
                    main,
                    primary: exif
                        .get_maker_note_field(&exif::panasonic::tags::LutPrimaryFile)
                        .map(|v| v.display_value().to_string())
                        .unwrap_or_default(),
                    primary_gain: exif
                        .get_maker_note_field(&exif::panasonic::tags::LutPrimaryGain)
                        .and_then(|v| v.value.get_uint(0))
                        .unwrap_or(0) as u8,
                    secondary: exif
                        .get_maker_note_field(&exif::panasonic::tags::LutSecondaryFile)
                        .map(|v| v.display_value().to_string())
                        .unwrap_or_default(),
                    secondary_gain: exif
                        .get_maker_note_field(&exif::panasonic::tags::LutSecondaryGain)
                        .and_then(|v| v.value.get_uint(0))
                        .unwrap_or(0) as u8,
                }
            }
            Ok(exif::MakerNoteVendor::Sony) => {
                // Looking for Sony Tag9416 with CreativeStyle
                if let Some(main) = exif
                    .get_maker_note_field(&exif::sony::tags::Sony_0x9416)
                    .and_then(|v| exif::sony::SonyTag9416::from_value(&v.value, le))
                    .map(|d| d.creative_style.to_string())
                {
                    MakePhotoStyle::Sony { main }
                } else {
                    MakePhotoStyle::None
                }
            }
            _ => MakePhotoStyle::None,
        }
    }

    pub(crate) fn main_name(&self) -> Option<String> {
        match &self {
            MakePhotoStyle::Nikon { main, .. }
            | MakePhotoStyle::Panasonic { main, .. }
            | MakePhotoStyle::Sony { main, .. } => Some(main.clone()),

            _ => None,
        }
    }

    pub(crate) fn lut_detail(&self) -> Option<String> {
        match self {
            MakePhotoStyle::Panasonic {
                primary, secondary, ..
            } => {
                if secondary.is_empty() {
                    Some(primary.clone())
                } else {
                    Some(format!("{primary} + {secondary}"))
                }
            }
            _ => None,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Debug)]
pub struct SimplifiedMakeNote {
    pub photo_style: MakePhotoStyle,
    /// Color temperature in Kelvin extracted from MakerNote, if available.
    pub color_temperature: Option<u16>,
}

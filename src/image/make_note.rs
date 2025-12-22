/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

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
    #[default]
    None,
}

impl MakePhotoStyle {
    pub fn from_exif(exif: &exif::Exif) -> MakePhotoStyle {
        match exif.maker_note_vendor() {
            Ok(exif::MakerNoteVendor::Nikon) => {
                // PictureControlData
                if let Some(main) = exif
                    .get_maker_note_field(&exif::nikon::tags::PictureControlData)
                    .and_then(|v| exif::nikon::NikonPictureControl::from_value(&v.value))
                    .and_then(|d| Some(d.name))
                {
                    MakePhotoStyle::Nikon { main }
                }
                // PictureControlData2
                else if let Some(main) = exif
                    .get_maker_note_field(&exif::nikon::tags::PictureControlData2)
                    .and_then(|v| exif::nikon::NikonPictureControl::from_value(&v.value))
                    .and_then(|d| Some(d.name))
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
            _ => MakePhotoStyle::None,
        }
    }

    pub(crate) fn main_name(&self) -> Option<String> {
        match &self {
            MakePhotoStyle::Nikon { main, .. } | MakePhotoStyle::Panasonic { main, .. } => {
                Some(main.clone())
            }

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
    // todo! - add more
    pub photo_style: MakePhotoStyle,
}

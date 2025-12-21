/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use exif::make_note::{
    maker_tag::{MakerNoteField, MakerNoteVendor},
    nikon, panasonic,
};

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
    pub(crate) fn main_name(&self) -> Option<String> {
        match &self {
            MakePhotoStyle::Nikon { main } | MakePhotoStyle::Panasonic { main, .. } => {
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

fn take_make_photo_style(
    vec_mnf: &Vec<MakerNoteField>,
    vendor: MakerNoteVendor,
    _tiff_offset: u32,
) -> Result<MakePhotoStyle, ()> {
    match vendor {
        exif::make_note::maker_tag::MakerNoteVendor::Panasonic => {
            // Panasonic specific tags
            let (mut main, mut primary, mut secondary) =
                (String::new(), String::new(), String::new());
            let (mut primary_gain, mut secondary_gain) = (0, 0);

            for item in vec_mnf {
                match item.tag {
                    panasonic::tags::PhotoStyleName => {
                        main = item.display_value().to_string();
                    }
                    panasonic::tags::LutPrimaryFile => {
                        primary = item.display_value().to_string();
                    }
                    panasonic::tags::LutPrimaryGain => {
                        primary_gain = item.value.get_uint(0).unwrap_or(0) as _;
                    }
                    panasonic::tags::LutSecondaryFile => {
                        secondary = item.display_value().to_string();
                    }
                    panasonic::tags::LutSecondaryGain => {
                        secondary_gain = item.value.get_uint(0).unwrap_or(0) as _;
                    }
                    _ => {}
                }
            }

            Ok(MakePhotoStyle::Panasonic {
                main,
                primary,
                primary_gain,
                secondary,
                secondary_gain,
            })
        }
        exif::make_note::maker_tag::MakerNoteVendor::Nikon => {
            // Nikon specific tags - need to parse Picture Control Data
            for item in vec_mnf {
                match item.tag {
                    nikon::tags::PictureControlData | nikon::tags::PictureControlData2 => {
                        // Picture Control Data or Picture Control Data 2
                        log::info!(
                            "Found Picture Control Data (tag: 0x{:04x})",
                            item.tag.number
                        );

                        // Extract the raw data
                        if let exif::Value::Undefined(data, _) = &item.value {
                            // https://github.com/exiftool/exiftool/blob/master/lib/Image/ExifTool/Nikon.pm#L2039-L2066

                            if data.len() > 28 {
                                // picture control name
                                let name_start = match data[1] {
                                    b'3' => 8,
                                    _ => 4,
                                };

                                let name_arr = &data[name_start..name_start + 20];
                                let end = name_arr
                                    .iter()
                                    .position(|&b| b == 0)
                                    .unwrap_or(name_arr.len());

                                let name =
                                    str::from_utf8(&name_arr[..end]).unwrap_or("").to_string();

                                return Ok(MakePhotoStyle::Nikon { main: name });
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(MakePhotoStyle::None)
        }
        _ => Ok(MakePhotoStyle::None),
    }
}

/// Parse Make Note with simplified form
/// ```rs
/// let mut value = self
///     .0
///     .as_ref()
///     .and_then(|exif| exif.get_field(Tag::MakerNote, In::PRIMARY))
///     .and_then(|field| {
///         let a = field.value.clone();
///         if let exif::Value::Undefined(vector, _) = a {
///             crate::dump!(vector);
///         }
///         Some(field.value.clone())
///     });
///
/// if let Some(exif::Value::Undefined(ref mut vector, offset)) = value {
///     crate::image::make_note::parse_make_note(vector, offset).ok()
/// } else {
///     None
/// }
/// ```
pub fn parse_make_note(
    make_note: &mut Vec<u8>,
    tiff_offset: u32,
    make: Option<&str>,
) -> Result<SimplifiedMakeNote, ()> {
    crate::dump!(make_note);

    if let Ok((ret, vendor, _)) =
        exif::make_note::parse_make_note_with_vendor(make_note, tiff_offset, make)
    {
        let photo_style = take_make_photo_style(&ret, vendor, tiff_offset)?;
        log::info!("make_note: {photo_style:?}");
        Ok(SimplifiedMakeNote { photo_style })
    } else {
        log::info!("Failed to parse make_note");
        Err(())
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Custom serialization for egui::Color32
//!
//! egui::Color32 doesn't implement Serialize/Deserialize by default,
//! so we provide custom implementations for JSON interop.

use serde::de::{Error, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Serialize egui::Color32 as [r, g, b, a] array
pub fn serialize_color32<S>(color: &egui::Color32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(4))?;
    seq.serialize_element(&color.r())?;
    seq.serialize_element(&color.g())?;
    seq.serialize_element(&color.b())?;
    seq.serialize_element(&color.a())?;
    seq.end()
}

/// Deserialize [r, g, b, a] array to egui::Color32
pub fn deserialize_color32<'de, D>(deserializer: D) -> Result<egui::Color32, D::Error>
where
    D: Deserializer<'de>,
{
    struct ColorVisitor;

    impl<'de> Visitor<'de> for ColorVisitor {
        type Value = egui::Color32;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a JSON array of 4 integers [r, g, b, a]")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let r = seq
                .next_element::<u8>()?
                .ok_or_else(|| Error::custom("missing red value"))?;
            let g = seq
                .next_element::<u8>()?
                .ok_or_else(|| Error::custom("missing green value"))?;
            let b = seq
                .next_element::<u8>()?
                .ok_or_else(|| Error::custom("missing blue value"))?;
            let a = seq
                .next_element::<u8>()?
                .ok_or_else(|| Error::custom("missing alpha value"))?;

            if seq.next_element::<u8>()?.is_some() {
                return Err(Error::custom("expected exactly 4 values for color"));
            }

            Ok(egui::Color32::from_rgba_unmultiplied(r, g, b, a))
        }
    }

    deserializer.deserialize_seq(ColorVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_serialization() {
        let color = egui::Color32::from_rgba_unmultiplied(255, 128, 64, 192);
        let json = serde_json::to_string(&color).unwrap();
        assert_eq!(json, "[255,128,64,192]");
    }

    #[test]
    fn test_color_deserialization() {
        let json = "[255,128,64,192]";
        let color: egui::Color32 = serde_json::from_str(json).unwrap();
        assert_eq!(color.r(), 255);
        assert_eq!(color.g(), 128);
        assert_eq!(color.b(), 64);
        assert_eq!(color.a(), 192);
    }
}

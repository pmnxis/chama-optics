/*
 * SPDX-FileCopyrightText: Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use crate::art::types::*;

impl ArtAsset {
    pub fn is_match(&self, (mnf, model): (&str, &str)) -> bool {
        let mnf_match = if !self.mnf.is_empty() && !mnf.is_empty() {
            mnf.to_ascii_lowercase().contains(self.mnf)
        } else {
            false
        };

        let model_match = if !self.model.is_empty() && !model.is_empty() {
            model.to_ascii_lowercase().contains(self.model)
        } else {
            false
        };

        if self.mnf_model_rel == MnfRelation::Any {
            mnf_match || model_match
        } else if self.mnf_model_rel == MnfRelation::Both {
            mnf_match && model_match
        } else {
            false
        }
    }

    pub fn get_match_arr(
        arr: &'static [ArtAsset],
        (mnf, model): (&str, &str),
    ) -> Option<&'static ArtAsset> {
        arr.iter().find(|asset| asset.is_match((mnf, model)))
    }

    pub fn draw(&self, target_height: u32) -> Result<image::DynamicImage, image::ImageError> {
        use image::{DynamicImage, ImageBuffer, Rgba};

        use resvg::usvg;
        use tiny_skia::Pixmap;

        let svg_bytes = self.data;

        // parse svg
        let opt = usvg::Options::default();
        let svg_str = std::str::from_utf8(svg_bytes).unwrap();
        let rtree = usvg::Tree::from_str(svg_str, &opt).map_err(|e| {
            image::ImageError::Decoding(image::error::DecodingError::new(
                image::error::ImageFormatHint::PathExtension(std::path::PathBuf::from(self.key)),
                e,
            ))
        })?;

        // get svg original size (viewBox)
        let svg_size = rtree.size();
        let orig_w = svg_size.width();
        let orig_h = svg_size.height();

        if orig_w <= 0.0 || orig_h <= 0.0 {
            return Err(image::ImageError::Decoding(
                image::error::DecodingError::new(
                    image::error::ImageFormatHint::PathExtension(std::path::PathBuf::from(
                        self.key,
                    )),
                    format!("Invalid SVG dimensions: {orig_w}x{orig_h}"),
                ),
            ));
        }

        // compute scale factor to match target height
        let scale = target_height as f32 / orig_h;
        let width_px = (orig_w * scale).ceil() as u32;
        let height_px = (orig_h * scale).ceil() as u32;

        // Create Pixmap and render
        let mut pixmap = Pixmap::new(width_px, height_px).ok_or(image::ImageError::Decoding(
            image::error::DecodingError::new(
                image::error::ImageFormatHint::PathExtension(std::path::PathBuf::from(self.key)),
                format!("Failed to create pixmap: {orig_w}x{orig_h}"),
            ),
        ))?;

        let transform = usvg::Transform::from_scale(scale, scale);
        resvg::render(&rtree, transform, &mut pixmap.as_mut());

        Ok(DynamicImage::ImageRgba8(
            ImageBuffer::<Rgba<u8>, _>::from_raw(width_px, height_px, pixmap.take()).ok_or({
                image::ImageError::Decoding(image::error::DecodingError::new(
                    image::error::ImageFormatHint::PathExtension(std::path::PathBuf::from(
                        self.key,
                    )),
                    format!("Failed to move rendered image to dynamic image"),
                ))
            })?,
        ))
    }
}

fn __fmt(value: &ArtAsset, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // show only 16bytes
    let preview_len = value.data.len().min(16);
    let ascii_preview: String = value.data[..preview_len]
        .iter()
        .map(|&b| {
            if b.is_ascii_graphic() || b == b' ' {
                b as char
            } else {
                '.'
            }
        })
        .collect();

    write!(
        f,
        "ArtAsset {{ key: {}, data: \"{}\" ({} bytes), color_type: {:?}, fill_ops: {:?}, mnf: {}, model: {}, mnf_model_rel: {:?} }}",
        value.key,
        ascii_preview,
        value.data.len(),
        value.color_type,
        value.fill_ops,
        value.mnf,
        value.model,
        value.mnf_model_rel,
    )
}

impl std::fmt::Display for ArtAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        __fmt(self, f)
    }
}

impl std::fmt::Debug for ArtAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        __fmt(self, f)
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! collection of themes

pub(crate) mod film;

pub trait Theme {
    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError>;

    fn ui_config(&mut self, ui: &mut egui::Ui);
}

#[macro_export]
macro_rules! px_w {
    ($value:expr, $img_width:expr) => {
        ($value as f32) * ($img_width as f32 / 4000.0)
    };
}

#[macro_export]
macro_rules! pxscale_w {
    ($value:expr, $img_width:expr) => {
        ab_glyph::PxScale::from(($value as f32) * ($img_width as f32 / 4000.0))
    };
}

#[macro_export]
macro_rules! px_h {
    ($value:expr, $img_height:expr) => {
        ($value as f32) * ($img_height as f32 / 2_666.666_7)
    };
}

#[macro_export]
macro_rules! pxscale_h {
    ($value:expr, $img_height:expr) => {
        ab_glyph::PxScale::from(($value as f32) * ($img_height as f32 / 2666.66667))
    };
}

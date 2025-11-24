/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::image::common::*;
use rust_i18n::t;
use std::io::Seek;
use std::path::PathBuf;

use crate::exif_impl::{OriginalExif, SimplifiedExif};

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum PackedImageEvent {
    None,
    Remove,
}

#[non_exhaustive]
pub struct PackedImage {
    /// path of image
    pub path: PathBuf,

    /// EXIF from image
    pub src_exif: OriginalExif,

    /// editable EXIF
    pub view_exif: SimplifiedExif,

    /// editable button for UI
    pub editable: bool,

    /// texture internally for egui framework
    /// but in testing environment, it would be Dummy enum
    pub texture: PackedTexture,
}

impl PackedImage {
    /// image::DynamicImage and is orientation required or not with boolean signal
    pub fn get_image(&self) -> Result<(image::DynamicImage, bool), image::ImageError> {
        let file = std::fs::File::open(self.path.clone())?;
        let mut buf_reader = std::io::BufReader::new(file);
        __load_image(&self.path, &mut buf_reader)
    }

    pub fn with_scale_and_orientation(
        &self,
        scale: crate::scale_config::ScaleConfig,
    ) -> Result<image::DynamicImage, image::ImageError> {
        use image::ImageBuffer;
        use image::Rgba;

        let (dyn_image, need_orientation) = self.get_image()?;
        let orientation = if need_orientation {
            self.view_exif.orientation
        } else {
            image::metadata::Orientation::NoTransforms
        };
        let (old_width, old_height) = (dyn_image.width(), dyn_image.height());
        let (new_width, new_height) =
            scale.apply(old_width, old_height, self.view_exif.is_vertical_rotated());

        log::debug!("({old_width} x {old_height}) -> ({new_width}x{new_height}");

        let resized_image: fast_image_resize::images::Image<'static> =
            resize_image(dyn_image, new_width, new_height)?;
        let buffer =
            ImageBuffer::<Rgba<u8>, _>::from_raw(new_width, new_height, resized_image.into_vec())
                .expect("Failed to convert to ImageBuffer");

        let mut dyn_image = image::DynamicImage::ImageRgba8(buffer);
        dyn_image.apply_orientation(orientation);

        Ok(dyn_image)
    }

    pub fn try_from_path(path: &PathBuf, ctx: &egui::Context) -> Result<Self, image::ImageError> {
        let file = std::fs::File::open(path)?;
        let mut buf_reader = std::io::BufReader::new(file);

        // Parse EXIF first
        let original_exif = OriginalExif::new(
            match exif::Reader::new().read_from_container(&mut buf_reader) {
                Ok(exif) => Some(exif),
                Err(e) => {
                    log::error!("Failed to parse EXIF from image: {e:?}");
                    None
                }
            },
        );

        buf_reader
            .seek(std::io::SeekFrom::Start(0))
            .expect("Failed reset seek zero");

        let (dyn_image, need_orientation) = __load_image(path, &mut buf_reader)?;
        let orientation = if need_orientation {
            original_exif.orientation()
        } else {
            image::metadata::Orientation::NoTransforms
        };

        let thumbnail = gen_thumbnail(dyn_image, orientation)?;
        let view_exif = SimplifiedExif::from(&original_exif);
        let file_name = path
            .clone()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        Ok(PackedImage {
            path: path.clone(),
            src_exif: original_exif,
            view_exif,
            editable: false,
            texture: PackedTexture::new(ctx.load_texture(
                file_name,
                thumbnail,
                egui::TextureOptions::NEAREST,
            )),
        })
    }

    pub fn try_from_path_cli(path: &PathBuf) -> Result<Self, image::ImageError> {
        let file = std::fs::File::open(path)?;
        let mut buf_reader = std::io::BufReader::new(file);

        // Parse EXIF first
        let original_exif = OriginalExif::new(
            match exif::Reader::new().read_from_container(&mut buf_reader) {
                Ok(exif) => Some(exif),
                Err(e) => {
                    log::error!("Failed to parse EXIF from image: {e:?}");
                    None
                }
            },
        );

        let view_exif = SimplifiedExif::from(&original_exif);

        Ok(Self {
            path: path.clone(),
            src_exif: original_exif,
            view_exif,
            editable: false,
            texture: PackedTexture::dummy(),
        })
    }

    pub fn file_name(&self) -> String {
        self.path
            .clone()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    pub fn prepostfixed_filename(
        &self,
        export_config: &crate::export_config::ExportConfig,
    ) -> String {
        let ext = export_config.output_format.extension();
        let postfix = &export_config.output_name.postfix;
        let prefix = &export_config.output_name.prefix;

        let stem = self.path.file_stem().unwrap_or_default().to_string_lossy();

        format!("{prefix}{stem}{postfix}.{ext}")
    }

    pub fn bulk_path(
        &self,
        export_config: &crate::export_config::ExportConfig,
    ) -> std::path::PathBuf {
        let file_name = self.prepostfixed_filename(export_config);
        let mut path = export_config.output_name.folder.clone();
        path.push(file_name);
        path
    }

    pub fn file_path(&self) -> String {
        self.path.clone().to_string_lossy().to_string()
    }

    fn update_editable_button(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let btn_text = if self.editable {
                t!("app.default.apply")
            } else {
                t!("app.default.edit")
            };
            if ui.button(btn_text).clicked() {
                self.editable = !self.editable;
            }
        });
    }

    pub fn update_ui(
        &mut self,
        ui: &mut egui::Ui,
        export_config: &crate::export_config::ExportConfig,
    ) -> PackedImageEvent {
        let mut ret = PackedImageEvent::None;

        // let max_height = crate::packed_image::THUMBNAIL_MAX_HEIGHT_AS_F32;
        // let width = max_height * self.texture.aspect_ratio();
        // let size = THUMBNAIL_DIMM;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                let ui_builder = egui::UiBuilder::new();
                // let orient = self.view_exif.orientation;
                // let (angle, _origin) = orient.egui_rotate();

                // EXIF Information
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(self.file_name());
                        self.update_editable_button(ui);
                    });

                    ui.scope_builder(ui_builder, |ui| {
                        egui::Grid::new(self.file_path())
                            .num_columns(2)
                            .spacing([10.0, 0.0])
                            .striped(true)
                            .show(ui, |ui| {
                                self.view_exif.update_ui(ui, self.editable);
                            })
                    });

                    if !self.editable {
                        ui.horizontal(|ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .add(
                                        egui::Button::new(t!("app.default.save"))
                                            .fill(egui::Color32::GREEN),
                                    )
                                    .clicked()
                                {
                                    let new_default_file_name =
                                        self.prepostfixed_filename(export_config);
                                    if let Some(output_path) = rfd::FileDialog::new()
                                        .set_file_name(new_default_file_name)
                                        .save_file()
                                    {
                                        match export_config.theme_reg.selected_theme_read().apply(
                                            self,
                                            export_config,
                                            &output_path,
                                        ) {
                                            Ok(_) => {
                                                log::info!(
                                                    "Saved with EXIF overlay to {output_path:?}"
                                                );
                                            }
                                            Err(e) => {
                                                log::error!("Failed to save EXIF overlay: {e:?}");
                                            }
                                        }
                                    }
                                }

                                if ui
                                    .add(
                                        egui::Button::new(t!("app.default.delete"))
                                            .fill(egui::Color32::RED),
                                    )
                                    .clicked()
                                {
                                    ret = PackedImageEvent::Remove;
                                }
                            });
                        });
                    }
                });

                // Thumbnail
                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    ui.add(
                        egui::Image::from_texture(self.texture.get())
                            // .rotate(angle, egui::Vec2::splat(0.5))
                            .corner_radius(4.0)
                            .fit_to_exact_size(THUMBNAIL_DIMM)
                            .shrink_to_fit(),
                    );
                    // .maintain_aspect_ratio(false), // .maintain_aspect_ratio(true),
                    // .fit_to_exact_size(THUMBNAIL_DIMM)
                });
            });
        });

        ret
    }
}

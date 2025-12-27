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

    /// Store original image bytes in memory for platforms without file system access
    /// (WASM, iOS sandboxed environments)
    #[cfg(not(feature = "desktop"))]
    pub image_bytes: Option<Vec<u8>>,
}

impl PackedImage {
    /// image::DynamicImage and is orientation required or not with boolean signal
    pub fn get_image(&self) -> Result<(image::DynamicImage, bool), image::ImageError> {
        #[cfg(feature = "desktop")]
        {
            let file = std::fs::File::open(self.path.clone())?;
            let mut buf_reader = std::io::BufReader::new(file);
            __load_image(&self.path, &mut buf_reader)
        }

        #[cfg(not(feature = "desktop"))]
        {
            // For non-desktop platforms (WASM, iOS), load from memory
            if let Some(bytes) = &self.image_bytes {
                __load_image_from_vec(&self.path, bytes.clone())
            } else {
                Err(image::ImageError::IoError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Image bytes not available for non-desktop platform",
                )))
            }
        }
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
        fn get_exif_with_thumbnail(
            buf_reader: &mut std::io::BufReader<std::fs::File>,
        ) -> (Option<exif::Exif>, Option<Vec<u8>>) {
            match exif::Reader::new().read_from_container(buf_reader) {
                Ok(exif) => {
                    // Currently it's temporary. EXIF-RS has optional MPF parsing stuff.
                    let thumbnail =
                        if let Some(biggest) = exif.thumbnails().iter().max_by_key(|e| e.length) {
                            // Avoid 160x120 image
                            log::info!("Thumbnail : {:?}", biggest);
                            if biggest.length >= 100 * 1024 {
                                log::info!("find out good thumbnail");
                                biggest.extract_data(buf_reader).ok()
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                    (Some(exif), thumbnail)
                }
                Err(e) => {
                    log::error!("Failed to parse EXIF from image: {e:?}");
                    (None, None)
                }
            }
        }

        let file = std::fs::File::open(path)?;
        let mut buf_reader = std::io::BufReader::new(file);

        // Parse EXIF first
        let (exif_or_none, exif_thumbnail) = get_exif_with_thumbnail(&mut buf_reader);
        let original_exif = OriginalExif::new(exif_or_none);

        buf_reader
            .seek(std::io::SeekFrom::Start(0))
            .expect("Failed reset seek zero");

        let (dyn_image, need_orientation) = if let Some(exif_thumbnail) = exif_thumbnail {
            log::info!("Used EXIF thumbnail : [{:X}]", exif_thumbnail.len());
            crate::dump!(exif_thumbnail);

            __load_image_from_vec(path, exif_thumbnail)
        } else {
            __load_image(path, &mut buf_reader)
        }?;

        log::debug!("{} x {}", dyn_image.width(), dyn_image.height());

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
            #[cfg(not(feature = "desktop"))]
            image_bytes: None, // Desktop uses file system, doesn't need bytes in memory
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
            #[cfg(not(feature = "desktop"))]
            image_bytes: None, // CLI mode is desktop-only, doesn't need bytes in memory
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
                                #[cfg(feature = "desktop")]
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
                            .fit_to_exact_size(THUMBNAIL_DIMM)
                            .shrink_to_fit()
                            .corner_radius(4.0),
                    );
                });
            });
        });

        ret
    }
}

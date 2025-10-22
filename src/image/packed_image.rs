/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use egui::ColorImage;
use fast_image_resize as fr;
use rust_i18n::t;
use std::io::Seek;
use std::path::PathBuf;

use crate::exif_impl::{OriginalExif, SimplifiedExif};

pub const THUMBNAIL_MAX_WIDTH: u32 = 330;
pub const THUMBNAIL_MAX_HEIGHT: u32 = 220;
pub const THUMBNAIL_MAX_WIDTH_AS_F32: f32 = 110.0; // considering retina display
pub const THUMBNAIL_MAX_HEIGHT_AS_F32: f32 = 165.0; // considering retina display
pub const THUMBNAIL_DIMM: egui::Vec2 =
    egui::Vec2::new(THUMBNAIL_MAX_HEIGHT_AS_F32, THUMBNAIL_MAX_WIDTH_AS_F32);

pub const THUMBMANIL_SCALE: crate::scale_config::ScaleConfig = crate::scale_config::ScaleConfig {
    mode: crate::scale_config::ScaleMode::ResizeAndCrop,
    value: THUMBNAIL_MAX_WIDTH,
    sub_value: THUMBNAIL_MAX_HEIGHT,
    scale_value: 2.0, // Don't care
};

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
    pub texture: egui::TextureHandle,
}

fn resize_image(
    decoded_image: image::DynamicImage,
    new_width: u32,
    new_height: u32,
) -> Result<fr::images::Image<'static>, image::ImageError> {
    let src_image = decoded_image.to_rgba8();
    let (src_width, src_height) = src_image.dimensions();

    let src_image = fr::images::Image::from_vec_u8(
        src_width,
        src_height,
        src_image.into_raw(),
        fr::PixelType::U8x4,
    )
    .map_err(|e| {
        log::error!("thumbnail : {e:?}");

        image::ImageError::Encoding(image::error::EncodingError::new(
            image::error::ImageFormatHint::Unknown,
            format!("thumbnail source prepare failed by {e:?}"),
        ))
    })?;

    // use fr instead of image
    // decoded_image.thumbnail(new_width, new_height)

    // let src_image = fr::images::Image::from_vec_u8(src_width, src_height, src_image.into_raw(), fr::PixelType::U8x4);
    // let src_image = fr::images::Image::from_slice_u8(src_width, src_height, &mut src_image.as_bytes(), src_image.as_bytes(), fr::PixelType::U8x4);

    // egui's ColorImage will upload to GPU, and it use F32x4
    let mut dst_image: fr::images::Image<'static> =
        fr::images::Image::new(new_width, new_height, fr::PixelType::U8x4);
    // let mut dst_image = fr::images::Image::new(new_width, new_height, fr::PixelType::F32x4);
    let mut resizer = fr::Resizer::new();
    resizer.resize(&src_image, &mut dst_image, None).unwrap();

    Ok(dst_image)
}

fn gen_thumbnail(
    decoded_image: image::DynamicImage,
    orientation: image::metadata::Orientation,
) -> Result<ColorImage, image::ImageError> {
    // future todo
    // resolve RGB -> RGBA makes clone+compute resource
    // Also this function does not cover U16 slice such as HDR
    let (src_width, src_height) = (decoded_image.width(), decoded_image.height());

    let is_vert_rot = crate::exif_impl::__is_vertical_rotated(orientation);

    let (mid_width, mid_height) = THUMBMANIL_SCALE.apply(src_width, src_height, is_vert_rot);
    let resized_image: fast_image_resize::images::Image<'static> =
        resize_image(decoded_image, mid_width, mid_height)?;

    let image_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        mid_width,
        mid_height,
        resized_image.into_vec(),
    )
    .expect("Failed to convert to ImageBuffer");

    let mut dyn_image = image::DynamicImage::ImageRgba8(image_buffer);

    dyn_image.apply_orientation(orientation);
    let x = (dyn_image.width() - THUMBNAIL_MAX_WIDTH) / 2;
    let y = (dyn_image.height() - THUMBNAIL_MAX_HEIGHT) / 2;

    let dyn_image = dyn_image.crop(x, y, THUMBNAIL_MAX_WIDTH, THUMBNAIL_MAX_HEIGHT);

    Ok(ColorImage::from_rgba_unmultiplied(
        [THUMBNAIL_MAX_WIDTH as usize, THUMBNAIL_MAX_HEIGHT as usize],
        dyn_image.as_bytes(),
    ))
}

fn __load_image(
    path: &PathBuf,
    buf_reader: &mut std::io::BufReader<std::fs::File>,
) -> Result<image::DynamicImage, image::ImageError> {
    let img_format = path
        .extension()
        .filter(|ext| !ext.is_empty())
        .and_then(image::ImageFormat::from_extension);

    let buf_reader = if img_format.is_some() {
        buf_reader
    } else {
        &mut std::io::BufReader::new(std::fs::File::open(path)?)
    };

    let decoder = if let Some(fmt) = img_format {
        image::ImageReader::with_format(
            buf_reader, // std::io::BufReader::new(std::fs::File::open(path)?),
            fmt,
        )
    } else {
        image::ImageReader::new(buf_reader)
    };

    decoder.decode().map_or_else(
        // let dyn_image = image::ImageReader::open(path)?.decode().map_or_else(
        |heic_suppose_or_err| {
            // Suppose HEIC/HEIF
            match heic_suppose_or_err {
                // Since libheif is depend on FFIed C library.
                // Pass buffer reader in to ffi is difficult.
                // Keep using path
                image::ImageError::Unsupported(unsp_e) => {
                    if img_format.is_none() {
                        crate::image::heic::load_heif(path).map_err(|e| {
                            image::error::ImageError::Unsupported(
                                image::error::UnsupportedError::from_format_and_kind(
                                    image::error::ImageFormatHint::PathExtension(
                                        path.to_path_buf(),
                                    ),
                                    image::error::UnsupportedErrorKind::GenericFeature(format!(
                                        "libheif internal error {e} and unsp_e : {unsp_e}"
                                    )),
                                ),
                            )
                        })
                    } else {
                        Err(image::error::ImageError::Unsupported(unsp_e))
                    }
                }
                other_err => Err(other_err),
            }
        },
        Ok,
    )
}

impl PackedImage {
    pub fn get_image(&self) -> Result<image::DynamicImage, image::ImageError> {
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

        let dyn_image = self.get_image()?;
        let orientation = self.view_exif.orientation;
        let (old_width, old_height) = (dyn_image.width(), dyn_image.height());
        let (new_width, new_height) =
            scale.apply(old_width, old_height, self.view_exif.is_vertical_rotated());

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

        let dyn_image = __load_image(path, &mut buf_reader)?;
        let orientation = original_exif.orientation();

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
            texture: ctx.load_texture(file_name, thumbnail, egui::TextureOptions::NEAREST),
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
                                        match export_config.theme_reg.selected_theme().apply(
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
                        egui::Image::from_texture(&self.texture)
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

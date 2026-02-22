/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::image::common::*;
use std::io::Seek;
use std::path::PathBuf;
use uuid::Uuid;

use crate::exif_impl::{OriginalExif, SimplifiedExif};

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
use rust_i18n::t;

// Re-export PackedImageEvent from common module for external access
#[allow(unused)]
pub use crate::image::common::PackedImageEvent;

#[non_exhaustive]
pub struct PackedImage {
    /// Unique identifier for this image (stable across deletions/reordering)
    pub uuid: Uuid,

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

    /// Store sticker-processed image in memory (for all platforms)
    /// This allows theme preview and export to use stickers without temporary files
    pub sticker_bytes: Option<Vec<u8>>,

    /// Perceptual hash for image similarity comparison (64-bit average hash)
    /// Calculated once during image loading for efficient grouping
    pub perceptual_hash: Option<u64>,

    /// Configured face detection areas with effects
    /// User-configured face regions that will have effects applied during export
    pub configured_faces: Vec<crate::effect::sticker_storage::FaceArea>,

    /// LUT ID configured for this image (None = no LUT applied)
    /// References a LUT in the global LutStorage
    pub lut_id: Option<Uuid>,

    /// Per-image crop/rotate transform
    /// Applied after EXIF orientation but before theme rendering.
    /// Face detection runs on original image; coords are transformed via this.
    pub crop_rotate: crate::effect::crop_rotate::CropRotateTransform,

    /// Pending async save-file dialog (macOS drag-drop safe)
    #[cfg(feature = "rfd")]
    #[allow(dead_code)] // used in desktop update_ui, not in ios_integration builds
    pub(crate) pending_save:
        Option<crate::util::async_file_dialog::PendingDialog<Option<std::path::PathBuf>>>,
}

impl PackedImage {
    /// image::DynamicImage and is orientation required or not with boolean signal
    pub fn get_image(&self) -> Result<(image::DynamicImage, bool), image::ImageError> {
        // Check sticker-processed image first (for all platforms)
        if let Some(sticker_bytes) = &self.sticker_bytes
            && let Ok(sticker_img) = image::load_from_memory(sticker_bytes)
        {
            log::info!("Loading sticker-processed image from sticker_bytes field");
            return Ok((sticker_img, true));
        }

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
                log::info!(
                    "📦 Loading image from {} bytes (path hint: {:?})",
                    bytes.len(),
                    self.path
                );
                let result = __load_image_from_vec(&self.path, bytes);
                if let Ok((ref img, _)) = result {
                    log::info!(
                        "📦 Loaded image dimensions: {}x{}",
                        img.width(),
                        img.height()
                    );
                }
                result
            } else {
                log::error!("❌ No image_bytes available for non-desktop platform");
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
        // Only consider vertical rotation for scale if orientation hasn't been pre-applied
        // (e.g., HEIF images decoded by Apple's native decoder already have correct dimensions)
        let is_vert_rot = need_orientation && self.view_exif.is_vertical_rotated();
        let (new_width, new_height) = scale.apply(old_width, old_height, is_vert_rot);

        log::debug!("({old_width} x {old_height}) -> ({new_width}x{new_height})");

        let resized_image: fast_image_resize::images::Image<'static> =
            resize_image(dyn_image, new_width, new_height)?;
        let buffer =
            ImageBuffer::<Rgba<u8>, _>::from_raw(new_width, new_height, resized_image.into_vec())
                .ok_or_else(|| {
                image::ImageError::Parameter(image::error::ParameterError::from_kind(
                    image::error::ParameterErrorKind::DimensionMismatch,
                ))
            })?;

        let mut dyn_image = image::DynamicImage::ImageRgba8(buffer);
        dyn_image.apply_orientation(orientation);

        // Apply crop/rotate transform after EXIF orientation
        if !self.crop_rotate.is_identity() {
            dyn_image = self.crop_rotate.apply(&dyn_image);
        }

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

            __load_image_from_vec(path, &exif_thumbnail)
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
            uuid: uuid::Uuid::new_v4(),
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
            sticker_bytes: None,   // No sticker data for manually loaded images
            perceptual_hash: None, // Not calculated for manually loaded images
            configured_faces: Vec::new(), // No faces configured yet
            lut_id: None,          // No LUT configured yet
            crop_rotate: crate::effect::crop_rotate::CropRotateTransform::default(),
            #[cfg(feature = "rfd")]
            pending_save: None,
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
            uuid: uuid::Uuid::new_v4(),
            path: path.clone(),
            src_exif: original_exif,
            view_exif,
            editable: false,
            texture: PackedTexture::dummy(),
            #[cfg(not(feature = "desktop"))]
            image_bytes: None, // CLI mode is desktop-only, doesn't need bytes in memory
            sticker_bytes: None, // CLI mode doesn't have sticker-processed images
            perceptual_hash: None, // CLI mode doesn't calculate hash
            configured_faces: Vec::new(), // No faces configured in CLI mode
            lut_id: None,        // No LUT configured in CLI mode
            crop_rotate: crate::effect::crop_rotate::CropRotateTransform::default(),
            #[cfg(feature = "rfd")]
            pending_save: None,
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
        self.prepostfixed_filename_with_override(export_config, None, None)
    }

    pub fn prepostfixed_filename_with_override(
        &self,
        export_config: &crate::export_config::ExportConfig,
        prefix_override: Option<&str>,
        postfix_override: Option<&str>,
    ) -> String {
        let ext = export_config.output_format.extension();

        // Use override if provided, otherwise use export_config
        let prefix = prefix_override.unwrap_or(&export_config.output_name.prefix);
        let postfix = postfix_override.unwrap_or(&export_config.output_name.postfix);

        let stem = self.path.file_stem().unwrap_or_default().to_string_lossy();

        // Format variables using EXIF data
        let formatted_prefix = self.view_exif.format_custom(prefix);
        let formatted_postfix = self.view_exif.format_custom(postfix);

        format!("{formatted_prefix}{stem}{formatted_postfix}.{ext}")
    }

    pub fn bulk_path(
        &self,
        export_config: &crate::export_config::ExportConfig,
    ) -> std::path::PathBuf {
        self.bulk_path_with_override(export_config, None, None)
    }

    pub fn bulk_path_with_override(
        &self,
        export_config: &crate::export_config::ExportConfig,
        prefix_override: Option<&str>,
        postfix_override: Option<&str>,
    ) -> std::path::PathBuf {
        let file_name = self.prepostfixed_filename_with_override(
            export_config,
            prefix_override,
            postfix_override,
        );
        let mut path = export_config.output_name.folder.clone();
        path.push(file_name);
        path
    }

    pub fn file_path(&self) -> String {
        self.path.clone().to_string_lossy().to_string()
    }

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    fn update_editable_button(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let btn_text = if self.editable {
                t!("common.actions.apply")
            } else {
                t!("common.actions.edit")
            };
            if ui.button(btn_text).clicked() {
                self.editable = !self.editable;
            }
        });
    }

    #[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
    pub fn update_ui(
        &mut self,
        ui: &mut egui::Ui,
        export_config: &crate::export_config::ExportConfig,
    ) -> PackedImageEvent {
        let mut ret = PackedImageEvent::None;

        // Poll pending save-file dialog
        #[cfg(feature = "rfd")]
        if let Some(ref pending) = self.pending_save
            && let Some(result) = pending.try_recv()
        {
            self.pending_save = None;
            if let Some(output_path) = result {
                match export_config.theme_reg.selected_theme_read().apply(
                    self,
                    export_config,
                    &output_path,
                ) {
                    Ok(_) => {
                        log::info!("Saved with EXIF overlay to {output_path:?}");
                    }
                    Err(e) => {
                        log::error!("Failed to save EXIF overlay: {e:?}");
                    }
                }
            }
        }

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
                                #[cfg(feature = "rfd")]
                                {
                                    let is_pending = self.pending_save.is_some();
                                    if ui
                                        .add_enabled(
                                            !is_pending,
                                            egui::Button::new(t!("common.actions.save"))
                                                .fill(egui::Color32::GREEN),
                                        )
                                        .clicked()
                                        && !is_pending
                                    {
                                        let new_default_file_name =
                                            self.prepostfixed_filename(export_config);
                                        self.pending_save =
                                            Some(crate::util::async_file_dialog::save_file_async(
                                                &new_default_file_name,
                                            ));
                                    }
                                }

                                #[cfg(all(feature = "desktop", not(feature = "rfd")))]
                                if ui
                                    .add(
                                        egui::Button::new(t!("common.actions.save"))
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
                                        egui::Button::new(t!("common.actions.delete"))
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

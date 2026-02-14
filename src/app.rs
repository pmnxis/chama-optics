/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::packed_image::PackedImage;
use crate::ui_state::ProgressState;
use rust_i18n::t;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Type alias for preview texture queue data
/// Contains: (ColorImage, UUID, dimensions, optional DynamicImage)
type PreviewTextureData = (
    egui::ColorImage,
    uuid::Uuid,
    (u32, u32),
    Option<image::DynamicImage>,
);

/// Type alias for thread-safe preview texture queue
type PreviewTextureQueue = Arc<Mutex<Option<PreviewTextureData>>>;

/// Type alias for face detection results
/// Contains: (Vec of face rectangles as (x, y, width, height), UUID, raw image dimensions (w, h))
type DetectionResultsData = (Vec<(i32, i32, u32, u32)>, uuid::Uuid, (u32, u32));

/// Type alias for thread-safe detection results queue
type DetectionResultsQueue = Arc<Mutex<Option<DetectionResultsData>>>;

/// Type alias for cheki preview queue data: (ColorImage, image_uuid)
type ChekiPreviewData = (egui::ColorImage, uuid::Uuid);

/// Type alias for thread-safe cheki preview queue
type ChekiPreviewQueue = Arc<Mutex<Option<ChekiPreviewData>>>;

/// Type alias for crop canvas preview queue data: (ColorImage, image_index, original_size)
type CropPreviewData = (egui::ColorImage, usize, (u32, u32));

/// Type alias for thread-safe crop canvas preview queue
type CropPreviewQueue = Arc<Mutex<Option<CropPreviewData>>>;

/// Main tab selection for the left sidebar
#[derive(serde::Deserialize, serde::Serialize, PartialEq, Clone, Copy, Debug, Default)]
pub enum MainTab {
    #[default]
    ImageList,
    Detection,
    ThemePreview,
    Color,
    Sticker,
    Cheki,
    ImportExport,
    Settings,
}

/// Interaction state for face rectangle editing in preview
#[derive(Debug, Clone, Default)]
pub enum FaceInteractionState {
    #[default]
    Idle,
    Dragging {
        face_index: usize,
        start_pos: egui::Pos2,
    },
    Resizing {
        face_index: usize,
        corner: ResizeCorner,
        start_pos: egui::Pos2,
        original_rect: (i32, i32, u32, u32),
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Interaction state for crop rectangle editing in crop/rotate canvas
#[derive(Debug, Clone, Default)]
pub enum CropInteractionState {
    #[default]
    Idle,
    DraggingCrop {
        start_pos: egui::Pos2,
        original_rect: crate::effect::crop_rotate::NormalizedRect,
    },
    ResizingCrop {
        corner: ResizeCorner,
        start_pos: egui::Pos2,
        original_rect: crate::effect::crop_rotate::NormalizedRect,
    },
}

/// Interaction state for cheki canvas (sticker/text dragging, resize, rotate)
#[derive(Debug, Clone, Default)]
pub enum ChekiInteractionState {
    #[default]
    Idle,
    DraggingSticker {
        sticker_index: usize,
        start_pos: egui::Pos2,
        original_x: f32,
        original_y: f32,
    },
    ResizingSticker {
        sticker_index: usize,
        start_pos: egui::Pos2,
        original_scale: f32,
        center: egui::Pos2,
    },
    RotatingSticker {
        sticker_index: usize,
        center: egui::Pos2,
        start_angle: f32,
        original_rotation: f32,
    },
    DraggingText {
        start_pos: egui::Pos2,
        original_x: f32,
        original_y: f32,
    },
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ChamaOptics {
    pub pending_paths: std::collections::VecDeque<PathBuf>,
    pub import_config: crate::import_config::ImportConfig,
    pub export_config: crate::export_config::ExportConfig,
    pub lang: crate::langs::Language,

    /// Image grouping configuration (experimental feature)
    pub image_grouping: crate::image_group::ImageGroupConfig,

    /// Show theme names in English (unique_name) instead of localized labels
    pub show_theme_name_in_english: bool,

    /// Temporary directory location for intermediate files
    pub temp_dir: crate::app_state::TempDir,

    /// Currently selected tab in sidebar
    selected_tab: MainTab,

    /// Sticker storage for custom sticker images
    pub sticker_storage: crate::effect::sticker_storage::StickerStorage,

    /// Sticker application config (scale, offset)
    pub sticker_config: crate::effect::sticker_storage::StickerConfig,

    /// Default effect to apply to newly detected faces
    pub default_face_effect: crate::effect::FaceEffectMode,

    /// LUT storage for color grading
    pub lut_storage: crate::effect::lut_storage::LutStorage,

    /// Color adjustments (future feature, placeholder)
    pub color_adjustments: crate::effect::color_adjustments::ColorAdjustments,

    /// Mosaic block size for effect application
    pub mosaic_block_size: u32,

    /// Stroke thickness for effect application
    pub stroke_thickness: u32,

    /// Stroke color for effect application (RGBA: r, g, b, a)
    pub stroke_color: egui::Color32,

    #[serde(skip)]
    /// Selected sticker ID for sticker preview tab
    pub(crate) selected_sticker_id: Option<uuid::Uuid>,

    #[serde(skip)]
    /// Selected image index for color tab
    pub(crate) color_selected_index: Option<usize>,

    #[serde(skip)]
    /// Cached original image texture for color tab (left side)
    pub(crate) color_original_texture: Option<egui::TextureHandle>,

    #[serde(skip)]
    /// Cached LUT-applied image texture for color tab (right side)
    pub(crate) color_lut_texture: Option<egui::TextureHandle>,

    #[serde(skip)]
    /// Color preview cache key (image_index, lut_id, color_adjustments)
    pub(crate) color_preview_cache_key: Option<(
        usize,
        Option<uuid::Uuid>,
        crate::effect::color_adjustments::ColorAdjustments,
    )>,

    #[serde(skip)]
    /// Cached LUT icon textures for gallery display (lut_id -> texture)
    pub(crate) lut_icon_textures: std::collections::HashMap<uuid::Uuid, egui::TextureHandle>,

    #[serde(skip)]
    pub packed_images: Vec<PackedImage>,

    #[serde(skip)]
    /// Active image groups (if grouping has been applied)
    pub image_groups: Option<Vec<crate::image_group::ImageGroup>>,

    #[serde(skip)]
    /// Selected image index for theme preview tab
    pub(crate) preview_selected_index: Option<usize>,

    #[serde(skip)]
    /// Cached theme preview texture
    pub(crate) theme_preview_texture: Option<egui::TextureHandle>,

    #[serde(skip)]
    /// Last theme preview generation params (to detect when to regenerate)
    pub(crate) theme_preview_cache_key: Option<(usize, String)>, // (image_index, theme_name)

    #[serde(skip)]
    /// Cached detection preview texture
    pub(crate) detection_preview_texture: Option<egui::TextureHandle>,

    #[serde(skip)]
    /// Sticker-processed images (one per packed image) - result of Detection tab
    pub(crate) sticker_processed_images:
        std::collections::HashMap<uuid::Uuid, Option<image::DynamicImage>>,

    #[serde(skip)]
    /// Last detection preview cache key
    pub(crate) detection_preview_cache_key: Option<(usize, usize)>, // (image_index, face_count)

    #[serde(skip)]
    /// Detected faces for the current image (editable)
    pub(crate) detected_faces: Vec<crate::effect::sticker_storage::FaceArea>,

    #[serde(skip)]
    /// Selected face index for editing
    pub(crate) selected_face_index: Option<usize>,

    #[serde(skip)]
    /// Interaction state for face rectangle editing
    pub(crate) face_interaction_state: FaceInteractionState,

    #[serde(skip)]
    /// Zoom level for detection preview (1.0 = 100%)
    pub(crate) detection_zoom: f32,

    #[serde(skip)]
    /// Pan offset for detection preview
    pub(crate) detection_pan: egui::Vec2,

    #[serde(skip)]
    /// Whether user is panning detection preview
    pub(crate) detection_is_panning: bool,

    #[serde(skip)]
    /// Pan start position
    pub(crate) detection_pan_start: egui::Vec2,

    #[serde(skip)]
    /// Progress tracking for face detection
    pub detection_progress: ProgressState,

    #[serde(skip)]
    /// Queue for storing preview image data from background thread
    pub preview_texture_queue: PreviewTextureQueue,

    #[serde(skip)]
    /// Original image size for detection preview (width, height) - AFTER orientation applied
    pub(crate) detection_preview_original_size: Option<(u32, u32)>,

    #[serde(skip)]
    /// Pending orientation for face detection coordinate transformation
    /// Stores the EXIF orientation when detection starts, used to transform coordinates
    pub(crate) detection_pending_orientation: Option<image::metadata::Orientation>,

    #[serde(skip)]
    /// Raw image dimensions before orientation applied (for coordinate transformation)
    pub(crate) detection_raw_image_size: Option<(u32, u32)>,

    #[serde(skip)]
    /// Queue for face detection results from background thread
    pub detection_results_queue: DetectionResultsQueue,

    #[serde(skip)]
    /// Cached InsightFace detector for reuse
    #[cfg(feature = "face_detection_insightface")]
    pub insightface_detector: std::sync::Arc<
        std::sync::Mutex<
            Option<std::sync::Arc<crate::effect::insightface_detector::InsightFaceDetector>>,
        >,
    >,

    #[serde(skip)]
    /// Background image texture for main screen
    background_texture: Option<egui::TextureHandle>,

    #[serde(skip)]
    /// Last detected theme mode (to reload texture on theme change)
    last_dark_mode: Option<bool>,

    #[serde(skip)]
    pub update: crate::util::check_update::CheckRelease,

    #[serde(skip)]
    pub save_progress: ProgressState,

    #[serde(skip)]
    pub load_progress: ProgressState,

    #[serde(skip)]
    pub loaded_image_queue: crate::image::loader::LoadedImageQueue,

    /// Per-image Cheki decorations (keyed by image UUID)
    #[serde(skip)]
    pub cheki_decorations:
        std::collections::HashMap<uuid::Uuid, crate::effect::cheki::ChekiDecoration>,

    /// Selected image index for Cheki tab
    #[serde(skip)]
    pub(crate) cheki_selected_index: Option<usize>,

    /// Cached Cheki preview texture
    #[serde(skip)]
    pub(crate) cheki_preview_texture: Option<egui::TextureHandle>,

    /// Cheki preview cache key (image_index, decoration_hash)
    #[serde(skip)]
    pub(crate) cheki_preview_cache_key: Option<(usize, u64)>,

    /// Cheki canvas interaction state (sticker/text dragging)
    #[serde(skip)]
    pub(crate) cheki_interaction_state: ChekiInteractionState,

    /// Crop/rotate canvas: rotated base image texture (before crop applied)
    #[serde(skip)]
    pub(crate) crop_canvas_texture: Option<egui::TextureHandle>,

    /// Crop/rotate canvas cache key: (image_index, rotation_90_count, rotation_degrees)
    #[serde(skip)]
    pub(crate) crop_canvas_cache_key: Option<(usize, u8, String)>,

    /// Crop/rotate canvas: dimensions of rotated image
    #[serde(skip)]
    pub(crate) crop_canvas_original_size: Option<(u32, u32)>,

    /// Crop/rotate canvas interaction state
    #[serde(skip)]
    pub(crate) crop_interaction_state: CropInteractionState,

    /// Background thread queue for cheki preview generation
    #[serde(skip)]
    pub(crate) cheki_preview_queue: ChekiPreviewQueue,

    /// Background thread queue for crop canvas generation
    #[serde(skip)]
    pub(crate) crop_preview_queue: CropPreviewQueue,
}

impl Default for ChamaOptics {
    fn default() -> Self {
        Self {
            pending_paths: std::collections::VecDeque::new(),
            import_config: crate::import_config::ImportConfig::default(),
            export_config: crate::export_config::ExportConfig::default(),
            lang: crate::langs::Language::get_system(),
            image_grouping: crate::image_group::ImageGroupConfig::default(),
            show_theme_name_in_english: true, // Default: show English names
            temp_dir: crate::app_state::TempDir::default(),
            selected_tab: MainTab::default(),
            sticker_storage: crate::effect::sticker_storage::StickerStorage::new(),
            sticker_config: crate::effect::sticker_storage::StickerConfig::default(),
            default_face_effect: crate::effect::FaceEffectMode::Sticker, // Default to Sticker mode
            mosaic_block_size: 10, // Default mosaic block size (pixels)
            stroke_thickness: 3,   // Default stroke thickness (pixels)
            stroke_color: egui::Color32::DARK_RED, // Default stroke color (red)
            lut_storage: crate::effect::lut_storage::LutStorage::new(),
            color_adjustments: crate::effect::color_adjustments::ColorAdjustments::default(),
            selected_sticker_id: None,
            color_selected_index: None,
            color_original_texture: None,
            color_lut_texture: None,
            color_preview_cache_key: None,
            lut_icon_textures: std::collections::HashMap::new(),
            packed_images: vec![],
            image_groups: None,
            preview_selected_index: None,
            theme_preview_texture: None,
            theme_preview_cache_key: None,
            detection_preview_texture: None,
            sticker_processed_images: std::collections::HashMap::new(),
            detection_preview_cache_key: None,
            detected_faces: vec![],
            selected_face_index: None,
            face_interaction_state: FaceInteractionState::default(),
            detection_zoom: 1.0,
            detection_pan: egui::Vec2::ZERO,
            detection_is_panning: false,
            detection_pan_start: egui::Vec2::ZERO,
            detection_progress: ProgressState::new(),
            preview_texture_queue: std::sync::Arc::new(std::sync::Mutex::new(None)),
            detection_preview_original_size: None,
            detection_pending_orientation: None,
            detection_raw_image_size: None,
            detection_results_queue: std::sync::Arc::new(std::sync::Mutex::new(None)),
            #[cfg(feature = "face_detection_insightface")]
            insightface_detector: std::sync::Arc::new(std::sync::Mutex::new(None)),
            background_texture: None,
            last_dark_mode: None,
            update: crate::util::check_update::CheckRelease::new(),
            save_progress: ProgressState::new(),
            load_progress: ProgressState::new(),
            loaded_image_queue: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            cheki_decorations: std::collections::HashMap::new(),
            cheki_selected_index: None,
            cheki_preview_texture: None,
            cheki_preview_cache_key: None,
            cheki_interaction_state: ChekiInteractionState::default(),
            crop_canvas_texture: None,
            crop_canvas_cache_key: None,
            crop_canvas_original_size: None,
            crop_interaction_state: CropInteractionState::default(),
            cheki_preview_queue: std::sync::Arc::new(std::sync::Mutex::new(None)),
            crop_preview_queue: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl ChamaOptics {
    /// Find image index by UUID
    fn find_image_by_uuid(&self, uuid: uuid::Uuid) -> Option<usize> {
        self.packed_images.iter().position(|img| img.uuid == uuid)
    }

    /// Invalidate all preview and detection caches
    pub fn invalidate_caches(&mut self) {
        self.theme_preview_cache_key = None;
        self.theme_preview_texture = None;
        self.detection_preview_cache_key = None;
        self.detection_preview_texture = None;
        self.detected_faces.clear();
        self.selected_face_index = None;
        // Color tab caches
        self.color_preview_cache_key = None;
        self.color_original_texture = None;
        self.color_lut_texture = None;
        // Cheki tab caches
        self.cheki_preview_cache_key = None;
        self.cheki_preview_texture = None;
        // Crop/rotate canvas caches
        self.crop_canvas_cache_key = None;
        self.crop_canvas_texture = None;
        self.crop_canvas_original_size = None;
    }

    /// Delete an image by index and handle related cleanup
    pub fn delete_image_by_index(&mut self, idx: usize) {
        if idx >= self.packed_images.len() {
            log::warn!("Attempted to delete image at invalid index {}", idx);
            return;
        }

        let removed_uuid = self.packed_images[idx].uuid;
        log::info!(
            "Deleting image at index {} with UUID {:?}",
            idx,
            removed_uuid
        );

        // Remove the image
        let _ = self.packed_images.remove(idx);

        // Update grouping: remove UUID from all groups
        if let Some(groups) = &mut self.image_groups {
            // Remove the UUID from all groups
            for group in groups.iter_mut() {
                group.image_uuids.retain(|&uuid| uuid != removed_uuid);
            }

            // Remove empty groups
            groups.retain(|g| !g.image_uuids.is_empty());

            // Clear grouping if no groups remain
            if groups.is_empty() {
                self.image_groups = None;
                log::info!("All groups removed after image deletion");
            } else {
                log::info!("Updated grouping after removing image at index {}", idx);
            }
        }

        // Clean up related data
        self.sticker_processed_images.remove(&removed_uuid);

        // Adjust preview_selected_index if needed
        if let Some(selected) = self.preview_selected_index {
            if selected == idx {
                // Deleted the selected image, try to select another
                self.preview_selected_index = if self.packed_images.is_empty() {
                    None
                } else if idx >= self.packed_images.len() {
                    // Was the last image, select the new last image
                    Some(self.packed_images.len() - 1)
                } else {
                    // Select the image that moved into this position
                    Some(idx)
                };
            } else if selected > idx {
                // Selected image shifted left due to deletion
                self.preview_selected_index = Some(selected - 1);
            }
        }

        // Invalidate caches
        self.invalidate_caches();

        log::info!("Successfully deleted image at index {}", idx);
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(all(feature = "desktop", not(feature = "ios_integration")))]
        crate::fonts::replace_fonts(&cc.egui_ctx);

        log::info!(
            "Current support langs : {:?}",
            rust_i18n::available_locales!()
        );

        let mut app: ChamaOptics = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();

        // Always start with ImageList tab
        app.selected_tab = MainTab::ImageList;

        app.lang.update_i18n();

        app
    }

    pub(crate) fn save_packed_image_all(&mut self, ui: &mut egui::Ui) {
        use rayon::prelude::*;
        use std::sync::atomic::Ordering;

        // Thread-safe struct for parallel processing
        struct SaveTask {
            path: std::path::PathBuf,
            view_exif: crate::exif_impl::SimplifiedExif,
            prefix: Option<String>,
            postfix: Option<String>,
            sticker_bytes: Option<Vec<u8>>,
            #[allow(dead_code)] // todo - windows issue, resolve later
            configured_faces: Vec<crate::effect::sticker_storage::FaceArea>,
            /// LUT ID configured for this image (for color grading)
            lut_id: Option<uuid::Uuid>,
            /// Per-image crop/rotate transform
            crop_rotate: crate::effect::crop_rotate::CropRotateTransform,
            /// Per-image cheki decoration (if configured)
            cheki_decoration: Option<crate::effect::cheki::ChekiDecoration>,
        }

        // save each
        fn __save_bulk_each(
            idx: usize,
            task: &SaveTask,
            export_config: &crate::export_config::ExportConfig,
            sticker_processed_images: &std::collections::HashMap<
                uuid::Uuid,
                Option<image::DynamicImage>,
            >,
            lut_storage: &mut crate::effect::lut_storage::LutStorage,
            sticker_storage: &crate::effect::sticker_storage::StickerStorage,
        ) -> Result<(), image::ImageError> {
            // Reconstruct PackedImage from path
            let mut pi = crate::packed_image::PackedImage::try_from_path_cli(&task.path)?;

            // Use saved view_exif instead of reconstructed one
            pi.view_exif = task.view_exif.clone();

            // Restore lut_id and crop_rotate from task
            pi.lut_id = task.lut_id;
            pi.crop_rotate = task.crop_rotate.clone();

            // Apply LUT to image if configured
            // LUT is applied before stickers/theme in the pipeline
            if let Some(lut_id) = task.lut_id {
                log::info!("Export: Applying LUT {:?} to image {}", lut_id, idx);

                // Load original image
                let mut dyn_image = image::open(&task.path)?;

                // Apply LUT
                lut_storage.apply_lut_to_image(lut_id, &mut dyn_image);

                // Save LUT-processed image to sticker_bytes for theme to use
                let original_ext = task
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("jpg");

                let format = match original_ext.to_lowercase().as_str() {
                    "png" => image::ImageFormat::Png,
                    "heic" | "heif" => image::ImageFormat::Jpeg,
                    _ => image::ImageFormat::Jpeg,
                };

                let mut bytes = Vec::new();
                if dyn_image
                    .write_to(&mut std::io::Cursor::new(&mut bytes), format)
                    .is_ok()
                {
                    pi.sticker_bytes = Some(bytes);
                    log::info!(
                        "Export: Saved LUT-processed image to sticker_bytes for image {}",
                        idx
                    );
                }
            } else {
                // No LUT - use sticker_bytes from task if available (prioritize over HashMap)
                pi.sticker_bytes = task.sticker_bytes.clone();
            }

            // Use sticker-processed image from HashMap as fallback
            let temp_path =
                if let Some(Some(sticker_image)) = sticker_processed_images.get(&pi.uuid) {
                    log::info!(
                        "Using sticker-processed image from HashMap for image {}",
                        idx
                    );

                    // Create temporary file with sticker-processed image
                    let temp_dir = std::env::temp_dir();
                    let temp_file_name = format!(
                        "chama_optics_sticker_{}{}",
                        idx,
                        task.path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("png")
                    );
                    let temp_path = temp_dir.join(&temp_file_name);

                    // Save sticker-processed image to temporary file
                    if let Err(e) = sticker_image.save(&temp_path) {
                        log::error!("Failed to save temp sticker image: {:?}", e);
                        None
                    } else {
                        log::info!("Successfully saved temp sticker image: {:?}", temp_path);
                        Some(temp_path)
                    }
                } else {
                    None
                };

            // Use sticker-processed image directly if available
            // NOTE: We keep pi.path pointing to original file to avoid format errors
            // The theme system will apply stickers from sticker_bytes field

            let sticker_available =
                task.sticker_bytes.is_some() || sticker_processed_images.get(&pi.uuid).is_some();

            log::info!(
                "Export: Image {} - sticker_available={}, temp_path.is_some={}, task.sticker_bytes.len={:?}, lookup_found={:?}",
                idx,
                sticker_available,
                temp_path.is_some(),
                task.sticker_bytes.as_ref().map(|b| b.len()),
                sticker_processed_images.get(&pi.uuid).is_some()
            );

            // Generate output path with export config (prefix, postfix, format, etc.)
            // NOTE: Always use original path, NOT: temp file path
            let new_path = pi.bulk_path_with_override(
                export_config,
                task.prefix.as_deref(),
                task.postfix.as_deref(),
            );

            // Detect faces on ORIGINAL image BEFORE theming (macOS only)
            // IMPORTANT: Only use faces if user has explicitly configured them in Detection tab
            let pre_detected_faces: Option<Vec<(i32, i32, u32, u32)>> = {
                if !task.configured_faces.is_empty() {
                    // Use configured faces from Detection tab - skip re-detection!
                    Some(
                        task.configured_faces
                            .iter()
                            .map(|f| (f.x, f.y, f.width, f.height))
                            .collect(),
                    )
                } else {
                    // No configured faces - skip face detection entirely
                    // Face detection should only run when explicitly ordered from Detection tab
                    log::info!(
                        "[INFO][Face Detection] No configured faces - skipping automatic face detection"
                    );
                    None
                }
            };

            // Apply theme (and face effects)
            // If cheki decoration is present, we apply it after the theme
            if let Some(ref cheki_deco) = task.cheki_decoration {
                // Apply theme to get image, then apply cheki on top, then save
                let themed_image = export_config
                    .theme_reg
                    .selected_theme_read()
                    .apply_to_image(&pi, export_config)?;
                let mut final_image = crate::effect::cheki_renderer::apply_cheki_decoration(
                    themed_image,
                    cheki_deco,
                    sticker_storage,
                );
                // Save the final image with face effects
                export_config.save_image_with_faces(
                    &mut final_image,
                    None,
                    &new_path,
                    pre_detected_faces,
                )?;
            } else {
                // No cheki - use normal theme apply_with_faces pipeline
                export_config
                    .theme_reg
                    .selected_theme_read()
                    .apply_with_faces(&pi, export_config, &new_path, pre_detected_faces)?;
            }
            Ok(())
        }

        if !self.export_config.output_name.check_folder_available(true) {
            log::error!(
                "Cannot access following directory {:?}",
                self.export_config.output_name.folder
            );
            // todo - warning on UI
        }

        // Clone cheki decorations for SaveTask construction
        let cheki_decos = &self.cheki_decorations;

        // Convert PackedImages to SaveTasks for parallel processing
        // If grouping is active, use group-specific prefix/postfix
        let tasks: Vec<SaveTask> = if let Some(ref groups) = self.image_groups {
            self.packed_images
                .iter()
                .map(|pi| {
                    // Find which group this image belongs to (by UUID)
                    let group_info = groups.iter().find(|g| g.image_uuids.contains(&pi.uuid));

                    SaveTask {
                        path: pi.path.clone(),
                        view_exif: pi.view_exif.clone(),
                        // Use group prefix/postfix only if not using default
                        prefix: group_info.and_then(|g| {
                            if g.use_default {
                                None
                            } else {
                                Some(g.prefix.text.clone())
                            }
                        }),
                        postfix: group_info.and_then(|g| {
                            if g.use_default {
                                None
                            } else {
                                Some(g.postfix.text.clone())
                            }
                        }),
                        sticker_bytes: pi.sticker_bytes.clone(),
                        configured_faces: pi.configured_faces.clone(),
                        lut_id: pi.lut_id,
                        crop_rotate: pi.crop_rotate.clone(),
                        cheki_decoration: cheki_decos.get(&pi.uuid).cloned(),
                    }
                })
                .collect()
        } else {
            // No grouping - use default config
            self.packed_images
                .iter()
                .map(|pi| SaveTask {
                    path: pi.path.clone(),
                    view_exif: pi.view_exif.clone(),
                    prefix: None,
                    postfix: None,
                    sticker_bytes: pi.sticker_bytes.clone(),
                    configured_faces: pi.configured_faces.clone(),
                    lut_id: pi.lut_id,
                    crop_rotate: pi.crop_rotate.clone(),
                    cheki_decoration: cheki_decos.get(&pi.uuid).cloned(),
                })
                .collect()
        };

        let total = tasks.len();

        // Initialize progress tracking with new ProgressState
        self.save_progress.start(total);

        log::info!("Starting background save of {} images", total);

        // Clone export_config for the background thread
        let clone_start = std::time::Instant::now();
        let export_config = self.export_config.clone();
        log::info!("ExportConfig clone took {:?}", clone_start.elapsed());

        // Clone sticker_processed_images for the background thread
        let sticker_processed_images = self.sticker_processed_images.clone();
        log::info!(
            "Export: sticker_processed_images HashMap has {} entries",
            sticker_processed_images.len()
        );
        for (uuid, val) in sticker_processed_images.iter() {
            log::info!("  - UUID {:?}: has image = {}", uuid, val.is_some());
        }

        // Clone lut_storage for the background thread (for LUT application during export)
        // Wrap in Mutex for thread-safe access during parallel processing
        let lut_storage = std::sync::Mutex::new(self.lut_storage.clone_for_thread());

        // Clone sticker_storage for the background thread (for cheki decoration rendering)
        let sticker_storage = self.sticker_storage.clone_for_thread();

        // Clone progress counter for use in parallel threads
        let progress_counter = self.save_progress.counter();

        // Get egui context for requesting repaint from background thread
        let ctx = ui.ctx().clone();

        // Spawn background thread for parallel processing
        std::thread::spawn(move || {
            log::info!("Background thread started");

            // Calculate optimal thread count: (CPU_COUNT / 2).max(CPU_COUNT - 3)
            let cpu_count = num_cpus::get();
            let thread_count = (cpu_count / 2).max(cpu_count.saturating_sub(3)).max(1);
            log::info!("Using {} threads out of {} CPUs", thread_count, cpu_count);

            // Configure rayon thread pool
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build()
                .unwrap();

            // Parallel processing using rayon
            pool.install(|| {
                tasks.par_iter().enumerate().for_each(|(idx, task)| {
                    log::info!("Processing image {}", idx);

                    // Get mutable access to lut_storage for this task
                    let mut lut_storage_guard = lut_storage.lock().unwrap();

                    match __save_bulk_each(
                        idx,
                        task,
                        &export_config,
                        &sticker_processed_images,
                        &mut lut_storage_guard,
                        &sticker_storage,
                    ) {
                        Ok(_) => {
                            log::info!("Successfully saved image {}", idx);
                        }
                        Err(e) => {
                            log::error!("Failed to save image {}: {e:?}", idx);
                        }
                    }

                    // Drop the lock before updating progress
                    drop(lut_storage_guard);

                    // Update progress counter AFTER processing
                    let current = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                    log::info!("Progress: {}/{}", current, total);

                    // Request UI repaint AFTER processing to show progress
                    ctx.request_repaint();
                })
            });

            log::info!("Background thread completed");
        });

        // Schedule progress bar to disappear after a short delay
        ui.ctx().request_repaint();
    }

    pub(crate) fn update_packed_image(&mut self, ui: &mut egui::Ui) {
        let mut remove_index: Option<usize> = None;
        let mut remove_group_idx: Option<usize> = None;

        // Determine group boundaries if grouping is active
        let group_starts: std::collections::HashSet<usize> =
            if let Some(groups) = &self.image_groups {
                groups
                    .iter()
                    .filter_map(|g| {
                        // Get first UUID and find its current index
                        g.image_uuids
                            .first()
                            .and_then(|&uuid| self.find_image_by_uuid(uuid))
                    })
                    .collect()
            } else {
                std::collections::HashSet::new()
            };

        // Pre-calculate suggestions for all groups to avoid borrow checker issues
        let group_suggestions: Vec<(String, String)> = if let Some(ref groups) = self.image_groups {
            groups
                .iter()
                .map(|group| {
                    // Find first image by UUID
                    group
                        .image_uuids
                        .first()
                        .and_then(|&uuid| self.find_image_by_uuid(uuid))
                        .and_then(|idx| self.packed_images.get(idx))
                        .map(|pi| {
                            let exif = &pi.view_exif;
                            (group.suggest_prefix(exif), group.suggest_postfix(exif))
                        })
                        .unwrap_or_else(|| (String::new(), String::new()))
                })
                .collect()
        } else {
            Vec::new()
        };

        for (idx, pi) in self.packed_images.iter_mut().enumerate() {
            // Show group separator and header for each group
            if group_starts.contains(&idx) {
                // Add separator before group (except for very first group)
                if idx > 0 {
                    ui.add_space(10.0);
                    ui.separator();
                }

                // Show group info with controls
                if let Some(groups) = &mut self.image_groups {
                    // Find group by checking if this idx matches the first UUID's index
                    let current_image_uuid = pi.uuid;
                    if let Some(group_idx) = groups
                        .iter()
                        .position(|g| g.image_uuids.first() == Some(&current_image_uuid))
                    {
                        let group = &mut groups[group_idx];

                        ui.vertical(|ui| {
                            // Group header with delete button and use default checkboxes
                            ui.horizontal(|ui| {
                                // Selection checkbox
                                ui.checkbox(&mut group.selected, "");

                                // Group label - show datetime/camera for Ungrouped, otherwise Group N
                                let label_text = if group.datetime.is_none() {
                                    t!(
                                        "app.image_grouping.group_label_format",
                                        name = t!("app.image_grouping.ungrouped_label"),
                                        count = group.image_uuids.len()
                                    )
                                } else {
                                    t!(
                                        "app.image_grouping.group_label_format",
                                        name = format!(
                                            "{} {}",
                                            t!("common.actions.group"),
                                            group_idx + 1
                                        ),
                                        count = group.image_uuids.len()
                                    )
                                };
                                ui.label(
                                    egui::RichText::new(label_text)
                                        .strong()
                                        .color(ui.visuals().strong_text_color()),
                                );

                                // Right-aligned controls on the same line
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        // Delete group button
                                        if ui
                                            .button("🗑")
                                            .on_hover_text(t!("common.actions.delete"))
                                            .clicked()
                                        {
                                            remove_group_idx = Some(group_idx);
                                        }

                                        // Use default prefix/postfix checkbox (applies to both)
                                        ui.checkbox(
                                            &mut group.use_default,
                                            t!("app.image_grouping.use_default"),
                                        );
                                    },
                                );
                            });

                            // Show datetime and camera_model for non-Ungrouped groups
                            if group.datetime.is_some() {
                                if let Some(dt) = group.datetime {
                                    let datetime_str = dt.format("%Y.%m.%d  %H:%M:%S").to_string();
                                    ui.label(
                                        egui::RichText::new(format!("📅 {}", datetime_str)).weak(),
                                    );
                                }
                                if !group.camera_model.is_empty() {
                                    ui.label(
                                        egui::RichText::new(format!("📷 {}", group.camera_model))
                                            .weak(),
                                    );
                                }
                            }

                            // Prefix and Postfix on one line (left and right)
                            ui.horizontal(|ui| {
                                let available_width = ui.available_width();
                                let label_width = 50.0;
                                let spacing = ui.spacing().item_spacing.x;
                                let input_width =
                                    (available_width - label_width * 2.0 - spacing * 4.0 - 40.0)
                                        / 2.0; // 40.0 for suggestion buttons

                                // Prefix (left side)
                                ui.label(t!("app.image_grouping.prefix_label"));
                                if !group.use_default {
                                    group.prefix.render_text_edit_with_autocomplete(
                                        ui,
                                        input_width,
                                        format!("group_{}_prefix", group_idx),
                                    );

                                    // Suggestion button with preview
                                    if group_idx < group_suggestions.len() {
                                        let suggested = &group_suggestions[group_idx].0;
                                        if !suggested.is_empty()
                                            && ui
                                                .button("💡")
                                                .on_hover_text(t!(
                                                    "app.image_grouping.suggestion_hint",
                                                    suggestion = suggested
                                                ))
                                                .clicked()
                                        {
                                            group.prefix.text = suggested.clone();
                                        }
                                    }
                                } else {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "(default: {})",
                                            self.export_config.output_name.prefix
                                        ))
                                        .weak(),
                                    );
                                }

                                // Postfix (right side)
                                ui.label(t!("app.image_grouping.postfix_label"));
                                if !group.use_default {
                                    group.postfix.render_text_edit_with_autocomplete(
                                        ui,
                                        input_width,
                                        format!("group_{}_postfix", group_idx),
                                    );

                                    // Suggestion button with preview
                                    if group_idx < group_suggestions.len() {
                                        let suggested = &group_suggestions[group_idx].1;
                                        if !suggested.is_empty()
                                            && ui
                                                .button("💡")
                                                .on_hover_text(t!(
                                                    "app.image_grouping.suggestion_hint",
                                                    suggestion = suggested
                                                ))
                                                .clicked()
                                        {
                                            group.postfix.text = suggested.clone();
                                        }
                                    }
                                } else {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "(default: {})",
                                            self.export_config.output_name.postfix
                                        ))
                                        .weak(),
                                    );
                                }
                            });
                        });
                    }
                }

                // Add spacing after group header (except for first group)
                if idx > 0 {
                    ui.add_space(5.0);
                }
            }

            match pi.update_ui(ui, &self.export_config) {
                crate::packed_image::PackedImageEvent::None => { /* Nothing */ }
                crate::packed_image::PackedImageEvent::Remove => {
                    // todo - ordering bigger number of index, and remove later
                    remove_index = Some(idx);
                }
            }
        }

        if let Some(idx) = remove_index {
            let removed_uuid = self.packed_images[idx].uuid;
            let _ = self.packed_images.remove(idx);

            // Update grouping: remove UUID from all groups
            if let Some(groups) = &mut self.image_groups {
                // Remove the UUID from all groups
                for group in groups.iter_mut() {
                    group.image_uuids.retain(|&uuid| uuid != removed_uuid);
                }

                // Remove empty groups
                groups.retain(|g| !g.image_uuids.is_empty());

                // Clear grouping if no groups remain
                if groups.is_empty() {
                    self.image_groups = None;
                    log::info!("All groups removed after image deletion");
                } else {
                    log::info!("Updated grouping after removing image at index {}", idx);
                }
            }
        }

        // Handle group deletion
        if let Some(group_idx) = remove_group_idx {
            // First, collect UUIDs before borrowing groups mutably
            let uuids_to_remove: Option<Vec<uuid::Uuid>> = self
                .image_groups
                .as_ref()
                .and_then(|groups| groups.get(group_idx))
                .map(|group| group.image_uuids.clone());

            if let Some(uuids) = uuids_to_remove {
                // Convert UUIDs to current indices (in reverse order for safe deletion)
                let mut indices_to_remove: Vec<usize> = uuids
                    .iter()
                    .filter_map(|&uuid| self.find_image_by_uuid(uuid))
                    .collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in descending order

                let total_removed = indices_to_remove.len();

                // Remove images from packed_images Vec
                for &img_idx in indices_to_remove.iter() {
                    if img_idx < self.packed_images.len() {
                        self.packed_images.remove(img_idx);
                    }
                }

                // Now borrow groups mutably to remove the group itself
                if let Some(groups) = &mut self.image_groups {
                    groups.remove(group_idx);

                    // No need to rebuild indices - UUIDs are stable!
                    // Just verify groups aren't empty after deletion
                    if groups.is_empty() {
                        self.image_groups = None;
                        log::info!("All groups removed");
                    } else {
                        log::info!(
                            "Removed group {} with {} images",
                            group_idx + 1,
                            total_removed
                        );
                    }
                }
            }
        }
    }

    /// Handle drag and drop for file paths
    fn handle_drag_drop(&mut self, ui: &mut egui::Ui) {
        ui.ctx().input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths: Vec<_> = i
                    .raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect();

                // Handle differently based on current tab
                match self.selected_tab {
                    MainTab::Sticker => {
                        // Add as stickers
                        for path in paths.iter() {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Sticker")
                                .to_string();

                            match self.sticker_storage.add_sticker(name.clone(), path) {
                                Ok(_) => {
                                    log::info!("Added sticker: {}", name);
                                }
                                Err(e) => {
                                    log::error!("Failed to add sticker {}: {:?}", name, e);
                                }
                            }
                        }
                    }
                    MainTab::Color => {
                        // Color tab: .cube files go to LUT storage, images go to image queue
                        for path in paths.iter() {
                            let ext = path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_lowercase();

                            if ext == "cube" {
                                // Add as LUT
                                let name = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("Unnamed LUT")
                                    .to_string();

                                match self.lut_storage.add_lut(name.clone(), path) {
                                    Ok(id) => {
                                        log::info!(
                                            "Added LUT via drag-drop: {} (id: {})",
                                            name,
                                            id
                                        );
                                        // Auto-assign to current image if one is selected
                                        if let Some(idx) = self.color_selected_index
                                            && let Some(pi) = self.packed_images.get_mut(idx)
                                        {
                                            pi.lut_id = Some(id);
                                            log::info!(
                                                "Auto-assigned new LUT to image index {}",
                                                idx
                                            );
                                        }
                                        // Invalidate preview cache
                                        self.color_preview_cache_key = None;
                                    }
                                    Err(e) => {
                                        log::error!("Failed to add LUT {}: {:?}", name, e);
                                    }
                                }
                            } else {
                                // Add as image
                                self.pending_paths.push_back(path.clone());
                            }
                        }
                    }
                    _ => {
                        // Add as images (default behavior)
                        for path in paths.iter() {
                            self.pending_paths.push_back(path.clone());
                        }
                    }
                }
            }
        });
    }

    /// Load and render background image based on theme
    fn render_background_image(&mut self, ui: &mut egui::Ui) {
        // Detect current theme (dark or light)
        let is_dark_mode = ui.ctx().style().visuals.dark_mode;

        // Invalidate texture if theme changed
        if self.last_dark_mode != Some(is_dark_mode) {
            self.background_texture = None;
            self.last_dark_mode = Some(is_dark_mode);
        }

        // Load appropriate background image based on theme
        if self.background_texture.is_none() {
            let image_data: &[u8] = if is_dark_mode {
                include_bytes!("../assets/dark-background.png")
            } else {
                include_bytes!("../assets/light-background.png")
            };

            if let Ok(image) = image::load_from_memory(image_data) {
                let size = [image.width() as _, image.height() as _];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                self.background_texture = Some(ui.ctx().load_texture(
                    "background",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }

        // Draw background image with opacity at bottom-right, 25% size
        if let Some(texture) = &self.background_texture {
            let available_rect = ui.available_rect_before_wrap();
            let texture_size = texture.size_vec2();

            // Scale to 25% of original size
            let scaled_size = texture_size * 0.25;

            // Position at bottom-right
            let image_pos = egui::pos2(
                available_rect.max.x - scaled_size.x,
                available_rect.max.y - scaled_size.y,
            );

            let image_rect = egui::Rect::from_min_size(image_pos, scaled_size);

            ui.painter().image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::from_white_alpha(100), // ~39% opacity
            );
        }
    }

    /// Process pending image loading and loaded images
    fn process_image_loading(&mut self, ui: &mut egui::Ui) {
        // Start parallel loading if there are pending paths and no active loading
        if !self.pending_paths.is_empty() && !self.load_progress.is_active() {
            // Collect ALL pending paths
            let all_paths: Vec<PathBuf> = self.pending_paths.drain(..).collect();
            let total = all_paths.len();

            // Start progress tracking
            self.load_progress.start(total);

            // Spawn parallel loader with work queue
            crate::image::loader::spawn_parallel_loader(
                all_paths,
                self.import_config.get_alt_fnumber,
                self.import_config.use_35mm_focal_length,
                self.load_progress.counter(),
                self.loaded_image_queue.clone(),
                ui.ctx().clone(),
            );
        }

        // Transfer loaded images from background thread to UI thread
        if let Ok(mut queue) = self.loaded_image_queue.try_lock()
            && !queue.is_empty()
        {
            log::info!("Transferring {} loaded images to UI", queue.len());

            // Process all loaded images from the queue
            let mut new_image_indices: Vec<uuid::Uuid> = Vec::new();
            for loaded_data in queue.drain(..) {
                log::info!("Creating packed image for {:?}", loaded_data.path);
                match crate::image::loader::create_packed_image_from_data(loaded_data, ui.ctx()) {
                    Some(packed_image) => {
                        log::info!("Successfully created packed image");
                        let new_uuid = packed_image.uuid;
                        self.packed_images.push(packed_image);
                        new_image_indices.push(new_uuid);
                    }
                    None => {
                        log::error!("Failed to create packed image");
                    }
                }
            }

            // Add new images to "Ungrouped" group if grouping is active
            if !new_image_indices.is_empty() && self.image_groups.is_some() {
                log::info!(
                    "Adding {} new images to Ungrouped group",
                    new_image_indices.len()
                );

                if let Some(groups) = &mut self.image_groups {
                    // Find or create "Ungrouped" group (datetime is None)
                    let ungrouped_idx = groups.iter().position(|g| g.datetime.is_none());

                    if let Some(idx) = ungrouped_idx {
                        // Append to existing Ungrouped group
                        groups[idx].image_uuids.extend(new_image_indices);
                    } else {
                        // Create new Ungrouped group (datetime is None)
                        groups.push(crate::image_group::ImageGroup {
                            image_uuids: new_image_indices,
                            datetime: None,
                            camera_model: "Ungrouped".to_string(),
                            prefix: crate::effect::variable_text::VariableText::new(),
                            postfix: crate::effect::variable_text::VariableText::new(),
                            selected: true,
                            use_default: false,
                        });
                    }
                }
            }

            log::info!("Total packed_images now: {}", self.packed_images.len());
        }
    }
}

impl eframe::App for ChamaOptics {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui_impl(ui, _frame);
        });
    }
}

impl ChamaOptics {
    fn ui_impl(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Render bottom panel using component
        crate::ui_components::render_bottom_panel(
            ui,
            &mut self.load_progress,
            &mut self.save_progress,
            &self.update,
        );

        // Left sidebar with icon-only tabs
        crate::ui_components::render_tab_sidebar(ui, &mut self.selected_tab);

        // Render central panel with tab-based content
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Handle drag and drop for all tabs
            self.handle_drag_drop(ui);

            // Load and render background image based on theme
            self.render_background_image(ui);

            // Render tab content on top of background
            match self.selected_tab {
                MainTab::ImageList => self.render_image_list_tab(ui),
                MainTab::Detection => self.render_detection_tab(ui),
                MainTab::ThemePreview => self.render_theme_preview_tab(ui),
                MainTab::Color => self.render_color_tab(ui),
                MainTab::Sticker => self.render_sticker_tab(ui),
                MainTab::Cheki => self.render_cheki_tab(ui),
                MainTab::ImportExport => self.render_import_export_tab(ui),
                MainTab::Settings => self.render_settings_tab(ui),
            }
        });

        // Process pending image loading and loaded images
        self.process_image_loading(ui);

        // Process face detection results from background thread
        self.process_detection_results();

        // Process preview texture from background thread
        self.process_preview_texture(ui);
    }
}

// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Sticker storage module - manages user-uploaded sticker images
//!
//! This module provides storage, loading, and management of custom sticker images
//! that can be applied to detected faces.

use image::{DynamicImage, GenericImage, GenericImageView, Rgba};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// A single sticker item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerItem {
    /// Unique identifier for this sticker
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Path to the sticker image file
    pub image_path: PathBuf,
    /// Timestamp when added
    #[serde(default = "default_timestamp")]
    pub timestamp: u64,
    /// File hash for integrity verification (computed on add)
    #[serde(default)]
    pub file_hash: Option<u64>,
    /// Whether the file hash mismatches (runtime only, not serialized)
    #[serde(skip)]
    pub hash_mismatch: bool,
    /// Whether the file is missing (runtime only, not serialized)
    #[serde(skip)]
    pub file_missing: bool,
}

fn default_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl StickerItem {
    pub fn new(name: String, image_path: PathBuf) -> Self {
        let file_hash = Self::compute_file_hash(&image_path);
        Self {
            id: Uuid::new_v4(),
            name,
            image_path,
            timestamp: default_timestamp(),
            file_hash,
            hash_mismatch: false,
            file_missing: false,
        }
    }

    /// Compute a simple hash of file contents for integrity check
    pub fn compute_file_hash(path: &Path) -> Option<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let data = std::fs::read(path).ok()?;
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        Some(hasher.finish())
    }

    /// Verify the file hash matches stored hash
    pub fn verify_hash(&mut self) -> bool {
        if !self.image_path.exists() {
            self.file_missing = true;
            self.hash_mismatch = false;
            return false;
        }

        self.file_missing = false;

        if let Some(stored_hash) = self.file_hash
            && let Some(current_hash) = Self::compute_file_hash(&self.image_path)
        {
            self.hash_mismatch = stored_hash != current_hash;
            return !self.hash_mismatch;
        }

        // No stored hash, consider it valid
        self.hash_mismatch = false;
        true
    }

    /// Load the sticker image from disk
    pub fn load_image(&self) -> Option<DynamicImage> {
        image::open(&self.image_path).ok()
    }
}

/// Storage manager for stickers
#[derive(Clone, Serialize, Deserialize)]
pub struct StickerStorage {
    /// List of available stickers
    pub stickers: Vec<StickerItem>,
    /// Default sticker ID to apply to new faces (None = no sticker)
    #[serde(default)]
    pub default_sticker_id: Option<Uuid>,
    /// Directory where stickers are stored
    #[serde(default)]
    pub storage_directory: PathBuf,
    /// Texture cache for sticker previews (runtime only, not serialized)
    #[serde(skip)]
    #[cfg(feature = "egui")]
    pub texture_cache: HashMap<Uuid, egui::TextureHandle>,
}

impl Default for StickerStorage {
    fn default() -> Self {
        Self {
            stickers: Vec::new(),
            default_sticker_id: None,
            storage_directory: Self::default_storage_path(),
            #[cfg(feature = "egui")]
            texture_cache: HashMap::new(),
        }
    }
}

impl StickerStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get default storage path for stickers
    pub fn default_storage_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("chama_optics")
            .join("stickers")
    }

    /// Ensure storage directory exists
    pub fn ensure_directory(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.storage_directory)
    }

    /// Add a new sticker from an image file
    pub fn add_sticker(&mut self, name: String, source_path: &Path) -> std::io::Result<Uuid> {
        self.ensure_directory()?;

        // Generate unique filename
        let id = Uuid::new_v4();
        let extension = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");
        let dest_filename = format!("{}_{}.{}", id, name.replace(' ', "_"), extension);
        let dest_path = self.storage_directory.join(&dest_filename);

        // Copy file to storage
        std::fs::copy(source_path, &dest_path)?;

        // Create sticker item with hash computation
        let sticker = StickerItem::new(name, dest_path);

        self.stickers.push(sticker);
        Ok(id)
    }

    /// Remove a sticker by ID
    pub fn remove_sticker(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.stickers.iter().position(|s| s.id == id) {
            let sticker = self.stickers.remove(pos);
            // Try to delete the file (ignore errors)
            let _ = std::fs::remove_file(&sticker.image_path);

            // Clear default if this was the default sticker
            if self.default_sticker_id == Some(id) {
                self.default_sticker_id = None;
            }
            true
        } else {
            false
        }
    }

    /// Get sticker by ID
    pub fn get_sticker(&self, id: Uuid) -> Option<&StickerItem> {
        self.stickers.iter().find(|s| s.id == id)
    }

    /// Get the default sticker (if set)
    pub fn get_default_sticker(&self) -> Option<&StickerItem> {
        self.default_sticker_id.and_then(|id| self.get_sticker(id))
    }

    /// Set the default sticker
    pub fn set_default_sticker(&mut self, id: Option<Uuid>) {
        self.default_sticker_id = id;
    }

    /// Build a lookup dictionary from sticker ID to image path
    pub fn build_sticker_dict(&self) -> HashMap<String, PathBuf> {
        self.stickers
            .iter()
            .map(|s| (s.id.to_string(), s.image_path.clone()))
            .collect()
    }

    /// Get or create texture handle for a sticker
    #[cfg(feature = "egui")]
    pub fn get_texture(&mut self, ctx: &egui::Context, id: Uuid) -> Option<egui::TextureHandle> {
        if let Some(texture) = self.texture_cache.get(&id) {
            return Some(texture.clone());
        }

        // Load image and create texture
        if let Some(sticker) = self.get_sticker(id)
            && let Some(img) = sticker.load_image()
        {
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let texture = ctx.load_texture(
                format!("sticker_{}", id),
                egui::ColorImage::from_rgba_unmultiplied(size, &rgba),
                egui::TextureOptions::LINEAR,
            );
            self.texture_cache.insert(id, texture.clone());
            return Some(texture);
        }
        None
    }

    /// Get sticker image by ID (for background threads)
    pub fn get_sticker_image(&self, id: Uuid) -> Option<DynamicImage> {
        self.get_sticker(id).and_then(|s| s.load_image())
    }

    /// Clone for use in background threads (excludes egui-specific fields)
    pub fn clone_for_thread(&self) -> Self {
        Self {
            stickers: self.stickers.clone(),
            default_sticker_id: self.default_sticker_id,
            storage_directory: self.storage_directory.clone(),
            #[cfg(feature = "egui")]
            texture_cache: HashMap::new(),
        }
    }

    /// Verify all stickers and update hash mismatch flags
    pub fn verify_all_stickers(&mut self) {
        for sticker in &mut self.stickers {
            sticker.verify_hash();

            // Remove cached textures for hash-mismatched stickers
            if sticker.file_hash.is_some() && sticker.hash_mismatch {
                #[cfg(feature = "egui")]
                self.texture_cache.remove(&sticker.id);
            }
        }
    }

    /// Render UI for sticker storage management
    #[cfg(feature = "egui")]
    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading(t!("sticker.title"));
        ui.separator();

        // Default sticker selection
        ui.horizontal(|ui| {
            ui.label(t!("sticker.default_sticker"));

            egui::ComboBox::from_id_salt("default_sticker_combo")
                .selected_text(
                    self.default_sticker_id
                        .and_then(|id| self.get_sticker(id))
                        .map(|s| s.name.as_str())
                        .unwrap_or("None"),
                )
                .show_ui(ui, |ui| {
                    // None option
                    if ui
                        .selectable_label(self.default_sticker_id.is_none(), "None")
                        .clicked()
                    {
                        self.default_sticker_id = None;
                    }
                    // Sticker options
                    for sticker in &self.stickers {
                        if ui
                            .selectable_label(
                                self.default_sticker_id == Some(sticker.id),
                                &sticker.name,
                            )
                            .clicked()
                        {
                            self.default_sticker_id = Some(sticker.id);
                        }
                    }
                });
        });

        ui.separator();

        // Sticker list
        ui.label(format!(
            "{} ({})",
            t!("sticker.stickers"),
            self.stickers.len()
        ));

        let mut to_remove: Option<Uuid> = None;
        let mut set_as_default: Option<Uuid> = None;

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for sticker in &self.stickers {
                    ui.horizontal(|ui| {
                        // Sticker name
                        ui.label(&sticker.name);

                        // Default indicator
                        if self.default_sticker_id == Some(sticker.id) {
                            ui.label("⭐");
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Delete button
                            if ui.button("🗑").on_hover_text(t!("sticker.delete")).clicked() {
                                to_remove = Some(sticker.id);
                            }

                            // Set as default button
                            if self.default_sticker_id != Some(sticker.id)
                                && ui
                                    .button("⭐")
                                    .on_hover_text(t!("sticker.set_default"))
                                    .clicked()
                            {
                                set_as_default = Some(sticker.id);
                            }
                        });
                    });
                }
            });

        // Handle removal
        if let Some(id) = to_remove {
            self.remove_sticker(id);
        }

        // Handle set default
        if let Some(id) = set_as_default {
            self.default_sticker_id = Some(id);
        }

        ui.separator();

        // Add sticker button (requires file dialog)
        #[cfg(feature = "rfd")]
        if ui.button(t!("sticker.add_sticker")).clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "webp", "gif"])
                .pick_file()
        {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Sticker")
                .to_string();

            if let Err(e) = self.add_sticker(name, &path) {
                log::error!("Failed to add sticker: {}", e);
            }
        }
    }
}

/// Configuration for applying a sticker to a face
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerConfig {
    /// Sticker ID to apply (references StickerStorage)
    pub sticker_id: Option<Uuid>,
    /// Scale factor relative to face size (1.0 = match face size)
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// Horizontal offset from face center
    #[serde(default)]
    pub offset_x: i32,
    /// Vertical offset from face center
    #[serde(default)]
    pub offset_y: i32,
}

fn default_scale() -> f32 {
    1.0
}

impl Default for StickerConfig {
    fn default() -> Self {
        Self {
            sticker_id: None,
            scale: 1.0,
            offset_x: 0,
            offset_y: 0,
        }
    }
}

/// Face detection result with optional sticker and effect assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceArea {
    /// Face bounding box: (x, y, width, height)
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Optional sticker ID assigned to this face
    pub sticker_id: Option<Uuid>,
    /// Effect mode to apply to this face
    #[serde(default)]
    pub effect_mode: super::FaceEffectMode,
}

impl FaceArea {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            sticker_id: None,
            effect_mode: super::FaceEffectMode::default(),
        }
    }

    pub fn from_tuple(face: (i32, i32, u32, u32)) -> Self {
        Self::new(face.0, face.1, face.2, face.3)
    }

    pub fn to_tuple(&self) -> (i32, i32, u32, u32) {
        (self.x, self.y, self.width, self.height)
    }
}

/// Apply stickers to detected faces using the sticker storage
pub fn apply_stickers_from_storage(
    mut image: DynamicImage,
    faces: &[FaceArea],
    storage: &StickerStorage,
    config: &StickerConfig,
) -> DynamicImage {
    for face in faces {
        // Determine which sticker to use (face-specific or default)
        let sticker_id = face
            .sticker_id
            .or(config.sticker_id)
            .or(storage.default_sticker_id);

        let Some(sticker_id) = sticker_id else {
            continue; // No sticker assigned
        };

        let Some(sticker_item) = storage.get_sticker(sticker_id) else {
            log::warn!("Sticker {} not found in storage", sticker_id);
            continue;
        };

        let Some(sticker_img) = sticker_item.load_image() else {
            log::warn!(
                "Failed to load sticker image: {:?}",
                sticker_item.image_path
            );
            continue;
        };

        // Calculate target size based on face size and scale
        let target_size = ((face.width as f32 * config.scale) as u32).max(20);

        // Resize sticker to target size
        let resized = sticker_img.resize(
            target_size,
            target_size,
            image::imageops::FilterType::Lanczos3,
        );

        // Calculate position (center of face + offset as percentage of sticker size)
        // offset_x and offset_y are percentages (-100 to 100) of sticker size
        let offset_pixel_x = (target_size as f32 * config.offset_x as f32 / 100.0) as i32;
        let offset_pixel_y = (target_size as f32 * config.offset_y as f32 / 100.0) as i32;

        let center_x = face.x + (face.width as i32 / 2) + offset_pixel_x;
        let center_y = face.y + (face.height as i32 / 2) + offset_pixel_y;

        // Overlay sticker
        overlay_sticker_image(&mut image, &resized, center_x, center_y);
    }

    image
}

/// Overlay a sticker image onto the base image with alpha blending
fn overlay_sticker_image(
    base: &mut DynamicImage,
    sticker: &DynamicImage,
    center_x: i32,
    center_y: i32,
) {
    let sticker_width = sticker.width() as i32;
    let sticker_height = sticker.height() as i32;

    let start_x = center_x - sticker_width / 2;
    let start_y = center_y - sticker_height / 2;

    for sy in 0..sticker_height {
        for sx in 0..sticker_width {
            let target_x = start_x + sx;
            let target_y = start_y + sy;

            // Check bounds
            if target_x >= 0
                && target_y >= 0
                && target_x < base.width() as i32
                && target_y < base.height() as i32
            {
                let sticker_pixel = sticker.get_pixel(sx as u32, sy as u32);

                // Only overlay if sticker pixel has some alpha
                if sticker_pixel[3] > 0 {
                    let base_pixel = base.get_pixel(target_x as u32, target_y as u32);

                    // Alpha blending
                    let alpha = sticker_pixel[3] as f32 / 255.0;
                    let inv_alpha = 1.0 - alpha;

                    let blended = Rgba([
                        (sticker_pixel[0] as f32 * alpha + base_pixel[0] as f32 * inv_alpha) as u8,
                        (sticker_pixel[1] as f32 * alpha + base_pixel[1] as f32 * inv_alpha) as u8,
                        (sticker_pixel[2] as f32 * alpha + base_pixel[2] as f32 * inv_alpha) as u8,
                        255,
                    ]);

                    base.put_pixel(target_x as u32, target_y as u32, blended);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sticker_item_new() {
        let sticker = StickerItem::new("Test".to_string(), PathBuf::from("/test/path.png"));
        assert_eq!(sticker.name, "Test");
        assert!(sticker.timestamp > 0);
    }

    #[test]
    fn test_sticker_storage_new() {
        let storage = StickerStorage::new();
        assert!(storage.stickers.is_empty());
        assert!(storage.default_sticker_id.is_none());
    }

    #[test]
    fn test_face_with_sticker() {
        let face = FaceArea::new(10, 20, 100, 150);
        assert_eq!(face.x, 10);
        assert_eq!(face.y, 20);
        assert_eq!(face.width, 100);
        assert_eq!(face.height, 150);
        assert!(face.sticker_id.is_none());
    }
}

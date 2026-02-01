// SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! LUT storage module - manages user-uploaded LUT files for color grading
//!
//! This module provides storage, loading, and management of CUBE format LUT files
//! that can be applied to images for color grading.

use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use wagahai_lut::{CubeLut, CubeParser, LutType};

#[cfg(feature = "desktop")]
use rust_i18n::t;

/// A single LUT item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LutItem {
    /// Unique identifier for this LUT
    pub id: Uuid,
    /// Display name
    pub name: String,
    /// Path to the LUT file (.cube)
    pub file_path: PathBuf,
    /// Timestamp when added
    #[serde(default = "default_timestamp")]
    pub timestamp: u64,
    /// File hash for integrity verification (computed on add)
    #[serde(default)]
    pub file_hash: Option<u64>,
    /// LUT type (1D or 3D)
    #[serde(default)]
    pub lut_type: StoredLutType,
    /// LUT size info (for display purposes)
    #[serde(default)]
    pub lut_size_info: String,
    /// Whether the file hash mismatches (runtime only, not serialized)
    #[serde(skip)]
    pub hash_mismatch: bool,
    /// Whether the file is missing (runtime only, not serialized)
    #[serde(skip)]
    pub file_missing: bool,
}

/// Stored LUT type (serializable version of wagahai_lut::LutType)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub enum StoredLutType {
    #[default]
    Unknown,
    Lut1D,
    Lut3D,
}

impl From<LutType> for StoredLutType {
    fn from(lt: LutType) -> Self {
        match lt {
            LutType::Lut1DFixed | LutType::Lut1DOther => StoredLutType::Lut1D,
            LutType::Lut3DFixed | LutType::Lut3DOther => StoredLutType::Lut3D,
        }
    }
}

fn default_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl LutItem {
    pub fn new(name: String, file_path: PathBuf, lut: &CubeLut) -> Self {
        let file_hash = Self::compute_file_hash(&file_path);
        let lut_type = lut.get_lut_type().into();
        let lut_size_info = Self::format_lut_size(lut);

        Self {
            id: Uuid::new_v4(),
            name,
            file_path,
            timestamp: default_timestamp(),
            file_hash,
            lut_type,
            lut_size_info,
            hash_mismatch: false,
            file_missing: false,
        }
    }

    /// Format LUT size for display
    fn format_lut_size(lut: &CubeLut) -> String {
        if lut.is_3d() {
            if let Some(lut_3d) = lut.lut_3d() {
                let size = lut_3d.size();
                format!("3D {}x{}x{}", size, size, size)
            } else {
                "3D".to_string()
            }
        } else if lut.is_1d() {
            if let Some(lut_1d) = lut.lut_1d() {
                format!("1D {}", lut_1d.size())
            } else {
                "1D".to_string()
            }
        } else {
            "Unknown".to_string()
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
        if !self.file_path.exists() {
            self.file_missing = true;
            self.hash_mismatch = false;
            return false;
        }

        self.file_missing = false;

        if let Some(stored_hash) = self.file_hash
            && let Some(current_hash) = Self::compute_file_hash(&self.file_path)
        {
            self.hash_mismatch = stored_hash != current_hash;
            return !self.hash_mismatch;
        }

        // No stored hash, consider it valid
        self.hash_mismatch = false;
        true
    }

    /// Load the LUT from disk
    pub fn load_lut(&self) -> Option<CubeLut> {
        CubeParser::from_file(&self.file_path).ok()
    }
}

/// Storage manager for LUTs
#[derive(Clone, Serialize, Deserialize)]
pub struct LutStorage {
    /// List of available LUTs
    pub luts: Vec<LutItem>,
    /// Currently selected LUT ID (None = no LUT applied)
    #[serde(default)]
    pub selected_lut_id: Option<Uuid>,
    /// Directory where LUTs are stored
    #[serde(default)]
    pub storage_directory: PathBuf,
    /// Parsed LUT cache (runtime only, not serialized)
    #[serde(skip)]
    lut_cache: HashMap<Uuid, CubeLut>,
}

impl Default for LutStorage {
    fn default() -> Self {
        Self {
            luts: Vec::new(),
            selected_lut_id: None,
            storage_directory: Self::default_storage_path(),
            lut_cache: HashMap::new(),
        }
    }
}

impl LutStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get default storage path for LUTs
    pub fn default_storage_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("chama_optics")
            .join("luts")
    }

    /// Ensure storage directory exists
    pub fn ensure_directory(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.storage_directory)
    }

    /// Add a new LUT from a .cube file
    pub fn add_lut(&mut self, name: String, source_path: &Path) -> Result<Uuid, LutStorageError> {
        self.ensure_directory()?;

        // Parse the LUT first to validate it
        let lut = CubeParser::from_file(source_path).map_err(LutStorageError::ParseError)?;

        // Generate unique filename
        let id = Uuid::new_v4();
        let dest_filename = format!("{}_{}.cube", id, name.replace(' ', "_"));
        let dest_path = self.storage_directory.join(&dest_filename);

        // Copy file to storage
        std::fs::copy(source_path, &dest_path)?;

        // Create LUT item
        let lut_item = LutItem::new(name, dest_path, &lut);
        let item_id = lut_item.id;

        // Cache the parsed LUT
        self.lut_cache.insert(item_id, lut);

        self.luts.push(lut_item);
        Ok(item_id)
    }

    /// Remove a LUT by ID
    pub fn remove_lut(&mut self, id: Uuid) -> bool {
        if let Some(pos) = self.luts.iter().position(|l| l.id == id) {
            let lut_item = self.luts.remove(pos);
            // Try to delete the file (ignore errors)
            let _ = std::fs::remove_file(&lut_item.file_path);

            // Clear cache
            self.lut_cache.remove(&id);

            // Clear selection if this was the selected LUT
            if self.selected_lut_id == Some(id) {
                self.selected_lut_id = None;
            }
            true
        } else {
            false
        }
    }

    /// Get LUT item by ID
    pub fn get_lut(&self, id: Uuid) -> Option<&LutItem> {
        self.luts.iter().find(|l| l.id == id)
    }

    /// Get the selected LUT item (if set)
    pub fn get_selected_lut(&self) -> Option<&LutItem> {
        self.selected_lut_id.and_then(|id| self.get_lut(id))
    }

    /// Set the selected LUT
    pub fn set_selected_lut(&mut self, id: Option<Uuid>) {
        self.selected_lut_id = id;
    }

    /// Get or load parsed LUT by ID (lazy loading with cache)
    pub fn get_parsed_lut(&mut self, id: Uuid) -> Option<&CubeLut> {
        // Check if already cached
        if self.lut_cache.contains_key(&id) {
            return self.lut_cache.get(&id);
        }

        // Try to load from disk
        if let Some(lut_item) = self.luts.iter().find(|l| l.id == id)
            && let Some(lut) = lut_item.load_lut()
        {
            self.lut_cache.insert(id, lut);
            return self.lut_cache.get(&id);
        }

        None
    }

    /// Apply the selected LUT to an image (in-place modification)
    pub fn apply_selected_lut(&mut self, image: &mut DynamicImage) -> bool {
        let Some(lut_id) = self.selected_lut_id else {
            return false;
        };

        self.apply_lut_to_image(lut_id, image)
    }

    /// Apply a specific LUT to an image (in-place modification)
    pub fn apply_lut_to_image(&mut self, lut_id: Uuid, image: &mut DynamicImage) -> bool {
        // Get the parsed LUT (load if needed)
        let Some(lut) = self.get_parsed_lut(lut_id) else {
            return false;
        };

        // Apply LUT based on image type
        match image {
            DynamicImage::ImageRgba8(img) => {
                wagahai_lut::lut::apply_rgba_mut(lut, img);
                true
            }
            DynamicImage::ImageRgb8(img) => {
                wagahai_lut::lut::apply_rgb_mut(lut, img);
                true
            }
            _ => {
                // Convert to RGBA8, apply, then convert back
                let mut rgba = image.to_rgba8();
                wagahai_lut::lut::apply_rgba_mut(lut, &mut rgba);
                *image = DynamicImage::ImageRgba8(rgba);
                true
            }
        }
    }

    /// Clone for use in background threads (excludes cache)
    pub fn clone_for_thread(&self) -> Self {
        Self {
            luts: self.luts.clone(),
            selected_lut_id: self.selected_lut_id,
            storage_directory: self.storage_directory.clone(),
            lut_cache: HashMap::new(),
        }
    }

    /// Update all LUT file paths to use the current storage directory
    /// This is needed on iOS where app container paths can change between launches
    pub fn update_file_paths(&mut self) {
        for lut_item in &mut self.luts {
            // Extract just the filename from the current path
            if let Some(filename) = lut_item.file_path.file_name() {
                // Reconstruct path using current storage_directory
                let new_path = self.storage_directory.join(filename);
                lut_item.file_path = new_path;
            }
        }
    }

    /// Verify all LUTs and update hash mismatch flags
    pub fn verify_all_luts(&mut self) {
        for lut_item in &mut self.luts {
            lut_item.verify_hash();

            // Remove cached LUT for hash-mismatched items
            if lut_item.file_hash.is_some() && lut_item.hash_mismatch {
                self.lut_cache.remove(&lut_item.id);
            }
        }
    }

    /// Render UI for LUT selection and management
    #[cfg(feature = "desktop")]
    pub fn update_ui(&mut self, ui: &mut egui::Ui) -> LutUiAction {
        let mut action = LutUiAction::None;

        ui.horizontal(|ui| {
            ui.label(t!("color.lut_select"));

            // LUT selection combo box
            let no_lut_text = t!("color.no_lut");
            let selected_name = self
                .selected_lut_id
                .and_then(|id| self.get_lut(id))
                .map(|l| l.name.clone())
                .unwrap_or_else(|| no_lut_text.to_string());

            egui::ComboBox::from_id_salt("lut_select_combo")
                .selected_text(&selected_name)
                .show_ui(ui, |ui| {
                    // None option
                    if ui
                        .selectable_label(self.selected_lut_id.is_none(), t!("color.no_lut"))
                        .clicked()
                    {
                        self.selected_lut_id = None;
                    }

                    ui.separator();

                    // LUT options
                    for lut_item in &self.luts {
                        let label = format!("{} ({})", lut_item.name, lut_item.lut_size_info);
                        let is_selected = self.selected_lut_id == Some(lut_item.id);

                        // Show warning icon if file is missing or hash mismatch
                        let mut label_text = egui::RichText::new(&label);
                        if lut_item.file_missing {
                            label_text = label_text.color(egui::Color32::RED);
                        } else if lut_item.hash_mismatch {
                            label_text = label_text.color(egui::Color32::YELLOW);
                        }

                        if ui.selectable_label(is_selected, label_text).clicked() {
                            self.selected_lut_id = Some(lut_item.id);
                        }
                    }
                });

            // Add LUT button
            if ui.button(t!("color.add_lut")).clicked() {
                action = LutUiAction::OpenAddDialog;
            }

            // Remove LUT button (only if a LUT is selected)
            if self.selected_lut_id.is_some()
                && ui.button(t!("color.remove_lut")).clicked()
                && let Some(id) = self.selected_lut_id
            {
                self.remove_lut(id);
            }
        });

        // Show selected LUT info
        if let Some(lut_item) = self.get_selected_lut() {
            ui.horizontal(|ui| {
                ui.label(t!("color.lut_info"));
                ui.label(format!("{} - {}", lut_item.name, lut_item.lut_size_info));
            });
        }

        action
    }
}

/// UI action returned from update_ui
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LutUiAction {
    None,
    OpenAddDialog,
}

/// Errors that can occur during LUT storage operations
#[derive(Debug)]
pub enum LutStorageError {
    IoError(std::io::Error),
    ParseError(wagahai_lut::error::CubeError),
}

impl From<std::io::Error> for LutStorageError {
    fn from(e: std::io::Error) -> Self {
        LutStorageError::IoError(e)
    }
}

impl std::fmt::Display for LutStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LutStorageError::IoError(e) => write!(f, "IO error: {}", e),
            LutStorageError::ParseError(e) => write!(f, "LUT parse error: {}", e),
        }
    }
}

impl std::error::Error for LutStorageError {}

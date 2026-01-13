/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Business logic layer - independent of UI framework

use crate::export_config::ExportConfig;
use crate::import_config::ImportConfig;
use crate::packed_image::PackedImage;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

/// Temporary directory location for intermediate files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TempDir {
    /// Use /tmp directory (default on Unix-like systems)
    #[default]
    Tmp,
    /// Use /var/tmp directory (more persistent on some systems)
    VarTmp,
}

impl TempDir {
    /// Get the path for temporary directory
    pub fn path(&self) -> &'static str {
        match self {
            TempDir::Tmp => "/tmp",
            TempDir::VarTmp => "/var/tmp",
        }
    }

    /// Get display label
    pub fn label(&self) -> &'static str {
        match self {
            TempDir::Tmp => "/tmp",
            TempDir::VarTmp => "/var/tmp",
        }
    }
}

/// Core application state containing business logic only
/// This struct is UI-framework independent and can be tested without egui
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AppState {
    /// Queue of image paths waiting to be loaded
    pub pending_paths: VecDeque<PathBuf>,

    /// Loaded images with their metadata
    #[serde(skip)]
    pub packed_images: Vec<PackedImage>,

    /// Import settings
    pub import_config: ImportConfig,

    /// Export settings (including theme registry)
    pub export_config: ExportConfig,

    /// Language selection
    pub lang: crate::langs::Language,

    /// Temporary directory location for intermediate files
    pub temp_dir: TempDir,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            pending_paths: VecDeque::new(),
            packed_images: Vec::new(),
            import_config: ImportConfig::default(),
            export_config: ExportConfig::default(),
            lang: crate::langs::Language::get_system(),
            temp_dir: TempDir::default(),
        }
    }
}

impl AppState {
    /// Add a single path to the pending queue
    pub fn add_path(&mut self, path: PathBuf) {
        self.pending_paths.push_back(path);
    }

    /// Add multiple paths to the pending queue
    pub fn add_paths(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        self.pending_paths.extend(paths);
    }

    /// Get the next pending path to load
    pub fn next_pending_path(&mut self) -> Option<PathBuf> {
        self.pending_paths.pop_front()
    }

    /// Check if there are pending paths to load
    pub fn has_pending_paths(&self) -> bool {
        !self.pending_paths.is_empty()
    }

    /// Get the number of pending paths
    pub fn pending_count(&self) -> usize {
        self.pending_paths.len()
    }

    /// Add a loaded image to the collection
    pub fn add_loaded_image(&mut self, image: PackedImage) {
        self.packed_images.push(image);
    }

    /// Remove all loaded images
    pub fn clear_images(&mut self) {
        self.packed_images.clear();
    }

    /// Get mutable reference to packed images
    pub fn images_mut(&mut self) -> &mut Vec<PackedImage> {
        &mut self.packed_images
    }

    /// Get immutable reference to packed images
    pub fn images(&self) -> &[PackedImage] {
        &self.packed_images
    }

    /// Get temporary directory path for intermediate files
    pub fn temp_dir_path(&self) -> &Path {
        Path::new(self.temp_dir.path())
    }

    /// Create save tasks for all loaded images
    pub fn create_save_tasks(&self) -> Vec<SaveTask> {
        self.packed_images
            .iter()
            .map(|pi| SaveTask {
                path: pi.path.clone(),
                view_exif: pi.view_exif.clone(),
            })
            .collect()
    }
}

/// Task for saving an image (UI-independent)
#[derive(Clone)]
pub struct SaveTask {
    pub path: PathBuf,
    pub view_exif: crate::exif_impl::SimplifiedExif,
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Image grouping features for organizing similar photos
//!
//! This module provides experimental features for grouping similar images.
//! Features are tested for stability before being promoted to main functionality.

use crate::effect::variable_text::VariableText;
use crate::packed_image::PackedImage;

/// Time duration threshold for grouping photos (in seconds)
const TIME_THRESHOLD_SECS: u64 = 300; // 5 minutes default

/// Image similarity threshold (0.0-1.0, higher = more similar required)
const SIMILARITY_THRESHOLD: f64 = 0.85;

/// Configuration for image grouping feature
#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct ImageGroupConfig {
    /// Enable grouping by date (same year/month/day)
    pub group_by_date: bool,

    /// Enable grouping by time proximity
    pub group_by_time: bool,

    /// Enable grouping by camera manufacturer
    pub group_by_camera_mnf: bool,

    /// Enable grouping by camera model
    pub group_by_camera: bool,

    /// Enable grouping by lens model
    pub group_by_lens: bool,

    /// Enable grouping by image similarity (perceptual hash)
    /// WARNING: This feature is experimental and may not work correctly yet
    pub group_by_similarity: bool,

    /// Time threshold in seconds
    pub time_threshold_secs: u64,

    /// Similarity threshold (0.0-1.0)
    pub similarity_threshold: f64,
}

impl Default for ImageGroupConfig {
    fn default() -> Self {
        Self {
            group_by_date: false,
            group_by_time: false, // Disabled by default, but code remains for future use
            group_by_camera_mnf: false,
            group_by_camera: false,
            group_by_lens: false,
            group_by_similarity: false, // Disabled by default - experimental feature
            time_threshold_secs: TIME_THRESHOLD_SECS,
            similarity_threshold: SIMILARITY_THRESHOLD,
        }
    }
}

impl ImageGroupConfig {
    /// Check if any grouping feature is enabled
    pub fn is_any_enabled(&self) -> bool {
        self.group_by_date
            || self.group_by_time
            || self.group_by_camera_mnf
            || self.group_by_camera
            || self.group_by_lens
            || self.group_by_similarity
    }
}

/// A group of similar images
#[derive(Clone, Debug)]
pub struct ImageGroup {
    /// UUIDs of images in this group (stable references independent of Vec position)
    pub image_uuids: Vec<uuid::Uuid>,

    /// Representative datetime for this group (from first image)
    pub datetime: Option<chrono::NaiveDateTime>,

    /// Camera model (if same_camera_only enabled)
    pub camera_model: String,

    /// Group-specific prefix for exported filenames (supports EXIF variables like {camera_model})
    pub prefix: VariableText,

    /// Group-specific postfix for exported filenames (supports EXIF variables like {focal})
    pub postfix: VariableText,

    /// Whether this group is selected for export
    pub selected: bool,

    /// Use default prefix/postfix from export config instead of group-specific
    pub use_default: bool,
}

impl ImageGroup {
    /// Generate suggested prefix based on EXIF data
    /// Priority: camera_mnf -> camera_model -> lens_model -> date
    pub fn suggest_prefix(&self, first_image_exif: &crate::exif_impl::SimplifiedExif) -> String {
        let camera_mnf = &first_image_exif.camera_mnf;
        let camera_model = &first_image_exif.camera_model;
        let lens_model = &first_image_exif.lens_model;

        // Build prefix components based on available data
        let mut components = Vec::new();

        if !camera_mnf.is_empty() {
            components.push("camera_mnf");
        }
        if !camera_model.is_empty() {
            components.push("camera_model");
        }
        if !lens_model.is_empty() {
            components.push("lens_model");
        }

        // Join components with underscores, or fallback to datetime
        if components.is_empty() {
            format!("{{{}}}_", "datetime")
        } else {
            components
                .iter()
                .map(|&c| format!("{{{}}}", c))
                .collect::<Vec<_>>()
                .join("_")
                + "_"
        }
    }

    /// Generate suggested postfix based on EXIF data
    /// Default: date
    pub fn suggest_postfix(&self, _first_image_exif: &crate::exif_impl::SimplifiedExif) -> String {
        format!("_{{{}}}", "datetime")
    }

    /// Apply suggestions to prefix and postfix
    pub fn apply_suggestions(&mut self, first_image_exif: &crate::exif_impl::SimplifiedExif) {
        self.prefix.text = self.suggest_prefix(first_image_exif);
        self.postfix.text = self.suggest_postfix(first_image_exif);
    }
}

/// Calculate Hamming distance between two hashes (number of differing bits)
fn hamming_distance(hash1: u64, hash2: u64) -> u32 {
    (hash1 ^ hash2).count_ones()
}

/// Calculate similarity score between two hashes (0.0-1.0, higher = more similar)
fn hash_similarity(hash1: u64, hash2: u64) -> f64 {
    let distance = hamming_distance(hash1, hash2);
    1.0 - (distance as f64 / 64.0)
}

/// Group images based on similarity criteria
pub fn group_similar_images(images: &[PackedImage], config: &ImageGroupConfig) -> Vec<ImageGroup> {
    if images.is_empty() {
        return Vec::new();
    }

    let mut groups: Vec<ImageGroup> = Vec::new();
    let mut assigned: Vec<bool> = vec![false; images.len()];

    // Use pre-calculated hashes from PackedImage if similarity grouping is enabled
    let hashes: Vec<Option<u64>> = if config.group_by_similarity {
        images.iter().map(|img| img.perceptual_hash).collect()
    } else {
        vec![None; images.len()]
    };

    // Pre-parse datetimes if time grouping is enabled
    let timestamps: Vec<Option<i64>> = if config.group_by_time {
        images
            .iter()
            .map(|img| img.view_exif.datetime.map(|dt| dt.and_utc().timestamp()))
            .collect()
    } else {
        vec![None; images.len()]
    };

    // Pre-extract dates if date grouping is enabled
    let dates: Vec<Option<String>> = if config.group_by_date {
        images
            .iter()
            .map(|img| {
                img.view_exif
                    .datetime
                    .map(|dt| dt.format("%Y:%m:%d").to_string())
            })
            .collect()
    } else {
        vec![None; images.len()]
    };

    for i in 0..images.len() {
        if assigned[i] {
            continue;
        }

        let mut group = ImageGroup {
            image_uuids: vec![images[i].uuid],
            datetime: images[i].view_exif.datetime,
            camera_model: images[i].view_exif.camera_model.clone(),
            prefix: VariableText::new(),
            postfix: VariableText::new(),
            selected: true, // Selected by default
            use_default: false,
        };

        // Apply suggestions based on first image EXIF
        group.apply_suggestions(&images[i].view_exif);

        assigned[i] = true;

        // Find similar images
        for j in (i + 1)..images.len() {
            if assigned[j] {
                continue;
            }

            let img_i = &images[i];
            let img_j = &images[j];

            let mut is_similar = true;

            // Check camera manufacturer match
            if config.group_by_camera_mnf
                && img_i.view_exif.camera_mnf != img_j.view_exif.camera_mnf
            {
                is_similar = false;
            }

            // Check camera model match
            if is_similar
                && config.group_by_camera
                && img_i.view_exif.camera_model != img_j.view_exif.camera_model
            {
                is_similar = false;
            }

            // Check lens model match
            if is_similar
                && config.group_by_lens
                && img_i.view_exif.lens_model != img_j.view_exif.lens_model
            {
                is_similar = false;
            }

            // Check date match (same day)
            if is_similar && config.group_by_date {
                if let (Some(d1), Some(d2)) = (&dates[i], &dates[j]) {
                    if d1 != d2 {
                        is_similar = false;
                    }
                } else {
                    // Cannot compare dates if extraction failed
                    is_similar = false;
                }
            }

            // Check time proximity
            if is_similar && config.group_by_time {
                if let (Some(t1), Some(t2)) = (timestamps[i], timestamps[j]) {
                    let time_diff: u64 = (t1 - t2).unsigned_abs();
                    if time_diff > config.time_threshold_secs {
                        is_similar = false;
                    }
                } else {
                    // Cannot compare times if parsing failed
                    is_similar = false;
                }
            }

            // Check image similarity
            if is_similar && config.group_by_similarity {
                if let (Some(h1), Some(h2)) = (hashes[i], hashes[j]) {
                    let similarity = hash_similarity(h1, h2);
                    if similarity < config.similarity_threshold {
                        is_similar = false;
                    }
                } else {
                    // Cannot compare hashes if calculation failed
                    is_similar = false;
                }
            }

            if is_similar {
                group.image_uuids.push(images[j].uuid);
                assigned[j] = true;
            }
        }

        groups.push(group);
    }

    // Sort groups by datetime
    groups.sort_by(|a, b| a.datetime.cmp(&b.datetime));

    groups
}

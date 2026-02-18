/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Pipeline presets — named, serializable pipeline configurations.
//!
//! Presets allow users to save, load, and share complete pipeline configurations.
//! They wrap `PipelineConfig` with metadata (name, description, tags).
//!
//! # Storage
//! Presets are stored as JSON files. The native side manages file I/O;
//! this module handles serialization/deserialization and preset management.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::config::PipelineConfig;

/// A named pipeline preset with metadata.
///
/// Wraps `PipelineConfig` with user-facing metadata for preset management.
/// The `id` is auto-generated and stable across renames.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePreset {
    /// Unique identifier (auto-generated, stable across renames).
    pub id: Uuid,

    /// User-visible preset name.
    pub name: String,

    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional tags for categorization (e.g. ["portrait", "warm"]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// The pipeline configuration.
    pub config: PipelineConfig,
}

impl PipelinePreset {
    /// Create a new preset from an existing pipeline config.
    pub fn new(name: impl Into<String>, config: PipelineConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            tags: Vec::new(),
            config,
        }
    }

    /// Create a preset with description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Create a preset with tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// In-memory collection of presets.
///
/// The native side is responsible for persisting presets to disk.
/// This struct manages the runtime collection for listing/lookup.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PresetCollection {
    pub presets: Vec<PipelinePreset>,
}

impl PresetCollection {
    pub fn new() -> Self {
        Self {
            presets: Vec::new(),
        }
    }

    /// Add a preset to the collection.
    pub fn add(&mut self, preset: PipelinePreset) {
        self.presets.push(preset);
    }

    /// Find a preset by ID.
    pub fn find_by_id(&self, id: Uuid) -> Option<&PipelinePreset> {
        self.presets.iter().find(|p| p.id == id)
    }

    /// Find a preset by name.
    pub fn find_by_name(&self, name: &str) -> Option<&PipelinePreset> {
        self.presets.iter().find(|p| p.name == name)
    }

    /// Remove a preset by ID. Returns the removed preset if found.
    pub fn remove(&mut self, id: Uuid) -> Option<PipelinePreset> {
        if let Some(pos) = self.presets.iter().position(|p| p.id == id) {
            Some(self.presets.remove(pos))
        } else {
            None
        }
    }

    /// Serialize the entire collection to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::color_adjustments::ColorAdjustments;
    use crate::pipeline::v1::stages::{PipelineStage, StageEntry};

    #[test]
    fn test_preset_serde_roundtrip() {
        let mut config = PipelineConfig::default();
        let mut adj = ColorAdjustments::new();
        adj.enabled = true;
        adj.exposure = 0.5;
        config
            .stages
            .push(StageEntry::enabled(PipelineStage::ColorAdjustments(adj)));

        let preset = PipelinePreset::new("Warm Portrait", config)
            .with_description("Warm tones for portrait photography")
            .with_tags(vec!["portrait".into(), "warm".into()]);

        let json = preset.to_json().unwrap();
        let deserialized = PipelinePreset::from_json(&json).unwrap();

        assert_eq!(deserialized.id, preset.id);
        assert_eq!(deserialized.name, "Warm Portrait");
        assert_eq!(
            deserialized.description.as_deref(),
            Some("Warm tones for portrait photography")
        );
        assert_eq!(deserialized.tags, vec!["portrait", "warm"]);
        assert_eq!(deserialized.config.stages.len(), 1);
    }

    #[test]
    fn test_preset_collection() {
        let mut collection = PresetCollection::new();

        let preset1 = PipelinePreset::new("Preset A", PipelineConfig::default());
        let preset2 = PipelinePreset::new("Preset B", PipelineConfig::default());
        let id1 = preset1.id;
        let id2 = preset2.id;

        collection.add(preset1);
        collection.add(preset2);

        assert_eq!(collection.presets.len(), 2);
        assert!(collection.find_by_id(id1).is_some());
        assert!(collection.find_by_name("Preset B").is_some());

        let removed = collection.remove(id1);
        assert!(removed.is_some());
        assert_eq!(collection.presets.len(), 1);
        assert!(collection.find_by_id(id2).is_some());
    }

    #[test]
    fn test_collection_serde_roundtrip() {
        let mut collection = PresetCollection::new();
        collection.add(PipelinePreset::new("A", PipelineConfig::default()));
        collection.add(PipelinePreset::new("B", PipelineConfig::default()));

        let json = collection.to_json().unwrap();
        let deserialized = PresetCollection::from_json(&json).unwrap();
        assert_eq!(deserialized.presets.len(), 2);
    }
}

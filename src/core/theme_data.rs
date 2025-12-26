/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Theme configuration data without GUI dependencies

use serde::{Deserialize, Serialize};

/// Theme types available in Chama Optics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeType {
    /// No theme (original image)
    Nothing,
    /// Simple frame border
    JustFrame,
    /// One line text
    OneLine,
    /// Two line text
    TwoLine,
    /// Film-style border
    Film,
    /// Film with date
    FilmDate,
    /// Film with glow effect
    FilmGlow,
    /// Camera strap style
    Strap,
    /// Monitor/screen frame
    Monitor,
    /// "Shot on" single line
    ShotOnOneLine,
    /// "Shot on" two lines
    ShotOnTwoLine,
    /// Lightroom style
    Lightroom,
}

impl ThemeType {
    /// Get the unique name identifier for this theme
    pub fn unique_name(&self) -> &'static str {
        match self {
            ThemeType::Nothing => "nothing",
            ThemeType::JustFrame => "just_frame",
            ThemeType::OneLine => "one_line",
            ThemeType::TwoLine => "two_line",
            ThemeType::Film => "film",
            ThemeType::FilmDate => "film_date",
            ThemeType::FilmGlow => "film_glow",
            ThemeType::Strap => "strap",
            ThemeType::Monitor => "monitor",
            ThemeType::ShotOnOneLine => "shot_on_one_line",
            ThemeType::ShotOnTwoLine => "shot_on_two_line",
            ThemeType::Lightroom => "lightroom",
        }
    }

    /// Get display label for this theme
    pub fn label(&self) -> &'static str {
        match self {
            ThemeType::Nothing => "No Theme",
            ThemeType::JustFrame => "Just Frame",
            ThemeType::OneLine => "One Line",
            ThemeType::TwoLine => "Two Line",
            ThemeType::Film => "Film",
            ThemeType::FilmDate => "Film + Date",
            ThemeType::FilmGlow => "Film Glow",
            ThemeType::Strap => "Strap",
            ThemeType::Monitor => "Monitor",
            ThemeType::ShotOnOneLine => "Shot On (1 Line)",
            ThemeType::ShotOnTwoLine => "Shot On (2 Lines)",
            ThemeType::Lightroom => "Lightroom",
        }
    }

    /// Get all available theme types
    pub fn all() -> Vec<ThemeType> {
        vec![
            ThemeType::Nothing,
            ThemeType::JustFrame,
            ThemeType::OneLine,
            ThemeType::TwoLine,
            ThemeType::Film,
            ThemeType::FilmDate,
            ThemeType::FilmGlow,
            ThemeType::Strap,
            ThemeType::Monitor,
            ThemeType::ShotOnOneLine,
            ThemeType::ShotOnTwoLine,
            ThemeType::Lightroom,
        ]
    }

    /// Parse theme type from unique name
    pub fn from_name(name: &str) -> Option<ThemeType> {
        match name {
            "nothing" => Some(ThemeType::Nothing),
            "just_frame" => Some(ThemeType::JustFrame),
            "one_line" => Some(ThemeType::OneLine),
            "two_line" => Some(ThemeType::TwoLine),
            "film" => Some(ThemeType::Film),
            "film_date" => Some(ThemeType::FilmDate),
            "film_glow" => Some(ThemeType::FilmGlow),
            "strap" => Some(ThemeType::Strap),
            "monitor" => Some(ThemeType::Monitor),
            "shot_on_one_line" => Some(ThemeType::ShotOnOneLine),
            "shot_on_two_line" => Some(ThemeType::ShotOnTwoLine),
            "lightroom" => Some(ThemeType::Lightroom),
            _ => None,
        }
    }
}

/// Theme configuration (headless version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Selected theme type
    pub theme_type: ThemeType,

    /// Custom theme parameters (theme-specific)
    /// Stored as JSON-serializable key-value pairs
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            theme_type: ThemeType::Nothing,
            parameters: std::collections::HashMap::new(),
        }
    }
}

impl ThemeConfig {
    /// Create a new theme configuration
    pub fn new(theme_type: ThemeType) -> Self {
        Self {
            theme_type,
            parameters: std::collections::HashMap::new(),
        }
    }

    /// Set a parameter value
    pub fn set_parameter<T: serde::Serialize>(
        &mut self,
        key: String,
        value: T,
    ) -> Result<(), serde_json::Error> {
        self.parameters.insert(key, serde_json::to_value(value)?);
        Ok(())
    }

    /// Get a parameter value
    pub fn get_parameter<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.parameters
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

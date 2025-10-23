/*

* SPDX-FileCopyrightText: © 2025 Jinwoo Park ([pmnxis@gmail.com](mailto:pmnxis@gmail.com))
*
* SPDX-License-Identifier: LicenseRef-Non-AI-MIT
  */

//! collection of themes

pub(crate) mod film;
pub(crate) mod nothing;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

pub fn color32_to_rgba(color: egui::Color32) -> image::Rgba<u8> {
    let [r, g, b, a] = color.to_array();
    image::Rgba([r, g, b, a])
}

pub trait Theme {
    /// return unique name of theme
    fn unique_name(&self) -> &'static str;

    /// return label to show on UI
    fn label(&self) -> std::borrow::Cow<'static, str>;

    fn apply(
        &self,
        pi: &crate::packed_image::PackedImage,
        export_config: &crate::export_config::ExportConfig,
        output_path: &std::path::Path,
    ) -> Result<(), image::ImageError>;

    fn ui_config(&mut self, ui: &mut egui::Ui);
}
/// Serializable state used for saving/loading preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeRegistryState {
    pub names: Vec<String>,
    pub selected: usize,
}

/// Runtime registry that holds real Theme trait objects.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeRegistry {
    #[serde(skip)]
    pub themes: Vec<Arc<RwLock<dyn Theme>>>,
    pub selected: usize,
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeRegistry {
    pub fn new() -> Self {
        let film = Arc::new(RwLock::new(film::Film::default())) as Arc<RwLock<dyn Theme>>;
        let nothing_theme =
            Arc::new(RwLock::new(nothing::Nothing::default())) as Arc<RwLock<dyn Theme>>;

        Self {
            themes: vec![film, nothing_theme],
            selected: 0,
        }
    }

    pub fn from_state(state: ThemeRegistryState) -> Self {
        let available: Vec<Arc<RwLock<dyn Theme>>> = vec![
            Arc::new(RwLock::new(film::Film::default())) as Arc<RwLock<dyn Theme>>,
            Arc::new(RwLock::new(nothing::Nothing::default())) as Arc<RwLock<dyn Theme>>,
        ];

        let mut ordered = Vec::new();
        let mut remaining = available.clone();

        for saved_name in &state.names {
            if let Some(pos) = remaining.iter().position(|t: &Arc<RwLock<dyn Theme>>| {
                t.read().unwrap().unique_name() == saved_name
            }) {
                ordered.push(remaining.remove(pos));
            }
        }

        ordered.extend(remaining);

        let selected = state.selected.min(ordered.len().saturating_sub(1));
        Self {
            themes: ordered,
            selected,
        }
    }

    pub fn to_state(&self) -> ThemeRegistryState {
        ThemeRegistryState {
            names: self
                .themes
                .iter()
                .map(|t| t.read().unwrap().unique_name().to_string())
                .collect(),
            selected: self.selected.min(self.themes.len().saturating_sub(1)),
        }
    }

    pub fn selected_theme_read(&self) -> std::sync::RwLockReadGuard<'_, dyn Theme> {
        self.themes[self.selected].read().unwrap()
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(t!("theme.selector"));
            egui::ComboBox::from_id_salt("theme_selector")
                .selected_text(self.themes[self.selected].read().unwrap().label())
                .show_ui(ui, |ui| {
                    for (i, theme) in self.themes.iter().enumerate() {
                        if ui
                            .selectable_label(i == self.selected, theme.read().unwrap().label())
                            .clicked()
                        {
                            self.selected = i;
                        }
                    }
                });

            ui.collapsing(t!("theme.settings"), |ui| {
                self.themes[self.selected].write().unwrap().ui_config(ui);
            });
        });
    }
}

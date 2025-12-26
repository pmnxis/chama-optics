/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::fonts::variable_font::*;
use egui::{self, Align, TextEdit, Ui};
use rust_i18n::t;
use std::borrow::Cow;

/// Available EXIF fields for autocomplete
const EXIF_FIELDS: &[ExifField] = &[
    ExifField {
        name: "camera_mnf",
        description: "Camera manufacturer",
        example: "Canon",
    },
    ExifField {
        name: "camera_model",
        description: "Camera model",
        example: "EOS R5",
    },
    ExifField {
        name: "lens_mnf",
        description: "Lens manufacturer",
        example: "Canon",
    },
    ExifField {
        name: "lens_model",
        description: "Lens model",
        example: "RF24-105mm F4 L IS USM",
    },
    ExifField {
        name: "focal",
        description: "Focal length (mm)",
        example: "35",
    },
    ExifField {
        name: "fnumber",
        description: "F-number / Aperture",
        example: "1.8",
    },
    ExifField {
        name: "exposure",
        description: "Exposure time",
        example: "1/125",
    },
    ExifField {
        name: "iso_speed",
        description: "ISO speed",
        example: "200",
    },
    ExifField {
        name: "datetime",
        description: "Date and time",
        example: "2025-01-15 14:30:00",
    },
    #[cfg(feature = "desktop")]
    ExifField {
        name: "photo_style",
        description: "Photo style (Panasonic/Nikon/Sony)",
        example: "Standard",
    },
    #[cfg(feature = "desktop")]
    ExifField {
        name: "lut_detail",
        description: "LUT detail (Panasonic/Nikon/Sony)",
        example: "V-Log",
    },
];

#[derive(Clone)]
struct ExifField {
    name: &'static str,
    description: &'static str,
    example: &'static str,
}

/// Autocomplete state for variable text input
#[derive(Default, Clone)]
struct AutocompleteState {
    /// Whether autocomplete popup is visible
    show_popup: bool,
    /// Current cursor position in the text
    cursor_pos: Option<usize>,
    /// Start position of the current variable being typed
    variable_start: Option<usize>,
    /// Current partial variable text (without the opening `{`)
    partial_variable: String,
    /// Selected index in the autocomplete list
    selected_index: usize,
}

impl AutocompleteState {
    fn new() -> Self {
        Self::default()
    }

    /// Reset autocomplete state
    fn reset(&mut self) {
        self.show_popup = false;
        self.cursor_pos = None;
        self.variable_start = None;
        self.partial_variable.clear();
        self.selected_index = 0;
    }

    /// Get filtered list of EXIF fields matching the partial input
    fn get_filtered_fields(&self) -> Vec<&'static ExifField> {
        if self.partial_variable.is_empty() {
            EXIF_FIELDS.iter().collect()
        } else {
            EXIF_FIELDS
                .iter()
                .filter(|field| field.name.starts_with(&self.partial_variable))
                .collect()
        }
    }

    /// Update autocomplete state based on text and cursor position
    fn update_from_text(&mut self, text: &str, cursor_pos: usize) {
        self.cursor_pos = Some(cursor_pos);

        // Ensure cursor_pos is a valid char boundary
        if !text.is_char_boundary(cursor_pos) {
            self.reset();
            return;
        }

        // Find if we're inside a variable (between `{` and `}` or at the end)
        let before_cursor = &text[..cursor_pos];

        if let Some(last_open) = before_cursor.rfind('{') {
            // Check if there's a closing brace between the opening brace and cursor
            let after_open = &before_cursor[last_open + 1..];
            if !after_open.contains('}') {
                // We're inside a variable
                self.show_popup = true;
                self.variable_start = Some(last_open);
                self.partial_variable = after_open.to_string();

                // Ensure selected index is within bounds
                let filtered = self.get_filtered_fields();
                if self.selected_index >= filtered.len() && !filtered.is_empty() {
                    self.selected_index = 0;
                }
                return;
            }
        }

        // Not inside a variable
        self.reset();
    }

    /// Insert the selected field at the cursor position
    fn insert_selected_field(&mut self, text: &mut String) -> bool {
        if !self.show_popup {
            return false;
        }

        let filtered = self.get_filtered_fields();
        if filtered.is_empty() {
            return false;
        }

        if let (Some(start), Some(_cursor)) = (self.variable_start, self.cursor_pos) {
            // Ensure start is a valid char boundary
            if !text.is_char_boundary(start) {
                self.reset();
                return false;
            }

            let field = filtered[self.selected_index];

            // Find the end position - either the closing `}` or the cursor position
            // This allows replacing existing completed variables
            let after_start = &text[start..];
            let end = if let Some(close_pos) = after_start.find('}') {
                start + close_pos + 1 // Include the `}` in the replacement
            } else {
                // No closing brace found, use cursor position
                self.cursor_pos.unwrap_or(text.len())
            };

            // Ensure end is a valid char boundary
            if !text.is_char_boundary(end) {
                self.reset();
                return false;
            }

            // Replace from `{` to `}` (or cursor) with the new variable
            text.replace_range(start..end, &format!("{{{}}}", field.name));

            self.reset();
            return true;
        }

        false
    }
}

#[rustfmt::skip]
#[repr(usize)]
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub enum VariableOrNot {
    Variable(BuiltinVariableFontIndex),
    Others(crate::fonts::font_unify::FontSelection),
}

impl std::default::Default for VariableOrNot {
    fn default() -> Self {
        Self::Variable(BuiltinVariableFontIndex::Barlow)
    }
}

pub struct VariableTextSlotDefault {
    pub text: &'static str,
    pub weight: u16,
    // variable index
    pub font_index: BuiltinVariableFontIndex,
    // fixed index
    pub fixed_index: Option<crate::BuiltinFontIndex>,
    pub prefer_fixed: bool,
}

impl VariableTextSlotDefault {
    pub const fn with_barlow(default: &'static str) -> Self {
        Self {
            text: default,
            weight: 300, // todo - get method with const gently
            font_index: BuiltinVariableFontIndex::Barlow,
            fixed_index: None,
            prefer_fixed: false,
        }
    }

    pub const fn with_barlow_weight(default: &'static str, weight: u16) -> Self {
        Self {
            text: default,
            weight,
            font_index: BuiltinVariableFontIndex::Barlow,
            fixed_index: None,
            prefer_fixed: false,
        }
    }

    pub const fn with_digital7(default: &'static str) -> Self {
        Self {
            text: default,
            weight: 300, // todo - get method with const gently
            font_index: BuiltinVariableFontIndex::Barlow, // don't need for default
            fixed_index: Some(crate::BuiltinFontIndex::Digital7),
            prefer_fixed: true, // will select fixed_index
        }
    }
}

impl From<VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: VariableTextSlotDefault) -> Self {
        if let Some(Some(x)) = value.prefer_fixed.then_some(value.fixed_index) {
            Self {
                text: value.text.into(),
                weight: value.weight,
                font_index: VariableOrNot::Others(crate::FONTS_UNIFY.builtin_select(x)),
                autocomplete_state: AutocompleteState::new(),
            }
        } else {
            Self {
                text: value.text.into(),
                weight: value.weight,
                font_index: VariableOrNot::Variable(value.font_index),
                autocomplete_state: AutocompleteState::new(),
            }
        }
    }
}
impl From<&VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: &VariableTextSlotDefault) -> Self {
        if let Some(Some(x)) = value.prefer_fixed.then_some(value.fixed_index) {
            Self {
                text: value.text.into(),
                weight: value.weight,
                font_index: VariableOrNot::Others(crate::FONTS_UNIFY.builtin_select(x)),
                autocomplete_state: AutocompleteState::new(),
            }
        } else {
            Self {
                text: value.text.into(),
                weight: value.weight,
                font_index: VariableOrNot::Variable(value.font_index),
                autocomplete_state: AutocompleteState::new(),
            }
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct VariableTextSlot {
    pub text: String,
    pub weight: u16,
    // todo - future selection variable fonts
    pub font_index: VariableOrNot,
    #[serde(skip)]
    autocomplete_state: AutocompleteState,
}

impl VariableTextSlot {
    #[allow(dead_code)]
    pub fn new(default: &'static str, weight: u16) -> Self {
        Self {
            text: default.to_string(),
            weight,
            font_index: VariableOrNot::default(),
            autocomplete_state: AutocompleteState::new(),
        }
    }

    #[allow(dead_code)]
    pub fn from_default(default: &'static VariableTextSlotDefault) -> Self {
        default.into()
    }

    pub fn format_custom(&self, exif: &crate::image::exif_impl::SimplifiedExif) -> String {
        exif.format_custom(self.text.clone())
    }

    pub fn get_font(&self) -> ab_glyph::FontArc {
        // todo - resolve &'static, &, * hell
        // todo - Result<T,E>
        match &self.font_index {
            VariableOrNot::Variable(var) => var.get_font_by_weight(self.weight).clone(),
            VariableOrNot::Others(others) => match crate::FONTS_UNIFY.search(others) {
                Ok(x) => x,
                Err(e) => {
                    log::error!("{e}");
                    BuiltinVariableFontIndex::Barlow
                        .get_font_by_weight(self.weight)
                        .clone()
                }
            },
        }
    }

    /// return false on bool tuple when it's not variable font
    pub fn get_font_with_new_weight(&self, weight: u16) -> (ab_glyph::FontArc, bool) {
        // todo - resolve &'static, &, * hell
        // todo - Result<T,E>
        match &self.font_index {
            VariableOrNot::Variable(var) => (var.get_font_by_weight(weight).clone(), true),
            VariableOrNot::Others(others) => (
                match crate::FONTS_UNIFY.search(others) {
                    Ok(x) => x,
                    Err(e) => {
                        log::error!("{e}");
                        BuiltinVariableFontIndex::Barlow
                            .get_font_by_weight(self.weight)
                            .clone()
                    }
                },
                false,
            ),
        }
    }

    pub fn text_dimensions(&self, scale: ab_glyph::PxScale, txt: impl AsRef<str>) -> (f32, f32) {
        crate::theme::text_dimensions(scale, &self.get_font(), txt.as_ref())
    }

    #[allow(unused)]
    pub fn format_dimensions(
        &self,
        scale: ab_glyph::PxScale,
        exif: &crate::image::exif_impl::SimplifiedExif,
        additional_text: impl AsRef<str> + std::fmt::Display,
    ) -> (f32, f32) {
        let txt = exif.format_custom(format!("{}{}", self.text, additional_text));

        self.text_dimensions(scale, txt)
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        label: Cow<'static, str>,
        default: &'static VariableTextSlotDefault,
    ) {
        // ensure outside is grid ui
        ui.vertical(|ui| {
            ui.add_space(2.0);
            ui.label(label.clone());
            ui.add_space(27.0);
        });

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add_space(2.0);

                // Use radio button style for variant selection
                let is_variable = matches!(self.font_index, VariableOrNot::Variable(_));
                if ui
                    .selectable_label(is_variable, t!("fonts_selector.variable.label"))
                    .on_hover_text(t!("fonts_selector.variable.hint"))
                    .clicked()
                    && !is_variable
                {
                    self.font_index = VariableOrNot::Variable(default.font_index);
                }

                let is_others = matches!(self.font_index, VariableOrNot::Others(_));
                if ui
                    .selectable_label(is_others, t!("fonts_selector.others.label"))
                    .on_hover_text(t!("fonts_selector.others.hint"))
                    .clicked()
                    && !is_others
                {
                    self.font_index = VariableOrNot::Others(
                        crate::FONTS_UNIFY.builtin_select(default.fixed_index.unwrap_or_default()),
                    );
                }

                if let VariableOrNot::Variable(ref mut variable_select) = self.font_index {
                    variable_select.update_ui(ui, label.clone());

                    let (start, end) = variable_select.get_font().range();
                    ui.add(egui::Slider::new(&mut self.weight, start..=end).step_by(100.0));
                } else if let VariableOrNot::Others(ref mut font_select) = self.font_index {
                    font_select.update_ui(ui, label.clone());
                };

                if ui.button("↺").clicked() {
                    *self = default.into();
                }
            });

            // Text edit with autocomplete
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 8.0).max(16.0);
                self.render_text_edit_with_autocomplete(ui, width, &label);
            });
            ui.add_space(4.0);
        });
    }

    /// Render a text edit with autocomplete functionality
    fn render_text_edit_with_autocomplete(
        &mut self,
        ui: &mut Ui,
        width: f32,
        id_salt: impl std::hash::Hash,
    ) -> egui::Response {
        let text_edit_id = ui.id().with(&id_salt).with("autocomplete_text_edit");

        let response = ui.add_sized(
            [width, 23.0],
            TextEdit::singleline(&mut self.text)
                .vertical_align(Align::Center)
                .id(text_edit_id),
        );

        // Get cursor position from the text edit state
        if let Some(mut text_edit_state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id)
            && let Some(cursor_range) = text_edit_state.cursor.char_range()
        {
            let cursor_pos = cursor_range.primary.index;
            self.autocomplete_state
                .update_from_text(&self.text, cursor_pos);

            // Handle keyboard input when popup is shown
            if self.autocomplete_state.show_popup {
                let filtered = self.autocomplete_state.get_filtered_fields();

                if !filtered.is_empty() {
                    ui.input(|i| {
                        // Arrow down
                        if i.key_pressed(egui::Key::ArrowDown) {
                            self.autocomplete_state.selected_index =
                                (self.autocomplete_state.selected_index + 1)
                                    .min(filtered.len() - 1);
                        }
                        // Arrow up
                        if i.key_pressed(egui::Key::ArrowUp) {
                            self.autocomplete_state.selected_index =
                                self.autocomplete_state.selected_index.saturating_sub(1);
                        }
                        // Tab or Enter to accept
                        if (i.key_pressed(egui::Key::Tab) || i.key_pressed(egui::Key::Enter))
                            && self
                                .autocomplete_state
                                .insert_selected_field(&mut self.text)
                        {
                            // Update cursor position after insertion
                            if let Some(start) = self.autocomplete_state.variable_start {
                                let new_cursor_pos = start
                                    + filtered[self.autocomplete_state.selected_index].name.len()
                                    + 2; // +2 for `{}`
                                text_edit_state.cursor.set_char_range(Some(
                                    egui::text::CCursorRange::one(egui::text::CCursor::new(
                                        new_cursor_pos,
                                    )),
                                ));
                            }
                        }
                        // Escape to close
                        if i.key_pressed(egui::Key::Escape) {
                            self.autocomplete_state.reset();
                        }
                    });
                }
            }

            text_edit_state.store(ui.ctx(), text_edit_id);
        }

        // Show autocomplete popup
        if self.autocomplete_state.show_popup {
            let filtered = self.autocomplete_state.get_filtered_fields();

            if !filtered.is_empty() {
                // Calculate popup position (below the text edit)
                let popup_pos = response.rect.left_bottom() + egui::vec2(0.0, 2.0);

                egui::Area::new(ui.id().with(&id_salt).with("autocomplete_popup"))
                    .fixed_pos(popup_pos)
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.set_min_width(300.0);
                            ui.set_max_height(200.0);

                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for (idx, field) in filtered.iter().enumerate() {
                                        let is_selected =
                                            idx == self.autocomplete_state.selected_index;

                                        ui.horizontal(|ui| {
                                            // Left: Field name (monospace)
                                            let item_response = ui.selectable_label(
                                                is_selected,
                                                egui::RichText::new(field.name).monospace(),
                                            );

                                            // Check for click first
                                            let was_clicked = item_response.clicked();

                                            // Show example on hover
                                            item_response.on_hover_ui(|ui| {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "Example: {}",
                                                        field.example
                                                    ))
                                                    .weak(),
                                                );
                                            });

                                            if was_clicked {
                                                self.autocomplete_state.selected_index = idx;
                                                self.autocomplete_state
                                                    .insert_selected_field(&mut self.text);
                                            }

                                            // Right: Description (always visible)
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new(field.description)
                                                            .small()
                                                            .weak(),
                                                    );
                                                },
                                            );
                                        });
                                    }
                                });
                        });
                    });
            }
        }

        response
    }
}

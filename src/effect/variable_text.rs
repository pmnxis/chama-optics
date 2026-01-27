/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use crate::fonts::variable_font::*;

#[cfg(not(feature = "ios_integration"))]
use std::borrow::Cow;

#[cfg(not(feature = "ios_integration"))]
use egui::{self, Align, TextEdit, Ui};

// Global fonts base directory for iOS (set via FFI)
#[cfg(feature = "ios_integration")]
static FONTS_BASE_DIR: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// Set the fonts base directory for iOS font loading
/// Call this from FFI before rendering themes
#[cfg(feature = "ios_integration")]
pub fn set_fonts_base_directory(path: &str) {
    if let Ok(mut dir) = FONTS_BASE_DIR.write() {
        *dir = path.to_string();
        log::info!("Fonts base directory set to: {}", path);
    }
}

/// Get the fonts base directory
#[cfg(feature = "ios_integration")]
pub fn get_fonts_base_directory() -> String {
    FONTS_BASE_DIR.read().map(|d| d.clone()).unwrap_or_default()
}

/// Available EXIF fields for autocomplete
#[cfg(not(feature = "ios_integration"))]
pub const EXIF_FIELDS: &[ExifField] = &[
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
#[allow(dead_code)] // there's possibility use again in iOS
pub struct ExifField {
    pub name: &'static str,
    pub description: &'static str,
    pub example: &'static str,
}

// Until iOS FFI support autocomplete, it's exclusive
/// Simple variable text for filename patterns (without font information)
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, PartialEq)]
#[cfg(not(feature = "ios_integration"))]
pub struct VariableText {
    pub text: String,
    #[serde(skip)]
    #[cfg(not(feature = "ios_integration"))]
    autocomplete_state: AutocompleteState,
}

// Until iOS FFI support autocomplete, it's exclusive
#[cfg(not(feature = "ios_integration"))]
impl VariableText {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            #[cfg(not(feature = "ios_integration"))]
            autocomplete_state: AutocompleteState::new(),
        }
    }

    /// Render a text edit with autocomplete functionality
    #[cfg(not(feature = "ios_integration"))]
    pub fn render_text_edit_with_autocomplete(
        &mut self,
        ui: &mut egui::Ui,
        width: f32,
        id_salt: impl std::hash::Hash,
    ) -> egui::Response {
        render_text_edit_autocomplete_impl(
            &mut self.text,
            &mut self.autocomplete_state,
            ui,
            width,
            id_salt,
        )
    }
}

// Until iOS FFI support autocomplete, it's exclusive
#[cfg(not(feature = "ios_integration"))]
impl Default for VariableText {
    fn default() -> Self {
        Self::new()
    }
}

/// Autocomplete state for variable text input
#[cfg(not(feature = "ios_integration"))]
#[derive(Default, Clone, Debug, PartialEq)]
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
    /// Pending text and cursor position to apply on next frame
    /// (new_text, cursor_position) - applied before TextEdit renders
    pending_insertion: Option<(String, usize)>,
}

#[cfg(not(feature = "ios_integration"))]
impl AutocompleteState {
    fn new() -> Self {
        Self::default()
    }

    /// Reset autocomplete state (preserves pending_cursor_pos for next frame)
    fn reset(&mut self) {
        self.show_popup = false;
        self.cursor_pos = None;
        self.variable_start = None;
        self.partial_variable.clear();
        self.selected_index = 0;
        // Note: pending_cursor_pos is NOT reset here - it needs to persist until applied
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
    #[cfg(not(feature = "ios_integration"))]
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

    /// Compute the text insertion result without modifying state yet
    /// Returns Some((new_text, new_cursor_position)) on success, None on failure
    fn compute_insertion(&self, text: &str) -> Option<(String, usize)> {
        if !self.show_popup {
            return None;
        }

        let filtered = self.get_filtered_fields();
        if filtered.is_empty() {
            return None;
        }

        if let (Some(start), Some(_cursor)) = (self.variable_start, self.cursor_pos) {
            // Ensure start is a valid char boundary
            if !text.is_char_boundary(start) {
                return None;
            }

            let field = filtered[self.selected_index];
            let field_name_len = field.name.len();

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
                return None;
            }

            // Build the new text
            let mut new_text = text.to_string();
            new_text.replace_range(start..end, &format!("{{{}}}", field.name));

            // Calculate new cursor position: start + 1 (for `{`) + field_name_len + 1 (for `}`)
            let new_cursor_pos = start + field_name_len + 2;

            return Some((new_text, new_cursor_pos));
        }

        None
    }

    /// Schedule an insertion to be applied on the next frame
    fn schedule_insertion(&mut self, text: &str) -> bool {
        if let Some((new_text, new_cursor_pos)) = self.compute_insertion(text) {
            self.pending_insertion = Some((new_text, new_cursor_pos));
            self.reset();
            true
        } else {
            false
        }
    }
}

// Desktop version with FontSelection support for system fonts
#[cfg(not(feature = "ios_integration"))]
#[rustfmt::skip]
#[repr(usize)]
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub enum VariableOrNot {
    Variable(BuiltinVariableFontIndex),
    Others(crate::fonts::font_unify::FontSelection),
}

// iOS version - only Variable variant since fonts are loaded from files
#[cfg(feature = "ios_integration")]
#[rustfmt::skip]
#[repr(usize)]
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub enum VariableOrNot {
    Variable(BuiltinVariableFontIndex),
}

impl std::default::Default for VariableOrNot {
    fn default() -> Self {
        Self::Variable(BuiltinVariableFontIndex::Barlow)
    }
}

// Desktop version - with system font selection support
#[cfg(not(feature = "ios_integration"))]
pub struct VariableTextSlotDefault {
    pub text: &'static str,
    pub weight: u16,
    pub font_index: BuiltinVariableFontIndex,
    pub fixed_index: Option<crate::BuiltinFontIndex>,
    pub prefer_fixed: bool,
}

// iOS version - fonts loaded from files
#[cfg(feature = "ios_integration")]
pub struct VariableTextSlotDefault {
    pub text: &'static str,
    pub weight: u16,
    pub font_index: BuiltinVariableFontIndex,
    pub font_file: &'static str,
}

/// Default font filenames for different font types
#[cfg(feature = "ios_integration")]
pub const FONT_FILE_BARLOW: &str = "Barlow-Variable-Remapped.ttf";

#[cfg(feature = "ios_integration")]
pub const FONT_FILE_DIGITAL7: &str = "digital-7.ttf";

// Desktop implementation
#[cfg(not(feature = "ios_integration"))]
impl VariableTextSlotDefault {
    pub const fn with_barlow(default: &'static str) -> Self {
        Self {
            text: default,
            weight: 300,
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
            weight: 300,
            font_index: BuiltinVariableFontIndex::Barlow,
            fixed_index: Some(crate::BuiltinFontIndex::Digital7),
            prefer_fixed: true,
        }
    }
}

// iOS implementation
#[cfg(feature = "ios_integration")]
impl VariableTextSlotDefault {
    pub const fn with_barlow(default: &'static str) -> Self {
        Self {
            text: default,
            weight: 300,
            font_index: BuiltinVariableFontIndex::Barlow,
            font_file: FONT_FILE_BARLOW,
        }
    }

    pub const fn with_barlow_weight(default: &'static str, weight: u16) -> Self {
        Self {
            text: default,
            weight,
            font_index: BuiltinVariableFontIndex::Barlow,
            font_file: FONT_FILE_BARLOW,
        }
    }

    pub const fn with_digital7(default: &'static str) -> Self {
        Self {
            text: default,
            weight: 300,
            font_index: BuiltinVariableFontIndex::Barlow,
            font_file: FONT_FILE_DIGITAL7,
        }
    }

    // /// Create with custom font file
    // #[allow(dead_code)] // this is scar of when tried with iOS integration
    // pub const fn with_font_file(
    //     default: &'static str,
    //     weight: u16,
    //     font_file: &'static str,
    // ) -> Self {
    //     Self {
    //         text: default,
    //         weight,
    //         font_index: BuiltinVariableFontIndex::Barlow,
    //         fixed_index: None,
    //         prefer_fixed: false,
    //         font_file,
    //     }
    // }
}

// Desktop version - supports both Variable and Others variants
#[cfg(not(feature = "ios_integration"))]
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

// iOS version - only Variable variant, fonts loaded from font_file
#[cfg(feature = "ios_integration")]
impl From<VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: VariableTextSlotDefault) -> Self {
        Self {
            text: value.text.into(),
            weight: value.weight,
            font_index: VariableOrNot::Variable(value.font_index),
            font_file: value.font_file.to_string(),
            #[cfg(not(feature = "ios_integration"))]
            autocomplete_state: AutocompleteState::new(),
        }
    }
}

// Desktop version - supports both Variable and Others variants
#[cfg(not(feature = "ios_integration"))]
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

// iOS version - only Variable variant, fonts loaded from font_file
#[cfg(feature = "ios_integration")]
impl From<&VariableTextSlotDefault> for VariableTextSlot {
    fn from(value: &VariableTextSlotDefault) -> Self {
        Self {
            text: value.text.into(),
            weight: value.weight,
            font_index: VariableOrNot::Variable(value.font_index),
            font_file: value.font_file.to_string(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct VariableTextSlot {
    pub text: String,
    pub weight: u16,
    // todo - future selection variable fonts
    pub font_index: VariableOrNot,
    /// Font filename for iOS path construction (just filename with extension)
    #[cfg(feature = "ios_integration")]
    #[serde(default)]
    pub font_file: String,
    #[serde(skip)]
    #[cfg(not(feature = "ios_integration"))]
    autocomplete_state: AutocompleteState,
}

impl VariableTextSlot {
    #[allow(dead_code)]
    pub fn new(default: &'static str, weight: u16) -> Self {
        Self {
            text: default.to_string(),
            weight,
            font_index: VariableOrNot::default(),
            #[cfg(feature = "ios_integration")]
            font_file: FONT_FILE_BARLOW.to_string(),
            #[cfg(not(feature = "ios_integration"))]
            autocomplete_state: AutocompleteState::new(),
        }
    }

    #[allow(dead_code)]
    pub fn from_default(default: &'static VariableTextSlotDefault) -> Self {
        default.into()
    }

    // /// Update font_file from a full path or just filename (for iOS FFI)
    // /// Extracts filename if full path is provided
    // #[allow(dead_code)] // this is scar of when tried with iOS integration
    // pub fn update_font_path(&mut self, font_path: &str) {
    //     use std::path::Path;
    //     // Extract just the filename from full path, or use as-is if just filename
    //     self.font_file = Path::new(font_path)
    //         .file_name()
    //         .and_then(|s| s.to_str())
    //         .unwrap_or(font_path)
    //         .to_string();
    // }

    pub fn format_custom(&self, exif: &crate::image::exif_impl::SimplifiedExif) -> String {
        exif.format_custom(self.text.clone())
    }

    /// Get font - on iOS loads directly from font_file, on desktop uses font_index
    #[cfg(not(feature = "ios_integration"))]
    pub fn get_font(&self) -> ab_glyph::FontArc {
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

    /// Get font - on iOS loads directly from font_file
    #[cfg(feature = "ios_integration")]
    pub fn get_font(&self) -> ab_glyph::FontArc {
        self.get_font_from_file()
    }

    /// Load font directly from font_file (for iOS)
    /// Uses FONTS_BASE_DIR + font_file to construct full path
    #[cfg(feature = "ios_integration")]
    fn get_font_from_file(&self) -> ab_glyph::FontArc {
        use ab_glyph::FontArc;
        use std::path::PathBuf;

        if self.font_file.is_empty() {
            panic!("Font loading failed on iOS: font_file is empty");
        }

        // Construct full path from base directory + filename
        let base_dir = get_fonts_base_directory();
        let full_path = if base_dir.is_empty() {
            // If no base dir set, try using font_file as-is (might be full path)
            PathBuf::from(&self.font_file)
        } else {
            PathBuf::from(&base_dir).join(&self.font_file)
        };

        log::debug!("Loading font from: {:?}", full_path);

        if let Ok(data) = std::fs::read(&full_path) {
            if let Ok(font) = FontArc::try_from_vec(data) {
                return font;
            }
            log::error!("Failed to parse font file: {:?}", full_path);
        } else {
            log::error!("Failed to read font file: {:?}", full_path);
        }

        panic!(
            "Font loading failed on iOS. font_file='{}', base_dir='{}', full_path='{:?}'",
            self.font_file, base_dir, full_path
        );
    }

    /// return false on bool tuple when it's not variable font
    #[cfg(not(feature = "ios_integration"))]
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

    /// iOS version - weight is ignored, uses font_file directly
    #[cfg(feature = "ios_integration")]
    pub fn get_font_with_new_weight(&self, _weight: u16) -> (ab_glyph::FontArc, bool) {
        // On iOS, fonts are loaded from file, weight is not adjustable at runtime
        (self.get_font_from_file(), false)
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

    #[cfg(not(feature = "ios_integration"))]
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        label: Cow<'static, str>,
        default: &'static VariableTextSlotDefault,
    ) {
        use rust_i18n::t;

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
    #[cfg(not(feature = "ios_integration"))]
    pub fn render_text_edit_with_autocomplete(
        &mut self,
        ui: &mut Ui,
        width: f32,
        id_salt: impl std::hash::Hash,
    ) -> egui::Response {
        render_text_edit_autocomplete_impl(
            &mut self.text,
            &mut self.autocomplete_state,
            ui,
            width,
            id_salt,
        )
    }
}

/// Shared implementation for text edit with autocomplete
#[cfg(not(feature = "ios_integration"))]
fn render_text_edit_autocomplete_impl(
    text: &mut String,
    autocomplete_state: &mut AutocompleteState,
    ui: &mut Ui,
    width: f32,
    id_salt: impl std::hash::Hash,
) -> egui::Response {
    let text_edit_id = ui.id().with(&id_salt).with("autocomplete_text_edit");
    let popup_id = ui.id().with(&id_salt).with("autocomplete_popup");

    // Handle pending insertion from previous frame (e.g., from click or Enter)
    // This must happen BEFORE the TextEdit is rendered so it sees the new text
    if let Some((new_text, new_cursor_pos)) = autocomplete_state.pending_insertion.take() {
        log::debug!(
            "[autocomplete] Applying pending insertion: '{}' cursor={}",
            new_text,
            new_cursor_pos
        );

        // Update the text
        *text = new_text;

        // Clear the TextEdit's internal state so it re-reads from text
        // This is necessary because egui's TextEdit caches text when focused
        ui.ctx()
            .data_mut(|d| d.remove::<egui::text_edit::TextEditState>(text_edit_id));

        // Store fresh state with the new cursor position
        let mut fresh_state = egui::text_edit::TextEditState::default();
        fresh_state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(new_cursor_pos),
            )));
        fresh_state.store(ui.ctx(), text_edit_id);

        // Request focus back to text edit
        ui.ctx().memory_mut(|mem| mem.request_focus(text_edit_id));

        // Request repaint to ensure the TextEdit picks up the changes
        ui.ctx().request_repaint();
    }

    // IMPORTANT: Consume arrow keys BEFORE rendering TextEdit (like egui_autocomplete)
    // This prevents TextEdit from moving cursor to start/end of line
    let (up_pressed, down_pressed) = if autocomplete_state.show_popup {
        let up = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
        let down = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
        (up, down)
    } else {
        (false, false)
    };

    let response = ui.add_sized(
        [width, 23.0],
        TextEdit::singleline(text)
            .vertical_align(Align::Center)
            .id(text_edit_id),
    );

    let has_focus = response.has_focus();

    // Get cursor position from the text edit state
    if let Some(text_edit_state) = egui::TextEdit::load_state(ui.ctx(), text_edit_id)
        && let Some(cursor_range) = text_edit_state.cursor.char_range()
    {
        let cursor_pos = cursor_range.primary.index;
        autocomplete_state.update_from_text(text, cursor_pos);
    }

    // Update selection index based on arrow keys
    if autocomplete_state.show_popup {
        let filtered_len = autocomplete_state.get_filtered_fields().len();
        if filtered_len > 0 {
            if down_pressed {
                autocomplete_state.selected_index =
                    (autocomplete_state.selected_index + 1).min(filtered_len - 1);
            }
            if up_pressed {
                autocomplete_state.selected_index =
                    autocomplete_state.selected_index.saturating_sub(1);
            }
        }
    }

    // Check for keyboard acceptance (Enter/Tab)
    let accepted_by_keyboard = has_focus
        && autocomplete_state.show_popup
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Tab));

    // Check for Escape to close
    let escape_pressed = has_focus
        && autocomplete_state.show_popup
        && ui.input(|i| i.key_pressed(egui::Key::Escape));

    if escape_pressed {
        log::debug!("[autocomplete] Escape pressed, closing popup");
        autocomplete_state.reset();
    }

    // Show autocomplete popup using egui's Popup API (like egui_autocomplete)
    let filtered = autocomplete_state.get_filtered_fields();
    let popup_should_open = autocomplete_state.show_popup && has_focus && !filtered.is_empty();

    let popup = egui::Popup::from_response(&response)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
        .open(popup_should_open);

    let popup_is_open = popup.is_open();

    popup.show(|ui| {
        ui.set_min_width(300.0);
        ui.set_max_height(200.0);

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for (idx, field) in filtered.iter().enumerate() {
                    let is_selected = idx == autocomplete_state.selected_index;

                    // Use toggle_value like egui_autocomplete - hover sets selection
                    let mut selected = is_selected;
                    let item_response = ui.toggle_value(
                        &mut selected,
                        egui::RichText::new(format!("{:<15} {}", field.name, field.description))
                            .monospace(),
                    );

                    // Auto-scroll to keep selected item visible when navigating with keyboard
                    if is_selected && (up_pressed || down_pressed) {
                        item_response.scroll_to_me(Some(egui::Align::Center));
                    }

                    // Update selected index based on hover (like egui_autocomplete)
                    if item_response.hovered() {
                        autocomplete_state.selected_index = idx;
                        log::trace!("[autocomplete] Hover on item {}: {}", idx, field.name);
                    }

                    // Show example on hover
                    item_response.on_hover_ui(|ui| {
                        ui.label(egui::RichText::new(format!("Example: {}", field.example)).weak());
                    });
                }
            });
    });

    log::trace!(
        "[autocomplete] has_focus={}, show_popup={}, popup_is_open={}, selected_idx={}",
        has_focus,
        autocomplete_state.show_popup,
        popup_is_open,
        autocomplete_state.selected_index
    );

    // Apply selection when:
    // 1. Keyboard acceptance (Enter/Tab), OR
    // 2. Popup was shown but is now closed (user clicked outside or lost focus)
    let should_apply = accepted_by_keyboard
        || (autocomplete_state.show_popup
            && !popup_is_open
            && autocomplete_state.cursor_pos.is_some());

    if should_apply {
        log::debug!(
            "[autocomplete] Applying selection: keyboard={}, popup_closed={}",
            accepted_by_keyboard,
            !popup_is_open
        );

        if autocomplete_state.schedule_insertion(text) {
            log::debug!("[autocomplete] Insertion scheduled successfully");
        } else {
            log::debug!("[autocomplete] Insertion scheduling failed");
        }
    }

    // Reset if lost focus and popup closed
    if !has_focus && !popup_is_open {
        autocomplete_state.reset();
    }

    response
}

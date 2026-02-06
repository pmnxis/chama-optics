/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Theme parameter schema system for automatic UI generation and FFI interop
//!
//! This module provides a way to define theme parameter metadata that can be:
//! - Used to generate egui UI controls automatically
//! - Serialized to JSON for Swift/FFI consumption
//! - Applied to update theme values from external sources (Swift UI)
//!
//! The schema system uses custom derive macros to extract parameter information
//! from theme struct field attributes.

use serde::{Deserialize, Serialize};

/// Type of parameter control to display in UI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    /// Numeric slider with min/max range
    Slider,
    /// Boolean toggle/checkbox
    Boolean,
    /// RGBA color picker
    Color,
    /// Text input field
    Text,
    /// Font selection with weight
    Font,
}

/// Metadata for a single theme parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMeta {
    /// Field name in the theme struct
    pub name: String,
    /// Human-readable label (can be i18n key)
    pub label: String,
    /// Optional help text/tooltip
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Type of UI control to show
    #[serde(rename = "type")]
    pub param_type: ParameterType,
    /// Minimum value (for numeric types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum value (for numeric types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Default value (serialized as JSON)
    pub default: serde_json::Value,
    /// Current value (serialized as JSON)
    pub current: serde_json::Value,
    /// Associated EXIF fields (for variable text slots)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exif_fields: Option<Vec<String>>,
}

/// Complete schema for a theme, including all parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSchema {
    /// Unique theme identifier
    pub theme_name: String,
    /// Human-readable theme label
    pub theme_label: String,
    /// List of all parameters
    pub parameters: Vec<ParameterMeta>,
}

/// Trait for types that can provide parameter schema information
///
/// This should be automatically implemented via derive macro:
/// ```ignore
/// #[derive(ThemeParameters, Serialize, Deserialize)]
/// struct MyTheme {
///     #[param(slider, min = 0, max = 100, label = "Border Size")]
///     border_size: u32,
///
///     #[param(color, label = "Font Color")]
///     font_color: egui::Color32,
/// }
/// ```
pub trait ThemeParameters {
    /// Get the schema for this theme
    fn schema(&self) -> ThemeSchema;

    /// Update theme parameters from JSON value map
    fn update_from_json(
        &mut self,
        updates: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), String>;

    /// Render UI controls for all non-border parameters
    ///
    /// This provides a default implementation that automatically generates UI based on the schema.
    /// Themes can override this for custom UI behavior.
    fn render_parameters_ui(&mut self, ui: &mut egui::Ui) {
        let schema = self.schema();

        ui.vertical(|ui| {
            ui.add_space(4.0);

            egui::Grid::new(format!("{}_params_grid", schema.theme_name))
                .num_columns(2)
                .spacing([4.0, 3.0])
                .show(ui, |ui| {
                    for param in &schema.parameters {
                        // Skip border parameters as they're handled separately
                        if param.name.starts_with("border.") {
                            continue;
                        }

                        // Render parameter UI based on type
                        match param.param_type {
                            ParameterType::Slider => {
                                ui.label(&param.label)
                                    .on_hover_text(param.hint.as_deref().unwrap_or(""));

                                // We can't actually modify the value here without reflection
                                // This is a limitation - themes will need custom impl for now
                                ui.label("[Auto UI - needs custom impl]");
                                ui.end_row();
                            }
                            ParameterType::Color => {
                                ui.label(&param.label);
                                ui.label("[Color picker - needs custom impl]");
                                ui.end_row();
                            }
                            ParameterType::Text => {
                                // Text parameters are handled by VariableTextSlot::ui()
                                // Skip them in auto-generated UI
                            }
                            _ => {}
                        }
                    }
                });
        });
    }
}

// Helper functions for converting between Rust types and JSON values

/// Convert egui::Color32 to JSON array [r, g, b, a]
pub fn color32_to_json(color: egui::Color32) -> serde_json::Value {
    serde_json::json!([color.r(), color.g(), color.b(), color.a()])
}

/// Convert JSON array [r, g, b, a] to egui::Color32
pub fn json_to_color32(value: &serde_json::Value) -> Result<egui::Color32, String> {
    match value.as_array() {
        Some(arr) if arr.len() == 4 => {
            let r = arr[0].as_u64().ok_or("Invalid red value")? as u8;
            let g = arr[1].as_u64().ok_or("Invalid green value")? as u8;
            let b = arr[2].as_u64().ok_or("Invalid blue value")? as u8;
            let a = arr[3].as_u64().ok_or("Invalid alpha value")? as u8;
            Ok(egui::Color32::from_rgba_unmultiplied(r, g, b, a))
        }
        _ => Err("Expected [r, g, b, a] array for color".to_string()),
    }
}

/// Convert Border struct to nested JSON object
pub fn border_to_json(border: &crate::effect::border::Border) -> serde_json::Value {
    serde_json::json!({
        "left": border.left,
        "right": border.right,
        "top": border.top,
        "bottom": border.bottom,
        "color": color32_to_json(border.color),
        "is_relative": border.is_relative,
    })
}

/// Convert JSON object to Border struct
pub fn json_to_border(value: &serde_json::Value) -> Result<crate::effect::border::Border, String> {
    let obj = value.as_object().ok_or("Expected object for border")?;

    Ok(crate::effect::border::Border {
        left: obj
            .get("left")
            .and_then(|v| v.as_u64())
            .ok_or("Missing or invalid 'left' field")? as u32,
        right: obj
            .get("right")
            .and_then(|v| v.as_u64())
            .ok_or("Missing or invalid 'right' field")? as u32,
        top: obj
            .get("top")
            .and_then(|v| v.as_u64())
            .ok_or("Missing or invalid 'top' field")? as u32,
        bottom: obj
            .get("bottom")
            .and_then(|v| v.as_u64())
            .ok_or("Missing or invalid 'bottom' field")? as u32,
        color: json_to_color32(obj.get("color").ok_or("Missing 'color' field")?)?,
        is_relative: obj
            .get("is_relative")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    })
}

/// Helper to parse numeric value from JSON (accepts both f64 and u64)
pub fn json_to_u32(value: &serde_json::Value) -> Result<u32, String> {
    let num = value
        .as_f64()
        .or_else(|| value.as_u64().map(|v| v as f64))
        .ok_or("Invalid numeric value")?;
    Ok(num.round() as u32)
}

/// Macro to reduce boilerplate in schema() implementation
///
/// Usage:
/// ```ignore
/// parameters: param_vec![
///     slider!(self, "border.left", "theme.border.left",
///             0.0, 500.0, DEFAULT_BORDER.left, self.border.left),
///     color!(self, "font_color", "Font color",
///            egui::Color32::BLACK, self.font_color),
/// ],
/// ```
#[macro_export]
macro_rules! param_slider {
    ($name:expr, $label:expr, $min:expr, $max:expr, $default:expr, $current:expr) => {
        $crate::theme::parameter_schema::ParameterMeta {
            name: $name.to_string(),
            label: $label.to_string(),
            hint: None,
            param_type: $crate::theme::parameter_schema::ParameterType::Slider,
            min: Some($min),
            max: Some($max),
            default: serde_json::json!($default),
            current: serde_json::json!($current),
            exif_fields: None,
        }
    };
    ($name:expr, $label:expr, $hint:expr, $min:expr, $max:expr, $default:expr, $current:expr) => {
        $crate::theme::parameter_schema::ParameterMeta {
            name: $name.to_string(),
            label: $label.to_string(),
            hint: Some($hint.to_string()),
            param_type: $crate::theme::parameter_schema::ParameterType::Slider,
            min: Some($min),
            max: Some($max),
            default: serde_json::json!($default),
            current: serde_json::json!($current),
            exif_fields: None,
        }
    };
}

#[macro_export]
macro_rules! param_color {
    ($name:expr, $label:expr, $default:expr, $current:expr) => {
        $crate::theme::parameter_schema::ParameterMeta {
            name: $name.to_string(),
            label: $label.to_string(),
            hint: None,
            param_type: $crate::theme::parameter_schema::ParameterType::Color,
            min: None,
            max: None,
            default: $crate::theme::parameter_schema::color32_to_json($default),
            current: $crate::theme::parameter_schema::color32_to_json($current),
            exif_fields: None,
        }
    };
}

#[macro_export]
macro_rules! param_text {
    ($name:expr, $label:expr, $hint:expr, $default:expr, $current:expr) => {
        $crate::theme::parameter_schema::ParameterMeta {
            name: $name.to_string(),
            label: $label.to_string(),
            hint: Some($hint.to_string()),
            param_type: $crate::theme::parameter_schema::ParameterType::Text,
            min: None,
            max: None,
            default: serde_json::json!($default),
            current: serde_json::json!($current),
            exif_fields: None,
        }
    };
}

#[macro_export]
macro_rules! param_font {
    ($name:expr, $label:expr, $hint:expr, $default:expr, $current:expr) => {
        $crate::theme::parameter_schema::ParameterMeta {
            name: $name.to_string(),
            label: $label.to_string(),
            hint: Some($hint.to_string()),
            param_type: $crate::theme::parameter_schema::ParameterType::Font,
            min: None,
            max: None,
            default: serde_json::json!($default),
            current: serde_json::json!($current),
            exif_fields: None,
        }
    };
}

/// Macro to implement ThemeParameters trait for a struct
///
/// This provides a derive-like experience without needing a procedural macro.
///
/// Usage:
/// ```ignore
/// impl_theme_parameters! {
///     ShotOnOneLine {
///         border.left: slider(0.0, 500.0, "theme.border.left", self.border.left, DEFAULT_BORDER.left),
///         border.right: slider(0.0, 500.0, "theme.border.right", self.border.right, DEFAULT_BORDER.right),
///         border_color: color("Border Color", self.border.color, DEFAULT_BORDER.color),
///         font_color: color("Font Color", self.font_color, egui::Color32::BLACK),
///         left_text: text("Left Text", "hint", self.left.text, DEFAULT_LEFT.text, ["camera", "lens"]),
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_theme_parameters {
    (
        $struct_name:ident {
            $( $key:literal : $typ:ident ( $($args:tt)* ) ),* $(,)?
        }
    ) => {
        impl $crate::theme::parameter_schema::ThemeParameters for $struct_name {
            fn schema(&self) -> $crate::theme::parameter_schema::ThemeSchema {
                $crate::theme::parameter_schema::ThemeSchema {
                    theme_name: self.unique_name().to_string(),
                    theme_label: self.label().to_string(),
                    parameters: vec![
                        $(
                            impl_theme_parameters!(@schema $typ, $key, $($args)*)
                        ),*
                    ],
                }
            }

            fn update_from_json(
                &mut self,
                updates: &serde_json::Map<String, serde_json::Value>
            ) -> Result<(), String> {
                $crate::update_param!(updates, {
                    $(
                        impl_theme_parameters!(@update $typ, $key, $($args)*)
                    ),*
                })
            }
        }
    };

    // Schema generators
    (@schema slider, $key:literal, $min:expr, $max:expr, $label:expr, $current:expr, $default:expr) => {
        $crate::param_slider!($key, $label, $min, $max, $default, $current)
    };

    (@schema slider, $key:literal, $min:expr, $max:expr, $label:expr, $hint:expr, $current:expr, $default:expr) => {
        $crate::param_slider!($key, $label, $hint, $min, $max, $default, $current)
    };

    (@schema color, $key:literal, $label:expr, $current:expr, $default:expr) => {
        $crate::param_color!($key, $label, $default, $current)
    };

    (@schema text, $key:literal, $label:expr, $hint:expr, $current:expr, $default:expr, [$($exif:expr),*]) => {
        $crate::param_text!($key, $label, $hint, $default, $current, vec![$($exif),*])
    };

    // Update generators
    (@update slider, $key:literal, $min:expr, $max:expr, $label:expr, $current:expr, $default:expr) => {
        $key => ($current, u32)
    };

    (@update slider, $key:literal, $min:expr, $max:expr, $label:expr, $hint:expr, $current:expr, $default:expr) => {
        $key => ($current, u32)
    };

    (@update color, $key:literal, $label:expr, $current:expr, $default:expr) => {
        $key => ($current, color)
    };

    (@update text, $key:literal, $label:expr, $hint:expr, $current:expr, $default:expr, [$($exif:expr),*]) => {
        $key => ($current, string)
    };
}

/// Macro to generate UI for a slider parameter with reset button
///
/// Usage:
/// ```ignore
/// ui_slider!(ui, self.font_height, DEFAULT_FONT_HEIGHT, 5..=80, t!("theme.font_height"), t!("hint"));
/// ```
#[macro_export]
macro_rules! ui_slider {
    // With hint
    ($ui:expr, $field:expr, $default:expr, $range:expr, $label:expr, $hint:expr) => {
        $ui.label($label).on_hover_text($hint);
        $ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut $field, $range).step_by(1.0));
            ui.label("% ");
            if ui.button("↺").clicked() {
                $field = $default;
            }
        });
        $ui.end_row();
    };
    // Without hint
    ($ui:expr, $field:expr, $default:expr, $range:expr, $label:expr) => {
        $ui.label($label);
        $ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut $field, $range).step_by(1.0));
            ui.label("% ");
            if ui.button("↺").clicked() {
                $field = $default;
            }
        });
        $ui.end_row();
    };
}

/// Macro to generate UI for a color parameter
///
/// Usage:
/// ```ignore
/// ui_color!(ui, self.font_color, t!("theme.font_color"));
/// ```
#[macro_export]
macro_rules! ui_color {
    ($ui:expr, $field:expr, $label:expr) => {
        $ui.label($label);
        egui::widgets::color_picker::color_edit_button_srgba(
            $ui,
            &mut $field,
            egui::color_picker::Alpha::Opaque,
        );
        $ui.end_row();
    };
}

/// Macro for update_from_json to reduce boilerplate
///
/// Usage:
/// ```ignore
/// update_param! {
///     updates,
///     "border.left" => (self.border.left, u32),
///     "font_color" => (self.font_color, color),
///     "left.text" => (self.left.text, string),
/// }
/// ```
#[macro_export]
macro_rules! update_param {
    ($updates:expr, { $( $key:expr => ($field:expr, $typ:ident) ),* $(,)? }) => {{
        for (key, value) in $updates {
            match key.as_str() {
                $(
                    $key => {
                        update_param!(@assign $field, value, $typ, $key);
                    }
                )*
                _ => {
                    // On iOS, skip warning for .font keys as they're handled separately
                    #[cfg(any(feature = "ios_integration", feature = "android_integration"))]
                    if key.ends_with(".font") {
                        continue;
                    }
                    log::warn!("Unknown parameter: {}", key);
                }
            }
        }
        Ok(())
    }};

    // Internal: type-specific assignment
    (@assign $field:expr, $value:expr, u32, $key:expr) => {
        let val = $value.as_f64()
            .or_else(|| $value.as_u64().map(|v| v as f64))
            .ok_or(concat!("Invalid ", $key, " value"))?;
        $field = val as u32;
    };

    (@assign $field:expr, $value:expr, color, $key:expr) => {
        $field = $crate::theme::parameter_schema::json_to_color32($value)?;
    };

    (@assign $field:expr, $value:expr, string, $key:expr) => {
        $field = $value.as_str()
            .ok_or(concat!("Invalid ", $key, " value"))?
            .to_string();
    };
}

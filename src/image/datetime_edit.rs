/*
 * SPDX-FileCopyrightText: © 2026 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use egui::{Color32, RichText, TextEdit, TextStyle};

/// State for datetime editing UI
#[derive(Clone, PartialEq)]
pub struct DatetimeEditState {
    pub edit_string: String,
}

impl DatetimeEditState {
    pub fn new() -> Self {
        Self {
            edit_string: String::new(),
        }
    }

    pub fn clear(&mut self) {
        self.edit_string.clear();
    }

    /// Initialize edit string from datetime if empty
    pub fn initialize_from_datetime(&mut self, datetime: Option<chrono::NaiveDateTime>) {
        if self.edit_string.is_empty() {
            self.edit_string = datetime
                .map(|dt| {
                    // log::debug!("DEBUG: Initializing datetime_edit from datetime: {:?}", dt);
                    dt.format("%Y.%m.%d  %H:%M:%S").to_string()
                })
                .unwrap_or_else(|| {
                    // log::debug!("DEBUG: datetime is None, using empty string");
                    String::new()
                });
        }
    }
}

impl Default for DatetimeEditState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render datetime editor with validation and hinting
pub fn render_datetime_editor(
    state: &mut DatetimeEditState,
    datetime: &mut Option<chrono::NaiveDateTime>,
    editable: bool,
    ui: &mut egui::Ui,
) {
    let small_text = |text: &str| RichText::new(text).text_style(TextStyle::Small);

    ui.label(small_text("DateTime"));

    if editable {
        // Initialize if empty
        state.initialize_from_datetime(*datetime);

        // Remove all separators and extract digits for parsing
        let cleaned: String = state
            .edit_string
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();

        // Build display string with partial hinting
        // let display_str = build_display_string(&cleaned);

        // Show both editable field and formatted display with color validation
        ui.horizontal(|ui| {
            let response = ui.add(
                TextEdit::singleline(&mut state.edit_string)
                    .font(TextStyle::Small)
                    .desired_width(100.0)
                    .hint_text("YYYY.MM.DD"),
            );

            // Show formatted hint with color validation
            render_validated_display(ui, &cleaned, small_text);

            if response.changed() {
                // Recalculate cleaned digits after change
                let cleaned: String = state
                    .edit_string
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                // Only update if we have exactly 14 digits (YYYYMMDDhhmmss)
                if cleaned.len() == 14 {
                    // log::debug!("DEBUG: Attempting to parse 14 digits");
                    match chrono::NaiveDateTime::parse_from_str(&cleaned, "%Y%m%d%H%M%S") {
                        Ok(dt) => {
                            // log::debug!("DEBUG: Successfully parsed datetime: {:?}", dt);
                            *datetime = Some(dt);
                        }
                        Err(e) => {
                            log::warn!("DEBUG: Failed to parse datetime: {}", e);
                        }
                    }
                } else if cleaned.is_empty() {
                    // User cleared the field
                    *datetime = None;
                }
            }
        });
    } else {
        // Clear edit string when exiting edit mode
        if !state.edit_string.is_empty() {
            state.clear();
        }

        // Display format: YYYY.MM.DD  hh:mm:ss
        let datetime_str = datetime
            .map(|dt| dt.format("%Y.%m.%d  %H:%M:%S").to_string())
            .unwrap_or_default();
        ui.label(small_text(&datetime_str));
    }

    ui.end_row();
}

/// Build display string with partial hinting
#[allow(dead_code)]
fn build_display_string(cleaned: &str) -> String {
    if cleaned.is_empty() {
        return "YYYY.MM.DD  hh:mm:ss".to_string();
    }

    if cleaned.len() >= 14 {
        // Full datetime, format it properly
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(cleaned, "%Y%m%d%H%M%S") {
            return dt.format("%Y.%m.%d  %H:%M:%S").to_string();
        } else {
            return cleaned.to_string();
        }
    }

    // Partial input, build with placeholders
    let mut result = String::new();
    let chars: Vec<char> = cleaned.chars().collect();

    // YYYY
    if chars.len() >= 4 {
        result.push_str(&cleaned[0..4]);
    } else {
        result.push_str(cleaned);
        result.push_str(&"YYYY"[cleaned.len()..]);
    }
    result.push('.');

    // MM
    if chars.len() >= 6 {
        result.push_str(&cleaned[4..6]);
    } else if chars.len() >= 5 {
        result.push_str(&cleaned[4..]);
        result.push('M');
    } else {
        result.push_str("MM");
    }
    result.push('.');

    // DD
    if chars.len() >= 8 {
        result.push_str(&cleaned[6..8]);
    } else if chars.len() >= 7 {
        result.push_str(&cleaned[6..]);
        result.push('D');
    } else {
        result.push_str("DD");
    }

    result.push_str("  ");

    // hh
    if chars.len() >= 10 {
        result.push_str(&cleaned[8..10]);
    } else if chars.len() >= 9 {
        result.push_str(&cleaned[8..]);
        result.push('h');
    } else {
        result.push_str("hh");
    }
    result.push(':');

    // mm
    if chars.len() >= 12 {
        result.push_str(&cleaned[10..12]);
    } else if chars.len() >= 11 {
        result.push_str(&cleaned[10..]);
        result.push('m');
    } else {
        result.push_str("mm");
    }
    result.push(':');

    // ss
    if chars.len() >= 14 {
        result.push_str(&cleaned[12..14]);
    } else if chars.len() >= 13 {
        result.push_str(&cleaned[12..]);
        result.push('s');
    } else {
        result.push_str("ss");
    }

    result
}

/// Render validated display with color validation
fn render_validated_display<F>(ui: &mut egui::Ui, cleaned: &str, small_text: F)
where
    F: Fn(&str) -> RichText,
{
    let chars: Vec<char> = cleaned.chars().collect();
    let n = chars.len();

    // Helper function to get two-digit text with proper padding
    let get_two_digit_owned = |start: usize, len: usize| -> String {
        if n >= start + len {
            cleaned[start..(start + len)].to_owned()
        } else if n > start {
            // Single digit, pad with leading zero if we have enough total digits
            if n >= 12 {
                // We have substantial input (12+ digits), pad it
                format!("0{}", &cleaned[start..(start + 1)])
            } else {
                cleaned[start..(start + 1)].to_owned()
            }
        } else if len == 2 {
            "MM".to_owned()
        } else if start >= 8 {
            "ss".to_owned()
        } else if start == 6 {
            "hh".to_owned()
        } else {
            "DD".to_owned()
        }
    };

    // YYYY - always valid (4 digits)
    let year_color = Color32::WHITE;
    let year_text = if n >= 4 {
        cleaned[0..4].to_string()
    } else if !cleaned.is_empty() {
        cleaned.to_string()
    } else {
        "YYYY".to_owned()
    };
    ui.label(small_text(&year_text).color(year_color));
    ui.label(small_text(".").color(Color32::GRAY));

    // MM - validate 01-12
    let month_valid = if n >= 6 {
        let month: u32 = cleaned[4..6].parse().unwrap_or(0);
        (1..=12).contains(&month)
    } else if n == 5 {
        // Single digit month, can't validate yet
        true
    } else {
        true // Not yet entered
    };
    let month_color = if month_valid {
        Color32::WHITE
    } else {
        Color32::RED
    };
    let month_text = get_two_digit_owned(4, 2);
    ui.label(small_text(&month_text).color(month_color));
    ui.label(small_text(".").color(Color32::GRAY));

    // DD - validate 01-31
    let day_valid = if n >= 8 {
        let day: u32 = cleaned[6..8].parse().unwrap_or(0);
        (1..=31).contains(&day)
    } else if n == 7 {
        // Single digit day, can't validate yet
        true
    } else {
        true // Not yet entered
    };
    let day_color = if day_valid {
        Color32::WHITE
    } else {
        Color32::RED
    };
    let day_text = get_two_digit_owned(6, 2);
    ui.label(small_text(&day_text).color(day_color));

    ui.label(small_text("  ").color(Color32::GRAY));

    // hh - validate 00-23
    let hour_valid = if n >= 10 {
        let hour: u32 = cleaned[8..10].parse().unwrap_or(0);
        (0..=23).contains(&hour)
    } else if n == 9 {
        // Single digit hour, can't validate yet
        true
    } else {
        true // Not yet entered
    };
    let hour_color = if hour_valid {
        Color32::WHITE
    } else {
        Color32::RED
    };
    let hour_text = get_two_digit_owned(8, 2);
    ui.label(small_text(&hour_text).color(hour_color));
    ui.label(small_text(":").color(Color32::GRAY));

    // mm - validate 00-59
    let minute_valid = if n >= 12 {
        let minute: u32 = cleaned[10..12].parse().unwrap_or(0);
        (0..=59).contains(&minute)
    } else if n == 11 {
        // Single digit minute, can't validate yet
        true
    } else {
        true // Not yet entered
    };
    let minute_color = if minute_valid {
        Color32::WHITE
    } else {
        Color32::RED
    };
    let minute_text = get_two_digit_owned(10, 2);
    ui.label(small_text(&minute_text).color(minute_color));
    ui.label(small_text(":").color(Color32::GRAY));

    // ss - validate 00-59
    let second_valid = if n >= 14 {
        let second: u32 = cleaned[12..14].parse().unwrap_or(0);
        (0..=59).contains(&second)
    } else if n == 13 {
        // Single digit second, can't validate yet
        true
    } else {
        true // Not yet entered
    };
    let second_color = if second_valid {
        Color32::WHITE
    } else {
        Color32::RED
    };
    let second_text = get_two_digit_owned(12, 2);
    ui.label(small_text(&second_text).color(second_color));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_display_string_empty() {
        let result = build_display_string("");
        assert_eq!(result, "YYYY.MM.DD  hh:mm:ss");
    }

    #[test]
    fn test_build_display_string_partial_year() {
        let result = build_display_string("202");
        assert_eq!(result, "202Y.MM.DD  hh:mm:ss");
    }

    #[test]
    fn test_build_display_string_partial_month() {
        let result = build_display_string("20251");
        assert_eq!(result, "2025.1M.DD  hh:mm:ss");
    }

    #[test]
    fn test_build_display_string_complete() {
        let result = build_display_string("20251130214115");
        assert_eq!(result, "2025.11.30  21:41:15");
    }

    #[test]
    fn test_build_display_string_partial_time() {
        let result = build_display_string("2025113021");
        assert_eq!(result, "2025.11.30  21:mm:ss");
    }
}

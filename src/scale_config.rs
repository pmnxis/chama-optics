/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy)]
pub enum ScaleConfig {
    /// No scale
    None,

    /// Resize with width with aspect ratio
    MaxWidth(u32),

    /// Resize with height with aspect ratio
    MaxHeight(u32),

    /// Resize with dividing width and height
    Divide(u32),

    /// Choose nearest width by considering common divisor
    NearCommonDivisorConsiderWidth(u32),

    /// Choose nearest height by considering common divisor
    NearCommonDivisorConsiderHeight(u32),
}

impl std::default::Default for ScaleConfig {
    fn default() -> Self {
        Self::None
    }
}

pub const SCALE_NEAR_COMMON_4K: ScaleConfig = ScaleConfig::NearCommonDivisorConsiderWidth(4072);

impl ScaleConfig {
    fn __apply(&self, width: u32, height: u32) -> (u32, u32) {
        match *self {
            ScaleConfig::None => (width, height),

            ScaleConfig::MaxWidth(target_w) => {
                if width == 0 || height == 0 || width < target_w {
                    return (width, height);
                }

                let ratio = target_w as f64 / width as f64;
                let new_h = (height as f64 * ratio).round() as u32;
                (target_w, new_h)
            }

            ScaleConfig::MaxHeight(target_h) => {
                if width == 0 || height == 0 || height < target_h {
                    return (width, height);
                }
                let ratio = target_h as f64 / height as f64;
                let new_w = (width as f64 * ratio).round() as u32;
                (new_w, target_h)
            }

            ScaleConfig::Divide(div) => {
                if div == 0 {
                    return (width, height);
                }
                (width / div, height / div)
            }

            ScaleConfig::NearCommonDivisorConsiderWidth(target_w) => {
                let gcd = gcd::euclid_nonzero_u32(
                    std::num::NonZero::new(width).unwrap(),
                    std::num::NonZero::new(height).unwrap(),
                );
                let w_unit = width / gcd;
                let h_unit = height / gcd;

                let k = (target_w as f64 / w_unit as f64).round() as u32;
                (w_unit * k, h_unit * k)
            }

            ScaleConfig::NearCommonDivisorConsiderHeight(target_h) => {
                let gcd = gcd::euclid_nonzero_u32(
                    std::num::NonZero::new(width).unwrap(),
                    std::num::NonZero::new(height).unwrap(),
                );
                let w_unit = width / gcd;
                let h_unit = height / gcd;

                let k = (target_h as f64 / h_unit as f64).round() as u32;
                (w_unit * k, h_unit * k)
            }
        }
    }

    pub fn apply(&self, width: u32, height: u32, is_vert_rot: bool) -> (u32, u32) {
        if !is_vert_rot {
            self.__apply(width, height)
        } else {
            match *self {
                ScaleConfig::MaxWidth(x) => ScaleConfig::MaxHeight(x),
                ScaleConfig::MaxHeight(x) => ScaleConfig::MaxWidth(x),
                ScaleConfig::NearCommonDivisorConsiderWidth(x) => {
                    ScaleConfig::NearCommonDivisorConsiderHeight(x)
                }
                ScaleConfig::NearCommonDivisorConsiderHeight(x) => {
                    ScaleConfig::NearCommonDivisorConsiderWidth(x)
                }
                others => others,
            }
            .__apply(width, height)
        }
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        let mut selected = match self {
            ScaleConfig::None => "None",
            ScaleConfig::MaxWidth(_) => "Max Width",
            ScaleConfig::MaxHeight(_) => "Max Height",
            ScaleConfig::Divide(_) => "Divide",
            ScaleConfig::NearCommonDivisorConsiderWidth(_) => "Near Common Divisor (Width)",
            ScaleConfig::NearCommonDivisorConsiderHeight(_) => "Near Common Divisor (Height)",
        }
        .to_string();

        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                ui.label("Scale Mode:");
                egui::ComboBox::from_id_salt("scale_mode")
                    .selected_text(&selected)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut selected, "None".to_string(), "None");
                        ui.selectable_value(&mut selected, "Max Width".to_string(), "Max Width");
                        ui.selectable_value(&mut selected, "Max Height".to_string(), "Max Height");
                        ui.selectable_value(&mut selected, "Divide".to_string(), "Divide");
                        ui.selectable_value(
                            &mut selected,
                            "Near Common Divisor (Width)".to_string(),
                            "Near Common Divisor (Width)",
                        );
                        ui.selectable_value(
                            &mut selected,
                            "Near Common Divisor (Height)".to_string(),
                            "Near Common Divisor (Height)",
                        );
                    });
            });

            // temp
            let mut value: u32 = match self {
                ScaleConfig::None => 0,
                ScaleConfig::MaxWidth(v)
                | ScaleConfig::MaxHeight(v)
                | ScaleConfig::Divide(v)
                | ScaleConfig::NearCommonDivisorConsiderWidth(v)
                | ScaleConfig::NearCommonDivisorConsiderHeight(v) => *v,
            };

            // Update by selected interface
            match selected.as_str() {
                "None" => *self = ScaleConfig::None,

                "Max Width" => {
                    ui.horizontal(|ui| {
                        ui.label("Max width:");
                        ui.add(egui::DragValue::new(&mut value).range(1..=20000));
                    });
                    *self = ScaleConfig::MaxWidth(value);
                }

                "Max Height" => {
                    ui.horizontal(|ui| {
                        ui.label("Max height:");
                        ui.add(egui::DragValue::new(&mut value).range(1..=20000));
                    });
                    *self = ScaleConfig::MaxHeight(value);
                }

                "Divide" => {
                    ui.horizontal(|ui| {
                        ui.label("Divide factor:");
                        ui.add(egui::DragValue::new(&mut value).range(1..=1000));
                    });
                    *self = ScaleConfig::Divide(value);
                }

                "Near Common Divisor (Width)" => {
                    ui.horizontal(|ui| {
                        ui.label("Target width:");
                        ui.add(egui::DragValue::new(&mut value).range(1..=20000));
                    });
                    *self = ScaleConfig::NearCommonDivisorConsiderWidth(value);
                }

                "Near Common Divisor (Height)" => {
                    ui.horizontal(|ui| {
                        ui.label("Target height:");
                        ui.add(egui::DragValue::new(&mut value).range(1..=20000));
                    });
                    *self = ScaleConfig::NearCommonDivisorConsiderHeight(value);
                }

                _ => {}
            };
        });
    }
}

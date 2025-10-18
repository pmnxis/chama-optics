/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{EnumIter, IntoEnumIterator};
use strum_macros::{EnumString, IntoStaticStr};

#[rustfmt::skip]
#[derive(
    EnumString, IntoStaticStr, EnumIter, Clone, Copy,
    Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord,
)]
#[strum(serialize_all = "lowercase")]
pub enum Language {
    En,
    Ko,
}

impl core::default::Default for Language {
    fn default() -> Self {
        Self::En
    }
}

impl Language {
    // get system locale but if there's nothing return default
    pub fn get_system() -> Self {
        let sys_loc = sys_locale::get_locale().unwrap_or_else(|| String::from("en-US"));

        if let Some(code) = sys_loc.split(['-', '_']).next() {
            Self::from_str(code).unwrap_or(Self::default())
        } else {
            Self::default()
        }
    }

    pub fn into_str(&self) -> &'static str {
        Into::<&'static str>::into(self)
    }

    pub fn update_i18n(&self) {
        rust_i18n::set_locale(self.into_str());
    }

    pub fn update_menu_ui(&mut self, ui: &mut egui::Ui) {
        ui.menu_button(t!("language.label"), |ui| {
            for lang in Language::iter() {
                if ui
                    .button(t!(format!("language.{}", lang.into_str())))
                    .clicked()
                {
                    *self = lang;
                    self.update_i18n();
                }
            }
        });
    }
}

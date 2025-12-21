/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use ab_glyph::FontArc;
use eframe::egui;
use font_kit::{handle::Handle, source::SystemSource};
use std::sync::{Arc, RwLock};
use std::thread;

use crate::fonts::FONTS_UNIFY;
use rust_i18n::t;

#[rustfmt::skip]
#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum FontSort {
    #[default]
    Builtin,
    System,
}

#[rustfmt::skip]
#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, strum::Display, Debug)]
pub enum FontError {
    Unknown,
    InvalidIndex(usize),
    FailedToGet(String),
    FailedToLoad(String),
    NonSelected,
    FileNotFound(String),
    FilePathNotFound,
    NonRecoverSelectedFont,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Default, Clone, Debug)]
pub struct FontIndex {
    pub sort: FontSort,
    pub index: usize,
    pub path: Option<std::path::PathBuf>,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Default, Clone, Debug)]
pub struct FontSelection {
    pub name: String,
    pub select: FontIndex,
    pub default: BuiltinFontIndex,
}

#[derive(Clone)]
pub struct SystemFont {
    pub name: String,
    pub(crate) handle: Handle,
}

fn __get_path_from_handle(handle: &Handle) -> Option<std::path::PathBuf> {
    if let Handle::Path {
        path,
        font_index: _,
    } = handle
    {
        Some(path.clone())
    } else {
        None
    }
}

// read font directly from path
fn __read_font_direct<P: AsRef<std::path::Path>>(path: P) -> Result<FontArc, FontError> {
    let data = std::fs::read(&path).map_err(|e| {
        log::error!("Cannot find font file. : {e}");
        FontError::FileNotFound(path.as_ref().to_string_lossy().to_string())
    })?;

    FontArc::try_from_vec(data)
        .map_err(|_| FontError::FailedToLoad(path.as_ref().to_string_lossy().to_string()))
}

fn __get_fontarc_from_handle(handle: &Handle, hint_when_err: &str) -> Result<FontArc, FontError> {
    match handle {
        Handle::Memory { bytes, .. } => FontArc::try_from_vec(bytes.to_vec())
            .map_err(|_| FontError::FailedToGet(hint_when_err.to_owned())),
        Handle::Path { path, .. } => __read_font_direct(path),
    }
}

impl SystemFont {
    fn get_path(&self) -> Option<std::path::PathBuf> {
        __get_path_from_handle(&self.handle)
    }
}

#[allow(dead_code)]
pub enum FontSearchResult {
    Found { found: FontIndex, font: FontArc },
    FoundButIndexReloaded { found: FontIndex, font: FontArc },
    FoundSysNonLoaded { font: FontArc },
}

#[rustfmt::skip]
#[repr(usize)]
#[derive(serde::Deserialize, serde::Serialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFontIndex {
    #[default]
    D2Coding,
    NtSansMed,
    Digital7,
    Digital7Italic,
}

pub struct FontsUnify {
    pub(self) builtin_fonts: [&'static crate::fonts::BuiltInFonts; 4],
    pub(self) system_fonts: Arc<RwLock<Vec<SystemFont>>>,
}

impl FontsUnify {
    pub fn new() -> Self {
        let system_fonts = Arc::new(RwLock::new(Vec::new()));
        let thread_ref = system_fonts.clone();

        thread::spawn(move || {
            let before_start = std::time::Instant::now();
            let mut ret = Vec::new();
            let sys_src = SystemSource::new();

            if let Ok(result) = sys_src.all_fonts() {
                for handle in result {
                    if let Ok(font) = handle.load() {
                        ret.push(SystemFont {
                            name: font.family_name(),
                            handle: handle.clone(),
                        });
                    }
                }
            }

            *thread_ref.write().unwrap() = ret;

            log::info!(
                "{} msecs passed for getting OS fonts",
                (std::time::Instant::now() - before_start).as_millis()
            );
        });

        Self {
            builtin_fonts: [
                &crate::fonts::FONT_D2CODING,
                &crate::fonts::FONT_SHSANS,
                &crate::fonts::FONT_DIGITAL_7,
                &crate::fonts::FONT_DIGITAL_7_ITALIC,
            ],
            system_fonts,
        }
    }

    pub fn builtin_select(&'static self, index: BuiltinFontIndex) -> FontSelection {
        FontSelection {
            name: self.builtin_fonts[index as usize].name.to_owned(),
            select: FontIndex {
                sort: FontSort::Builtin,
                index: index as usize,
                path: None,
            },
            default: index,
        }
    }

    // pub fn get(&'static self, font_index: &FontIndex) -> Result<FontArc, FontError> {
    //     match font_index.sort {
    //         FontSort::Builtin => {
    //             let font = self
    //                 .builtin_fonts
    //                 .get(font_index.index)
    //                 .ok_or_else(|| FontError::InvalidIndex(font_index.index))?;
    //             FontArc::try_from_slice(font.data)
    //                 .map_err(|_| FontError::FailedToGet(font.name.to_string()))
    //         }

    //         FontSort::System => {
    //             let sys_lock = self.system_fonts.read().unwrap();
    //             let sf = sys_lock
    //                 .get(font_index.index)
    //                 .ok_or_else(|| FontError::InvalidIndex(font_index.index))?
    //                 .clone(); // copy small handle struct
    //             drop(sys_lock); // ✅ release read lock early

    //             __get_fontarc_from_handle(&sf.handle, &sf.name)
    //         } // Not use FontSort::None anymore
    //           // TBD
    //           // FontSort::None => Err(FontError::NonSelected),
    //     }
    // }

    pub fn deep_get(&'static self, select: &FontSelection) -> Result<FontSearchResult, FontError> {
        fn get_with_matching_name<'a>(
            read: &'a std::sync::RwLockReadGuard<'_, Vec<SystemFont>>,
            select: &FontSelection,
        ) -> Result<&'a SystemFont, Option<(FontIndex, &'a SystemFont)>> {
            if let Some(sf) = read.get(select.select.index)
                && sf.name != select.name
            {
                return Ok(sf);
            }

            if let Some((index, item)) = read
                .iter()
                .enumerate()
                .find(|(_, item)| item.name == select.name)
            {
                Err(Some((
                    FontIndex {
                        sort: FontSort::System,
                        index,
                        path: item.get_path(),
                    },
                    item,
                )))
            } else {
                Err(None)
            }
        }

        let name = &select.name;
        if let Some(index) = self.builtin_fonts.iter().position(|item| item.name == name) {
            return Ok(FontSearchResult::Found {
                found: FontIndex {
                    sort: FontSort::Builtin,
                    index,
                    path: None,
                },
                font: FontArc::try_from_slice(self.builtin_fonts[index].data)
                    .map_err(|_| FontError::FailedToGet(name.clone()))?,
            });
        }

        if let Ok(sys_fonts) = self.system_fonts.read() {
            // When system still couldn't get list
            if sys_fonts.is_empty() {
                if let Some(path) = &select.select.path {
                    // todo - read by Handle, and compare family name
                    Ok(FontSearchResult::FoundSysNonLoaded {
                        font: __read_font_direct(path)?,
                    })
                } else {
                    Err(FontError::FilePathNotFound)
                }
            } else {
                // Access to list
                match get_with_matching_name(&sys_fonts, select) {
                    Ok(font) => Ok(FontSearchResult::Found {
                        found: select.select.clone(),
                        font: __get_fontarc_from_handle(&font.handle, name)?,
                    }),
                    // alt access when index was changed
                    Err(Some((new_idx, new_sys_font))) => {
                        Ok(FontSearchResult::FoundButIndexReloaded {
                            found: new_idx,
                            font: __get_fontarc_from_handle(&new_sys_font.handle, name)?,
                        })
                    }
                    Err(None) => Err(FontError::NonRecoverSelectedFont),
                }
            }
        } else {
            // When failed to access Rwlock
            // do samthing when sys_fonts.len() == 0
            if let Some(path) = &select.select.path {
                // todo - read by Handle, and compare family name
                Ok(FontSearchResult::FoundSysNonLoaded {
                    font: __read_font_direct(path)?,
                })
            } else {
                Err(FontError::FilePathNotFound)
            }
        }
    }

    #[allow(dead_code)]
    pub fn search_mut(&'static self, select: &mut FontSelection) -> Result<FontArc, FontError> {
        match self.deep_get(select) {
            Ok(FontSearchResult::Found { found: _, font }) => Ok(font),
            Ok(FontSearchResult::FoundButIndexReloaded { found, font }) => {
                // replace here
                *select = FontSelection {
                    name: select.name.to_owned(),
                    select: found,
                    default: select.default,
                };
                Ok(font)
            }
            Ok(FontSearchResult::FoundSysNonLoaded { font }) => Ok(font),
            Err(e) => Err(e),
        }
    }

    pub fn search(&'static self, select: &FontSelection) -> Result<FontArc, FontError> {
        match self.deep_get(select) {
            Ok(FontSearchResult::Found { found: _, font }) => Ok(font),
            Ok(FontSearchResult::FoundButIndexReloaded { found, font }) => {
                log::warn!("Font index was changed {found:?}");

                Ok(font)
            }
            Ok(FontSearchResult::FoundSysNonLoaded { font }) => Ok(font),
            Err(e) => Err(e),
        }
    }

    #[allow(dead_code)]
    pub fn get_by_select(&'static self, select: &FontSelection) -> Result<FontArc, FontError> {
        let prev_idx = select.select.index;
        match select.select.sort {
            FontSort::Builtin => {
                let font = self
                    .builtin_fonts
                    .get(prev_idx)
                    .ok_or(FontError::InvalidIndex(prev_idx))?;
                FontArc::try_from_slice(font.data)
                    .map_err(|_| FontError::FailedToGet(font.name.to_string()))
            }

            FontSort::System => {
                let sys_lock = self.system_fonts.read().unwrap();
                let sf = sys_lock
                    .get(prev_idx)
                    .ok_or(FontError::InvalidIndex(prev_idx))?
                    .clone(); // copy small handle struct
                drop(sys_lock); // ✅ release read lock early

                __get_fontarc_from_handle(&sf.handle, &sf.name)
            } // Not use FontSort::None anymore
              // TBD
        }
    }
}

impl From<FontError> for image::ImageError {
    fn from(value: FontError) -> Self {
        match value {
            FontError::FileNotFound(path) => image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Font not found: {path}"),
            )),
            FontError::FailedToLoad(path) => image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid font format: {path}"),
            )),
            FontError::FailedToGet(path) => image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to get : {path}"),
            )),
            others => image::ImageError::IoError(std::io::Error::other(format!("{others}"))),
        }
    }
}

impl FontSelection {
    /// use `crate::FONTS_UNIFY.builtin_select(` instead
    #[deprecated]
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            select: FontIndex::default(),
            name: String::new(),
            default: BuiltinFontIndex::default(),
        }
    }

    pub fn update_ui<S: Into<egui::WidgetText>>(&mut self, ui: &mut egui::Ui, label: S) {
        let current_font_label = match self.select.sort {
            FontSort::Builtin => FONTS_UNIFY
                .builtin_fonts
                .get(self.select.index)
                .map(|f| {
                    if self.default as usize == self.select.index {
                        format!("{} [{}]", f.name, t!("fonts_selector.default"))
                    } else {
                        f.name.to_string()
                    }
                })
                .unwrap_or_else(|| "<Invalid builtin font>".to_string()),

            FontSort::System => {
                let sys_lock = FONTS_UNIFY.system_fonts.read().unwrap();
                sys_lock
                    .get(self.select.index)
                    .map(|sf| sf.name.clone())
                    .unwrap_or_else(|| t!("fonts_selector.system_fonts_loading").to_string())
            } // Not use FontSort::None anymore
              // TBD
              // FontSort::None => "Select a font".to_string(),
        };

        egui::ComboBox::from_id_salt(label.into().text())
            .selected_text(current_font_label)
            .show_ui(ui, |ui| {
                ui.label(t!("fonts_selector.builtin_fonts"));
                for (i, font) in FONTS_UNIFY.builtin_fonts.iter().enumerate() {
                    let selected = self.select.sort == FontSort::Builtin && self.select.index == i;

                    let is_clicked = if self.default as usize == i {
                        ui.selectable_label(
                            selected,
                            format!("{} [{}]", font.name, t!("fonts_selector.default")),
                        )
                    } else {
                        ui.selectable_label(selected, font.name)
                    }
                    .clicked();

                    if is_clicked {
                        self.select = FontIndex {
                            sort: FontSort::Builtin,
                            index: i,
                            path: None,
                        };
                        self.name = font.name.to_string();
                    }
                }

                ui.separator();
                ui.label(t!("fonts_selector.system_fonts"));

                let sys_lock = FONTS_UNIFY.system_fonts.read().unwrap();
                if sys_lock.is_empty() {
                    ui.label(t!("fonts_selector.system_fonts_loading"));
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(200));
                } else {
                    for (i, sf) in sys_lock.iter().enumerate() {
                        let selected =
                            self.select.sort == FontSort::System && self.select.index == i;
                        if ui.selectable_label(selected, &sf.name).clicked() {
                            self.select = FontIndex {
                                sort: FontSort::System,
                                index: i,
                                path: sf.get_path(),
                            };
                            self.name = sf.name.clone();
                        }
                    }
                }
            });
    }

    pub fn update_ui_with_label<S: Into<egui::WidgetText> + Clone>(
        &mut self,
        ui: &mut egui::Ui,
        label: S,
    ) {
        ui.horizontal(|ui| {
            ui.label(label.clone());
            self.update_ui(ui, label);
        });
    }

    pub fn update_ui_with_default_label(&mut self, ui: &mut egui::Ui) {
        let label = t!("fonts_selector.select_a_font");
        ui.horizontal(|ui| {
            ui.label(label.clone());
            self.update_ui(ui, label);
        });
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

rust_i18n::i18n!("locales");

mod app;
pub mod export_config;
pub(crate) mod fonts;
// pub(crate) use fonts::builtin_fonts;
pub(crate) mod import_config;
pub(crate) use export_config::scale_config;
pub(crate) mod langs;
pub(crate) use art::ART_UNIFY;
pub(crate) use fonts::FONTS_UNIFY;
pub(crate) use fonts::font_unify::{BuiltinFontIndex, FontSelection};
pub(crate) use image::{exif_impl, packed_image};

pub mod art;
pub(crate) mod effect;
pub(crate) mod image;
pub mod test_helper;
mod util;

pub mod theme;
// pub(crate) mod preview;

pub use app::ChamaOptics;

#[macro_use]
pub mod dump;

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

// #![warn(clippy::all, rust_2018_idioms)]
rust_i18n::i18n!("locales");

mod app;
pub(crate) mod export_config;
pub(crate) mod fonts;
// pub(crate) use fonts::builtin_fonts;
pub(crate) mod import_config;
pub(crate) use export_config::scale_config;
pub(crate) mod langs;
pub(crate) use image::{exif_impl, packed_image};
pub(crate) mod image;

pub mod theme;
// pub(crate) mod preview;

pub use app::ChamaOptics;

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

// #![warn(clippy::all, rust_2018_idioms)]
rust_i18n::i18n!("locales");

mod app;
pub mod builtin_fonts;
pub(crate) mod exif_impl;
pub(crate) mod export_config;
mod fonts;
pub(crate) mod heic;
pub(crate) mod import_config;
pub(crate) mod langs;
pub(crate) mod output_format;
pub(crate) mod packed_image;
pub(crate) mod scale_config;
pub mod theme;
// pub(crate) mod preview;

pub use app::ChamaOptics;

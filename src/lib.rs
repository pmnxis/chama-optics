/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

// #![warn(clippy::all, rust_2018_idioms)]

mod app;
pub(crate) mod exif_impl;
mod fonts;
pub(crate) mod heic;
pub(crate) mod orientation;
pub(crate) mod packed_image;
pub(crate) mod theme;
// pub(crate) mod preview;

pub use app::ChamaOptics;

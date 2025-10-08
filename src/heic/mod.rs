/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod native;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::load_heif;

#[cfg(target_arch = "wasm32")]
pub(crate) use web::*;

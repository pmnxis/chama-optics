/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

pub mod types;

#[cfg(has_logo_asset_path)]
include!(env!("LOGO_ASSET_PATH"));
#[cfg(not(has_logo_asset_path))]
include!("../../assets/auto_generated/logo_assets.rs");

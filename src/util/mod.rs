/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#[cfg(all(feature = "desktop", not(feature = "ios_integration")))]
pub(crate) mod check_update;

#[cfg(feature = "rfd")]
pub(crate) mod async_file_dialog;

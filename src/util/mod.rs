/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */
#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) mod check_update;

#[cfg(any(feature = "desktop", feature = "web"))]
pub(crate) mod web_download;

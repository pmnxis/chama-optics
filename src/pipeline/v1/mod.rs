/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Pipeline V1 — first versioned export pipeline
//!
//! Queue-based stage execution with Decoration (Theme/Cheki) always last.
//! All types are Serialize/Deserialize for JSON FFI, CLI, and preset support.

mod bridge;
mod config;
mod context;
mod execute;
mod preset;
mod preview;
mod stages;
mod validation;

pub use bridge::*;
pub use config::*;
pub use context::*;
pub use execute::*;
pub use preset::*;
pub use preview::*;
pub use stages::*;
pub use validation::*;

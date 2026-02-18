/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Export Pipeline module
//!
//! Platform-agnostic, cfg-free image processing pipeline.
//! Current version is re-exported at the top level.
//!
//! ```
//! use crate::pipeline::PipelineConfig;          // always latest
//! use crate::pipeline::v1::PipelineConfig;      // explicit v1
//! ```

pub mod v1;
pub use v1::*;

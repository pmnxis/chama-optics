/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Pipeline configuration validation.
//!
//! Validates stage ordering and configuration constraints before execution.

use std::fmt;

use super::stages::PipelineStage;

/// Pipeline validation error.
#[derive(Debug)]
pub enum PipelineError {
    /// CropRotate must be the first stage if present.
    CropRotateNotFirst { found_at: usize },
    /// Stage-specific error during execution.
    StageError(String),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineError::CropRotateNotFirst { found_at } => {
                write!(
                    f,
                    "CropRotate must be the first stage, but found at index {}",
                    found_at
                )
            }
            PipelineError::StageError(msg) => write!(f, "Stage error: {}", msg),
        }
    }
}

impl std::error::Error for PipelineError {}

impl super::config::PipelineConfig {
    /// Validate the pipeline configuration before execution.
    ///
    /// Rules:
    /// - CropRotate must be the first enabled stage if present.
    pub fn validate(&self) -> Result<(), PipelineError> {
        // Check: CropRotate must be first if present
        let mut found_non_crop = false;
        for (i, entry) in self.stages.iter().enumerate() {
            if !entry.enabled {
                continue;
            }
            if matches!(&entry.stage, PipelineStage::CropRotate(_)) {
                if found_non_crop {
                    return Err(PipelineError::CropRotateNotFirst { found_at: i });
                }
            } else {
                found_non_crop = true;
            }
        }

        Ok(())
    }
}

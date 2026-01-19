/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! UI state management layer - progress tracking, timing, etc.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// UI-specific state for progress tracking and timing
#[derive(Default)]
pub struct UiState {
    pub save_progress: ProgressState,
    pub load_progress: ProgressState,
    pub update_checker: crate::util::check_update::CheckRelease,
}

/// Progress tracking for async operations
pub struct ProgressState {
    current: Arc<AtomicUsize>,
    total: usize,
    started_at: Option<Instant>,
    completed_at: Option<Instant>,
}

impl ProgressState {
    pub fn new() -> Self {
        Self {
            current: Arc::new(AtomicUsize::new(0)),
            total: 0,
            started_at: None,
            completed_at: None,
        }
    }

    /// Start tracking progress for a new operation
    pub fn start(&mut self, total: usize) {
        self.total = total;
        self.current.store(0, Ordering::Relaxed);
        self.started_at = Some(Instant::now());
        self.completed_at = None;
    }

    /// Get a cloneable handle to the progress counter for background threads
    pub fn counter(&self) -> Arc<AtomicUsize> {
        self.current.clone()
    }

    /// Get current progress value
    pub fn current(&self) -> usize {
        self.current.load(Ordering::Relaxed)
    }

    /// Get total items
    pub fn total(&self) -> usize {
        self.total
    }

    /// Get progress as a float (0.0 to 1.0)
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.current() as f32 / self.total.max(1) as f32
        }
    }

    /// Check is this started
    pub fn is_started(&self) -> bool {
        self.started_at.is_some()
    }

    /// Check if operation is in progress
    pub fn is_active(&self) -> bool {
        self.total > 0
    }

    /// Check if operation is complete
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.current() >= self.total
    }

    /// Mark operation as complete
    pub fn mark_complete(&mut self) {
        if self.is_complete() && self.completed_at.is_none() {
            self.started_at = None;
            self.completed_at = Some(Instant::now());
        }
    }

    /// Reset progress (called after completion message timeout)
    pub fn reset(&mut self) {
        self.total = 0;
        self.current.store(0, Ordering::Relaxed);
        self.started_at = None;
        self.completed_at = None;
    }

    /// Check if enough time has elapsed since completion to hide the progress bar
    pub fn should_hide(&self, duration: std::time::Duration) -> bool {
        if self.started_at.is_none() {
            true
        } else if let Some(completed_at) = self.completed_at {
            completed_at.elapsed() > duration
        } else {
            false
        }
    }
}

impl Default for ProgressState {
    fn default() -> Self {
        Self::new()
    }
}

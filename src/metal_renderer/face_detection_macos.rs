/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Face detection integration for macOS
//!
//! This module provides face detection functionality using to new detector architecture
//! which supports VisionKit, MediaPipe, and YOLO engines.

#[allow(unused_imports)]
use crate::effect::face_detection::FaceDetectionEngine;
use std::path::Path;

/// Face detector adapter for macOS
/// This creates an appropriate detector based on selected engine
pub struct MacFaceDetector {
    engine: FaceDetectionEngine,
}

impl MacFaceDetector {
    /// Create a new face detector with specified engine
    pub fn new(engine: FaceDetectionEngine) -> Self {
        Self { engine }
    }

    /// Detect faces using selected engine
    pub fn detect_faces(&self, _image_path: &Path) -> Vec<(i32, i32, u32, u32)> {
        #[cfg(feature = "face_detection_visionkit")]
        use crate::effect::face_detectors::FaceDetector;

        match self.engine {
            #[cfg(feature = "face_detection_visionkit")]
            FaceDetectionEngine::VisionKit => {
                let detector = crate::effect::face_detectors::VisionKitDetector::new();
                detector.detect_faces(_image_path)
            }
            _ => {
                log::warn!("Face detection engine {:?} not available", self.engine);
                vec![]
            }
        }
    }

}

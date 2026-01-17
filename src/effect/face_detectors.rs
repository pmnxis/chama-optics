/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Face detection implementations
//! All detectors are stateless to work correctly on multiple calls

use std::path::Path;

/// Face detector trait
pub trait FaceDetector: Send + Sync {
    /// Detect faces in an image
    /// Returns a list of (x, y, width, height) rectangles
    fn detect_faces(&self, image_path: &Path) -> Vec<(i32, i32, u32, u32)>;

    /// Get engine name
    fn engine_name(&self) -> &'static str;
}

/// VisionKit detector (Swift bridge on macOS/iOS)
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub struct VisionKitDetector;

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl Default for VisionKitDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl VisionKitDetector {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl FaceDetector for VisionKitDetector {
    fn detect_faces(&self, image_path: &Path) -> Vec<(i32, i32, u32, u32)> {
        // This calls Swift bridge - Swift code handles autoreleasepool
        // The Rust FFI function is stateless
        #[cfg(target_os = "ios")]
        {
            // iOS: Call Swift bridge via FFI
            // The Swift side handles Vision framework with proper memory management
            log::info!("VisionKit detector called on iOS for: {:?}", image_path);
            // TODO: Implement FFI call to Swift bridge
            vec![]
        }

        #[cfg(target_os = "macos")]
        {
            // macOS: Call swift script via process
            log::info!("VisionKit detector called on macOS for: {:?}", image_path);

            // Call swift script for actual Vision framework detection
            use std::process::Command;

            let script_path = Path::new("macos/face_detector.swift");
            if !script_path.exists() {
                log::error!("VisionKit script not found at: {:?}", script_path);
                return vec![];
            }

            // Create JSON input
            let input_json = serde_json::json!({
                "image_path": image_path.to_str()
            });

            let output = Command::new("swift")
                .arg(script_path)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    use std::io::Write;
                    let stdin = child.stdin.as_mut().expect("stdin");
                    stdin.write_all(input_json.to_string().as_bytes())?;
                    child.wait_with_output()
                });

            match output {
                Ok(result) => {
                    if !result.status.success() {
                        log::error!(
                            "VisionKit script failed: {}",
                            String::from_utf8_lossy(&result.stderr)
                        );
                        return vec![];
                    }

                    // Parse JSON output
                    let output_str = String::from_utf8_lossy(&result.stdout);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output_str)
                        && let Some(faces) = json["faces"].as_array()
                    {
                        return faces
                            .iter()
                            .filter_map(|face| {
                                let x = face["x"].as_i64().map(|v| v as i32);
                                let y = face["y"].as_i64().map(|v| v as i32);
                                let width = face["width"].as_u64().map(|v| v as u32);
                                let height = face["height"].as_u64().map(|v| v as u32);

                                match (x, y, width, height) {
                                    (Some(x), Some(y), Some(width), Some(height)) => {
                                        Some((x, y, width, height))
                                    }
                                    _ => None,
                                }
                            })
                            .collect();
                    }
                }
                Err(e) => {
                    log::error!("Failed to run VisionKit script: {}", e);
                }
            }

            vec![]
        }
    }

    fn engine_name(&self) -> &'static str {
        "VisionKit"
    }
}

/// No-op detector for platforms without any detection engine
#[cfg(not(any(
    target_os = "ios",
    target_os = "macos",
    feature = "face_detection_insightface"
)))]
pub struct NoOpDetector;

#[cfg(not(any(
    target_os = "ios",
    target_os = "macos",
    feature = "face_detection_insightface"
)))]
impl NoOpDetector {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(any(
    target_os = "ios",
    target_os = "macos",
    feature = "face_detection_insightface"
)))]
impl Default for NoOpDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(any(
    target_os = "ios",
    target_os = "macos",
    feature = "face_detection_insightface"
)))]
impl FaceDetector for NoOpDetector {
    fn detect_faces(&self, _image_path: &Path) -> Vec<(i32, i32, u32, u32)> {
        log::warn!("No face detector available on this platform");
        vec![]
    }

    fn engine_name(&self) -> &'static str {
        "NoOp"
    }
}

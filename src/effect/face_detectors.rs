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

// ── VisionKit FFI (macOS: linked Swift static library) ──

#[cfg(all(target_os = "macos", feature = "face_detection_visionkit"))]
#[repr(C)]
struct CFaceRectResult {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[cfg(all(target_os = "macos", feature = "face_detection_visionkit"))]
unsafe extern "C" {
    fn visionkit_detect_faces(
        image_path: *const std::ffi::c_char,
        speed_mode: i32,
        out_count: *mut i32,
    ) -> *mut std::ffi::c_void;

    fn visionkit_free_faces(ptr: *mut std::ffi::c_void, count: i32);
}

/// VisionKit detector (Swift FFI on macOS, Swift app bridge on iOS)
#[cfg(all(
    any(target_os = "ios", target_os = "macos"),
    feature = "face_detection_visionkit"
))]
pub struct VisionKitDetector {
    speed_mode: i32,
}

#[cfg(all(
    any(target_os = "ios", target_os = "macos"),
    feature = "face_detection_visionkit"
))]
impl Default for VisionKitDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(
    any(target_os = "ios", target_os = "macos"),
    feature = "face_detection_visionkit"
))]
impl VisionKitDetector {
    pub fn new() -> Self {
        Self { speed_mode: 0 } // Fastest by default
    }

    pub fn with_speed_mode(speed_mode: i32) -> Self {
        Self { speed_mode }
    }
}

#[cfg(all(
    any(target_os = "ios", target_os = "macos"),
    feature = "face_detection_visionkit"
))]
impl FaceDetector for VisionKitDetector {
    fn detect_faces(&self, image_path: &Path) -> Vec<(i32, i32, u32, u32)> {
        #[cfg(target_os = "ios")]
        {
            log::info!("VisionKit detector called on iOS for: {:?}", image_path);
            // iOS: Face detection handled by Swift app (FaceDetectionBridge.swift)
            vec![]
        }

        #[cfg(target_os = "macos")]
        {
            use std::ffi::CString;

            let path_str = match image_path.to_str() {
                Some(s) => s,
                None => {
                    log::error!("VisionKit: invalid image path (non-UTF8)");
                    return vec![];
                }
            };

            let c_path = match CString::new(path_str) {
                Ok(c) => c,
                Err(_) => {
                    log::error!("VisionKit: image path contains null byte");
                    return vec![];
                }
            };

            let mut count: i32 = 0;
            let raw_ptr =
                unsafe { visionkit_detect_faces(c_path.as_ptr(), self.speed_mode, &mut count) };

            if raw_ptr.is_null() || count <= 0 {
                log::info!("VisionKit detected 0 face(s) in {}", path_str);
                return vec![];
            }

            let ptr = raw_ptr as *const CFaceRectResult;
            let faces: Vec<(i32, i32, u32, u32)> = (0..count as usize)
                .map(|i| unsafe {
                    let r = &*ptr.add(i);
                    (r.x, r.y, r.width, r.height)
                })
                .collect();

            unsafe { visionkit_free_faces(raw_ptr, count) };

            log::info!("VisionKit detected {} face(s) in {}", faces.len(), path_str);
            faces
        }
    }

    fn engine_name(&self) -> &'static str {
        "VisionKit"
    }
}

/// No-op detector for platforms without any detection engine
#[cfg(not(any(
    feature = "face_detection_visionkit",
    feature = "face_detection_insightface"
)))]
pub struct NoOpDetector;

#[cfg(not(any(
    feature = "face_detection_visionkit",
    feature = "face_detection_insightface"
)))]
impl NoOpDetector {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(any(
    feature = "face_detection_visionkit",
    feature = "face_detection_insightface"
)))]
impl Default for NoOpDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(any(
    feature = "face_detection_visionkit",
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

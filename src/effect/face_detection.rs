/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

use rust_i18n::t;
use std::path::Path;
use strum::Display;

/// Detection speed modes — controls sliding window strategy for large images.
/// Shared by InsightFace (ort) and Candle face detectors.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg(any(
    feature = "face_detection_insightface",
    feature = "face_detection_candle"
))]
pub enum SpeedMode {
    /// Fastest: No sliding window (whole image resized to 640×640), ~0.5s avg
    Fastest,
    /// Fast: min(w,h) sliding windows only, ~0.6s avg
    Fast,
    /// Normal: 1 depth level from m_max window, ~7s avg
    Normal,
    /// Slow: 2 depth levels from m_max down, ~13s avg
    Slow,
    /// Slowest: 3 depth levels from m_max down, ~28s avg.
    /// For professional ILC cameras (Panasonic/Sony/Canon/Sigma/Fuji/Hasselblad/Nikon/Leica)
    /// the depth extends to m_max+1 levels, reaching down to 640 px.
    Slowest,
}

#[cfg(any(
    feature = "face_detection_insightface",
    feature = "face_detection_candle"
))]
impl SpeedMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SpeedMode::Fastest => "fastest",
            SpeedMode::Fast => "fast",
            SpeedMode::Normal => "normal",
            SpeedMode::Slow => "slow",
            SpeedMode::Slowest => "slowest",
        }
    }

    pub fn max_depth(&self) -> u32 {
        match self {
            SpeedMode::Fastest => 0,
            SpeedMode::Fast => 1,
            SpeedMode::Normal => 1,
            SpeedMode::Slow => 2,
            SpeedMode::Slowest => 3,
        }
    }
}

/// Returns `true` if the EXIF Make string belongs to a professional ILC camera brand.
///
/// When `Slowest` mode is active for an ILC camera, the sliding-window pyramid is
/// extended by one extra level (m_max + 1), reaching down to the 640 px base window.
pub fn is_ilc_camera_make(make: &str) -> bool {
    let m = make.to_lowercase();
    m.contains("panasonic")
        || m.contains("sony")
        || m.contains("canon")
        || m.contains("sigma")
        || m.contains("fuji")
        || m.contains("hasselblad")
        || m.contains("nikon")
        || m.contains("leica")
}

/// Face effect mode - what effect to apply to detected faces
/// This enum matches the iOS FaceEffectType for consistency
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize, Default)]
pub enum FaceEffectMode {
    /// No effect applied to faces
    #[default]
    None,
    /// Mosaic/pixelate effect
    Mosaic,
    /// Stroke/border around face
    Stroke,
    /// Combined mosaic inside + stroke border
    MosaicStroke,
    /// Sticker overlay (handled separately)
    Sticker,
}

impl FaceEffectMode {
    /// Get display name for the effect mode
    pub fn display_name(&self) -> &'static str {
        match self {
            FaceEffectMode::None => "None",
            FaceEffectMode::Mosaic => "Mosaic",
            FaceEffectMode::Stroke => "Stroke",
            FaceEffectMode::MosaicStroke => "Mosaic+Stroke",
            FaceEffectMode::Sticker => "Sticker",
        }
    }

    /// Get all available modes for UI
    pub fn all_modes() -> &'static [FaceEffectMode] {
        &[
            FaceEffectMode::None,
            FaceEffectMode::Mosaic,
            FaceEffectMode::Stroke,
            FaceEffectMode::MosaicStroke,
            FaceEffectMode::Sticker,
        ]
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize, Display, PartialEq, Eq, Debug)]
pub enum FaceDetectionEngine {
    #[cfg(feature = "face_detection_visionkit")]
    VisionKit,
    #[cfg(feature = "face_detection_insightface")]
    InsightFace,
    #[cfg(feature = "face_detection_candle")]
    Candle,
    #[cfg(not(any(
        feature = "face_detection_visionkit",
        feature = "face_detection_insightface",
        feature = "face_detection_candle"
    )))]
    NoOp,
}

impl FaceDetectionEngine {
    /// Get display name for engine
    pub fn display_name(&self) -> &'static str {
        match self {
            #[cfg(feature = "face_detection_visionkit")]
            Self::VisionKit => "VisionKit",
            #[cfg(feature = "face_detection_insightface")]
            Self::InsightFace => "InsightFace",
            #[cfg(feature = "face_detection_candle")]
            Self::Candle => "Candle",
            #[cfg(not(any(
                feature = "face_detection_visionkit",
                feature = "face_detection_insightface",
                feature = "face_detection_candle"
            )))]
            Self::NoOp => "NoOp",
        }
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct FaceDetection {
    pub engine: FaceDetectionEngine,
    /// Effect mode to apply to detected faces
    #[serde(default)]
    pub effect_mode: FaceEffectMode,
    #[cfg(any(target_os = "ios", target_os = "android"))]
    pub border_thickness: u32, // todo - remove later
    /// Stroke border color
    pub border_color: egui::Color32, // todo - is this really used?
    /// Mosaic block size in pixels
    pub mosaic_block_size: u32,
    /// Legacy: mask faces with blur (deprecated, use effect_mode instead)
    pub mask_faces: bool, // todo - is this really used?
    #[cfg(any(
        feature = "face_detection_insightface",
        feature = "face_detection_candle"
    ))]
    pub speed_mode: SpeedMode,
    #[cfg(feature = "face_detection_insightface")]
    pub provider: crate::effect::insightface_detector::ExecutionProvider,
    pub recursive_detection: bool,
    pub recursive_min_size: u32,
    pub recursive_max_depth: u32,
    pub recursive_overlap: bool,
    pub recursive_overlap_ratio: f32,
}

const DEFAULT_MOSAIC_BLOCK_SIZE: u32 = 100u32;

impl core::default::Default for FaceDetection {
    fn default() -> Self {
        // Default to a bright red color that's visible on most images
        let [r, g, b, a] = [255, 0, 0, 255];

        #[cfg(all(
            not(any(target_os = "ios", target_os = "android")),
            feature = "face_detection_visionkit",
            feature = "face_detection_insightface"
        ))]
        {
            FaceDetection {
                engine: FaceDetectionEngine::VisionKit,
                effect_mode: FaceEffectMode::None,
                border_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
                mosaic_block_size: DEFAULT_MOSAIC_BLOCK_SIZE,
                mask_faces: false,
                speed_mode: SpeedMode::Normal,
                provider:
                    crate::effect::insightface_detector::ExecutionProvider::CPUExecutionProvider,
                recursive_detection: false,
                recursive_min_size: 64,
                recursive_max_depth: 4,
                recursive_overlap: true,
                recursive_overlap_ratio: 0.25,
            }
        }

        #[cfg(all(
            not(any(target_os = "ios", target_os = "android")),
            feature = "face_detection_visionkit",
            not(feature = "face_detection_insightface")
        ))]
        {
            FaceDetection {
                engine: FaceDetectionEngine::VisionKit,
                effect_mode: FaceEffectMode::None,
                border_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
                mosaic_block_size: DEFAULT_MOSAIC_BLOCK_SIZE,
                mask_faces: false,
                recursive_detection: false,
                recursive_min_size: 64,
                recursive_max_depth: 4,
                recursive_overlap: true,
                recursive_overlap_ratio: 0.25,
            }
        }

        #[cfg(all(
            not(any(target_os = "ios", target_os = "android")),
            feature = "face_detection_insightface",
            not(feature = "face_detection_visionkit")
        ))]
        {
            FaceDetection {
                engine: FaceDetectionEngine::InsightFace,
                effect_mode: FaceEffectMode::None,
                border_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
                mosaic_block_size: DEFAULT_MOSAIC_BLOCK_SIZE,
                mask_faces: false,
                speed_mode: SpeedMode::Normal,
                provider: crate::effect::insightface_detector::ExecutionProvider::OnnxAuto,
                recursive_detection: false,
                recursive_min_size: 64,
                recursive_max_depth: 4,
                recursive_overlap: true,
                recursive_overlap_ratio: 0.25,
            }
        }

        #[cfg(all(
            not(any(target_os = "ios", target_os = "android")),
            feature = "face_detection_candle",
            not(feature = "face_detection_visionkit"),
            not(feature = "face_detection_insightface")
        ))]
        {
            FaceDetection {
                engine: FaceDetectionEngine::Candle,
                effect_mode: FaceEffectMode::None,
                border_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
                mosaic_block_size: DEFAULT_MOSAIC_BLOCK_SIZE,
                mask_faces: false,
                speed_mode: SpeedMode::Normal,
                recursive_detection: false,
                recursive_min_size: 64,
                recursive_max_depth: 4,
                recursive_overlap: true,
                recursive_overlap_ratio: 0.25,
            }
        }

        #[cfg(all(
            not(any(
                feature = "face_detection_visionkit",
                feature = "face_detection_insightface",
                feature = "face_detection_candle"
            )),
            not(any(target_os = "ios", target_os = "android"))
        ))]
        {
            FaceDetection {
                engine: FaceDetectionEngine::NoOp,
                effect_mode: FaceEffectMode::None,
                border_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
                mosaic_block_size: DEFAULT_MOSAIC_BLOCK_SIZE,
                mask_faces: false,
                recursive_detection: false,
                recursive_min_size: 64,
                recursive_max_depth: 4,
                recursive_overlap: true,
                recursive_overlap_ratio: 0.25,
            }
        }

        #[cfg(all(target_os = "ios", feature = "face_detection_visionkit"))]
        {
            Self {
                engine: FaceDetectionEngine::VisionKit,
                effect_mode: FaceEffectMode::None,
                border_thickness: 4,
                border_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
                mosaic_block_size: DEFAULT_MOSAIC_BLOCK_SIZE,
                mask_faces: false,
                recursive_detection: false,
                recursive_min_size: 64,
                recursive_max_depth: 4,
                recursive_overlap: true,
                recursive_overlap_ratio: 0.25,
            }
        }

        #[cfg(all(
            any(target_os = "ios", target_os = "android"),
            not(feature = "face_detection_visionkit"),
            not(feature = "face_detection_candle")
        ))]
        {
            Self {
                engine: FaceDetectionEngine::NoOp,
                effect_mode: FaceEffectMode::None,
                border_thickness: 4,
                border_color: egui::Color32::from_rgba_unmultiplied_const(r, g, b, a),
                mosaic_block_size: DEFAULT_MOSAIC_BLOCK_SIZE,
                mask_faces: false,
                recursive_detection: false,
                recursive_min_size: 64,
                recursive_max_depth: 4,
                recursive_overlap: true,
                recursive_overlap_ratio: 0.25,
            }
        }
    }
}

impl FaceDetection {
    /// Draw face detection rectangles on image
    /// This method draws rectangles for detected faces with no fill and a border
    /// Optionally applies blur masking to detected faces
    // todo - need to be remove later
    #[cfg(any(target_os = "ios", target_os = "android"))]
    pub fn apply(
        &self,
        dyn_image: &mut image::DynamicImage,
        face_rectangles: Vec<(i32, i32, u32, u32)>,
    ) -> Result<(), image::ImageError> {
        // Apply blur masking if enabled
        if self.mask_faces {
            for (x, y, width, height) in &face_rectangles {
                self.apply_blur_mask(dyn_image, *x, *y, *width, *height)?;
            }
        }

        // Draw rectangles if not masking (or draw on top of mask)
        let color: image::Rgba<u8> = crate::theme::color32_to_rgba(self.border_color);
        let thickness = self.border_thickness as i32;

        for (x, y, width, height) in face_rectangles {
            let _rect = imageproc::rect::Rect::at(x, y).of_size(width, height);

            // Draw a hollow rectangle (no fill, just border)
            // For thicker borders, we draw multiple rectangles with decreasing size
            for offset in 0..thickness {
                let inner_rect = imageproc::rect::Rect::at(x + offset, y + offset).of_size(
                    width.saturating_sub(2 * offset as u32),
                    height.saturating_sub(2 * offset as u32),
                );

                imageproc::drawing::draw_hollow_rect_mut(dyn_image, inner_rect, color);
            }
        }

        Ok(())
    }

    /// Apply blur masking to a face region
    // todo - need to be remove later
    #[cfg(any(target_os = "ios", target_os = "android"))]
    fn apply_blur_mask(
        &self,
        dyn_image: &mut image::DynamicImage,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<(), image::ImageError> {
        use image::imageops;

        // Get face region
        let face_region = dyn_image.crop_imm(x as u32, y as u32, width, height);

        // Apply Gaussian blur
        let blurred = imageops::blur(&face_region, 20.0); // assume it's 20.0

        // Replace original region with blurred version
        imageops::replace(dyn_image, &blurred, x as i64, y as i64);

        Ok(())
    }

    /// Run recursive face detection on an image region
    /// This method divides image into sub-regions and recursively searches for faces
    /// until all faces are found or minimum size is reached
    pub fn detect_faces_recursive<D: super::face_detectors::FaceDetector>(
        &self,
        detector: &D,
        image_path: &Path,
        img_width: u32,
        img_height: u32,
    ) -> Vec<(i32, i32, u32, u32)> {
        if !self.recursive_detection {
            // If recursive detection is disabled, use standard detection
            return detector.detect_faces(image_path);
        }

        log::info!("Recursive face detection started");
        log::info!("Image dimensions: {}x{}", img_width, img_height);
        log::info!(
            "Min size: {}px, Overlap ratio: {:.0}%",
            self.recursive_min_size,
            self.recursive_overlap_ratio * 100.0
        );

        let mut all_faces = vec![];

        // Step 1: Initial detection on full image
        let initial_faces = detector.detect_faces(image_path);
        log::info!("Initial detection found {} faces", initial_faces.len());

        // Mark covered regions
        let mut covered_regions = initial_faces.clone();

        // Step 2: Recursively search uncovered regions
        self.search_region_recursive(
            detector,
            image_path,
            0,
            0,
            img_width,
            img_height,
            &mut covered_regions,
            &mut all_faces,
            0,
        );

        // Remove duplicates (similar faces that may be detected multiple times)
        all_faces = self.deduplicate_faces(all_faces);

        log::info!(
            "Recursive detection complete: {} unique faces found",
            all_faces.len()
        );
        all_faces
    }

    /// Recursively search a region for faces
    #[allow(clippy::too_many_arguments)]
    fn search_region_recursive<D: super::face_detectors::FaceDetector>(
        &self,
        detector: &D,
        image_path: &Path,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        covered_regions: &mut Vec<(i32, i32, u32, u32)>,
        all_faces: &mut Vec<(i32, i32, u32, u32)>,
        depth: usize,
    ) {
        // Stop if region is too small
        if width < self.recursive_min_size || height < self.recursive_min_size {
            log::debug!(
                "Stopped recursion: region too small ({}x{}) at depth {}",
                width,
                height,
                depth
            );
            return;
        }

        // Stop if max depth is reached
        if depth >= self.recursive_max_depth as usize {
            log::debug!(
                "Stopped recursion: max depth {} reached",
                self.recursive_max_depth
            );
            return;
        }

        // Check if region is already covered by a detected face
        if self.is_region_covered(x, y, width, height, covered_regions) {
            log::debug!("Skipping covered region at depth {}", depth);
            return;
        }

        // Create cropped image for this region
        let img = match image::open(image_path) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image: {}", e);
                return;
            }
        };

        let cropped = img.crop_imm(
            x.max(0) as u32,
            y.max(0) as u32,
            width.min(img.width() - x.max(0) as u32),
            height.min(img.height() - y.max(0) as u32),
        );

        // Save to temp file and run detection
        let temp_path =
            std::path::PathBuf::from(format!("/tmp/chama_optics_temp_region_{}.png", depth));

        if let Err(e) = cropped.save(&temp_path) {
            log::error!("Failed to save temp image: {}", e);
            return;
        }

        // Run detection on this region
        let region_faces = detector.detect_faces(&temp_path);
        log::debug!(
            "Detection at depth {}: {} faces found in region {}x{} at ({}, {})",
            depth,
            region_faces.len(),
            width,
            height,
            x,
            y
        );

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        if region_faces.is_empty() {
            // No faces found, divide region and recurse
            self.divide_and_recurse(
                detector,
                image_path,
                x,
                y,
                width,
                height,
                covered_regions,
                all_faces,
                depth + 1,
            );
        } else {
            // Faces found, add to results
            for (face_x, face_y, face_w, face_h) in region_faces {
                // Adjust coordinates to full image
                let global_x = x + face_x;
                let global_y = y + face_y;

                // Check for overlap with existing faces
                if !self.is_face_covered(global_x, global_y, face_w, face_h, covered_regions) {
                    all_faces.push((global_x, global_y, face_w, face_h));
                    covered_regions.push((global_x, global_y, face_w, face_h));
                    log::info!(
                        "Found face at ({}, {}) size {}x{} at depth {}",
                        global_x,
                        global_y,
                        face_w,
                        face_h,
                        depth
                    );
                }
            }

            // If overlap is enabled, search remaining uncovered parts
            if self.recursive_overlap {
                self.search_uncovered_subregions(
                    detector,
                    image_path,
                    x,
                    y,
                    width,
                    height,
                    covered_regions,
                    all_faces,
                    depth + 1,
                );
            }
        }
    }

    /// Divide region into subregions and recurse
    #[allow(clippy::too_many_arguments)]
    fn divide_and_recurse<D: super::face_detectors::FaceDetector>(
        &self,
        detector: &D,
        image_path: &Path,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        covered_regions: &mut Vec<(i32, i32, u32, u32)>,
        all_faces: &mut Vec<(i32, i32, u32, u32)>,
        depth: usize,
    ) {
        log::debug!(
            "Dividing region {}x{} at ({}, {}) into 4 subregions",
            width,
            height,
            x,
            y
        );

        let half_w = width / 2;
        let half_h = height / 2;

        // Top-left
        self.search_region_recursive(
            detector,
            image_path,
            x,
            y,
            half_w,
            half_h,
            covered_regions,
            all_faces,
            depth,
        );

        // Top-right
        self.search_region_recursive(
            detector,
            image_path,
            x + half_w as i32,
            y,
            width - half_w,
            half_h,
            covered_regions,
            all_faces,
            depth,
        );

        // Bottom-left
        self.search_region_recursive(
            detector,
            image_path,
            x,
            y + half_h as i32,
            half_w,
            height - half_h,
            covered_regions,
            all_faces,
            depth,
        );

        // Bottom-right
        self.search_region_recursive(
            detector,
            image_path,
            x + half_w as i32,
            y + half_h as i32,
            width - half_w,
            height - half_h,
            covered_regions,
            all_faces,
            depth,
        );
    }

    /// Search uncovered subregions within a region
    #[allow(clippy::too_many_arguments)]
    fn search_uncovered_subregions<D: super::face_detectors::FaceDetector>(
        &self,
        detector: &D,
        image_path: &Path,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        covered_regions: &mut Vec<(i32, i32, u32, u32)>,
        all_faces: &mut Vec<(i32, i32, u32, u32)>,
        depth: usize,
    ) {
        let overlap_x = (width as f32 * self.recursive_overlap_ratio) as u32;
        let overlap_y = (height as f32 * self.recursive_overlap_ratio) as u32;

        // Divide region into 3x3 grid with overlap
        let grid_size = 3;
        let step_x = (width - 2 * overlap_x) / (grid_size - 1);
        let step_y = (height - 2 * overlap_y) / (grid_size - 1);

        for i in 0..grid_size {
            for j in 0..grid_size {
                let sub_x = x + (i as i32 * step_x as i32) - overlap_x as i32;
                let sub_y = y + (j as i32 * step_y as i32) - overlap_y as i32;
                let sub_w = step_x + 2 * overlap_x;
                let sub_h = step_y + 2 * overlap_y;

                if !self.is_region_covered(sub_x, sub_y, sub_w, sub_h, covered_regions) {
                    self.search_region_recursive(
                        detector,
                        image_path,
                        sub_x,
                        sub_y,
                        sub_w,
                        sub_h,
                        covered_regions,
                        all_faces,
                        depth,
                    );
                }
            }
        }
    }

    /// Check if a region is covered by any detected face
    fn is_region_covered(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        covered_regions: &[(i32, i32, u32, u32)],
    ) -> bool {
        let coverage_threshold = 0.5; // 50% coverage threshold

        for (face_x, face_y, face_w, face_h) in covered_regions {
            let overlap_area = self
                .calculate_overlap_area(x, y, width, height, *face_x, *face_y, *face_w, *face_h);

            let region_area = width * height;

            if overlap_area as f32 > region_area as f32 * coverage_threshold {
                return true;
            }
        }

        false
    }

    /// Check if a face is already covered by existing faces
    fn is_face_covered(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        covered_regions: &[(i32, i32, u32, u32)],
    ) -> bool {
        for (face_x, face_y, face_w, face_h) in covered_regions {
            let iou = self.calculate_iou(x, y, width, height, *face_x, *face_y, *face_w, *face_h);

            if iou > 0.3 {
                return true;
            }
        }

        false
    }

    /// Calculate intersection over union for two rectangles
    #[allow(clippy::too_many_arguments)]
    fn calculate_iou(
        &self,
        x1: i32,
        y1: i32,
        w1: u32,
        h1: u32,
        x2: i32,
        y2: i32,
        w2: u32,
        h2: u32,
    ) -> f32 {
        let x1_end = x1 + w1 as i32;
        let y1_end = y1 + h1 as i32;
        let x2_end = x2 + w2 as i32;
        let y2_end = y2 + h2 as i32;

        let x_overlap = (x1_end.min(x2_end) - x1.max(x2)).max(0);
        let y_overlap = (y1_end.min(y2_end) - y1.max(y2)).max(0);

        let intersection = x_overlap * y_overlap;
        let area1 = w1 * h1;
        let area2 = w2 * h2;
        let union = area1 + area2 - intersection as u32;

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Calculate overlap area between two rectangles
    #[allow(clippy::too_many_arguments)]
    fn calculate_overlap_area(
        &self,
        x1: i32,
        y1: i32,
        w1: u32,
        h1: u32,
        x2: i32,
        y2: i32,
        w2: u32,
        h2: u32,
    ) -> i32 {
        let x1_end = x1 + w1 as i32;
        let y1_end = y1 + h1 as i32;
        let x2_end = x2 + w2 as i32;
        let y2_end = y2 + h2 as i32;

        let x_overlap = (x1_end.min(x2_end) - x1.max(x2)).max(0);
        let y_overlap = (y1_end.min(y2_end) - y1.max(y2)).max(0);

        x_overlap * y_overlap
    }

    /// Remove duplicate faces (faces with high IoU)
    fn deduplicate_faces(&self, faces: Vec<(i32, i32, u32, u32)>) -> Vec<(i32, i32, u32, u32)> {
        let mut unique_faces: Vec<(i32, i32, u32, u32)> = vec![];
        let mut indices: Vec<usize> = (0..faces.len()).collect();

        // Sort by size (larger faces first)
        indices.sort_by(|&a, &b| {
            let area_a = faces[a].2 * faces[a].3;
            let area_b = faces[b].2 * faces[b].3;
            area_b.cmp(&area_a)
        });

        for i in indices {
            let face = faces[i];
            let mut is_duplicate = false;

            for existing in &unique_faces {
                let iou = self.calculate_iou(
                    face.0, face.1, face.2, face.3, existing.0, existing.1, existing.2, existing.3,
                );

                if iou > 0.5 {
                    is_duplicate = true;
                    break;
                }
            }

            if !is_duplicate {
                unique_faces.push(face);
            }
        }

        log::info!(
            "Deduplicated: {} faces -> {} unique faces",
            faces.len(),
            unique_faces.len()
        );
        unique_faces
    }

    pub fn update_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.collapsing(t!("face_detection.detail_of_detection_engine"), |ui| {
                // Engine selection
                ui.label(t!("face_detection.engine"));
                egui::ComboBox::from_label(t!("face_detection.engine"))
                    .selected_text(format!("{}", self.engine))
                    .show_ui(ui, |ui| {
                        // Show VisionKit if feature is enabled
                        #[cfg(feature = "face_detection_visionkit")]
                        {
                            ui.selectable_value(
                                &mut self.engine,
                                FaceDetectionEngine::VisionKit,
                                "VisionKit",
                            );
                        }
                        // Show InsightFace (enabled if feature is active, otherwise disabled)
                        #[cfg(feature = "face_detection_insightface")]
                        {
                            ui.selectable_value(
                                &mut self.engine,
                                FaceDetectionEngine::InsightFace,
                                "InsightFace",
                            );
                        }
                        #[cfg(not(feature = "face_detection_insightface"))]
                        {
                            ui.colored_label(
                                ui.visuals().weak_text_color(),
                                "InsightFace (requires feature flag)",
                            );
                        }
                        // Show Candle (pure Rust ONNX, works on WASM + desktop)
                        #[cfg(feature = "face_detection_candle")]
                        {
                            ui.selectable_value(
                                &mut self.engine,
                                FaceDetectionEngine::Candle,
                                "Candle",
                            );
                        }
                    });

                ui.separator();

                // Show speed mode options for InsightFace and Candle
                #[cfg(any(
                    feature = "face_detection_insightface",
                    feature = "face_detection_candle"
                ))]
                {
                    let mut show_speed_mode = false;
                    #[cfg(feature = "face_detection_insightface")]
                    {
                        show_speed_mode |= matches!(self.engine, FaceDetectionEngine::InsightFace);
                    }
                    #[cfg(feature = "face_detection_candle")]
                    {
                        show_speed_mode |= matches!(self.engine, FaceDetectionEngine::Candle);
                    }
                    if show_speed_mode {
                        // Speed mode selection
                        ui.label(t!("face_detection.speed_mode"))
                            .on_hover_text(t!("face_detection.speed_mode_hint"));

                        egui::ComboBox::from_label(t!("face_detection.speed_mode"))
                            .selected_text(self.speed_mode.as_str().to_string())
                            .show_ui(ui, |ui| {
                                let modes = [
                                    (
                                        SpeedMode::Fastest,
                                        t!("face_detection.speed_mode_fastest"),
                                        "Single or two person photo",
                                    ),
                                    (
                                        SpeedMode::Fast,
                                        t!("face_detection.speed_mode_fast"),
                                        "Single or two person photo with an unusual aspect ratio",
                                    ),
                                    (
                                        SpeedMode::Normal,
                                        t!("face_detection.speed_mode_normal"),
                                        "Group photo of around 10 people",
                                    ),
                                    (
                                        SpeedMode::Slow,
                                        t!("face_detection.speed_mode_slow"),
                                        "Group photo of 40~50 people",
                                    ),
                                    (
                                        SpeedMode::Slowest,
                                        t!("face_detection.speed_mode_slowest"),
                                        "Large group photo of more than 50 people (ILC cameras: extends to 640px)",
                                    ),
                                ];

                                for (mode, label, hint) in modes {
                                    ui.selectable_value(&mut self.speed_mode, mode, label)
                                        .on_hover_text(hint);
                                }
                            });

                        ui.separator();
                    }
                } // #[cfg(any(face_detection_insightface, face_detection_candle))]

                // Execution provider selection (InsightFace only)
                #[cfg(feature = "face_detection_insightface")]
                if matches!(self.engine, FaceDetectionEngine::InsightFace) {
                    ui.label(t!("face_detection.execution_provider"))
                        .on_hover_text(t!("face_detection.execution_provider_hint"));

                    egui::ComboBox::from_label(t!("face_detection.execution_provider"))
                        .selected_text(self.provider.as_str().to_string())
                        .show_ui(ui, |ui| {
                            use crate::effect::insightface_detector::ExecutionProvider;

                            ui.selectable_value(
                                &mut self.provider,
                                ExecutionProvider::CPUExecutionProvider,
                                t!("face_detection.provider_cpu"),
                            )
                            .on_hover_text(t!("face_detection.provider_cpu_hint"));

                            ui.selectable_value(
                                &mut self.provider,
                                ExecutionProvider::OnnxAuto,
                                t!("face_detection.provider_onnx_auto"),
                            )
                            .on_hover_text(t!("face_detection.provider_onnx_auto_hint"));

                            #[cfg(target_os = "macos")]
                            ui.selectable_value(
                                &mut self.provider,
                                ExecutionProvider::CoreMLExecutionProvider,
                                t!("face_detection.provider_coreml"),
                            )
                            .on_hover_text(t!("face_detection.provider_coreml_hint"));
                        });

                    ui.separator();
                }
            });
        });
    }

    pub fn get_current_engine_name(&self) -> String {
        #[cfg(feature = "face_detection_insightface")]
        {
            if matches!(self.engine, FaceDetectionEngine::InsightFace) {
                return format!(
                    "{} {} {}",
                    self.engine.display_name(),
                    self.provider.as_str(),
                    self.speed_mode.as_str()
                );
            }
        }
        #[cfg(any(
            feature = "face_detection_insightface",
            feature = "face_detection_candle"
        ))]
        {
            format!(
                "{} {}",
                self.engine.display_name(),
                self.speed_mode.as_str()
            )
        }
        #[cfg(not(any(
            feature = "face_detection_insightface",
            feature = "face_detection_candle"
        )))]
        {
            self.engine.display_name().to_string()
        }
    }
}

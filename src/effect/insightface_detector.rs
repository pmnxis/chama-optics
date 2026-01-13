/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! InsightFace face detector implementation using ONNX Runtime
//!
//! Uses InsightFace buffalo_l model via ONNX Runtime
//! Implements multi-stage slicing algorithm with configurable speed modes and providers

#[cfg(feature = "face_detection_insightface")]
use ort::session::Session;
#[cfg(feature = "face_detection_insightface")]
use ort::value::Tensor;
#[cfg(feature = "face_detection_insightface")]
use std::path::Path;
#[cfg(feature = "face_detection_insightface")]
use std::sync::RwLock;

/// Detection speed modes based on Python test results
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "face_detection_insightface")]
pub enum SpeedMode {
    /// Fastest: No sliding window (whole image resized to 640×640), ~0.5s avg
    Fastest,
    /// Fast: 2560×2560 windows only, ~0.6s avg
    Fast,
    /// Normal: 2560×2560 windows only, ~7s avg
    Normal,
    /// Slow: 2560×2560 and 1280×1280 windows, ~13s avg
    Slow,
    /// Slowest: 2560×2560, 1280×1280, and 640×640 windows, ~28s avg
    Slowest,
}

#[cfg(feature = "face_detection_insightface")]
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
            SpeedMode::Fastest => 0, // No sliding window (whole image resized)
            SpeedMode::Fast => 1,    // Only 2560×2560 windows
            SpeedMode::Normal => 1,  // Only 2560×2560 windows
            SpeedMode::Slow => 2,    // 2560×2560 and 1280×1280 windows
            SpeedMode::Slowest => 3, // 2560×2560, 1280×1280, and 640×640 windows
        }
    }
}

/// ONNX Runtime execution providers
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "face_detection_insightface")]
pub enum ExecutionProvider {
    /// CPU-only execution
    CPUExecutionProvider,
    /// CUDA GPU acceleration
    CUDAExecutionProvider,
    /// Metal Performance Shaders (macOS)
    CoreMLExecutionProvider,
    /// TensorRT (NVIDIA)
    TensorRTExecutionProvider,
}

#[cfg(feature = "face_detection_insightface")]
impl ExecutionProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionProvider::CPUExecutionProvider => "CPUExecutionProvider",
            ExecutionProvider::CUDAExecutionProvider => "CUDAExecutionProvider",
            ExecutionProvider::CoreMLExecutionProvider => "CoreMLExecutionProvider",
            ExecutionProvider::TensorRTExecutionProvider => "TensorRTExecutionProvider",
        }
    }
}

/// InsightFace detector using ONNX Runtime
#[cfg(feature = "face_detection_insightface")]
pub struct InsightFaceDetector {
    session: RwLock<Session>,
    max_depth: u32,
    window_size: u32,
    overlap_ratio: f32,
    #[allow(dead_code)]
    speed_mode: SpeedMode,
    #[allow(dead_code)]
    provider: ExecutionProvider,
}

impl core::default::Default for InsightFaceDetector {
    /// Create detector with default settings (Normal speed, CPU provider)
    fn default() -> Self {
        Self::new(SpeedMode::Normal, ExecutionProvider::CPUExecutionProvider)
    }
}

#[cfg(feature = "face_detection_insightface")]
impl InsightFaceDetector {
    /// Create detector with specified speed mode and execution provider
    /// On macOS/iOS, automatically uses CoreML for hardware acceleration
    pub fn new(speed_mode: SpeedMode, provider: ExecutionProvider) -> Self {
        log::info!("Loading InsightFace ONNX model...");
        log::info!(
            "Speed mode: {}, Provider: {}",
            speed_mode.as_str(),
            provider.as_str()
        );

        // On macOS/iOS, automatically use CoreML for hardware acceleration
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let actual_provider = {
            if matches!(provider, ExecutionProvider::CPUExecutionProvider) {
                log::info!(
                    "Auto-selecting CoreML Execution Provider for hardware acceleration on Apple platform"
                );
                ExecutionProvider::CoreMLExecutionProvider
            } else {
                provider
            }
        };

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        let actual_provider = provider;

        // Load InsightFace face detection model (det_10g.onnx)
        // Try multiple paths in order:
        // 1. Environment variable INSIGHTFACE_MODEL_PATH
        // 2. assets/download/det_10g.onnx (for macOS/desktop builds)
        // 3. models/buffalo_l/det_10g.onnx (alternative path)
        let model_path = if let Ok(env_path) = std::env::var("INSIGHTFACE_MODEL_PATH") {
            log::info!("Using model from INSIGHTFACE_MODEL_PATH: {}", env_path);
            Path::new(&env_path).to_path_buf()
        } else if Path::new("assets/download/det_10g.onnx").exists() {
            log::info!("Using model from assets/download/det_10g.onnx");
            Path::new("assets/download/det_10g.onnx").to_path_buf()
        } else if Path::new("models/buffalo_l/det_10g.onnx").exists() {
            log::info!("Using model from models/buffalo_l/det_10g.onnx");
            Path::new("models/buffalo_l/det_10g.onnx").to_path_buf()
        } else {
            // Model not found - provide helpful error message
            let error_msg = "InsightFace model file not found. Searched paths:\n\
                 1. INSIGHTFACE_MODEL_PATH environment variable\n\
                 2. assets/download/det_10g.onnx\n\
                 3. models/buffalo_l/det_10g.onnx\n\n\
                 To download the model, build with: cargo build --features build_assets\n\
                 Or download manually and place in one of the paths above."
                .to_string();
            log::error!("{}", error_msg);
            panic!("{}", error_msg);
        };

        if !model_path.exists() {
            let error_msg = format!("InsightFace model path does not exist: {:?}", model_path);
            log::error!("{}", error_msg);
            panic!("{}", error_msg);
        }

        log::info!("Loading InsightFace ONNX model from: {:?}", model_path);

        // Read model file and load into ONNX Runtime
        let model_bytes = std::fs::read(model_path).expect("Failed to read InsightFace model file");

        // Create ONNX Runtime session with specified provider
        let session_builder = Session::builder()
            .unwrap_or_else(|e| panic!("Failed to create session builder: {}", e));

        // Configure execution provider
        let session = match actual_provider {
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            ExecutionProvider::CoreMLExecutionProvider => {
                log::info!("Enabling CoreML Execution Provider for hardware acceleration");
                session_builder
                    .with_execution_providers([
                        ort::execution_providers::CoreMLExecutionProvider::default().build(),
                        ort::execution_providers::CPUExecutionProvider::default().build(),
                    ])
                    .unwrap_or_else(|e| {
                        panic!("Failed to configure CoreML execution provider: {}", e)
                    })
            }
            _ => session_builder,
        };

        let session = session
            .commit_from_memory(&model_bytes)
            .expect("Failed to load ONNX model");

        log::info!("InsightFace ONNX model loaded successfully");

        Self {
            session: RwLock::new(session),
            max_depth: speed_mode.max_depth(),
            window_size: 640,
            overlap_ratio: 0.1, // 10% overlap for sliding windows
            speed_mode,
            provider,
        }
    }

    /// Preprocess image for InsightFace model
    /// Convert RGB image to normalized CHW tensor
    fn preprocess_image(&self, img: &image::DynamicImage) -> Vec<f32> {
        let target_size = self.window_size as usize;
        let resized = img.resize_exact(
            target_size as u32,
            target_size as u32,
            image::imageops::FilterType::Lanczos3,
        );

        // Convert to RGB
        let rgb = resized.to_rgb8();

        // Convert to CHW tensor [3, 640, 640]
        let mut data = Vec::with_capacity(3 * target_size * target_size);
        for c in 0..3 {
            for y in 0..target_size {
                for x in 0..target_size {
                    let pixel = rgb.get_pixel(x as u32, y as u32);
                    let val = match c {
                        0 => pixel[0], // R
                        1 => pixel[1], // G
                        2 => pixel[2], // B
                        _ => pixel[0],
                    };
                    // Normalize to [0, 1]
                    data.push(val as f32 / 255.0);
                }
            }
        }

        data
    }

    /// Generate anchors for a given stride
    fn generate_anchors(&self, stride: usize, feat_size: usize) -> Vec<(f32, f32)> {
        let mut anchors = Vec::new();
        // RetinaFace uses 2 anchors per location
        let num_anchors = 2;

        for i in 0..feat_size {
            for j in 0..feat_size {
                for _ in 0..num_anchors {
                    let cx = (j as f32 + 0.5) * stride as f32;
                    let cy = (i as f32 + 0.5) * stride as f32;
                    anchors.push((cx, cy));
                }
            }
        }
        anchors
    }

    /// Decode bbox predictions with anchors
    fn decode_bbox(
        &self,
        bbox_pred: &[f32],
        anchor: (f32, f32),
        stride: f32,
    ) -> (f32, f32, f32, f32) {
        // InsightFace SCRFD format: bbox_pred = [distance_left, distance_top, distance_right, distance_bottom]
        // from anchor point, scaled by stride
        let distance_left = bbox_pred[0] * stride;
        let distance_top = bbox_pred[1] * stride;
        let distance_right = bbox_pred[2] * stride;
        let distance_bottom = bbox_pred[3] * stride;

        // Calculate corners from anchor and distances
        let x1 = anchor.0 - distance_left;
        let y1 = anchor.1 - distance_top;
        let x2 = anchor.0 + distance_right;
        let y2 = anchor.1 + distance_bottom;

        // Convert to (x, y, w, h) format
        let x = x1;
        let y = y1;
        let w = x2 - x1;
        let h = y2 - y1;

        // Clamp to valid ranges (0 to 640)
        let x = x.clamp(0.0, 640.0);
        let y = y.clamp(0.0, 640.0);
        let w = w.max(1.0).min(640.0 - x);
        let h = h.max(1.0).min(640.0 - y);

        (x, y, w, h)
    }

    /// Run ONNX inference and decode outputs
    /// Returns detections as Vec<(x, y, w, h, score)>
    fn run_inference(&self, input_data: Vec<f32>) -> Vec<(f32, f32, f32, f32, f32)> {
        log::debug!("Running ONNX inference...");

        // Get session write lock for inference
        let mut session = self.session.write().unwrap();
        let allocator = session.allocator();

        // Create input tensor with correct shape [1, 3, 640, 640]
        let mut input_tensor = Tensor::<f32>::new(allocator, [1i64, 3, 640, 640])
            .expect("Failed to create input tensor");

        // Fill tensor with data
        {
            let (_, data) = input_tensor.extract_tensor_mut();
            data.copy_from_slice(&input_data);
        }

        // Run inference
        let outputs = session
            .run(ort::inputs![&input_tensor])
            .expect("ONNX inference failed");

        log::debug!("Number of outputs: {}", outputs.len());

        // Convert outputs to Vec for easier indexing
        let output_values: Vec<ort::value::Value> = outputs.into_iter().map(|(_, v)| v).collect();

        // Log output shapes for debugging
        for (i, output) in output_values.iter().enumerate() {
            if let Ok((shape, _)) = output.try_extract_tensor::<f32>() {
                log::debug!("Output {}: shape = {:?}", i, shape);
            }
        }

        // InsightFace det_10g outputs 9 tensors grouped by type:
        // Outputs 0-2: scores for stride 8, 16, 32
        // Outputs 3-5: bboxes for stride 8, 16, 32
        // Outputs 6-8: keypoints for stride 8, 16, 32

        let mut all_detections = Vec::new();

        // Strides and corresponding feature sizes for 640x640 input
        let strides = [8, 16, 32];
        let feat_sizes = [80, 40, 20]; // 640/8=80, 640/16=40, 640/32=20

        // Process all 3 strides
        for stride_idx in 0..3 {
            // Indices: scores are 0-2, bboxes are 3-5, kps are 6-8
            let score_idx = stride_idx;
            let bbox_idx = stride_idx + 3;
            let kps_idx = stride_idx + 6;

            // Skip if not enough outputs
            if kps_idx >= output_values.len() {
                log::warn!("Not enough outputs for stride {}", stride_idx);
                continue;
            }

            // Extract score, bbox, kps tensors
            let scores = match output_values[score_idx].try_extract_tensor::<f32>() {
                Ok((_, data)) => data.to_vec(),
                Err(e) => {
                    log::warn!("Failed to extract scores at stride {}: {}", stride_idx, e);
                    continue;
                }
            };

            let bboxes = match output_values[bbox_idx].try_extract_tensor::<f32>() {
                Ok((_, data)) => data.to_vec(),
                Err(e) => {
                    log::warn!("Failed to extract bboxes at stride {}: {}", stride_idx, e);
                    continue;
                }
            };

            let _kps = match output_values[kps_idx].try_extract_tensor::<f32>() {
                Ok((_, data)) => data.to_vec(),
                Err(e) => {
                    log::warn!(
                        "Failed to extract keypoints at stride {}: {}",
                        stride_idx,
                        e
                    );
                    continue;
                }
            };

            // Generate anchors for this stride
            let stride = strides[stride_idx];
            let feat_size = feat_sizes[stride_idx];
            let anchors = self.generate_anchors(stride, feat_size);

            // Calculate number of detections for this stride
            let num_dets = scores.len();
            let num_bbox_dets = bboxes.len() / 4;

            if num_dets != num_bbox_dets {
                log::warn!(
                    "Mismatched detection counts at stride {}: scores={}, bboxes={}",
                    stride_idx,
                    num_dets,
                    num_bbox_dets
                );
                continue;
            }

            if num_dets != anchors.len() {
                log::warn!(
                    "Anchor count mismatch at stride {}: anchors={}, detections={}",
                    stride_idx,
                    anchors.len(),
                    num_dets
                );
                continue;
            }

            log::debug!("Stride {} has {} detections", stride_idx, num_dets);

            // Log first few high-confidence detections for debugging
            let mut logged_count = 0;
            for i in 0..num_dets {
                let score = scores[i];
                if score > 0.5 && logged_count < 3 {
                    let bbox_pred = [
                        bboxes[i * 4],
                        bboxes[i * 4 + 1],
                        bboxes[i * 4 + 2],
                        bboxes[i * 4 + 3],
                    ];
                    let anchor = anchors[i];
                    let (x, y, w, h) = self.decode_bbox(&bbox_pred, anchor, stride as f32);
                    log::debug!(
                        "  Sample stride {} detection: score={:.3}, raw_bbox=[{:.2}, {:.2}, {:.2}, {:.2}], anchor=({:.1}, {:.1}), decoded=({:.1}, {:.1}, {:.1}, {:.1})",
                        stride_idx,
                        score,
                        bbox_pred[0],
                        bbox_pred[1],
                        bbox_pred[2],
                        bbox_pred[3],
                        anchor.0,
                        anchor.1,
                        x,
                        y,
                        w,
                        h
                    );
                    logged_count += 1;
                }
            }

            // Parse detections with anchor decoding
            for i in 0..num_dets {
                let score = scores[i];

                // Extract bbox deltas
                let bbox_pred = [
                    bboxes[i * 4],
                    bboxes[i * 4 + 1],
                    bboxes[i * 4 + 2],
                    bboxes[i * 4 + 3],
                ];

                // Decode bbox with anchor
                let (x, y, w, h) = self.decode_bbox(&bbox_pred, anchors[i], stride as f32);

                all_detections.push((x, y, w, h, score));
            }
        }

        log::debug!(
            "Total detections across all strides: {}",
            all_detections.len()
        );
        all_detections
    }

    /// Postprocess output to face rectangles with NMS
    fn postprocess_output(
        &self,
        detections: Vec<(f32, f32, f32, f32, f32)>,
        img_width: u32,
        img_height: u32,
        score_threshold: f32,
    ) -> Vec<(i32, i32, u32, u32)> {
        let mut faces = vec![];

        log::debug!("Processing {} detections", detections.len());

        // Extract high-confidence faces
        let mut candidate_faces: Vec<(f32, f32, f32, f32, f32)> = Vec::new();
        for (x, y, w, h, score) in detections {
            if score >= score_threshold {
                candidate_faces.push((x, y, w, h, score));
            }
        }

        log::debug!("Found {} faces above threshold", candidate_faces.len());

        // Apply NMS
        let nms_faces = self.nms_float(candidate_faces);

        // Convert to original image coordinates
        let scale_x = img_width as f32 / self.window_size as f32;
        let scale_y = img_height as f32 / self.window_size as f32;

        for (x, y, w, h, score) in nms_faces {
            // Scale to original image size
            let x_scaled = x * scale_x;
            let y_scaled = y * scale_y;
            let w_scaled = w * scale_x;
            let h_scaled = h * scale_y;

            // Clamp to image bounds
            let x1 = x_scaled.max(0.0).min(img_width as f32 - 1.0) as i32;
            let y1 = y_scaled.max(0.0).min(img_height as f32 - 1.0) as i32;
            let x2 = (x_scaled + w_scaled).max(0.0).min(img_width as f32);
            let y2 = (y_scaled + h_scaled).max(0.0).min(img_height as f32);

            let width = (x2 - x1 as f32).max(1.0) as u32;
            let height = (y2 - y1 as f32).max(1.0) as u32;

            // Filter out tiny or invalid boxes (minimum 10x10 pixels)
            if width >= 10 && height >= 10 && width <= img_width && height <= img_height {
                log::debug!(
                    "Face: score={:.3}, bbox=({}, {}, {}, {})",
                    score,
                    x1,
                    y1,
                    width,
                    height
                );
                faces.push((x1, y1, width, height));
            } else {
                log::trace!(
                    "Filtered out invalid box: score={:.3}, bbox=({}, {}, {}, {})",
                    score,
                    x1,
                    y1,
                    width,
                    height
                );
            }
        }

        faces
    }

    /// Non-Maximum Suppression for float coordinates
    fn nms_float(
        &self,
        mut faces: Vec<(f32, f32, f32, f32, f32)>,
    ) -> Vec<(f32, f32, f32, f32, f32)> {
        if faces.is_empty() {
            return faces;
        }

        // Sort by score (highest first)
        faces.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

        let mut keep: Vec<(f32, f32, f32, f32, f32)> = vec![];
        let iou_threshold = 0.4;

        for face in faces {
            let mut should_keep = true;

            for kept in &keep {
                let iou = self.calculate_iou_float(
                    &(face.0, face.1, face.2, face.3),
                    &(kept.0, kept.1, kept.2, kept.3),
                );

                if iou >= iou_threshold {
                    should_keep = false;
                    break;
                }
            }

            if should_keep {
                keep.push(face);
            }
        }

        keep
    }

    /// Calculate IoU between two rectangles (float version)
    fn calculate_iou_float(&self, a: &(f32, f32, f32, f32), b: &(f32, f32, f32, f32)) -> f32 {
        let (ax1, ay1, aw, ah) = *a;
        let (bx1, by1, bw, bh) = *b;

        let ax2 = ax1 + aw;
        let ay2 = ay1 + ah;
        let bx2 = bx1 + bw;
        let by2 = by1 + bh;

        // Calculate intersection
        let x1 = ax1.max(bx1);
        let y1 = ay1.max(by1);
        let x2 = ax2.min(bx2);
        let y2 = ay2.min(by2);

        if x2 < x1 || y2 < y1 {
            return 0.0;
        }

        let intersection = (x2 - x1) * (y2 - y1);
        let area_a = aw * ah;
        let area_b = bw * bh;
        let union = area_a + area_b - intersection;

        if union == 0.0 {
            return 0.0;
        }

        intersection / union
    }

    /// Non-Maximum Suppression for int coordinates
    fn nms(
        &self,
        mut faces: Vec<(i32, i32, u32, u32)>,
        iou_threshold: f32,
    ) -> Vec<(i32, i32, u32, u32)> {
        if faces.is_empty() {
            return faces;
        }

        // Sort by area (largest first)
        faces.sort_by(|a, b| {
            let area_a = a.2 * a.3;
            let area_b = b.2 * b.3;
            area_b.cmp(&area_a)
        });

        let mut keep = vec![];
        while let Some(current) = faces.pop() {
            keep.push(current);
            faces.retain(|face| self.calculate_iou(&current, face) < iou_threshold);
        }

        keep
    }

    /// Calculate IoU between two rectangles (int version)
    fn calculate_iou(&self, a: &(i32, i32, u32, u32), b: &(i32, i32, u32, u32)) -> f32 {
        let (ax1, ay1, aw, ah) = *a;
        let (bx1, by1, bw, bh) = *b;

        let ax2 = ax1 + aw as i32;
        let ay2 = ay1 + ah as i32;
        let bx2 = bx1 + bw as i32;
        let by2 = by1 + bh as i32;

        // Calculate intersection
        let x1 = ax1.max(bx1);
        let y1 = ay1.max(by1);
        let x2 = ax2.min(bx2);
        let y2 = ay2.min(by2);

        if x2 < x1 || y2 < y1 {
            return 0.0;
        }

        let intersection = (x2 - x1) * (y2 - y1);
        let area_a = aw * ah;
        let area_b = bw * bh;
        let union = area_a + area_b - intersection as u32;

        intersection as f32 / union as f32
    }

    /// Detect faces in a single window using ONNX
    fn detect_single(
        &self,
        img: &image::DynamicImage,
        offset_x: i32,
        offset_y: i32,
        orig_width: u32,
        orig_height: u32,
    ) -> Vec<(i32, i32, u32, u32)> {
        log::debug!("Running single detection...");

        // Preprocess image
        let input_data = self.preprocess_image(img);

        // Run inference
        let detections = self.run_inference(input_data);

        // Postprocess with original image dimensions
        let faces = self.postprocess_output(detections, orig_width, orig_height, 0.5);

        // Adjust coordinates to global image
        faces
            .into_iter()
            .map(|(x, y, w, h)| {
                let global_x = offset_x + x;
                let global_y = offset_y + y;
                (global_x, global_y, w, h)
            })
            .collect()
    }

    /// Detect faces using 640x640 sliding window with configurable depth
    pub fn detect_faces_sliding_window(
        &self,
        image_path: &Path,
        max_depth: Option<u32>,
    ) -> Vec<(i32, i32, u32, u32)> {
        let max_depth = max_depth.unwrap_or(self.max_depth);
        log::info!(
            "Sliding window detection: window=640x640, max_depth={}, overlap={:.0}%",
            max_depth,
            self.overlap_ratio * 100.0
        );

        // Load image
        let img = match image::open(image_path) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image: {}", e);
                return vec![];
            }
        };

        let img_width = img.width();
        let img_height = img.height();

        // If image fits in window, just run detection once
        if img_width <= self.window_size && img_height <= self.window_size {
            log::info!("Image fits in window, running single detection");
            return self.detect_single(&img, 0, 0, img_width, img_height);
        }

        // Special case: max_depth=0 means no sliding window (Fastest mode)
        if max_depth == 0 {
            log::info!("Fastest mode: processing whole image resized to 640×640");
            return self.detect_single(&img, 0, 0, img_width, img_height);
        }

        // Calculate scaling factors for different depths
        // Process from largest windows (coarse) to smallest (fine detail)
        // Depth 0: largest window (2^max_depth × 640)
        // Depth 1: half size (2^(max_depth-1) × 640)
        // ...
        // Depth max_depth-1: base window (640x640)
        let mut all_faces = vec![];

        for depth in 0..max_depth as usize {
            // Reverse the depth to process large windows first
            let scale_factor = 1 << (max_depth as usize - depth - 1); // 2^(max_depth - depth - 1)
            let window_scaled = self.window_size * scale_factor;

            log::info!(
                "Processing depth {}: window_size={}, scale={}",
                depth,
                window_scaled,
                scale_factor
            );

            // Calculate step size (with overlap)
            let step = (window_scaled as f32 * (1.0 - self.overlap_ratio)) as i32;

            // Generate sliding window positions
            let mut windows = vec![];
            let mut x = 0i32;
            while x < img_width as i32 {
                let mut y = 0i32;
                while y < img_height as i32 {
                    let w = (window_scaled as i32).min(img_width as i32 - x);
                    let h = (window_scaled as i32).min(img_height as i32 - y);
                    if w > 0 && h > 0 {
                        windows.push((x, y, w as u32, h as u32));
                    }
                    y += step;
                }
                x += step;
            }

            log::debug!("Generated {} windows at depth {}", windows.len(), depth);

            // Detect faces in each window
            for (wx, wy, ww, wh) in windows {
                // Validate window dimensions
                if ww == 0 || wh == 0 {
                    log::debug!("Skipping window with zero dimensions at ({}, {})", wx, wy);
                    continue;
                }

                // Calculate actual crop dimensions with bounds checking
                let crop_x = wx as u32;
                let crop_y = wy as u32;
                let crop_w = ww.min(img.width().saturating_sub(crop_x));
                let crop_h = wh.min(img.height().saturating_sub(crop_y));

                // Skip if crop dimensions are invalid
                if crop_w == 0 || crop_h == 0 {
                    log::debug!(
                        "Skipping window with invalid crop dimensions: crop_w={}, crop_h={} at ({}, {})",
                        crop_w,
                        crop_h,
                        wx,
                        wy
                    );
                    continue;
                }

                // Crop window region
                let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);

                // Double-check crop is valid
                if cropped.width() == 0 || cropped.height() == 0 {
                    log::debug!("Skipping empty window crop at ({}, {})", wx, wy);
                    continue;
                }

                // Run detection using ONNX
                let window_faces = self.detect_single(&cropped, wx, wy, ww, wh);

                // Add to results
                all_faces.extend(window_faces);
            }
        }

        // Apply NMS to remove duplicates
        all_faces = self.nms(all_faces, 0.4);

        log::info!("Sliding window complete: {} faces found", all_faces.len());
        all_faces
    }
}

#[cfg(not(feature = "face_detection_insightface"))]
/// Placeholder type when feature is disabled
pub struct InsightFaceDetector;

#[cfg(feature = "face_detection_insightface")]
impl super::face_detectors::FaceDetector for InsightFaceDetector {
    fn detect_faces(&self, image_path: &Path) -> Vec<(i32, i32, u32, u32)> {
        // Load image to get dimensions
        let img = match image::open(image_path) {
            Ok(img) => img,
            Err(e) => {
                log::error!("Failed to load image: {}", e);
                return vec![];
            }
        };

        let img_width = img.width();
        let img_height = img.height();

        log::info!(
            "Running InsightFace inference on {}x{} image",
            img_width,
            img_height
        );

        // For small images, use single detection
        if img_width <= 640 && img_height <= 640 {
            let faces = self.detect_single(&img, 0, 0, img_width, img_height);
            log::info!("InsightFace detected {} faces", faces.len());
            faces
        } else {
            // For large images, use sliding window with appropriate depth
            // Use SpeedMode's max_depth, which provides sensible defaults:
            // Fastest/Fast: 0 (no sliding window, just resized image)
            // Normal: 1 (640×640 and 1280×1280 windows)
            // Slow: 2 (640×640, 1280×1280, and 2560×2560 windows)
            // Slowest: 3 (640×640, 1280×1280, 2560×2560, and 5120×5120 windows)
            self.detect_faces_sliding_window(image_path, None) // Use default from SpeedMode
        }
    }

    fn engine_name(&self) -> &'static str {
        "InsightFace (ONNX)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_insightface_detector() {
    //     let detector =
    //         InsightFaceDetector::new(SpeedMode::Normal, ExecutionProvider::CPUExecutionProvider);
    //     assert_eq!(detector.engine_name(), "InsightFace (ONNX)");
    //     assert_eq!(detector.max_depth, 1); // Normal mode has max_depth 1
    //     assert_eq!(detector.window_size, 640);
    //     assert_eq!(detector.speed_mode, SpeedMode::Normal);
    //     assert_eq!(detector.provider, ExecutionProvider::CPUExecutionProvider);
    // }

    #[test]
    #[cfg(feature = "face_detection_insightface")]
    fn test_speed_modes() {
        assert_eq!(SpeedMode::Fastest.max_depth(), 0);
        assert_eq!(SpeedMode::Fast.max_depth(), 0);
        assert_eq!(SpeedMode::Normal.max_depth(), 1);
        assert_eq!(SpeedMode::Slow.max_depth(), 2);
        assert_eq!(SpeedMode::Slowest.max_depth(), 3);
    }

    #[test]
    #[cfg(feature = "face_detection_insightface")]
    fn test_default_detector() {
        let detector = InsightFaceDetector::default();
        assert_eq!(detector.speed_mode, SpeedMode::Normal);
        assert_eq!(detector.provider, ExecutionProvider::CPUExecutionProvider);
    }

    #[test]
    #[cfg(not(feature = "face_detection_insightface"))]
    fn test_insightface_disabled() {
        // This test ensures that module compiles when the feature is disabled
    }
}

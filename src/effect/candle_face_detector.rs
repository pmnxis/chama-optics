/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Face detection using candle-onnx (pure Rust, works on WASM + desktop).
//! Runs the InsightFace SCRFD det_10g.onnx model via candle-onnx::simple_eval.
//! Preprocessing and postprocessing are ported from insightface_detector.rs.

use candle_core::{Device, Tensor};
use std::collections::HashMap;

use crate::effect::face_detection::SpeedMode;

/// Face detector backed by candle-onnx (SCRFD / det_10g.onnx).
pub struct CandleFaceDetector {
    model: candle_onnx::onnx::ModelProto,
    window_size: u32,
    overlap_ratio: f32,
}

/// Get the embedded ONNX model bytes (shared with ort_web_detector on WASM).
pub fn model_bytes() -> &'static [u8] {
    include_bytes!(env!("INSIGHTFACE_MODEL_PATH"))
}

impl CandleFaceDetector {
    /// Create a new detector from embedded model bytes.
    pub fn new() -> Result<Self, String> {
        Self::from_bytes(model_bytes())
    }

    /// Create a new detector from raw ONNX model bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        use prost::Message;
        let mut model = candle_onnx::onnx::ModelProto::decode(bytes)
            .map_err(|e| format!("Failed to decode ONNX model: {}", e))?;

        // Fix Resize nodes: SCRFD model exports Resize ops with both `scales` (input[2])
        // and `sizes` (input[3]) input names set, but only one has real data.
        // candle-onnx's simple_eval checks the name string (not tensor content), so it
        // incorrectly thinks both are present. Clear the unused input.
        if let Some(graph) = &mut model.graph {
            for node in &mut graph.node {
                if node.op_type == "Resize" && node.input.len() >= 4
                    && !node.input[2].is_empty() && !node.input[3].is_empty() {
                        // Both scales and sizes have names — prefer sizes, clear scales
                        node.input[2] = String::new();
                    }
            }
        }

        Ok(Self {
            model,
            window_size: 640,
            overlap_ratio: 0.1,
        })
    }

    /// Detect faces in a DynamicImage with speed mode. Returns (x, y, width, height) rectangles.
    pub fn detect_faces_from_image(
        &self,
        img: &image::DynamicImage,
        speed_mode: SpeedMode,
    ) -> Vec<(i32, i32, u32, u32)> {
        let img_width = img.width();
        let img_height = img.height();

        log::info!(
            "CandleFaceDetector: running on {}x{} image, speed_mode={}",
            img_width,
            img_height,
            speed_mode.as_str()
        );

        // If image fits in window, just run single detection
        if img_width <= self.window_size && img_height <= self.window_size {
            return self.detect_single(img, 0, 0, img_width, img_height);
        }

        // Step 1: Always run on the whole image resized to 640x640
        let mut all_faces = self.detect_single(img, 0, 0, img_width, img_height);

        if speed_mode == SpeedMode::Fastest {
            log::info!("Fastest mode: whole image resized to 640x640 only");
            return all_faces;
        }

        // Step 2: Fast mode — sliding windows using min(w,h) as window size
        let window_size = img_width.min(img_height);
        let step = (window_size as f32 * (1.0 - self.overlap_ratio)) as i32;

        log::info!("Fast mode: window_size={}", window_size);

        let mut windows = Vec::new();
        let mut x = 0i32;
        while x < img_width as i32 {
            let mut y = 0i32;
            while y < img_height as i32 {
                let w = (window_size as i32).min(img_width as i32 - x);
                let h = (window_size as i32).min(img_height as i32 - y);
                if w > 0 && h > 0 {
                    windows.push((x, y, w as u32, h as u32));
                }
                y += step;
            }
            x += step;
        }

        for (wx, wy, ww, wh) in &windows {
            let crop_x = *wx as u32;
            let crop_y = *wy as u32;
            let crop_w = (*ww).min(img.width().saturating_sub(crop_x));
            let crop_h = (*wh).min(img.height().saturating_sub(crop_y));
            if crop_w == 0 || crop_h == 0 {
                continue;
            }
            let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
            all_faces.extend(self.detect_single(&cropped, *wx, *wy, *ww, *wh));
        }

        if speed_mode == SpeedMode::Fast {
            return Self::nms_final(all_faces);
        }

        // Step 3+: Deeper sliding windows (Normal/Slow/Slowest)
        let max_depth = speed_mode.max_depth();
        for depth in 0..max_depth as usize {
            let scale_factor = 1u32 << (max_depth as usize - depth - 1);
            let window_scaled = self.window_size * scale_factor;

            log::info!(
                "Depth {}: window_size={}, scale={}",
                depth,
                window_scaled,
                scale_factor
            );

            let step = (window_scaled as f32 * (1.0 - self.overlap_ratio)) as i32;
            let mut x = 0i32;
            while x < img_width as i32 {
                let mut y = 0i32;
                while y < img_height as i32 {
                    let w = (window_scaled as i32).min(img_width as i32 - x);
                    let h = (window_scaled as i32).min(img_height as i32 - y);
                    if w > 0 && h > 0 {
                        let crop_x = x as u32;
                        let crop_y = y as u32;
                        let crop_w = (w as u32).min(img.width().saturating_sub(crop_x));
                        let crop_h = (h as u32).min(img.height().saturating_sub(crop_y));
                        if crop_w > 0 && crop_h > 0 {
                            let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
                            all_faces
                                .extend(self.detect_single(&cropped, x, y, w as u32, h as u32));
                        }
                    }
                    y += step;
                }
                x += step;
            }
        }

        Self::nms_final(all_faces)
    }

    /// Run detection on a single image region (resized to 640x640).
    /// Returned coordinates are in the original image space.
    fn detect_single(
        &self,
        img: &image::DynamicImage,
        offset_x: i32,
        offset_y: i32,
        region_w: u32,
        region_h: u32,
    ) -> Vec<(i32, i32, u32, u32)> {
        let input_data = self.preprocess_image(img);

        let detections = match self.run_inference(input_data) {
            Ok(dets) => dets,
            Err(e) => {
                log::error!("CandleFaceDetector inference failed: {}", e);
                return vec![];
            }
        };

        // Scale to region coordinates, then offset to global
        let scale_x = region_w as f32 / self.window_size as f32;
        let scale_y = region_h as f32 / self.window_size as f32;

        let mut faces = Vec::new();
        for (x, y, w, h, score) in detections {
            if score < 0.5 {
                continue;
            }
            let gx = (x * scale_x) as i32 + offset_x;
            let gy = (y * scale_y) as i32 + offset_y;
            let gw = (w * scale_x).max(1.0) as u32;
            let gh = (h * scale_y).max(1.0) as u32;

            if gw >= 10 && gh >= 10 {
                faces.push((gx, gy, gw, gh));
            }
        }
        faces
    }

    // ---- Preprocessing (from insightface_detector.rs) ----

    fn preprocess_image(&self, img: &image::DynamicImage) -> Vec<f32> {
        let target_size = self.window_size as usize;
        let resized = img.resize_exact(
            target_size as u32,
            target_size as u32,
            image::imageops::FilterType::Lanczos3,
        );
        let rgb = resized.to_rgb8();

        // CHW tensor [3, H, W], normalised to [0, 1]
        let mut data = Vec::with_capacity(3 * target_size * target_size);
        for c in 0..3u8 {
            for y in 0..target_size {
                for x in 0..target_size {
                    let pixel = rgb.get_pixel(x as u32, y as u32);
                    data.push(pixel[c as usize] as f32 / 255.0);
                }
            }
        }
        data
    }

    // ---- Inference via candle-onnx ----

    #[allow(clippy::type_complexity)]
    fn run_inference(
        &self,
        input_data: Vec<f32>,
    ) -> Result<Vec<(f32, f32, f32, f32, f32)>, String> {
        let device = Device::Cpu;
        let ws = self.window_size as usize;

        // Create input tensor [1, 3, ws, ws]
        let input_tensor =
            Tensor::from_vec(input_data, &[1, 3, ws, ws], &device).map_err(|e| e.to_string())?;

        // Discover the input name from the model graph
        let graph = self.model.graph.as_ref().ok_or("ONNX model has no graph")?;
        let input_name = graph
            .input
            .first()
            .ok_or("ONNX model has no inputs")?
            .name
            .clone();

        let mut inputs = HashMap::new();
        inputs.insert(input_name, input_tensor);

        // Run inference
        let outputs = candle_onnx::simple_eval(&self.model, inputs).map_err(|e| e.to_string())?;

        // Classify outputs by last dimension:
        //   scores: last_dim=1, bboxes: last_dim=4, kps: last_dim=10
        // Within each group, sort by descending anchor count (stride 8→16→32)
        let mut scores_list: Vec<(&String, &Tensor, usize)> = Vec::new();
        let mut bboxes_list: Vec<(&String, &Tensor, usize)> = Vec::new();

        for (name, tensor) in &outputs {
            let dims = tensor.dims();
            if dims.len() < 2 {
                continue;
            }
            let last_dim = dims[dims.len() - 1];
            let anchor_count = dims[dims.len() - 2];
            match last_dim {
                1 => scores_list.push((name, tensor, anchor_count)),
                4 => bboxes_list.push((name, tensor, anchor_count)),
                _ => {} // kps (10) or other — skip
            }
        }

        // Sort by anchor count descending (largest feat_size = stride 8 first)
        scores_list.sort_by(|a, b| b.2.cmp(&a.2));
        bboxes_list.sort_by(|a, b| b.2.cmp(&a.2));

        if scores_list.len() < 3 || bboxes_list.len() < 3 {
            return Err(format!(
                "Expected 3 score and 3 bbox outputs, got {} scores and {} bboxes",
                scores_list.len(),
                bboxes_list.len()
            ));
        }

        log::info!(
            "Output mapping: scores=[{}], bboxes=[{}]",
            scores_list
                .iter()
                .map(|(n, _, c)| format!("{}({})", n, c))
                .collect::<Vec<_>>()
                .join(", "),
            bboxes_list
                .iter()
                .map(|(n, _, c)| format!("{}({})", n, c))
                .collect::<Vec<_>>()
                .join(", "),
        );

        // SCRFD strides: 8, 16, 32 → feat_sizes: 80, 40, 20
        let strides = [8usize, 16, 32];
        let feat_sizes = [80usize, 40, 20]; // 640/8, 640/16, 640/32

        let mut all_detections = Vec::new();

        for stride_idx in 0..3 {
            let scores: Vec<f32> = scores_list[stride_idx]
                .1
                .flatten_all()
                .and_then(|t| t.to_vec1())
                .map_err(|e| format!("scores extract: {}", e))?;

            let bboxes: Vec<f32> = bboxes_list[stride_idx]
                .1
                .flatten_all()
                .and_then(|t| t.to_vec1())
                .map_err(|e| format!("bboxes extract: {}", e))?;

            let stride = strides[stride_idx];
            let feat_size = feat_sizes[stride_idx];
            let anchors = Self::generate_anchors(stride, feat_size);

            let num_dets = scores.len();
            let num_bbox_dets = bboxes.len() / 4;

            if num_dets != num_bbox_dets || num_dets != anchors.len() {
                log::warn!(
                    "Stride {} count mismatch: scores={}, bboxes/4={}, anchors={}",
                    stride,
                    num_dets,
                    num_bbox_dets,
                    anchors.len()
                );
                continue;
            }

            for i in 0..num_dets {
                let score = scores[i];
                let bbox_pred = &bboxes[i * 4..i * 4 + 4];
                let (x, y, w, h) = Self::decode_bbox(bbox_pred, anchors[i], stride as f32);
                all_detections.push((x, y, w, h, score));
            }
        }

        Ok(all_detections)
    }

    // ---- Anchor generation & bbox decoding (from insightface_detector.rs) ----

    fn generate_anchors(stride: usize, feat_size: usize) -> Vec<(f32, f32)> {
        let mut anchors = Vec::new();
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

    fn decode_bbox(bbox_pred: &[f32], anchor: (f32, f32), stride: f32) -> (f32, f32, f32, f32) {
        let dl = bbox_pred[0] * stride;
        let dt = bbox_pred[1] * stride;
        let dr = bbox_pred[2] * stride;
        let db = bbox_pred[3] * stride;

        let x1 = anchor.0 - dl;
        let y1 = anchor.1 - dt;
        let x2 = anchor.0 + dr;
        let y2 = anchor.1 + db;

        let x = x1.clamp(0.0, 640.0);
        let y = y1.clamp(0.0, 640.0);
        let w = (x2 - x1).max(1.0).min(640.0 - x);
        let h = (y2 - y1).max(1.0).min(640.0 - y);

        (x, y, w, h)
    }

    // ---- Post-processing / NMS ----

    /// Final NMS on integer (global) face rectangles from all windows.
    fn nms_final(faces: Vec<(i32, i32, u32, u32)>) -> Vec<(i32, i32, u32, u32)> {
        if faces.is_empty() {
            return faces;
        }
        let iou_threshold = 0.4f32;
        let mut keep = Vec::new();
        for face in &faces {
            let dominated = keep
                .iter()
                .any(|kept: &(i32, i32, u32, u32)| Self::iou_int(*face, *kept) >= iou_threshold);
            if !dominated {
                keep.push(*face);
            }
        }
        log::info!("NMS: {} → {} faces", faces.len(), keep.len());
        keep
    }

    fn iou_int(a: (i32, i32, u32, u32), b: (i32, i32, u32, u32)) -> f32 {
        let (ax1, ay1, aw, ah) = (a.0 as f32, a.1 as f32, a.2 as f32, a.3 as f32);
        let (bx1, by1, bw, bh) = (b.0 as f32, b.1 as f32, b.2 as f32, b.3 as f32);
        Self::iou_float((ax1, ay1, aw, ah), (bx1, by1, bw, bh))
    }

    #[allow(dead_code)]
    fn nms_float(mut faces: Vec<(f32, f32, f32, f32, f32)>) -> Vec<(f32, f32, f32, f32, f32)> {
        if faces.is_empty() {
            return faces;
        }
        faces.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

        let iou_threshold = 0.4f32;
        let mut keep = Vec::new();

        for face in faces {
            let dominated = keep.iter().any(|kept: &(f32, f32, f32, f32, f32)| {
                Self::iou_float(
                    (face.0, face.1, face.2, face.3),
                    (kept.0, kept.1, kept.2, kept.3),
                ) >= iou_threshold
            });
            if !dominated {
                keep.push(face);
            }
        }
        keep
    }

    fn iou_float(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> f32 {
        let (ax1, ay1, aw, ah) = a;
        let (bx1, by1, bw, bh) = b;
        let ax2 = ax1 + aw;
        let ay2 = ay1 + ah;
        let bx2 = bx1 + bw;
        let by2 = by1 + bh;

        let ix1 = ax1.max(bx1);
        let iy1 = ay1.max(by1);
        let ix2 = ax2.min(bx2);
        let iy2 = ay2.min(by2);

        if ix2 < ix1 || iy2 < iy1 {
            return 0.0;
        }
        let intersection = (ix2 - ix1) * (iy2 - iy1);
        let union = aw * ah + bw * bh - intersection;
        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }
}

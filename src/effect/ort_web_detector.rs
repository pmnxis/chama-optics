/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Face detection using ONNX Runtime Web with WebGPU acceleration (WASM only).
//!
//! Preprocessing (image resize, RGB normalization, CHW layout) and postprocessing
//! (anchor decoding, NMS) run in Rust. Only ONNX inference is dispatched to
//! JavaScript via ONNX Runtime Web's WebGPU execution provider.
//!
//! Data flow per inference:
//!   Rust (preprocess) → JS (ORT WebGPU inference) → Rust (postprocess)

use js_sys::{Array, Float32Array, Promise, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::effect::face_detection::SpeedMode;

#[wasm_bindgen(module = "/js/ort_helper.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn init_ort_session(model_bytes: &[u8]) -> Result<Promise, JsValue>;

    #[wasm_bindgen(catch)]
    fn run_ort_inference(input_data: &[f32], height: u32, width: u32) -> Result<Promise, JsValue>;

    fn is_ort_ready() -> bool;

    fn get_ort_backend() -> String;
}

const WINDOW_SIZE: u32 = 640;
const OVERLAP_RATIO: f32 = 0.1;

/// Preprocess the ONNX model for WebGPU compatibility.
/// ORT Web's WebGPU backend doesn't support AveragePool with ceil_mode=1,
/// so we change it to ceil_mode=0 (floor mode). The output size difference
/// is at most 1 pixel, which doesn't affect face detection quality.
fn preprocess_model_for_webgpu(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use prost::Message;
    let mut model = candle_onnx::onnx::ModelProto::decode(bytes)
        .map_err(|e| format!("Failed to decode ONNX model: {}", e))?;

    if let Some(graph) = &mut model.graph {
        for node in &mut graph.node {
            if node.op_type == "AveragePool" {
                for attr in &mut node.attribute {
                    if attr.name == "ceil_mode" && attr.i == 1 {
                        attr.i = 0;
                        log::info!("Fixed AveragePool ceil_mode: 1 -> 0 for WebGPU compat");
                    }
                }
            }
        }
    }

    let mut buf = Vec::with_capacity(bytes.len());
    model
        .encode(&mut buf)
        .map_err(|e| format!("Failed to re-encode model: {}", e))?;
    Ok(buf)
}

/// Ensure the ORT session is initialized (loads model on first call).
pub async fn ensure_session() -> Result<String, String> {
    if is_ort_ready() {
        return Ok(get_ort_backend());
    }

    // Share model bytes with candle_face_detector (single include_bytes! in binary)
    let raw_bytes = crate::effect::candle_face_detector::model_bytes();
    log::info!(
        "Preprocessing ONNX model for WebGPU ({} bytes)",
        raw_bytes.len()
    );

    // Fix AveragePool ceil_mode for WebGPU compatibility
    let model_bytes = preprocess_model_for_webgpu(raw_bytes)?;
    log::info!(
        "Initializing ORT Web session ({} bytes model)",
        model_bytes.len()
    );

    let promise =
        init_ort_session(&model_bytes).map_err(|e| format!("JS call failed: {:?}", e))?;

    let result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("ORT session init failed: {:?}", e))?;

    let backend = result
        .as_string()
        .unwrap_or_else(|| "unknown".to_string());
    log::info!("ORT Web session ready, backend: {}", backend);
    Ok(backend)
}

/// Detect faces in a DynamicImage using ORT Web (WebGPU). Async.
pub async fn detect_faces(
    img: &image::DynamicImage,
    speed_mode: SpeedMode,
) -> Vec<(i32, i32, u32, u32)> {
    let img_width = img.width();
    let img_height = img.height();

    log::info!(
        "ORT Web: detecting on {}x{} image, speed_mode={}",
        img_width,
        img_height,
        speed_mode.as_str()
    );

    // Ensure session is ready
    if let Err(e) = ensure_session().await {
        log::error!("ORT Web session init failed: {}", e);
        return vec![];
    }

    // If image fits in window, just run single detection
    if img_width <= WINDOW_SIZE && img_height <= WINDOW_SIZE {
        return detect_single(img, 0, 0, img_width, img_height).await;
    }

    // Step 1: Always run on the whole image resized to 640x640
    let mut all_faces = detect_single(img, 0, 0, img_width, img_height).await;

    if speed_mode == SpeedMode::Fastest {
        log::info!("Fastest mode: whole image resized to 640x640 only");
        return all_faces;
    }

    // Step 2: Fast mode — sliding windows using min(w,h) as window size
    let window_size = img_width.min(img_height);
    let step = (window_size as f32 * (1.0 - OVERLAP_RATIO)) as i32;

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
        all_faces.extend(detect_single(&cropped, *wx, *wy, *ww, *wh).await);
    }

    if speed_mode == SpeedMode::Fast {
        return nms_final(all_faces);
    }

    // Step 3+: Deeper sliding windows (Normal/Slow/Slowest)
    let max_depth = speed_mode.max_depth();
    for depth in 0..max_depth as usize {
        let scale_factor = 1u32 << (max_depth as usize - depth - 1);
        let window_scaled = WINDOW_SIZE * scale_factor;

        log::info!(
            "Depth {}: window_size={}, scale={}",
            depth,
            window_scaled,
            scale_factor
        );

        let step = (window_scaled as f32 * (1.0 - OVERLAP_RATIO)) as i32;
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
                        all_faces.extend(
                            detect_single(&cropped, x, y, w as u32, h as u32).await,
                        );
                    }
                }
                y += step;
            }
            x += step;
        }
    }

    nms_final(all_faces)
}

/// Run detection on a single image region (resized to 640x640).
/// Returned coordinates are in the original image space.
async fn detect_single(
    img: &image::DynamicImage,
    offset_x: i32,
    offset_y: i32,
    region_w: u32,
    region_h: u32,
) -> Vec<(i32, i32, u32, u32)> {
    let input_data = preprocess_image(img);

    let detections = match run_inference_js(input_data).await {
        Ok(dets) => dets,
        Err(e) => {
            log::error!("ORT Web inference failed: {}", e);
            return vec![];
        }
    };

    // Scale to region coordinates, then offset to global
    let scale_x = region_w as f32 / WINDOW_SIZE as f32;
    let scale_y = region_h as f32 / WINDOW_SIZE as f32;

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

// ---- Preprocessing ----

fn preprocess_image(img: &image::DynamicImage) -> Vec<f32> {
    let target_size = WINDOW_SIZE as usize;
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

// ---- Inference via ONNX Runtime Web JS ----

async fn run_inference_js(
    input_data: Vec<f32>,
) -> Result<Vec<(f32, f32, f32, f32, f32)>, String> {
    let ws = WINDOW_SIZE;
    let promise = run_ort_inference(&input_data, ws, ws)
        .map_err(|e| format!("JS inference call failed: {:?}", e))?;

    let js_result = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("ORT inference failed: {:?}", e))?;

    // Parse output tensors from JS array
    let outputs = Array::from(&js_result);

    // Classify outputs by last dimension:
    //   scores: last_dim=1, bboxes: last_dim=4, kps: last_dim=10
    let mut scores_list: Vec<(String, Vec<f32>, usize)> = Vec::new();
    let mut bboxes_list: Vec<(String, Vec<f32>, usize)> = Vec::new();

    for i in 0..outputs.length() {
        let output = outputs.get(i);

        let name = Reflect::get(&output, &"name".into())
            .map_err(|_| "Missing output name".to_string())?
            .as_string()
            .ok_or("Invalid output name")?;

        let dims_js = Array::from(
            &Reflect::get(&output, &"dims".into())
                .map_err(|_| "Missing dims".to_string())?,
        );

        let data_js = Reflect::get(&output, &"data".into())
            .map_err(|_| "Missing data".to_string())?
            .dyn_into::<Float32Array>()
            .map_err(|_| "Invalid data type".to_string())?;

        let dims: Vec<usize> = (0..dims_js.length())
            .map(|j| dims_js.get(j).as_f64().unwrap_or(0.0) as usize)
            .collect();

        if dims.len() < 2 {
            continue;
        }

        let last_dim = dims[dims.len() - 1];
        let anchor_count = dims[dims.len() - 2];

        let mut data = vec![0f32; data_js.length() as usize];
        data_js.copy_to(&mut data);

        match last_dim {
            1 => scores_list.push((name, data, anchor_count)),
            4 => bboxes_list.push((name, data, anchor_count)),
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
        "ORT output mapping: scores=[{}], bboxes=[{}]",
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
        let scores = &scores_list[stride_idx].1;
        let bboxes = &bboxes_list[stride_idx].1;

        let stride = strides[stride_idx];
        let feat_size = feat_sizes[stride_idx];
        let anchors = generate_anchors(stride, feat_size);

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
            let (x, y, w, h) = decode_bbox(bbox_pred, anchors[i], stride as f32);
            all_detections.push((x, y, w, h, score));
        }
    }

    Ok(all_detections)
}

// ---- Anchor generation & bbox decoding ----

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

// ---- NMS ----

fn nms_final(faces: Vec<(i32, i32, u32, u32)>) -> Vec<(i32, i32, u32, u32)> {
    if faces.is_empty() {
        return faces;
    }
    let iou_threshold = 0.4f32;
    let mut keep = Vec::new();
    for face in &faces {
        let dominated = keep.iter().any(|kept: &(i32, i32, u32, u32)| {
            iou_int(*face, *kept) >= iou_threshold
        });
        if !dominated {
            keep.push(*face);
        }
    }
    log::info!("NMS: {} -> {} faces", faces.len(), keep.len());
    keep
}

fn iou_int(a: (i32, i32, u32, u32), b: (i32, i32, u32, u32)) -> f32 {
    let (ax1, ay1, aw, ah) = (a.0 as f32, a.1 as f32, a.2 as f32, a.3 as f32);
    let (bx1, by1, bw, bh) = (b.0 as f32, b.1 as f32, b.2 as f32, b.3 as f32);
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

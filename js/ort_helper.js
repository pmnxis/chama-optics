/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

// Bridge to ONNX Runtime Web for face detection inference via WebGPU.
// ort.webgpu.min.js must be loaded via <script> tag before this module.
//
// Data flow:
//   1. Model bytes (WASM → JS): ~16MB one-time copy at session init
//   2. Input tensor (WASM → JS): ~4.9MB per inference (1×3×640×640 × 4 bytes)
//   3. Output tensors (JS → WASM): ~1MB per inference (scores + bboxes)

let ortSession = null;
let ortBackend = 'none';

export async function init_ort_session(modelBytes) {
    if (typeof ort === 'undefined') {
        throw new Error(
            "ONNX Runtime Web not loaded. Ensure ort.webgpu.min.js is included in index.html."
        );
    }

    ort.env.wasm.wasmPaths =
        'https://cdn.jsdelivr.net/npm/onnxruntime-web@1.20.1/dist/';

    // Detect WebGPU support and choose execution providers
    let providers = ['wasm'];
    if (navigator.gpu) {
        try {
            const adapter = await navigator.gpu.requestAdapter();
            if (adapter) {
                providers = ['webgpu', 'wasm'];
            }
        } catch (e) {
            console.warn('WebGPU adapter request failed, falling back to WASM CPU:', e);
        }
    }

    console.log('[ORT] Creating session with providers:', providers);

    // Copy model bytes from WASM memory before async operations
    // (the WASM memory view may be invalidated during async ops)
    const modelCopy = new Uint8Array(modelBytes);

    ortSession = await ort.InferenceSession.create(modelCopy.buffer, {
        executionProviders: providers,
    });

    ortBackend = providers[0];

    console.log('[ORT] Session created. Backend:', ortBackend);
    console.log('[ORT] Input names:', ortSession.inputNames);
    console.log('[ORT] Output names:', ortSession.outputNames);

    return ortBackend;
}

export async function run_ort_inference(inputData, height, width) {
    if (!ortSession) {
        throw new Error("ORT session not initialized. Call init_ort_session first.");
    }

    // Copy input data from WASM memory immediately
    // (Float32Array view into WASM linear memory may be invalidated during async ops)
    const inputCopy = new Float32Array(inputData);

    const inputTensor = new ort.Tensor('float32', inputCopy, [1, 3, height, width]);
    const feeds = {};
    feeds[ortSession.inputNames[0]] = inputTensor;

    const results = await ortSession.run(feeds);

    // Collect all outputs: array of { name, dims, data }
    const outputs = [];
    for (const name of ortSession.outputNames) {
        const tensor = results[name];
        // getData() downloads from GPU to CPU if tensor is on GPU
        const data = await tensor.getData();
        outputs.push({
            name: name,
            dims: Array.from(tensor.dims),
            data: new Float32Array(data),
        });
    }

    return outputs;
}

export function is_ort_ready() {
    return ortSession !== null;
}

export function get_ort_backend() {
    return ortBackend;
}

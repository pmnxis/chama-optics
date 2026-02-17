/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

// Bridge to libheif-js CDN for HEIF/HEIC decoding in browser.
// libheif-bundle.js must be loaded via <script> tag before this module.
//
// Overhead note: WASM<->JS boundary involves data copies:
//   1. HEIF bytes from WASM linear memory -> JS (input)
//   2. Decoded RGBA pixels from JS -> WASM linear memory (output)
// For a 24MP image this means ~10-30MB in + ~96MB out as temporary copies.

let libheifInstance = null;

export async function decode_heif_from_bytes(data) {
    if (!libheifInstance) {
        if (typeof libheif === 'undefined') {
            throw new Error("libheif-js not loaded. Ensure libheif-bundle.js script is included in index.html.");
        }
        libheifInstance = await libheif();
    }

    const decoder = new libheifInstance.HeifDecoder();
    const images = decoder.decode(data);
    if (!images || images.length === 0) {
        throw new Error("No images found in HEIF data");
    }

    const image = images[0];
    const width = image.get_width();
    const height = image.get_height();

    const canvas = new OffscreenCanvas(width, height);
    const ctx = canvas.getContext('2d');
    const imageData = ctx.createImageData(width, height);

    await new Promise((resolve, reject) => {
        image.display(imageData, (result) => {
            if (!result) return reject(new Error("HEIF display failed"));
            resolve();
        });
    });

    // Return RGBA data directly — single Uint8Array for minimal JS->WASM copy
    return { width, height, data: new Uint8Array(imageData.data.buffer) };
}

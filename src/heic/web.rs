/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Not use anymore

use egui::{ColorImage, TextureOptions};
use js_sys::{Promise, Reflect, Uint8Array};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::{with_app_instance, with_ctx_instance};

#[wasm_bindgen(module = "/js/heif_helper.js")]
extern "C" {
    fn decode_heif_from_bytes(data: &[u8]) -> Promise;
}

pub(crate) fn load_heif_from_bytes(
    buffer: &[u8],
    name: String, // image name
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = buffer.to_vec();

    spawn_local(async move {
        let js_result = wasm_bindgen_futures::JsFuture::from(decode_heif_from_bytes(&bytes)).await;

        let result: Result<(), Box<dyn std::error::Error>> = match js_result {
            Ok(js_value) => {
                let width: Result<usize, Box<dyn std::error::Error>> =
                    match Reflect::get(&js_value, &"width".into()) {
                        Ok(v) => v
                            .as_f64()
                            .map(|f| f as usize)
                            .ok_or_else(|| "Invalid width".into()),
                        Err(_) => Err("Missing width".into()),
                    };

                let height: Result<usize, Box<dyn std::error::Error>> =
                    match Reflect::get(&js_value, &"height".into()) {
                        Ok(v) => v
                            .as_f64()
                            .map(|f| f as usize)
                            .ok_or_else(|| "Invalid height".into()),
                        Err(_) => Err("Missing height".into()),
                    };

                let data: Result<Uint8Array, Box<dyn std::error::Error>> =
                    match Reflect::get(&js_value, &"data".into()) {
                        Ok(v) => v
                            .dyn_into::<Uint8Array>()
                            .map_err(|_| "Invalid buffer".into()),
                        Err(_) => Err("Missing data".into()),
                    };

                match (width, height, data) {
                    (Ok(w), Ok(h), Ok(buf)) => {
                        let mut rgba = vec![0u8; buf.length() as usize];
                        buf.copy_to(&mut rgba);

                        if rgba.len() != w * h * 4 {
                            Err("Unexpected buffer size".into())
                        } else {
                            // RGBA -> RGB
                            let rgb: Vec<u8> = rgba
                                .chunks_exact(4)
                                .flat_map(|chunk| &chunk[0..3])
                                .copied()
                                .collect();

                            let color_image = ColorImage::from_rgb([w, h], &rgb);

                            with_app_instance(|app| {
                                with_ctx_instance(|ctx| {
                                    let texture = ctx.load_texture(
                                        name.clone(),
                                        color_image,
                                        TextureOptions::NEAREST,
                                    );
                                    app.borrow_mut().images.push((name.clone(), texture));
                                    ctx.request_repaint();
                                });
                            });

                            Ok(())
                        }
                    }
                    _ => Err("Failed to parse JS values".into()),
                }
            }
            Err(e) => Err(format!("JS Error: {:?}", e).into()),
        };

        if let Err(e) = result {
            log::error!("HEIC load failed: {:?}", e);
        }
    });

    Ok(())
}

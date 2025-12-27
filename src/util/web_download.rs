/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Web browser file download utilities for WASM

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Download a file in the browser
#[cfg(target_arch = "wasm32")]
pub fn download_file(filename: &str, data: &[u8], mime_type: &str) -> Result<(), String> {
    use wasm_bindgen::JsValue;

    let window = web_sys::window().ok_or("No window object")?;
    let document = window.document().ok_or("No document object")?;

    // Create a Blob from the data
    let array = js_sys::Uint8Array::new_with_length(data.len() as u32);
    array.copy_from(data);

    let blob_parts = js_sys::Array::new();
    blob_parts.push(&array);

    let mut blob_options = web_sys::BlobPropertyBag::new();
    blob_options.type_(mime_type);

    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &blob_options)
        .map_err(|e| format!("Failed to create blob: {:?}", e))?;

    // Create object URL
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create object URL: {:?}", e))?;

    // Create download link
    let a = document
        .create_element("a")
        .map_err(|e| format!("Failed to create element: {:?}", e))?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|e| format!("Failed to cast to anchor: {:?}", e))?;

    a.set_href(&url);
    a.set_download(filename);
    a.style().set_property("display", "none").ok();

    // Append to body, click, and remove
    let body = document.body().ok_or("No body element")?;
    body.append_child(&a)
        .map_err(|e| format!("Failed to append child: {:?}", e))?;

    a.click();

    body.remove_child(&a)
        .map_err(|e| format!("Failed to remove child: {:?}", e))?;

    // Revoke object URL to free memory
    web_sys::Url::revoke_object_url(&url).map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}

/// Desktop version - not used in WASM
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn download_file(_filename: &str, _data: &[u8], _mime_type: &str) -> Result<(), String> {
    Err("download_file is only available in WASM".to_string())
}

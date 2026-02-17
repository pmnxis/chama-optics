/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Browser download helpers for WASM.
//! Creates Blob → Object URL → hidden <a download> → click → revoke.

use wasm_bindgen::JsCast;

/// Trigger a browser file download from raw bytes.
pub fn download_file(filename: &str, data: &[u8], mime_type: &str) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => {
            log::error!("No window object available");
            return;
        }
    };

    let array = js_sys::Uint8Array::from(data);
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&array.buffer());

    let mut options = web_sys::BlobPropertyBag::new();
    options.type_(mime_type);

    let blob = match web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options) {
        Ok(b) => b,
        Err(e) => {
            log::error!("Failed to create Blob: {:?}", e);
            return;
        }
    };

    let url = match web_sys::Url::create_object_url_with_blob(&blob) {
        Ok(u) => u,
        Err(e) => {
            log::error!("Failed to create object URL: {:?}", e);
            return;
        }
    };

    let document = window.document().unwrap();
    let anchor = document
        .create_element("a")
        .unwrap()
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .unwrap();

    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.style().set_property("display", "none").ok();

    let body = document.body().unwrap();
    body.append_child(&anchor).ok();
    anchor.click();
    body.remove_child(&anchor).ok();

    // Revoke after a short delay to ensure download starts
    let url_clone = url.clone();
    let closure = wasm_bindgen::closure::Closure::once(move || {
        web_sys::Url::revoke_object_url(&url_clone).ok();
    });
    window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            5000,
        )
        .ok();
    closure.forget();
}

/// Create a zip archive in memory from a list of (filename, bytes) entries,
/// then trigger a browser download.
pub fn download_zip(zip_filename: &str, entries: &[(&str, &[u8])]) {
    use std::io::Write;

    let buf = std::io::Cursor::new(Vec::new());
    let mut zip_writer = zip::ZipWriter::new(buf);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (name, data) in entries {
        if let Err(e) = zip_writer.start_file(*name, options) {
            log::error!("Failed to start zip entry '{}': {:?}", name, e);
            continue;
        }
        if let Err(e) = zip_writer.write_all(data) {
            log::error!("Failed to write zip entry '{}': {:?}", name, e);
        }
    }

    let zip_data = match zip_writer.finish() {
        Ok(cursor) => cursor.into_inner(),
        Err(e) => {
            log::error!("Failed to finalize zip: {:?}", e);
            return;
        }
    };

    log::info!(
        "Zip created: {} ({} bytes, {} entries)",
        zip_filename,
        zip_data.len(),
        entries.len()
    );

    download_file(zip_filename, &zip_data, "application/zip");
}

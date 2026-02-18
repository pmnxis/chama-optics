/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Browser download helpers for WASM.
//! Creates Blob → Object URL → hidden <a download> → click → revoke.

use wasm_bindgen::JsCast;

/// Trigger a browser file download from raw bytes.
/// Returns `Err` with a description if any step fails.
pub fn download_file(filename: &str, data: &[u8], mime_type: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window object available")?;
    let document = window.document().ok_or("No document available")?;
    let body = document.body().ok_or("No body element available")?;

    let array = js_sys::Uint8Array::from(data);
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&array.buffer());

    let mut options = web_sys::BlobPropertyBag::new();
    options.type_(mime_type);

    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)
        .map_err(|e| format!("Failed to create Blob: {:?}", e))?;

    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create object URL: {:?}", e))?;

    let anchor: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| format!("Failed to create anchor element: {:?}", e))?
        .dyn_into()
        .map_err(|_| "Created element is not an anchor".to_string())?;

    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.style().set_property("display", "none").ok();

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

    Ok(())
}

/// Create a zip archive in memory from a list of (filename, bytes) entries,
/// then trigger a browser download.
pub fn download_zip(zip_filename: &str, entries: &[(&str, &[u8])]) -> Result<(), String> {
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
            return Err(format!("Failed to finalize zip: {:?}", e));
        }
    };

    log::info!(
        "Zip created: {} ({} bytes, {} entries)",
        zip_filename,
        zip_data.len(),
        entries.len()
    );

    download_file(zip_filename, &zip_data, "application/zip")
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

//! Web/WASM file input helpers using web-sys

use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

/// A pending file read result: (filename, bytes)
pub type FileData = (String, Vec<u8>);

/// Shared queue for file data from web file input
/// Uses Arc<Mutex> for compatibility with egui's frame-based polling
pub type PendingFileQueue = Arc<std::sync::Mutex<Vec<FileData>>>;

/// Open a multi-file picker dialog (browser native <input type="file" multiple>).
/// Selected files are asynchronously read via FileReader and pushed to the queue.
pub fn pick_files_to_queue(queue: PendingFileQueue) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let input = document
        .create_element("input")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    input.set_type("file");
    input.set_attribute("multiple", "").unwrap();
    input
        .set_attribute("accept", "image/*,.heic,.heif")
        .unwrap();

    let queue_ref = queue.clone();

    let onchange = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let input: web_sys::HtmlInputElement = event.target().unwrap().dyn_into().unwrap();

        if let Some(files) = input.files() {
            for i in 0..files.length() {
                if let Some(file) = files.get(i) {
                    let filename = file.name();
                    let queue_inner = queue_ref.clone();

                    let reader = web_sys::FileReader::new().unwrap();
                    let reader_clone = reader.clone();

                    let onloadend = Closure::once(Box::new(move |_e: web_sys::Event| {
                        if let Ok(result) = reader_clone.result() {
                            let array = js_sys::Uint8Array::new(&result);
                            let bytes = array.to_vec();
                            log::info!("Web file loaded: {} ({} bytes)", filename, bytes.len());
                            if let Ok(mut q) = queue_inner.lock() {
                                q.push((filename, bytes));
                            }
                        }
                    }) as Box<dyn FnOnce(_)>);

                    reader.set_onloadend(Some(onloadend.as_ref().unchecked_ref()));
                    reader.read_as_array_buffer(&file).unwrap();
                    onloadend.forget();
                }
            }
        }
    }) as Box<dyn FnMut(_)>);

    input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
    input.click();
    onchange.forget();
}

/// Process dropped file bytes from egui's DroppedFile into (name, bytes) tuples
pub fn extract_dropped_file_bytes(dropped_files: &[egui::DroppedFile]) -> Vec<FileData> {
    dropped_files
        .iter()
        .filter_map(|f| {
            let name = f.name.clone();
            f.bytes.as_ref().map(|bytes| (name, bytes.to_vec()))
        })
        .collect()
}

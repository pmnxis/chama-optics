/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

// Some helpers are only called in desktop-only code paths but are still compiled
// when `--all-features` also enables `ios_integration`.
#![allow(dead_code)]

//! Async file dialog helper — wraps `rfd::AsyncFileDialog` to avoid winit re-entrant panics on macOS.
//!
//! On macOS, `rfd::FileDialog` (sync) uses `NSSavePanel::runModal` which blocks the main thread
//! in a nested run loop. If the user drag-drops inside the dialog, macOS dispatches a drag event
//! to the underlying winit window, hitting the re-entrant event handler guard → panic → abort.
//!
//! These helpers spawn a background thread that drives the async dialog future, sending the result
//! through an `mpsc` channel. The UI thread polls via `try_recv()` each frame.

use std::path::PathBuf;
use std::sync::{Mutex, mpsc};

/// A pending file dialog result.
///
/// Wraps an `mpsc::Receiver` (inside a `Mutex` for `Sync` safety) that will
/// eventually deliver the dialog result from a background thread.
/// This type intentionally does NOT implement `Clone`, `Serialize`,
/// `Deserialize`, `PartialEq`, etc.
/// Fields containing this type must be `#[serde(skip)]`.
pub struct PendingDialog<T> {
    rx: Mutex<mpsc::Receiver<T>>,
}

impl<T> PendingDialog<T> {
    /// Non-blocking poll. Returns `Some(result)` if the dialog has completed,
    /// `None` if still pending or the sender was dropped.
    pub fn try_recv(&self) -> Option<T> {
        self.rx.lock().ok()?.try_recv().ok()
    }
}

/// Minimal single-future executor for background threads.
/// Uses `Waker::noop()` (stable since Rust 1.85) + `thread::yield_now()`.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::task::{Context, Poll, Waker};

    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);

    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(result) => return result,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

/// Spawn an async folder picker dialog.
pub fn pick_folder_async() -> PendingDialog<Option<PathBuf>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = block_on(rfd::AsyncFileDialog::new().pick_folder());
        let _ = tx.send(result.map(|h| h.path().to_path_buf()));
    });
    PendingDialog { rx: Mutex::new(rx) }
}

/// Spawn an async multi-file picker dialog.
pub fn pick_files_async() -> PendingDialog<Option<Vec<PathBuf>>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = block_on(rfd::AsyncFileDialog::new().pick_files());
        let paths = result.map(|handles| handles.iter().map(|h| h.path().to_path_buf()).collect());
        let _ = tx.send(paths);
    });
    PendingDialog { rx: Mutex::new(rx) }
}

/// Spawn an async single-file picker dialog with optional filter.
pub fn pick_file_async(
    filter_name: Option<&str>,
    extensions: Option<&[&str]>,
) -> PendingDialog<Option<PathBuf>> {
    let filter_name = filter_name.map(|s| s.to_owned());
    let extensions: Option<Vec<String>> =
        extensions.map(|exts| exts.iter().map(|s| s.to_string()).collect());

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut dialog = rfd::AsyncFileDialog::new();
        if let (Some(name), Some(exts)) = (&filter_name, &extensions) {
            let ext_refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
            dialog = dialog.add_filter(name, &ext_refs);
        }
        let result = block_on(dialog.pick_file());
        let _ = tx.send(result.map(|h| h.path().to_path_buf()));
    });
    PendingDialog { rx: Mutex::new(rx) }
}

/// Spawn an async single-file picker dialog with title and filter.
pub fn pick_file_with_title_async(
    title: &str,
    filter_name: &str,
    extensions: &[&str],
) -> PendingDialog<Option<PathBuf>> {
    let title = title.to_owned();
    let filter_name = filter_name.to_owned();
    let extensions: Vec<String> = extensions.iter().map(|s| s.to_string()).collect();

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
        let dialog = rfd::AsyncFileDialog::new()
            .add_filter(&filter_name, &ext_refs)
            .set_title(&title);
        let result = block_on(dialog.pick_file());
        let _ = tx.send(result.map(|h| h.path().to_path_buf()));
    });
    PendingDialog { rx: Mutex::new(rx) }
}

/// Spawn an async save-file dialog with a suggested filename.
pub fn save_file_async(default_file_name: &str) -> PendingDialog<Option<PathBuf>> {
    let default_file_name = default_file_name.to_owned();

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let dialog = rfd::AsyncFileDialog::new().set_file_name(&default_file_name);
        let result = block_on(dialog.save_file());
        let _ = tx.send(result.map(|h| h.path().to_path_buf()));
    });
    PendingDialog { rx: Mutex::new(rx) }
}

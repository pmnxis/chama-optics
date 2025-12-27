/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Parallel image loading system

use crate::exif_impl::{OriginalExif, SimplifiedExif};
use crate::packed_image::PackedImage;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

/// Intermediate loaded image data (before texture creation)
/// Note: We don't store OriginalExif here because exif::Exif is not Clone
/// Instead, we re-parse it in the UI thread if needed
pub struct LoadedImageData {
    pub path: PathBuf,
    pub view_exif: SimplifiedExif,
    pub thumbnail: Option<egui::ColorImage>, // Pre-generated thumbnail
    pub orientation: image::metadata::Orientation,

    /// Store original image bytes for non-desktop platforms (WASM, iOS)
    #[cfg(not(feature = "desktop"))]
    pub image_bytes: Option<Vec<u8>>,
}

/// Shared queue for loaded images waiting for texture creation
pub type LoadedImageQueue = Arc<Mutex<Vec<LoadedImageData>>>;

/// Load a single image in a background thread (EXIF + image data only)
/// Returns LoadedImageData that can be used to create PackedImage in UI thread
pub fn load_image_data(
    path: &PathBuf,
    get_alt_fnumber: bool,
) -> Result<LoadedImageData, image::ImageError> {
    use std::io::Seek;

    let file = std::fs::File::open(path)?;
    let mut buf_reader = std::io::BufReader::new(file);

    // Parse EXIF
    let original_exif = OriginalExif::new(
        match exif::Reader::new().read_from_container(&mut buf_reader) {
            Ok(exif) => Some(exif),
            Err(e) => {
                log::error!("Failed to parse EXIF from {:?}: {e:?}", path);
                None
            }
        },
    );

    let mut view_exif = SimplifiedExif::from(&original_exif);
    if get_alt_fnumber {
        view_exif.replace_with_fnumber_alt_when_invalid();
    }

    buf_reader
        .seek(std::io::SeekFrom::Start(0))
        .expect("Failed to reset seek");

    // Load image data for thumbnail
    let (dyn_image, need_orientation) = crate::image::common::__load_image(path, &mut buf_reader)?;

    let orientation = if need_orientation {
        original_exif.orientation()
    } else {
        image::metadata::Orientation::NoTransforms
    };

    // Generate thumbnail immediately (CPU-bound, can be parallelized)
    let thumbnail = crate::image::common::gen_thumbnail(dyn_image, orientation)?;

    Ok(LoadedImageData {
        path: path.clone(),
        view_exif,
        thumbnail: Some(thumbnail),
        orientation,
        #[cfg(not(feature = "desktop"))]
        image_bytes: None, // Desktop doesn't need to store bytes
    })
}

/// Load image from memory buffer (for WASM file picker)
#[cfg(target_arch = "wasm32")]
pub fn load_image_from_memory(
    bytes: &[u8],
    filename: &str,
    get_alt_fnumber: bool,
) -> Result<LoadedImageData, image::ImageError> {
    use std::io::{Cursor, Seek};

    let mut cursor = Cursor::new(bytes);

    // Parse EXIF
    let original_exif =
        OriginalExif::new(match exif::Reader::new().read_from_container(&mut cursor) {
            Ok(exif) => Some(exif),
            Err(e) => {
                log::error!("Failed to parse EXIF from {}: {e:?}", filename);
                None
            }
        });

    let mut view_exif = SimplifiedExif::from(&original_exif);
    if get_alt_fnumber {
        view_exif.replace_with_fnumber_alt_when_invalid();
    }

    cursor
        .seek(std::io::SeekFrom::Start(0))
        .expect("Failed to reset cursor");

    // Create a pseudo-path for web
    let pseudo_path = PathBuf::from(format!("web://{}", filename));

    // Load image data from memory
    let dyn_image = image::load_from_memory(bytes)?;

    // WASM doesn't need orientation from file format
    let need_orientation = true;

    let orientation = if need_orientation {
        original_exif.orientation()
    } else {
        image::metadata::Orientation::NoTransforms
    };

    // Generate thumbnail immediately
    let thumbnail = crate::image::common::gen_thumbnail(dyn_image, orientation)?;

    Ok(LoadedImageData {
        path: pseudo_path,
        view_exif,
        thumbnail: Some(thumbnail),
        orientation,
        #[cfg(not(feature = "desktop"))]
        image_bytes: Some(bytes.to_vec()),
    })
}

/// Spawn a background thread pool to load images in parallel
/// Images are loaded in order and placed in the queue
pub fn spawn_parallel_loader(
    paths: Vec<PathBuf>,
    get_alt_fnumber: bool,
    progress_counter: Arc<AtomicUsize>,
    result_queue: LoadedImageQueue,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        use rayon::prelude::*;
        use std::sync::atomic::Ordering;

        // Note: Single-threaded loading (1 thread) performs better than multi-core for now
        log::info!("Starting image loading: {}", paths.len(),);

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();

        // Process in parallel and push to queue immediately upon completion
        pool.install(|| {
            paths.par_iter().for_each(|path| {
                match load_image_data(path, get_alt_fnumber) {
                    Ok(loaded_data) => {
                        // Push to queue immediately
                        if let Ok(mut queue) = result_queue.lock() {
                            queue.push(loaded_data);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to load image {:?} - {e:?}", path);
                    }
                }

                // Update progress and request repaint
                progress_counter.fetch_add(1, Ordering::Relaxed);
                ctx.request_repaint();
            })
        });
    });
}

/// Convert LoadedImageData to PackedImage (must be called in UI thread for texture creation)
pub fn create_packed_image_from_data(
    data: LoadedImageData,
    ctx: &egui::Context,
) -> Option<PackedImage> {
    #[cfg(not(target_arch = "wasm32"))]
    let src_exif = {
        // Re-parse EXIF in UI thread (since OriginalExif is not Clone)
        let file = std::fs::File::open(&data.path).ok()?;
        let mut buf_reader = std::io::BufReader::new(file);

        OriginalExif::new(
            match exif::Reader::new().read_from_container(&mut buf_reader) {
                Ok(exif) => Some(exif),
                Err(e) => {
                    log::warn!("Failed to re-parse EXIF for {:?}: {e:?}", data.path);
                    None
                }
            },
        )
    };

    #[cfg(target_arch = "wasm32")]
    let src_exif = {
        // WASM: Cannot re-read file, use empty EXIF
        // The view_exif already has all the data we need
        OriginalExif::new(None)
    };

    // Use pre-generated thumbnail
    if let Some(thumbnail) = data.thumbnail {
        let file_name = data
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let texture = crate::image::common::PackedTexture::new(ctx.load_texture(
            file_name,
            thumbnail,
            egui::TextureOptions::NEAREST,
        ));

        Some(PackedImage {
            path: data.path,
            src_exif,
            view_exif: data.view_exif,
            editable: false,
            texture,
            #[cfg(not(feature = "desktop"))]
            image_bytes: data.image_bytes,
        })
    } else {
        log::error!("No thumbnail for {:?}", data.path);
        None
    }
}

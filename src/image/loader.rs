/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Parallel image loading system

use crate::exif_impl::{OriginalExif, SimplifiedExif};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
use crate::packed_image::PackedImage;

/// Intermediate loaded image data (before texture creation)
/// Note: We don't store OriginalExif here because exif::Exif is not Clone
/// Instead, we re-parse it in UI thread if needed
pub struct LoadedImageData {
    pub path: PathBuf,
    pub view_exif: SimplifiedExif,
    pub thumbnail: Option<egui::ColorImage>, // Pre-generated thumbnail
    pub orientation: image::metadata::Orientation,

    /// Store original image bytes for non-desktop platforms (WASM, iOS)
    #[cfg(not(feature = "desktop"))]
    pub image_bytes: Option<Vec<u8>>,

    /// Perceptual hash calculated from thumbnail
    pub perceptual_hash: Option<u64>,
}

/// Shared queue for loaded images waiting for texture creation
pub type LoadedImageQueue = Arc<Mutex<Vec<LoadedImageData>>>;

/// Calculate perceptual hash from ColorImage (8x8 average hash)
pub(crate) fn calculate_perceptual_hash_from_thumbnail(thumbnail: &egui::ColorImage) -> Option<u64> {
    // Convert ColorImage to grayscale 8x8
    let width = thumbnail.size[0];
    let height = thumbnail.size[1];

    if width == 0 || height == 0 {
        return None;
    }

    // Resize to 8x8 using simple nearest neighbor
    let mut gray_8x8 = [0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            let src_x = (x * width) / 8;
            let src_y = (y * height) / 8;
            let pixel_idx = src_y * width + src_x;

            if pixel_idx < thumbnail.pixels.len() {
                let pixel = thumbnail.pixels[pixel_idx];
                // Convert RGBA to grayscale: 0.299*R + 0.587*G + 0.114*B
                let gray = ((pixel.r() as f32 * 0.299
                    + pixel.g() as f32 * 0.587
                    + pixel.b() as f32 * 0.114)
                    * 255.0) as u8;
                gray_8x8[y * 8 + x] = gray;
            }
        }
    }

    // Calculate average
    let sum: u32 = gray_8x8.iter().map(|&v| v as u32).sum();
    let avg = (sum / 64) as u8;

    // Create hash: 1 if pixel > average, 0 otherwise
    let mut hash: u64 = 0;
    for (i, &pixel) in gray_8x8.iter().enumerate() {
        if pixel > avg {
            hash |= 1 << i;
        }
    }

    Some(hash)
}

/// Load a single image in a background thread (EXIF + image data only)
/// Returns LoadedImageData that can be used to create PackedImage in UI thread
pub fn load_image_data(
    path: &PathBuf,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
    simplify_lens_model: bool,
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
    if use_35mm_focal_length {
        view_exif.use_35mm_focal_length(&original_exif);
    }
    if simplify_lens_model {
        view_exif.apply_simplify_lens_model();
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

    // Calculate perceptual hash from thumbnail for efficient grouping
    let perceptual_hash = calculate_perceptual_hash_from_thumbnail(&thumbnail);

    Ok(LoadedImageData {
        path: path.clone(),
        view_exif,
        thumbnail: Some(thumbnail),
        orientation,
        #[cfg(not(feature = "desktop"))]
        image_bytes: None, // Desktop doesn't need to store bytes
        perceptual_hash,
    })
}

// left this code for future even not support wasm
/// Load image from memory buffer (for WASM file picker)
#[cfg(target_arch = "wasm32")]
pub fn load_image_from_memory(
    bytes: &[u8],
    filename: &str,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
    simplify_lens_model: bool,
) -> Result<LoadedImageData, image::ImageError> {
    use std::io::{Cursor, Seek};

    let mut cursor = Cursor::new(bytes);

    // Parse EXIF
    let original_exif: OriginalExif =
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
    if use_35mm_focal_length {
        view_exif.use_35mm_focal_length(&original_exif);
    }
    if simplify_lens_model {
        view_exif.apply_simplify_lens_model();
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

    // Calculate perceptual hash from thumbnail for efficient grouping
    let perceptual_hash = calculate_perceptual_hash_from_thumbnail(&thumbnail);

    Ok(LoadedImageData {
        path: pseudo_path,
        view_exif,
        thumbnail: Some(thumbnail),
        orientation,
        #[cfg(not(feature = "desktop"))]
        image_bytes: Some(bytes.to_vec()),
        perceptual_hash,
    })
}

/// Spawn a background thread pool to load images in parallel
/// Images are loaded in order and placed in queue
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_parallel_loader(
    paths: Vec<PathBuf>,
    get_alt_fnumber: bool,
    use_35mm_focal_length: bool,
    simplify_lens_model: bool,
    progress_counter: Arc<AtomicUsize>,
    result_queue: LoadedImageQueue,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        use std::sync::atomic::Ordering;

        log::info!("Starting image loading: {}", paths.len());

        // Sequential loading (single-threaded is actually faster for I/O-bound work)
        for path in &paths {
            match load_image_data(path, get_alt_fnumber, use_35mm_focal_length, simplify_lens_model)
            {
                Ok(loaded_data) => {
                    if let Ok(mut queue) = result_queue.lock() {
                        queue.push(loaded_data);
                    }
                }
                Err(e) => {
                    log::error!("Failed to load image {:?} - {e:?}", path);
                }
            }

            progress_counter.fetch_add(1, Ordering::Relaxed);
            ctx.request_repaint();
        }
    });
}

/// Convert LoadedImageData to PackedImage (must be called in UI thread for texture creation)
#[cfg(not(any(feature = "ios_integration", feature = "android_integration")))]
pub fn create_packed_image_from_data(
    data: LoadedImageData,
    ctx: &egui::Context,
) -> Option<PackedImage> {
    // Extract data first to avoid scoping issues
    let view_exif = &data.view_exif;
    let perceptual_hash = &data.perceptual_hash;

    // Use pre-generated thumbnail if available
    if let Some(thumbnail) = &data.thumbnail {
        let file_name = data
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // thumbnail is already egui::ColorImage, use it directly
        let texture = crate::image::common::PackedTexture::new(ctx.load_texture(
            file_name,
            thumbnail.clone(),
            egui::TextureOptions::NEAREST,
        ));

        // src_exif is not available in LoadedImageData, create empty OriginalExif
        let src_exif = crate::exif_impl::OriginalExif::new(None);

        Some(PackedImage {
            uuid: uuid::Uuid::new_v4(),
            path: data.path.clone(),
            src_exif,
            view_exif: view_exif.clone(),
            editable: false,
            texture,
            #[cfg(not(feature = "desktop"))]
            image_bytes: data.image_bytes,
            sticker_bytes: None, // No sticker data from loader
            perceptual_hash: *perceptual_hash,
            configured_faces: Vec::new(), // No faces configured from loader
            lut_id: None,                 // No LUT configured from loader
            crop_rotate: crate::effect::crop_rotate::CropRotateTransform::default(),
            #[cfg(feature = "rfd")]
            pending_save: None,
        })
    } else {
        log::error!("No thumbnail for {:?}", data.path);
        None
    }
}

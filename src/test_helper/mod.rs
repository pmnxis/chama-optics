/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

pub fn list_import_images_path() -> std::io::Result<Vec<std::path::PathBuf>> {
    let import_dir = std::path::Path::new("test_image/import");

    if !import_dir.exists() {
        return Ok(Vec::new());
    }

    let exts = ["jpg", "jpeg", "png", "hif"];

    let mut images = Vec::new();

    for entry in std::fs::read_dir(import_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if exts.contains(&ext.to_lowercase().as_str()) {
                    images.push(path);
                }
            }
        }
    }

    Ok(images)
}

pub fn list_import_packed_images() -> Vec<crate::packed_image::PackedImage> {
    let path_list = list_import_images_path().unwrap();
    path_list
        .iter()
        .filter_map(
            |pb| match crate::packed_image::PackedImage::try_from_path_cli(pb) {
                Ok(img) => Some(img),
                Err(err) => {
                    eprintln!("Failed to load test image {pb:?}: {err}");
                    None
                }
            },
        )
        .collect()
}

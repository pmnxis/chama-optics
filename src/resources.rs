/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! Unified resource management for fonts, models, and logos
//!
//! When `ext_res` feature is enabled, loads from Resources/ directory
//! Otherwise, uses embedded resources (include_bytes!)

#[cfg(feature = "ext_res")]
use std::path::PathBuf;

// Embedded font data (const statics - compiled into binary when ext_res is disabled)
// Not used for WASM — fonts are loaded at runtime from HTTP (like ext_res)
#[cfg(all(not(feature = "ext_res"), not(target_arch = "wasm32")))]
const FONT_D2CODING: &[u8] = include_bytes!("../assets/fonts/D2Coding-Ver1.3.2-20180524-all.ttc");
#[cfg(all(not(feature = "ext_res"), not(target_arch = "wasm32")))]
const FONT_SOURCE_HAN_SANS: &[u8] = include_bytes!("../assets/fonts/SourceHanSansVF-remapped.otf");
#[cfg(all(not(feature = "ext_res"), not(target_arch = "wasm32")))]
const FONT_BARLOW: &[u8] = include_bytes!("../assets/fonts/Barlow-Variable-Remapped.ttf");
#[cfg(all(not(feature = "ext_res"), not(target_arch = "wasm32")))]
const FONT_BARLOW_NARROW: &[u8] =
    include_bytes!("../assets/fonts/Barlow-Variable-Remapped-Narrow.ttf");
#[cfg(all(not(feature = "ext_res"), feature = "desktop"))]
const FONT_DYNAPUFF: &[u8] = include_bytes!(env!("DYNAPUFF_FONT_PATH"));

// Embedded model data (const static - compiled into binary when ext_res is disabled)
#[cfg(all(not(feature = "ext_res"), has_insightface_model))]
const MODEL_INSIGHTFACE: &[u8] = include_bytes!(env!("INSIGHTFACE_MODEL_PATH"));

/// Get the Resources directory path for the current platform
/// Searches multiple locations to support both bundled apps and cargo run
#[cfg(feature = "ext_res")]
fn get_resources_dir() -> Option<PathBuf> {
    use std::env;

    #[cfg(target_os = "macos")]
    {
        let exe_path = env::current_exe().ok()?;

        // Try 1: App bundle location (Contents/MacOS/../Resources)
        let bundle_resources = exe_path
            .parent()? // Remove executable name
            .parent()? // Contents/MacOS -> Contents
            .join("Resources");

        if bundle_resources.exists() {
            log::debug!("Found Resources in app bundle: {:?}", bundle_resources);
            return Some(bundle_resources);
        }

        // Try 2: Development location (target/debug/Resources or target/release/Resources)
        if let Some(parent) = exe_path.parent() {
            let dev_resources = parent.join("Resources");
            if dev_resources.exists() {
                log::debug!("Found Resources in dev location: {:?}", dev_resources);
                return Some(dev_resources);
            }
        }

        log::debug!("Resources directory not found in any location");
        None
    }

    #[cfg(target_os = "linux")]
    {
        let exe_path = env::current_exe().ok()?;

        // Try 1: Next to executable (for installed apps)
        let app_resources = exe_path.parent()?.join("Resources");
        if app_resources.exists() {
            return Some(app_resources);
        }

        // Try 2: Development location (target/debug/Resources or target/release/Resources)
        if let Some(parent) = exe_path.parent() {
            let dev_resources = parent.join("Resources");
            if dev_resources.exists() {
                return Some(dev_resources);
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    {
        let exe_path = env::current_exe().ok()?;

        // Try 1: Next to executable (for installed apps)
        let app_resources = exe_path.parent()?.join("Resources");
        if app_resources.exists() {
            return Some(app_resources);
        }

        // Try 2: Development location (target/debug/Resources or target/release/Resources)
        if let Some(parent) = exe_path.parent() {
            let dev_resources = parent.join("Resources");
            if dev_resources.exists() {
                return Some(dev_resources);
            }
        }

        None
    }

    #[cfg(target_os = "ios")]
    {
        // For iOS, resources are in app bundle but loaded via Swift bridge
        None
    }
}

/// Load a font file by name
#[allow(dead_code)]
pub fn load_font(font_name: &str) -> Option<Vec<u8>> {
    // ext_res: Load from Resources/Fonts/ directory on disk
    #[cfg(feature = "ext_res")]
    {
        if let Some(resources_dir) = get_resources_dir() {
            let font_path = resources_dir.join("Fonts").join(font_name);
            if font_path.exists() {
                log::info!("Loading font from external file: {:?}", font_path);
                match std::fs::read(&font_path) {
                    Ok(bytes) => {
                        log::debug!(
                            "Successfully loaded external font {} ({} bytes)",
                            font_name,
                            bytes.len()
                        );
                        return Some(bytes);
                    }
                    Err(e) => {
                        log::warn!("Failed to read external font {}: {}", font_name, e);
                    }
                }
            } else {
                log::debug!("Font not found at: {:?}", font_path);
            }
        }
        log::error!("Failed to load font from external resources: {}", font_name);
        None
    }

    // Embedded: Use include_bytes! data (desktop without ext_res)
    #[cfg(all(not(feature = "ext_res"), not(target_arch = "wasm32")))]
    {
        log::debug!("Loading embedded font: {}", font_name);
        match font_name {
            "D2Coding-Ver1.3.2-20180524-all.ttc" => Some(FONT_D2CODING.to_vec()),
            "SourceHanSansVF-remapped.otf" => Some(FONT_SOURCE_HAN_SANS.to_vec()),
            "Barlow-Variable-Remapped.ttf" => Some(FONT_BARLOW.to_vec()),
            "Barlow-Variable-Remapped-Narrow.ttf" => Some(FONT_BARLOW_NARROW.to_vec()),
            #[cfg(feature = "desktop")]
            "DynaPuff-Variable.ttf" => Some(FONT_DYNAPUFF.to_vec()),
            _ => {
                log::warn!("Unknown embedded font: {}", font_name);
                None
            }
        }
    }

    // WASM: Read from pre-loaded font cache (populated by preload_fonts())
    #[cfg(target_arch = "wasm32")]
    {
        log::debug!("Loading font from WASM cache: {}", font_name);
        wasm_font_cache::get(font_name)
    }
}

/// Load a model file by name
#[allow(dead_code)]
pub fn load_model(model_name: &str) -> Option<Vec<u8>> {
    #[cfg(feature = "ext_res")]
    {
        if let Some(resources_dir) = get_resources_dir() {
            let model_path = resources_dir.join("Models").join(model_name);
            if model_path.exists() {
                log::info!("Loading model from external file: {:?}", model_path);
                match std::fs::read(&model_path) {
                    Ok(bytes) => {
                        log::debug!(
                            "Successfully loaded external model {} ({} bytes)",
                            model_name,
                            bytes.len()
                        );
                        return Some(bytes);
                    }
                    Err(e) => {
                        log::warn!("Failed to read external model {}: {}", model_name, e);
                    }
                }
            } else {
                log::debug!("Model not found at: {:?}", model_path);
            }
        }
        log::error!(
            "Failed to load model from external resources: {}",
            model_name
        );
        None
    }

    #[cfg(not(feature = "ext_res"))]
    {
        // Use embedded model (const static)
        log::debug!("Loading embedded model: {}", model_name);
        match model_name {
            #[cfg(has_insightface_model)]
            "det_10g.onnx" => Some(MODEL_INSIGHTFACE.to_vec()),
            _ => {
                log::warn!("Unknown embedded model: {}", model_name);
                None
            }
        }
    }
}

/// Load a logo/SVG file by name
#[allow(dead_code)]
pub fn load_logo(logo_name: &str) -> Option<Vec<u8>> {
    #[cfg(feature = "ext_res")]
    {
        if let Some(resources_dir) = get_resources_dir() {
            let logo_path = resources_dir.join("Logos").join(logo_name);
            if logo_path.exists() {
                log::debug!("Loading logo from external file: {:?}", logo_path);
                match std::fs::read(&logo_path) {
                    Ok(bytes) => {
                        log::debug!(
                            "Successfully loaded external logo {} ({} bytes)",
                            logo_name,
                            bytes.len()
                        );
                        return Some(bytes);
                    }
                    Err(e) => {
                        log::warn!("Failed to read external logo {}: {}", logo_name, e);
                    }
                }
            } else {
                log::debug!("Logo not found at: {:?}", logo_path);
            }
        }
        None
    }

    #[cfg(not(feature = "ext_res"))]
    {
        // Logos are downloaded during build time, no embedded fallback yet
        log::warn!(
            "Logo loading without ext_res not yet implemented: {}",
            logo_name
        );
        None
    }
}

// ===== WASM Font Cache =====
// Fonts are fetched asynchronously at startup and cached for synchronous access.

#[cfg(target_arch = "wasm32")]
mod wasm_font_cache {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    static FONT_CACHE: OnceLock<HashMap<String, Vec<u8>>> = OnceLock::new();

    pub fn init(fonts: HashMap<String, Vec<u8>>) {
        FONT_CACHE.set(fonts).ok();
    }

    pub fn get(name: &str) -> Option<Vec<u8>> {
        FONT_CACHE.get()?.get(name).cloned()
    }
}

/// Preload all font files from the web server into memory cache.
/// Must be called (and awaited) before any font access on WASM.
#[cfg(target_arch = "wasm32")]
pub async fn preload_fonts() {
    let font_names = [
        "D2Coding-Ver1.3.2-20180524-all.ttc",
        "SourceHanSansVF-remapped.otf",
        "Barlow-Variable-Remapped.ttf",
        "Barlow-Variable-Remapped-Narrow.ttf",
        "digital-7.ttf",
        "digital-7-italic.ttf",
        "DynaPuff-Variable.ttf",
    ];

    let mut cache = std::collections::HashMap::new();

    for name in &font_names {
        let url = format!("./Fonts/{}", name);
        match fetch_bytes(&url).await {
            Ok(bytes) => {
                log::info!("Preloaded font {} ({} bytes)", name, bytes.len());
                cache.insert(name.to_string(), bytes);
            }
            Err(e) => {
                log::warn!("Failed to preload font {}: {}", name, e);
            }
        }
    }

    log::info!(
        "Font preload complete: {}/{} fonts loaded",
        cache.len(),
        font_names.len()
    );
    wasm_font_cache::init(cache);
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/js/font_loader.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn fetch_font_bytes(url: &str) -> Result<JsValue, JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    // Wrap the async fetch in a JS Promise so we can race it with a timeout
    let url_owned = url.to_string();
    let fetch_promise = js_sys::Promise::new(&mut |resolve, reject| {
        let url_inner = url_owned.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_font_bytes(&url_inner).await {
                Ok(val) => {
                    let _ = resolve.call1(&JsValue::NULL, &val);
                }
                Err(e) => {
                    let _ = reject.call1(&JsValue::NULL, &e);
                }
            }
        });
    });

    let js_val = crate::util::web_helper::race_with_timeout(fetch_promise, 15_000)
        .await
        .map_err(|e| format!("Font fetch '{}': {}", url, e))?;

    let uint8_array = js_sys::Uint8Array::new(&js_val);
    let vec = uint8_array.to_vec();
    log::debug!("fetch {} → {} bytes", url, vec.len());
    Ok(vec)
}

/// List available fonts
#[allow(dead_code)]
pub fn list_available_fonts() -> Vec<String> {
    let mut fonts = Vec::new();

    #[cfg(feature = "ext_res")]
    {
        if let Some(resources_dir) = get_resources_dir() {
            let fonts_dir = resources_dir.join("Fonts");
            if let Ok(entries) = std::fs::read_dir(fonts_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.ends_with(".ttf")
                            || name.ends_with(".otf")
                            || name.ends_with(".ttc")
                        {
                            fonts.push(name);
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(feature = "ext_res"))]
    {
        // Add embedded font names
        fonts.extend_from_slice(&[
            "D2Coding-Ver1.3.2-20180524-all.ttc".to_string(),
            "SourceHanSansVF-remapped.otf".to_string(),
            "Barlow-Variable-Remapped.ttf".to_string(),
            "Barlow-Variable-Remapped-Narrow.ttf".to_string(),
            "DynaPuff-Variable.ttf".to_string(),
        ]);
    }

    fonts
}

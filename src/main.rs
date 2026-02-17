/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#![warn(clippy::all)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// Desktop-only imports (not available when ios_integration or WASM is enabled)
#[cfg(all(not(feature = "ios_integration"), not(target_arch = "wasm32")))]
use chama_optics::ChamaOptics;

// WASM imports
#[cfg(target_arch = "wasm32")]
use chama_optics::ChamaOptics;

// When compiling natively with desktop feature (and not ios_integration):
#[cfg(all(not(feature = "ios_integration"), not(target_arch = "wasm32")))]
fn main() -> eframe::Result<()> {
    env_logger::init();
    log::info!("env_logger initialized");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([550.0, 680.0])
            .with_min_inner_size([500.0, 630.0])
            .with_drag_and_drop(true)
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/mac-icon.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "ChamaOptics",
        native_options,
        Box::new(|cc| Ok(Box::new(ChamaOptics::new(cc)))),
    )
}

// WASM entry point — eframe renders to a <canvas> element via WebGL
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;

    // Redirect log messages to browser console
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        // Preload fonts from server before starting eframe
        // (fonts must be cached before lazy_static FONTS_UNIFY is accessed)
        chama_optics::resources::preload_fonts().await;

        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id is not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(ChamaOptics::new(cc)))),
            )
            .await;

        // Remove the loading text
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            if let Some(parent) = loading_text.parent_node() {
                parent.remove_child(&loading_text).ok();
            }
        }

        if let Err(e) = start_result {
            log::error!("Failed to start eframe: {:?}", e);
            panic!("Failed to start eframe: {:?}", e);
        }
    });
}

// When compiling with ios_integration
#[cfg(feature = "ios_integration")]
fn main() {
    println!("This binary is not used for iOS builds. Use the library directly via FFI.");
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

#![warn(clippy::all)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use chama_optics::ChamaOptics;
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[clap(long, default_value_t = false)]
    testkit: bool,

    #[clap(long, default_value = "film")]
    theme: String,
}

fn cli_launch(args: &Cli) {
    let pi_list = chama_optics::test_helper::list_import_packed_images();

    let export_config = chama_optics::export_config::ExportConfig::testkit_default();
    let theme = export_config.theme_reg.find(&args.theme).unwrap();

    for pi in pi_list.iter() {
        let new_path = pi.bulk_path(&export_config);
        println!("{:?} -> {:?}", pi.path, new_path);

        theme.apply(pi, &export_config, &new_path).unwrap();
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();
    log::info!("env_logger initialized");

    let args = Cli::parse();

    if !args.testkit {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([550.0, 680.0])
                .with_min_inner_size([500.0, 630.0])
                .with_drag_and_drop(true)
                .with_icon(
                    eframe::icon_data::from_png_bytes(
                        &include_bytes!("../assets/mac-icon.png")[..],
                    )
                    .expect("Failed to load icon"),
                ),
            ..Default::default()
        };
        eframe::run_native(
            "ChamaOptics",
            native_options,
            Box::new(|cc| Ok(Box::new(ChamaOptics::new(cc)))),
        )
    } else {
        cli_launch(&args);
        eframe::Result::Ok(())
    }
}

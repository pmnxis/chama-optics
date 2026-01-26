/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: MIT
 */

use rust_i18n::t;

#[cfg(all(feature = "desktop", not(feature = "ios_integration")))]
fn __launch_exp<P: AsRef<std::path::Path>>(path: P) {
    let path = path.as_ref();

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

#[cfg(all(feature = "desktop", not(feature = "ios_integration")))]
pub(crate) fn launch_explorer_ui<P: AsRef<std::path::Path>>(ui: &mut egui::Ui, path: P) {
    if ui
        .button(t!("export_config.open_folder.label"))
        .on_hover_text(t!("export_config.open_folder.description"))
        .clicked()
    {
        __launch_exp(path);
    }
}

/*
 * SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
 *
 * SPDX-License-Identifier: LicenseRef-Non-AI-MIT
 */

//! UI component rendering functions following egui best practices

use crate::ui_state::ProgressState;
use rust_i18n::t;

/// Render the bottom panel with progress bars and version info
pub fn render_bottom_panel(
    ui: &mut egui::Ui,
    load_progress: &mut ProgressState,
    save_progress: &mut ProgressState,
    update_checker: &crate::util::check_update::CheckRelease,
) {
    egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
        // Loading progress bar (blue)
        if load_progress.is_active() {
            render_progress_bar(
                ui,
                load_progress,
                &t!("progress.loading.in_progress"),
                &t!("progress.loading.completed"),
                Some(egui::Color32::from_rgb(0, 120, 215)), // Blue
            );
        }

        // Saving progress bar (default green)
        if save_progress.is_active() {
            render_progress_bar(
                ui,
                save_progress,
                &t!("progress.saving.in_progress"),
                &t!("progress.saving.completed"),
                None, // Default color
            );
        }

        // Version info
        render_version_info(ui, update_checker);
    });
}

/// Render a progress bar with consistent styling
fn render_progress_bar(
    ui: &mut egui::Ui,
    progress: &mut ProgressState,
    in_progress_label: &str,
    completed_label: &str,
    color: Option<egui::Color32>,
) {
    ui.vertical(|ui| {
        let current = progress.current();
        let total = progress.total();
        let fraction = progress.fraction();

        if !progress.is_complete() {
            // Still in progress
            ui.ctx().request_repaint();

            ui.label(format!(
                "{}: {}/{} ({:.0}%)",
                in_progress_label,
                current,
                total,
                fraction * 100.0
            ));

            let mut bar = egui::ProgressBar::new(fraction)
                .show_percentage()
                .animate(true);

            if let Some(c) = color {
                bar = bar.fill(c);
            }

            ui.add(bar);
        } else {
            // Completed - mark and show for 2 seconds
            progress.mark_complete();

            if !progress.should_hide(std::time::Duration::from_secs(2)) {
                ui.label(format!("{}: {}/{}", completed_label, current, total));

                let mut bar = egui::ProgressBar::new(1.0).show_percentage();
                if let Some(c) = color {
                    bar = bar.fill(c);
                }
                ui.add(bar);

                ui.ctx().request_repaint();
            } else {
                // Hide after 2 seconds
                progress.reset();
            }
        }
    });
}

/// Render version information and update check
fn render_version_info(
    ui: &mut egui::Ui,
    update_checker: &crate::util::check_update::CheckRelease,
) {
    egui::warn_if_debug_build(ui);
    ui.horizontal(|ui| {
        ui.label("ChamaOptics");
        ui.add_space(20.0);
        ui.label(format!(
            "v{} ({})",
            env!("PROJECT_VERSION"),
            env!("GIT_COMMIT_SHORT_HASH")
        ));
        ui.add_space(20.0);

        update_checker.ui(ui);
    });
}

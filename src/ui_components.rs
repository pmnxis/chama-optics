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
        ui.horizontal(|ui| {
            // Left: Debug build warning (always on left, invisible if not debug)
            egui::warn_if_debug_build(ui);

            // Center: Progress bars (expands to fill available space)
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
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
            });

            // Right: Version info (always on right)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                render_version_info(ui, update_checker);
            });
        });
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
    let current = progress.current();
    let total = progress.total();
    let fraction = progress.fraction();

    if !progress.is_complete() {
        // Still in progress
        ui.ctx().request_repaint();

        let mut bar = egui::ProgressBar::new(fraction)
            .text(format!(
                "{} {}/{} ({:.0}%)",
                in_progress_label,
                current,
                total,
                fraction * 100.0
            ))
            .animate(true);

        if let Some(c) = color {
            bar = bar.fill(c);
        }

        ui.add(bar);
    } else {
        // Completed - mark and show for 2 seconds
        progress.mark_complete();

        if !progress.should_hide(std::time::Duration::from_secs(2)) {
            let mut bar = egui::ProgressBar::new(1.0)
                .text(format!("{} {}/{}", completed_label, current, total));
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
}

/// Render version information and update check
fn render_version_info(
    ui: &mut egui::Ui,
    update_checker: &crate::util::check_update::CheckRelease,
) {
    // Right-to-left layout, so elements appear in reverse order

    // Only show update checker on desktop (not on web)
    #[cfg(not(target_arch = "wasm32"))]
    {
        update_checker.ui(ui);
        ui.add_space(20.0);
    }

    // Suppress unused warning for WASM
    #[cfg(target_arch = "wasm32")]
    let _ = update_checker;

    ui.label(format!(
        "v{} ({})",
        env!("PROJECT_VERSION"),
        env!("GIT_COMMIT_SHORT_HASH")
    ));
}

/// Render the left sidebar with tab icons
pub fn render_tab_sidebar(ui: &mut egui::Ui, selected_tab: &mut crate::app::MainTab) {
    use crate::app::MainTab;

    egui::Panel::left("tab_sidebar")
        .resizable(false)
        .exact_size(50.0)
        .show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);

                // Tab 1: Image List (☰)
                render_tab_button(
                    ui,
                    selected_tab,
                    MainTab::ImageList,
                    "☰",
                    "List",
                    &t!("tabs.image_list"),
                );

                ui.add_space(5.0);

                // Tab 2: Theme Preview (▦)
                render_tab_button(
                    ui,
                    selected_tab,
                    MainTab::ThemePreview,
                    "▦",
                    "Theme",
                    &t!("tabs.theme_preview"),
                );

                ui.add_space(5.0);

                // Tab 3: Import&Export (⚙)
                render_tab_button(
                    ui,
                    selected_tab,
                    MainTab::ImportExport,
                    "⚙",
                    "Config",
                    &t!("tabs.import_export"),
                );

                ui.add_space(5.0);

                // Tab 4: Settings (…)
                render_tab_button(
                    ui,
                    selected_tab,
                    MainTab::Settings,
                    "…",
                    "Misc",
                    &t!("tabs.settings"),
                );
            });
        });
}

/// Render a single tab button with icon and label
fn render_tab_button(
    ui: &mut egui::Ui,
    selected_tab: &mut crate::app::MainTab,
    tab: crate::app::MainTab,
    icon: &str,
    label: &str,
    hover_text: &str,
) {
    ui.vertical_centered(|ui| {
        let is_active = *selected_tab == tab;
        if ui
            .selectable_label(is_active, egui::RichText::new(icon).size(24.0))
            .on_hover_text(hover_text)
            .clicked()
        {
            *selected_tab = tab;
        }
        let text_color = if is_active {
            ui.visuals().strong_text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        ui.label(egui::RichText::new(label).size(9.0).color(text_color));
    });
}

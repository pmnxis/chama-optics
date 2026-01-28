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
    update_checker.ui(ui);
    ui.add_space(20.0);

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

                // Tab 2: Face Detection (👤)
                render_tab_button(
                    ui,
                    selected_tab,
                    MainTab::Detection,
                    "👤",
                    "Detect",
                    &t!("tabs.detection", default = "Face Detection"),
                );

                ui.add_space(5.0);

                // Tab 3: Theme Preview (▦)
                render_tab_button(
                    ui,
                    selected_tab,
                    MainTab::ThemePreview,
                    "▦",
                    "Theme",
                    &t!("tabs.theme_preview"),
                );

                ui.add_space(5.0);

                // Tab 4: Color Grading (🎨)
                render_tab_button(
                    ui,
                    selected_tab,
                    MainTab::Color,
                    "🎨",
                    "Color",
                    &t!("tabs.color", default = "Color Grading"),
                );

                ui.add_space(5.0);

                // Tab 5: Sticker (🎭)
                render_tab_button(
                    ui,
                    selected_tab,
                    MainTab::Sticker,
                    "🎭",
                    "Sticker",
                    &t!("tabs.sticker", default = "Sticker Storage"),
                );

                ui.add_space(5.0);

                // Tab 5: Import&Export (⚙)
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

/// Common horizontal scrollable gallery component for image selection
///
/// # Type Parameters
/// * `T` - The type of item being displayed (e.g., PackedImage, Sticker)
/// * `ID` - The ID type (e.g., usize, Uuid)
///
/// # Callbacks
/// * `items` - Iterator over items to display
/// * `get_id` - Function to extract ID from item
/// * `get_name` - Function to extract display name from item
/// * `get_texture` - Function to get texture handle from item (returns None if not loaded yet)
/// * `is_selected` - Function to check if item is currently selected
/// * `is_default` - Optional function to check if item is marked as default (shows ⭐)
/// * `show_warning` - Optional function to show warning icons (❌/⚠️)
/// * `on_select` - Callback when an item is clicked
/// * `on_delete` - Optional callback when delete button is clicked on hover
///
/// # Returns
/// * `Option<ID>` - The ID of item to delete (if any delete button was clicked)
#[allow(clippy::too_many_arguments)]
pub fn render_horizontal_gallery<T, ID, F, G, H, I, J, K>(
    ui: &mut egui::Ui,
    items: impl IntoIterator<Item = T>,
    get_id: F,
    get_name: G,
    get_texture: H,
    is_selected: I,
    is_default: Option<J>,
    show_warning: Option<K>,
    on_select: &mut impl FnMut(ID),
    on_delete: Option<&mut dyn FnMut(ID)>,
) -> Option<ID>
where
    T: Clone,
    ID: Copy + std::fmt::Debug,
    F: Fn(&T) -> ID,
    G: Fn(&T) -> String,
    H: Fn(&egui::Context, &T) -> Option<egui::TextureHandle>,
    I: Fn(ID) -> bool,
    J: Fn(&T) -> bool,
    K: Fn(&T) -> Option<(bool, bool)>, // Returns (file_missing, hash_mismatch)
{
    let mut item_to_delete: Option<ID> = None;

    // Fixed height gallery to prevent layout shift during loading
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 120.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            egui::ScrollArea::horizontal()
                .id_salt("horizontal_gallery")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for item in items {
                            let id = get_id(&item);
                            let is_sel = is_selected(id);

                            // Container for thumbnail + filename (fixed width to prevent layout shift)
                            let container_response = ui.allocate_ui_with_layout(
                                egui::vec2(80.0, 100.0),
                                egui::Layout::top_down(egui::Align::Center),
                                |ui| {
                                    // Thumbnail (80x80) with optional selection frame
                                    let thumbnail_size = egui::vec2(80.0, 80.0);

                                    let frame = if is_sel {
                                        egui::Frame::new().stroke(egui::Stroke::new(
                                            2.0,
                                            egui::Color32::from_rgb(0, 150, 255),
                                        ))
                                    } else {
                                        egui::Frame::NONE
                                    };

                                    let image_response = if let Some(texture) =
                                        get_texture(ui.ctx(), &item)
                                    {
                                        frame
                                            .show(ui, |ui| {
                                                ui.add(
                                                    egui::Image::from_texture(&texture)
                                                        .fit_to_exact_size(thumbnail_size)
                                                        .sense(egui::Sense::click_and_drag()),
                                                )
                                            })
                                            .inner
                                    } else {
                                        // Placeholder for not-yet-loaded items
                                        let (response, _painter) = ui
                                            .allocate_painter(thumbnail_size, egui::Sense::click());
                                        ui.painter().text(
                                            response.rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            "📷",
                                            egui::FontId::proportional(32.0),
                                            ui.visuals().weak_text_color(),
                                        );
                                        response
                                    };

                                    // File name (small text, centered, max width 80px)
                                    let name = get_name(&item);
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&name)
                                                .size(10.0)
                                                .color(ui.visuals().weak_text_color()),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(&name);

                                    // Show default indicator if applicable
                                    if let Some(ref is_def) = is_default
                                        && is_def(&item)
                                    {
                                        ui.label(
                                            egui::RichText::new("⭐")
                                                .small()
                                                .color(ui.visuals().warn_fg_color),
                                        );
                                    }

                                    // Show warnings if applicable
                                    if let Some(ref show_warn) = show_warning
                                        && let Some((missing, modified)) = show_warn(&item)
                                    {
                                        if missing {
                                            ui.label(
                                                egui::RichText::new("❌")
                                                    .size(12.0)
                                                    .color(ui.visuals().error_fg_color),
                                            );
                                        } else if modified {
                                            ui.label(
                                                egui::RichText::new("⚠️")
                                                    .size(12.0)
                                                    .color(ui.visuals().warn_fg_color),
                                            );
                                        }
                                    }

                                    image_response
                                },
                            );

                            let image_response = container_response.inner;
                            let rect = image_response.rect;

                            // Check if mouse is hovering over image
                            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                            let is_hovered = if let Some(pos) = pointer_pos {
                                rect.contains(pos)
                            } else {
                                false
                            };

                            // Delete button on hover (top-right corner)
                            if is_hovered && on_delete.is_some() {
                                let button_size = 20.0;
                                let delete_button_rect = egui::Rect::from_min_size(
                                    rect.right_top() + egui::vec2(-button_size, 0.0),
                                    egui::vec2(button_size, button_size),
                                );

                                // Draw delete button visuals (red circle with X)
                                let center = delete_button_rect.center();
                                ui.painter().circle_filled(
                                    center,
                                    10.0,
                                    egui::Color32::from_rgba_premultiplied(220, 50, 50, 220),
                                );

                                // Draw X using lines
                                let x_size = 5.0;
                                ui.painter().line_segment(
                                    [
                                        center + egui::vec2(-x_size, -x_size),
                                        center + egui::vec2(x_size, x_size),
                                    ],
                                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                                );
                                ui.painter().line_segment(
                                    [
                                        center + egui::vec2(x_size, -x_size),
                                        center + egui::vec2(-x_size, x_size),
                                    ],
                                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                                );

                                // Check if delete button was clicked
                                if let Some(pos) = pointer_pos {
                                    if delete_button_rect.contains(pos) && image_response.clicked()
                                    {
                                        item_to_delete = Some(id);
                                    } else if image_response.clicked() {
                                        on_select(id);
                                    }
                                }
                            } else if image_response.clicked() {
                                on_select(id);
                            }

                            ui.add_space(5.0);
                        }
                    });
                });
        },
    );

    item_to_delete
}

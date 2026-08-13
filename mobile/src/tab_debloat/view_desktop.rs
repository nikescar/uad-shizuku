//! Desktop view for debloat tab
//!
//! This module implements the desktop layout (800px+) with:
//! - 200px left sidebar: Filter controls and options
//! - Main content area: Search bar, batch actions, error banner, package table
//!
//! The desktop view uses virtual scrolling for performance with large package lists.

use eframe::egui;

use crate::viewmodel::ViewModelState;
use super::state::TabDebloatState;
use super::components::render_package_table;

/// Sidebar width in pixels
const SIDEBAR_WIDTH: f32 = 200.0;

/// Render desktop view with sidebar layout
///
/// This function implements the desktop interface with:
/// - Left sidebar (200px): Category filters, options, advanced settings
/// - Main content area: Search, batch actions, error banner, virtual table
///
/// # Arguments
/// * `ui` - egui context for rendering
/// * `vm_state` - ViewModel state (read-only access to packages and UAD lists)
/// * `local_state` - Tab-local UI state (mutable for selection, filters, etc.)
pub fn render(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
) {
    egui::SidePanel::left("debloat_sidebar")
        .exact_width(SIDEBAR_WIDTH)
        .resizable(false)
        .show_inside(ui, |ui| {
            render_sidebar(ui, vm_state, local_state);
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        render_main_content(ui, vm_state, local_state);
    });
}

/// Render the left sidebar with filter controls
fn render_sidebar(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
) {
    ui.vertical(|ui| {
        ui.heading("Filters");
        ui.separator();

        // Category filters section
        ui.label("Category");
        ui.horizontal(|ui| {
            if ui.selectable_label(local_state.active_filter.category_filter.is_none(), "All").clicked() {
                local_state.active_filter.category_filter = None;
                local_state.table_version += 1;
            }
            ui.label(format!("({})", local_state.cached_counts.all));
        });

        ui.horizontal(|ui| {
            if ui.selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("recommended"),
                "Recommended"
            ).clicked() {
                local_state.active_filter.category_filter = Some("recommended".to_string());
                local_state.table_version += 1;
            }
            ui.label(format!("({})", local_state.cached_counts.recommended));
        });

        ui.horizontal(|ui| {
            if ui.selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("unsafe"),
                "Unsafe"
            ).clicked() {
                local_state.active_filter.category_filter = Some("unsafe".to_string());
                local_state.table_version += 1;
            }
            ui.label(format!("({})", local_state.cached_counts.unsafe_apps));
        });

        ui.horizontal(|ui| {
            if ui.selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("expert"),
                "Expert"
            ).clicked() {
                local_state.active_filter.category_filter = Some("expert".to_string());
                local_state.table_version += 1;
            }
            ui.label(format!("({})", local_state.cached_counts.expert));
        });

        ui.add_space(16.0);

        // Options section
        ui.separator();
        ui.heading("Options");

        if ui.checkbox(&mut local_state.active_filter.show_only_enabled, "Show only enabled").changed() {
            local_state.table_version += 1;
        }

        if ui.checkbox(&mut local_state.active_filter.hide_system_apps, "Hide system apps").changed() {
            local_state.table_version += 1;
        }

        ui.add_space(16.0);

        // Advanced settings section
        ui.separator();
        ui.heading("Advanced");

        ui.checkbox(&mut local_state.unsafe_app_remove, "Unsafe removal");
        ui.checkbox(&mut local_state.expert_app_remove, "Expert mode");

        // Display device info if available
        if let Some(device) = &local_state.selected_device {
            ui.add_space(16.0);
            ui.separator();
            ui.label("Device");
            ui.label(device);
        }

        // Display package count from ViewModel
        ui.add_space(16.0);
        ui.separator();
        ui.label(format!("Total packages: {}", vm_state.packages.len()));
        ui.label(format!("Filtered: {}", vm_state.filtered_packages.len()));
    });
}

/// Render the main content area
fn render_main_content(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
) {
    ui.vertical(|ui| {
        // Search bar
        render_search_bar(ui, local_state);

        ui.add_space(8.0);

        // Batch action buttons
        render_batch_actions(ui, local_state);

        ui.add_space(8.0);

        // Error banner (if any errors exist)
        render_error_banner(ui, local_state);

        ui.separator();

        // Package table (virtual scrolling)
        render_package_table(
            ui,
            &vm_state.filtered_packages,
            &mut local_state.selected_packages,
        );
    });
}

/// Render search bar for text filtering with debouncing
fn render_search_bar(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
    ui.horizontal(|ui| {
        ui.label("Search:");
        let response = ui.text_edit_singleline(&mut local_state.pending_filter_text);
        if response.changed() {
            // User typed something - start/reset debounce timer
            local_state.last_filter_input = Some(std::time::Instant::now());
            local_state.active_filter.text_filter = local_state.pending_filter_text.clone();
        }
        if ui.button("Clear").clicked() {
            local_state.pending_filter_text.clear();
            local_state.applied_filter_text.clear();
            local_state.active_filter.text_filter.clear();
            local_state.last_filter_input = Some(std::time::Instant::now());
        }
    });
}

/// Render batch action buttons
fn render_batch_actions(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
    ui.horizontal(|ui| {
        let selection_count = local_state.selected_packages.len();
        ui.label(format!("Selected: {}", selection_count));

        ui.add_enabled_ui(selection_count > 0, |ui| {
            if ui.button("Uninstall").clicked() {
                // TODO: Trigger batch uninstall via ViewModel command
                log::info!("Batch uninstall requested for {} packages", selection_count);
            }

            if ui.button("Disable").clicked() {
                // TODO: Trigger batch disable via ViewModel command
                log::info!("Batch disable requested for {} packages", selection_count);
            }

            if ui.button("Enable").clicked() {
                // TODO: Trigger batch enable via ViewModel command
                log::info!("Batch enable requested for {} packages", selection_count);
            }
        });

        if selection_count > 0 {
            if ui.button("Clear Selection").clicked() {
                local_state.selected_packages.clear();
            }
        }

        // Select all / Deselect all buttons
        if ui.button("Select All").clicked() {
            // TODO: Select all filtered packages
            log::info!("Select all requested");
        }
    });
}

/// Render error banner if there are active errors
fn render_error_banner(ui: &mut egui::Ui, local_state: &TabDebloatState) {
    // Check if batch operations have errors
    let has_error = !local_state.batch_uninstall_state.status_message.is_empty()
        || !local_state.batch_disable_state.status_message.is_empty()
        || !local_state.batch_enable_state.status_message.is_empty();

    if has_error {
        ui.colored_label(
            egui::Color32::from_rgb(255, 100, 100),
            "⚠ Operation errors detected",
        );

        if !local_state.batch_uninstall_state.status_message.is_empty() {
            ui.label(format!("Uninstall: {}", local_state.batch_uninstall_state.status_message));
        }

        if !local_state.batch_disable_state.status_message.is_empty() {
            ui.label(format!("Disable: {}", local_state.batch_disable_state.status_message));
        }

        if !local_state.batch_enable_state.status_message.is_empty() {
            ui.label(format!("Enable: {}", local_state.batch_enable_state.status_message));
        }

        ui.separator();
    }

    // Show batch operation progress if active
    render_batch_progress(ui, local_state);
}

/// Render batch operation progress bars
fn render_batch_progress(ui: &mut egui::Ui, local_state: &TabDebloatState) {
    // Uninstall progress
    if let Ok(guard) = local_state.batch_uninstall_progress.try_lock() {
        if let Some(progress) = *guard {
            ui.horizontal(|ui| {
                ui.label("Uninstalling:");
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                if let Ok(mut cancelled) = local_state.batch_uninstall_cancelled.try_lock() {
                    if ui.button("Cancel").clicked() {
                        *cancelled = true;
                    }
                }
            });
        }
    }

    // Disable progress
    if let Ok(guard) = local_state.batch_disable_progress.try_lock() {
        if let Some(progress) = *guard {
            ui.horizontal(|ui| {
                ui.label("Disabling:");
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                if let Ok(mut cancelled) = local_state.batch_disable_cancelled.try_lock() {
                    if ui.button("Cancel").clicked() {
                        *cancelled = true;
                    }
                }
            });
        }
    }

    // Enable progress
    if let Ok(guard) = local_state.batch_enable_progress.try_lock() {
        if let Some(progress) = *guard {
            ui.horizontal(|ui| {
                ui.label("Enabling:");
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                if let Ok(mut cancelled) = local_state.batch_enable_cancelled.try_lock() {
                    if ui.button("Cancel").clicked() {
                        *cancelled = true;
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_width() {
        assert_eq!(SIDEBAR_WIDTH, 200.0);
    }
}

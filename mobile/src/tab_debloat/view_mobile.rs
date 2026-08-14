//! Mobile view for debloat tab
//!
//! This module implements the mobile layout (<800px) with:
//! - Stacked vertical layout for narrow screens
//! - Collapsible filter section
//! - Card-based package list (48px minimum per card)
//! - Batch actions at bottom
//!
//! The mobile view is optimized for touch interaction and vertical scrolling.

use eframe::egui;

use super::components::package_cards::render_package_cards;
use super::state::TabDebloatState;
use crate::viewmodel::ViewModelState;

/// Render mobile view with stacked layout
///
/// This function implements the mobile interface with:
/// - Top section: Search bar and collapsible filters
/// - Middle section: Package cards (scrollable)
/// - Bottom section: Batch action buttons
///
/// # Arguments
/// * `ui` - egui context for rendering
/// * `vm_state` - ViewModel state (read-only access to packages and UAD lists)
/// * `local_state` - Tab-local UI state (mutable for selection, filters, etc.)
pub fn render(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) {
    ui.vertical(|ui| {
        // Search bar (always visible)
        render_search_bar(ui, local_state);

        ui.add_space(8.0);

        // Collapsible filter section
        render_filter_section(ui, vm_state, local_state);

        ui.add_space(8.0);

        // Error banner (if any errors exist)
        render_error_banner(ui, local_state);

        ui.separator();

        // Package cards (scrollable, takes remaining space)
        render_package_list(ui, vm_state, local_state, google_play_enabled, fdroid_enabled, apkmirror_enabled, android_package_enabled);

        ui.separator();

        // Batch actions at bottom (fixed)
        render_batch_actions(ui, local_state);
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
        }
        if ui.button("Clear").clicked() {
            local_state.pending_filter_text.clear();
            local_state.applied_filter_text.clear();
            local_state.active_filter.text_filter.clear();
            local_state.last_filter_input = None;
        }
    });
}

/// Render collapsible filter section
fn render_filter_section(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
) {
    egui::CollapsingHeader::new("Filters")
        .default_open(false)
        .show(ui, |ui| {
            // Category filters
            ui.label("Category");

            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(
                        local_state.active_filter.category_filter.is_none(),
                        format!("All ({})", local_state.cached_counts.all),
                    )
                    .clicked()
                {
                    local_state.active_filter.category_filter = None;
                    local_state.table_version += 1;
                }

                if ui
                    .selectable_label(
                        local_state.active_filter.category_filter.as_deref() == Some("recommended"),
                        format!("Recommended ({})", local_state.cached_counts.recommended),
                    )
                    .clicked()
                {
                    local_state.active_filter.category_filter = Some("recommended".to_string());
                    local_state.table_version += 1;
                }

                if ui
                    .selectable_label(
                        local_state.active_filter.category_filter.as_deref() == Some("unsafe"),
                        format!("Unsafe ({})", local_state.cached_counts.unsafe_apps),
                    )
                    .clicked()
                {
                    local_state.active_filter.category_filter = Some("unsafe".to_string());
                    local_state.table_version += 1;
                }

                if ui
                    .selectable_label(
                        local_state.active_filter.category_filter.as_deref() == Some("expert"),
                        format!("Expert ({})", local_state.cached_counts.expert),
                    )
                    .clicked()
                {
                    local_state.active_filter.category_filter = Some("expert".to_string());
                    local_state.table_version += 1;
                }
            });

            ui.add_space(8.0);

            // Options
            ui.separator();
            ui.label("Options");

            if ui
                .checkbox(
                    &mut local_state.active_filter.show_only_enabled,
                    "Show only enabled",
                )
                .changed()
            {
                local_state.table_version += 1;
            }

            if ui
                .checkbox(
                    &mut local_state.active_filter.hide_system_apps,
                    "Hide system apps",
                )
                .changed()
            {
                local_state.table_version += 1;
            }

            ui.add_space(8.0);

            // Advanced settings
            ui.separator();
            ui.label("Advanced");

            ui.checkbox(&mut local_state.unsafe_app_remove, "Unsafe removal");
            ui.checkbox(&mut local_state.expert_app_remove, "Expert mode");

            // Device info
            if let Some(device) = &local_state.selected_device {
                ui.add_space(8.0);
                ui.separator();
                ui.label(format!("Device: {}", device));
            }

            // Package count
            ui.add_space(8.0);
            ui.separator();
            ui.label(format!(
                "Total: {} | Filtered: {}",
                vm_state.packages.len(),
                vm_state.filtered_packages.len()
            ));
        });
}

/// Render package list with card layout
fn render_package_list(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) {
    // Prepare app metadata (icons, titles) if renderers are enabled
    log::info!("[DEBLOAT] Renderer flags - GP: {}, FD: {}, APK: {}, AP: {}",
        google_play_enabled, fdroid_enabled, apkmirror_enabled, android_package_enabled);

    let package_ids: Vec<String> = vm_state.filtered_packages.iter().map(|p| p.pkg.clone()).collect();
    let system_packages: std::collections::HashSet<String> = vm_state.packages.iter()
        .filter(|p| p.flags.contains("SYSTEM"))
        .map(|p| p.pkg.clone())
        .collect();

    log::info!("[DEBLOAT] Preparing metadata for {} packages ({} system)",
        package_ids.len(), system_packages.len());

    let app_metadata = crate::app_metadata_renderer::prepare_app_info_for_display(
        ui.ctx(),
        &package_ids,
        &system_packages,
        vm_state,
        google_play_enabled,
        fdroid_enabled,
        apkmirror_enabled,
        android_package_enabled,
    );

    log::info!("[DEBLOAT] Got metadata for {} packages", app_metadata.len());

    // Allocate remaining vertical space for scrollable list
    let available_height = ui.available_height() - 60.0; // Reserve space for batch actions

    ui.vertical(|ui| {
        ui.set_min_height(available_height);
        render_package_cards(
            ui,
            &vm_state.filtered_packages,
            &mut local_state.selected_packages,
            vm_state.uad_ng_lists.as_ref(),
            &app_metadata,
        );
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
            ui.label(format!(
                "Uninstall: {}",
                local_state.batch_uninstall_state.status_message
            ));
        }

        if !local_state.batch_disable_state.status_message.is_empty() {
            ui.label(format!(
                "Disable: {}",
                local_state.batch_disable_state.status_message
            ));
        }

        if !local_state.batch_enable_state.status_message.is_empty() {
            ui.label(format!(
                "Enable: {}",
                local_state.batch_enable_state.status_message
            ));
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

/// Render batch action buttons at bottom
fn render_batch_actions(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
    ui.vertical(|ui| {
        let selection_count = local_state.selected_packages.len();

        // Selection count
        ui.label(format!("Selected: {}", selection_count));

        ui.add_space(4.0);

        // Action buttons in grid layout for mobile
        ui.horizontal_wrapped(|ui| {
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

                if ui.button("Clear Selection").clicked() {
                    local_state.selected_packages.clear();
                }
            });

            if ui.button("Select All").clicked() {
                // TODO: Select all filtered packages
                log::info!("Select all requested");
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mobile_view_module_exists() {
        // Ensure module compiles
        assert!(true);
    }
}

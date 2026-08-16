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

use super::components::package_table_mobile::render_package_table_mobile;
use super::filter_logic;
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
    viewmodel: &crate::viewmodel::ViewModel,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) {
    ui.vertical(|ui| {
        // Flattened filter section (includes search, filters, batch actions)
        render_filter_section(ui, vm_state, local_state, viewmodel);

        ui.add_space(8.0);

        // Error banner (if any errors exist)
        render_error_banner(ui, local_state);

        ui.separator();

        // Package cards (scrollable, takes remaining space)
        render_package_list(
            ui,
            vm_state,
            local_state,
            google_play_enabled,
            fdroid_enabled,
            apkmirror_enabled,
            android_package_enabled,
        );
    });
}

/// Render flattened 6-line filter section (no collapsing)
fn render_filter_section(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
) {
    // Line 1: Search bar
    ui.horizontal(|ui| {
        ui.label("Search:");
        let response = ui.add_sized(
            [200.0, ui.spacing().interact_size.y],
            egui::TextEdit::singleline(&mut local_state.pending_filter_text),
        );
        if response.changed() {
            local_state.last_filter_input = Some(std::time::Instant::now());
        }
        if ui.button("Clear").clicked() {
            local_state.pending_filter_text.clear();
            local_state.applied_filter_text.clear();
            local_state.active_filter.text_filter.clear();
            local_state.last_filter_input = None;
        }
    });

    // Line 2: Category (read-only display)
    ui.horizontal(|ui| {
        let (category_name, enabled_count, total_count) =
            match &local_state.active_filter.category_filter {
                Some(cat) if cat == "recommended" => (
                    "Recommended",
                    local_state.cached_counts.recommended_enabled,
                    local_state.cached_counts.recommended,
                ),
                Some(cat) if cat == "advanced" => (
                    "Advanced",
                    local_state.cached_counts.advanced_enabled,
                    local_state.cached_counts.advanced,
                ),
                Some(cat) if cat == "expert" => (
                    "Expert",
                    local_state.cached_counts.expert_enabled,
                    local_state.cached_counts.expert,
                ),
                Some(cat) if cat == "unsafe" => (
                    "Unsafe",
                    local_state.cached_counts.unsafe_apps_enabled,
                    local_state.cached_counts.unsafe_apps,
                ),
                _ => (
                    "All",
                    local_state.cached_counts.all_enabled,
                    local_state.cached_counts.all,
                ),
            };
        ui.label(format!(
            "Category: {} ({}/{})",
            category_name, enabled_count, total_count
        ));
    });

    // Line 3: Options checkboxes
    ui.horizontal(|ui| {
        ui.label("Options");
        if ui
            .checkbox(
                &mut local_state.active_filter.show_only_enabled,
                "Show only enabled",
            )
            .changed()
        {
            let text_filter = if local_state.applied_filter_text.is_empty() {
                None
            } else {
                Some(local_state.applied_filter_text.clone())
            };
            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                local_state.active_filter.category_filter.clone(),
                local_state.active_filter.show_only_enabled,
                local_state.active_filter.hide_system_apps,
            ) {
                log::error!("Failed to apply show_only_enabled filter: {}", e);
            }
        }
        if ui
            .checkbox(
                &mut local_state.active_filter.hide_system_apps,
                "Hide system apps",
            )
            .changed()
        {
            let text_filter = if local_state.applied_filter_text.is_empty() {
                None
            } else {
                Some(local_state.applied_filter_text.clone())
            };
            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                local_state.active_filter.category_filter.clone(),
                local_state.active_filter.show_only_enabled,
                local_state.active_filter.hide_system_apps,
            ) {
                log::error!("Failed to apply hide_system_apps filter: {}", e);
            }
        }
    });

    // Line 4: Advanced checkboxes
    ui.horizontal(|ui| {
        ui.label("Advanced");
        ui.checkbox(&mut local_state.unsafe_app_remove, "Unsafe removal");
        ui.checkbox(&mut local_state.expert_app_remove, "Expert removal");
    });

    // Line 5: Package counts
    ui.label(format!(
        "Total Packages {} Filtered {}",
        vm_state.packages.len(),
        vm_state.filtered_packages.len()
    ));

    // Line 6: Batch actions (moved from bottom)
    let selection_count = local_state.selected_packages.len();
    ui.horizontal(|ui| {
        ui.label(format!("Selected: {}", selection_count));
        ui.add_enabled_ui(selection_count > 0, |ui| {
            if ui.button("Uninstall").clicked() {
                log::info!("Batch uninstall - will wire in Task 7");
            }
            if ui.button("Disable").clicked() {
                log::info!("Batch disable - will wire in Task 6");
            }
            if ui.button("Enable").clicked() {
                log::info!("Batch enable - will wire in Task 6");
            }
            if ui.button("Clear Selection").clicked() {
                local_state.selected_packages.clear();
            }
        });
        if ui.button("Select All").clicked() {
            for pkg in &vm_state.filtered_packages {
                local_state.selected_packages.insert(pkg.pkg.clone());
            }
        }
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
    log::debug!(
        "[DEBLOAT] Renderer flags - GP: {}, FD: {}, APK: {}, AP: {}",
        google_play_enabled,
        fdroid_enabled,
        apkmirror_enabled,
        android_package_enabled
    );

    log::debug!(
        "[DEBLOAT] render_package_list called with {} filtered packages",
        vm_state.filtered_packages.len()
    );

    let package_ids: Vec<String> = vm_state
        .filtered_packages
        .iter()
        .map(|p| p.pkg.clone())
        .collect();
    let system_packages: std::collections::HashSet<String> = vm_state
        .packages
        .iter()
        .filter(|p| p.flags.contains("SYSTEM"))
        .map(|p| p.pkg.clone())
        .collect();

    log::debug!(
        "[DEBLOAT] Preparing metadata for {} packages ({} system)",
        package_ids.len(),
        system_packages.len()
    );

    let full_metadata = crate::app_metadata_renderer::prepare_app_info_for_display(
        ui.ctx(),
        &package_ids,
        &system_packages,
        vm_state,
        google_play_enabled,
        fdroid_enabled,
        apkmirror_enabled,
        android_package_enabled,
    );

    // Convert to mobile table format (texture, title only - omit developer and version)
    let app_metadata: std::collections::HashMap<String, (Option<egui::TextureHandle>, String)> =
        full_metadata
            .iter()
            .map(|(pkg_id, (texture, title, _developer, _version))| {
                (pkg_id.clone(), (texture.clone(), title.clone()))
            })
            .collect();

    log::debug!("[DEBLOAT] Got metadata for {} packages", app_metadata.len());

    // === DIAGNOSTIC LOGGING START ===
    // Sample HashMap keys (first 5)
    let sample_keys: Vec<_> = app_metadata.keys().take(5).cloned().collect();
    log::debug!("[DEBLOAT] Sample app_metadata keys: {:?}", sample_keys);

    // Sample package IDs from filtered list (first 5)
    let sample_packages: Vec<_> = vm_state
        .filtered_packages
        .iter()
        .take(5)
        .map(|p| p.pkg.clone())
        .collect();
    log::debug!(
        "[DEBLOAT] Sample filtered package IDs: {:?}",
        sample_packages
    );

    // Mismatch analysis - first 5 packages not in HashMap
    let missing: Vec<_> = vm_state
        .filtered_packages
        .iter()
        .filter(|p| !app_metadata.contains_key(&p.pkg))
        .take(5)
        .map(|p| p.pkg.clone())
        .collect();
    log::debug!(
        "[DEBLOAT] First 5 packages missing from metadata: {:?}",
        missing
    );

    // Success rate - how many packages have metadata
    let found_count = vm_state
        .filtered_packages
        .iter()
        .filter(|p| app_metadata.contains_key(&p.pkg))
        .count();
    log::debug!(
        "[DEBLOAT] Rendering metrics - Total: {}, With metadata: {}, Hit rate: {:.1}%",
        vm_state.filtered_packages.len(),
        found_count,
        (found_count as f32 / vm_state.filtered_packages.len() as f32) * 100.0
    );
    // === DIAGNOSTIC LOGGING END ===

    // Allocate remaining vertical space for scrollable list
    let available_height = ui.available_height() - 60.0; // Reserve space for batch actions

    ui.vertical(|ui| {
        ui.set_min_height(available_height);

        // Render package table and handle callbacks
        render_package_table_mobile(
            ui,
            &vm_state.filtered_packages,
            &mut local_state.selected_packages,
            vm_state.uad_ng_lists.as_ref(),
            &app_metadata,
            local_state.unsafe_app_remove,
            local_state.expert_app_remove,
            &mut |pkg_id| {
                if let Some(idx) = vm_state
                    .filtered_packages
                    .iter()
                    .position(|p| p.pkg == pkg_id)
                {
                    local_state.package_details_dialog.selected_package_index = Some(idx);
                    local_state.package_details_dialog.open = true;
                }
            },
            &mut |_pkg_id, _is_enabled| {
                log::debug!("Toggle {}: {}", _pkg_id, _is_enabled);
            },
            &mut |_pkg_id| {
                log::debug!("Delete: {}", _pkg_id);
            },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mobile_view_module_exists() {
        // Ensure module compiles
        assert!(true);
    }
}

//! Shared filter logic for desktop and mobile debloat views

use eframe::egui;

use super::state::TabDebloatState;
use crate::viewmodel::ViewModelState;

/// Render category filter buttons (All, Recommended, Advanced, Expert, Unsafe)
///
/// Updates `local_state.active_filter.category_filter` and calls ViewModel
/// to apply the filter immediately.
pub fn render_category_filters(
    ui: &mut egui::Ui,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
) {
    ui.label("Category");

    ui.horizontal_wrapped(|ui| {
        if ui
            .selectable_label(
                local_state.active_filter.category_filter.is_none(),
                format!("All ({}/{})", local_state.cached_counts.all_enabled, local_state.cached_counts.all),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = None;
            ui.ctx().request_repaint(); // Force UI update

            // Apply filter immediately via ViewModel
            let text_filter = if local_state.applied_filter_text.is_empty() {
                None
            } else {
                Some(local_state.applied_filter_text.clone())
            };

            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                None,
                local_state.active_filter.show_only_enabled,
                local_state.active_filter.hide_system_apps,
            ) {
                log::error!("Failed to apply 'All' filter: {}", e);
            } else {
                log::debug!("Applied category filter: All");
            }
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("recommended"),
                format!("Recommended ({}/{})", local_state.cached_counts.recommended_enabled, local_state.cached_counts.recommended),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("recommended".to_string());
            ui.ctx().request_repaint(); // Force UI update

            // Apply filter immediately via ViewModel
            let text_filter = if local_state.applied_filter_text.is_empty() {
                None
            } else {
                Some(local_state.applied_filter_text.clone())
            };

            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                Some("recommended".to_string()),
                local_state.active_filter.show_only_enabled,
                local_state.active_filter.hide_system_apps,
            ) {
                log::error!("Failed to apply 'Recommended' filter: {}", e);
            } else {
                log::debug!("Applied category filter: recommended");
            }
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("advanced"),
                format!("Advanced ({}/{})", local_state.cached_counts.advanced_enabled, local_state.cached_counts.advanced),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("advanced".to_string());

            // Apply filter immediately via ViewModel
            let text_filter = if local_state.applied_filter_text.is_empty() {
                None
            } else {
                Some(local_state.applied_filter_text.clone())
            };

            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                Some("advanced".to_string()),
                local_state.active_filter.show_only_enabled,
                local_state.active_filter.hide_system_apps,
            ) {
                log::error!("Failed to apply 'Advanced' filter: {}", e);
            } else {
                log::debug!("Applied category filter: advanced");
            }
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("expert"),
                format!("Expert ({}/{})", local_state.cached_counts.expert_enabled, local_state.cached_counts.expert),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("expert".to_string());

            // Apply filter immediately via ViewModel
            let text_filter = if local_state.applied_filter_text.is_empty() {
                None
            } else {
                Some(local_state.applied_filter_text.clone())
            };

            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                Some("expert".to_string()),
                local_state.active_filter.show_only_enabled,
                local_state.active_filter.hide_system_apps,
            ) {
                log::error!("Failed to apply 'Expert' filter: {}", e);
            } else {
                log::debug!("Applied category filter: expert");
            }
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("unsafe"),
                format!("Unsafe ({}/{})", local_state.cached_counts.unsafe_apps_enabled, local_state.cached_counts.unsafe_apps),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("unsafe".to_string());

            // Apply filter immediately via ViewModel
            let text_filter = if local_state.applied_filter_text.is_empty() {
                None
            } else {
                Some(local_state.applied_filter_text.clone())
            };

            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                Some("unsafe".to_string()),
                local_state.active_filter.show_only_enabled,
                local_state.active_filter.hide_system_apps,
            ) {
                log::error!("Failed to apply 'Unsafe' filter: {}", e);
            } else {
                log::debug!("Applied category filter: unsafe");
            }
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("unknown"),
                format!("Unknown ({}/{})", local_state.cached_counts.unknown_apps_enabled, local_state.cached_counts.unknown_apps),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("unknown".to_string());

            // Apply filter immediately via ViewModel
            let text_filter = if local_state.applied_filter_text.is_empty() {
                None
            } else {
                Some(local_state.applied_filter_text.clone())
            };

            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                Some("unknown".to_string()),
                local_state.active_filter.show_only_enabled,
                local_state.active_filter.hide_system_apps,
            ) {
                log::error!("Failed to apply 'Unknown' filter: {}", e);
            } else {
                log::debug!("Applied category filter: unknown");
            }
        }
    });
}

/// Render options checkboxes (Show only enabled, Hide system apps)
pub fn render_options_checkboxes(
    ui: &mut egui::Ui,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
) {
    ui.label("Options");

    if ui
        .checkbox(
            &mut local_state.active_filter.show_only_enabled,
            "Show only enabled",
        )
        .changed()
    {
        // Apply filter immediately via ViewModel
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
            log::error!("Failed to apply 'Show only enabled' filter: {}", e);
        } else {
            log::debug!(
                "Applied 'Show only enabled' filter: {}",
                local_state.active_filter.show_only_enabled
            );
        }
    }

    if ui
        .checkbox(
            &mut local_state.active_filter.hide_system_apps,
            "Hide system apps",
        )
        .changed()
    {
        // Apply filter immediately via ViewModel
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
            log::error!("Failed to apply 'Hide system apps' filter: {}", e);
        } else {
            log::debug!(
                "Applied 'Hide system apps' filter: {}",
                local_state.active_filter.hide_system_apps
            );
        }
    }
}

/// Render advanced settings checkboxes (Unsafe removal, Expert removal)
pub fn render_advanced_settings(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
    ui.label("Advanced");

    ui.checkbox(&mut local_state.unsafe_app_remove, "Unsafe removal");
    ui.checkbox(&mut local_state.expert_app_remove, "Expert removal");
}

/// Render device info and package counts
pub fn render_package_counts(ui: &mut egui::Ui, vm_state: &ViewModelState, local_state: &TabDebloatState) {
    // Device info (if available)
    if let Some(device) = &local_state.selected_device {
        ui.separator();
        ui.label("Device");
        ui.label(device);
        ui.add_space(8.0);
    }

    // Package counts
    ui.separator();
    ui.label(format!("Total packages: {}", vm_state.packages.len()));
    ui.label(format!("Filtered: {}", vm_state.filtered_packages.len()));
}

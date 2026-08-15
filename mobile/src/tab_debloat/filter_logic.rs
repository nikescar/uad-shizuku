//! Shared filter logic for desktop and mobile debloat views

use eframe::egui;

use super::state::TabDebloatState;
use crate::viewmodel::ViewModelState;

/// Render category filter buttons (All, Recommended, Advanced, Expert, Unsafe)
///
/// Updates `local_state.active_filter.category_filter` and increments `table_version`
/// when selection changes.
pub fn render_category_filters(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
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
            local_state.table_version += 1;
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("recommended"),
                format!("Recommended ({}/{})", local_state.cached_counts.recommended_enabled, local_state.cached_counts.recommended),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("recommended".to_string());
            local_state.table_version += 1;
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("advanced"),
                format!("Advanced ({}/{})", local_state.cached_counts.advanced_enabled, local_state.cached_counts.advanced),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("advanced".to_string());
            local_state.table_version += 1;
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("expert"),
                format!("Expert ({}/{})", local_state.cached_counts.expert_enabled, local_state.cached_counts.expert),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("expert".to_string());
            local_state.table_version += 1;
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("unsafe"),
                format!("Unsafe ({}/{})", local_state.cached_counts.unsafe_apps_enabled, local_state.cached_counts.unsafe_apps),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("unsafe".to_string());
            local_state.table_version += 1;
        }
    });
}

/// Render options checkboxes (Show only enabled, Hide system apps)
pub fn render_options_checkboxes(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
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

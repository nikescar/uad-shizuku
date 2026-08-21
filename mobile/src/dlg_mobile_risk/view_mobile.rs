//! Render entry point for the mobile IzzyRisk/Stalkerware drill-down.
//!
//! Mirrors `tab_debloat::view_mobile`'s shape: filter row, then a virtualized table, then
//! the dialogs it owns. Unlike debloat, filtering here is local/synchronous (no ViewModel
//! filter command) since IzzyRisk/Stalkerware aren't part of the debloat filter pipeline.

use eframe::egui;
use std::collections::{HashMap, HashSet};

use super::components::package_table_mobile::render_risk_table_mobile;
use super::filter_logic;
use super::state::RiskTableState;
use super::RiskCategory;
use crate::adb::PackageFingerprint;
use crate::viewmodel::ViewModelState;

#[allow(clippy::too_many_arguments)]
pub fn render(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    vm_state: &ViewModelState,
    local_state: &mut RiskTableState,
    viewmodel: &crate::viewmodel::ViewModel,
    installed_packages: &[PackageFingerprint],
    package_risk_scores: &HashMap<String, i32>,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) {
    let Some(category) = local_state.category.clone() else {
        ui.label("No category selected");
        return;
    };

    render_filter_row(ui, local_state);
    ui.add_space(8.0);

    let is_izzyrisk = matches!(
        category,
        RiskCategory::IzzyRiskSafe
            | RiskCategory::IzzyRiskNormal
            | RiskCategory::IzzyRiskModerate
            | RiskCategory::IzzyRiskHigh
    );

    let filtered_packages: Vec<&PackageFingerprint> = installed_packages
        .iter()
        .filter(|pkg| {
            let matches_category = if is_izzyrisk {
                match package_risk_scores.get(&pkg.pkg) {
                    Some(&score) => filter_logic::matches_izzyrisk_category(&category, score),
                    None => false,
                }
            } else {
                match &vm_state.stalkerware_indicators {
                    Some(indicators) => {
                        let is_stalkerware = indicators.is_stalkerware(&pkg.pkg);
                        (category == RiskCategory::StalkerwareDetected) == is_stalkerware
                    }
                    None => false,
                }
            };

            matches_category
                && filter_logic::should_show_package(
                    pkg,
                    local_state.show_only_enabled,
                    local_state.hide_system_app,
                )
                && filter_logic::matches_text_filter(&local_state.text_filter, pkg, vm_state)
        })
        .collect();

    let package_ids: Vec<String> = filtered_packages.iter().map(|p| p.pkg.clone()).collect();
    let system_packages: HashSet<String> = installed_packages
        .iter()
        .filter(|p| p.flags.contains("SYSTEM"))
        .map(|p| p.pkg.clone())
        .collect();

    let full_metadata = crate::app_metadata_renderer::prepare_app_info_for_display(
        ctx,
        &package_ids,
        &system_packages,
        vm_state,
        viewmodel,
        google_play_enabled,
        fdroid_enabled,
        apkmirror_enabled,
        android_package_enabled,
    );
    let app_metadata: crate::tab_debloat::components::package_table_mobile::AppDisplayData =
        full_metadata
            .iter()
            .map(|(pkg_id, (texture, title, _developer, _version))| {
                (pkg_id.clone(), (texture.clone(), title.clone()))
            })
            .collect();

    egui::ScrollArea::vertical()
        .id_salt("risk_table_mobile_scroll")
        .show(ui, |ui| {
            render_risk_table_mobile(
                ui,
                &filtered_packages,
                &category,
                package_risk_scores,
                vm_state.uad_ng_lists.as_ref(),
                &app_metadata,
                unsafe_app_remove,
                expert_app_remove,
                &mut |pkg_id| {
                    if let Some(idx) = installed_packages.iter().position(|p| p.pkg == pkg_id) {
                        local_state.mobile_info_dialog.open(idx);
                    }
                },
                &mut |pkg_id, is_enabled| {
                    let key = if is_enabled {
                        "disable_clicked_package"
                    } else {
                        "enable_clicked_package"
                    };
                    ctx.data_mut(|data| {
                        data.insert_temp(egui::Id::new(key), pkg_id.to_string());
                    });
                },
                &mut |pkg_id| {
                    if let Some(package) = installed_packages.iter().find(|p| p.pkg == pkg_id) {
                        let is_system = package.flags.contains("SYSTEM");
                        local_state
                            .uninstall_confirm_dialog
                            .open_single(pkg_id.to_string(), is_system);
                    }
                },
            );
        });

    local_state
        .mobile_info_dialog
        .show(ctx, installed_packages, &vm_state.uad_ng_lists);

    if local_state.uninstall_confirm_dialog.show(ctx) {
        if let (Some(pkg_id), Some(&is_system)) = (
            local_state
                .uninstall_confirm_dialog
                .packages
                .first()
                .cloned(),
            local_state.uninstall_confirm_dialog.is_system.first(),
        ) {
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("uninstall_clicked_package"), pkg_id);
                data.insert_temp(egui::Id::new("uninstall_clicked_is_system"), is_system);
            });
        }
    }
}

fn render_filter_row(ui: &mut egui::Ui, local_state: &mut RiskTableState) {
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut local_state.show_only_enabled, "Show only enabled");
        ui.add_space(10.0);
        ui.checkbox(&mut local_state.hide_system_app, "Hide system apps");
        ui.add_space(10.0);
        ui.label("Filter:");
        let response = ui.add(
            egui::TextEdit::singleline(&mut local_state.text_filter)
                .hint_text("Search...")
                .desired_width(200.0),
        );
        #[cfg(target_os = "android")]
        {
            if response.gained_focus() {
                let _ = crate::android_inputmethod::show_soft_input();
            }
            if response.lost_focus() {
                let _ = crate::android_inputmethod::hide_soft_input();
            }
        }
        crate::clipboard_popup::show_clipboard_popup(ui, &response, &mut local_state.text_filter);
        if !local_state.text_filter.is_empty() && ui.button("X").clicked() {
            local_state.text_filter.clear();
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_view_mobile_module_compiles() {
        assert!(true);
    }
}

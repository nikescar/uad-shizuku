//! Mobile-optimized risk table component (IzzyRisk / Stalkerware).
//!
//! 2 columns: Name/Status (+ risk score/permissions for IzzyRisk) + Tasks.
//! Modeled on `tab_debloat::components::package_table_mobile`, minus the checkbox column
//! (neither risk table has batch-select) and with an extra secondary line for IzzyRisk rows.

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::HashMap;

use crate::adb_stt::PackageFingerprint;
use crate::dlg_mobile_risk::RiskCategory;
use crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::tab_debloat::components::package_table_mobile::AppDisplayData;
use crate::uad_shizuku_app::UadNgLists;
use egui_material3::icon_button_standard;

// Taller than debloat's 56.0: IzzyRisk rows carry a third text line (risk score/permissions).
const ROW_HEIGHT: f32 = 64.0;
const TASKS_COLUMN_WIDTH: f32 = 200.0;
const MOBILE_BUTTON_SPACING: f32 = 16.0;
const MOBILE_TOUCH_TARGET: f32 = 40.0;

fn is_izzyrisk(category: &RiskCategory) -> bool {
    matches!(
        category,
        RiskCategory::IzzyRiskSafe
            | RiskCategory::IzzyRiskNormal
            | RiskCategory::IzzyRiskModerate
            | RiskCategory::IzzyRiskHigh
    )
}

/// Secondary text line shown under the title for IzzyRisk rows only.
fn izzyrisk_secondary_line(
    pkg_id: &str,
    package_risk_scores: &HashMap<String, i32>,
    permissions_count: usize,
) -> String {
    let risk_score = package_risk_scores.get(pkg_id).copied().unwrap_or(0);
    format!("Risk {} \u{b7} {} perms", risk_score, permissions_count)
}

fn is_row_enabled(package: &PackageFingerprint) -> bool {
    package.users.first().map_or(false, |user| {
        let enabled = user.enabled;
        let installed = user.installed;
        let is_system = package.flags.contains("SYSTEM");
        !(enabled == 0 && !installed && is_system || enabled == 2 || enabled == 3)
    })
}

/// Whether the delete/uninstall button should show for this package, gated by the
/// Unsafe/Expert removal toggles — mirrors `render_action_buttons_static`'s gating
/// (dlg_dashcounter_details.rs:806-808).
fn show_delete_button(
    pkg_id: &str,
    uad_ng_lists: Option<&UadNgLists>,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
) -> bool {
    let category = uad_ng_lists
        .and_then(|lists| lists.apps.get(pkg_id))
        .map(|app| app.removal.as_str());

    match category {
        Some("Unsafe") => unsafe_app_remove,
        Some("Expert") => expert_app_remove,
        _ => true,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_risk_table_mobile(
    ui: &mut egui::Ui,
    packages: &[&PackageFingerprint],
    category: &RiskCategory,
    package_risk_scores: &HashMap<String, i32>,
    uad_ng_lists: Option<&UadNgLists>,
    app_display_data: &AppDisplayData,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
    on_info_clicked: &mut dyn FnMut(&str),
    on_toggle_clicked: &mut dyn FnMut(&str, bool),
    on_delete_clicked: &mut dyn FnMut(&str),
) {
    let show_risk_line = is_izzyrisk(category);

    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::remainder())
        .column(Column::exact(TASKS_COLUMN_WIDTH))
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.label("Name");
            });
            header.col(|ui| {
                ui.label("Tasks");
            });
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, packages.len(), |mut row| {
                let package = packages[row.index()];

                // Column 1: Name/Status (+ risk line for IzzyRisk)
                row.col(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let (texture_handle, app_title) = app_display_data
                            .get(&package.pkg)
                            .map(|(tex, title)| (tex.as_ref(), Some(title.as_str())))
                            .unwrap_or((None, None));

                        if let Some(tex) = texture_handle {
                            ui.image((tex.id(), egui::vec2(38.0, 38.0)));
                        }

                        ui.vertical(|ui| {
                            ui.style_mut().spacing.item_spacing.y = 2.0;

                            if let Some(title) = app_title {
                                let text_color = ui.style().visuals.text_color();
                                ui.label(egui::RichText::new(title).strong().color(text_color));
                                ui.label(egui::RichText::new(&package.pkg).small().weak());
                            } else {
                                ui.label(&package.pkg);
                            }

                            let (status_text, status_color) = if package.users.is_empty() {
                                ("Uninstalled", egui::Color32::from_rgb(128, 128, 128))
                            } else {
                                let user = &package.users[0];
                                let is_system = package.flags.contains("SYSTEM");
                                if user.enabled == 0 && !user.installed && is_system {
                                    ("Removed", egui::Color32::from_rgb(158, 158, 158))
                                } else if user.enabled == 2 {
                                    ("Disabled", egui::Color32::from_rgb(211, 47, 47))
                                } else if user.enabled == 3 {
                                    ("Disabled-User", egui::Color32::from_rgb(244, 67, 54))
                                } else {
                                    ("Enabled", egui::Color32::from_rgb(56, 142, 60))
                                }
                            };
                            ui.label(egui::RichText::new(status_text).color(status_color));

                            if show_risk_line {
                                let permissions_count = package.installPermissions.len();
                                ui.label(
                                    egui::RichText::new(izzyrisk_secondary_line(
                                        &package.pkg,
                                        package_risk_scores,
                                        permissions_count,
                                    ))
                                    .small()
                                    .weak(),
                                );
                            }
                        });
                    });
                });

                // Column 2: Tasks (info / toggle / uninstall)
                row.col(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = MOBILE_BUTTON_SPACING;
                        ui.style_mut().spacing.interact_size =
                            egui::vec2(MOBILE_TOUCH_TARGET, MOBILE_TOUCH_TARGET);

                        if ui
                            .add(icon_button_standard(ICON_INFO.to_string()))
                            .on_hover_text("Package details")
                            .clicked()
                        {
                            on_info_clicked(&package.pkg);
                        }

                        let is_enabled = is_row_enabled(package);
                        let toggle_icon = if is_enabled {
                            ICON_TOGGLE_ON
                        } else {
                            ICON_TOGGLE_OFF
                        };
                        let toggle_text = if is_enabled { "Disable" } else { "Enable" };

                        if ui
                            .add(icon_button_standard(toggle_icon.to_string()))
                            .on_hover_text(toggle_text)
                            .clicked()
                        {
                            on_toggle_clicked(&package.pkg, is_enabled);
                        }

                        if show_delete_button(
                            &package.pkg,
                            uad_ng_lists,
                            unsafe_app_remove,
                            expert_app_remove,
                        ) {
                            if ui
                                .add(icon_button_standard(ICON_DELETE.to_string()))
                                .on_hover_text("Uninstall package")
                                .clicked()
                            {
                                on_delete_clicked(&package.pkg);
                            }
                        }
                    });
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(ROW_HEIGHT, 64.0);
        assert_eq!(TASKS_COLUMN_WIDTH, 200.0);
        assert_eq!(MOBILE_BUTTON_SPACING, 16.0);
        assert_eq!(MOBILE_TOUCH_TARGET, 40.0);
    }

    #[test]
    fn test_is_izzyrisk() {
        assert!(is_izzyrisk(&RiskCategory::IzzyRiskSafe));
        assert!(is_izzyrisk(&RiskCategory::IzzyRiskHigh));
        assert!(!is_izzyrisk(&RiskCategory::StalkerwareDetected));
        assert!(!is_izzyrisk(&RiskCategory::StalkerwareUndetected));
    }

    #[test]
    fn test_izzyrisk_secondary_line_formatting() {
        let mut scores = HashMap::new();
        scores.insert("com.example.app".to_string(), 15);
        assert_eq!(
            izzyrisk_secondary_line("com.example.app", &scores, 4),
            "Risk 15 \u{b7} 4 perms"
        );
    }

    #[test]
    fn test_izzyrisk_secondary_line_defaults_to_zero_score() {
        let scores = HashMap::new();
        assert_eq!(
            izzyrisk_secondary_line("com.unknown.app", &scores, 0),
            "Risk 0 \u{b7} 0 perms"
        );
    }

    #[test]
    fn test_show_delete_button_defaults_true_with_no_uad_lists() {
        assert!(show_delete_button("com.example.app", None, false, false));
        assert!(show_delete_button("com.example.app", None, true, true));
    }
}

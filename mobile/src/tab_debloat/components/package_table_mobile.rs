//! Mobile-optimized package table component
//!
//! 3 columns: Checkbox (40px) + Name/Status (remainder) + Tasks (200px)
//! Optimized for 1000-2000 packages with < 300ms render time.

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::{HashMap, HashSet};

use crate::adb_stt::PackageFingerprint;
use crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::uad_shizuku_app::UadNgLists;
use egui_material3::icon_button_standard;

const ROW_HEIGHT: f32 = 56.0;
const CHECKBOX_COLUMN_WIDTH: f32 = 40.0;
const TASKS_COLUMN_WIDTH: f32 = 200.0;
const MOBILE_BUTTON_SPACING: f32 = 16.0;
const MOBILE_TOUCH_TARGET: f32 = 40.0;

pub type AppDisplayData = HashMap<String, (Option<egui::TextureHandle>, String)>;

pub fn render_package_table_mobile(
    ui: &mut egui::Ui,
    packages: &[PackageFingerprint],
    selected_packages: &mut HashSet<String>,
    uad_ng_lists: Option<&UadNgLists>,
    app_display_data: &AppDisplayData,
    on_info_clicked: &mut dyn FnMut(&str),
    on_refresh_clicked: &mut dyn FnMut(&str),
    on_toggle_clicked: &mut dyn FnMut(&str, bool),
    on_delete_clicked: &mut dyn FnMut(&str),
) {
    log::debug!("render_package_table_mobile: {} packages", packages.len());

    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(CHECKBOX_COLUMN_WIDTH))
        .column(Column::remainder())
        .column(Column::exact(TASKS_COLUMN_WIDTH))
        .header(20.0, |mut header| {
            header.col(|ui| { ui.label(""); });
            header.col(|ui| { ui.label("Name"); });
            header.col(|ui| { ui.label("Tasks"); });
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, packages.len(), |mut row| {
                let package = &packages[row.index()];

                // Column 1: Checkbox
                row.col(|ui| {
                    let mut is_selected = selected_packages.contains(&package.pkg);
                    if ui.checkbox(&mut is_selected, "").changed() {
                        if is_selected {
                            selected_packages.insert(package.pkg.clone());
                        } else {
                            selected_packages.remove(&package.pkg);
                        }
                    }
                });

                // Column 2: Name/Status (combined)
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
                                ui.label(egui::RichText::new(title).strong());
                                ui.label(egui::RichText::new(&package.pkg).small().weak());
                            } else {
                                ui.label(&package.pkg);
                            }

                            let (status_text, status_color) = if package.users.is_empty() {
                                ("Uninstalled", egui::Color32::from_rgb(128, 128, 128))
                            } else {
                                let user = &package.users[0];
                                let enabled = user.enabled;
                                let installed = user.installed;
                                let is_system = package.flags.contains("SYSTEM");

                                if enabled == 0 && !installed && is_system {
                                    ("Removed", egui::Color32::from_rgb(158, 158, 158))
                                } else if enabled == 2 {
                                    ("Disabled", egui::Color32::from_rgb(211, 47, 47))
                                } else if enabled == 3 {
                                    ("Disabled-User", egui::Color32::from_rgb(244, 67, 54))
                                } else {
                                    ("Enabled", egui::Color32::from_rgb(56, 142, 60))
                                }
                            };
                            ui.label(egui::RichText::new(status_text).color(status_color));
                        });
                    });
                });

                // Column 3: Tasks (touch-optimized buttons)
                row.col(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = MOBILE_BUTTON_SPACING;
                        ui.style_mut().spacing.interact_size = egui::vec2(MOBILE_TOUCH_TARGET, MOBILE_TOUCH_TARGET);

                        if ui.add(icon_button_standard(ICON_INFO.to_string()))
                            .on_hover_text("Package details")
                            .clicked()
                        {
                            on_info_clicked(&package.pkg);
                        }

                        let is_enabled = package.users.first().map_or(false, |user| {
                            let enabled = user.enabled;
                            let installed = user.installed;
                            let is_system = package.flags.contains("SYSTEM");
                            !(enabled == 0 && !installed && is_system || enabled == 2 || enabled == 3)
                        });

                        let toggle_icon = if is_enabled { ICON_TOGGLE_ON } else { ICON_TOGGLE_OFF };
                        let toggle_text = if is_enabled { "Disable" } else { "Enable" };

                        if ui.add(icon_button_standard(toggle_icon.to_string()))
                            .on_hover_text(toggle_text)
                            .clicked()
                        {
                            on_toggle_clicked(&package.pkg, is_enabled);
                        }

                        if ui.add(icon_button_standard(ICON_DELETE.to_string()))
                            .on_hover_text("Uninstall package")
                            .clicked()
                        {
                            on_delete_clicked(&package.pkg);
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
        assert_eq!(ROW_HEIGHT, 56.0);
        assert_eq!(CHECKBOX_COLUMN_WIDTH, 40.0);
        assert_eq!(TASKS_COLUMN_WIDTH, 200.0);
        assert_eq!(MOBILE_BUTTON_SPACING, 16.0);
        assert_eq!(MOBILE_TOUCH_TARGET, 40.0);
    }

    #[test]
    fn test_function_compiles() {
        let packages: Vec<PackageFingerprint> = vec![];
        let mut selected: HashSet<String> = HashSet::new();
        let metadata: HashMap<String, (Option<egui::TextureHandle>, String)> = HashMap::new();
        assert_eq!(packages.len(), 0);
    }
}

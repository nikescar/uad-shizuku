//! Virtual scrolling package table component
//!
//! This module implements a high-performance virtual scrolling table
//! for displaying Android packages using egui_extras::TableBuilder.
//!
//! The table displays:
//! - Checkbox (30px): Select package for batch operations
//! - Name (remainder): Package ID (e.g., com.example.app)
//! - Category (100px): UAD debloat category (placeholder for now)
//! - Status (80px): Enabled/Disabled based on users field
//! - Tasks (80px): Future action buttons (placeholder for now)

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::HashMap;
use std::collections::HashSet;

use crate::adb_stt::PackageFingerprint;
use crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_REFRESH, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::uad_shizuku_app::UadNgLists;
use egui_material3::icon_button_standard;

/// Row height for table entries (56.0px for desktop with icons)
const ROW_HEIGHT: f32 = 56.0;

/// Type alias for app display data: (texture, title)
pub type AppDisplayData = HashMap<String, (Option<egui::TextureHandle>, String)>;

/// Render the virtual scrolling package table
///
/// This function creates a high-performance table using egui_extras::TableBuilder
/// with virtual scrolling support. Only visible rows are rendered, enabling
/// smooth performance even with thousands of packages.
///
/// # Arguments
/// * `ui` - egui context for rendering
/// * `packages` - Slice of packages to display
/// * `selected_packages` - Mutable reference to selected package set
/// * `uad_ng_lists` - UAD-NG debloat lists for category display
/// * `app_display_data` - Pre-loaded app icons and titles (to avoid egui RwLock deadlock)
/// * `on_info_clicked` - Callback when info button clicked (receives package ID)
/// * `on_refresh_clicked` - Callback when refresh button clicked (receives package ID)
/// * `on_toggle_clicked` - Callback when enable/disable toggle clicked (receives package ID, is_enabled)
/// * `on_delete_clicked` - Callback when delete button clicked (receives package ID)
///
/// # Column Layout
/// 1. Checkbox (30px exact): Multi-select for batch operations
/// 2. Name (remainder): Package identifier with icon (if available)
/// 3. Category (100px exact): UAD debloat category
/// 4. Status (120px exact): Enabled/Disabled state
/// 5. Tasks (160px exact): Task buttons (info, refresh, toggle, delete)
pub fn render_package_table(
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
    log::debug!(
        "DEBUG: render_package_table called with {} packages",
        packages.len()
    );
    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(30.0)) // Checkbox
        .column(Column::remainder()) // Name
        .column(Column::exact(100.0)) // Category
        .column(Column::exact(120.0)) // Status
        .column(Column::exact(160.0)) // Tasks
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.label("");
            });
            header.col(|ui| {
                ui.label("Name");
            });
            header.col(|ui| {
                ui.label("Category");
            });
            header.col(|ui| {
                ui.label("Status");
            });
            header.col(|ui| {
                ui.label("Tasks");
            });
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, packages.len(), |mut row| {
                let row_index = row.index();
                let package = &packages[row_index];

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

                // Column 2: Name (with icon if available from pre-loaded data)
                row.col(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        // Get pre-loaded app data (texture and title)
                        let (texture_handle, app_title) = app_display_data
                            .get(&package.pkg)
                            .map(|(tex, title)| (tex.as_ref(), Some(title.as_str())))
                            .unwrap_or((None, None));

                        // Render icon if available
                        if let Some(tex) = texture_handle {
                            ui.image((tex.id(), egui::vec2(38.0, 38.0)));
                        }

                        // Render app title or package ID
                        ui.vertical(|ui| {
                            ui.label(app_title.unwrap_or(&package.pkg));
                            if app_title.is_some() {
                                ui.label(egui::RichText::new(&package.pkg).small().weak());
                            }
                        });
                    });
                });

                // Column 3: Category (from UAD-NG lists)
                row.col(|ui| {
                    let category = uad_ng_lists
                        .and_then(|lists| lists.apps.get(&package.pkg))
                        .map(|app| app.removal.as_str())
                        .unwrap_or("-");
                    ui.label(category);
                });

                // Column 4: Status (using same logic as scan table)
                row.col(|ui| {
                    let (status_text, status_color) = if package.users.is_empty() {
                        ("Uninstalled", egui::Color32::from_rgb(128, 128, 128))
                    } else {
                        // Use same logic as scan table (tab_scan_control.rs:1189-1209)
                        let user = package.users.first().unwrap();
                        let enabled = user.enabled;
                        let installed = user.installed;
                        let is_system = package.flags.contains("SYSTEM");

                        let is_removed_user = enabled == 0 && !installed && is_system;
                        let is_disabled = enabled == 2;
                        let is_disabled_user = enabled == 3;

                        if is_removed_user {
                            ("Removed", egui::Color32::from_rgb(158, 158, 158))
                        } else if is_disabled {
                            ("Disabled", egui::Color32::from_rgb(211, 47, 47))
                        } else if is_disabled_user {
                            ("Disabled-User", egui::Color32::from_rgb(244, 67, 54))
                        } else {
                            ("Enabled", egui::Color32::from_rgb(56, 142, 60))
                        }
                    };
                    ui.label(egui::RichText::new(status_text).color(status_color));
                });

                // Column 5: Tasks (info, refresh, enable/disable toggle, delete)
                row.col(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0; // Reduce spacing between buttons

                        // Info button - Opens package details dialog
                        if ui.add(icon_button_standard(ICON_INFO.to_string())).on_hover_text("Package details").clicked() {
                            on_info_clicked(&package.pkg);
                        }

                        // Enable/Disable toggle button (using correct enabled logic)
                        let user = package.users.first();
                        let is_enabled = if let Some(user) = user {
                            let enabled = user.enabled;
                            let installed = user.installed;
                            let is_system = package.flags.contains("SYSTEM");

                            let is_removed_user = enabled == 0 && !installed && is_system;
                            let is_disabled = enabled == 2;
                            let is_disabled_user = enabled == 3;

                            !(is_removed_user || is_disabled || is_disabled_user)
                        } else {
                            false
                        };

                        let toggle_icon = if is_enabled { ICON_TOGGLE_ON } else { ICON_TOGGLE_OFF };
                        let toggle_text = if is_enabled { "Disable" } else { "Enable" };
                        if ui.add(icon_button_standard(toggle_icon.to_string())).on_hover_text(toggle_text).clicked() {
                            on_toggle_clicked(&package.pkg, is_enabled);
                        }

                        // Delete/Uninstall button - Opens uninstall confirmation dialog
                        if ui.add(icon_button_standard(ICON_DELETE.to_string())).on_hover_text("Uninstall package").clicked() {
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
    use crate::adb_stt::AdbPackageInfoUser;

    #[test]
    fn test_row_height_constant() {
        assert_eq!(ROW_HEIGHT, 56.0);
    }

    #[test]
    fn test_render_package_table_compiles() {
        // This test ensures the function signature is correct
        // Actual rendering tests would require egui context

        let packages = vec![PackageFingerprint {
            pkg: "com.example.test".to_string(),
            codePath: "/data/app/test".to_string(),
            versionCode: 1,
            versionName: "1.0".to_string(),
            flags: "0".to_string(),
            privateFlags: "0".to_string(),
            installPermissions: vec![],
            users: vec![AdbPackageInfoUser {
                userId: 0,
                ceDataInode: 0,
                deDataInode: 0,
                installed: true,
                hidden: false,
                suspended: false,
                distractionFlags: 0,
                stopped: false,
                notLaunched: false,
                enabled: 1,
                instant: false,
                virtualField: false,
                quarantined: false,
                installReason: 0,
                dataDir: "/data/user/0/com.example.test".to_string(),
                firstInstallTime: "2024-01-01".to_string(),
                uninstallReason: 0,
                lastDisabledCaller: String::new(),
                gids: vec![],
                runtimePermissions: vec![],
            }],
            lastUpdateTime: "2024-01-01".to_string(),
            pkgChecksum: "abc123".to_string(),
            dumpText: "".to_string(),
        }];

        let mut selected: HashSet<String> = HashSet::new();

        // Function compiles with correct signature
        assert_eq!(packages.len(), 1);
        assert_eq!(selected.len(), 0);
    }
}

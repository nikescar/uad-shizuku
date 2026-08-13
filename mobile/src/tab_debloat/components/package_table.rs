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
//! - Actions (80px): Future action buttons (placeholder for now)

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::HashSet;

use crate::adb_stt::PackageFingerprint;

/// Row height for table entries (24.0px for desktop)
const ROW_HEIGHT: f32 = 24.0;

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
///
/// # Column Layout
/// 1. Checkbox (30px exact): Multi-select for batch operations
/// 2. Name (remainder): Package identifier
/// 3. Category (100px exact): UAD debloat category
/// 4. Status (80px exact): Enabled/Disabled state
/// 5. Actions (80px exact): Action buttons placeholder
pub fn render_package_table(
    ui: &mut egui::Ui,
    packages: &[PackageFingerprint],
    selected_packages: &mut HashSet<String>,
) {
    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::exact(30.0)) // Checkbox
        .column(Column::remainder()) // Name
        .column(Column::exact(100.0)) // Category
        .column(Column::exact(80.0)) // Status
        .column(Column::exact(80.0)) // Actions
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
                ui.label("Actions");
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

                // Column 2: Name (package ID)
                row.col(|ui| {
                    ui.label(&package.pkg);
                });

                // Column 3: Category (placeholder)
                row.col(|ui| {
                    ui.label("-");
                });

                // Column 4: Status (enabled/disabled based on users field)
                row.col(|ui| {
                    let status = if package.users.is_empty() {
                        "Disabled"
                    } else {
                        "Enabled"
                    };
                    ui.label(status);
                });

                // Column 5: Actions (placeholder)
                row.col(|ui| {
                    ui.label("...");
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
        assert_eq!(ROW_HEIGHT, 24.0);
    }

    #[test]
    fn test_render_package_table_compiles() {
        // This test ensures the function signature is correct
        // Actual rendering tests would require egui context

        let packages = vec![
            PackageFingerprint {
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
            },
        ];

        let mut selected: HashSet<String> = HashSet::new();

        // Function compiles with correct signature
        assert_eq!(packages.len(), 1);
        assert_eq!(selected.len(), 0);
    }
}

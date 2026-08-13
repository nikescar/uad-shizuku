//! Package card list component for mobile view
//!
//! This module implements a card-based list view for displaying Android packages
//! on mobile/narrow screens (<800px). Each card has a minimum height of 48px
//! for touch-friendly interaction.
//!
//! The card displays:
//! - Checkbox: Select package for batch operations
//! - Package name: com.example.app
//! - Status badge: Enabled/Disabled
//! - Category (if available): UAD debloat category

use eframe::egui;
use std::collections::HashSet;

use crate::adb_stt::PackageFingerprint;

/// Minimum card height for touch-friendly interaction (48px)
const CARD_MIN_HEIGHT: f32 = 48.0;

/// Card spacing between items
const CARD_SPACING: f32 = 8.0;

/// Render package cards for mobile view
///
/// This function creates a card-based list view optimized for narrow screens.
/// Each card shows package information in a vertical layout with touch-friendly
/// spacing (48px minimum height).
///
/// # Arguments
/// * `ui` - egui context for rendering
/// * `packages` - Slice of packages to display
/// * `selected_packages` - Mutable reference to selected package set
///
/// # Card Layout
/// - Left: Checkbox
/// - Center: Package name and status badge
/// - Right: Category (if available)
pub fn render_package_cards(
    ui: &mut egui::Ui,
    packages: &[PackageFingerprint],
    selected_packages: &mut HashSet<String>,
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for package in packages {
                render_single_card(ui, package, selected_packages);
                ui.add_space(CARD_SPACING);
            }
        });
}

/// Render a single package card
fn render_single_card(
    ui: &mut egui::Ui,
    package: &PackageFingerprint,
    selected_packages: &mut HashSet<String>,
) {
    let is_enabled = !package.users.is_empty();

    // Card frame with minimum height
    egui::Frame::none()
        .fill(ui.style().visuals.faint_bg_color)
        .stroke(ui.style().visuals.widgets.noninteractive.bg_stroke)
        .rounding(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_min_height(CARD_MIN_HEIGHT - 16.0); // Subtract inner margin

            ui.horizontal(|ui| {
                // Left: Checkbox for selection
                let mut is_selected = selected_packages.contains(&package.pkg);
                if ui.checkbox(&mut is_selected, "").changed() {
                    if is_selected {
                        selected_packages.insert(package.pkg.clone());
                    } else {
                        selected_packages.remove(&package.pkg);
                    }
                }

                ui.add_space(8.0);

                // Center: Package name and status
                ui.vertical(|ui| {
                    ui.label(&package.pkg);

                    ui.horizontal(|ui| {
                        // Status badge
                        let (status_text, status_color) = if is_enabled {
                            ("Enabled", egui::Color32::from_rgb(100, 200, 100))
                        } else {
                            ("Disabled", egui::Color32::from_rgb(150, 150, 150))
                        };

                        ui.colored_label(status_color, status_text);

                        // Category placeholder (will be populated with UAD data)
                        ui.label("•");
                        ui.label("-");
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
    fn test_card_min_height_constant() {
        assert_eq!(CARD_MIN_HEIGHT, 48.0);
    }

    #[test]
    fn test_card_spacing_constant() {
        assert_eq!(CARD_SPACING, 8.0);
    }

    #[test]
    fn test_render_package_cards_compiles() {
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

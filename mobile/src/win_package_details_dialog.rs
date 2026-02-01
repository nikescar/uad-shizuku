use crate::adb::PackageFingerprint;
use crate::uad_shizuku_app::UadNgLists;
pub use crate::win_package_details_dialog_stt::*;
use eframe::egui;
use egui_material3::MaterialButton;

impl PackageDetailsDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, package_index: usize) {
        self.selected_package_index = Some(package_index);
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        installed_packages: &[PackageFingerprint],
        uad_ng_lists: &Option<UadNgLists>,
    ) {
        if !self.open {
            return;
        }

        let Some(pkg_idx) = self.selected_package_index else {
            return;
        };

        let Some(package) = installed_packages.get(pkg_idx) else {
            return;
        };

        // Get UAD info if available
        let uad_info = uad_ng_lists
            .as_ref()
            .and_then(|lists| lists.apps.get(&package.pkg));

        let mut close_clicked = false;

        egui::Window::new(format!("Package Details: {}", package.pkg))
            .id(egui::Id::new("package_details_window"))
            .title_bar(false)
            .resizable(true)
            .collapsible(true)
            .scroll([false, true])
            .default_width(600.0)
            .max_width(ctx.screen_rect().width() - 40.0)
            .show(ctx, |ui| {
                ui.set_max_width(ctx.screen_rect().width() - 80.0);
                ui.add_space(8.0);

                // Basic package information
                ui.heading("Package Information");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Package:");
                    ui.label(&package.pkg);
                });

                if !package.versionName.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Version Name:");
                        ui.label(&package.versionName);
                    });
                }

                if !package.flags.is_empty() {
                    ui.label("Flags(Pkg Flags):");
                    // Parse flags from "[ FLAG1 FLAG2 FLAG3 ]" format
                    let flags_list: Vec<&str> = package
                        .flags
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .split_whitespace()
                        .filter(|s| !s.is_empty())
                        .collect();
                    for flag in flags_list {
                        ui.add(egui::Label::new(format!("  • {}", flag)).wrap());
                    }
                    ui.add_space(4.0);
                }

                if !package.privateFlags.is_empty() {
                    ui.label("Private Flags(Private Pkg Flags):");
                    let flags_list: Vec<&str> = package
                        .privateFlags
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .split_whitespace()
                        .filter(|s| !s.is_empty())
                        .collect();
                    for flag in flags_list {
                        ui.add(egui::Label::new(format!("  • {}", flag)).wrap());
                    }
                    ui.add_space(4.0);
                }

                if !package.lastUpdateTime.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Last Update Time:");
                        ui.label(&package.lastUpdateTime);
                    });
                }

                if !package.installPermissions.is_empty() {
                    ui.heading("Install Permissions");
                    ui.add_space(4.0);
                    // tracing::debug!("Displaying {} install permissions for package {}", package.installPermissions.len(), package.pkg);
                    ui.label(format!("Count: {}", package.installPermissions.len()));
                    for perm in &package.installPermissions {
                        ui.add(egui::Label::new(format!("  • {}", perm)).wrap());
                    }
                    ui.add_space(8.0);
                } else {
                    tracing::trace!("Package {} has no install permissions", package.pkg);
                }

                ui.add_space(12.0);

                // UAD Debloat Information
                if let Some(uad_entry) = uad_info {
                    ui.heading("Debloat Information");
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Removal Category:");
                        ui.label(&uad_entry.removal);
                    });

                    ui.horizontal(|ui| {
                        ui.label("List:");
                        ui.label(&uad_entry.list);
                    });

                    ui.label("Description:");
                    ui.add(egui::Label::new(&uad_entry.description).wrap());

                    if !uad_entry.dependencies.is_empty() {
                        ui.add_space(4.0);
                        ui.label("Dependencies:");
                        for dep in &uad_entry.dependencies {
                            ui.label(format!("  • {}", dep));
                        }
                    }

                    if !uad_entry.needed_by.is_empty() {
                        ui.add_space(4.0);
                        ui.label("Needed By:");
                        for needed in &uad_entry.needed_by {
                            ui.label(format!("  • {}", needed));
                        }
                    }

                    if !uad_entry.labels.is_empty() {
                        ui.add_space(4.0);
                        ui.label("Labels:");
                        ui.horizontal_wrapped(|ui| {
                            for label in &uad_entry.labels {
                                ui.label(format!("[{}]", label));
                            }
                        });
                    }

                    ui.add_space(12.0);
                }

                // User information
                if let Some(user) = package.users.get(0) {
                    ui.heading("User Information (User 0)");
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Installed:");
                        ui.label(if user.installed { "Yes" } else { "No" });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Hidden:");
                        ui.label(if user.hidden { "Yes" } else { "No" });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Suspended:");
                        ui.label(if user.suspended { "Yes" } else { "No" });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Stopped:");
                        ui.label(if user.stopped { "Yes" } else { "No" });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Not Launched:");
                        ui.label(if user.notLaunched { "Yes" } else { "No" });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Enabled:");
                        ui.label(format!(
                            "{} ({})",
                            Self::enabled_to_string(user.enabled),
                            user.enabled
                        ));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Install Reason:");
                        ui.label(format!(
                            "{} ({})",
                            Self::install_reason_to_string(user.installReason),
                            user.installReason
                        ));
                    });

                    if !user.dataDir.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("Data Dir:");
                            ui.add(egui::Label::new(&user.dataDir).wrap());
                        });
                    }

                    if !user.firstInstallTime.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("First Install Time:");
                            ui.label(&user.firstInstallTime);
                        });
                    }

                    if !user.runtimePermissions.is_empty() {
                        ui.add_space(4.0);
                        // tracing::debug!("Displaying {} runtime permissions for package {} user {}",
                        // user.runtimePermissions.len(), package.pkg, user.userId);
                        ui.label(format!(
                            "Runtime Permissions ({}):",
                            user.runtimePermissions.len()
                        ));
                        for perm in &user.runtimePermissions {
                            ui.add(egui::Label::new(format!("  • {}", perm)).wrap());
                        }
                    } else {
                        tracing::trace!(
                            "Package {} user {} has no runtime permissions",
                            package.pkg,
                            user.userId
                        );
                    }

                    ui.add_space(12.0);
                }

                // TODO: Libraries section disabled - fields not in PackageFingerprint
                // if !package.usesLibraries.is_empty() || !package.usesLibraryFiles.is_empty() {
                //     ui.heading("Libraries");
                //     ui.add_space(4.0);
                //
                //     if !package.usesLibraries.is_empty() {
                //         ui.label(format!("Uses Libraries ({}):", package.usesLibraries.len()));
                //         for lib in &package.usesLibraries {
                //             ui.add(egui::Label::new(format!("  • {}", lib)).wrap());
                //         }
                //         ui.add_space(4.0);
                //     }
                //
                //     if !package.usesLibraryFiles.is_empty() {
                //         ui.label(format!(
                //             "Uses Library Files ({}):",
                //             package.usesLibraryFiles.len()
                //         ));
                //         for file in &package.usesLibraryFiles {
                //             ui.add(egui::Label::new(format!("  • {}", file)).wrap());
                //         }
                //     }
                //
                //     ui.add_space(8.0);
                // }

                // TODO: Permissions section disabled - fields not in PackageFingerprint
                // if !package.declaredPermissions.is_empty() {
                //     ui.heading("Declared Permissions");
                //     ui.add_space(4.0);
                //     ui.label(format!(
                //         "Declared Permissions ({}):",
                //         package.declaredPermissions.len()
                //     ));
                //     for perm in &package.declaredPermissions {
                //         ui.add(egui::Label::new(format!("  • {}", perm)).wrap());
                //     }
                //     ui.add_space(8.0);
                // }
                //

                ui.add_space(8.0);

                // Add close button
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(MaterialButton::filled("Close")).clicked() {
                            close_clicked = true;
                        }
                    });
                });
            });

        // Handle close button click
        if close_clicked {
            self.close();
        }
    }

    fn enabled_to_string(enabled: i32) -> &'static str {
        match enabled {
            0 => "DEFAULT",
            1 => "ENABLED",
            2 => "DISABLED",
            3 => "DISABLED_USER",
            _ => "UNKNOWN",
        }
    }

    fn install_reason_to_string(install_reason: i32) -> &'static str {
        match install_reason {
            0 => "UNKNOWN",
            1 => "POLICY",
            2 => "DEVICE_RESTORE",
            3 => "DEVICE_SETUP",
            4 => "USER_REQUESTED",
            _ => "UNKNOWN",
        }
    }
}

use crate::adb::PackageFingerprint;
use crate::shared_store_stt::get_shared_store;
use crate::uad_shizuku_app::UadNgLists;
pub use crate::dlg_package_details_stt::*;
use eframe::egui;
use egui_material3::{MaterialButton, tabs_primary};

impl DlgPackageDetails {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, package_index: usize) {
        self.selected_package_index = Some(package_index);
        self.selected_tab = 0;
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

        let pkg_id = &package.pkg;
        let store = get_shared_store();

        // Check what data is available
        let has_uad = uad_ng_lists.as_ref().and_then(|lists| lists.apps.get(pkg_id)).is_some();
        let has_googleplay = store.get_cached_google_play_app(pkg_id).is_some();
        let has_fdroid = store.get_cached_fdroid_app(pkg_id).is_some();
        let has_apkmirror = store.get_cached_apkmirror_app(pkg_id).is_some();
        let has_virustotal = {
            let vt_state = store.get_vt_scanner_state();
            vt_state.and_then(|state| state.lock().ok().and_then(|s| s.get(pkg_id).cloned())).is_some()
        };
        let has_hybridanalysis = {
            let ha_state = store.get_ha_scanner_state();
            ha_state.and_then(|state| state.lock().ok().and_then(|s| s.get(pkg_id).cloned())).is_some()
        };

        let mut close_clicked = false;

        egui::Window::new(format!("Package Details: {}", pkg_id))
            .id(egui::Id::new("package_details_window"))
            .title_bar(false)
            .resizable(true)
            .collapsible(false)
            .scroll([false, false])
            .min_width(700.0)
            .min_height(500.0)
            .resize(|r| {
                r.default_size([ctx.screen_rect().width() - 40.0, ctx.screen_rect().height() - 40.0])
                    .max_size([ctx.screen_rect().width() - 40.0, ctx.screen_rect().height() - 40.0])
            })
            .show(ctx, |ui| {
                // Build tab labels dynamically
                let mut tabs = tabs_primary(&mut self.selected_tab)
                    .id_salt("package_details_tabs")
                    .tab("pkg");

                if has_uad {
                    tabs = tabs.tab("uad");
                }
                if has_googleplay {
                    tabs = tabs.tab("googleplay");
                }
                if has_fdroid {
                    tabs = tabs.tab("fdroid");
                }
                if has_apkmirror {
                    tabs = tabs.tab("apkmirror");
                }
                if has_virustotal {
                    tabs = tabs.tab("virustotal");
                }
                if has_hybridanalysis {
                    tabs = tabs.tab("hybridanalysis");
                }

                ui.add(tabs);
                ui.add_space(10.0);

                let max_height = ui.available_height() - 50.0; // Reserve space for close button

                // Map tab index to actual tab based on what's available
                let mut tab_index = 0;
                let mut selected_tab_type = "pkg";
                
                for tab_type in ["pkg", "uad", "googleplay", "fdroid", "apkmirror", "virustotal", "hybridanalysis"] {
                    match tab_type {
                        "pkg" => {
                            if self.selected_tab == tab_index {
                                selected_tab_type = "pkg";
                                break;
                            }
                            tab_index += 1;
                        }
                        "uad" if has_uad => {
                            if self.selected_tab == tab_index {
                                selected_tab_type = "uad";
                                break;
                            }
                            tab_index += 1;
                        }
                        "googleplay" if has_googleplay => {
                            if self.selected_tab == tab_index {
                                selected_tab_type = "googleplay";
                                break;
                            }
                            tab_index += 1;
                        }
                        "fdroid" if has_fdroid => {
                            if self.selected_tab == tab_index {
                                selected_tab_type = "fdroid";
                                break;
                            }
                            tab_index += 1;
                        }
                        "apkmirror" if has_apkmirror => {
                            if self.selected_tab == tab_index {
                                selected_tab_type = "apkmirror";
                                break;
                            }
                            tab_index += 1;
                        }
                        "virustotal" if has_virustotal => {
                            if self.selected_tab == tab_index {
                                selected_tab_type = "virustotal";
                                break;
                            }
                            tab_index += 1;
                        }
                        "hybridanalysis" if has_hybridanalysis => {
                            if self.selected_tab == tab_index {
                                selected_tab_type = "hybridanalysis";
                                break;
                            }
                            tab_index += 1;
                        }
                        _ => {}
                    }
                }

                egui::ScrollArea::both()
                    .id_salt("package_details_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        match selected_tab_type {
                            "pkg" => self.render_pkg_tab(ui, package),
                            "uad" => self.render_uad_tab(ui, pkg_id, uad_ng_lists),
                            "googleplay" => self.render_googleplay_tab(ui, pkg_id),
                            "fdroid" => self.render_fdroid_tab(ui, pkg_id),
                            "apkmirror" => self.render_apkmirror_tab(ui, pkg_id),
                            "virustotal" => self.render_virustotal_tab(ui, pkg_id),
                            "hybridanalysis" => self.render_hybridanalysis_tab(ui, pkg_id),
                            _ => {}
                        }
                    });

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

    fn render_pkg_tab(&self, ui: &mut egui::Ui, package: &PackageFingerprint) {
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

        ui.horizontal(|ui| {
            ui.label("Version Code:");
            ui.label(format!("{}", package.versionCode));
        });

        if !package.flags.is_empty() {
            ui.label("Flags(Pkg Flags):");
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
            ui.label(format!("Count: {}", package.installPermissions.len()));
            for perm in &package.installPermissions {
                ui.add(egui::Label::new(format!("  • {}", perm)).wrap());
            }
            ui.add_space(8.0);
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
                ui.label(format!(
                    "Runtime Permissions ({}):",
                    user.runtimePermissions.len()
                ));
                for perm in &user.runtimePermissions {
                    ui.add(egui::Label::new(format!("  • {}", perm)).wrap());
                }
            }
        }
    }

    fn render_uad_tab(&self, ui: &mut egui::Ui, pkg_id: &str, uad_ng_lists: &Option<UadNgLists>) {
        let uad_info = uad_ng_lists.as_ref().and_then(|lists| lists.apps.get(pkg_id));

        if let Some(uad_entry) = uad_info {
            ui.heading("UAD Debloat Information");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Package:");
                ui.label(pkg_id);
            });

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
        } else {
            ui.label("No UAD information available for this package");
        }
    }

    fn render_googleplay_tab(&self, ui: &mut egui::Ui, pkg_id: &str) {
        let store = get_shared_store();
        let app = store.get_cached_google_play_app(pkg_id);
        let texture = store.get_google_play_texture(pkg_id);

        if let Some(app) = app {
            ui.heading("Google Play Information");
            ui.add_space(4.0);

            // Show icon if available
            if let Some(texture) = texture {
                ui.image(&texture);
                ui.add_space(4.0);
            }

            ui.horizontal(|ui| {
                ui.label("Package ID:");
                ui.label(&app.package_id);
            });

            if !app.title.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.label(&app.title);
                });
            }

            if !app.developer.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Developer:");
                    ui.label(&app.developer);
                });
            }

            if let Some(version) = &app.version {
                ui.horizontal(|ui| {
                    ui.label("Version:");
                    ui.label(version);
                });
            }

            if let Some(score) = app.score {
                ui.horizontal(|ui| {
                    ui.label("Rating:");
                    ui.label(format!("{:.1}/5.0", score));
                });
            }

            if let Some(installs) = &app.installs {
                ui.horizontal(|ui| {
                    ui.label("Installs:");
                    ui.label(installs);
                });
            }

            if let Some(updated) = app.updated {
                ui.horizontal(|ui| {
                    ui.label("Last Updated:");
                    ui.label(format!("{}", updated));
                });
            }
        } else {
            ui.label("No Google Play information available for this package");
        }
    }

    fn render_fdroid_tab(&self, ui: &mut egui::Ui, pkg_id: &str) {
        let store = get_shared_store();
        let app = store.get_cached_fdroid_app(pkg_id);
        let texture = store.get_fdroid_texture(pkg_id);

        if let Some(app) = app {
            ui.heading("F-Droid Information");
            ui.add_space(4.0);

            // Show icon if available
            if let Some(texture) = texture {
                ui.image(&texture);
                ui.add_space(4.0);
            }

            ui.horizontal(|ui| {
                ui.label("Package ID:");
                ui.label(&app.package_id);
            });

            if !app.title.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.label(&app.title);
                });
            }

            if !app.developer.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Developer:");
                    ui.label(&app.developer);
                });
            }

            if let Some(version) = &app.version {
                ui.horizontal(|ui| {
                    ui.label("Version:");
                    ui.label(version);
                });
            }

            if let Some(license) = &app.license {
                ui.horizontal(|ui| {
                    ui.label("License:");
                    ui.label(license);
                });
            }

            if let Some(description) = &app.description {
                ui.add_space(4.0);
                ui.label("Description:");
                ui.add(egui::Label::new(description).wrap());
            }

            if let Some(updated) = app.updated {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Last Updated:");
                    ui.label(format!("{}", updated));
                });
            }
        } else {
            ui.label("No F-Droid information available for this package");
        }
    }

    fn render_apkmirror_tab(&self, ui: &mut egui::Ui, pkg_id: &str) {
        let store = get_shared_store();
        let app = store.get_cached_apkmirror_app(pkg_id);
        let texture = store.get_apkmirror_texture(pkg_id);

        if let Some(app) = app {
            ui.heading("APKMirror Information");
            ui.add_space(4.0);

            // Show icon if available
            if let Some(texture) = texture {
                ui.image(&texture);
                ui.add_space(4.0);
            }

            ui.horizontal(|ui| {
                ui.label("Package ID:");
                ui.label(&app.package_id);
            });

            if !app.title.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Title:");
                    ui.label(&app.title);
                });
            }

            if !app.developer.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Developer:");
                    ui.label(&app.developer);
                });
            }

            if let Some(version) = &app.version {
                ui.horizontal(|ui| {
                    ui.label("Version:");
                    ui.label(version);
                });
            }

            if let Some(icon_url) = &app.icon_url {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Icon URL:");
                    ui.add(egui::Label::new(icon_url).wrap());
                });
            }
        } else {
            ui.label("No APKMirror information available for this package");
        }
    }

    fn render_virustotal_tab(&self, ui: &mut egui::Ui, pkg_id: &str) {
        let store = get_shared_store();
        let vt_state = store.get_vt_scanner_state();
        
        if let Some(state) = vt_state {
            if let Ok(scanner_state) = state.lock() {
                if let Some(scan_status) = scanner_state.get(pkg_id) {
                    ui.heading("VirusTotal Scan Results");
                    ui.add_space(4.0);

                    match scan_status {
                        crate::calc_virustotal_stt::ScanStatus::Completed(result) => {
                            ui.label(format!("Files scanned: {} of {}", 
                                result.file_results.len(), 
                                result.files_attempted));
                            
                            if result.files_skipped_invalid_hash > 0 {
                                ui.label(format!("Files skipped (invalid hash): {}", 
                                    result.files_skipped_invalid_hash));
                            }
                            ui.add_space(8.0);

                            for file_result in &result.file_results {
                                ui.separator();
                                ui.label(format!("File: {}", file_result.file_path));
                                
                                ui.horizontal(|ui| {
                                    ui.label("SHA256:");
                                    ui.add(egui::Label::new(&file_result.sha256).wrap());
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Malicious:");
                                    ui.label(format!("{}", file_result.malicious));
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Suspicious:");
                                    ui.label(format!("{}", file_result.suspicious));
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Undetected:");
                                    ui.label(format!("{}", file_result.undetected));
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Harmless:");
                                    ui.label(format!("{}", file_result.harmless));
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Report Link:");
                                    ui.hyperlink_to("View on VirusTotal", &file_result.vt_link);
                                });
                                
                                if let Some(error) = &file_result.error {
                                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                                }
                                ui.add_space(4.0);
                            }
                            return;
                        }
                        crate::calc_virustotal_stt::ScanStatus::Scanning { scanned, total, operation } => {
                            ui.label(format!("Scanning: {} / {} ({})", scanned, total, operation));
                            return;
                        }
                        crate::calc_virustotal_stt::ScanStatus::Pending => {
                            ui.label("Scan pending...");
                            return;
                        }
                        crate::calc_virustotal_stt::ScanStatus::Error(err) => {
                            ui.colored_label(egui::Color32::RED, format!("Scan error: {}", err));
                            return;
                        }
                    }
                }
            }
        }
        
        ui.label("No VirusTotal scan results available for this package");
    }

    fn render_hybridanalysis_tab(&self, ui: &mut egui::Ui, pkg_id: &str) {
        let store = get_shared_store();
        let ha_state = store.get_ha_scanner_state();
        
        if let Some(state) = ha_state {
            if let Ok(scanner_state) = state.lock() {
                if let Some(scan_status) = scanner_state.get(pkg_id) {
                    ui.heading("Hybrid Analysis Scan Results");
                    ui.add_space(4.0);

                    match scan_status {
                        crate::calc_hybridanalysis_stt::ScanStatus::Completed(result) => {
                            ui.label(format!("Files scanned: {}", result.file_results.len()));
                            ui.add_space(8.0);

                            for file_result in &result.file_results {
                                ui.separator();
                                ui.label(format!("File: {}", file_result.file_path));
                                
                                ui.horizontal(|ui| {
                                    ui.label("SHA256:");
                                    ui.add(egui::Label::new(&file_result.sha256).wrap());
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Verdict:");
                                    ui.label(&file_result.verdict);
                                });

                                if let Some(threat_score) = file_result.threat_score {
                                    ui.horizontal(|ui| {
                                        ui.label("Threat Score:");
                                        ui.label(format!("{}/100", threat_score));
                                    });
                                }

                                if let Some(threat_level) = file_result.threat_level {
                                    ui.horizontal(|ui| {
                                        ui.label("Threat Level:");
                                        ui.label(format!("{}", threat_level));
                                    });
                                }

                                if let Some(total_sigs) = file_result.total_signatures {
                                    ui.horizontal(|ui| {
                                        ui.label("Total Signatures:");
                                        ui.label(format!("{}", total_sigs));
                                    });
                                }

                                if !file_result.classification_tags.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.label("Tags:");
                                        ui.label(file_result.classification_tags.join(", "));
                                    });
                                }

                                ui.horizontal(|ui| {
                                    ui.label("Report Link:");
                                    ui.hyperlink_to("View on Hybrid Analysis", &file_result.ha_link);
                                });
                                
                                if let Some(error) = &file_result.error_message {
                                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                                }
                                ui.add_space(4.0);
                            }
                            return;
                        }
                        crate::calc_hybridanalysis_stt::ScanStatus::Scanning { scanned, total, operation } => {
                            ui.label(format!("Scanning: {} / {} ({})", scanned, total, operation));
                            return;
                        }
                        crate::calc_hybridanalysis_stt::ScanStatus::Pending => {
                            ui.label("Scan pending...");
                            return;
                        }
                        crate::calc_hybridanalysis_stt::ScanStatus::Error(err) => {
                            ui.colored_label(egui::Color32::RED, format!("Scan error: {}", err));
                            return;
                        }
                    }
                }
            }
        }
        
        ui.label("No Hybrid Analysis scan results available for this package");
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

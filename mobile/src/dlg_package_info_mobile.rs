use crate::viewmodel::ViewModelState;
use crate::uad_shizuku_app::UadNgLists;
use eframe::egui;

pub struct DlgPackageInfoMobile {
    pub open: bool,
    pub selected_package_index: Option<usize>,
}

impl Default for DlgPackageInfoMobile {
    fn default() -> Self {
        Self {
            open: false,
            selected_package_index: None,
        }
    }
}

impl DlgPackageInfoMobile {
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
        vm_state: &ViewModelState,
        uad_ng_lists: &Option<UadNgLists>,
    ) {
        if !self.open {
            return;
        }

        let Some(pkg_idx) = self.selected_package_index else {
            return;
        };

        let Some(package) = vm_state.filtered_packages.get(pkg_idx) else {
            log::error!("Package index {} out of bounds", pkg_idx);
            self.close();
            return;
        };

        let pkg_id = &package.pkg;
        let mut close_clicked = false;

        egui::Window::new(format!("Package Info: {}", pkg_id))
            .id(egui::Id::new("package_info_mobile_window"))
            .title_bar(true)
            .resizable(false)
            .collapsible(false)
            .scroll([false, true])
            .fixed_size([
                ctx.screen_rect().width() - 40.0,
                ctx.screen_rect().height() - 40.0,
            ])
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button("Close").clicked() {
                        close_clicked = true;
                    }
                });

                ui.add_space(16.0);

                egui::ScrollArea::vertical()
                    .id_source("mobile_info_scroll")
                    .show(ui, |ui| {
                        render_package_section(ui, package);
                        ui.separator();

                        if let Some(uad_data) =
                            uad_ng_lists.as_ref().and_then(|l| l.apps.get(pkg_id))
                        {
                            render_uad_section(ui, uad_data);
                            ui.separator();
                        }

                        if let Some(gp_data) = vm_state.cached_metadata.get_google_play(pkg_id) {
                            render_google_play_section(ui, gp_data);
                            ui.separator();
                        }

                        if let Some(fd_data) = vm_state.cached_metadata.get_fdroid(pkg_id) {
                            render_fdroid_section(ui, fd_data);
                            ui.separator();
                        }

                        if let Some(apk_data) = vm_state.cached_metadata.get_apkmirror(pkg_id) {
                            render_apkmirror_section(ui, apk_data);
                            ui.separator();
                        }

                        if let Some(vt_state) = &vm_state.vt_scanner_state {
                            if let Ok(guard) = vt_state.lock() {
                                if let Some(scan_result) = guard.get(pkg_id) {
                                    render_virustotal_section(ui, scan_result);
                                    ui.separator();
                                }
                            }
                        }

                        if let Some(ha_state) = &vm_state.ha_scanner_state {
                            if let Ok(guard) = ha_state.lock() {
                                if let Some(scan_result) = guard.get(pkg_id) {
                                    render_hybridanalysis_section(ui, scan_result);
                                    ui.separator();
                                }
                            }
                        }
                    });
            });

        if close_clicked {
            self.close();
        }
    }
}

fn render_package_section(ui: &mut egui::Ui, package: &crate::adb_stt::PackageFingerprint) {
    ui.heading("Package Information");
    ui.add_space(8.0);

    ui.label(format!("Package ID: {}", package.pkg));
    ui.label(format!("Version: {}", package.versionName));

    if !package.users.is_empty() {
        let user = &package.users[0];
        ui.label(format!("Installed: {}", user.installed));
        ui.label(format!("Enabled: {}", user.enabled));
    }

    ui.label(format!("Flags: {}", package.flags));
}

fn render_uad_section(ui: &mut egui::Ui, uad_data: &crate::uad_shizuku_app_stt::AppEntry) {
    ui.heading("UAD-NG Debloat Information");
    ui.add_space(8.0);

    ui.label(format!("Removal Category: {}", uad_data.removal));
    ui.label(format!("Description: {}", uad_data.description));

    if !uad_data.dependencies.is_empty() {
        ui.label(format!("Dependencies: {}", uad_data.dependencies.join(", ")));
    }
}

fn render_google_play_section(ui: &mut egui::Ui, gp_data: &crate::models::GooglePlayApp) {
    ui.heading("Google Play");
    ui.add_space(8.0);

    ui.label(format!("Title: {}", gp_data.title));
    ui.label(format!("Developer: {}", gp_data.developer));
    if let Some(score) = gp_data.score {
        ui.label(format!("Score: {:.1} ⭐", score));
    }
}

fn render_fdroid_section(ui: &mut egui::Ui, fd_data: &crate::models::FDroidApp) {
    ui.heading("F-Droid");
    ui.add_space(8.0);

    ui.label(format!("Title: {}", fd_data.title));
    if let Some(description) = &fd_data.description {
        ui.label(format!("Description: {}", description));
    }
}

fn render_apkmirror_section(ui: &mut egui::Ui, apk_data: &crate::models::ApkMirrorApp) {
    ui.heading("APKMirror");
    ui.add_space(8.0);

    ui.label(format!("Title: {}", apk_data.title));
    ui.label(format!("Developer: {}", apk_data.developer));
}

fn render_virustotal_section(ui: &mut egui::Ui, vt_result: &crate::calc_virustotal_stt::ScanStatus) {
    ui.heading("VirusTotal Scan");
    ui.add_space(8.0);

    match vt_result {
        crate::calc_virustotal_stt::ScanStatus::Completed(result) => {
            let mut malicious_count = 0;
            let mut suspicious_count = 0;
            for file_result in &result.file_results {
                malicious_count += file_result.malicious;
                suspicious_count += file_result.suspicious;
            }

            if malicious_count > 0 {
                ui.colored_label(
                    egui::Color32::from_rgb(211, 47, 47),
                    format!("✗ {} malicious detections", malicious_count),
                );
            } else if suspicious_count > 0 {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 152, 0),
                    format!("⚠ {} suspicious detections", suspicious_count),
                );
            } else {
                ui.colored_label(egui::Color32::from_rgb(56, 142, 60), "✓ Clean");
            }
        }
        crate::calc_virustotal_stt::ScanStatus::Error(msg) => {
            ui.label(format!("Error: {}", msg));
        }
        _ => {
            ui.label("Scan in progress...");
        }
    }
}

fn render_hybridanalysis_section(
    ui: &mut egui::Ui,
    ha_result: &crate::calc_hybridanalysis_stt::ScanStatus,
) {
    ui.heading("HybridAnalysis");
    ui.add_space(8.0);

    match ha_result {
        crate::calc_hybridanalysis_stt::ScanStatus::Completed(result) => {
            let mut is_malicious = false;
            let mut is_suspicious = false;

            for file_result in &result.file_results {
                if file_result.verdict.to_lowercase().contains("malicious") {
                    is_malicious = true;
                } else if file_result.verdict.to_lowercase().contains("suspicious") {
                    is_suspicious = true;
                }
            }

            if is_malicious {
                ui.colored_label(egui::Color32::from_rgb(211, 47, 47), "✗ Malicious");
            } else if is_suspicious {
                ui.colored_label(egui::Color32::from_rgb(255, 152, 0), "⚠ Suspicious");
            } else {
                ui.colored_label(egui::Color32::from_rgb(56, 142, 60), "✓ Clean");
            }
        }
        crate::calc_hybridanalysis_stt::ScanStatus::Error(msg) => {
            ui.label(format!("Error: {}", msg));
        }
        _ => {
            ui.label("Scan in progress...");
        }
    }
}

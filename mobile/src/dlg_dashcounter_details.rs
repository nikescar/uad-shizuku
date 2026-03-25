use crate::adb::PackageFingerprint;
use crate::shared_store_stt::get_shared_store;
use crate::uad_shizuku_app::UadNgLists;
pub use crate::dlg_dashcounter_details_stt::*;
use crate::calc;
use crate::calc_stalkerware_stt::StalkerwareIndicators;
use crate::material_symbol_icons::{ICON_INFO, ICON_DELETE, ICON_REFRESH};
use eframe::egui;
use egui_material3::{data_table, MaterialButton, DataTableCell, icon_button_standard};
use egui_i18n::tr;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

impl DlgDashCounterDetails {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self, category: DashCounterCategory, count_enabled: usize, count_total: usize) {
        self.category = Some(category);
        self.count_enabled = count_enabled;
        self.count_total = count_total;
        self.sort_column = None;
        self.sort_ascending = true;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    /// Render VirusTotal scan result cell
    fn render_vt_cell(
        vt_result: Option<crate::calc_virustotal_stt::ScanStatus>,
        idx: usize,
    ) -> DataTableCell {
        DataTableCell::widget(move |ui: &mut egui::Ui| {
            egui::ScrollArea::horizontal()
                .id_salt(format!("vt_scroll_dash_{}", idx))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;

                        match &vt_result {
                            None => {
                                ui.label(tr!("scan-not-initialized"));
                            }
                            Some(crate::calc_virustotal_stt::ScanStatus::Pending) => {
                                ui.label(tr!("scan-not-scanned"));
                            }
                            Some(crate::calc_virustotal_stt::ScanStatus::Scanning { scanned, total, .. }) => {
                                ui.label(tr!("scan-scanning", { scanned: scanned, total: total }));
                            }
                            Some(crate::calc_virustotal_stt::ScanStatus::Completed(result)) => {
                                for (i, file_result) in result.file_results.iter().enumerate() {
                                    let (text, bg_color) = if file_result.error.is_some() {
                                        (tr!("scan-error"), egui::Color32::from_rgb(211, 47, 47))
                                    } else if file_result.skipped {
                                        (tr!("scan-skip"), egui::Color32::from_rgb(128, 128, 128))
                                    } else if file_result.not_found {
                                        (tr!("scan-404"), egui::Color32::from_rgb(128, 128, 128))
                                    } else if file_result.malicious > 0 {
                                        (tr!("scan-malicious", { count: file_result.malicious + file_result.suspicious, total: file_result.total() }), egui::Color32::from_rgb(211, 47, 47))
                                    } else if file_result.suspicious > 0 {
                                        (tr!("scan-suspicious", { count: file_result.suspicious, total: file_result.total() }), egui::Color32::from_rgb(255, 152, 0))
                                    } else {
                                        (tr!("scan-clean", { count: file_result.total(), total: file_result.total() }), egui::Color32::from_rgb(56, 142, 60))
                                    };

                                    let inner_response = egui::Frame::new()
                                        .fill(bg_color)
                                        .corner_radius(8.0)
                                        .inner_margin(egui::Margin::symmetric(12, 6))
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(12.0))
                                        });

                                    let response = ui.interact(
                                        inner_response.response.rect,
                                        ui.id().with(format!("vt_chip_dash_{}_{}", idx, i)),
                                        egui::Sense::click()
                                    );

                                    if let Some(ref err) = file_result.error {
                                        response.on_hover_text(format!("{}\n{}", file_result.file_path, err));
                                    } else {
                                        if response.clicked() {
                                            #[cfg(not(target_os = "android"))]
                                            {
                                                if let Err(err) = webbrowser::open(&file_result.vt_link) {
                                                    log::error!("Failed to open VirusTotal link: {}", err);
                                                }
                                            }
                                        }
                                        response.on_hover_text(&file_result.file_path);
                                    }
                                }
                            }
                            Some(crate::calc_virustotal_stt::ScanStatus::Error(e)) => {
                                ui.label(tr!("scan-error-msg", { message: e.clone() }));
                            }
                        }
                    });
                });
        })
    }

    /// Render HybridAnalysis scan result cell
    fn render_ha_cell(
        ha_result: Option<crate::calc_hybridanalysis_stt::ScanStatus>,
        idx: usize,
        ha_tag_ignorelist: String,
    ) -> DataTableCell {
        DataTableCell::widget(move |ui: &mut egui::Ui| {
            egui::ScrollArea::horizontal()
                .id_salt(format!("ha_scroll_dash_{}", idx))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;

                        match &ha_result {
                            None => {
                                ui.label(tr!("scan-not-initialized"));
                            }
                            Some(crate::calc_hybridanalysis_stt::ScanStatus::Pending) => {
                                ui.label(tr!("scan-not-scanned"));
                            }
                            Some(crate::calc_hybridanalysis_stt::ScanStatus::Scanning { scanned, total, .. }) => {
                                ui.label(tr!("scan-scanning", { scanned: scanned, total: total }));
                            }
                            Some(crate::calc_hybridanalysis_stt::ScanStatus::Completed(result)) => {
                                if result.file_results.is_empty() {
                                    ui.label(tr!("scan-no-results"));
                                }
                                for (i, file_result) in result.file_results.iter().enumerate() {
                                    // Build translated display text (copied from tab_scan_control.rs)
                                    let text = Self::get_ha_display_text(file_result);

                                    // Check if all tags are ignored
                                    let ignorelist_tags: Vec<String> = ha_tag_ignorelist
                                        .split(',')
                                        .map(|s| s.trim().to_lowercase())
                                        .filter(|s| !s.is_empty())
                                        .collect();

                                    let all_tags_ignored = if file_result.classification_tags.is_empty() {
                                        true
                                    } else {
                                        file_result.classification_tags.iter().all(|tag| {
                                            ignorelist_tags.contains(&tag.to_lowercase())
                                        })
                                    };

                                    let bg_color = match file_result.verdict.as_str() {
                                        "malicious" => {
                                            if all_tags_ignored {
                                                egui::Color32::from_rgb(128, 128, 128)
                                            } else {
                                                egui::Color32::from_rgb(211, 47, 47)
                                            }
                                        },
                                        "suspicious" => egui::Color32::from_rgb(255, 152, 0),
                                        "whitelisted" => egui::Color32::from_rgb(56, 142, 60),
                                        "no specific threat" => egui::Color32::from_rgb(0, 150, 136),
                                        "no-result" => egui::Color32::from_rgb(158, 158, 158),
                                        "rate_limited" => egui::Color32::from_rgb(156, 39, 176),
                                        "submitted" => egui::Color32::from_rgb(33, 150, 243),
                                        "pending_analysis" => egui::Color32::from_rgb(33, 150, 243),
                                        "upload_error" | "analysis_error" => egui::Color32::from_rgb(211, 47, 47),
                                        "404 Not Found" => egui::Color32::from_rgb(158, 158, 158),
                                        "" => egui::Color32::from_rgb(158, 158, 158),
                                        _ => egui::Color32::from_rgb(158, 158, 158),
                                    };

                                    let inner_response = egui::Frame::new()
                                        .fill(bg_color)
                                        .corner_radius(8.0)
                                        .inner_margin(egui::Margin::symmetric(12, 6))
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(12.0))
                                        });

                                    let response = ui.interact(
                                        inner_response.response.rect,
                                        ui.id().with(format!("ha_chip_dash_{}_{}", idx, i)),
                                        egui::Sense::click()
                                    );

                                    if let Some(ref error_msg) = file_result.error_message {
                                        response.on_hover_text(format!("{}\n{}", file_result.file_path, error_msg));
                                    } else {
                                        if response.clicked() {
                                            #[cfg(not(target_os = "android"))]
                                            {
                                                if !file_result.ha_link.is_empty() {
                                                    if let Err(err) = webbrowser::open(&file_result.ha_link) {
                                                        log::error!("Failed to open HybridAnalysis link: {}", err);
                                                    }
                                                }
                                            }
                                        }
                                        response.on_hover_text(&file_result.file_path);
                                    }
                                }
                            }
                            Some(crate::calc_hybridanalysis_stt::ScanStatus::Error(e)) => {
                                ui.label(tr!("scan-error-msg", { message: e.clone() }));
                            }
                        }
                    });
                });
        })
    }

    /// Get HybridAnalysis display text (extracted from tab_scan_control.rs)
    fn get_ha_display_text(file_result: &crate::calc_hybridanalysis_stt::FileScanResult) -> String {
        // For error states, show translated error message
        if file_result.verdict == "upload_error" || file_result.verdict == "analysis_error" {
            if let Some(ref error_msg) = file_result.error_message {
                if error_msg.contains("File too large") {
                    if let Some(mb_pos) = error_msg.find(" MB ") {
                        if let Some(start) = error_msg[..mb_pos].rfind(|c: char| !c.is_numeric() && c != '.') {
                            let size = &error_msg[start+1..mb_pos+3];
                            return tr!("ha-file-too-large", { size: size.to_string() });
                        } else {
                            return tr!("ha-file-too-large-default");
                        }
                    } else {
                        return tr!("ha-file-too-large-default");
                    }
                } else if error_msg.contains("No such file or directory") {
                    return tr!("ha-pull-failed");
                } else if error_msg.contains("Failed to create tmp directory") {
                    return tr!("ha-temp-dir-error");
                } else {
                    if file_result.verdict == "upload_error" {
                        return tr!("ha-upload-error");
                    } else {
                        return tr!("ha-analysis-error");
                    }
                }
            } else if file_result.verdict == "upload_error" {
                return tr!("ha-upload-error");
            } else {
                return tr!("ha-analysis-error");
            }
        }

        // Get base translated text
        let has_tags = !file_result.classification_tags.is_empty();
        let base_text = if has_tags {
            let tags_str = file_result.classification_tags.join(", ");
            match file_result.verdict.as_str() {
                "malicious" => tr!("ha-malicious-tags", { tags: tags_str }),
                "suspicious" => tr!("ha-suspicious-tags", { tags: tags_str }),
                "whitelisted" => tr!("ha-whitelisted-tags", { tags: tags_str }),
                "no specific threat" => tr!("ha-no-specific-threat-tags", { tags: tags_str }),
                _ => match file_result.verdict.as_str() {
                    "no-result" => tr!("ha-no-result"),
                    "rate_limited" => tr!("ha-rate-limited"),
                    "submitted" => tr!("ha-submitted"),
                    "pending_analysis" => tr!("ha-pending-analysis"),
                    "404 Not Found" => tr!("ha-404"),
                    "" => tr!("ha-skipped"),
                    _ => file_result.verdict.clone(),
                },
            }
        } else if let Some(score) = file_result.threat_score {
            match file_result.verdict.as_str() {
                "malicious" => tr!("ha-malicious-score", { score: score }),
                "suspicious" => tr!("ha-suspicious-score", { score: score }),
                "whitelisted" => tr!("ha-whitelisted-score", { score: score }),
                "no specific threat" => tr!("ha-no-specific-threat-score", { score: score }),
                _ => match file_result.verdict.as_str() {
                    "no-result" => tr!("ha-no-result"),
                    "rate_limited" => tr!("ha-rate-limited"),
                    "submitted" => tr!("ha-submitted"),
                    "pending_analysis" => tr!("ha-pending-analysis"),
                    "404 Not Found" => tr!("ha-404"),
                    "" => tr!("ha-skipped"),
                    _ => file_result.verdict.clone(),
                },
            }
        } else {
            match file_result.verdict.as_str() {
                "malicious" => tr!("ha-malicious"),
                "suspicious" => tr!("ha-suspicious"),
                "whitelisted" => tr!("ha-whitelisted"),
                "no specific threat" => tr!("ha-no-specific-threat"),
                "no-result" => tr!("ha-no-result"),
                "rate_limited" => tr!("ha-rate-limited"),
                "submitted" => tr!("ha-submitted"),
                "pending_analysis" => {
                    if let Some(ref job_id) = file_result.job_id {
                        let short_id = if job_id.len() > 8 { &job_id[..8] } else { job_id };
                        tr!("ha-pending", { jobid: short_id.to_string() })
                    } else {
                        tr!("ha-pending-analysis")
                    }
                },
                "404 Not Found" => tr!("ha-404"),
                "" => tr!("ha-skipped"),
                _ => file_result.verdict.clone(),
            }
        };

        // Check for wait_until time
        if let Some(wait_until) = file_result.wait_until {
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if wait_until > now {
                let remaining_secs = wait_until - now;
                let hours = remaining_secs / 3600;
                let mins = (remaining_secs % 3600) / 60;
                if hours > 0 {
                    tr!("ha-wait-hours", { text: base_text, hours: hours, mins: mins })
                } else if mins > 0 {
                    tr!("ha-wait-mins", { text: base_text, mins: mins })
                } else {
                    tr!("ha-wait-less-than-min", { text: base_text })
                }
            } else {
                base_text
            }
        } else {
            base_text
        }
    }

    /// Render action buttons (copied from tab_debloat_control.rs)
    fn render_action_buttons_static(
        ui: &mut egui::Ui,
        pkg_id: &str,
        package: &PackageFingerprint,
        clicked_idx: Arc<Mutex<Option<usize>>>,
        row_idx: usize,
        debloat_category: Option<&str>,
        unsafe_app_remove: bool,
        show_refresh_button: bool,
    ) {
        let pkg_id_clone = pkg_id.to_string();

        let enabled_str = if let Some(user) = package.users.get(0) {
            match user.enabled {
                0 => {
                    let is_system = package.flags.contains("SYSTEM");
                    if !user.installed && is_system {
                        "REMOVED_USER"
                    } else {
                        "DEFAULT"
                    }
                }
                1 => "ENABLED",
                2 => "DISABLED",
                3 => "DISABLED_USER",
                _ => "UNKNOWN",
            }
        } else {
            "UNKNOWN"
        };

        let is_system = package.flags.contains("SYSTEM");
        let is_unsafe_blocked = debloat_category == Some("Unsafe") && !unsafe_app_remove;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            // Info button
            if ui.add(icon_button_standard(ICON_INFO.to_string()))
                .on_hover_text(tr!("package-info")).clicked() {
                if let Ok(mut clicked) = clicked_idx.lock() {
                    *clicked = Some(row_idx);
                }
            }

            // Refresh button - delete scan results and re-queue (only for VT/HA tables)
            if show_refresh_button {
                if ui.add(icon_button_standard(ICON_REFRESH.to_string()))
                    .on_hover_text(tr!("refresh-list")).clicked() {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("refresh_clicked_package"), pkg_id_clone.clone());
                    });
                }
            }

            // Enable/disable toggle
            let pkg_enabled = enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED");
            let can_show_toggle = !is_unsafe_blocked || !pkg_enabled;

            if can_show_toggle {
                let mut enabled = pkg_enabled;
                if toggle_ui(ui, &mut enabled).clicked() {
                    if enabled {
                        ui.data_mut(|data| {
                            data.insert_temp(egui::Id::new("enable_clicked_package"), pkg_id_clone.clone());
                        });
                    } else {
                        ui.data_mut(|data| {
                            data.insert_temp(egui::Id::new("disable_clicked_package"), pkg_id_clone.clone());
                        });
                    }
                }
            }

            // Uninstall button (only for enabled apps and not unsafe_blocked)
            if (enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED")) && !is_unsafe_blocked {
                if ui.add(icon_button_standard(ICON_DELETE.to_string())
                    .icon_color(egui::Color32::from_rgb(211, 47, 47)))
                    .on_hover_text(tr!("uninstall")).clicked() {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("uninstall_clicked_package"), pkg_id_clone);
                        data.insert_temp(egui::Id::new("uninstall_clicked_is_system"), is_system);
                    });
                }
            }
        });
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        installed_packages: &[PackageFingerprint],
        uad_ng_lists: &Option<UadNgLists>,
        stalkerware_indicators: &Option<StalkerwareIndicators>,
        package_risk_scores: &HashMap<String, i32>,
        unsafe_app_remove: bool,
        hybridanalysis_tag_ignorelist: &str,
    ) {
        if !self.open {
            return;
        }

        let Some(category) = self.category.clone() else {
            return;
        };

        let mut close_clicked = false;
        let title = self.get_window_title(&category);

        // Track which package info was clicked
        let clicked_package_idx: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));

        egui::Window::new(&title)
            .id(egui::Id::new("dashcounter_details_window"))
            .title_bar(false)
            .resizable(true)
            .collapsible(false)
            .scroll([false, false])
            .min_width(800.0)
            .min_height(600.0)
            .resize(|r| {
                r.default_size([ctx.content_rect().width() - 40.0, ctx.content_rect().height() - 40.0])
                    .max_size([ctx.content_rect().width() - 40.0, ctx.content_rect().height() - 40.0])
            })
            .show(ctx, |ui| {
                // Show description
                ui.heading(&title);
                ui.add_space(10.0);

                // Filters (copied from tab_debloat_control.rs)
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr!("show-only-enabled"));
                    toggle_ui(ui, &mut self.show_only_enabled);
                    ui.add_space(10.0);
                    ui.label(tr!("hide-system-app"));
                    toggle_ui(ui, &mut self.hide_system_app);
                    ui.add_space(10.0);
                    ui.label(tr!("filter"));
                    let response = ui.add(egui::TextEdit::singleline(&mut self.text_filter)
                        .hint_text(tr!("filter-hint"))
                        .desired_width(200.0));
                    #[cfg(target_os = "android")]
                    {
                        if response.gained_focus() {
                            let _ = crate::android_inputmethod::show_soft_input();
                        }
                        if response.lost_focus() {
                            let _ = crate::android_inputmethod::hide_soft_input();
                        }
                    }
                    crate::clipboard_popup::show_clipboard_popup(ui, &response, &mut self.text_filter);
                    if !self.text_filter.is_empty() && ui.button("X").clicked() {
                        self.text_filter.clear();
                    }
                });

                ui.add_space(10.0);

                let max_height = ui.available_height() - 50.0;

                egui::ScrollArea::both()
                    .id_salt("dashcounter_details_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        match &category {
                            DashCounterCategory::DebloatRecommend
                            | DashCounterCategory::DebloatAdvanced
                            | DashCounterCategory::DebloatExpert
                            | DashCounterCategory::DebloatUnsafe
                            | DashCounterCategory::DebloatUnknown => {
                                self.render_debloat_table(ui, ctx, installed_packages, uad_ng_lists, &category, clicked_package_idx.clone(), unsafe_app_remove);
                            }
                            DashCounterCategory::StalkerwareDetected
                            | DashCounterCategory::StalkerwareUndetected => {
                                self.render_stalkerware_table(ui, ctx, installed_packages, stalkerware_indicators, &category, clicked_package_idx.clone(), unsafe_app_remove);
                            }
                            DashCounterCategory::IzzyRiskSafe
                            | DashCounterCategory::IzzyRiskNormal
                            | DashCounterCategory::IzzyRiskModerate
                            | DashCounterCategory::IzzyRiskHigh => {
                                self.render_izzyrisk_table(ui, ctx, installed_packages, package_risk_scores, &category, clicked_package_idx.clone(), unsafe_app_remove);
                            }
                            DashCounterCategory::VirusTotalMalicious
                            | DashCounterCategory::VirusTotalSuspicious
                            | DashCounterCategory::VirusTotalSafe
                            | DashCounterCategory::VirusTotalNotScanned => {
                                self.render_virustotal_table(ui, ctx, installed_packages, &category, clicked_package_idx.clone(), unsafe_app_remove);
                            }
                            DashCounterCategory::HybridAnalysisMalicious
                            | DashCounterCategory::HybridAnalysisMaliciousIgnored
                            | DashCounterCategory::HybridAnalysisSuspicious
                            | DashCounterCategory::HybridAnalysisSafe
                            | DashCounterCategory::HybridAnalysisNotScanned => {
                                self.render_hybridanalysis_table(ui, ctx, installed_packages, &category, clicked_package_idx.clone(), hybridanalysis_tag_ignorelist, unsafe_app_remove);
                            }
                        }
                    });

                ui.add_space(8.0);

                // Close button
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(MaterialButton::filled("Close")).clicked() {
                            close_clicked = true;
                        }
                    });
                });
            });

        if close_clicked {
            self.close();
        }

        // Handle package details dialog open
        let clicked_idx = {
            clicked_package_idx.lock().ok().and_then(|guard| *guard)
        };

        if let Some(idx) = clicked_idx {
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("info_clicked_package"), installed_packages[idx].pkg.clone());
            });
        }
    }

    fn get_window_title(&self, category: &DashCounterCategory) -> String {
        let base_title = match category {
            DashCounterCategory::DebloatRecommend => "Debloat: Recommended",
            DashCounterCategory::DebloatAdvanced => "Debloat: Advanced",
            DashCounterCategory::DebloatExpert => "Debloat: Expert",
            DashCounterCategory::DebloatUnsafe => "Debloat: Unsafe",
            DashCounterCategory::DebloatUnknown => "Debloat: Unknown",
            DashCounterCategory::StalkerwareDetected => "Stalkerware: Detected",
            DashCounterCategory::StalkerwareUndetected => "Stalkerware: Undetected",
            DashCounterCategory::IzzyRiskSafe => "IzzyRisk: Safe (0)",
            DashCounterCategory::IzzyRiskNormal => "IzzyRisk: Normal (1-10)",
            DashCounterCategory::IzzyRiskModerate => "IzzyRisk: Moderate (11-20)",
            DashCounterCategory::IzzyRiskHigh => "IzzyRisk: High (20+)",
            DashCounterCategory::VirusTotalMalicious => "VirusTotal: Malicious",
            DashCounterCategory::VirusTotalSuspicious => "VirusTotal: Suspicious",
            DashCounterCategory::VirusTotalSafe => "VirusTotal: Safe",
            DashCounterCategory::VirusTotalNotScanned => "VirusTotal: Not Scanned",
            DashCounterCategory::HybridAnalysisMalicious => "HybridAnalysis: Malicious",
            DashCounterCategory::HybridAnalysisMaliciousIgnored => "HybridAnalysis: Malicious (Ignored)",
            DashCounterCategory::HybridAnalysisSuspicious => "HybridAnalysis: Suspicious",
            DashCounterCategory::HybridAnalysisSafe => "HybridAnalysis: Safe",
            DashCounterCategory::HybridAnalysisNotScanned => "HybridAnalysis: Not Scanned",
        };
        format!("{} ({}/{})", base_title, self.count_enabled, self.count_total)
    }

    fn render_debloat_table(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        installed_packages: &[PackageFingerprint],
        uad_ng_lists: &Option<UadNgLists>,
        category: &DashCounterCategory,
        clicked_package_idx: Arc<Mutex<Option<usize>>>,
        unsafe_app_remove: bool,
    ) {
        let Some(uad_lists) = uad_ng_lists else {
            ui.label("UAD lists not loaded");
            return;
        };

        let target_removal = match category {
            DashCounterCategory::DebloatRecommend => "Recommended",
            DashCounterCategory::DebloatAdvanced => "Advanced",
            DashCounterCategory::DebloatExpert => "Expert",
            DashCounterCategory::DebloatUnsafe => "Unsafe",
            DashCounterCategory::DebloatUnknown => "Unknown",
            _ => return,
        };

        // Filter packages by category and filters
        let mut filtered_packages: Vec<&PackageFingerprint> = installed_packages
            .iter()
            .filter(|pkg| {
                // Category filter
                let matches_category = if let Some(uad_entry) = uad_lists.apps.get(&pkg.pkg) {
                    uad_entry.removal == target_removal
                } else {
                    target_removal == "Unknown"
                };

                // Apply other filters
                matches_category
                    && self.should_show_package(pkg)
                    && self.matches_text_filter(&pkg.pkg, pkg)
            })
            .collect();

        // Sort packages if needed
        if let Some(col) = self.sort_column {
            self.sort_debloat_packages(&mut filtered_packages, col, uad_lists);
        }

        // Fetch Android package info for all filtered packages (on Android only)
        #[cfg(target_os = "android")]
        {
            let store = get_shared_store();
            for pkg in &filtered_packages {
                // Check if not already cached
                if store.get_cached_android_package_app(&pkg.pkg).is_none() {
                    if let Some(info) = crate::calc_androidpackage::fetch_android_package_info(&pkg.pkg) {
                        store.set_cached_android_package_app(pkg.pkg.clone(), info);
                    }
                }
            }
        }

        // Build datatable
        let mut table = data_table()
            .id(egui::Id::new("debloat_details_table"))
            .sortable_column("Apps", 400.0, false)
            .sortable_column("", 200.0, false);

        // Set initial sort state
        if let Some(sort_col) = self.sort_column {
            use egui_material3::SortDirection;
            let direction = if self.sort_ascending {
                SortDirection::Ascending
            } else {
                SortDirection::Descending
            };
            table = table.sort_by(sort_col, direction);
        }

        for (idx, pkg) in filtered_packages.iter().enumerate() {
            let app_desc_cell = calc::render_app_description_cell(ctx, &pkg.pkg);
            let pkg_id = pkg.pkg.clone();
            let pkg_clone = (*pkg).clone();
            let clicked_idx_clone = clicked_package_idx.clone();
            let debloat_cat = target_removal;

            // Find the actual index in installed_packages
            let actual_idx = installed_packages.iter().position(|p| p.pkg == pkg.pkg).unwrap_or(idx);

            table = table.row(|row| {
                row.custom_cell(app_desc_cell)
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, clicked_idx_clone.clone(), actual_idx, Some(debloat_cat), unsafe_app_remove, false);
                    }))
            });
        }

        let table_response = table.show(ui);

        // Handle sort state sync
        let (widget_sort_col, widget_sort_dir) = table_response.sort_state;
        let widget_sort_ascending = matches!(widget_sort_dir, egui_material3::SortDirection::Ascending);

        if widget_sort_col != self.sort_column
            || (widget_sort_col.is_some() && widget_sort_ascending != self.sort_ascending)
        {
            self.sort_column = widget_sort_col;
            self.sort_ascending = widget_sort_ascending;
        }

        // Handle column clicks
        if let Some(clicked_col) = table_response.column_clicked {
            if self.sort_column == Some(clicked_col) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(clicked_col);
                self.sort_ascending = true;
            }
        }
    }

    fn render_stalkerware_table(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        installed_packages: &[PackageFingerprint],
        stalkerware_indicators: &Option<StalkerwareIndicators>,
        category: &DashCounterCategory,
        clicked_package_idx: Arc<Mutex<Option<usize>>>,
        unsafe_app_remove: bool,
    ) {
        let Some(indicators) = stalkerware_indicators else {
            ui.label("Stalkerware indicators not loaded");
            return;
        };

        let is_detected = matches!(category, DashCounterCategory::StalkerwareDetected);

        // Filter packages
        let mut filtered_packages: Vec<&PackageFingerprint> = installed_packages
            .iter()
            .filter(|pkg| {
                let is_stalkerware = indicators.is_stalkerware(&pkg.pkg);
                let matches_category = if is_detected {
                    is_stalkerware
                } else {
                    !is_stalkerware
                };

                // Apply other filters
                matches_category
                    && self.should_show_package(pkg)
                    && self.matches_text_filter(&pkg.pkg, pkg)
            })
            .collect();

        // Sort if needed
        if let Some(col) = self.sort_column {
            self.sort_stalkerware_packages(&mut filtered_packages, col);
        }

        // Fetch Android package info for all filtered packages (on Android only)
        #[cfg(target_os = "android")]
        {
            let store = get_shared_store();
            for pkg in &filtered_packages {
                // Check if not already cached
                if store.get_cached_android_package_app(&pkg.pkg).is_none() {
                    if let Some(info) = crate::calc_androidpackage::fetch_android_package_info(&pkg.pkg) {
                        store.set_cached_android_package_app(pkg.pkg.clone(), info);
                    }
                }
            }
        }

        // Build datatable
        let mut table = data_table()
            .id(egui::Id::new("stalkerware_details_table"))
            .sortable_column("Apps", 400.0, false)
            .sortable_column("", 200.0, false);

        // Set initial sort state
        if let Some(sort_col) = self.sort_column {
            use egui_material3::SortDirection;
            let direction = if self.sort_ascending {
                SortDirection::Ascending
            } else {
                SortDirection::Descending
            };
            table = table.sort_by(sort_col, direction);
        }

        for (idx, pkg) in filtered_packages.iter().enumerate() {
            let app_desc_cell = calc::render_app_description_cell(ctx, &pkg.pkg);
            let pkg_id = pkg.pkg.clone();
            let pkg_clone = (*pkg).clone();
            let clicked_idx_clone = clicked_package_idx.clone();
            let actual_idx = installed_packages.iter().position(|p| p.pkg == pkg.pkg).unwrap_or(idx);

            table = table.row(|row| {
                row.custom_cell(app_desc_cell)
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, clicked_idx_clone.clone(), actual_idx, None, unsafe_app_remove, false);
                    }))
            });
        }

        let table_response = table.show(ui);

        // Handle sort state sync
        let (widget_sort_col, widget_sort_dir) = table_response.sort_state;
        let widget_sort_ascending = matches!(widget_sort_dir, egui_material3::SortDirection::Ascending);

        if widget_sort_col != self.sort_column
            || (widget_sort_col.is_some() && widget_sort_ascending != self.sort_ascending)
        {
            self.sort_column = widget_sort_col;
            self.sort_ascending = widget_sort_ascending;
        }

        // Handle column clicks
        if let Some(clicked_col) = table_response.column_clicked {
            if self.sort_column == Some(clicked_col) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(clicked_col);
                self.sort_ascending = true;
            }
        }
    }

    fn render_izzyrisk_table(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        installed_packages: &[PackageFingerprint],
        package_risk_scores: &HashMap<String, i32>,
        category: &DashCounterCategory,
        clicked_package_idx: Arc<Mutex<Option<usize>>>,
        unsafe_app_remove: bool,
    ) {
        // Filter packages by risk level and filters
        let mut filtered_packages: Vec<&PackageFingerprint> = installed_packages
            .iter()
            .filter(|pkg| {
                let matches_category = if let Some(&score) = package_risk_scores.get(&pkg.pkg) {
                    match category {
                        DashCounterCategory::IzzyRiskSafe => score == 0,
                        DashCounterCategory::IzzyRiskNormal => score >= 1 && score <= 10,
                        DashCounterCategory::IzzyRiskModerate => score >= 11 && score <= 20,
                        DashCounterCategory::IzzyRiskHigh => score > 20,
                        _ => false,
                    }
                } else {
                    false
                };

                // Apply other filters
                matches_category
                    && self.should_show_package(pkg)
                    && self.matches_text_filter(&pkg.pkg, pkg)
            })
            .collect();

        // Sort if needed
        if let Some(col) = self.sort_column {
            self.sort_izzyrisk_packages(&mut filtered_packages, col, package_risk_scores);
        }

        // Fetch Android package info for all filtered packages (on Android only)
        #[cfg(target_os = "android")]
        {
            let store = get_shared_store();
            for pkg in &filtered_packages {
                // Check if not already cached
                if store.get_cached_android_package_app(&pkg.pkg).is_none() {
                    if let Some(info) = crate::calc_androidpackage::fetch_android_package_info(&pkg.pkg) {
                        store.set_cached_android_package_app(pkg.pkg.clone(), info);
                    }
                }
            }
        }

        // Build datatable
        let mut table = data_table()
            .id(egui::Id::new("izzyrisk_details_table"))
            .sortable_column("Apps", 300.0, false)
            .sortable_column("Risk Score", 100.0, true)
            .sortable_column("Caused Permissions", 200.0, false)
            .sortable_column("", 200.0, false);

        // Set initial sort state
        if let Some(sort_col) = self.sort_column {
            use egui_material3::SortDirection;
            let direction = if self.sort_ascending {
                SortDirection::Ascending
            } else {
                SortDirection::Descending
            };
            table = table.sort_by(sort_col, direction);
        }

        for (idx, pkg) in filtered_packages.iter().enumerate() {
            let app_desc_cell = calc::render_app_description_cell(ctx, &pkg.pkg);
            let risk_score = package_risk_scores.get(&pkg.pkg).copied().unwrap_or(0);

            // Get caused permissions (install permissions)
            let permissions_text = if pkg.installPermissions.is_empty() {
                "None".to_string()
            } else {
                format!("{} permissions", pkg.installPermissions.len())
            };

            let pkg_id = pkg.pkg.clone();
            let pkg_clone = (*pkg).clone();
            let clicked_idx_clone = clicked_package_idx.clone();
            let actual_idx = installed_packages.iter().position(|p| p.pkg == pkg.pkg).unwrap_or(idx);

            table = table.row(|row| {
                row.custom_cell(app_desc_cell)
                    .cell(&risk_score.to_string())
                    .cell(&permissions_text)
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, clicked_idx_clone.clone(), actual_idx, None, unsafe_app_remove, false);
                    }))
            });
        }

        let table_response = table.show(ui);

        // Handle sort state sync
        let (widget_sort_col, widget_sort_dir) = table_response.sort_state;
        let widget_sort_ascending = matches!(widget_sort_dir, egui_material3::SortDirection::Ascending);

        if widget_sort_col != self.sort_column
            || (widget_sort_col.is_some() && widget_sort_ascending != self.sort_ascending)
        {
            self.sort_column = widget_sort_col;
            self.sort_ascending = widget_sort_ascending;
        }

        // Handle column clicks
        if let Some(clicked_col) = table_response.column_clicked {
            if self.sort_column == Some(clicked_col) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(clicked_col);
                self.sort_ascending = true;
            }
        }
    }

    fn render_virustotal_table(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        installed_packages: &[PackageFingerprint],
        category: &DashCounterCategory,
        clicked_package_idx: Arc<Mutex<Option<usize>>>,
        unsafe_app_remove: bool,
    ) {
        let store = get_shared_store();
        let vt_state = store.get_vt_scanner_state();

        // Filter packages by VT status
        let mut filtered_packages: Vec<&PackageFingerprint> = installed_packages
            .iter()
            .filter(|pkg| {
                let matches_category = if let Some(state) = &vt_state {
                    if let Ok(scanner_state) = state.lock() {
                        match scanner_state.get(&pkg.pkg) {
                            Some(crate::calc_virustotal_stt::ScanStatus::Completed(result)) => {
                                // Check file-level flags for categorization
                                let has_not_found = result.file_results.iter().any(|fr| fr.not_found);
                                let has_skipped = result.file_results.iter().any(|fr| fr.skipped);
                                let has_error = result.file_results.iter().any(|fr| fr.error.is_some());

                                // If any file was not found, skipped, or had error, count as not_scanned
                                if has_not_found || has_skipped || has_error {
                                    matches!(category, DashCounterCategory::VirusTotalNotScanned)
                                } else {
                                    match category {
                                        DashCounterCategory::VirusTotalMalicious => {
                                            result.file_results.iter().any(|f| f.malicious > 0)
                                        }
                                        DashCounterCategory::VirusTotalSuspicious => {
                                            result.file_results.iter().any(|f| f.suspicious > 0 && f.malicious == 0)
                                        }
                                        DashCounterCategory::VirusTotalSafe => {
                                            result.file_results.iter().all(|f| f.malicious == 0 && f.suspicious == 0)
                                        }
                                        DashCounterCategory::VirusTotalNotScanned => false,
                                        _ => false,
                                    }
                                }
                            }
                            _ => {
                                // Pending, Scanning, Error, or None (not in scanner_state)
                                matches!(category, DashCounterCategory::VirusTotalNotScanned)
                            }
                        }
                    } else {
                        matches!(category, DashCounterCategory::VirusTotalNotScanned)
                    }
                } else {
                    matches!(category, DashCounterCategory::VirusTotalNotScanned)
                };
                matches_category
                    && self.should_show_package(pkg)
                    && self.matches_text_filter(&pkg.pkg, pkg)
            })
            .collect();

        // Sort if needed
        if let Some(col) = self.sort_column {
            self.sort_virustotal_packages(&mut filtered_packages, col);
        }

        // Fetch Android package info for all filtered packages (on Android only)
        #[cfg(target_os = "android")]
        {
            let store = get_shared_store();
            for pkg in &filtered_packages {
                // Check if not already cached
                if store.get_cached_android_package_app(&pkg.pkg).is_none() {
                    if let Some(info) = crate::calc_androidpackage::fetch_android_package_info(&pkg.pkg) {
                        store.set_cached_android_package_app(pkg.pkg.clone(), info);
                    }
                }
            }
        }

        // Build datatable
        let mut table = data_table()
            .id(egui::Id::new("virustotal_details_table"))
            .sortable_column("Apps", 300.0, false)
            .sortable_column(tr!("col-virustotal"), 200.0, false)
            .sortable_column("", 200.0, false);

        // Set initial sort state
        if let Some(sort_col) = self.sort_column {
            use egui_material3::SortDirection;
            let direction = if self.sort_ascending {
                SortDirection::Ascending
            } else {
                SortDirection::Descending
            };
            table = table.sort_by(sort_col, direction);
        }

        for (idx, pkg) in filtered_packages.iter().enumerate() {
            let app_desc_cell = calc::render_app_description_cell(ctx, &pkg.pkg);
            let pkg_id = pkg.pkg.clone();
            let pkg_clone = (*pkg).clone();
            let clicked_idx_clone = clicked_package_idx.clone();
            let actual_idx = installed_packages.iter().position(|p| p.pkg == pkg.pkg).unwrap_or(idx);

            // Get VT scan result for this package
            let vt_scan_result = vt_state.as_ref().and_then(|state| {
                state.lock().ok().and_then(|scanner_state| {
                    scanner_state.get(&pkg.pkg).cloned()
                })
            });

            table = table.row(|row| {
                row.custom_cell(app_desc_cell)
                    .custom_cell(Self::render_vt_cell(vt_scan_result, idx))
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, clicked_idx_clone.clone(), actual_idx, None, unsafe_app_remove, true);
                    }))
            });
        }

        let table_response = table.show(ui);

        // Handle sort state sync
        let (widget_sort_col, widget_sort_dir) = table_response.sort_state;
        let widget_sort_ascending = matches!(widget_sort_dir, egui_material3::SortDirection::Ascending);

        if widget_sort_col != self.sort_column
            || (widget_sort_col.is_some() && widget_sort_ascending != self.sort_ascending)
        {
            self.sort_column = widget_sort_col;
            self.sort_ascending = widget_sort_ascending;
        }

        // Handle column clicks
        if let Some(clicked_col) = table_response.column_clicked {
            if self.sort_column == Some(clicked_col) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(clicked_col);
                self.sort_ascending = true;
            }
        }
    }

    fn render_hybridanalysis_table(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        installed_packages: &[PackageFingerprint],
        category: &DashCounterCategory,
        clicked_package_idx: Arc<Mutex<Option<usize>>>,
        hybridanalysis_tag_ignorelist: &str,
        unsafe_app_remove: bool,
    ) {
        let store = get_shared_store();
        let ha_state = store.get_ha_scanner_state();

        // Filter packages by HA status
        let mut filtered_packages: Vec<&PackageFingerprint> = installed_packages
            .iter()
            .filter(|pkg| {
                let matches_category = if let Some(state) = &ha_state {
                    if let Ok(scanner_state) = state.lock() {
                        match scanner_state.get(&pkg.pkg) {
                            Some(crate::calc_hybridanalysis_stt::ScanStatus::Completed(result)) => {
                                // Check if any file has non-scan verdict (404, skipped, upload_error, etc.)
                                let has_non_scan = result.file_results.iter().any(|fr| {
                                    fr.verdict == "404 Not Found" ||
                                    fr.verdict == "" ||
                                    fr.verdict == "upload_error" ||
                                    fr.verdict == "analysis_error"
                                });

                                // If any file was not scanned properly, count as not_scanned
                                if has_non_scan {
                                    matches!(category, DashCounterCategory::HybridAnalysisNotScanned)
                                } else {
                                    // Helper to check if all tags are ignored
                                    let check_all_tags_ignored = |file_result: &crate::calc_hybridanalysis_stt::FileScanResult| -> bool {
                                        let ignorelist_tags: Vec<String> = hybridanalysis_tag_ignorelist
                                            .split(',')
                                            .map(|s| s.trim().to_lowercase())
                                            .filter(|s| !s.is_empty())
                                            .collect();

                                        if file_result.classification_tags.is_empty() {
                                            true // No tags means we treat it as ignored
                                        } else {
                                            file_result.classification_tags.iter().all(|tag| {
                                                ignorelist_tags.contains(&tag.to_lowercase())
                                            })
                                        }
                                    };

                                    // Check for malicious files with/without ignored tags
                                    let has_malicious_ignored = result.file_results.iter()
                                        .any(|fr| fr.verdict == "malicious" && check_all_tags_ignored(fr));
                                    let has_malicious_normal = result.file_results.iter()
                                        .any(|fr| fr.verdict == "malicious" && !check_all_tags_ignored(fr));

                                    match category {
                                        DashCounterCategory::HybridAnalysisMalicious => {
                                            has_malicious_normal
                                        }
                                        DashCounterCategory::HybridAnalysisMaliciousIgnored => {
                                            has_malicious_ignored && !has_malicious_normal
                                        }
                                        DashCounterCategory::HybridAnalysisSuspicious => {
                                            result.file_results.iter().any(|f| f.verdict.to_lowercase().contains("suspicious"))
                                        }
                                        DashCounterCategory::HybridAnalysisSafe => {
                                            result.file_results.iter().all(|f| !f.verdict.to_lowercase().contains("malicious") && !f.verdict.to_lowercase().contains("suspicious"))
                                        }
                                        DashCounterCategory::HybridAnalysisNotScanned => false,
                                        _ => false,
                                    }
                                }
                            }
                            _ => {
                                // Pending, Scanning, Error, or None (not in scanner_state)
                                matches!(category, DashCounterCategory::HybridAnalysisNotScanned)
                            }
                        }
                    } else {
                        matches!(category, DashCounterCategory::HybridAnalysisNotScanned)
                    }
                } else {
                    matches!(category, DashCounterCategory::HybridAnalysisNotScanned)
                };
                matches_category
                    && self.should_show_package(pkg)
                    && self.matches_text_filter(&pkg.pkg, pkg)
            })
            .collect();

        // Sort if needed
        if let Some(col) = self.sort_column {
            self.sort_hybridanalysis_packages(&mut filtered_packages, col);
        }

        // Fetch Android package info for all filtered packages (on Android only)
        #[cfg(target_os = "android")]
        {
            let store = get_shared_store();
            for pkg in &filtered_packages {
                // Check if not already cached
                if store.get_cached_android_package_app(&pkg.pkg).is_none() {
                    if let Some(info) = crate::calc_androidpackage::fetch_android_package_info(&pkg.pkg) {
                        store.set_cached_android_package_app(pkg.pkg.clone(), info);
                    }
                }
            }
        }

        // Build datatable
        let mut table = data_table()
            .id(egui::Id::new("hybridanalysis_details_table"))
            .sortable_column("Apps", 300.0, false)
            .sortable_column(tr!("col-hybrid-analysis"), 200.0, false)
            .sortable_column("", 200.0, false);

        // Set initial sort state
        if let Some(sort_col) = self.sort_column {
            use egui_material3::SortDirection;
            let direction = if self.sort_ascending {
                SortDirection::Ascending
            } else {
                SortDirection::Descending
            };
            table = table.sort_by(sort_col, direction);
        }

        for (idx, pkg) in filtered_packages.iter().enumerate() {
            let app_desc_cell = calc::render_app_description_cell(ctx, &pkg.pkg);
            let pkg_id = pkg.pkg.clone();
            let pkg_clone = (*pkg).clone();
            let clicked_idx_clone = clicked_package_idx.clone();
            let actual_idx = installed_packages.iter().position(|p| p.pkg == pkg.pkg).unwrap_or(idx);

            // Get HA scan result for this package
            let ha_scan_result = ha_state.as_ref().and_then(|state| {
                state.lock().ok().and_then(|scanner_state| {
                    scanner_state.get(&pkg.pkg).cloned()
                })
            });

            let ha_tag_ignorelist_clone = hybridanalysis_tag_ignorelist.to_string();

            table = table.row(|row| {
                row.custom_cell(app_desc_cell)
                    .custom_cell(Self::render_ha_cell(ha_scan_result, idx, ha_tag_ignorelist_clone))
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, clicked_idx_clone.clone(), actual_idx, None, unsafe_app_remove, true);
                    }))
            });
        }

        let table_response = table.show(ui);

        // Handle sort state sync
        let (widget_sort_col, widget_sort_dir) = table_response.sort_state;
        let widget_sort_ascending = matches!(widget_sort_dir, egui_material3::SortDirection::Ascending);

        if widget_sort_col != self.sort_column
            || (widget_sort_col.is_some() && widget_sort_ascending != self.sort_ascending)
        {
            self.sort_column = widget_sort_col;
            self.sort_ascending = widget_sort_ascending;
        }

        // Handle column clicks
        if let Some(clicked_col) = table_response.column_clicked {
            if self.sort_column == Some(clicked_col) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(clicked_col);
                self.sort_ascending = true;
            }
        }
    }

    // Sorting helper methods

    /// Get display name for a package (matches what render_app_description_cell shows)
    fn get_display_name(pkg_id: &str) -> String {
        let store = get_shared_store();

        // Priority 1: Android Package (native icons)
        if let Some(android_app) = store.get_cached_android_package_app(pkg_id) {
            if !android_app.label.is_empty() {
                return android_app.label.to_lowercase();
            }
        }

        // Priority 2-4: External sources (disabled on Android)
        #[cfg(not(target_os = "android"))]
        {
            // Priority 2: FDroid
            if let Some(fdroid_app) = store.get_cached_fdroid_app(pkg_id) {
                if !fdroid_app.title.is_empty() {
                    return fdroid_app.title.to_lowercase();
                }
            }

            // Priority 3: GooglePlay
            if let Some(gp_app) = store.get_cached_google_play_app(pkg_id) {
                if !gp_app.title.is_empty() {
                    return gp_app.title.to_lowercase();
                }
            }

            // Priority 4: APKMirror
            if let Some(am_app) = store.get_cached_apkmirror_app(pkg_id) {
                if !am_app.title.is_empty() {
                    return am_app.title.to_lowercase();
                }
            }
        }

        // Fallback: package ID
        pkg_id.to_lowercase()
    }

    /// Check if package matches text filter
    fn matches_text_filter(&self, pkg_id: &str, package: &PackageFingerprint) -> bool {
        if self.text_filter.is_empty() {
            return true;
        }

        let filter_lower = self.text_filter.to_lowercase();

        // Check package name
        if pkg_id.to_lowercase().contains(&filter_lower) {
            return true;
        }

        // Check display name (app title)
        let display_name = Self::get_display_name(pkg_id);
        if display_name.contains(&filter_lower) {
            return true;
        }

        // Check version
        if package.versionName.to_lowercase().contains(&filter_lower) {
            return true;
        }

        false
    }

    /// Check if package should be shown based on filters
    fn should_show_package(&self, package: &PackageFingerprint) -> bool {
        let is_system = package.flags.contains("SYSTEM");
        let is_enabled = Self::get_enabled_status(package);

        // Apply show_only_enabled filter
        if self.show_only_enabled && !is_enabled {
            return false;
        }

        // Apply hide_system_app filter
        if self.hide_system_app && is_system {
            return false;
        }

        true
    }

    /// Get enabled status for sorting (true = enabled, false = disabled)
    fn get_enabled_status(package: &PackageFingerprint) -> bool {
        if let Some(user) = package.users.get(0) {
            let enabled = user.enabled;
            let is_system = package.flags.contains("SYSTEM");

            match enabled {
                0 => {
                    // DEFAULT - enabled if system and installed, or if user app
                    if is_system {
                        user.installed
                    } else {
                        true
                    }
                }
                1 => true,  // ENABLED
                2 | 3 => false,  // DISABLED or DISABLED_USER
                _ => false,
            }
        } else {
            false
        }
    }

    fn sort_debloat_packages(
        &self,
        packages: &mut [&PackageFingerprint],
        column: usize,
        _uad_lists: &UadNgLists,
    ) {
        packages.sort_by(|a, b| {
            let ordering = match column {
                0 => {
                    let name_a = Self::get_display_name(&a.pkg);
                    let name_b = Self::get_display_name(&b.pkg);
                    name_a.cmp(&name_b)
                }
                1 => {
                    // Sort by enabled/disabled status
                    let enabled_a = Self::get_enabled_status(a);
                    let enabled_b = Self::get_enabled_status(b);
                    enabled_b.cmp(&enabled_a) // Enabled first (true > false)
                }
                _ => std::cmp::Ordering::Equal,
            };
            if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
    }

    fn sort_stalkerware_packages(&self, packages: &mut [&PackageFingerprint], column: usize) {
        packages.sort_by(|a, b| {
            let ordering = match column {
                0 => {
                    let name_a = Self::get_display_name(&a.pkg);
                    let name_b = Self::get_display_name(&b.pkg);
                    name_a.cmp(&name_b)
                }
                1 => {
                    // Sort by enabled/disabled status
                    let enabled_a = Self::get_enabled_status(a);
                    let enabled_b = Self::get_enabled_status(b);
                    enabled_b.cmp(&enabled_a) // Enabled first (true > false)
                }
                _ => std::cmp::Ordering::Equal,
            };
            if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
    }

    fn sort_izzyrisk_packages(
        &self,
        packages: &mut [&PackageFingerprint],
        column: usize,
        package_risk_scores: &HashMap<String, i32>,
    ) {
        packages.sort_by(|a, b| {
            let ordering = match column {
                0 => {
                    let name_a = Self::get_display_name(&a.pkg);
                    let name_b = Self::get_display_name(&b.pkg);
                    name_a.cmp(&name_b)
                }
                1 => {
                    let score_a = package_risk_scores.get(&a.pkg).copied().unwrap_or(0);
                    let score_b = package_risk_scores.get(&b.pkg).copied().unwrap_or(0);
                    score_a.cmp(&score_b)
                }
                2 => {
                    let perms_a = a.installPermissions.len();
                    let perms_b = b.installPermissions.len();
                    perms_a.cmp(&perms_b)
                }
                3 => {
                    // Sort by enabled/disabled status
                    let enabled_a = Self::get_enabled_status(a);
                    let enabled_b = Self::get_enabled_status(b);
                    enabled_b.cmp(&enabled_a) // Enabled first (true > false)
                }
                _ => std::cmp::Ordering::Equal,
            };
            if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
    }

    fn sort_virustotal_packages(&self, packages: &mut [&PackageFingerprint], column: usize) {
        packages.sort_by(|a, b| {
            let ordering = match column {
                0 => {
                    let name_a = Self::get_display_name(&a.pkg);
                    let name_b = Self::get_display_name(&b.pkg);
                    name_a.cmp(&name_b)
                }
                1 => {
                    // Column 1 is VirusTotal results - no special sorting
                    std::cmp::Ordering::Equal
                }
                2 => {
                    // Sort by enabled/disabled status
                    let enabled_a = Self::get_enabled_status(a);
                    let enabled_b = Self::get_enabled_status(b);
                    enabled_b.cmp(&enabled_a) // Enabled first (true > false)
                }
                _ => std::cmp::Ordering::Equal,
            };
            if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
    }

    fn sort_hybridanalysis_packages(&self, packages: &mut [&PackageFingerprint], column: usize) {
        packages.sort_by(|a, b| {
            let ordering = match column {
                0 => {
                    let name_a = Self::get_display_name(&a.pkg);
                    let name_b = Self::get_display_name(&b.pkg);
                    name_a.cmp(&name_b)
                }
                1 => {
                    // Column 1 is HybridAnalysis results - no special sorting
                    std::cmp::Ordering::Equal
                }
                2 => {
                    // Sort by enabled/disabled status
                    let enabled_a = Self::get_enabled_status(a);
                    let enabled_b = Self::get_enabled_status(b);
                    enabled_b.cmp(&enabled_a) // Enabled first (true > false)
                }
                _ => std::cmp::Ordering::Equal,
            };
            if self.sort_ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
    }
}

/// iOS-style toggle switch (copied from tab_debloat_control.rs)
fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter().circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

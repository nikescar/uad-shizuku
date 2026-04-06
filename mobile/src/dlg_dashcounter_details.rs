use crate::adb::PackageFingerprint;
use crate::shared_store_stt::get_shared_store;
use crate::uad_shizuku_app::UadNgLists;
pub use crate::dlg_dashcounter_details_stt::*;
use crate::calc;
use crate::calc_stalkerware_stt::StalkerwareIndicators;
use crate::material_symbol_icons::{ICON_DELETE, ICON_REFRESH, ICON_SETTINGS, ICON_INFO, ICON_DOWNLOAD};
use crate::svg_stt::*;
use eframe::egui;
use egui_material3::{data_table, MaterialButton, DataTableCell, icon_button_standard, show_tooltip_on_hover, TooltipPosition};
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
        self.current_page = 0;
        self.open = true;
        self.invalidate_cache();
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    /// Generate a cache key based on current state
    fn generate_cache_key(&self, installed_packages: &[PackageFingerprint]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash all state that affects rendering
        format!("{:?}", self.category).hash(&mut hasher);
        self.sort_column.hash(&mut hasher);
        self.sort_ascending.hash(&mut hasher);
        self.show_only_enabled.hash(&mut hasher);
        self.hide_system_app.hash(&mut hasher);
        self.text_filter.hash(&mut hasher);
        self.current_page.hash(&mut hasher);
        self.items_per_page.hash(&mut hasher);

        // Hash package list length (lightweight proxy for package changes)
        installed_packages.len().hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Check if cache needs refresh based on time throttle and state changes
    fn should_refresh_cache(&self, current_time: f64, new_cache_key: &str) -> bool {
        // Always refresh if cache key changed
        if self.cache_key != new_cache_key {
            return true;
        }

        // Throttle: only refresh if enough time has passed
        (current_time - self.last_refresh_time) >= self.refresh_interval
    }

    /// Invalidate cache
    fn invalidate_cache(&mut self) {
        self.cache_key.clear();
        self.cached_rows.clear();
        self.last_refresh_time = 0.0;
    }

    /// Pre-compute row data for caching
    fn prepare_row_cache(&mut self, packages: &[&PackageFingerprint], current_time: f64) {
        self.cached_rows.clear();

        // Store package IDs for cache validation
        for pkg in packages {
            self.cached_rows.push(CachedRowData {
                package_id: pkg.pkg.clone(),
            });
        }

        self.last_refresh_time = current_time;
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

    /// Create a clickable app description cell that opens package details on click
    fn render_clickable_app_cell(
        ctx: &egui::Context,
        pkg_id: &str,
        clicked_package_idx: Arc<Mutex<Option<usize>>>,
        row_idx: usize,
    ) -> DataTableCell {
        let pkg_id_owned = pkg_id.to_string();
        let ctx_clone = ctx.clone();

        // Create clickable cell
        DataTableCell::widget(move |ui: &mut egui::Ui| {
            // Get app info
            let store = get_shared_store();
            let on_surface = egui_material3::get_global_color("onSurface");

            let mut texture: Option<egui::TextureId> = None;
            let mut title_text = pkg_id_owned.clone();
            let mut subtitle_text = String::new();

            // Priority 1: Android Package (native icons)
            if let Some(android_app) = store.get_cached_android_package_app(&pkg_id_owned) {
                if !android_app.icon_bytes.is_empty() {
                    if let Some(tex) = calc::load_texture_from_bytes(&ctx_clone, &pkg_id_owned, &android_app.icon_bytes) {
                        texture = Some(tex.id());
                        if !android_app.label.is_empty() {
                            title_text = android_app.label.clone();
                        }
                    }
                }
            }

            // Priority 2-4: External sources (disabled on Android)
            #[cfg(not(target_os = "android"))]
            {
                // Priority 2: FDroid
                if texture.is_none() {
                    if let Some(fdroid_app) = store.get_cached_fdroid_app(&pkg_id_owned) {
                        if let Some(icon) = &fdroid_app.icon_base64 {
                            if let Some(tex) = calc::load_texture_from_base64(&ctx_clone, "fd", &pkg_id_owned, icon) {
                                texture = Some(tex.id());
                                if !fdroid_app.title.is_empty() {
                                    title_text = fdroid_app.title.clone();
                                }
                                if !fdroid_app.developer.is_empty() {
                                    subtitle_text = fdroid_app.developer.clone();
                                }
                            }
                        }
                    }
                }

                // Priority 3: GooglePlay
                if texture.is_none() {
                    if let Some(gp_app) = store.get_cached_google_play_app(&pkg_id_owned) {
                        if let Some(icon) = &gp_app.icon_base64 {
                            if let Some(tex) = calc::load_texture_from_base64(&ctx_clone, "gp", &pkg_id_owned, icon) {
                                texture = Some(tex.id());
                                if !gp_app.title.is_empty() {
                                    title_text = gp_app.title.clone();
                                }
                                if !gp_app.developer.is_empty() {
                                    subtitle_text = gp_app.developer.clone();
                                }
                            }
                        }
                    }
                }

                // Priority 4: APKMirror
                if texture.is_none() {
                    if let Some(am_app) = store.get_cached_apkmirror_app(&pkg_id_owned) {
                        if let Some(icon) = &am_app.icon_base64 {
                            if let Some(tex) = calc::load_texture_from_base64(&ctx_clone, "am", &pkg_id_owned, icon) {
                                texture = Some(tex.id());
                                if !am_app.title.is_empty() {
                                    title_text = am_app.title.clone();
                                }
                                if !am_app.developer.is_empty() {
                                    subtitle_text = am_app.developer.clone();
                                }
                            }
                        }
                    }
                }
            }

            // If no subtitle, use package ID
            if subtitle_text.is_empty() {
                subtitle_text = pkg_id_owned.clone();
            }

            // Render the content in a clickable area
            let response = ui.horizontal(|ui| {
                // App icon (38x38)
                if let Some(tex_id) = texture {
                    let size = egui::Vec2::new(38.0, 38.0);
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(tex_id, size)));
                } else {
                    ui.add_space(38.0);
                }

                ui.add_space(8.0);

                // Title and subtitle
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(egui::RichText::new(&title_text).color(on_surface).size(14.0));
                    ui.label(egui::RichText::new(&subtitle_text)
                        .color(egui::Color32::from_rgba_unmultiplied(
                            on_surface.r(),
                            on_surface.g(),
                            on_surface.b(),
                            153,
                        ))
                        .size(12.0));
                });
            }).response;

            // Make the entire cell clickable
            let sense_response = ui.interact(response.rect, egui::Id::new(format!("clickable_app_cell_{}", row_idx)), egui::Sense::click());

            if sense_response.clicked() {
                if let Ok(mut clicked) = clicked_package_idx.lock() {
                    *clicked = Some(row_idx);
                }
            }

            // Change cursor on hover
            if sense_response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        })
    }

    /// Render action buttons (copied from tab_debloat_control.rs)
    fn render_action_buttons_static(
        ui: &mut egui::Ui,
        pkg_id: &str,
        package: &PackageFingerprint,
        debloat_category: Option<&str>,
        unsafe_app_remove: bool,
        expert_app_remove: bool,
        show_refresh_button: bool,
        uad_ng_lists: &Option<UadNgLists>,
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

        // Check if app is classified as "Unsafe" or "Expert" in debloat lists, regardless of current view
        let actual_debloat_category = uad_ng_lists.as_ref()
            .and_then(|lists| lists.apps.get(pkg_id))
            .map(|entry| entry.removal.as_str());
        let is_unsafe_blocked = actual_debloat_category == Some("Unsafe") && !unsafe_app_remove;
        let is_expert_blocked = actual_debloat_category == Some("Expert") && !expert_app_remove;
        let is_blocked = is_unsafe_blocked || is_expert_blocked;

        ui.centered_and_justified(|ui| {
            egui::ScrollArea::horizontal()
                .id_salt(format!("action_buttons_{}", pkg_id))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;

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
            let can_show_toggle = !is_blocked || !pkg_enabled;

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

            // Uninstall button (only for enabled apps and not blocked)
            if (enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED")) && !is_blocked {
                if ui.add(icon_button_standard(ICON_DELETE.to_string())
                    .icon_color(egui::Color32::from_rgb(211, 47, 47)))
                    .on_hover_text(tr!("uninstall")).clicked() {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("uninstall_clicked_package"), pkg_id_clone.clone());
                        data.insert_temp(egui::Id::new("uninstall_clicked_is_system"), is_system);
                    });
                }
            }

            // Settings button for blocked enabled apps (when nothing else shows)
            if is_blocked && pkg_enabled {
                if ui.add(icon_button_standard(ICON_SETTINGS.to_string()))
                    .on_hover_text(tr!("settings")).clicked() {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("settings_button_clicked"), true);
                    });
                }
            }
                    });
                });
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
        expert_app_remove: bool,
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
                                self.render_debloat_table(ui, ctx, installed_packages, uad_ng_lists, &category, clicked_package_idx.clone(), unsafe_app_remove, expert_app_remove);
                            }
                            DashCounterCategory::StalkerwareDetected
                            | DashCounterCategory::StalkerwareUndetected => {
                                self.render_stalkerware_table(ui, ctx, installed_packages, stalkerware_indicators, &category, clicked_package_idx.clone(), unsafe_app_remove, expert_app_remove, uad_ng_lists);
                            }
                            DashCounterCategory::IzzyRiskSafe
                            | DashCounterCategory::IzzyRiskNormal
                            | DashCounterCategory::IzzyRiskModerate
                            | DashCounterCategory::IzzyRiskHigh => {
                                self.render_izzyrisk_table(ui, ctx, installed_packages, package_risk_scores, &category, clicked_package_idx.clone(), unsafe_app_remove, expert_app_remove, uad_ng_lists);
                            }
                            DashCounterCategory::VirusTotalMalicious
                            | DashCounterCategory::VirusTotalSuspicious
                            | DashCounterCategory::VirusTotalSafe
                            | DashCounterCategory::VirusTotalNotScanned => {
                                self.render_virustotal_table(ui, ctx, installed_packages, &category, clicked_package_idx.clone(), unsafe_app_remove, expert_app_remove, uad_ng_lists);
                            }
                            DashCounterCategory::HybridAnalysisMalicious
                            | DashCounterCategory::HybridAnalysisMaliciousIgnored
                            | DashCounterCategory::HybridAnalysisSuspicious
                            | DashCounterCategory::HybridAnalysisSafe
                            | DashCounterCategory::HybridAnalysisNotScanned => {
                                self.render_hybridanalysis_table(ui, ctx, installed_packages, &category, clicked_package_idx.clone(), hybridanalysis_tag_ignorelist, unsafe_app_remove, expert_app_remove, uad_ng_lists);
                            }
                            DashCounterCategory::OffaCategory(_) | DashCounterCategory::FmhyCategory(_) => {
                                self.render_apps_table(ui, ctx, installed_packages, &category, clicked_package_idx.clone());
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
            DashCounterCategory::OffaCategory(ref cat_name) => return format!("FOSS/OFFA: {} ({}/{})", cat_name, self.count_enabled, self.count_total),
            DashCounterCategory::FmhyCategory(ref cat_name) => return format!("FOSS/FMHY: {} ({}/{})", cat_name, self.count_enabled, self.count_total),
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
        expert_app_remove: bool,
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

        // Cache management for performance
        let current_time = ui.input(|i| i.time);
        let new_cache_key = self.generate_cache_key(installed_packages);

        // Pagination (calculate early for cache)
        let total_items = filtered_packages.len();
        let total_pages = if total_items == 0 { 1 } else { (total_items + self.items_per_page - 1) / self.items_per_page.max(1) };
        if self.current_page >= total_pages {
            self.current_page = total_pages.saturating_sub(1);
        }
        let start_idx = self.current_page * self.items_per_page;
        let end_idx = (start_idx + self.items_per_page).min(total_items);
        let page_packages = if start_idx < total_items { &filtered_packages[start_idx..end_idx] } else { &[] };

        if self.should_refresh_cache(current_time, &new_cache_key) {
            self.prepare_row_cache(page_packages, current_time);
            self.cache_key = new_cache_key;
        }

        // Build datatable with responsive column widths
        let available_width = ui.available_width();
        // Reserve space for drawer column (~40px)
        let content_width = available_width - 40.0;
        // Columns: Apps (66.67%) + Actions (33.33%) = 100%
        let mut table = data_table()
            .id(egui::Id::new("debloat_details_table"))
            .allow_drawer(true)
            .sortable_column("Apps", content_width * 0.6667, false)
            .sortable_column("", content_width * 0.3333, false);

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

        // Build table rows from cached data
        for (idx, pkg) in page_packages.iter().enumerate() {
            let idx = start_idx + idx;
            let pkg_id = pkg.pkg.clone();
            let pkg_clone = (*pkg).clone();
            let clicked_idx_clone = clicked_package_idx.clone();
            let debloat_cat = target_removal;

            // Find the actual index in installed_packages
            let actual_idx = installed_packages.iter().position(|p| p.pkg == pkg.pkg).unwrap_or(idx);

            let app_desc_cell = Self::render_clickable_app_cell(ctx, &pkg.pkg, clicked_idx_clone.clone(), actual_idx);
            let uad_lists_clone = uad_ng_lists.clone();
            let pkg_id_for_drawer = pkg_id.clone();
            let uad_lists_for_drawer = uad_ng_lists.clone();

            table = table.row(|row| {
                let mut row_builder = row.custom_cell(app_desc_cell)
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, Some(debloat_cat), unsafe_app_remove, expert_app_remove, false, &uad_lists_clone);
                    }));

                // Add drawer if UAD description exists
                if let Some(uad_entry) = uad_lists_for_drawer.as_ref().and_then(|lists| lists.apps.get(&pkg_id_for_drawer)) {
                    let description = uad_entry.description.clone();
                    row_builder = row_builder.drawer(move |ui| {
                        ui.add_space(8.0);
                        ui.label("Description:");
                        ui.add(egui::Label::new(&description).wrap());
                    });
                }
                row_builder
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

        // Pagination controls
        if total_pages > 1 {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.button("◀").clicked() && self.current_page > 0 {
                    self.current_page -= 1;
                }
                ui.add_space(10.0);
                ui.label(format!("Page {} of {} ({} items)", self.current_page + 1, total_pages, total_items));
                ui.add_space(10.0);
                if ui.button("▶").clicked() && self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
            });
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
        expert_app_remove: bool,
        uad_ng_lists: &Option<UadNgLists>,
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

        // Cache management for performance
        let current_time = ui.input(|i| i.time);
        let new_cache_key = self.generate_cache_key(installed_packages);

        // Pagination (calculate early for cache)
        let total_items = filtered_packages.len();
        let total_pages = if total_items == 0 { 1 } else { (total_items + self.items_per_page - 1) / self.items_per_page.max(1) };
        if self.current_page >= total_pages {
            self.current_page = total_pages.saturating_sub(1);
        }
        let start_idx = self.current_page * self.items_per_page;
        let end_idx = (start_idx + self.items_per_page).min(total_items);
        let page_packages = if start_idx < total_items { &filtered_packages[start_idx..end_idx] } else { &[] };

        if self.should_refresh_cache(current_time, &new_cache_key) {
            self.prepare_row_cache(page_packages, current_time);
            self.cache_key = new_cache_key;
        }

        // Build datatable with responsive column widths
        let available_width = ui.available_width();
        // Reserve space for drawer column (~40px)
        let content_width = available_width - 40.0;
        // Columns: Apps (66.67%) + Actions (33.33%) = 100%
        let mut table = data_table()
            .id(egui::Id::new("stalkerware_details_table"))
            .allow_drawer(true)
            .sortable_column("Apps", content_width * 0.6667, false)
            .sortable_column("", content_width * 0.3333, false);

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

        // Build table rows from cached data
        for (idx, pkg) in page_packages.iter().enumerate() {
            let idx = start_idx + idx;
            let pkg_id = pkg.pkg.clone();
            let pkg_clone = (*pkg).clone();
            let clicked_idx_clone = clicked_package_idx.clone();
            let actual_idx = installed_packages.iter().position(|p| p.pkg == pkg.pkg).unwrap_or(idx);
            let uad_lists_clone = uad_ng_lists.clone();

            let app_desc_cell = Self::render_clickable_app_cell(ctx, &pkg.pkg, clicked_idx_clone.clone(), actual_idx);
            let pkg_id_for_drawer = pkg_id.clone();
            let uad_lists_for_drawer = uad_lists_clone.clone();

            table = table.row(|row| {
                let mut row_builder = row.custom_cell(app_desc_cell)
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, None, unsafe_app_remove, expert_app_remove, false, &uad_lists_clone);
                    }));

                // Add drawer if UAD description exists
                if let Some(uad_entry) = uad_lists_for_drawer.as_ref().and_then(|lists| lists.apps.get(&pkg_id_for_drawer)) {
                    let description = uad_entry.description.clone();
                    row_builder = row_builder.drawer(move |ui| {
                        ui.add_space(8.0);
                        ui.label("Description:");
                        ui.add(egui::Label::new(&description).wrap());
                    });
                }
                row_builder
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

        // Pagination controls
        if total_pages > 1 {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.button("◀").clicked() && self.current_page > 0 {
                    self.current_page -= 1;
                }
                ui.add_space(10.0);
                ui.label(format!("Page {} of {} ({} items)", self.current_page + 1, total_pages, total_items));
                ui.add_space(10.0);
                if ui.button("▶").clicked() && self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
            });
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
        expert_app_remove: bool,
        uad_ng_lists: &Option<UadNgLists>,
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

        // Cache management for performance
        let current_time = ui.input(|i| i.time);
        let new_cache_key = self.generate_cache_key(installed_packages);

        // Pagination (calculate early for cache)
        let total_items = filtered_packages.len();
        let total_pages = if total_items == 0 { 1 } else { (total_items + self.items_per_page - 1) / self.items_per_page.max(1) };
        if self.current_page >= total_pages {
            self.current_page = total_pages.saturating_sub(1);
        }
        let start_idx = self.current_page * self.items_per_page;
        let end_idx = (start_idx + self.items_per_page).min(total_items);
        let page_packages = if start_idx < total_items { &filtered_packages[start_idx..end_idx] } else { &[] };

        if self.should_refresh_cache(current_time, &new_cache_key) {
            self.prepare_row_cache(page_packages, current_time);
            self.cache_key = new_cache_key;
        }

        // Build datatable with responsive column widths
        let available_width = ui.available_width();
        // Reserve space for drawer column (~40px)
        let content_width = available_width - 40.0;
        let screen_width = ctx.screen_rect().width();
        let is_narrow_screen = screen_width < 600.0;

        // Use abbreviated headers for narrow screens
        let risk_score_header = if is_narrow_screen { "RS" } else { "Risk Score" };
        let permissions_header = if is_narrow_screen { "Perm" } else { "Caused Permissions" };

        // Columns: Apps (37.5%) + Risk Score (12.5%) + Permissions (25%) + Actions (25%) = 100%
        let mut table = data_table()
            .id(egui::Id::new("izzyrisk_details_table"))
            .allow_drawer(true)
            .sortable_column("Apps", content_width * 0.375, false);

        // Add Risk Score column with tooltip if abbreviated
        if is_narrow_screen {
            table = table
                .sortable_column(risk_score_header, content_width * 0.125, true)
                .column_tooltip("Risk Score");
        } else {
            table = table.sortable_column(risk_score_header, content_width * 0.125, true);
        }

        // Add Permissions column with tooltip if abbreviated
        if is_narrow_screen {
            table = table
                .sortable_column(permissions_header, content_width * 0.25, false)
                .column_tooltip("Caused Permissions");
        } else {
            table = table.sortable_column(permissions_header, content_width * 0.25, false);
        }

        table = table.sortable_column("", content_width * 0.25, false);

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

        // Build table rows from cached data
        for (idx, pkg) in page_packages.iter().enumerate() {
            let idx = start_idx + idx;
            let risk_score = package_risk_scores.get(&pkg.pkg).copied().unwrap_or(0);

            // Get caused permissions (install permissions)
            let permissions_text = if pkg.installPermissions.is_empty() {
                "0".to_string()
            } else {
                pkg.installPermissions.len().to_string()
            };

            let pkg_id = pkg.pkg.clone();
            let pkg_clone = (*pkg).clone();
            let clicked_idx_clone = clicked_package_idx.clone();
            let actual_idx = installed_packages.iter().position(|p| p.pkg == pkg.pkg).unwrap_or(idx);
            let uad_lists_clone = uad_ng_lists.clone();

            let app_desc_cell = Self::render_clickable_app_cell(ctx, &pkg.pkg, clicked_idx_clone.clone(), actual_idx);
            let pkg_id_for_drawer = pkg_id.clone();
            let uad_lists_for_drawer = uad_lists_clone.clone();

            table = table.row(|row| {
                let mut row_builder = row.custom_cell(app_desc_cell)
                    .cell(&risk_score.to_string())
                    .cell(&permissions_text)
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, None, unsafe_app_remove, expert_app_remove, false, &uad_lists_clone);
                    }));

                // Add drawer if UAD description exists
                if let Some(uad_entry) = uad_lists_for_drawer.as_ref().and_then(|lists| lists.apps.get(&pkg_id_for_drawer)) {
                    let description = uad_entry.description.clone();
                    row_builder = row_builder.drawer(move |ui| {
                        ui.add_space(8.0);
                        ui.label("Description:");
                        ui.add(egui::Label::new(&description).wrap());
                    });
                }
                row_builder
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

        // Pagination controls
        if total_pages > 1 {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.button("◀").clicked() && self.current_page > 0 {
                    self.current_page -= 1;
                }
                ui.add_space(10.0);
                ui.label(format!("Page {} of {} ({} items)", self.current_page + 1, total_pages, total_items));
                ui.add_space(10.0);
                if ui.button("▶").clicked() && self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
            });
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
        expert_app_remove: bool,
        uad_ng_lists: &Option<UadNgLists>,
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

        // Cache management for performance
        let current_time = ui.input(|i| i.time);
        let new_cache_key = self.generate_cache_key(installed_packages);

        // Pagination (calculate early for cache)
        let total_items = filtered_packages.len();
        let total_pages = if total_items == 0 { 1 } else { (total_items + self.items_per_page - 1) / self.items_per_page.max(1) };
        if self.current_page >= total_pages {
            self.current_page = total_pages.saturating_sub(1);
        }
        let start_idx = self.current_page * self.items_per_page;
        let end_idx = (start_idx + self.items_per_page).min(total_items);
        let page_packages = if start_idx < total_items { &filtered_packages[start_idx..end_idx] } else { &[] };

        if self.should_refresh_cache(current_time, &new_cache_key) {
            self.prepare_row_cache(page_packages, current_time);
            self.cache_key = new_cache_key;
        }

        // Build datatable with responsive column widths
        let available_width = ui.available_width();
        // Reserve space for drawer column (~40px)
        let content_width = available_width - 40.0;
        // Columns: Apps (42.86%) + VirusTotal (28.57%) + Actions (28.57%) = 100%
        let mut table = data_table()
            .id(egui::Id::new("virustotal_details_table"))
            .allow_drawer(true)
            .sortable_column("Apps", content_width * 0.4286, false)
            .sortable_column(tr!("col-virustotal"), content_width * 0.2857, false)
            .sortable_column("", content_width * 0.2857, false);

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

        // Build table rows from cached data
        for (idx, pkg) in page_packages.iter().enumerate() {
            let idx = start_idx + idx;
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

            let app_desc_cell = Self::render_clickable_app_cell(ctx, &pkg.pkg, clicked_idx_clone.clone(), actual_idx);
            let uad_lists_clone = uad_ng_lists.clone();
            let pkg_id_for_drawer = pkg_id.clone();
            let uad_lists_for_drawer = uad_ng_lists.clone();

            table = table.row(|row| {
                let mut row_builder = row.custom_cell(app_desc_cell)
                    .custom_cell(Self::render_vt_cell(vt_scan_result, idx))
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, None, unsafe_app_remove, expert_app_remove, true, &uad_lists_clone);
                    }));

                // Add drawer if UAD description exists
                if let Some(uad_entry) = uad_lists_for_drawer.as_ref().and_then(|lists| lists.apps.get(&pkg_id_for_drawer)) {
                    let description = uad_entry.description.clone();
                    row_builder = row_builder.drawer(move |ui| {
                        ui.add_space(8.0);
                        ui.label("Description:");
                        ui.add(egui::Label::new(&description).wrap());
                    });
                }
                row_builder
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

        // Pagination controls
        if total_pages > 1 {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.button("◀").clicked() && self.current_page > 0 {
                    self.current_page -= 1;
                }
                ui.add_space(10.0);
                ui.label(format!("Page {} of {} ({} items)", self.current_page + 1, total_pages, total_items));
                ui.add_space(10.0);
                if ui.button("▶").clicked() && self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
            });
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
        expert_app_remove: bool,
        uad_ng_lists: &Option<UadNgLists>,
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

        // Cache management for performance
        let current_time = ui.input(|i| i.time);
        let new_cache_key = self.generate_cache_key(installed_packages);

        // Pagination (calculate early for cache)
        let total_items = filtered_packages.len();
        let total_pages = if total_items == 0 { 1 } else { (total_items + self.items_per_page - 1) / self.items_per_page.max(1) };
        if self.current_page >= total_pages {
            self.current_page = total_pages.saturating_sub(1);
        }
        let start_idx = self.current_page * self.items_per_page;
        let end_idx = (start_idx + self.items_per_page).min(total_items);
        let page_packages = if start_idx < total_items { &filtered_packages[start_idx..end_idx] } else { &[] };

        if self.should_refresh_cache(current_time, &new_cache_key) {
            self.prepare_row_cache(page_packages, current_time);
            self.cache_key = new_cache_key;
        }

        // Build datatable with responsive column widths
        let available_width = ui.available_width();
        // Reserve space for drawer column (~40px)
        let content_width = available_width - 40.0;
        // Columns: Apps (42.86%) + HybridAnalysis (28.57%) + Actions (28.57%) = 100%
        let mut table = data_table()
            .id(egui::Id::new("hybridanalysis_details_table"))
            .allow_drawer(true)
            .sortable_column("Apps", content_width * 0.4286, false)
            .sortable_column(tr!("col-hybrid-analysis"), content_width * 0.2857, false)
            .sortable_column("", content_width * 0.2857, false);

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

        // Build table rows from cached data
        for (idx, pkg) in page_packages.iter().enumerate() {
            let idx = start_idx + idx;
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

            let app_desc_cell = Self::render_clickable_app_cell(ctx, &pkg.pkg, clicked_idx_clone.clone(), actual_idx);
            let ha_tag_ignorelist_clone = hybridanalysis_tag_ignorelist.to_string();
            let uad_lists_clone = uad_ng_lists.clone();
            let pkg_id_for_drawer = pkg_id.clone();
            let uad_lists_for_drawer = uad_ng_lists.clone();

            table = table.row(|row| {
                let mut row_builder = row.custom_cell(app_desc_cell)
                    .custom_cell(Self::render_ha_cell(ha_scan_result, idx, ha_tag_ignorelist_clone))
                    .custom_cell(DataTableCell::widget(move |ui: &mut egui::Ui| {
                        Self::render_action_buttons_static(ui, &pkg_id, &pkg_clone, None, unsafe_app_remove, expert_app_remove, true, &uad_lists_clone);
                    }));

                // Add drawer if UAD description exists
                if let Some(uad_entry) = uad_lists_for_drawer.as_ref().and_then(|lists| lists.apps.get(&pkg_id_for_drawer)) {
                    let description = uad_entry.description.clone();
                    row_builder = row_builder.drawer(move |ui| {
                        ui.add_space(8.0);
                        ui.label("Description:");
                        ui.add(egui::Label::new(&description).wrap());
                    });
                }
                row_builder
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

        // Pagination controls
        if total_pages > 1 {
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                if ui.button("◀").clicked() && self.current_page > 0 {
                    self.current_page -= 1;
                }
                ui.add_space(10.0);
                ui.label(format!("Page {} of {} ({} items)", self.current_page + 1, total_pages, total_items));
                ui.add_space(10.0);
                if ui.button("▶").clicked() && self.current_page + 1 < total_pages {
                    self.current_page += 1;
                }
            });
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

    fn render_apps_table(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        installed_packages: &[PackageFingerprint],
        category: &DashCounterCategory,
        _clicked_package_idx: Arc<Mutex<Option<usize>>>,
    ) {
        use crate::material_symbol_icons::{ICON_DOWNLOAD, ICON_INFO, ICON_DELETE};

        let category_name = match category {
            DashCounterCategory::OffaCategory(ref name) | DashCounterCategory::FmhyCategory(ref name) => name.clone(),
            _ => return,
        };

        // Filter apps by text filter
        let mut filtered_apps: Vec<_> = self.offa_apps.iter()
            .filter(|app| {
                if self.text_filter.is_empty() {
                    true
                } else {
                    let filter_lower = self.text_filter.to_lowercase();
                    app.name.to_lowercase().contains(&filter_lower) ||
                    app.category.to_lowercase().contains(&filter_lower) ||
                    app.package_name.as_ref().map_or(false, |p| p.to_lowercase().contains(&filter_lower))
                }
            })
            .collect();

        // Sort if needed
        if let Some(col) = self.sort_column {
            filtered_apps.sort_by(|a, b| {
                let ordering = match col {
                    0 => a.name.cmp(&b.name),
                    _ => std::cmp::Ordering::Equal,
                };
                if self.sort_ascending {
                    ordering
                } else {
                    ordering.reverse()
                }
            });
        }

        // Apply pagination
        let total_filtered = filtered_apps.len();
        let start_idx = self.current_page * self.items_per_page;
        let end_idx = (start_idx + self.items_per_page).min(total_filtered);
        let paginated_apps = if start_idx < total_filtered {
            &filtered_apps[start_idx..end_idx]
        } else {
            &[]
        };

        // Cache management for performance (time-based throttling)
        let current_time = ui.input(|i| i.time);
        if (current_time - self.last_refresh_time) >= self.refresh_interval {
            self.last_refresh_time = current_time;
        }

        // Build datatable
        let available_width = ui.available_width();
        let mut table = data_table()
            .id(egui::Id::new("apps_details_table"))
            .sortable_column("App", available_width * 0.5, false)
            .sortable_column("Links", available_width * 0.25, false)
            .sortable_column("", available_width * 0.25, false);

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

        // Build rows
        for app in paginated_apps {
            let app_clone = (*app).clone();
            let installed_packages_clone = installed_packages.to_vec();

            table = table.row(move |row| {
                // App name column
                let app_name_text = app_clone.name.clone();
                let app_cell = DataTableCell::widget(move |ui: &mut egui::Ui| {
                    ui.label(&app_name_text);
                });

                // Links column
                let app_for_links = app_clone.clone();
                let links_cell = DataTableCell::widget(move |ui: &mut egui::Ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt(format!("offa_links_scroll_{}", app_for_links.name))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                let num_links = app_for_links.links.len();
                                let estimated_width = (num_links as f32) * 44.0;
                                ui.set_min_width(estimated_width);

                                for (url, link_type) in &app_for_links.links {
                                    let svg = match link_type.as_str() {
                                        "fdroid" | "fdroid-downloadable" => FDROID_SVG,
                                        "izzy" | "izzy-downloadable" => IZZYONDROID_SVG,
                                        "github" | "github-downloadable" => GITHUB_SVG,
                                        "gitlab" | "gitlab-downloadable" => GITLAB_SVG,
                                        "googleplay" => GOOGLEPLAY_SVG,
                                        "reddit" => REDDIT_SVG,
                                        "discord" => DISCORD_SVG,
                                        "matrix" => MATRIX_SVG,
                                        "telegram" => TELEGRAM_SVG,
                                        "youtube" => YOUTUBE_SVG,
                                        "source" => SOURCE_SVG,
                                        _ => HOME_SVG,
                                    };

                                    let response = ui
                                        .add(icon_button_standard("")
                                            .svg_data(svg))
                                        .on_hover_text(url.as_str());

                                    if response.clicked() {
                                        #[cfg(not(target_os = "android"))]
                                        {
                                            if let Err(e) = webbrowser::open(url) {
                                                log::error!("Failed to open URL: {}", e);
                                            }
                                        }
                                        #[cfg(target_os = "android")]
                                        {
                                            let _ = webbrowser::open(url);
                                        }
                                    }
                                }
                            });
                        });
                });

                // Actions column (install/info/uninstall) - copied from tab_apps_control.rs:1128-1203
                let app_for_install = app_clone.clone();
                let installed_pkgs = installed_packages_clone.clone();
                let actions_cell = DataTableCell::widget(move |ui: &mut egui::Ui| {
                    // Extract package name from links if not explicitly set
                    let package_name = app_for_install.package_name.clone().or_else(|| {
                        // Try to extract from F-Droid or IzzyOnDroid link
                        for (url, _link_type) in &app_for_install.links {
                            // F-Droid format: https://f-droid.org/packages/com.example.app or /com.example.app/
                            if url.contains("f-droid.org") && url.contains("/packages/") {
                                if let Some(start) = url.find("/packages/") {
                                    let after = &url[start + 10..];
                                    let end = after.find('/').unwrap_or(after.len());
                                    let pkg = after[..end].trim();
                                    if !pkg.is_empty() && pkg.contains('.') {
                                        return Some(pkg.to_string());
                                    }
                                }
                            }
                            // IzzyOnDroid format: https://apt.izzysoft.de/fdroid/index/apk/com.example.app
                            else if url.contains("izzysoft.de") && url.contains("/apk/") {
                                if let Some(start) = url.find("/apk/") {
                                    let after = &url[start + 5..];
                                    let end = after.find('/').unwrap_or(after.len());
                                    let pkg = after[..end].trim();
                                    if !pkg.is_empty() && pkg.contains('.') {
                                        return Some(pkg.to_string());
                                    }
                                }
                            }
                        }
                        None
                    });

                    // Check if app is installed - only use exact package name matching from URLs
                    let (is_installed, installed_pkg_info) = if let Some(ref pkg_name) = package_name {
                        // Exact package name match only
                        if let Some(pkg) = installed_pkgs.iter().find(|p| &p.pkg == pkg_name) {
                            let is_system = pkg.flags.contains("SYSTEM");
                            let enabled_state = pkg.users.first().map(|u| {
                                match u.enabled {
                                    0 => if !u.installed && is_system { "REMOVED_USER" } else { "DEFAULT" },
                                    1 => "ENABLED",
                                    2 => "DISABLED",
                                    3 => "DISABLED_USER",
                                    _ => "UNKNOWN",
                                }
                            }).unwrap_or("UNKNOWN").to_string();
                            (true, Some((pkg_name.clone(), is_system, enabled_state)))
                        } else {
                            (false, None)
                        }
                    } else {
                        // No package name extracted from URL - cannot determine if installed
                        (false, None)
                    };

                        // Get downloadable link for install button
                        let downloadable_link = app_for_install.links.iter()
                            .find(|(_, link_type)| {
                                matches!(link_type.as_str(),
                                    "fdroid-downloadable" | "izzy-downloadable" | "github-downloadable" | "gitlab-downloadable")
                            })
                            .or_else(|| {
                                // Fallback to any fdroid/izzy/github link
                                app_for_install.links.iter().find(|(_, link_type)| {
                                    matches!(link_type.as_str(), "fdroid" | "izzy" | "github" | "gitlab")
                                })
                            })
                            .map(|(url, link_type)| (url.clone(), link_type.clone()));

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;

                        if is_installed {
                            // Info button - open package details dialog
                            if ui.add(icon_button_standard(ICON_INFO.to_string())).on_hover_text(tr!("package-info")).clicked() {
                                if let Some((ref pkg_name, _, _)) = installed_pkg_info {
                                    ui.data_mut(|data| {
                                        data.insert_temp(
                                            egui::Id::new("info_clicked_package"),
                                            pkg_name.clone(),
                                        );
                                    });
                                }
                            }

                            if let Some((ref pkg_name, is_system, ref enabled_state)) = installed_pkg_info {
                                // Enable/disable toggle
                                let pkg_enabled = enabled_state == "DEFAULT" || enabled_state == "ENABLED";
                                let mut enabled = pkg_enabled;
                                if toggle_ui(ui, &mut enabled).clicked() {
                                    if enabled {
                                        ui.data_mut(|data| {
                                            data.insert_temp(
                                                egui::Id::new("enable_clicked_package"),
                                                pkg_name.clone(),
                                            );
                                        });
                                    } else {
                                        ui.data_mut(|data| {
                                            data.insert_temp(
                                                egui::Id::new("disable_clicked_package"),
                                                pkg_name.clone(),
                                            );
                                        });
                                    }
                                }

                                if enabled_state == "DEFAULT" || enabled_state == "ENABLED" {
                                    if ui.add(icon_button_standard(ICON_DELETE.to_string()).icon_color(egui::Color32::from_rgb(211, 47, 47))).on_hover_text(tr!("uninstall")).clicked() {
                                        ui.data_mut(|data| {
                                            data.insert_temp(
                                                egui::Id::new("uninstall_clicked_package"),
                                                pkg_name.clone(),
                                            );
                                            data.insert_temp(
                                                egui::Id::new("uninstall_clicked_is_system"),
                                                is_system,
                                            );
                                            data.insert_temp(
                                                egui::Id::new("uninstall_clicked_app_name"),
                                                app_for_install.name.clone(),
                                            );
                                        });
                                    }
                                }
                            }
                        } else if let Some((ref url, ref link_type)) = downloadable_link {
                            let hover_text = format!("[{}]\n{}", link_type, url);

                            if ui.add(icon_button_standard(ICON_DOWNLOAD.to_string())).on_hover_text(&hover_text).clicked() {
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("install_clicked_app"), app_for_install.clone());
                                });
                            }
                        }
                    });
                });

                row.custom_cell(app_cell)
                    .custom_cell(links_cell)
                    .custom_cell(actions_cell)
            });
        }

        // Show table and handle sorting
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

        // Pagination controls
        if total_filtered > self.items_per_page {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let total_pages = (total_filtered + self.items_per_page - 1) / self.items_per_page;
                if self.current_page > 0 {
                    if ui.button("Previous").clicked() {
                        self.current_page -= 1;
                    }
                }
                ui.label(format!("Page {} of {}", self.current_page + 1, total_pages));
                if self.current_page + 1 < total_pages {
                    if ui.button("Next").clicked() {
                        self.current_page += 1;
                    }
                }
            });
        }
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

    /// Helper to get sort key for VirusTotal scan results
    fn get_vt_sort_key(
        pkg_name: &str,
        vt_state: &Option<Arc<Mutex<HashMap<String, crate::calc_virustotal_stt::ScanStatus>>>>,
    ) -> String {
        if let Some(state) = vt_state {
            if let Ok(scanner_state) = state.lock() {
                if let Some(status) = scanner_state.get(pkg_name) {
                    return match status {
                        crate::calc_virustotal_stt::ScanStatus::Pending => "1-scan-not-scanned".to_string(),
                        crate::calc_virustotal_stt::ScanStatus::Scanning { scanned, total, .. } => {
                            format!("2-scan-scanning-{:04}-{:04}", scanned, total)
                        }
                        crate::calc_virustotal_stt::ScanStatus::Completed(result) => {
                            // Prioritize by severity: malicious > suspicious > clean > error/skip/404
                            let mut has_malicious = false;
                            let mut has_suspicious = false;
                            let mut has_clean = false;
                            let mut has_error = false;
                            let mut has_skip = false;
                            let mut has_404 = false;
                            let mut malicious_count = 0;
                            let mut suspicious_count = 0;

                            for file_result in &result.file_results {
                                if file_result.error.is_some() {
                                    has_error = true;
                                } else if file_result.skipped {
                                    has_skip = true;
                                } else if file_result.not_found {
                                    has_404 = true;
                                } else if file_result.malicious > 0 {
                                    has_malicious = true;
                                    malicious_count += file_result.malicious + file_result.suspicious;
                                } else if file_result.suspicious > 0 {
                                    has_suspicious = true;
                                    suspicious_count += file_result.suspicious;
                                } else {
                                    has_clean = true;
                                }
                            }

                            if has_malicious {
                                format!("3-scan-malicious-{:04}", malicious_count)
                            } else if has_suspicious {
                                format!("4-scan-suspicious-{:04}", suspicious_count)
                            } else if has_clean {
                                "5-scan-clean".to_string()
                            } else if has_error {
                                "6-scan-error".to_string()
                            } else if has_skip {
                                "7-scan-skip".to_string()
                            } else if has_404 {
                                "8-scan-404".to_string()
                            } else {
                                "9-scan-unknown".to_string()
                            }
                        }
                        crate::calc_virustotal_stt::ScanStatus::Error(_) => "6-scan-error".to_string(),
                    };
                }
            }
        }
        "0-scan-not-initialized".to_string()
    }

    /// Helper to get sort key for HybridAnalysis scan results
    fn get_ha_sort_key(
        pkg_name: &str,
        ha_state: &Option<Arc<Mutex<HashMap<String, crate::calc_hybridanalysis_stt::ScanStatus>>>>,
    ) -> String {
        if let Some(state) = ha_state {
            if let Ok(scanner_state) = state.lock() {
                if let Some(status) = scanner_state.get(pkg_name) {
                    return match status {
                        crate::calc_hybridanalysis_stt::ScanStatus::Pending => "1-scan-not-scanned".to_string(),
                        crate::calc_hybridanalysis_stt::ScanStatus::Scanning { scanned, total, .. } => {
                            format!("2-scan-scanning-{:04}-{:04}", scanned, total)
                        }
                        crate::calc_hybridanalysis_stt::ScanStatus::Completed(result) => {
                            if result.file_results.is_empty() {
                                return "9-scan-no-results".to_string();
                            }

                            // Prioritize by severity: malicious > suspicious > whitelisted > no specific threat > other
                            let mut priority = 99;
                            let mut verdict_text = String::new();

                            for file_result in &result.file_results {
                                let (file_priority, file_verdict) = match file_result.verdict.as_str() {
                                    "malicious" => (3, "malicious"),
                                    "suspicious" => (4, "suspicious"),
                                    "whitelisted" => (5, "whitelisted"),
                                    "no specific threat" => (6, "no-specific-threat"),
                                    "no-result" => (7, "no-result"),
                                    "rate_limited" => (8, "rate-limited"),
                                    "submitted" => (9, "submitted"),
                                    "pending_analysis" => (10, "pending-analysis"),
                                    "upload_error" => (11, "upload-error"),
                                    "analysis_error" => (12, "analysis-error"),
                                    "404 Not Found" => (13, "404-not-found"),
                                    "" => (14, "skipped"),
                                    _ => (15, "unknown"),
                                };

                                if file_priority < priority {
                                    priority = file_priority;
                                    verdict_text = file_verdict.to_string();
                                }
                            }

                            format!("{}-{}", priority, verdict_text)
                        }
                        crate::calc_hybridanalysis_stt::ScanStatus::Error(_) => "12-scan-error".to_string(),
                    };
                }
            }
        }
        "0-scan-not-initialized".to_string()
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
                    // Sort by VirusTotal button text
                    let store = get_shared_store();
                    let vt_state = store.get_vt_scanner_state();

                    let text_a = Self::get_vt_sort_key(&a.pkg, &vt_state);
                    let text_b = Self::get_vt_sort_key(&b.pkg, &vt_state);
                    text_a.cmp(&text_b)
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
                    // Sort by HybridAnalysis button text
                    let store = get_shared_store();
                    let ha_state = store.get_ha_scanner_state();

                    let text_a = Self::get_ha_sort_key(&a.pkg, &ha_state);
                    let text_b = Self::get_ha_sort_key(&b.pkg, &ha_state);
                    text_a.cmp(&text_b)
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

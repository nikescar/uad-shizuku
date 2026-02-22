use crate::adb::PackageFingerprint;
use crate::calc_hybridanalysis;
use crate::calc_izzyrisk;
use crate::calc_virustotal;
use crate::db;
use crate::db_hybridanalysis;
use crate::db_virustotal;
use crate::shared_store_stt::get_shared_store;
pub use crate::tab_scan_control_stt::*;
use crate::dlg_package_details::DlgPackageDetails;
use crate::dlg_uninstall_confirm::DlgUninstallConfirm;
use eframe::egui;
use egui_async::Bind;
use egui_i18n::tr;
use egui_material3::{data_table, icon_button_standard, theme::get_global_color, MaterialButton};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

// SVG icons as constants (moved to svg_stt.rs)
use crate::material_symbol_icons::{ICON_INFO, ICON_REFRESH, ICON_DELETE, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::{DESKTOP_MIN_WIDTH, BASE_TABLE_WIDTH};

impl Default for TabScanControl {
    fn default() -> Self {
        Self {
            open: false,
            // NOTE: installed_packages, uad_ng_lists, vt_scanner_state, ha_scanner_state,
            // cached apps, and textures are now in shared_store_stt::SharedStore
            selected_packages: Vec::new(),
            cached_scan_counts: CachedScanCounts::default(),
            package_risk_scores: HashMap::new(),
            izzyrisk_bind: Bind::new(true), // retain = true to keep scores across frames
            package_details_dialog: DlgPackageDetails::new(),
            vt_rate_limiter: None,
            vt_package_paths_cache: None,
            vt_scan_state: ScanStateMachine::default(),
            ha_rate_limiter: None,
            ha_package_paths_cache: None,
            ha_scan_state: ScanStateMachine::default(),
            izzyrisk_scan_state: ScanStateMachine::default(),
            izzyrisk_scan_progress: Arc::new(Mutex::new(None)),
            izzyrisk_scan_cancelled: Arc::new(Mutex::new(false)),
            shared_package_risk_scores: Arc::new(Mutex::new(HashMap::new())),
            vt_api_key: None,
            ha_api_key: None,
            device_serial: None,
            virustotal_submit_enabled: false,
            hybridanalysis_submit_enabled: false,
            sort_column: None,
            sort_ascending: true,
            active_vt_filter: VtFilter::All,
            active_ha_filter: HaFilter::All,
            vt_scan_progress: Arc::new(Mutex::new(None)),
            vt_scan_cancelled: Arc::new(Mutex::new(false)),
            ha_scan_progress: Arc::new(Mutex::new(None)),
            ha_scan_cancelled: Arc::new(Mutex::new(false)),
            show_only_enabled: false,
            hide_system_app: false,
            google_play_renderer_enabled: false,
            fdroid_renderer_enabled: false,
            apkmirror_renderer_enabled: false,
            android_package_renderer_enabled: false,
            text_filter: String::new(),
            unsafe_app_remove: false,
            uninstall_confirm_dialog: DlgUninstallConfirm::default(),
        }
    }
}

impl TabScanControl {
    pub fn update_packages(&mut self, packages: Vec<PackageFingerprint>) {
        // Store packages in shared store
        let store = get_shared_store();
        store.set_installed_packages(packages.clone());

        // Resize selection vector to match package count
        self.selected_packages.resize(packages.len(), false);

        // Calculate risk scores using Bind state machine
        self.calculate_all_risk_scores();

        // Initialize VirusTotal scanner state
        if self.vt_api_key.as_ref().map_or(false, |k| k.len() >= 10) && self.device_serial.is_some() {
            self.run_virustotal();
        }

        // Initialize Hybrid Analysis scanner state
        if self.ha_api_key.as_ref().map_or(false, |k| k.len() >= 10) && self.device_serial.is_some() {
            self.run_hybridanalysis();
        }

        // Clear textures cache when packages are updated (will be reloaded on demand)
        store.clear_all_textures();
    }

    /// Update cached app info in shared store
    pub fn update_cached_app_info(
        &mut self,
        google_play_apps: &HashMap<String, crate::models::GooglePlayApp>,
        fdroid_apps: &HashMap<String, crate::models::FDroidApp>,
        apkmirror_apps: &HashMap<String, crate::models::ApkMirrorApp>,
    ) {
        let store = get_shared_store();
        for (pkg_id, app) in google_play_apps {
            store.set_cached_google_play_app(pkg_id.clone(), app.clone());
        }
        for (pkg_id, app) in fdroid_apps {
            store.set_cached_fdroid_app(pkg_id.clone(), app.clone());
        }
        for (pkg_id, app) in apkmirror_apps {
            store.set_cached_apkmirror_app(pkg_id.clone(), app.clone());
        }
    }

    /// Load texture from base64 encoded icon data
    fn load_texture_from_base64(
        &mut self,
        ctx: &egui::Context,
        pkg_id: &str,
        base64_data: &str,
    ) -> Option<egui::TextureHandle> {
        let store = get_shared_store();

        // Check shared store for existing texture
        if let Some(texture) = store.get_google_play_texture(pkg_id)
            .or_else(|| store.get_fdroid_texture(pkg_id))
            .or_else(|| store.get_apkmirror_texture(pkg_id))
        {
            return Some(texture);
        }

        let raw_base64 = if let Some(comma_pos) = base64_data.find(",") {
            &base64_data[comma_pos + 1..]
        } else {
            base64_data
        };

        match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, raw_base64) {
            Ok(bytes) => match image::load_from_memory(&bytes) {
                Ok(image) => {
                    let size = [image.width() as _, image.height() as _];
                    let image_buffer = image.to_rgba8();
                    let pixels = image_buffer.as_flat_samples();
                    let color_image =
                        egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    let texture = ctx.load_texture(
                        format!("app_icon_{}", pkg_id),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );
                    // Store texture in shared store (use google_play as default)
                    store.set_google_play_texture(pkg_id.to_string(), texture.clone());
                    return Some(texture);
                }
                Err(e) => {
                    log::debug!("Failed to load image for {}: {}", pkg_id, e);
                }
            },
            Err(e) => {
                log::debug!("Failed to decode base64 for {}: {}", pkg_id, e);
            }
        }
        None
    }

    fn load_texture_from_bytes(
        ctx: &egui::Context,
        package_id: &str,
        png_bytes: &[u8],
    ) -> Option<egui::TextureHandle> {
        let store = get_shared_store();

        if let Some(texture) = store.get_android_package_texture(package_id) {
            return Some(texture);
        }

        match image::load_from_memory(png_bytes) {
            Ok(image) => {
                let size = [image.width() as _, image.height() as _];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                let color_image =
                    egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                let texture = ctx.load_texture(
                    format!("ap_{}", package_id),
                    color_image,
                    egui::TextureOptions::LINEAR,
                );
                store.set_android_package_texture(package_id.to_string(), texture.clone());
                Some(texture)
            }
            Err(e) => {
                log::debug!("Failed to load image for {}: {}", package_id, e);
                None
            }
        }
    }

    /// Prepare app info data map for all visible packages
    pub fn prepare_app_info_for_display(
        &mut self,
        ctx: &egui::Context,
        package_ids: &[String],
        system_packages: &std::collections::HashSet<String>,
    ) -> HashMap<String, (Option<egui::TextureHandle>, String, String, Option<String>)> {
        let mut app_data_map = HashMap::new();

        if !self.google_play_renderer_enabled
            && !self.fdroid_renderer_enabled
            && !self.apkmirror_renderer_enabled
            && !self.android_package_renderer_enabled
        {
            return app_data_map;
        }

        let store = get_shared_store();
        let cached_fdroid_apps = store.get_cached_fdroid_apps();
        let cached_google_play_apps = store.get_cached_google_play_apps();
        let cached_apkmirror_apps = store.get_cached_apkmirror_apps();

        let mut apps_to_load: Vec<(String, Option<String>, String, String, Option<String>)> =
            Vec::new();

        for pkg_id in package_ids {
            // Android Package renderer (highest priority on Android)
            if self.android_package_renderer_enabled {
                if let Some(ap_app) = store.get_cached_android_package_app(pkg_id) {
                    let texture = Self::load_texture_from_bytes(ctx, pkg_id, &ap_app.icon_bytes);
                    app_data_map.insert(
                        pkg_id.clone(),
                        (texture, ap_app.label.clone(), pkg_id.clone(), None),
                    );
                    continue;
                } else {
                    #[cfg(target_os = "android")]
                    {
                        if let Some(info) = crate::calc_androidpackage::fetch_android_package_info(pkg_id) {
                            let texture = Self::load_texture_from_bytes(ctx, pkg_id, &info.icon_bytes);
                            store.set_cached_android_package_app(pkg_id.clone(), info.clone());
                            app_data_map.insert(
                                pkg_id.clone(),
                                (texture, info.label.clone(), pkg_id.clone(), None),
                            );
                            continue;
                        }
                    }
                }
            }

            let is_system = system_packages.contains(pkg_id);

            if !is_system {
                if self.fdroid_renderer_enabled {
                    if let Some(fd_app) = cached_fdroid_apps.get(pkg_id) {
                        if fd_app.raw_response != "404" {
                            apps_to_load.push((
                                pkg_id.clone(),
                                fd_app.icon_base64.clone(),
                                fd_app.title.clone(),
                                fd_app.developer.clone(),
                                fd_app.version.clone(),
                            ));
                            continue;
                        }
                    }
                }

                if self.google_play_renderer_enabled {
                    if let Some(gp_app) = cached_google_play_apps.get(pkg_id) {
                        if gp_app.raw_response != "404" {
                            apps_to_load.push((
                                pkg_id.clone(),
                                gp_app.icon_base64.clone(),
                                gp_app.title.clone(),
                                gp_app.developer.clone(),
                                gp_app.version.clone(),
                            ));
                            continue;
                        }
                    }
                }
            } else {
                if self.apkmirror_renderer_enabled {
                    if let Some(am_app) = cached_apkmirror_apps.get(pkg_id) {
                        if am_app.raw_response != "404" {
                            apps_to_load.push((
                                pkg_id.clone(),
                                am_app.icon_base64.clone(),
                                am_app.title.clone(),
                                am_app.developer.clone(),
                                am_app.version.clone(),
                            ));
                            continue;
                        }
                    }
                }
            }
        }

        for (pkg_id, icon_base64, title, developer, version) in apps_to_load {
            let texture = icon_base64
                .as_ref()
                .and_then(|b64| self.load_texture_from_base64(ctx, &pkg_id, b64));
            app_data_map.insert(pkg_id, (texture, title, developer, version));
        }

        app_data_map
    }

    fn run_virustotal(&mut self) {
        let store = get_shared_store();
        let installed_packages = store.get_installed_packages();

        if let Some(ref device) = self.device_serial {
            let api_key = self.vt_api_key.clone().unwrap();

            // Start state machine
            self.vt_scan_state.start();

            // Call the new function from calc_virustotal
            let (scanner_state, rate_limiter) = calc_virustotal::run_virustotal(
                installed_packages,
                device.clone(),
                api_key,
                self.virustotal_submit_enabled,
                self.package_risk_scores.clone(),
                self.vt_scan_progress.clone(),
                self.vt_scan_cancelled.clone(),
            );

            // Store scanner state in shared store
            store.set_vt_scanner_state(Some(scanner_state));
            self.vt_rate_limiter = Some(rate_limiter);
        }
    }

    fn run_hybridanalysis(&mut self) {
        let store = get_shared_store();
        let installed_packages = store.get_installed_packages();

        if let Some(ref device) = self.device_serial {
            let api_key = self.ha_api_key.clone().unwrap();

            // Start state machine
            self.ha_scan_state.start();

            // Call the new function from calc_hybridanalysis
            let (scanner_state, rate_limiter) = calc_hybridanalysis::run_hybridanalysis(
                installed_packages,
                device.clone(),
                api_key,
                self.hybridanalysis_submit_enabled,
                self.package_risk_scores.clone(),
                self.ha_scan_progress.clone(),
                self.ha_scan_cancelled.clone(),
            );

            // Store scanner state in shared store
            store.set_ha_scanner_state(Some(scanner_state));
            self.ha_rate_limiter = Some(rate_limiter);
        }
    }

    pub fn update_uad_ng_lists(&mut self, lists: crate::uad_shizuku_app::UadNgLists) {
        let store = get_shared_store();
        store.set_uad_ng_lists(Some(lists));
    }

    /// Calculate risk scores for all installed packages in background thread
    fn calculate_all_risk_scores(&mut self) {
        // Clear local scores and shared scores
        self.package_risk_scores.clear();
        if let Ok(mut shared) = self.shared_package_risk_scores.lock() {
            shared.clear();
        }

        let store = get_shared_store();
        let device_serial = self.device_serial.clone();
        let installed_packages = store.get_installed_packages();
        
        // Don't start calculation if no device is selected or no packages exist
        if device_serial.is_none() || installed_packages.is_empty() {
            log::debug!("Skipping IzzyRisk calculation: device_serial={:?}, packages_count={}", 
                device_serial, installed_packages.len());
            return;
        }
        
        let shared_scores = self.shared_package_risk_scores.clone();
        let progress_clone = self.izzyrisk_scan_progress.clone();
        let cancelled_clone = self.izzyrisk_scan_cancelled.clone();

        // Start state machine
        self.izzyrisk_scan_state.start();

        if let Ok(mut p) = progress_clone.lock() {
            *p = Some(0.0);
        }
        if let Ok(mut cancelled) = cancelled_clone.lock() {
            *cancelled = false;
        }

        log::info!(
            "Starting IzzyRisk calculation for {} packages",
            installed_packages.len()
        );

        // Call the async function from calc_izzyrisk module
        calc_izzyrisk::calculate_all_risk_scores_async(
            installed_packages,
            device_serial,
            shared_scores,
            progress_clone,
            cancelled_clone,
        );
    }

    /// Get the risk score for a package by name
    fn get_risk_score(&self, package_name: &str) -> i32 {
        // First check local cache
        if let Some(score) = self.package_risk_scores.get(package_name) {
            return *score;
        }
        // Then check shared scores from background thread
        if let Ok(shared) = self.shared_package_risk_scores.lock() {
            if let Some(score) = shared.get(package_name) {
                return *score;
            }
        }
        0
    }

    /// Sync shared risk scores from background thread to local cache
    fn sync_risk_scores(&mut self) {
        if let Ok(shared) = self.shared_package_risk_scores.lock() {
            for (pkg, score) in shared.iter() {
                self.package_risk_scores.insert(pkg.clone(), *score);
            }
        }
    }

    /// Sort packages based on the current sort column and direction
    fn sort_packages(&mut self) {
        if let Some(col_idx) = self.sort_column {
            let store = get_shared_store();
            let vt_scanner_state = store.get_vt_scanner_state();
            let ha_scanner_state = store.get_ha_scanner_state();
            let package_risk_scores = self.package_risk_scores.clone();
            let sort_ascending = self.sort_ascending;

            let mut installed_packages = store.get_installed_packages();
            installed_packages.sort_by(|a, b| {
                let ordering = match col_idx {
                    0 => {
                        let name_a = format!("{} ({})", a.pkg, a.versionName);
                        let name_b = format!("{} ({})", b.pkg, b.versionName);
                        name_a.cmp(&name_b)
                    }
                    1 => {
                        let risk_a = package_risk_scores.get(&a.pkg).copied().unwrap_or(0);
                        let risk_b = package_risk_scores.get(&b.pkg).copied().unwrap_or(0);
                        risk_a.cmp(&risk_b)
                    }
                    2 => {
                        // Sort by: result category * 100_000 + malicious * 1000 + suspicious
                        // Result categories: malicious=5, suspicious=4, clean=3, not_found=2, skipped=1, error=0
                        let get_vt_sort_key = |pkg_name: &str| -> i64 {
                            if let Some(ref state) = vt_scanner_state {
                                let state_lock = state.lock().unwrap();
                                match state_lock.get(pkg_name) {
                                    Some(calc_virustotal::ScanStatus::Completed(result)) => {
                                        result
                                            .file_results
                                            .iter()
                                            .map(|fr| {
                                                if fr.skipped {
                                                    1_i64 * 100_000
                                                } else if fr.not_found {
                                                    2_i64 * 100_000
                                                } else if fr.malicious > 0 {
                                                    // Malicious: category 5 + malicious count + suspicious
                                                    5_i64 * 100_000 + (fr.malicious as i64) * 1000 + (fr.suspicious as i64)
                                                } else if fr.suspicious > 0 {
                                                    // Suspicious only: category 4 + suspicious count
                                                    4_i64 * 100_000 + (fr.suspicious as i64) * 1000
                                                } else {
                                                    // Clean: category 3
                                                    3_i64 * 100_000
                                                }
                                            })
                                            .max()
                                            .unwrap_or(0)
                                    }
                                    Some(calc_virustotal::ScanStatus::Error(_)) => -1,
                                    Some(calc_virustotal::ScanStatus::Scanning { .. }) => -2,
                                    Some(calc_virustotal::ScanStatus::Pending) => -3,
                                    None => -4,
                                }
                            } else {
                                -4
                            }
                        };

                        let score_a = get_vt_sort_key(&a.pkg);
                        let score_b = get_vt_sort_key(&b.pkg);
                        score_a.cmp(&score_b)
                    }
                    3 => {
                        // Sort by: verdict_priority * 1_000_000 + threat_score * 1000 + tags_count
                        // Verdict priorities: malicious=5, suspicious=4, no specific threat=3, no-result=2, whitelisted=1
                        let get_ha_sort_key = |pkg_name: &str| -> i64 {
                            if let Some(ref state) = ha_scanner_state {
                                let state_lock = state.lock().unwrap();
                                match state_lock.get(pkg_name) {
                                    Some(calc_hybridanalysis::ScanStatus::Completed(result)) => {
                                        result
                                            .file_results
                                            .iter()
                                            .map(|fr| {
                                                let verdict_priority: i64 = match fr.verdict.as_str() {
                                                    "malicious" => 5,
                                                    "suspicious" => 4,
                                                    "no specific threat" => 3,
                                                    "no-result" => 2,
                                                    "whitelisted" => 1,
                                                    "submitted" => 0,
                                                    _ => 2,
                                                };
                                                let score = fr.threat_score.unwrap_or(0) as i64;
                                                let tags_count = fr.classification_tags.len() as i64;
                                                // Composite key: verdict * 1M + score * 1K + tags
                                                verdict_priority * 1_000_000 + score * 1000 + tags_count
                                            })
                                            .max()
                                            .unwrap_or(-1)
                                    }
                                    Some(calc_hybridanalysis::ScanStatus::Error(_)) => -2,
                                    Some(calc_hybridanalysis::ScanStatus::Scanning { .. }) => -3,
                                    Some(calc_hybridanalysis::ScanStatus::Pending) => -4,
                                    None => -5,
                                }
                            } else {
                                -5
                            }
                        };

                        let score_a = get_ha_sort_key(&a.pkg);
                        let score_b = get_ha_sort_key(&b.pkg);
                        score_a.cmp(&score_b)
                    }
                    _ => std::cmp::Ordering::Equal,
                };

                if sort_ascending {
                    ordering
                } else {
                    ordering.reverse()
                }
            });
            store.set_installed_packages(installed_packages);
        }
    }

    /// Check if a package is enabled (not disabled or removed)
    fn is_package_enabled(package: &PackageFingerprint) -> bool {
        let is_system = package.flags.contains("SYSTEM");
        package
            .users
            .first()
            .map(|u| {
                let is_removed_user = u.enabled == 0 && !u.installed && is_system;
                let is_disabled = u.enabled == 2;
                let is_disabled_user = u.enabled == 3;
                !(is_removed_user || is_disabled || is_disabled_user)
            })
            .unwrap_or(true)
    }

    /// Update cached VT/HA counts if scanner state has changed
    fn update_cached_scan_counts(
        &mut self,
        installed_packages: &[PackageFingerprint],
        vt_scanner_state: &Option<calc_virustotal::ScannerState>,
        ha_scanner_state: &Option<calc_hybridanalysis::ScannerState>,
        ha_tag_ignorelist: &str,
    ) {
        // Check if cache needs updating based on progress changes
        let vt_progress = self.vt_scan_state.progress;
        let ha_progress = self.ha_scan_state.progress;

        // Cache is only valid if progress matches AND at least one scan has been initialized
        // This prevents showing 0 counts when both progresses are None during initialization
        let both_none = vt_progress.is_none() && ha_progress.is_none();
        let has_scanner_state = vt_scanner_state.is_some() || ha_scanner_state.is_some();
        let cache_needs_init = both_none && has_scanner_state && self.cached_scan_counts.vt_counts.0.1 == 0;
        
        let cache_valid = self.cached_scan_counts.vt_progress == vt_progress
            && self.cached_scan_counts.ha_progress == ha_progress
            && !cache_needs_init;

        if cache_valid {
            return;
        }

        // Compute VT counts with enabled/total pairs
        let mut vt_all = (0usize, 0usize);  // (enabled, total)
        let mut vt_malicious = (0usize, 0usize);
        let mut vt_suspicious = (0usize, 0usize);
        let mut vt_safe = (0usize, 0usize);
        let mut vt_not_scanned = (0usize, 0usize);

        if let Some(ref scanner_state) = vt_scanner_state {
            let state = scanner_state.lock().unwrap();
            for package in installed_packages {
                if !self.should_show_package_ha_with_state(package, ha_scanner_state, ha_tag_ignorelist) {
                    continue;
                }

                let is_enabled = Self::is_package_enabled(package);
                vt_all.1 += 1;
                if is_enabled { vt_all.0 += 1; }

                match state.get(&package.pkg) {
                    Some(calc_virustotal::ScanStatus::Completed(result)) => {
                        let mal_count: i32 =
                            result.file_results.iter().map(|fr| fr.malicious).sum();
                        let sus_count: i32 =
                            result.file_results.iter().map(|fr| fr.suspicious).sum();

                        if mal_count > 0 {
                            vt_malicious.1 += 1;
                            if is_enabled { vt_malicious.0 += 1; }
                        } else if sus_count > 0 {
                            vt_suspicious.1 += 1;
                            if is_enabled { vt_suspicious.0 += 1; }
                        } else {
                            vt_safe.1 += 1;
                            if is_enabled { vt_safe.0 += 1; }
                        }
                    }
                    _ => {
                        vt_not_scanned.1 += 1;
                        if is_enabled { vt_not_scanned.0 += 1; }
                    }
                }
            }
        } else {
            for package in installed_packages {
                if self.should_show_package_ha_with_state(package, ha_scanner_state, ha_tag_ignorelist) {
                    let is_enabled = Self::is_package_enabled(package);
                    vt_all.1 += 1;
                    vt_not_scanned.1 += 1;
                    if is_enabled {
                        vt_all.0 += 1;
                        vt_not_scanned.0 += 1;
                    }
                }
            }
        }

        // Helper to check if all tags are ignored
        let check_all_tags_ignored = |file_result: &calc_hybridanalysis::FileScanResult| -> bool {
            let ignorelist_tags: Vec<String> = ha_tag_ignorelist
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();

            if file_result.classification_tags.is_empty() {
                true
            } else {
                file_result.classification_tags.iter().all(|tag| {
                    ignorelist_tags.contains(&tag.to_lowercase())
                })
            }
        };

        // Compute HA counts with enabled/total pairs
        let mut ha_all = (0usize, 0usize);
        let mut ha_malicious = (0usize, 0usize);
        let mut ha_malicious_ignored = (0usize, 0usize);
        let mut ha_suspicious = (0usize, 0usize);
        let mut ha_safe = (0usize, 0usize);
        let mut ha_not_scanned = (0usize, 0usize);

        if let Some(ref scanner_state) = ha_scanner_state {
            let state = scanner_state.lock().unwrap();
            for package in installed_packages {
                if !self.should_show_package_vt_with_state(package, vt_scanner_state) {
                    continue;
                }

                let is_enabled = Self::is_package_enabled(package);
                ha_all.1 += 1;
                if is_enabled { ha_all.0 += 1; }

                match state.get(&package.pkg) {
                    Some(calc_hybridanalysis::ScanStatus::Completed(result)) => {
                        // Check if any file is malicious with/without ignored tags
                        let has_malicious_ignored = result.file_results.iter()
                            .any(|fr| fr.verdict == "malicious" && check_all_tags_ignored(fr));
                        let has_malicious_normal = result.file_results.iter()
                            .any(|fr| fr.verdict == "malicious" && !check_all_tags_ignored(fr));
                        let has_suspicious = result.file_results.iter()
                            .any(|fr| fr.verdict == "suspicious");

                        // Prioritize: malicious_normal > malicious_ignored > suspicious > safe
                        if has_malicious_normal {
                            ha_malicious.1 += 1;
                            if is_enabled { ha_malicious.0 += 1; }
                        } else if has_malicious_ignored {
                            ha_malicious_ignored.1 += 1;
                            if is_enabled { ha_malicious_ignored.0 += 1; }
                        } else if has_suspicious {
                            ha_suspicious.1 += 1;
                            if is_enabled { ha_suspicious.0 += 1; }
                        } else {
                            ha_safe.1 += 1;
                            if is_enabled { ha_safe.0 += 1; }
                        }
                    }
                    _ => {
                        ha_not_scanned.1 += 1;
                        if is_enabled { ha_not_scanned.0 += 1; }
                    }
                }
            }
        } else {
            for package in installed_packages {
                if self.should_show_package_vt_with_state(package, vt_scanner_state) {
                    let is_enabled = Self::is_package_enabled(package);
                    ha_all.1 += 1;
                    ha_not_scanned.1 += 1;
                    if is_enabled {
                        ha_all.0 += 1;
                        ha_not_scanned.0 += 1;
                    }
                }
            }
        }

        // Update cache with (enabled, total) tuples for each category
        self.cached_scan_counts.vt_counts = (vt_all, vt_malicious, vt_suspicious, vt_safe, vt_not_scanned);
        self.cached_scan_counts.ha_counts = (ha_all, ha_malicious, ha_malicious_ignored, ha_suspicious, ha_safe, ha_not_scanned);
        self.cached_scan_counts.vt_progress = vt_progress;
        self.cached_scan_counts.ha_progress = ha_progress;
    }

    /// Returns ((all_enabled, all_total), (mal_enabled, mal_total), (sus_enabled, sus_total), (safe_enabled, safe_total), (not_scanned_enabled, not_scanned_total))
    fn get_vt_counts(&self) -> ((usize, usize), (usize, usize), (usize, usize), (usize, usize), (usize, usize)) {
        self.cached_scan_counts.vt_counts
    }

    /// Returns ((all_enabled, all_total), (mal_enabled, mal_total), (mal_ignored_enabled, mal_ignored_total), (sus_enabled, sus_total), (safe_enabled, safe_total), (not_scanned_enabled, not_scanned_total))
    fn get_ha_counts(&self) -> ((usize, usize), (usize, usize), (usize, usize), (usize, usize), (usize, usize), (usize, usize)) {
        self.cached_scan_counts.ha_counts
    }

    fn should_show_package_vt_with_state(
        &self,
        package: &PackageFingerprint,
        vt_scanner_state: &Option<calc_virustotal::ScannerState>,
    ) -> bool {
        match self.active_vt_filter {
            VtFilter::All => true,
            VtFilter::Malicious => {
                if let Some(ref scanner_state) = vt_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    match state.get(&package.pkg) {
                        Some(calc_virustotal::ScanStatus::Completed(result)) => {
                            result.file_results.iter().map(|fr| fr.malicious).sum::<i32>() > 0
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            VtFilter::Suspicious => {
                if let Some(ref scanner_state) = vt_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    match state.get(&package.pkg) {
                        Some(calc_virustotal::ScanStatus::Completed(result)) => {
                            let mal: i32 = result.file_results.iter().map(|fr| fr.malicious).sum();
                            let sus: i32 = result.file_results.iter().map(|fr| fr.suspicious).sum();
                            mal == 0 && sus > 0
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            VtFilter::Safe => {
                if let Some(ref scanner_state) = vt_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    match state.get(&package.pkg) {
                        Some(calc_virustotal::ScanStatus::Completed(result)) => {
                            let mal: i32 = result.file_results.iter().map(|fr| fr.malicious).sum();
                            let sus: i32 = result.file_results.iter().map(|fr| fr.suspicious).sum();
                            mal == 0 && sus == 0
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            VtFilter::NotScanned => {
                if let Some(ref scanner_state) = vt_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    !matches!(
                        state.get(&package.pkg),
                        Some(calc_virustotal::ScanStatus::Completed(_))
                    )
                } else {
                    true
                }
            }
        }
    }

    fn should_show_package_ha_with_state(
        &self,
        package: &PackageFingerprint,
        ha_scanner_state: &Option<calc_hybridanalysis::ScannerState>,
        ha_tag_ignorelist: &str,
    ) -> bool {
        // Helper function to check if all tags are ignored
        let check_all_tags_ignored = |file_result: &calc_hybridanalysis::FileScanResult| -> bool {
            let ignorelist_tags: Vec<String> = ha_tag_ignorelist
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

        match self.active_ha_filter {
            HaFilter::All => true,
            HaFilter::Malicious => {
                if let Some(ref scanner_state) = ha_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    match state.get(&package.pkg) {
                        Some(calc_hybridanalysis::ScanStatus::Completed(result)) => result
                            .file_results
                            .iter()
                            .any(|fr| fr.verdict == "malicious" && !check_all_tags_ignored(fr)),
                        _ => false,
                    }
                } else {
                    false
                }
            }
            HaFilter::MaliciousIgnored => {
                if let Some(ref scanner_state) = ha_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    match state.get(&package.pkg) {
                        Some(calc_hybridanalysis::ScanStatus::Completed(result)) => result
                            .file_results
                            .iter()
                            .any(|fr| fr.verdict == "malicious" && check_all_tags_ignored(fr)),
                        _ => false,
                    }
                } else {
                    false
                }
            }
            HaFilter::Suspicious => {
                if let Some(ref scanner_state) = ha_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    match state.get(&package.pkg) {
                        Some(calc_hybridanalysis::ScanStatus::Completed(result)) => {
                            !result.file_results.iter().any(|fr| fr.verdict == "malicious")
                                && result.file_results.iter().any(|fr| fr.verdict == "suspicious")
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            HaFilter::Safe => {
                if let Some(ref scanner_state) = ha_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    match state.get(&package.pkg) {
                        Some(calc_hybridanalysis::ScanStatus::Completed(result)) => !result
                            .file_results
                            .iter()
                            .any(|fr| fr.verdict == "malicious" || fr.verdict == "suspicious"),
                        _ => false,
                    }
                } else {
                    false
                }
            }
            HaFilter::NotScanned => {
                if let Some(ref scanner_state) = ha_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    !matches!(
                        state.get(&package.pkg),
                        Some(calc_hybridanalysis::ScanStatus::Completed(_))
                    )
                } else {
                    true
                }
            }
        }
    }

    fn should_show_package_with_state(
        &self,
        package: &PackageFingerprint,
        vt_scanner_state: &Option<calc_virustotal::ScannerState>,
        ha_scanner_state: &Option<calc_hybridanalysis::ScannerState>,
        ha_tag_ignorelist: &str,
    ) -> bool {
        if self.hide_system_app && package.flags.contains("SYSTEM") {
            return false;
        }

        if self.show_only_enabled {
            let is_system = package.flags.contains("SYSTEM");
            let should_show = package
                .users
                .first()
                .map(|user| {
                    let enabled = user.enabled;
                    let installed = user.installed;

                    let is_removed_user = enabled == 0 && !installed && is_system;
                    let is_disabled = enabled == 2;
                    let is_disabled_user = enabled == 3;

                    !(is_removed_user || is_disabled || is_disabled_user)
                })
                .unwrap_or(false);

            if !should_show {
                return false;
            }
        }

        self.should_show_package_vt_with_state(package, vt_scanner_state)
            && self.should_show_package_ha_with_state(package, ha_scanner_state, ha_tag_ignorelist)
    }

    // Legacy methods that fetch from store (used by get_vt_counts/get_ha_counts)
    fn should_show_package_vt(&self, package: &PackageFingerprint) -> bool {
        let store = get_shared_store();
        let vt_scanner_state = store.get_vt_scanner_state();
        self.should_show_package_vt_with_state(package, &vt_scanner_state)
    }

    fn should_show_package_ha(&self, package: &PackageFingerprint) -> bool {
        let store = get_shared_store();
        let ha_scanner_state = store.get_ha_scanner_state();
        // Legacy method - uses empty ignorelist for backward compatibility
        self.should_show_package_ha_with_state(package, &ha_scanner_state, "")
    }

    fn should_show_package(&self, package: &PackageFingerprint) -> bool {
        let store = get_shared_store();
        let vt_scanner_state = store.get_vt_scanner_state();
        let ha_scanner_state = store.get_ha_scanner_state();
        // Legacy method - uses empty ignorelist for backward compatibility
        self.should_show_package_with_state(package, &vt_scanner_state, &ha_scanner_state, "")
    }

    fn matches_text_filter_with_cache(
        &self,
        package: &PackageFingerprint,
        cached_fdroid_apps: &HashMap<String, crate::models::FDroidApp>,
        cached_google_play_apps: &HashMap<String, crate::models::GooglePlayApp>,
        cached_apkmirror_apps: &HashMap<String, crate::models::ApkMirrorApp>,
    ) -> bool {
        if self.text_filter.is_empty() {
            return true;
        }

        let filter_lower = self.text_filter.to_lowercase();

        // Check package name and version
        let package_name = format!("{} ({})", package.pkg, package.versionName).to_lowercase();
        if package_name.contains(&filter_lower) {
            return true;
        }

        // Check IzzyRisk score
        let risk_score = self.get_risk_score(&package.pkg);
        if risk_score.to_string().contains(&filter_lower) {
            return true;
        }

        // Check cached app info (title and developer) - use pre-fetched maps
        let is_system = package.flags.contains("SYSTEM");

        if !is_system {
            if let Some(fd_app) = cached_fdroid_apps.get(&package.pkg) {
                if fd_app.raw_response != "404" {
                    if fd_app.title.to_lowercase().contains(&filter_lower) {
                        return true;
                    }
                    if fd_app.developer.to_lowercase().contains(&filter_lower) {
                        return true;
                    }
                }
            }

            if let Some(gp_app) = cached_google_play_apps.get(&package.pkg) {
                if gp_app.raw_response != "404" {
                    if gp_app.title.to_lowercase().contains(&filter_lower) {
                        return true;
                    }
                    if gp_app.developer.to_lowercase().contains(&filter_lower) {
                        return true;
                    }
                }
            }
        } else {
            if let Some(am_app) = cached_apkmirror_apps.get(&package.pkg) {
                if am_app.raw_response != "404" {
                    if am_app.title.to_lowercase().contains(&filter_lower) {
                        return true;
                    }
                    if am_app.developer.to_lowercase().contains(&filter_lower) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn matches_text_filter(&self, package: &PackageFingerprint) -> bool {
        let store = get_shared_store();
        let cached_fdroid_apps = store.get_cached_fdroid_apps();
        let cached_google_play_apps = store.get_cached_google_play_apps();
        let cached_apkmirror_apps = store.get_cached_apkmirror_apps();
        self.matches_text_filter_with_cache(
            package,
            &cached_fdroid_apps,
            &cached_google_play_apps,
            &cached_apkmirror_apps,
        )
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, hybridanalysis_tag_ignorelist: &str) {
        // Note: Progress sync is now done in uad_shizuku_app.sync_scan_progress() before rendering
        // to ensure progress bars hide immediately when background tasks complete
        
        // Sync risk scores from background thread
        self.sync_risk_scores();

        // Pre-fetch data once at the start to avoid repeated clones
        let hybridanalysis_tag_ignorelist = hybridanalysis_tag_ignorelist.to_string();
        let shared_store = crate::shared_store_stt::get_shared_store();
        let installed_packages = shared_store.get_installed_packages();
        let vt_scanner_state = shared_store.get_vt_scanner_state();
        let ha_scanner_state = shared_store.get_ha_scanner_state();
        let uad_ng_lists = shared_store.get_uad_ng_lists();

        // Pre-fetch cached app data maps for efficient lookups
        let cached_fdroid_apps = shared_store.get_cached_fdroid_apps();
        let cached_google_play_apps = shared_store.get_cached_google_play_apps();
        let cached_apkmirror_apps = shared_store.get_cached_apkmirror_apps();

        // Update cached scan counts if needed (only recomputes when scanner progress changes)
        self.update_cached_scan_counts(&installed_packages, &vt_scanner_state, &ha_scanner_state, &hybridanalysis_tag_ignorelist);

        // Check if mobile view for filter button style
        let filter_is_mobile = ui.available_width() < DESKTOP_MIN_WIDTH;
        
        if !installed_packages.is_empty() {

            // VirusTotal Filter Buttons
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(130.0);
                    ui.label(tr!("virustotal-filter"));
                });

                let (all, malicious, suspicious, safe, not_scanned) = self.get_vt_counts();
                let all_text = tr!("all", { enabled: all.0, total: all.1 });
                let mal_text = tr!("malicious", { enabled: malicious.0, total: malicious.1 });
                let sus_text = tr!("suspicious", { enabled: suspicious.0, total: suspicious.1 });
                let safe_text = tr!("safe", { enabled: safe.0, total: safe.1 });
                let not_scanned_text = tr!("not-scanned", { enabled: not_scanned.0, total: not_scanned.1 });

                if filter_is_mobile {
                    // Mobile: use small MaterialButton with custom colors (same as desktop)
                    let show_all_colors = self.active_vt_filter == VtFilter::All;

                    let button = if self.active_vt_filter == VtFilter::All {
                        MaterialButton::filled(&all_text).small().fill(egui::Color32::from_rgb(158, 158, 158))
                    } else {
                        MaterialButton::outlined(&all_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::All;
                    }

                    let button = if self.active_vt_filter == VtFilter::Malicious || show_all_colors {
                        MaterialButton::filled(&mal_text).small().fill(egui::Color32::from_rgb(211, 47, 47))
                    } else {
                        MaterialButton::outlined(&mal_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::Malicious;
                    }

                    let button = if self.active_vt_filter == VtFilter::Suspicious || show_all_colors {
                        MaterialButton::filled(&sus_text).small().fill(egui::Color32::from_rgb(255, 152, 0))
                    } else {
                        MaterialButton::outlined(&sus_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::Suspicious;
                    }

                    let button = if self.active_vt_filter == VtFilter::Safe || show_all_colors {
                        MaterialButton::filled(&safe_text).small().fill(egui::Color32::from_rgb(56, 142, 60))
                    } else {
                        MaterialButton::outlined(&safe_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::Safe;
                    }

                    let button = if self.active_vt_filter == VtFilter::NotScanned || show_all_colors {
                        MaterialButton::filled(&not_scanned_text).small().fill(egui::Color32::from_rgb(128, 128, 128))
                    } else {
                        MaterialButton::outlined(&not_scanned_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::NotScanned;
                    }
                } else {
                    // Desktop: use small MaterialButton with custom colors
                    let show_all_colors = self.active_vt_filter == VtFilter::All;

                    let button = if self.active_vt_filter == VtFilter::All {
                        MaterialButton::filled(&all_text).small().fill(egui::Color32::from_rgb(158, 158, 158))
                    } else {
                        MaterialButton::outlined(&all_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::All;
                    }

                    let button = if self.active_vt_filter == VtFilter::Malicious || show_all_colors {
                        MaterialButton::filled(&mal_text).small().fill(egui::Color32::from_rgb(211, 47, 47))
                    } else {
                        MaterialButton::outlined(&mal_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::Malicious;
                    }

                    let button = if self.active_vt_filter == VtFilter::Suspicious || show_all_colors {
                        MaterialButton::filled(&sus_text).small().fill(egui::Color32::from_rgb(255, 152, 0))
                    } else {
                        MaterialButton::outlined(&sus_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::Suspicious;
                    }

                    let button = if self.active_vt_filter == VtFilter::Safe || show_all_colors {
                        MaterialButton::filled(&safe_text).small().fill(egui::Color32::from_rgb(56, 142, 60))
                    } else {
                        MaterialButton::outlined(&safe_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::Safe;
                    }

                    let button = if self.active_vt_filter == VtFilter::NotScanned || show_all_colors {
                        MaterialButton::filled(&not_scanned_text).small().fill(egui::Color32::from_rgb(128, 128, 128))
                    } else {
                        MaterialButton::outlined(&not_scanned_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_vt_filter = VtFilter::NotScanned;
                    }
                }
            });
            ui.add_space(5.0);

            // Hybrid Analysis Filter Buttons
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(130.0);
                    ui.label(tr!("hybrid-analysis-filter"));
                });

                let (all, malicious, malicious_ignored, suspicious, safe, not_scanned) = self.get_ha_counts();
                let all_text = tr!("all", { enabled: all.0, total: all.1 });
                let mal_text = tr!("malicious", { enabled: malicious.0, total: malicious.1 });
                let mal_ignored_text = tr!("malicious-ignored", { enabled: malicious_ignored.0, total: malicious_ignored.1 });
                let sus_text = tr!("suspicious", { enabled: suspicious.0, total: suspicious.1 });
                let safe_text = tr!("no-specific-threat", { enabled: safe.0, total: safe.1 });
                let not_scanned_text = tr!("not-scanned", { enabled: not_scanned.0, total: not_scanned.1 });

                if filter_is_mobile {
                    // Mobile: use small MaterialButton with custom colors (same as desktop)
                    let show_all_colors = self.active_ha_filter == HaFilter::All;

                    let button = if self.active_ha_filter == HaFilter::All {
                        MaterialButton::filled(&all_text).small().fill(egui::Color32::from_rgb(158, 158, 158))
                    } else {
                        MaterialButton::outlined(&all_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::All;
                    }

                    let button = if self.active_ha_filter == HaFilter::Malicious || show_all_colors {
                        MaterialButton::filled(&mal_text).small().fill(egui::Color32::from_rgb(211, 47, 47))
                    } else {
                        MaterialButton::outlined(&mal_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::Malicious;
                    }

                    let button = if self.active_ha_filter == HaFilter::MaliciousIgnored || show_all_colors {
                        MaterialButton::filled(&mal_ignored_text).small().fill(egui::Color32::from_rgb(128, 128, 128))
                    } else {
                        MaterialButton::outlined(&mal_ignored_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::MaliciousIgnored;
                    }

                    let button = if self.active_ha_filter == HaFilter::Suspicious || show_all_colors {
                        MaterialButton::filled(&sus_text).small().fill(egui::Color32::from_rgb(255, 152, 0))
                    } else {
                        MaterialButton::outlined(&sus_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::Suspicious;
                    }

                    let button = if self.active_ha_filter == HaFilter::Safe || show_all_colors {
                        MaterialButton::filled(&safe_text).small().fill(egui::Color32::from_rgb(0, 150, 136))
                    } else {
                        MaterialButton::outlined(&safe_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::Safe;
                    }

                    let button = if self.active_ha_filter == HaFilter::NotScanned || show_all_colors {
                        MaterialButton::filled(&not_scanned_text).small().fill(egui::Color32::from_rgb(128, 128, 128))
                    } else {
                        MaterialButton::outlined(&not_scanned_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::NotScanned;
                    }
                } else {
                    // Desktop: use small MaterialButton with custom colors
                    let show_all_colors = self.active_ha_filter == HaFilter::All;

                    let button = if self.active_ha_filter == HaFilter::All {
                        MaterialButton::filled(&all_text).small().fill(egui::Color32::from_rgb(158, 158, 158))
                    } else {
                        MaterialButton::outlined(&all_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::All;
                    }

                    let button = if self.active_ha_filter == HaFilter::Malicious || show_all_colors {
                        MaterialButton::filled(&mal_text).small().fill(egui::Color32::from_rgb(211, 47, 47))
                    } else {
                        MaterialButton::outlined(&mal_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::Malicious;
                    }

                    let button = if self.active_ha_filter == HaFilter::MaliciousIgnored || show_all_colors {
                        MaterialButton::filled(&mal_ignored_text).small().fill(egui::Color32::from_rgb(128, 128, 128))
                    } else {
                        MaterialButton::outlined(&mal_ignored_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::MaliciousIgnored;
                    }

                    let button = if self.active_ha_filter == HaFilter::Suspicious || show_all_colors {
                        MaterialButton::filled(&sus_text).small().fill(egui::Color32::from_rgb(255, 152, 0))
                    } else {
                        MaterialButton::outlined(&sus_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::Suspicious;
                    }

                    let button = if self.active_ha_filter == HaFilter::Safe || show_all_colors {
                        MaterialButton::filled(&safe_text).small().fill(egui::Color32::from_rgb(0, 150, 136))
                    } else {
                        MaterialButton::outlined(&safe_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::Safe;
                    }

                    let button = if self.active_ha_filter == HaFilter::NotScanned || show_all_colors {
                        MaterialButton::filled(&not_scanned_text).small().fill(egui::Color32::from_rgb(128, 128, 128))
                    } else {
                        MaterialButton::outlined(&not_scanned_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_ha_filter = HaFilter::NotScanned;
                    }
                }
            });


        }

        if installed_packages.is_empty() {
            ui.label(tr!("no-packages-loaded"));
            return;
        }

        ui.add_space(10.0);

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
            if !self.text_filter.is_empty() && ui.button("✕").clicked() {
                self.text_filter.clear();
            }
        });

        ui.horizontal(|ui| { 
            // Sort buttons for hidden columns in mobile view
            if !filter_is_mobile {
                return;
            }
            
            ui.label(tr!("sort-by"));
            
            // IzzyRisk sort button
            let izzy_selected = self.sort_column == Some(1);
            let izzy_label = if izzy_selected {
                format!("{} {}", tr!("col-izzy-risk"), if self.sort_ascending { "▲" } else { "▼" })
            } else {
                format!("{} {}", tr!("col-izzy-risk"), "▼") // Default descending
            };
            if ui.selectable_label(izzy_selected, izzy_label).clicked() {
                if self.sort_column == Some(1) {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(1);
                    self.sort_ascending = false;
                }
                self.sort_packages();
            }
            
            // VirusTotal sort button
            let vt_selected = self.sort_column == Some(2);
            let vt_label = if vt_selected {
                format!("{} {}", tr!("col-virustotal"), if self.sort_ascending { "▲" } else { "▼" })
            } else {
                format!("{} {}", tr!("col-virustotal"), "▼") // Default descending
            };
            if ui.selectable_label(vt_selected, vt_label).clicked() {
                if self.sort_column == Some(2) {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(2);
                    self.sort_ascending = false;
                }
                self.sort_packages();
            }
            
            // HybridAnalysis sort button
            let ha_selected = self.sort_column == Some(3);
            let ha_label = if ha_selected {
                format!("{} {}", tr!("col-hybrid-analysis"), if self.sort_ascending { "▲" } else { "▼" })
            } else {
                format!("{} {}", tr!("col-hybrid-analysis"), "▼") // Default descending
            };
            if ui.selectable_label(ha_selected, ha_label).clicked() {
                if self.sort_column == Some(3) {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(3);
                    self.sort_ascending = false;
                }
                self.sort_packages();
            }
        }); 

        if self.sort_column.is_some() {
            self.sort_packages();
        }

        let clicked_package_idx = Arc::new(Mutex::new(None::<usize>));

        let visible_package_ids: Vec<String> = installed_packages
            .iter()
            .filter(|p| self.should_show_package_with_state(p, &vt_scanner_state, &ha_scanner_state, &hybridanalysis_tag_ignorelist))
            .filter(|p| self.matches_text_filter_with_cache(p, &cached_fdroid_apps, &cached_google_play_apps, &cached_apkmirror_apps))
            .map(|p| p.pkg.clone())
            .collect();

        let system_packages: std::collections::HashSet<String> = installed_packages
            .iter()
            .filter(|p| p.flags.contains("SYSTEM"))
            .map(|p| p.pkg.clone())
            .collect();

        let app_data_map =
            self.prepare_app_info_for_display(ui.ctx(), &visible_package_ids, &system_packages);

        // Note: vt_scanner_state and ha_scanner_state are already pre-fetched at the start of ui()

        // Get viewport width for responsive design
        let available_width = ui.ctx().content_rect().width();
        let is_desktop = available_width >= DESKTOP_MIN_WIDTH;
        let width_ratio = available_width / BASE_TABLE_WIDTH;

        let mut interactive_table = data_table()
            .id(egui::Id::new("scan_data_table"))
            .default_row_height(if is_desktop { 56.0 } else { 80.0 })
            // .auto_row_height(true)
            .sortable_column(tr!("col-package-name"), if is_desktop { 350.0 * width_ratio } else { available_width * 0.65 }, false);
        if is_desktop {
            interactive_table = interactive_table
                .sortable_column(tr!("col-izzy-risk"), 80.0 * width_ratio, true)
                .sortable_column(tr!("col-virustotal"), 200.0 * width_ratio, false)
                .sortable_column(tr!("col-hybrid-analysis"), 200.0 * width_ratio, false);
        }
        interactive_table = interactive_table
            .sortable_column(tr!("col-tasks"), if is_desktop { 170.0 * width_ratio } else { available_width * 0.3 }, false)
            .allow_selection(false);

        for (idx, package) in installed_packages.iter().enumerate() {
            if !self.should_show_package_with_state(package, &vt_scanner_state, &ha_scanner_state, &hybridanalysis_tag_ignorelist)
                || !self.matches_text_filter_with_cache(package, &cached_fdroid_apps, &cached_google_play_apps, &cached_apkmirror_apps)
            {
                continue;
            }

            let package_name = format!("{} ({})", package.pkg, package.versionName);
            let app_display_data = app_data_map.get(&package.pkg).cloned();
            let risk_score = self.get_risk_score(&package.pkg);
            let izzyrisk = risk_score.to_string();

            // Get scan results for closures
            let vt_scan_result = if let Some(ref scanner_state) = vt_scanner_state {
                scanner_state.lock().unwrap().get(&package.pkg).cloned()
            } else {
                None
            };

            let ha_scan_result = if let Some(ref scanner_state) = ha_scanner_state {
                scanner_state.lock().unwrap().get(&package.pkg).cloned()
            } else {
                None
            };

            let clicked_idx_clone = clicked_package_idx.clone();
            let is_system = package.flags.contains("SYSTEM");
            let enabled = package
                .users
                .get(0)
                .map(|u| match u.enabled {
                    0 => {
                        if !u.installed && is_system {
                            "REMOVED_USER"
                        } else {
                            "DEFAULT"
                        }
                    }
                    1 => "ENABLED",
                    2 => "DISABLED",
                    3 => "DISABLED_USER",
                    _ => "UNKNOWN",
                })
                .unwrap_or("DEFAULT");
            let enabled_str = enabled.to_string();
            let package_name_for_buttons = package.pkg.clone();
            let is_unsafe_blocked = !self.unsafe_app_remove && uad_ng_lists.as_ref()
                .and_then(|lists| lists.apps.get(&package.pkg))
                .map(|app| app.removal == "Unsafe")
                .unwrap_or(false);

            let (app_texture_id, app_title, app_developer, _app_version) =
                if let Some((texture_opt, title, developer, version)) = &app_display_data {
                    (
                        texture_opt.as_ref().map(|t| t.id()),
                        Some(title.clone()),
                        Some(developer.clone()),
                        version.clone(),
                    )
                } else {
                    (None, None, None, None)
                };
            let package_name_for_cell = package_name.clone();

            interactive_table = interactive_table.row(|table_row| {
                // Package Name column
                let vt_result_for_cell = vt_scan_result.clone();
                let ha_result_for_cell = ha_scan_result.clone();
                let izzyrisk_for_cell = izzyrisk.clone();
                let ha_tag_ignorelist_for_cell = hybridanalysis_tag_ignorelist.clone();
                let row_builder = if let (Some(title), Some(developer)) =
                    (app_title.clone(), app_developer.clone())
                {
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        if !is_desktop {
                            ui.add_space(8.0);
                        }
                        ui.horizontal(|ui| {
                            if let Some(tex_id) = app_texture_id {
                                ui.image((tex_id, egui::vec2(38.0, 38.0)));
                            }
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.1;
                                egui::ScrollArea::horizontal()
                                    .id_salt(format!("scan_title_scroll_{}", idx))
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        ui.add(egui::Label::new(egui::RichText::new(&title).strong()).wrap_mode(egui::TextWrapMode::Extend));
                                    });
                                ui.label(
                                    egui::RichText::new(&developer)
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                                
                                if !is_desktop {
                                    ui.add_space(4.0);
                                    egui::ScrollArea::horizontal()
                                        .id_salt(format!("scan_badge_scroll_{}", idx))
                                        .auto_shrink([false, true])
                                        .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 4.0;

                                            // Show IzzyRisk in mobile view
                                            egui::Frame::new()
                                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 158, 158)))
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(format!("Risk:{}", &izzyrisk_for_cell)).size(10.0));
                                                });

                                            // Show VT results in mobile view
                                            match &vt_result_for_cell {
                                                Some(calc_virustotal::ScanStatus::Completed(result)) => {
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
                                                            .corner_radius(6.0)
                                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                                            .show(ui, |ui| {
                                                                ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(10.0));
                                                            });

                                                        let response = ui.interact(
                                                            inner_response.response.rect,
                                                            ui.id().with(format!("m_vt_chip_{}_{}", idx, i)),
                                                            egui::Sense::click()
                                                        );

                                                        if let Some(ref err) = file_result.error {
                                                            response.on_hover_text(format!("{}\n{}", file_result.file_path, err));
                                                        } else {
                                                            if response.clicked() {
                                                                if let Err(err) = webbrowser::open(&file_result.vt_link) {
                                                                    log::error!("Failed to open VirusTotal link: {}", err);
                                                                }
                                                            }
                                                            response.on_hover_text(&file_result.file_path);
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }

                                            // Show HA results in mobile view
                                            match &ha_result_for_cell {
                                                Some(calc_hybridanalysis::ScanStatus::Completed(result)) => {
                                                    for (i, file_result) in result.file_results.iter().enumerate() {
                                                        let text = {
                                                            if file_result.verdict == "upload_error" || file_result.verdict == "analysis_error" {
                                                                if let Some(ref error_msg) = file_result.error_message {
                                                                    if error_msg.contains("File too large") {
                                                                        if let Some(mb_pos) = error_msg.find(" MB ") {
                                                                            if let Some(start) = error_msg[..mb_pos].rfind(|c: char| !c.is_numeric() && c != '.') {
                                                                                let size = &error_msg[start+1..mb_pos+3];
                                                                                tr!("ha-file-too-large", { size: size.to_string() })
                                                                            } else {
                                                                                tr!("ha-file-too-large-default")
                                                                            }
                                                                        } else {
                                                                            tr!("ha-file-too-large-default")
                                                                        }
                                                                    } else if error_msg.contains("No such file or directory") {
                                                                        tr!("ha-pull-failed")
                                                                    } else if error_msg.contains("Failed to create tmp directory") {
                                                                        tr!("ha-temp-dir-error")
                                                                    } else {
                                                                        if file_result.verdict == "upload_error" {
                                                                            tr!("ha-upload-error")
                                                                        } else {
                                                                            tr!("ha-analysis-error")
                                                                        }
                                                                    }
                                                                } else if file_result.verdict == "upload_error" {
                                                                    tr!("ha-upload-error")
                                                                } else {
                                                                    tr!("ha-analysis-error")
                                                                }
                                                            } else {
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
                                                        };

                                                        let ignorelist_tags: Vec<String> = ha_tag_ignorelist_for_cell
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
                                                            "pending_analysis" => egui::Color32::from_rgb(255, 193, 7),
                                                            "analysis_error" | "upload_error" => egui::Color32::from_rgb(211, 47, 47),
                                                            "404 Not Found" | "" => egui::Color32::from_rgb(128, 128, 128),
                                                            _ => egui::Color32::from_rgb(158, 158, 158),
                                                        };

                                                        let inner_response = egui::Frame::new()
                                                            .fill(bg_color)
                                                            .corner_radius(6.0)
                                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                                            .show(ui, |ui| {
                                                                ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(10.0));
                                                            });

                                                        let response = ui.interact(
                                                            inner_response.response.rect,
                                                            ui.id().with(format!("m_ha_chip_{}_{}", idx, i)),
                                                            egui::Sense::click()
                                                        );

                                                        if response.clicked() {
                                                            if let Err(err) = webbrowser::open(&file_result.ha_link) {
                                                                log::error!("Failed to open Hybrid Analysis link: {}", err);
                                                            }
                                                        }

                                                        response.on_hover_text(&file_result.file_path);
                                                    }
                                                }
                                                _ => {}
                                            }
                                        });
                                    });
                                }
                            });
                        });
                        if !is_desktop {
                            ui.add_space(8.0);
                        }
                    })
                } else {
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        if !is_desktop {
                            ui.add_space(8.0);
                        }
                        ui.vertical(|ui| {
                            egui::ScrollArea::horizontal()
                                .id_salt(format!("scan_title_scroll2_{}", idx))
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    ui.add(egui::Label::new(&package_name_for_cell).wrap());
                                });
                            if !is_desktop {
                                ui.add_space(4.0);
                                egui::ScrollArea::horizontal()
                                    .id_salt(format!("scan_badge_scroll2_{}", idx))
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;

                                        // Show IzzyRisk in mobile view
                                        egui::Frame::new()
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 158, 158)))
                                            .corner_radius(6.0)
                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(format!("Risk:{}", &izzyrisk_for_cell)).size(10.0));
                                            });

                                        // Show VT results in mobile view
                                        match &vt_result_for_cell {
                                            Some(calc_virustotal::ScanStatus::Completed(result)) => {
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
                                                        .corner_radius(6.0)
                                                        .inner_margin(egui::Margin::symmetric(8, 3))
                                                        .show(ui, |ui| {
                                                            ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(10.0));
                                                        });

                                                    let response = ui.interact(
                                                        inner_response.response.rect,
                                                        ui.id().with(format!("m2_vt_chip_{}_{}", idx, i)),
                                                        egui::Sense::click()
                                                    );

                                                    if let Some(ref err) = file_result.error {
                                                        response.on_hover_text(format!("{}\n{}", file_result.file_path, err));
                                                    } else {
                                                        if response.clicked() {
                                                            if let Err(err) = webbrowser::open(&file_result.vt_link) {
                                                                log::error!("Failed to open VirusTotal link: {}", err);
                                                            }
                                                        }
                                                        response.on_hover_text(&file_result.file_path);
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }

                                        // Show HA results in mobile view
                                        match &ha_result_for_cell {
                                            Some(calc_hybridanalysis::ScanStatus::Completed(result)) => {
                                                for (i, file_result) in result.file_results.iter().enumerate() {
                                                    let text = {
                                                        if file_result.verdict == "upload_error" || file_result.verdict == "analysis_error" {
                                                            if let Some(ref error_msg) = file_result.error_message {
                                                                if error_msg.contains("File too large") {
                                                                    if let Some(mb_pos) = error_msg.find(" MB ") {
                                                                        if let Some(start) = error_msg[..mb_pos].rfind(|c: char| !c.is_numeric() && c != '.') {
                                                                            let size = &error_msg[start+1..mb_pos+3];
                                                                            tr!("ha-file-too-large", { size: size.to_string() })
                                                                        } else {
                                                                            tr!("ha-file-too-large-default")
                                                                        }
                                                                    } else {
                                                                        tr!("ha-file-too-large-default")
                                                                    }
                                                                } else if error_msg.contains("No such file or directory") {
                                                                    tr!("ha-pull-failed")
                                                                } else if error_msg.contains("Failed to create tmp directory") {
                                                                    tr!("ha-temp-dir-error")
                                                                } else {
                                                                    if file_result.verdict == "upload_error" {
                                                                        tr!("ha-upload-error")
                                                                    } else {
                                                                        tr!("ha-analysis-error")
                                                                    }
                                                                }
                                                            } else if file_result.verdict == "upload_error" {
                                                                tr!("ha-upload-error")
                                                            } else {
                                                                tr!("ha-analysis-error")
                                                            }
                                                        } else {
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
                                                    };

                                                    let ignorelist_tags: Vec<String> = ha_tag_ignorelist_for_cell
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
                                                        "pending_analysis" => egui::Color32::from_rgb(255, 193, 7),
                                                        "analysis_error" | "upload_error" => egui::Color32::from_rgb(211, 47, 47),
                                                        "404 Not Found" | "" => egui::Color32::from_rgb(128, 128, 128),
                                                        _ => egui::Color32::from_rgb(158, 158, 158),
                                                    };

                                                    let inner_response = egui::Frame::new()
                                                        .fill(bg_color)
                                                        .corner_radius(6.0)
                                                        .inner_margin(egui::Margin::symmetric(8, 3))
                                                        .show(ui, |ui| {
                                                            ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(10.0));
                                                        });

                                                    let response = ui.interact(
                                                        inner_response.response.rect,
                                                        ui.id().with(format!("m2_ha_chip_{}_{}", idx, i)),
                                                        egui::Sense::click()
                                                    );

                                                    if response.clicked() {
                                                        if let Err(err) = webbrowser::open(&file_result.ha_link) {
                                                            log::error!("Failed to open Hybrid Analysis link: {}", err);
                                                        }
                                                    }

                                                    response.on_hover_text(&file_result.file_path);
                                                }
                                            }
                                            _ => {}
                                        }
                                    });
                                });
                            }
                        });
                        if !is_desktop {
                            ui.add_space(8.0);
                        }
                    })
                };

                // IzzyRisk column (desktop only)
                let row_builder = if is_desktop {
                row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    ui.label(&izzyrisk);
                })
                } else { row_builder };

                // VirusTotal column (desktop only)
                let vt_result = vt_scan_result.clone();
                let row_builder = if is_desktop { row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt(format!("vt_scroll_{}", idx))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;

                                // State machine pattern for VT column
                                match &vt_result {
                                    None => {
                                        ui.label(tr!("scan-not-initialized"));
                                    }
                                    Some(calc_virustotal::ScanStatus::Pending) => {
                                        ui.label(tr!("scan-not-scanned"));
                                    }
                                    Some(calc_virustotal::ScanStatus::Scanning { scanned, total, .. }) => {
                                        ui.label(tr!("scan-scanning", { scanned: scanned, total: total }));
                                    }
                                    Some(calc_virustotal::ScanStatus::Completed(result)) => {
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
                                                ui.id().with(format!("vt_chip_{}_{}", idx, i)),
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
                                    Some(calc_virustotal::ScanStatus::Error(e)) => {
                                        ui.label(tr!("scan-error-msg", { message: e.clone() }));
                                    }
                                }
                            });
                        });
                })
                } else { row_builder };

                // HybridAnalysis column (desktop only)
                let ha_result = ha_scan_result.clone();
                let ha_tag_ignorelist = hybridanalysis_tag_ignorelist.clone();
                let row_builder = if is_desktop { row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt(format!("ha_scroll_{}", idx))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;

                                // State machine pattern for HA column
                                match &ha_result {
                                    None => {
                                        ui.label(tr!("scan-not-initialized"));
                                    }
                                    Some(calc_hybridanalysis::ScanStatus::Pending) => {
                                        ui.label(tr!("scan-not-scanned"));
                                    }
                                    Some(calc_hybridanalysis::ScanStatus::Scanning { scanned, total, .. }) => {
                                        ui.label(tr!("scan-scanning", { scanned: scanned, total: total }));
                                    }
                                    Some(calc_hybridanalysis::ScanStatus::Completed(result)) => {
                                        if result.file_results.is_empty() {
                                            ui.label(tr!("scan-no-results"));
                                        }
                                        for (i, file_result) in result.file_results.iter().enumerate() {
                                            // Build translated display text
                                            let text = {
                                                // For error states, show translated error message
                                                if file_result.verdict == "upload_error" || file_result.verdict == "analysis_error" {
                                                    if let Some(ref error_msg) = file_result.error_message {
                                                        if error_msg.contains("File too large") {
                                                            if let Some(mb_pos) = error_msg.find(" MB ") {
                                                                if let Some(start) = error_msg[..mb_pos].rfind(|c: char| !c.is_numeric() && c != '.') {
                                                                    let size = &error_msg[start+1..mb_pos+3];
                                                                    tr!("ha-file-too-large", { size: size.to_string() })
                                                                } else {
                                                                    tr!("ha-file-too-large-default")
                                                                }
                                                            } else {
                                                                tr!("ha-file-too-large-default")
                                                            }
                                                        } else if error_msg.contains("No such file or directory") {
                                                            tr!("ha-pull-failed")
                                                        } else if error_msg.contains("Failed to create tmp directory") {
                                                            tr!("ha-temp-dir-error")
                                                        } else {
                                                            if file_result.verdict == "upload_error" {
                                                                tr!("ha-upload-error")
                                                            } else {
                                                                tr!("ha-analysis-error")
                                                            }
                                                        }
                                                    } else if file_result.verdict == "upload_error" {
                                                        tr!("ha-upload-error")
                                                    } else {
                                                        tr!("ha-analysis-error")
                                                    }
                                                } else {
                                                    // Get base translated text
                                                    // Priority: tags first, then score, then plain verdict
                                                    let has_tags = !file_result.classification_tags.is_empty();
                                                    let base_text = if has_tags {
                                                        // Show verdict with tags (e.g., "malicious(rat, jrat)")
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
                                                        // No tags, show verdict with score
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
                                                        // No tags and no score, show plain verdict
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
                                            };
                                        
                                            // Check if all tags are ignored
                                            let ignorelist_tags: Vec<String> = ha_tag_ignorelist
                                                .split(',')
                                                .map(|s| s.trim().to_lowercase())
                                                .filter(|s| !s.is_empty())
                                                .collect();
                                        
                                            let all_tags_ignored = if file_result.classification_tags.is_empty() {
                                                // No tags means we should treat it as ignored
                                                true
                                            } else {
                                                // Check if all tags are in the ignorelist
                                                file_result.classification_tags.iter().all(|tag| {
                                                    ignorelist_tags.contains(&tag.to_lowercase())
                                                })
                                            };
                                        
                                            let bg_color = match file_result.verdict.as_str() {
                                                "malicious" => {
                                                    if all_tags_ignored {
                                                        egui::Color32::from_rgb(128, 128, 128) // Gray for ignored tags
                                                    } else {
                                                        egui::Color32::from_rgb(211, 47, 47) // Red for real malicious
                                                    }
                                                },
                                                "suspicious" => egui::Color32::from_rgb(255, 152, 0),
                                                "whitelisted" => egui::Color32::from_rgb(56, 142, 60),
                                                "no specific threat" => egui::Color32::from_rgb(0, 150, 136),
                                                "no-result" => egui::Color32::from_rgb(158, 158, 158),
                                                "rate_limited" => egui::Color32::from_rgb(156, 39, 176),
                                                "submitted" => egui::Color32::from_rgb(33, 150, 243),
                                                "pending_analysis" => egui::Color32::from_rgb(255, 193, 7),
                                                "analysis_error" | "upload_error" => egui::Color32::from_rgb(211, 47, 47),
                                                "404 Not Found" | "" => egui::Color32::from_rgb(128, 128, 128),
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
                                                ui.id().with(format!("ha_chip_{}_{}", idx, i)),
                                                egui::Sense::click()
                                            );

                                            if response.clicked() {
                                                #[cfg(not(target_os = "android"))]
                                                {
                                                    if let Err(err) = webbrowser::open(&file_result.ha_link) {
                                                        log::error!("Failed to open Hybrid Analysis link: {}", err);
                                                    }
                                                }
                                            }

                                            response.on_hover_text(&file_result.file_path);
                                        }
                                    }
                                    Some(calc_hybridanalysis::ScanStatus::Error(e)) => {
                                        ui.label(tr!("scan-error-msg", { message: e.clone() }));
                                    }
                                }
                            });
                        });
                })
                } else { row_builder };

                // Tasks column
                let row_builder = row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt(format!("scan_task_scroll_{}", idx))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            
                            // Info button - open package details dialog
                            if ui.add(icon_button_standard(ICON_INFO.to_string())).on_hover_text(tr!("package-info")).clicked() {
                                if let Ok(mut clicked) = clicked_idx_clone.lock() {
                                    *clicked = Some(idx);
                                }
                            }

                            // Refresh button - delete scan results and re-queue
                            if ui.add(icon_button_standard(ICON_REFRESH.to_string())).on_hover_text(tr!("refresh-list")).clicked() {
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("refresh_clicked_package"), package_name_for_buttons.clone());
                                });
                            }

                            // Enable/disable toggle
                            let pkg_enabled = enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED");
                            let can_show_toggle = !is_unsafe_blocked || !pkg_enabled;

                            if can_show_toggle {
                                let mut enabled = pkg_enabled;
                                if toggle_ui(ui, &mut enabled).clicked() {
                                    if enabled {
                                        ui.data_mut(|data| {
                                            data.insert_temp(egui::Id::new("enable_clicked_package"), package_name_for_buttons.clone());
                                        });
                                    } else {
                                        ui.data_mut(|data| {
                                            data.insert_temp(egui::Id::new("disable_clicked_package"), package_name_for_buttons.clone());
                                        });
                                    }
                                }
                            }

                            // Uninstall button
                            if (enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED")) && !is_unsafe_blocked {
                                if ui.add(icon_button_standard(ICON_DELETE.to_string()).icon_color(egui::Color32::from_rgb(211, 47, 47))).on_hover_text(tr!("uninstall")).clicked() {
                                    ui.data_mut(|data| {
                                        data.insert_temp(egui::Id::new("uninstall_clicked_package"), package_name_for_buttons.clone());
                                        data.insert_temp(egui::Id::new("uninstall_clicked_is_system"), is_system);
                                    });
                                }
                            }
                        });
                    });
                })
                .id(format!("scan_table_row_{}", idx));

                row_builder
            });
        }

        // Sort column index mapping: self.sort_column uses logical (desktop) indices
        // Desktop: [0=PackageName, 1=IzzyRisk, 2=VT, 3=HA, 4=Tasks]
        // Mobile:  [0=PackageName, 1=Tasks]
        let to_physical = |logical: usize| -> usize {
            if is_desktop { logical } else { match logical { 0 => 0, _ => 1 } }
        };
        let to_logical = |physical: usize| -> usize {
            if is_desktop { physical } else { match physical { 0 => 0, _ => 4 } }
        };

        // Set sort state
        if let Some(sort_col) = self.sort_column {
            use egui_material3::SortDirection;
            let direction = if self.sort_ascending {
                SortDirection::Ascending
            } else {
                SortDirection::Descending
            };
            if is_desktop || sort_col == 0 || sort_col == 4 {
                interactive_table = interactive_table.sort_by(to_physical(sort_col), direction);
            }
        }

        let table_response = interactive_table.show(ui);

        // Sync sort state from widget, but only when sorting by a column the widget knows about.
        // On mobile, hidden columns (1-3) are managed by the mobile sort buttons, not the table widget.
        let mobile_hidden_sort = !is_desktop && matches!(self.sort_column, Some(1..=3));
        if !mobile_hidden_sort {
            let (widget_sort_col, widget_sort_dir) = table_response.sort_state;
            let logical_sort_col = widget_sort_col.map(|c| to_logical(c));
            let widget_sort_ascending =
                matches!(widget_sort_dir, egui_material3::SortDirection::Ascending);

            if logical_sort_col != self.sort_column
                || (logical_sort_col.is_some() && widget_sort_ascending != self.sort_ascending)
            {
                self.sort_column = logical_sort_col;
                self.sort_ascending = widget_sort_ascending;
                if self.sort_column.is_some() {
                    self.sort_packages();
                }
            }
        }

        if let Some(clicked_col) = table_response.column_clicked {
            let logical_clicked = to_logical(clicked_col);
            if self.sort_column == Some(logical_clicked) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(logical_clicked);
                self.sort_ascending = true;
            }
            self.sort_packages();
        }

        if table_response.selected_rows.len() == self.selected_packages.len() {
            self.selected_packages = table_response.selected_rows;
        }
    

        // Handle package info button click
        if let Ok(clicked) = clicked_package_idx.lock() {
            if let Some(idx) = *clicked {
                self.package_details_dialog.open(idx);
            }
        }

        // Handle button clicks
        let mut uninstall_package: Option<String> = None;
        let mut uninstall_is_system: bool = false;
        let mut enable_package: Option<String> = None;
        let mut disable_package: Option<String> = None;
        let mut refresh_package: Option<String> = None;

        ui.data_mut(|data| {
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("uninstall_clicked_package")) {
                uninstall_package = Some(pkg);
                uninstall_is_system = data
                    .get_temp::<bool>(egui::Id::new("uninstall_clicked_is_system"))
                    .unwrap_or(false);
                data.remove::<String>(egui::Id::new("uninstall_clicked_package"));
                data.remove::<bool>(egui::Id::new("uninstall_clicked_is_system"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("enable_clicked_package")) {
                enable_package = Some(pkg);
                data.remove::<String>(egui::Id::new("enable_clicked_package"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("disable_clicked_package")) {
                disable_package = Some(pkg);
                data.remove::<String>(egui::Id::new("disable_clicked_package"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("refresh_clicked_package")) {
                refresh_package = Some(pkg);
                data.remove::<String>(egui::Id::new("refresh_clicked_package"));
            }
        });

        // Open confirm dialog for uninstall
        if let Some(pkg_name) = uninstall_package {
            self.uninstall_confirm_dialog.open_single(pkg_name, uninstall_is_system);
        }

        // Perform enable
        if let Some(pkg_name) = enable_package {
            if let Some(ref device) = self.device_serial {
                match crate::adb::enable_app(&pkg_name, device) {
                    Ok(output) => {
                        log::info!("App enabled successfully: {}", output);

                        let shared_store = crate::shared_store_stt::get_shared_store();
                        let mut installed_packages = shared_store.installed_packages.lock().unwrap();
                        if let Some(pkg) = installed_packages
                            .iter_mut()
                            .find(|p| p.pkg == pkg_name)
                        {
                            for user in pkg.users.iter_mut() {
                                user.enabled = 1;
                                user.installed = true;
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to enable app: {}", e);
                    }
                }
            } else {
                log::error!("No device selected for enable");
            }
        }

        // Perform disable
        if let Some(pkg_name) = disable_package {
            if let Some(ref device) = self.device_serial {
                match crate::adb::disable_app_current_user(&pkg_name, device, None) {
                    Ok(output) => {
                        log::info!("App disabled successfully: {}", output);

                        let shared_store = crate::shared_store_stt::get_shared_store();
                        let mut installed_packages = shared_store.installed_packages.lock().unwrap();
                        if let Some(pkg) = installed_packages
                            .iter_mut()
                            .find(|p| p.pkg == pkg_name)
                        {
                            for user in pkg.users.iter_mut() {
                                user.enabled = 3;
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to disable app: {}", e);
                    }
                }
            } else {
                log::error!("No device selected for disable");
            }
        }

        // Perform refresh (delete scan results and re-scan)
        if let Some(pkg_name) = refresh_package {
            log::info!("Refreshing scan results for: {}", pkg_name);

            // Delete from database
            let mut conn = db::establish_connection();
            if let Err(e) = db_virustotal::delete_results_by_package(&mut conn, &pkg_name) {
                log::error!("Failed to delete VirusTotal results for {}: {}", pkg_name, e);
            } else {
                log::info!("Deleted VirusTotal results for: {}", pkg_name);
            }

            if let Err(e) = db_hybridanalysis::delete_results_by_package(&mut conn, &pkg_name) {
                log::error!("Failed to delete HybridAnalysis results for {}: {}", pkg_name, e);
            } else {
                log::info!("Deleted HybridAnalysis results for: {}", pkg_name);
            }

            // Get package info for scanning
            let shared_store = crate::shared_store_stt::get_shared_store();
            let installed_packages = shared_store.installed_packages.lock().unwrap();
            let package_info = installed_packages.iter().find(|p| p.pkg == pkg_name).cloned();

            if let Some(package) = package_info {
                // Get hashes for the package
                let device_serial = self.device_serial.clone();
                let cached_packages = if let Some(ref serial) = device_serial {
                    crate::db_package_cache::get_cached_packages_with_apk(serial)
                } else {
                    vec![]
                };
                let cached_pkg = cached_packages.iter().find(|cp| cp.pkg_id == pkg_name);

                let mut paths_str = String::new();
                let mut sha256sums_str = String::new();

                if let Some(cp) = cached_pkg {
                    if let (Some(path), Some(sha256)) = (&cp.apk_path, &cp.apk_sha256sum) {
                        paths_str = path.clone();
                        sha256sums_str = sha256.clone();
                    }
                }

                if paths_str.is_empty() || sha256sums_str.is_empty() {
                    paths_str = package.codePath.clone();
                    sha256sums_str = package.pkgChecksum.clone();
                }

                // Get proper hashes if needed
                if let Some(ref serial) = device_serial {
                    let paths: Vec<&str> = paths_str.split(' ').collect();
                    let sha256sums: Vec<&str> = sha256sums_str.split(' ').collect();
                    let needs_directory_scan = paths.iter().any(|p| !p.ends_with(".apk"));
                    let has_invalid_hashes = sha256sums.iter().any(|s| s.len() != 64);

                    if needs_directory_scan || has_invalid_hashes {
                        if let Ok((new_paths, new_sha256sums)) = crate::adb::get_single_package_sha256sum(serial, &pkg_name) {
                            if !new_paths.is_empty() && !new_sha256sums.is_empty() {
                                paths_str = new_paths;
                                sha256sums_str = new_sha256sums;
                            }
                        }
                    }
                }

                let final_paths: Vec<&str> = paths_str.split(' ').collect();
                let final_sha256sums: Vec<&str> = sha256sums_str.split(' ').collect();
                let hashes: Vec<(String, String)> = final_paths
                    .iter()
                    .zip(final_sha256sums.iter())
                    .filter(|(p, s)| !p.is_empty() && s.len() == 64)
                    .map(|(p, s)| (p.to_string(), s.to_string()))
                    .collect();

                // Start VirusTotal scan in background
                let shared_store = crate::shared_store_stt::get_shared_store();
                let vt_scanner_state = shared_store.vt_scanner_state.lock().unwrap().clone();
                if let (Some(ref vt_state), Some(ref vt_limiter), Some(ref api_key), Some(ref serial)) = (
                    &vt_scanner_state,
                    &self.vt_rate_limiter,
                    &self.vt_api_key,
                    &self.device_serial,
                ) {
                    let vt_state_clone = vt_state.clone();
                    let vt_limiter_clone = vt_limiter.clone();
                    let api_key_clone = api_key.clone();
                    let serial_clone = serial.clone();
                    let pkg_name_clone = pkg_name.clone();
                    let hashes_clone = hashes.clone();
                    let vt_submit = self.virustotal_submit_enabled;

                    // Reset state to Pending first
                    if let Ok(mut state) = vt_state.lock() {
                        state.insert(pkg_name.clone(), calc_virustotal::ScanStatus::Pending);
                    }

                    thread::spawn(move || {
                        log::info!("Starting VT re-scan for: {}", pkg_name_clone);
                        if let Err(e) = calc_virustotal::analyze_package(
                            &pkg_name_clone,
                            hashes_clone,
                            &vt_state_clone,
                            &vt_limiter_clone,
                            &api_key_clone,
                            &serial_clone,
                            vt_submit,
                            &None,
                        ) {
                            log::error!("Error re-scanning VT for {}: {}", pkg_name_clone, e);
                        }
                    });
                }

                // Start HybridAnalysis scan in background
                let shared_store = crate::shared_store_stt::get_shared_store();
                let ha_scanner_state = shared_store.ha_scanner_state.lock().unwrap().clone();
                if let (Some(ref ha_state), Some(ref ha_limiter), Some(ref api_key), Some(ref serial)) = (
                    &ha_scanner_state,
                    &self.ha_rate_limiter,
                    &self.ha_api_key,
                    &self.device_serial,
                ) {
                    let ha_state_clone = ha_state.clone();
                    let ha_limiter_clone = ha_limiter.clone();
                    let api_key_clone = api_key.clone();
                    let serial_clone = serial.clone();
                    let pkg_name_clone = pkg_name.clone();
                    let hashes_clone = hashes.clone();
                    let ha_submit = self.hybridanalysis_submit_enabled;

                    // Reset state to Pending first
                    if let Ok(mut state) = ha_state.lock() {
                        state.insert(pkg_name.clone(), calc_hybridanalysis::ScanStatus::Pending);
                    }

                    thread::spawn(move || {
                        log::info!("Starting HA re-scan for: {}", pkg_name_clone);
                        if let Err(e) = calc_hybridanalysis::analyze_package(
                            &pkg_name_clone,
                            hashes_clone,
                            &ha_state_clone,
                            &ha_limiter_clone,
                            &api_key_clone,
                            &serial_clone,
                            ha_submit,
                            &None,
                        ) {
                            log::error!("Error re-scanning HA for {}: {}", pkg_name_clone, e);
                        }
                    });
                }
            }
        }

        // Show uninstall confirm dialog and execute on confirmation
        if self.uninstall_confirm_dialog.show(ui.ctx()) {
            let pkgs = std::mem::take(&mut self.uninstall_confirm_dialog.packages);
            let sys_flags = std::mem::take(&mut self.uninstall_confirm_dialog.is_system);
            self.uninstall_confirm_dialog.reset();

            if let Some(ref device) = self.device_serial {
                for (pkg_name, is_system) in pkgs.into_iter().zip(sys_flags.into_iter()) {
                    let uninstall_result = if is_system {
                        crate::adb::uninstall_app_user(&pkg_name, device, None)
                    } else {
                        crate::adb::uninstall_app(&pkg_name, device)
                    };

                    match uninstall_result {
                        Ok(output) => {
                            log::info!("App uninstalled successfully: {}", output);

                            let shared_store = crate::shared_store_stt::get_shared_store();
                            let mut installed_packages = shared_store.installed_packages.lock().unwrap();
                            let is_system = installed_packages
                                .iter()
                                .find(|p| p.pkg == pkg_name)
                                .map(|p| p.flags.contains("SYSTEM"))
                                .unwrap_or(false);

                            if is_system {
                                if let Some(pkg) = installed_packages
                                    .iter_mut()
                                    .find(|p| p.pkg == pkg_name)
                                {
                                    for user in pkg.users.iter_mut() {
                                        user.installed = false;
                                        user.enabled = 0;
                                    }
                                }
                            } else {
                                installed_packages.retain(|pkg| pkg.pkg != pkg_name);
                                if let Some(idx_to_remove) = installed_packages
                                    .iter()
                                    .position(|p| p.pkg == pkg_name)
                                {
                                    if idx_to_remove < self.selected_packages.len() {
                                        self.selected_packages.remove(idx_to_remove);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to uninstall app({}): {}", pkg_name, e);
                        }
                    }
                }
            } else {
                log::error!("No device selected for uninstall");
            }
        }

        // Show package details dialog
        let shared_store = crate::shared_store_stt::get_shared_store();
        let installed_packages = shared_store.installed_packages.lock().unwrap();
        let uad_ng_lists = shared_store.uad_ng_lists.lock().unwrap();
        self.package_details_dialog
            .show(ui.ctx(), &installed_packages, &uad_ng_lists);
    }
}

/// iOS-style toggle switch
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
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

use crate::adb::PackageFingerprint;
use crate::calc_hybridanalysis::{self};
use crate::calc_izzyrisk;
use crate::calc_virustotal::{self};
use crate::db;
use crate::db_hybridanalysis;
use crate::db_virustotal;
use crate::uad_shizuku_app::UadNgLists;
pub use crate::tab_scan_control_stt::*;
use crate::win_package_details_dialog::PackageDetailsDialog;
use eframe::egui;
use egui_async::Bind;
use egui_i18n::tr;
use egui_material3::{assist_chip, data_table, theme::get_global_color, MaterialButton};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// SVG icons as constants (moved to svg_stt.rs)
use crate::svg_stt::*;

impl Default for TabScanControl {
    fn default() -> Self {
        Self {
            open: false,
            installed_packages: Vec::new(),
            uad_ng_lists: None,
            selected_packages: Vec::new(),
            package_risk_scores: HashMap::new(),
            izzyrisk_bind: Bind::new(true), // retain = true to keep scores across frames
            package_details_dialog: PackageDetailsDialog::new(),
            vt_scanner_state: None,
            vt_rate_limiter: None,
            vt_package_paths_cache: None,
            vt_scan_state: ScanStateMachine::default(),
            ha_scanner_state: None,
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
            cached_google_play_apps: HashMap::new(),
            cached_fdroid_apps: HashMap::new(),
            cached_apkmirror_apps: HashMap::new(),
            app_textures: HashMap::new(),
            show_only_enabled: false,
            hide_system_app: false,
            google_play_renderer_enabled: false,
            fdroid_renderer_enabled: false,
            apkmirror_renderer_enabled: false,
        }
    }
}

impl TabScanControl {
    pub fn update_packages(&mut self, packages: Vec<PackageFingerprint>) {
        self.installed_packages = packages;
        // Resize selection vector to match package count
        self.selected_packages
            .resize(self.installed_packages.len(), false);

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
        self.app_textures.clear();
    }

    /// Update cached app info from TabDebloatControl
    pub fn update_cached_app_info(
        &mut self,
        google_play_apps: &HashMap<String, crate::models::GooglePlayApp>,
        fdroid_apps: &HashMap<String, crate::models::FDroidApp>,
        apkmirror_apps: &HashMap<String, crate::models::ApkMirrorApp>,
    ) {
        self.cached_google_play_apps = google_play_apps.clone();
        self.cached_fdroid_apps = fdroid_apps.clone();
        self.cached_apkmirror_apps = apkmirror_apps.clone();
    }

    /// Load texture from base64 encoded icon data
    fn load_texture_from_base64(
        &mut self,
        ctx: &egui::Context,
        pkg_id: &str,
        base64_data: &str,
    ) -> Option<egui::TextureHandle> {
        if let Some(texture) = self.app_textures.get(pkg_id) {
            return Some(texture.clone());
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
                    self.app_textures
                        .insert(pkg_id.to_string(), texture.clone());
                    return Some(texture);
                }
                Err(e) => {
                    tracing::debug!("Failed to load image for {}: {}", pkg_id, e);
                }
            },
            Err(e) => {
                tracing::debug!("Failed to decode base64 for {}: {}", pkg_id, e);
            }
        }
        None
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
        {
            return app_data_map;
        }

        let mut apps_to_load: Vec<(String, Option<String>, String, String, Option<String>)> =
            Vec::new();

        for pkg_id in package_ids {
            let is_system = system_packages.contains(pkg_id);

            if !is_system {
                if self.fdroid_renderer_enabled {
                    if let Some(fd_app) = self.cached_fdroid_apps.get(pkg_id) {
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
                    if let Some(gp_app) = self.cached_google_play_apps.get(pkg_id) {
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
                    if let Some(am_app) = self.cached_apkmirror_apps.get(pkg_id) {
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
        let package_names: Vec<String> = self
            .installed_packages
            .iter()
            .map(|p| p.pkg.clone())
            .collect();
        self.vt_scanner_state = Some(calc_virustotal::init_scanner_state(&package_names));

        if self.vt_rate_limiter.is_none() {
            self.vt_rate_limiter = Some(Arc::new(Mutex::new(calc_virustotal::RateLimiter::new(
                4,
                Duration::from_secs(60),
                Duration::from_secs(5),
            ))));
        }

        if let Some(ref device) = self.device_serial {
            let device_serial = device.clone();
            let mut installed_packages = self.installed_packages.clone();
            let scanner_state = self.vt_scanner_state.clone().unwrap();
            let api_key = self.vt_api_key.clone().unwrap();
            let rate_limiter = self.vt_rate_limiter.clone().unwrap();
            let virustotal_submit_enabled = self.virustotal_submit_enabled;
            let package_risk_scores = self.package_risk_scores.clone();

            let vt_scan_progress_clone = self.vt_scan_progress.clone();
            let vt_scan_cancelled_clone = self.vt_scan_cancelled.clone();

            // Start state machine
            self.vt_scan_state.start();

            if let Ok(mut p) = vt_scan_progress_clone.lock() {
                *p = Some(0.0);
            }
            if let Ok(mut cancelled) = vt_scan_cancelled_clone.lock() {
                *cancelled = false;
            }

            tracing::info!(
                "Starting VirusTotal scan for {} packages",
                installed_packages.len()
            );

            std::thread::spawn(move || {
                installed_packages.sort_by(|a, b| {
                    let score_a = package_risk_scores.get(&a.pkg).copied().unwrap_or(0);
                    let score_b = package_risk_scores.get(&b.pkg).copied().unwrap_or(0);
                    score_b.cmp(&score_a)
                });

                let cached_packages =
                    crate::db_package_cache::get_cached_packages_with_apk(&device_serial);

                let mut cached_packages_map: HashMap<String, crate::models::PackageInfoCache> =
                    HashMap::new();
                for cp in cached_packages {
                    cached_packages_map.insert(cp.pkg_id.clone(), cp);
                }

                let total = installed_packages.len();
                let mut skipped_cached = 0usize;

                for (i, package) in installed_packages.iter().enumerate() {
                    if let Ok(cancelled) = vt_scan_cancelled_clone.lock() {
                        if *cancelled {
                            tracing::info!("VirusTotal scan cancelled by user");
                            break;
                        }
                    }

                    if let Ok(mut p) = vt_scan_progress_clone.lock() {
                        *p = Some(i as f32 / total as f32);
                    }

                    let pkg_name = &package.pkg;

                    {
                        let s = scanner_state.lock().unwrap();
                        if matches!(
                            s.get(pkg_name),
                            Some(calc_virustotal::ScanStatus::Completed(_))
                        ) {
                            skipped_cached += 1;
                            continue;
                        }
                    }

                    let mut paths_str = String::new();
                    let mut sha256sums_str = String::new();

                    if let Some(cached_pkg) = cached_packages_map.get(pkg_name) {
                        if let (Some(path), Some(sha256)) =
                            (&cached_pkg.apk_path, &cached_pkg.apk_sha256sum)
                        {
                            paths_str = path.clone();
                            sha256sums_str = sha256.clone();
                        }
                    }

                    if paths_str.is_empty() || sha256sums_str.is_empty() {
                        paths_str = package.codePath.clone();
                        sha256sums_str = package.pkgChecksum.clone();
                    }

                    if !paths_str.is_empty() && !sha256sums_str.is_empty() {
                        let paths: Vec<&str> = paths_str.split(' ').collect();
                        let sha256sums: Vec<&str> = sha256sums_str.split(' ').collect();
                        let needs_directory_scan = paths.iter().any(|p| !p.ends_with(".apk"));
                        let has_invalid_hashes = sha256sums.iter().any(|s| s.len() != 64);

                        let (final_paths_str, final_sha256sums_str) =
                            if needs_directory_scan || has_invalid_hashes {
                                match crate::adb::get_single_package_sha256sum(
                                    &device_serial,
                                    pkg_name,
                                ) {
                                    Ok((new_paths, new_sha256sums)) => {
                                        if !new_paths.is_empty() && !new_sha256sums.is_empty() {
                                            (new_paths, new_sha256sums)
                                        } else if !has_invalid_hashes {
                                            (paths_str.clone(), sha256sums_str.clone())
                                        } else {
                                            (String::new(), String::new())
                                        }
                                    }
                                    Err(e) => {
                                        if !has_invalid_hashes {
                                            tracing::warn!(
                                                "Failed to get sha256sums for {}: {}, using cached values",
                                                pkg_name,
                                                e
                                            );
                                            (paths_str.clone(), sha256sums_str.clone())
                                        } else {
                                            tracing::warn!(
                                                "Failed to get sha256sums for {}: {}, skipping",
                                                pkg_name,
                                                e
                                            );
                                            (String::new(), String::new())
                                        }
                                    }
                                }
                            } else {
                                (paths_str.clone(), sha256sums_str.clone())
                            };

                        let final_paths: Vec<&str> = final_paths_str.split(' ').collect();
                        let final_sha256sums: Vec<&str> = final_sha256sums_str.split(' ').collect();

                        let hashes: Vec<(String, String)> = final_paths
                            .iter()
                            .zip(final_sha256sums.iter())
                            .filter(|(p, s)| !p.is_empty() && s.len() == 64)
                            .map(|(p, s)| (p.to_string(), s.to_string()))
                            .collect();

                        tracing::info!(
                            "Analyzing package {} with {} files (Risk: {})",
                            pkg_name,
                            hashes.len(),
                            package_risk_scores.get(pkg_name).copied().unwrap_or(0)
                        );

                        if let Err(e) = calc_virustotal::analyze_package(
                            &pkg_name,
                            hashes,
                            &scanner_state,
                            &rate_limiter,
                            &api_key,
                            &device_serial,
                            virustotal_submit_enabled,
                            &None,
                        ) {
                            tracing::error!("Error analyzing package {}: {}", pkg_name, e);
                        }
                    }
                }

                tracing::info!(
                    "VirusTotal scan complete: {} cached, {} processed",
                    skipped_cached,
                    total - skipped_cached
                );

                if let Ok(mut p) = vt_scan_progress_clone.lock() {
                    *p = None;
                }
            });
        }
    }

    fn run_hybridanalysis(&mut self) {
        let package_names: Vec<String> = self
            .installed_packages
            .iter()
            .map(|p| p.pkg.clone())
            .collect();
        self.ha_scanner_state = Some(calc_hybridanalysis::init_scanner_state(&package_names));

        if self.ha_rate_limiter.is_none() {
            self.ha_rate_limiter = Some(Arc::new(Mutex::new(
                calc_hybridanalysis::RateLimiter::new(Duration::from_secs(3)),
            )));
        }

        if let Some(ref device) = self.device_serial {
            let device_serial = device.clone();
            let mut installed_packages = self.installed_packages.clone();
            let scanner_state = self.ha_scanner_state.clone().unwrap();
            let api_key = self.ha_api_key.clone().unwrap();
            let rate_limiter = self.ha_rate_limiter.clone().unwrap();
            let hybridanalysis_submit_enabled = self.hybridanalysis_submit_enabled;
            let package_risk_scores = self.package_risk_scores.clone();

            let ha_scan_progress_clone = self.ha_scan_progress.clone();
            let ha_scan_cancelled_clone = self.ha_scan_cancelled.clone();

            // Start state machine
            self.ha_scan_state.start();

            if let Ok(mut p) = ha_scan_progress_clone.lock() {
                *p = Some(0.0);
            }
            if let Ok(mut cancelled) = ha_scan_cancelled_clone.lock() {
                *cancelled = false;
            }

            tracing::info!(
                "Starting HybridAnalysis scan for {} packages",
                installed_packages.len()
            );

            std::thread::spawn(move || {
                let mut effective_submit_enabled = hybridanalysis_submit_enabled;
                tracing::info!("Checking Hybrid Analysis API quota...");
                match crate::api_hybridanalysis::check_quota(&api_key) {
                    Ok(quota) => {
                        if let Some(detonation) = quota.detonation {
                            if detonation.quota_reached {
                                tracing::warn!("Hybrid Analysis detonation quota reached!");
                                effective_submit_enabled = false;
                            }
                            if let Some(apikey_info) = detonation.apikey {
                                if apikey_info.quota_reached {
                                    tracing::warn!("Hybrid Analysis API key quota reached!");
                                    effective_submit_enabled = false;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to check Hybrid Analysis quota: {}", e);
                    }
                }

                installed_packages.sort_by(|a, b| {
                    let score_a = package_risk_scores.get(&a.pkg).copied().unwrap_or(0);
                    let score_b = package_risk_scores.get(&b.pkg).copied().unwrap_or(0);
                    score_b.cmp(&score_a)
                });

                let cached_packages =
                    crate::db_package_cache::get_cached_packages_with_apk(&device_serial);

                let mut cached_packages_map: HashMap<String, crate::models::PackageInfoCache> =
                    HashMap::new();
                for cp in cached_packages {
                    cached_packages_map.insert(cp.pkg_id.clone(), cp);
                }

                let total = installed_packages.len();
                let mut skipped_cached = 0usize;

                for (i, package) in installed_packages.iter().enumerate() {
                    if let Ok(cancelled) = ha_scan_cancelled_clone.lock() {
                        if *cancelled {
                            tracing::info!("Hybrid Analysis scan cancelled by user");
                            break;
                        }
                    }

                    if let Ok(mut p) = ha_scan_progress_clone.lock() {
                        *p = Some(i as f32 / total as f32);
                    }

                    let pkg_name = &package.pkg;

                    {
                        let s = scanner_state.lock().unwrap();
                        if matches!(
                            s.get(pkg_name),
                            Some(calc_hybridanalysis::ScanStatus::Completed(_))
                        ) {
                            skipped_cached += 1;
                            continue;
                        }
                    }

                    let mut paths_str = String::new();
                    let mut sha256sums_str = String::new();

                    if let Some(cached_pkg) = cached_packages_map.get(pkg_name) {
                        if let (Some(path), Some(sha256)) =
                            (&cached_pkg.apk_path, &cached_pkg.apk_sha256sum)
                        {
                            paths_str = path.clone();
                            sha256sums_str = sha256.clone();
                        }
                    }

                    if paths_str.is_empty() || sha256sums_str.is_empty() {
                        paths_str = package.codePath.clone();
                        sha256sums_str = package.pkgChecksum.clone();
                    }

                    if !paths_str.is_empty() && !sha256sums_str.is_empty() {
                        let paths: Vec<&str> = paths_str.split(' ').collect();
                        let sha256sums: Vec<&str> = sha256sums_str.split(' ').collect();
                        let needs_directory_scan = paths.iter().any(|p| !p.ends_with(".apk"));
                        let has_invalid_hashes = sha256sums.iter().any(|s| s.len() != 64);

                        let (final_paths_str, final_sha256sums_str) =
                            if needs_directory_scan || has_invalid_hashes {
                                match crate::adb::get_single_package_sha256sum(
                                    &device_serial,
                                    pkg_name,
                                ) {
                                    Ok((new_paths, new_sha256sums)) => {
                                        if !new_paths.is_empty() && !new_sha256sums.is_empty() {
                                            (new_paths, new_sha256sums)
                                        } else if !has_invalid_hashes {
                                            (paths_str.clone(), sha256sums_str.clone())
                                        } else {
                                            (String::new(), String::new())
                                        }
                                    }
                                    Err(e) => {
                                        if !has_invalid_hashes {
                                            tracing::warn!(
                                                "Failed to get sha256sums for {}: {}, using cached",
                                                pkg_name,
                                                e
                                            );
                                            (paths_str.clone(), sha256sums_str.clone())
                                        } else {
                                            tracing::warn!(
                                                "Failed to get sha256sums for {}: {}, skipping",
                                                pkg_name,
                                                e
                                            );
                                            (String::new(), String::new())
                                        }
                                    }
                                }
                            } else {
                                (paths_str.clone(), sha256sums_str.clone())
                            };

                        let final_paths: Vec<&str> = final_paths_str.split(' ').collect();
                        let final_sha256sums: Vec<&str> = final_sha256sums_str.split(' ').collect();

                        let hashes: Vec<(String, String)> = final_paths
                            .iter()
                            .zip(final_sha256sums.iter())
                            .filter(|(p, s)| !p.is_empty() && s.len() == 64)
                            .map(|(p, s)| (p.to_string(), s.to_string()))
                            .collect();

                        tracing::info!(
                            "Analyzing package {} with {} files (Risk: {})",
                            pkg_name,
                            hashes.len(),
                            package_risk_scores.get(pkg_name).copied().unwrap_or(0)
                        );

                        if let Err(e) = calc_hybridanalysis::analyze_package(
                            &pkg_name,
                            hashes,
                            &scanner_state,
                            &rate_limiter,
                            &api_key,
                            &device_serial,
                            effective_submit_enabled,
                            &None,
                        ) {
                            tracing::error!("Error analyzing package {}: {}", pkg_name, e);
                        }
                    } else {
                        tracing::error!("Failed to get path and sha256 for package {}", pkg_name);
                    }
                }

                tracing::info!(
                    "Hybrid Analysis scan complete: {} cached, {} processed",
                    skipped_cached,
                    total - skipped_cached
                );

                // Second pass: poll pending jobs
                tracing::info!("Checking for pending jobs...");
                loop {
                    if let Ok(cancelled) = ha_scan_cancelled_clone.lock() {
                        if *cancelled {
                            tracing::info!("Hybrid Analysis scan cancelled during pending check");
                            break;
                        }
                    }

                    let pending_count = calc_hybridanalysis::check_pending_jobs(
                        &scanner_state,
                        &rate_limiter,
                        &api_key,
                        &None,
                    );

                    if pending_count == 0 {
                        tracing::info!("All pending jobs completed");
                        break;
                    }

                    tracing::info!("{} jobs still pending, waiting 30 seconds", pending_count);

                    for _ in 0..30 {
                        if let Ok(cancelled) = ha_scan_cancelled_clone.lock() {
                            if *cancelled {
                                tracing::info!("Hybrid Analysis scan cancelled during wait");
                                break;
                            }
                        }
                        thread::sleep(Duration::from_secs(1));
                    }
                }

                if let Ok(mut p) = ha_scan_progress_clone.lock() {
                    *p = None;
                }
            });
        }
    }

    pub fn update_uad_ng_lists(&mut self, lists: UadNgLists) {
        self.uad_ng_lists = Some(lists);
    }

    /// Calculate risk scores for all installed packages in background thread
    fn calculate_all_risk_scores(&mut self) {
        // Clear local scores and shared scores
        self.package_risk_scores.clear();
        if let Ok(mut shared) = self.shared_package_risk_scores.lock() {
            shared.clear();
        }

        let device_serial = self.device_serial.clone();
        let installed_packages = self.installed_packages.clone();
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

        tracing::info!(
            "Starting IzzyRisk calculation for {} packages",
            installed_packages.len()
        );

        thread::spawn(move || {
            let device_serial_str = device_serial.as_deref().unwrap_or("");

            let cached_packages_map: HashMap<String, crate::models::PackageInfoCache> =
                if !device_serial_str.is_empty() {
                    crate::db_package_cache::get_all_cached_packages(device_serial_str)
                        .into_iter()
                        .map(|cp| (cp.pkg_id.clone(), cp))
                        .collect()
                } else {
                    HashMap::new()
                };

            let mut cache_hits = 0;
            let mut cache_misses = 0;
            let total = installed_packages.len();

            for (i, package) in installed_packages.iter().enumerate() {
                // Check for cancellation
                if let Ok(cancelled) = cancelled_clone.lock() {
                    if *cancelled {
                        tracing::info!("IzzyRisk calculation cancelled by user");
                        break;
                    }
                }

                // Update progress
                if let Ok(mut p) = progress_clone.lock() {
                    *p = Some(i as f32 / total as f32);
                }

                let risk_score = if device_serial_str.is_empty() {
                    // No device serial: calculate without caching
                    calc_izzyrisk::calculate_izzyrisk(package)
                } else {
                    let cached_pkg = cached_packages_map.get(&package.pkg);
                    let score = calc_izzyrisk::calculate_and_cache_izzyrisk(
                        package,
                        cached_pkg,
                        device_serial_str,
                    );
                    if cached_pkg.and_then(|c| c.izzyscore).is_some() {
                        cache_hits += 1;
                    } else {
                        cache_misses += 1;
                    }
                    score
                };

                // Update shared scores
                if let Ok(mut shared) = shared_scores.lock() {
                    shared.insert(package.pkg.clone(), risk_score);
                }
            }

            tracing::info!(
                "IzzyRisk calculation complete: {} packages ({} cached, {} computed)",
                total,
                cache_hits,
                cache_misses
            );

            // Clear progress when done
            if let Ok(mut p) = progress_clone.lock() {
                *p = None;
            }
        });
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
            let scanner_state = self.vt_scanner_state.clone();
            let package_risk_scores = self.package_risk_scores.clone();

            self.installed_packages.sort_by(|a, b| {
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
                        let get_vt_sort_key = |pkg_name: &str| -> i32 {
                            if let Some(ref state) = scanner_state {
                                let state_lock = state.lock().unwrap();
                                match state_lock.get(pkg_name) {
                                    Some(calc_virustotal::ScanStatus::Completed(result)) => result
                                        .file_results
                                        .iter()
                                        .map(|fr| fr.malicious + fr.suspicious)
                                        .sum::<i32>(),
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
                        let get_ha_sort_key = |pkg_name: &str| -> i32 {
                            if let Some(ref ha_state) = self.ha_scanner_state {
                                let state_lock = ha_state.lock().unwrap();
                                match state_lock.get(pkg_name) {
                                    Some(calc_hybridanalysis::ScanStatus::Completed(result)) => {
                                        result
                                            .file_results
                                            .iter()
                                            .map(|fr| match fr.verdict.as_str() {
                                                "malicious" => 4,
                                                "suspicious" => 3,
                                                "no specific threat" => 2,
                                                "no-result" => 1,
                                                "whitelisted" => 0,
                                                "submitted" => -1,
                                                _ => 1,
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

                if self.sort_ascending {
                    ordering
                } else {
                    ordering.reverse()
                }
            });
        }
    }

    fn get_vt_counts(&self) -> (usize, usize, usize, usize, usize) {
        let mut total = 0;
        let mut malicious = 0;
        let mut suspicious = 0;
        let mut safe = 0;
        let mut not_scanned = 0;

        if let Some(ref scanner_state) = self.vt_scanner_state {
            let state = scanner_state.lock().unwrap();
            for package in &self.installed_packages {
                if !self.should_show_package_ha(package) {
                    continue;
                }

                total += 1;
                match state.get(&package.pkg) {
                    Some(calc_virustotal::ScanStatus::Completed(result)) => {
                        let mal_count: i32 =
                            result.file_results.iter().map(|fr| fr.malicious).sum();
                        let sus_count: i32 =
                            result.file_results.iter().map(|fr| fr.suspicious).sum();

                        if mal_count > 0 {
                            malicious += 1;
                        } else if sus_count > 0 {
                            suspicious += 1;
                        } else {
                            safe += 1;
                        }
                    }
                    _ => not_scanned += 1,
                }
            }
        } else {
            for package in &self.installed_packages {
                if self.should_show_package_ha(package) {
                    total += 1;
                    not_scanned += 1;
                }
            }
        }

        (total, malicious, suspicious, safe, not_scanned)
    }

    fn get_ha_counts(&self) -> (usize, usize, usize, usize, usize) {
        let mut total = 0;
        let mut malicious = 0;
        let mut suspicious = 0;
        let mut safe = 0;
        let mut not_scanned = 0;

        if let Some(ref scanner_state) = self.ha_scanner_state {
            let state = scanner_state.lock().unwrap();
            for package in &self.installed_packages {
                if !self.should_show_package_vt(package) {
                    continue;
                }

                total += 1;
                match state.get(&package.pkg) {
                    Some(calc_hybridanalysis::ScanStatus::Completed(result)) => {
                        let max_severity = result
                            .file_results
                            .iter()
                            .map(|fr| match fr.verdict.as_str() {
                                "malicious" => 2,
                                "suspicious" => 1,
                                _ => 0,
                            })
                            .max()
                            .unwrap_or(0);

                        match max_severity {
                            2 => malicious += 1,
                            1 => suspicious += 1,
                            _ => safe += 1,
                        }
                    }
                    _ => not_scanned += 1,
                }
            }
        } else {
            for package in &self.installed_packages {
                if self.should_show_package_vt(package) {
                    total += 1;
                    not_scanned += 1;
                }
            }
        }

        (total, malicious, suspicious, safe, not_scanned)
    }

    fn should_show_package_vt(&self, package: &PackageFingerprint) -> bool {
        match self.active_vt_filter {
            VtFilter::All => true,
            VtFilter::Malicious => {
                if let Some(ref scanner_state) = self.vt_scanner_state {
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
                if let Some(ref scanner_state) = self.vt_scanner_state {
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
                if let Some(ref scanner_state) = self.vt_scanner_state {
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
                if let Some(ref scanner_state) = self.vt_scanner_state {
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

    fn should_show_package_ha(&self, package: &PackageFingerprint) -> bool {
        match self.active_ha_filter {
            HaFilter::All => true,
            HaFilter::Malicious => {
                if let Some(ref scanner_state) = self.ha_scanner_state {
                    let state = scanner_state.lock().unwrap();
                    match state.get(&package.pkg) {
                        Some(calc_hybridanalysis::ScanStatus::Completed(result)) => result
                            .file_results
                            .iter()
                            .any(|fr| fr.verdict == "malicious"),
                        _ => false,
                    }
                } else {
                    false
                }
            }
            HaFilter::Suspicious => {
                if let Some(ref scanner_state) = self.ha_scanner_state {
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
                if let Some(ref scanner_state) = self.ha_scanner_state {
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
                if let Some(ref scanner_state) = self.ha_scanner_state {
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

    fn should_show_package(&self, package: &PackageFingerprint) -> bool {
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

        self.should_show_package_vt(package) && self.should_show_package_ha(package)
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Sync progress from background threads to state machines
        if let Ok(progress) = self.vt_scan_progress.lock() {
            if let Some(p) = *progress {
                self.vt_scan_state.update_progress(p);
            } else if self.vt_scan_state.is_running {
                self.vt_scan_state.complete();
            }
        }
        if let Ok(progress) = self.ha_scan_progress.lock() {
            if let Some(p) = *progress {
                self.ha_scan_state.update_progress(p);
            } else if self.ha_scan_state.is_running {
                self.ha_scan_state.complete();
            }
        }
        // Sync IzzyRisk progress
        if let Ok(progress) = self.izzyrisk_scan_progress.lock() {
            if let Some(p) = *progress {
                self.izzyrisk_scan_state.update_progress(p);
            } else if self.izzyrisk_scan_state.is_running {
                self.izzyrisk_scan_state.complete();
            }
        }
        // Sync risk scores from background thread
        self.sync_risk_scores();

        // VirusTotal Filter Buttons
        if !self.installed_packages.is_empty() {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    ui.label(tr!("virustotal-filter"));

                    // Show progress using state machine
                    if let Some(p) = self.vt_scan_state.progress {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.horizontal(|ui| {
                            ui.add(progress_bar).on_hover_text(tr!("scanning-packages"));

                            if ui.button("Stop").clicked() {
                                tracing::info!("Stop Virustotal scan clicked");
                                self.vt_scan_state.cancel();
                                if let Ok(mut cancelled) = self.vt_scan_cancelled.lock() {
                                    *cancelled = true;
                                }
                                if let Ok(mut progress) = self.vt_scan_progress.lock() {
                                    *progress = None;
                                }
                            }
                        });
                    }
                });

                let (total, malicious, suspicious, safe, not_scanned) = self.get_vt_counts();
                let show_all_colors = self.active_vt_filter == VtFilter::All;

                let all_text = tr!("all", { count: total });
                let button = if self.active_vt_filter == VtFilter::All {
                    MaterialButton::filled(&all_text)
                        .fill(egui::Color32::from_rgb(158, 158, 158))
                } else {
                    MaterialButton::outlined(&all_text)
                };
                if ui.add(button).clicked() {
                    self.active_vt_filter = VtFilter::All;
                }

                let mal_text = tr!("malicious", { count: malicious });
                let button = if self.active_vt_filter == VtFilter::Malicious || show_all_colors {
                    MaterialButton::filled(&mal_text)
                        .fill(egui::Color32::from_rgb(211, 47, 47))
                } else {
                    MaterialButton::outlined(&mal_text)
                };
                if ui.add(button).clicked() {
                    self.active_vt_filter = VtFilter::Malicious;
                }

                let sus_text = tr!("suspicious", { count: suspicious });
                let button = if self.active_vt_filter == VtFilter::Suspicious || show_all_colors {
                    MaterialButton::filled(&sus_text)
                        .fill(egui::Color32::from_rgb(255, 152, 0))
                } else {
                    MaterialButton::outlined(&sus_text)
                };
                if ui.add(button).clicked() {
                    self.active_vt_filter = VtFilter::Suspicious;
                }

                let safe_text = tr!("safe", { count: safe });
                let button = if self.active_vt_filter == VtFilter::Safe || show_all_colors {
                    MaterialButton::filled(&safe_text)
                        .fill(egui::Color32::from_rgb(56, 142, 60))
                } else {
                    MaterialButton::outlined(&safe_text)
                };
                if ui.add(button).clicked() {
                    self.active_vt_filter = VtFilter::Safe;
                }

                let not_scanned_text = tr!("not-scanned", { count: not_scanned });
                let button = if self.active_vt_filter == VtFilter::NotScanned || show_all_colors {
                    MaterialButton::filled(&not_scanned_text)
                        .fill(egui::Color32::from_rgb(128, 128, 128))
                } else {
                    MaterialButton::outlined(&not_scanned_text)
                };
                if ui.add(button).clicked() {
                    self.active_vt_filter = VtFilter::NotScanned;
                }
            });
            ui.add_space(5.0);

            // Hybrid Analysis Filter Buttons
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(150.0);
                    ui.label(tr!("hybrid-analysis-filter"));

                    // Show progress using state machine
                    if let Some(p) = self.ha_scan_state.progress {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.horizontal(|ui| {
                            ui.add(progress_bar).on_hover_text(tr!("scanning-packages"));

                            if ui.button("Stop").clicked() {
                                tracing::info!("Stop Hybrid Analysis scan clicked");
                                self.ha_scan_state.cancel();
                                if let Ok(mut cancelled) = self.ha_scan_cancelled.lock() {
                                    *cancelled = true;
                                }
                                if let Ok(mut progress) = self.ha_scan_progress.lock() {
                                    *progress = None;
                                }
                            }
                        });
                    }
                });

                let (total, malicious, suspicious, safe, not_scanned) = self.get_ha_counts();
                let show_all_colors = self.active_ha_filter == HaFilter::All;

                let all_text = tr!("all", { count: total });
                let button = if self.active_ha_filter == HaFilter::All {
                    MaterialButton::filled(&all_text)
                        .fill(egui::Color32::from_rgb(158, 158, 158))
                } else {
                    MaterialButton::outlined(&all_text)
                };
                if ui.add(button).clicked() {
                    self.active_ha_filter = HaFilter::All;
                }

                let mal_text = tr!("malicious", { count: malicious });
                let button = if self.active_ha_filter == HaFilter::Malicious || show_all_colors {
                    MaterialButton::filled(&mal_text)
                        .fill(egui::Color32::from_rgb(211, 47, 47))
                } else {
                    MaterialButton::outlined(&mal_text)
                };
                if ui.add(button).clicked() {
                    self.active_ha_filter = HaFilter::Malicious;
                }

                let sus_text = tr!("suspicious", { count: suspicious });
                let button = if self.active_ha_filter == HaFilter::Suspicious || show_all_colors {
                    MaterialButton::filled(&sus_text)
                        .fill(egui::Color32::from_rgb(255, 152, 0))
                } else {
                    MaterialButton::outlined(&sus_text)
                };
                if ui.add(button).clicked() {
                    self.active_ha_filter = HaFilter::Suspicious;
                }

                let safe_text = tr!("no-specific-threat", { count: safe });
                let button = if self.active_ha_filter == HaFilter::Safe || show_all_colors {
                    MaterialButton::filled(&safe_text)
                        .fill(egui::Color32::from_rgb(0, 150, 136))
                } else {
                    MaterialButton::outlined(&safe_text)
                };
                if ui.add(button).clicked() {
                    self.active_ha_filter = HaFilter::Safe;
                }

                let not_scanned_text = tr!("not-scanned", { count: not_scanned });
                let button = if self.active_ha_filter == HaFilter::NotScanned || show_all_colors {
                    MaterialButton::filled(&not_scanned_text)
                        .fill(egui::Color32::from_rgb(128, 128, 128))
                } else {
                    MaterialButton::outlined(&not_scanned_text)
                };
                if ui.add(button).clicked() {
                    self.active_ha_filter = HaFilter::NotScanned;
                }
            });

            // IzzyRisk progress bar
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    if let Some(p) = self.izzyrisk_scan_state.progress {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.set_width(150.0);
                        ui.label(tr!("izzyrisk-calculation"));
                        ui.horizontal(|ui| {
                            ui.add(progress_bar).on_hover_text(tr!("calculating-risk-scores"));

                            if ui.button("Stop").clicked() {
                                tracing::info!("Stop IzzyRisk calculation clicked");
                                self.izzyrisk_scan_state.cancel();
                                if let Ok(mut cancelled) = self.izzyrisk_scan_cancelled.lock() {
                                    *cancelled = true;
                                }
                                if let Ok(mut progress) = self.izzyrisk_scan_progress.lock() {
                                    *progress = None;
                                }
                            }
                        });
                    }
                });
            });
        }

        ui.add_space(10.0);

        if self.installed_packages.is_empty() {
            ui.label(tr!("no-packages-loaded"));
            return;
        }
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label(tr!("show-only-enabled"));
            toggle_ui(ui, &mut self.show_only_enabled);
            ui.add_space(10.0);
            ui.label(tr!("hide-system-app"));
            toggle_ui(ui, &mut self.hide_system_app);
        });
        ui.add_space(10.0);

        // Apply Material theme styling
        let surface = get_global_color("surface");
        let on_surface = get_global_color("onSurface");
        let primary = get_global_color("primary");

        let mut style = (*ui.ctx().style()).clone();
        style.visuals.widgets.noninteractive.bg_fill = surface;
        style.visuals.widgets.inactive.bg_fill = surface;
        style.visuals.widgets.hovered.bg_fill =
            egui::Color32::from_rgba_premultiplied(primary.r(), primary.g(), primary.b(), 20);
        style.visuals.widgets.active.bg_fill =
            egui::Color32::from_rgba_premultiplied(primary.r(), primary.g(), primary.b(), 40);
        style.visuals.selection.bg_fill = primary;
        style.visuals.widgets.noninteractive.fg_stroke.color = on_surface;
        style.visuals.widgets.inactive.fg_stroke.color = on_surface;
        style.visuals.widgets.hovered.fg_stroke.color = on_surface;
        style.visuals.widgets.active.fg_stroke.color = on_surface;
        style.visuals.striped = true;
        style.visuals.faint_bg_color = egui::Color32::from_rgba_premultiplied(
            on_surface.r(),
            on_surface.g(),
            on_surface.b(),
            10,
        );
        ui.ctx().set_style(style);

        if self.sort_column.is_some() {
            self.sort_packages();
        }

        let clicked_package_idx = Arc::new(Mutex::new(None::<usize>));

        let visible_package_ids: Vec<String> = self
            .installed_packages
            .iter()
            .filter(|p| self.should_show_package(p))
            .map(|p| p.pkg.clone())
            .collect();

        let system_packages: std::collections::HashSet<String> = self
            .installed_packages
            .iter()
            .filter(|p| p.flags.contains("SYSTEM"))
            .map(|p| p.pkg.clone())
            .collect();

        let app_data_map =
            self.prepare_app_info_for_display(ui.ctx(), &visible_package_ids, &system_packages);

        // Clone scanner states for use in closures
        let vt_scanner_state = self.vt_scanner_state.clone();
        let ha_scanner_state = self.ha_scanner_state.clone();

        let mut interactive_table = data_table()
            .id(egui::Id::new("scan_data_table"))
            .sortable_column(tr!("col-package-name"), 350.0, false)
            .sortable_column(tr!("col-izzy-risk"), 80.0, true)
            .sortable_column(tr!("col-virustotal"), 200.0, false)
            .sortable_column(tr!("col-hybrid-analysis"), 200.0, false)
            .sortable_column(tr!("col-tasks"), 170.0, false)
            .allow_selection(false);

        for (idx, package) in self.installed_packages.iter().enumerate() {
            if !self.should_show_package(package) {
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
                let row_builder = if let (Some(title), Some(developer)) =
                    (app_title.clone(), app_developer.clone())
                {
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        ui.horizontal(|ui| {
                            if let Some(tex_id) = app_texture_id {
                                ui.image((tex_id, egui::vec2(38.0, 38.0)));
                            }
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.1;
                                ui.label(egui::RichText::new(&title).strong());
                                ui.label(
                                    egui::RichText::new(&developer)
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                            });
                        });
                    })
                } else {
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        ui.add(egui::Label::new(&package_name_for_cell).wrap());
                    })
                };

                // IzzyRisk column
                let row_builder = row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    ui.label(&izzyrisk);
                });

                // VirusTotal column with state machine rendering
                let vt_result = vt_scan_result.clone();
                let row_builder = row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt(format!("vt_scroll_{}", idx))
                        .auto_shrink([false, false])
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
                                                        if let Err(err) = open::that(&file_result.vt_link) {
                                                            tracing::error!("Failed to open VirusTotal link: {}", err);
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
                });

                // HybridAnalysis column with state machine rendering
                let ha_result = ha_scan_result.clone();
                let row_builder = row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt(format!("ha_scroll_{}", idx))
                        .auto_shrink([false, false])
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
                                                    let base_text = if let Some(score) = file_result.threat_score {
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
                                            };
                                            let bg_color = match file_result.verdict.as_str() {
                                                "malicious" => egui::Color32::from_rgb(211, 47, 47),
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
                                                    if let Err(err) = open::that(&file_result.ha_link) {
                                                        tracing::error!("Failed to open Hybrid Analysis link: {}", err);
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
                });

                // Tasks column
                let row_builder = row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    ui.horizontal(|ui| {
                        // Refresh chip - delete scan results and re-queue
                        let refresh_chip = assist_chip("")
                            .leading_icon_svg(REFRESH_SVG)
                            .elevated(true);
                        let pkg_name_refresh = package_name_for_buttons.clone();
                        let refresh_response = ui.add(refresh_chip.on_click(|| {
                            tracing::info!("Refresh clicked for: {}", pkg_name_refresh);
                        }));
                        if refresh_response.clicked() {
                            ui.data_mut(|data| {
                                data.insert_temp(egui::Id::new("refresh_clicked_package"), package_name_for_buttons.clone());
                            });
                        }
                        refresh_response.on_hover_text(tr!("refresh-scan"));

                        // let chip = assist_chip("")
                        //     .leading_icon_svg(INFO_SVG)
                        //     .elevated(true);
                        // if ui.add(chip.on_click(|| {
                        //     tracing::info!("Opening package info dialog");
                        // })).clicked() {
                        //     if let Ok(mut clicked) = clicked_idx_clone.lock() {
                        //         *clicked = Some(idx);
                        //     }
                        // }

                        if enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED") {
                            let uninstall_chip = assist_chip("")
                                .leading_icon_svg(TRASH_RED_SVG)
                                .elevated(true);

                            let pkg_name_uninstall = package_name_for_buttons.clone();
                            if ui.add(uninstall_chip.on_click(|| {
                                tracing::info!("Uninstall clicked for: {}", pkg_name_uninstall);
                            })).clicked() {
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("uninstall_clicked_package"), package_name_for_buttons.clone());
                                    data.insert_temp(egui::Id::new("uninstall_clicked_is_system"), is_system);
                                });
                            }
                        }

                        if enabled_str.contains("REMOVED_USER") || enabled_str.contains("DISABLED_USER") || enabled_str.contains("DISABLED") {
                            let enable_chip = assist_chip("")
                                .leading_icon_svg(ENABLE_GREEN_SVG)
                                .elevated(true);

                            let pkg_name_enable = package_name_for_buttons.clone();
                            if ui.add(enable_chip.on_click(|| {
                                tracing::info!("Enable clicked for: {}", pkg_name_enable);
                            })).clicked() {
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("enable_clicked_package"), package_name_for_buttons.clone());
                                });
                            }
                        }

                        if enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED") {
                            let disable_chip = assist_chip("")
                                .leading_icon_svg(DISABLE_RED_SVG)
                                .elevated(true);

                            let pkg_name_disable = package_name_for_buttons.clone();
                            if ui.add(disable_chip.on_click(|| {
                                tracing::info!("Disable clicked for: {}", pkg_name_disable);
                            })).clicked() {
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("disable_clicked_package"), package_name_for_buttons.clone());
                                });
                            }
                        }
                    });
                })
                .id(format!("scan_table_row_{}", idx));

                row_builder
            });
        }

        // Set sort state
        if let Some(sort_col) = self.sort_column {
            use egui_material3::SortDirection;
            let direction = if self.sort_ascending {
                SortDirection::Ascending
            } else {
                SortDirection::Descending
            };
            interactive_table = interactive_table.sort_by(sort_col, direction);
        }

        let table_response = interactive_table.show(ui);

        // Sync sort state
        let (widget_sort_col, widget_sort_dir) = table_response.sort_state;
        let widget_sort_ascending =
            matches!(widget_sort_dir, egui_material3::SortDirection::Ascending);

        if widget_sort_col != self.sort_column
            || (widget_sort_col.is_some() && widget_sort_ascending != self.sort_ascending)
        {
            self.sort_column = widget_sort_col;
            self.sort_ascending = widget_sort_ascending;
            if self.sort_column.is_some() {
                self.sort_packages();
            }
        }

        if let Some(clicked_col) = table_response.column_clicked {
            if self.sort_column == Some(clicked_col) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(clicked_col);
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

        // Perform uninstall
        if let Some(pkg_name) = uninstall_package {
            if let Some(ref device) = self.device_serial {
                let uninstall_result = if uninstall_is_system {
                    crate::adb::uninstall_app_user(&pkg_name, device, None)
                } else {
                    crate::adb::uninstall_app(&pkg_name, device)
                };

                match uninstall_result {
                    Ok(output) => {
                        tracing::info!("App uninstalled successfully: {}", output);

                        let is_system = self
                            .installed_packages
                            .iter()
                            .find(|p| p.pkg == pkg_name)
                            .map(|p| p.flags.contains("SYSTEM"))
                            .unwrap_or(false);

                        if is_system {
                            if let Some(pkg) = self
                                .installed_packages
                                .iter_mut()
                                .find(|p| p.pkg == pkg_name)
                            {
                                for user in pkg.users.iter_mut() {
                                    user.installed = false;
                                    user.enabled = 0;
                                }
                            }
                        } else {
                            self.installed_packages.retain(|pkg| pkg.pkg != pkg_name);
                            if let Some(idx_to_remove) = self
                                .installed_packages
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
                        tracing::error!("Failed to uninstall app({}): {}", pkg_name, e);
                    }
                }
            } else {
                tracing::error!("No device selected for uninstall");
            }
        }

        // Perform enable
        if let Some(pkg_name) = enable_package {
            if let Some(ref device) = self.device_serial {
                match crate::adb::enable_app(&pkg_name, device) {
                    Ok(output) => {
                        tracing::info!("App enabled successfully: {}", output);

                        if let Some(pkg) = self
                            .installed_packages
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
                        tracing::error!("Failed to enable app: {}", e);
                    }
                }
            } else {
                tracing::error!("No device selected for enable");
            }
        }

        // Perform disable
        if let Some(pkg_name) = disable_package {
            if let Some(ref device) = self.device_serial {
                match crate::adb::disable_app_current_user(&pkg_name, device, None) {
                    Ok(output) => {
                        tracing::info!("App disabled successfully: {}", output);

                        if let Some(pkg) = self
                            .installed_packages
                            .iter_mut()
                            .find(|p| p.pkg == pkg_name)
                        {
                            for user in pkg.users.iter_mut() {
                                user.enabled = 3;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to disable app: {}", e);
                    }
                }
            } else {
                tracing::error!("No device selected for disable");
            }
        }

        // Perform refresh (delete scan results and re-scan)
        if let Some(pkg_name) = refresh_package {
            tracing::info!("Refreshing scan results for: {}", pkg_name);

            // Delete from database
            let mut conn = db::establish_connection();
            if let Err(e) = db_virustotal::delete_results_by_package(&mut conn, &pkg_name) {
                tracing::error!("Failed to delete VirusTotal results for {}: {}", pkg_name, e);
            } else {
                tracing::info!("Deleted VirusTotal results for: {}", pkg_name);
            }

            if let Err(e) = db_hybridanalysis::delete_results_by_package(&mut conn, &pkg_name) {
                tracing::error!("Failed to delete HybridAnalysis results for {}: {}", pkg_name, e);
            } else {
                tracing::info!("Deleted HybridAnalysis results for: {}", pkg_name);
            }

            // Get package info for scanning
            let package_info = self.installed_packages.iter().find(|p| p.pkg == pkg_name).cloned();

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
                if let (Some(ref vt_state), Some(ref vt_limiter), Some(ref api_key), Some(ref serial)) = (
                    &self.vt_scanner_state,
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
                        tracing::info!("Starting VT re-scan for: {}", pkg_name_clone);
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
                            tracing::error!("Error re-scanning VT for {}: {}", pkg_name_clone, e);
                        }
                    });
                }

                // Start HybridAnalysis scan in background
                if let (Some(ref ha_state), Some(ref ha_limiter), Some(ref api_key), Some(ref serial)) = (
                    &self.ha_scanner_state,
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
                        tracing::info!("Starting HA re-scan for: {}", pkg_name_clone);
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
                            tracing::error!("Error re-scanning HA for {}: {}", pkg_name_clone, e);
                        }
                    });
                }
            }
        }

        // Show package details dialog
        self.package_details_dialog
            .show(ui.ctx(), &self.installed_packages, &self.uad_ng_lists);
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

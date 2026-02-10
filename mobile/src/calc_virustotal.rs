use crate::adb;
use crate::api_virustotal::{self, VtError};
use crate::db;
use crate::db_virustotal;
use crate::is_valid_package_id;
use crate::models::VirusTotalResult;
use crate::Config;
use egui_i18n::tr;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub use crate::calc_virustotal_stt::*;

impl FileScanResult {
    pub fn total(&self) -> i32 {
        self.malicious + self.suspicious + self.undetected + self.harmless
    }
}

impl CalcVirustotal {
    pub fn from_db_results(db_results: Vec<VirusTotalResult>) -> Self {
        let files_attempted = db_results.len();
        let file_results = db_results
            .into_iter()
            .map(|r| FileScanResult {
                file_path: r.file_path.clone(),
                sha256: r.sha256.clone(),
                malicious: r.malicious,
                suspicious: r.suspicious,
                undetected: r.undetected,
                harmless: r.harmless,
                dex_count: r.dex_count,
                reputation: r.reputation,
                vt_link: format!("https://www.virustotal.com/gui/file/{}", r.sha256),
                not_found: r.raw_response.contains("404 Not Found"),
                skipped: false,
                error: None,
            })
            .collect();

        Self {
            file_results,
            files_attempted,
            files_skipped_invalid_hash: 0,
        }
    }
}

impl RateLimiter {
    pub fn new(max_requests: usize, time_window: Duration, min_interval: Duration) -> Self {
        Self {
            request_times: Vec::new(),
            max_requests,
            time_window,
            min_interval,
            rate_limit_until: None,
        }
    }

    /// Wait if necessary to respect rate limits, then record the request
    /// Returns duration waited, or None if no wait was needed
    fn check_wait_needed(&mut self) -> Option<Duration> {
        let now = Instant::now();
        let mut wait_duration = Duration::ZERO;

        // First check if we're in a global rate limit period (from 429 error)
        if let Some(until) = self.rate_limit_until {
            if now < until {
                wait_duration = until.duration_since(now);
                log::info!("Global rate limit active, waiting {:?}", wait_duration);
                return Some(wait_duration);
            } else {
                // Rate limit period has passed, clear it
                self.rate_limit_until = None;
            }
        }

        // Remove requests outside the time window
        self.request_times
            .retain(|&time| now.duration_since(time) < self.time_window);

        // Check minimum interval from last request
        if let Some(&last) = self.request_times.last() {
            let since_last = now.duration_since(last);
            if since_last < self.min_interval {
                let interval_wait = self.min_interval - since_last;
                if interval_wait > wait_duration {
                    wait_duration = interval_wait;
                }
            }
        }

        // If we've hit the limit, wait until the oldest request expires
        if self.request_times.len() >= self.max_requests {
            if let Some(&oldest) = self.request_times.first() {
                let limit_wait = self.time_window.saturating_sub(now.duration_since(oldest));
                if limit_wait > wait_duration {
                    wait_duration = limit_wait;
                }
            }
        }

        if wait_duration > Duration::ZERO {
            Some(wait_duration)
        } else {
            None
        }
    }

    fn record_request(&mut self) {
        self.request_times.push(Instant::now());
        // Remove oldest if we exceed max_requests (though logic above should handle it, this keeps vector size bounded if logic changes)
        if self.request_times.len() > self.max_requests * 2 {
            self.request_times.remove(0);
        }
    }

    /// Wait if necessary to respect rate limits, then record the request
    #[allow(dead_code)]
    fn wait_if_needed(&mut self) {
        if let Some(duration) = self.check_wait_needed() {
            log::debug!("Sleeping for {:?}", duration);
            thread::sleep(duration);
        }

        // Record this request
        self.record_request();
    }

    /// Set global rate limit (for 429 errors) - all threads will wait until this time
    fn set_rate_limit(&mut self, duration: Duration) {
        let until = Instant::now() + duration;

        // Only update if this extends the existing rate limit
        if self.rate_limit_until.is_none() || self.rate_limit_until.unwrap() < until {
            log::warn!(
                "Setting global rate limit for {:?} (until {:?})",
                duration,
                until
            );
            self.rate_limit_until = Some(until);
            // Clear request times since we're resetting
            self.request_times.clear();
        }
    }

    /// Get the number of available requests in the current time window
    fn available_requests(&self) -> usize {
        let now = Instant::now();

        // If in global rate limit, no requests available
        if let Some(until) = self.rate_limit_until {
            if now < until {
                return 0;
            }
        }

        // Count requests within the time window
        let recent_requests = self
            .request_times
            .iter()
            .filter(|&&time| now.duration_since(time) < self.time_window)
            .count();

        self.max_requests.saturating_sub(recent_requests)
    }
}

/// Initialize scanner state by checking database cache
pub fn init_scanner_state(package_names: &[String]) -> ScannerState {
    let state = Arc::new(Mutex::new(HashMap::new()));
    let mut conn = db::establish_connection();

    for pkg_name in package_names {
        // Check if we have cached results
        match db_virustotal::get_results_by_package(&mut conn, pkg_name) {
            Ok(cached_results) if !cached_results.is_empty() => {
                let result = CalcVirustotal::from_db_results(cached_results);
                state
                    .lock()
                    .unwrap()
                    .insert(pkg_name.clone(), ScanStatus::Completed(result));
            }
            Ok(_) => {
                // No cached result, mark as pending
                state
                    .lock()
                    .unwrap()
                    .insert(pkg_name.clone(), ScanStatus::Pending);
            }
            Err(e) => {
                log::error!("Error checking cache for {}: {}", pkg_name, e);
                state
                    .lock()
                    .unwrap()
                    .insert(pkg_name.clone(), ScanStatus::Error(e.to_string()));
            }
        }
    }

    state
}

/// Analyze hashes using VirusTotal API
pub fn analyze_package(
    package_name: &str,
    hashes: Vec<(String, String)>,
    state: &ScannerState,
    rate_limiter: &SharedRateLimiter,
    api_key: &str,
    device_serial: &str,
    allow_upload: bool,
    repaint_signal: &Option<Arc<dyn Fn() + Send + Sync>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Skip package IDs with less than 2 domain levels (e.g., com.android)
    if !is_valid_package_id(package_name) {
        log::debug!(
            "Skipping VirusTotal scan for invalid package ID: {}",
            package_name
        );
        let mut s = state.lock().unwrap();
        s.insert(
            package_name.to_string(),
            ScanStatus::Error(tr!("error-invalid-package-id")),
        );
        return Ok(());
    }

    let config = Config::new()?;
    let mut conn = db::establish_connection();
    let total_files = hashes.len();

    let mut file_results = Vec::new();
    let mut last_error: Option<String> = None;
    let mut last_was_cache = false;
    let mut files_skipped_invalid_hash = 0usize;

    // Timeout duration for this file
    let timeout_duration = Duration::from_secs(60);

    for (idx, (file_path, sha256)) in hashes.iter().enumerate() {
        // Validate SHA256 hash length (must be 64 hex characters)
        if sha256.len() != 64 {
            log::warn!(
                "Skipping invalid SHA256 hash for {}: {} (length: {}, expected 64)",
                file_path,
                sha256,
                sha256.len()
            );
            files_skipped_invalid_hash += 1;
            continue;
        }

        let start_time = Instant::now();
        let max_wait_time = start_time + timeout_duration;

        // Check database cache first
        if let Ok(Some(cached)) = db_virustotal::get_result_by_sha256(&mut conn, sha256) {
            log::debug!("Found cached result for {}", sha256);

            // Check if it's a cached 404
            if cached.raw_response.contains("404 Not Found") {
                log::info!("Found cached 404 for {}", sha256);
                file_results.push(FileScanResult {
                    file_path: file_path.clone(),
                    sha256: cached.sha256.clone(),
                    malicious: 0,
                    suspicious: 0,
                    undetected: 0,
                    harmless: 0,
                    dex_count: None,
                    reputation: 0,
                    vt_link: format!("https://www.virustotal.com/gui/file/{}", cached.sha256),
                    not_found: true,
                    skipped: false,
                    error: None,
                });
                last_was_cache = true;
                continue;
            }

            // Update status - using cached result
            {
                let mut s = state.lock().unwrap();
                s.insert(
                    package_name.to_string(),
                    ScanStatus::Scanning {
                        scanned: idx + 1,
                        total: total_files,
                        operation: tr!("status-checking-api"),
                    },
                );
            }
            if let Some(signal) = repaint_signal {
                signal();
            }

            file_results.push(FileScanResult {
                file_path: file_path.clone(),
                sha256: cached.sha256.clone(),
                malicious: cached.malicious,
                suspicious: cached.suspicious,
                undetected: cached.undetected,
                harmless: cached.harmless,
                dex_count: cached.dex_count,
                reputation: cached.reputation,
                vt_link: format!("https://www.virustotal.com/gui/file/{}", cached.sha256),
                not_found: false,
                skipped: false,
                error: None,
            });
            last_was_cache = true;
            continue;
        }

        // Update status - checking API
        {
            let mut s = state.lock().unwrap();
            s.insert(
                package_name.to_string(),
                ScanStatus::Scanning {
                    scanned: idx + 1,
                    total: total_files,
                    operation: "Checking API".to_string(),
                },
            );
        }
        if let Some(signal) = repaint_signal {
            signal();
        }

        // Not in cache, query VirusTotal API

        // Wait for rate limit with timeout
        loop {
            // if Instant::now() > max_wait_time {
            //     last_error = Some(tr!("error-timeout-rate-limit"));
            //     break;
            // }

            let wait_needed = {
                let mut limiter = rate_limiter.lock().unwrap();
                if last_was_cache && limiter.rate_limit_until.is_none() {
                    None
                } else {
                    limiter.check_wait_needed()
                }
            };

            if let Some(duration) = wait_needed {
                thread::sleep(duration);
            } else {
                break;
            }
        }
        if last_error.is_some() {
            log::warn!("Skipping {} due to timeout", file_path);
            continue;
        }

        last_was_cache = false;

        // Record request
        {
            let mut limiter = rate_limiter.lock().unwrap();
            limiter.record_request();
        }

        log::info!("Querying VirusTotal API for SHA256: {}", sha256);
        match api_virustotal::get_file_report(sha256, api_key) {
            Ok(response) => {
                let available_after = rate_limiter.lock().unwrap().available_requests();
                log::info!(
                    "Got VirusTotal report for {}",
                    sha256
                );

                // Save to database via queue
                db_virustotal::queue_upsert(
                    package_name.to_string(),
                    file_path.clone(),
                    sha256.clone(),
                    response.clone(),
                )?;

                let default_stats = api_virustotal::LastAnalysisStats {
                    malicious: 0,
                    suspicious: 0,
                    undetected: 0,
                    harmless: 0,
                    timeout: 0,
                    confirmed_timeout: 0,
                    failure: 0,
                    type_unsupported: 0,
                };
                let stats = response
                    .data
                    .attributes
                    .last_analysis_stats
                    .as_ref()
                    .unwrap_or(&default_stats);
                let dex_count = response
                    .data
                    .attributes
                    .androguard
                    .as_ref()
                    .and_then(|a| a.risk_indicator.as_ref())
                    .and_then(|r| r.apk.as_ref())
                    .and_then(|a| a.dex);

                file_results.push(FileScanResult {
                    file_path: file_path.clone(),
                    sha256: sha256.clone(),
                    malicious: stats.malicious,
                    suspicious: stats.suspicious,
                    undetected: stats.undetected,
                    harmless: stats.harmless,
                    dex_count,
                    reputation: response.data.attributes.reputation,
                    vt_link: format!("https://www.virustotal.com/gui/file/{}", sha256),
                    not_found: false,
                    skipped: false,
                    error: None,
                });
            }
            Err(VtError::NotFound) if allow_upload => {
                log::info!(
                    "File {} sha256 {} not found in VirusTotal, uploading",
                    file_path,
                    sha256
                );

                // Update status - pulling file
                {
                    let mut s = state.lock().unwrap();
                    s.insert(
                        package_name.to_string(),
                        ScanStatus::Scanning {
                            scanned: idx + 1,
                            total: total_files,
                            operation: tr!("status-pulling-file"),
                        },
                    );
                }
                if let Some(signal) = repaint_signal {
                    signal();
                }

                if Instant::now() > max_wait_time {
                    last_error = Some("Timeout before pulling file".to_string());
                    continue;
                }

                // Pull file to temp directory
                // Note: This requires ADB to be available. If it's not, this will fail.

                // Skip if the path appears to be a directory (doesn't have a file extension or ends with /)
                if file_path.ends_with('/') || !file_path.contains('.') {
                    log::warn!("Skipping directory or invalid path: {}", file_path);
                    let err_msg = format!("Path is a directory, not a file: {}", file_path);
                    file_results.push(FileScanResult {
                        file_path: file_path.clone(),
                        sha256: sha256.clone(),
                        malicious: 0,
                        suspicious: 0,
                        undetected: 0,
                        harmless: 0,
                        dex_count: None,
                        reputation: 0,
                        vt_link: format!("https://www.virustotal.com/gui/file/{}", sha256),
                        not_found: false,
                        skipped: false,
                        error: Some(err_msg.clone()),
                    });
                    last_error = Some(err_msg);
                    continue;
                }

                // Only upload APK and SO files to VirusTotal (skip .prof, .dm, .art, etc.)
                if !file_path.ends_with(".apk") && !file_path.ends_with(".so") {
                    log::info!(
                        "Skipping file for upload: {} (only .apk and .so files are uploaded)",
                        file_path
                    );
                    file_results.push(FileScanResult {
                        file_path: file_path.clone(),
                        sha256: sha256.clone(),
                        malicious: 0,
                        suspicious: 0,
                        undetected: 0,
                        harmless: 0,
                        dex_count: None,
                        reputation: 0,
                        vt_link: format!("https://www.virustotal.com/gui/file/{}", sha256),
                        not_found: false,
                        skipped: true,
                        error: None,
                    });
                    continue;
                }

                let tmp_dir_str = config.tmp_dir.to_str().ok_or("Invalid tmp directory path")?;
                let expected_filename = format!("{}.apk", package_name.replace('.', "_"));
                let local_path = config.tmp_dir.join(&expected_filename);

                #[cfg(not(target_os = "android"))]
                let pull_result =
                    adb::pull_file_to_temp(device_serial, file_path, tmp_dir_str, package_name);
                #[cfg(target_os = "android")]
                let pull_result: Result<String, std::io::Error> = Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "File pulling not supported on Android platform",
                ));
                match pull_result {
                    Ok(tmp_file) => {
                        // Update status - uploading
                        {
                            let mut s = state.lock().unwrap();
                            s.insert(
                                package_name.to_string(),
                                ScanStatus::Scanning {
                                    scanned: idx + 1,
                                    total: total_files,
                                    operation: tr!("status-uploading"),
                                },
                            );
                        }
                        if let Some(signal) = repaint_signal {
                            signal();
                        }

                        // Upload file
                        // Wait for rate limit
                        loop {
                            // if Instant::now() > max_wait_time {
                            //     last_error = Some(tr!("error-timeout-rate-limit"));
                            //     break;
                            // }
                            let wait_needed = {
                                let mut limiter = rate_limiter.lock().unwrap();
                                limiter.check_wait_needed()
                            };
                            if let Some(duration) = wait_needed {
                                thread::sleep(duration);
                            } else {
                                break;
                            }
                        }
                        if last_error.is_some() {
                            let _ = std::fs::remove_file(&tmp_file);
                            log::warn!("Skipping upload {} due to timeout", tmp_file);
                            continue;
                        }

                        {
                            let mut limiter = rate_limiter.lock().unwrap();
                            limiter.record_request();
                        }

                        log::info!("Uploading file to VirusTotal: {}", tmp_file);
                        match api_virustotal::upload_file_smart(Path::new(&tmp_file), api_key) {
                            Ok(upload_response) => {
                                let available_after =
                                    rate_limiter.lock().unwrap().available_requests();
                                log::info!("Uploaded file {}, analysis ID: {}", sha256, upload_response.data.id);

                                // Poll for results until timeout
                                log::info!("Waiting for analysis to complete...");

                                let mut analysis_result = None;
                                while Instant::now() < max_wait_time {
                                    // Wait a bit before polling
                                    thread::sleep(Duration::from_secs(10));

                                    // Check rate limit for polling
                                    let mut can_poll = false;
                                    {
                                        let mut limiter = rate_limiter.lock().unwrap();
                                        if limiter.check_wait_needed().is_none() {
                                            limiter.record_request();
                                            can_poll = true;
                                        }
                                    }

                                    if can_poll {
                                        log::info!(
                                            "Retrieving analysis results for: {}",
                                            upload_response.data.id
                                        );
                                        match api_virustotal::get_analysis(
                                            &upload_response.data.id,
                                            api_key,
                                        ) {
                                            Ok(response) => {
                                                // Check status
                                                let status = response
                                                    .data
                                                    .attributes
                                                    .status
                                                    .as_deref()
                                                    .unwrap_or("unknown");
                                                if status == "completed" {
                                                    analysis_result = Some(response);
                                                    break;
                                                } else {
                                                    log::debug!("Analysis status: {}", status);
                                                }
                                            }
                                            Err(VtError::RateLimit { retry_after }) => {
                                                rate_limiter.lock().unwrap().set_rate_limit(
                                                    Duration::from_secs(retry_after),
                                                );
                                            }
                                            Err(e) => {
                                                log::warn!("Error polling analysis: {}", e);
                                            }
                                        }
                                    }
                                }

                                if let Some(response) = analysis_result {
                                    let available_after =
                                        rate_limiter.lock().unwrap().available_requests();
                                    log::info!(
                                        "Got analysis result for {}",
                                        sha256
                                    );

                                    // Save to database via queue
                                    db_virustotal::queue_upsert(
                                        package_name.to_string(),
                                        file_path.clone(),
                                        sha256.clone(),
                                        response.clone(),
                                    )?;

                                    let default_stats = api_virustotal::LastAnalysisStats {
                                        malicious: 0,
                                        suspicious: 0,
                                        undetected: 0,
                                        harmless: 0,
                                        timeout: 0,
                                        confirmed_timeout: 0,
                                        failure: 0,
                                        type_unsupported: 0,
                                    };
                                    let stats = response
                                        .data
                                        .attributes
                                        .last_analysis_stats
                                        .as_ref()
                                        .unwrap_or(&default_stats);
                                    let dex_count = response
                                        .data
                                        .attributes
                                        .androguard
                                        .as_ref()
                                        .and_then(|a| a.risk_indicator.as_ref())
                                        .and_then(|r| r.apk.as_ref())
                                        .and_then(|a| a.dex);

                                    file_results.push(FileScanResult {
                                        file_path: file_path.clone(),
                                        sha256: sha256.clone(),
                                        malicious: stats.malicious,
                                        suspicious: stats.suspicious,
                                        undetected: stats.undetected,
                                        harmless: stats.harmless,
                                        dex_count,
                                        reputation: response.data.attributes.reputation,
                                        vt_link: format!(
                                            "https://www.virustotal.com/gui/file/{}",
                                            sha256
                                        ),
                                        not_found: false,
                                        skipped: false,
                                        error: None,
                                    });
                                } else {
                                    let err_msg = format!(
                                        "Timeout waiting for VirusTotal analysis to complete for package '{}' (sha256: {}, file: {})",
                                        package_name, sha256, file_path
                                    );
                                    log::error!("{}", err_msg);
                                    last_error = Some(err_msg);
                                }
                            }
                            Err(VtError::RateLimit { retry_after }) => {
                                rate_limiter
                                    .lock()
                                    .unwrap()
                                    .set_rate_limit(Duration::from_secs(retry_after));
                            }
                            Err(e) => {
                                let err_msg = format!("Failed to upload file: {}", e);
                                log::error!("Failed to upload file: {}", e);
                                file_results.push(FileScanResult {
                                    file_path: file_path.clone(),
                                    sha256: sha256.clone(),
                                    malicious: 0,
                                    suspicious: 0,
                                    undetected: 0,
                                    harmless: 0,
                                    dex_count: None,
                                    reputation: 0,
                                    vt_link: format!(
                                        "https://www.virustotal.com/gui/file/{}",
                                        sha256
                                    ),
                                    not_found: false,
                                    skipped: false,
                                    error: Some(err_msg.clone()),
                                });
                                last_error = Some(err_msg);
                            }
                        }

                        // Clean up temp file
                        let _ = std::fs::remove_file(&tmp_file);
                    }
                    Err(e) => {
                        let err_msg = format!("Failed to pull file: {}", e);
                        log::error!("Failed to pull file: {}", e);
                        file_results.push(FileScanResult {
                            file_path: file_path.clone(),
                            sha256: sha256.clone(),
                            malicious: 0,
                            suspicious: 0,
                            undetected: 0,
                            harmless: 0,
                            dex_count: None,
                            reputation: 0,
                            vt_link: format!("https://www.virustotal.com/gui/file/{}", sha256),
                            not_found: false,
                            skipped: false,
                            error: Some(err_msg.clone()),
                        });
                        last_error = Some(err_msg);
                    }
                }
            }
            Err(VtError::NotFound) => {
                log::warn!(
                    "Sha256 {} for file {} not found in VirusTotal and upload is disabled",
                    sha256,
                    file_path
                );

                // Cache 404
                let not_found_response = api_virustotal::VirusTotalResponse {
                    data: api_virustotal::VirusTotalData {
                        id: sha256.to_string(),
                        data_type: "file".to_string(),
                        attributes: api_virustotal::VirusTotalAttributes {
                            last_analysis_date: Some(0),
                            last_analysis_stats: Some(api_virustotal::LastAnalysisStats {
                                malicious: 0,
                                suspicious: 0,
                                undetected: 0,
                                harmless: 0,
                                timeout: 0,
                                confirmed_timeout: 0,
                                failure: 0,
                                type_unsupported: 0,
                            }),
                            reputation: 0,
                            androguard: None,
                            status: Some("404 Not Found".to_string()),
                        },
                    },
                };

                if let Err(e) = db_virustotal::queue_upsert(
                    package_name.to_string(),
                    file_path.clone(),
                    sha256.clone(),
                    not_found_response,
                ) {
                    log::error!("Failed to cache 404 for {}: {}", sha256, e);
                }

                file_results.push(FileScanResult {
                    file_path: file_path.clone(),
                    sha256: sha256.clone(),
                    malicious: 0,
                    suspicious: 0,
                    undetected: 0,
                    harmless: 0,
                    dex_count: None,
                    reputation: 0,
                    vt_link: format!("https://www.virustotal.com/gui/file/{}", sha256),
                    not_found: true,
                    skipped: false,
                    error: None,
                });
            }
            Err(VtError::RateLimit { retry_after }) => {
                rate_limiter
                    .lock()
                    .unwrap()
                    .set_rate_limit(Duration::from_secs(retry_after));
                // Retry this file
                // (In a production system, we'd want a more sophisticated retry mechanism)
            }
            Err(e) => {
                let err_msg = e.to_string();
                log::error!("Error getting VirusTotal report for {}: {}", sha256, e);
                file_results.push(FileScanResult {
                    file_path: file_path.clone(),
                    sha256: sha256.clone(),
                    malicious: 0,
                    suspicious: 0,
                    undetected: 0,
                    harmless: 0,
                    dex_count: None,
                    reputation: 0,
                    vt_link: format!("https://www.virustotal.com/gui/file/{}", sha256),
                    not_found: false,
                    skipped: false,
                    error: Some(err_msg.clone()),
                });
                last_error = Some(err_msg);
            }
        }
    }

    // Update final status
    let mut s = state.lock().unwrap();
    if file_results.is_empty() {
        if let Some(err) = last_error {
            s.insert(package_name.to_string(), ScanStatus::Error(err));
        } else {
            // No results but no specific error
            let result = CalcVirustotal {
                file_results,
                files_attempted: total_files,
                files_skipped_invalid_hash,
            };
            s.insert(package_name.to_string(), ScanStatus::Completed(result));
        }
    } else {
        let result = CalcVirustotal {
            file_results,
            files_attempted: total_files,
            files_skipped_invalid_hash,
        };
        s.insert(package_name.to_string(), ScanStatus::Completed(result));
    }
    if let Some(signal) = repaint_signal {
        signal();
    }

    Ok(())
}

/// Run VirusTotal scanning for a list of packages in a background thread.
/// This function initializes the scanner state and spawns a background thread
/// to scan all packages using the VirusTotal API.
///
/// # Arguments
/// * `installed_packages` - List of packages to scan
/// * `device_serial` - Device serial number for ADB operations
/// * `api_key` - VirusTotal API key
/// * `virustotal_submit_enabled` - Whether to submit unknown files to VirusTotal
/// * `package_risk_scores` - Risk scores for sorting packages by priority
/// * `vt_scan_progress` - Shared progress value for UI updates
/// * `vt_scan_cancelled` - Shared cancellation flag
///
/// # Returns
/// Returns the scanner state and rate limiter for tracking progress
pub fn run_virustotal(
    installed_packages: Vec<crate::adb::PackageFingerprint>,
    device_serial: String,
    api_key: String,
    virustotal_submit_enabled: bool,
    package_risk_scores: HashMap<String, i32>,
    vt_scan_progress: Arc<Mutex<Option<f32>>>,
    vt_scan_cancelled: Arc<Mutex<bool>>,
) -> (ScannerState, SharedRateLimiter) {
    let package_names: Vec<String> = installed_packages.iter().map(|p| p.pkg.clone()).collect();
    let scanner_state = init_scanner_state(&package_names);

    let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(
        4,
        Duration::from_secs(60),
        Duration::from_secs(5),
    )));

    // Initialize progress
    if let Ok(mut p) = vt_scan_progress.lock() {
        *p = Some(0.0);
    }
    if let Ok(mut cancelled) = vt_scan_cancelled.lock() {
        *cancelled = false;
    }

    log::info!(
        "Starting VirusTotal scan for {} packages",
        installed_packages.len()
    );

    let scanner_state_clone = scanner_state.clone();
    let rate_limiter_clone = rate_limiter.clone();
    let vt_scan_progress_clone = vt_scan_progress;
    let vt_scan_cancelled_clone = vt_scan_cancelled;

    std::thread::spawn(move || {
        let mut packages = installed_packages;
        packages.sort_by(|a, b| {
            let perms_a: usize = a.users.iter().map(|u| u.runtimePermissions.len()).sum();
            let perms_b: usize = b.users.iter().map(|u| u.runtimePermissions.len()).sum();
            perms_b.cmp(&perms_a)
        });

        let cached_packages =
            crate::db_package_cache::get_cached_packages_with_apk(&device_serial);

        let mut cached_packages_map: HashMap<String, crate::models::PackageInfoCache> =
            HashMap::new();
        for cp in cached_packages {
            cached_packages_map.insert(cp.pkg_id.clone(), cp);
        }

        let total = packages.len();
        let mut skipped_cached = 0usize;

        for (i, package) in packages.iter().enumerate() {
            if let Ok(cancelled) = vt_scan_cancelled_clone.lock() {
                if *cancelled {
                    log::info!("VirusTotal scan cancelled by user");
                    break;
                }
            }

            if let Ok(mut p) = vt_scan_progress_clone.lock() {
                *p = Some(i as f32 / total as f32);
            }

            let pkg_name = &package.pkg;

            {
                let s = scanner_state_clone.lock().unwrap();
                if matches!(s.get(pkg_name), Some(ScanStatus::Completed(_))) {
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
                        match crate::adb::get_single_package_sha256sum(&device_serial, pkg_name) {
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
                                    log::warn!(
                                        "Failed to get sha256sums for {}: {}, using cached values",
                                        pkg_name,
                                        e
                                    );
                                    (paths_str.clone(), sha256sums_str.clone())
                                } else {
                                    log::warn!(
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

                log::info!(
                    "Analyzing package {} with {} files (Risk: {})",
                    pkg_name,
                    hashes.len(),
                    package_risk_scores.get(pkg_name).copied().unwrap_or(0)
                );

                if let Err(e) = analyze_package(
                    pkg_name,
                    hashes,
                    &scanner_state_clone,
                    &rate_limiter_clone,
                    &api_key,
                    &device_serial,
                    virustotal_submit_enabled,
                    &None,
                ) {
                    log::error!("Error analyzing package {}: {}", pkg_name, e);
                }
            }
        }

        log::info!(
            "VirusTotal scan complete: {} cached, {} processed",
            skipped_cached,
            total - skipped_cached
        );

        if let Ok(mut p) = vt_scan_progress_clone.lock() {
            *p = None;
        }
    });

    (scanner_state, rate_limiter)
}

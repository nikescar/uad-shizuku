use crate::adb;
use crate::api_hybridanalysis::{self, HaError};
use crate::db;
use crate::db_hybridanalysis;
use crate::is_valid_package_id;
use crate::models::HybridAnalysisResult;
use crate::Config;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub use crate::calc_hybridanalysis_stt::*;

impl FileScanResult {
    pub fn get_display_text(&self) -> String {
        // For error states, show the error message if available
        if self.verdict == "upload_error" || self.verdict == "analysis_error" {
            if let Some(ref error_msg) = self.error_message {
                // Simplify common error messages
                let simplified = if error_msg.contains("File too large") {
                    // Extract size information if present
                    if let Some(mb_pos) = error_msg.find(" MB ") {
                        if let Some(start) = error_msg[..mb_pos].rfind(|c: char| !c.is_numeric() && c != '.') {
                            let size = &error_msg[start+1..mb_pos+3]; // Include " MB"
                            format!("File too large: {}", size)
                        } else {
                            "File too large (>100 MB)".to_string()
                        }
                    } else {
                        "File too large (>100 MB)".to_string()
                    }
                } else if error_msg.contains("No such file or directory") {
                    "Pull failed: File not found".to_string()
                } else if error_msg.contains("Failed to create tmp directory") {
                    "Temp dir error".to_string()
                } else {
                    // Keep first 50 chars of error message
                    if error_msg.len() > 50 {
                        format!("{}...", &error_msg[..50])
                    } else {
                        error_msg.clone()
                    }
                };
                return simplified;
            }
        }

        let base_text = if let Some(score) = self.threat_score {
            format!("{} ({})", self.verdict, score)
        } else {
            self.verdict.clone()
        };

        // For pending_analysis, show truncated job_id
        if self.verdict == "pending_analysis" {
            if let Some(ref job_id) = self.job_id {
                // Show first 8 chars of job_id
                let short_id = if job_id.len() > 8 {
                    &job_id[..8]
                } else {
                    job_id
                };
                return format!("pending ({}...)", short_id);
            }
        }

        // If there's a wait_until time, show how long to wait
        if let Some(wait_until) = self.wait_until {
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
                    return format!("{} (wait {}h{}m)", base_text, hours, mins);
                } else if mins > 0 {
                    return format!("{} (wait {}m)", base_text, mins);
                } else {
                    return format!("{} (wait <1m)", base_text);
                }
            }
        }

        base_text
    }
}

impl CalcHybridAnalysis {
    pub fn from_db_results(db_results: Vec<HybridAnalysisResult>) -> Self {
        let file_results = db_results
            .into_iter()
            .map(|r| FileScanResult {
                file_path: r.file_path.clone(),
                sha256: r.sha256.clone(),
                verdict: r.verdict.clone(),
                threat_score: r.threat_score,
                threat_level: r.threat_level,
                classification_tags: serde_json::from_str(&r.classification_tags)
                    .unwrap_or_default(),
                total_signatures: r.total_signatures,
                ha_link: format!("https://hybrid-analysis.com/sample/{}", r.sha256),
                wait_until: None,
                job_id: None,
                error_message: r.error_message.clone(),
            })
            .collect();

        Self { file_results }
    }
}

impl RateLimiter {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            last_request: None,
            min_interval,
            rate_limit_until: None,
            upload_rate_limit_until: None,
            requests_last_minute: Vec::new(),
            requests_last_hour: Vec::new(),
        }
    }

    /// Check if we need to wait and return the duration, or None if no wait needed
    fn check_wait_needed(&mut self) -> Option<Duration> {
        log::debug!("check_wait_needed: Starting rate limit checks");
        let now = Instant::now();

        // First check if we're in a global rate limit period (from 429 error)
        if let Some(until) = self.rate_limit_until {
            if now < until {
                let wait_duration = until.duration_since(now);
                log::info!("Global rate limit active, need to wait {:?}", wait_duration);
                return Some(wait_duration);
            } else {
                // Rate limit period has passed, clear it
                self.rate_limit_until = None;
            }
        }

        // Check 100 requests/minute limit
        self.requests_last_minute
            .retain(|&t| now.duration_since(t) < Duration::from_secs(60));
        if self.requests_last_minute.len() >= 100 {
            let oldest = self.requests_last_minute[0];
            let wait_duration = Duration::from_secs(60).saturating_sub(now.duration_since(oldest));
            if wait_duration > Duration::ZERO {
                log::info!(
                    "Rate limit: 100/minute reached, need to wait {:?}",
                    wait_duration
                );
                return Some(wait_duration);
            }
        }

        // Check 1500 requests/hour limit
        self.requests_last_hour
            .retain(|&t| now.duration_since(t) < Duration::from_secs(3600));
        if self.requests_last_hour.len() >= 1500 {
            let oldest = self.requests_last_hour[0];
            let wait_duration =
                Duration::from_secs(3600).saturating_sub(now.duration_since(oldest));
            if wait_duration > Duration::ZERO {
                log::info!(
                    "Rate limit: 1500/hour reached, need to wait {:?}",
                    wait_duration
                );
                return Some(wait_duration);
            }
        }

        // Check minimum interval from last request
        if let Some(last) = self.last_request {
            let since_last = now.duration_since(last);
            if since_last < self.min_interval {
                let wait_duration = self.min_interval - since_last;
                log::debug!("Need to wait {:?} for minimum interval", wait_duration);
                return Some(wait_duration);
            }
        }

        None
    }

    /// Record that a request was made
    fn record_request(&mut self) {
        let now = Instant::now();
        self.last_request = Some(now);
        self.requests_last_minute.push(now);
        self.requests_last_hour.push(now);
    }

    /// Wait if necessary to respect rate limits, then record the request
    #[allow(dead_code)]
    fn wait_if_needed(&mut self) {
        log::debug!("wait_if_needed: Starting rate limit checks");

        // Loop until no wait is needed
        loop {
            if let Some(duration) = self.check_wait_needed() {
                log::debug!("Sleeping for {:?}", duration);
                thread::sleep(duration);
            } else {
                break;
            }
        }

        // Record this request
        self.record_request();
    }

    /// Check if upload needs to wait and return duration
    fn check_upload_wait_needed(&mut self) -> Option<Duration> {
        log::debug!("check_upload_wait_needed: Checking upload rate limits");
        let now = Instant::now();

        // Check if we're in an upload rate limit period (1 hour for 429 on uploads)
        if let Some(until) = self.upload_rate_limit_until {
            if now < until {
                let wait_duration = until.duration_since(now);
                log::warn!(
                    "Upload rate limit active (429 from previous upload), need to wait {:?}",
                    wait_duration
                );
                return Some(wait_duration);
            } else {
                // Rate limit period has passed, clear it
                log::debug!("Upload rate limit period has passed, clearing");
                self.upload_rate_limit_until = None;
            }
        }

        // Use the same general rate limiting for uploads
        self.check_wait_needed()
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
        }
    }

    /// Set upload rate limit (for 429 errors on uploads) - 1 day wait
    fn set_upload_rate_limit(&mut self) {
        let duration = Duration::from_secs(86400); // 1 day
        let until = Instant::now() + duration;

        // Only update if this extends the existing rate limit
        if self.upload_rate_limit_until.is_none() || self.upload_rate_limit_until.unwrap() < until {
            log::warn!("Setting upload rate limit for 1 day (until {:?})", until);
            self.upload_rate_limit_until = Some(until);
        }
    }
}

/// Initialize scanner state by checking database cache
pub fn init_scanner_state(package_names: &[String]) -> ScannerState {
    let state = Arc::new(Mutex::new(HashMap::new()));
    let mut conn = db::establish_connection();

    for pkg_name in package_names {
        // Check if we have cached results
        match db_hybridanalysis::get_results_by_package(&mut conn, pkg_name) {
            Ok(cached_results) if !cached_results.is_empty() => {
                let result = CalcHybridAnalysis::from_db_results(cached_results);
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

/// Sleep with periodic status updates to show countdown
fn sleep_with_updates(
    duration: Duration,
    package_name: &str,
    idx: usize,
    total_files: usize,
    operation_prefix: &str,
    state: &ScannerState,
    repaint_signal: &Option<Arc<dyn Fn() + Send + Sync>>,
    max_wait_time: Option<Instant>,
) -> Result<(), String> {
    let update_interval = Duration::from_secs(1);
    let mut remaining = duration;

    while remaining > Duration::ZERO {
        // Check timeout
        if let Some(max_time) = max_wait_time {
            if Instant::now() >= max_time {
                return Err("Timeout waiting".to_string());
            }
        }

        let sleep_time = if remaining > update_interval {
            update_interval
        } else {
            remaining
        };

        thread::sleep(sleep_time);
        remaining = remaining.saturating_sub(sleep_time);

        if remaining > Duration::ZERO {
            // Update status with countdown
            {
                let mut s = state.lock().unwrap();
                s.insert(
                    package_name.to_string(),
                    ScanStatus::Scanning {
                        scanned: idx,
                        total: total_files,
                        operation: format!("{} ({}s)", operation_prefix, remaining.as_secs()),
                    },
                );
            }
            if let Some(signal) = repaint_signal {
                signal();
            }
        }
    }
    Ok(())
}

/// Analyze hashes using Hybrid Analysis API
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
            "Skipping Hybrid Analysis scan for invalid package ID: {}",
            package_name
        );
        let mut s = state.lock().unwrap();
        s.insert(
            package_name.to_string(),
            ScanStatus::Error("Invalid package ID (less than 2 domain levels)".to_string()),
        );
        return Ok(());
    }

    let config = Config::new()?;
    let mut conn = db::establish_connection();
    let total_files = hashes.len();

    let mut file_results = Vec::new();
    let mut last_error: Option<String> = None;
    let mut last_was_cache = false;

    // Set a global timeout for the entire package analysis or per-file?
    // User asked "if waiting is more than 60 seconds print skip error".
    // We'll apply this per-file to ensure no single file hangs everything.
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
            continue;
        }

        let start_time = Instant::now();
        let max_wait_time = start_time + timeout_duration;

        // Check database cache first
        if let Ok(Some(cached)) = db_hybridanalysis::get_result_by_sha256(&mut conn, sha256) {
            log::debug!("Found cached Hybrid Analysis result for {}", sha256);

            // Update status - using cached result
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

            file_results.push(FileScanResult {
                file_path: file_path.clone(),
                sha256: cached.sha256.clone(),
                verdict: cached.verdict.clone(),
                threat_score: cached.threat_score,
                threat_level: cached.threat_level,
                classification_tags: serde_json::from_str(&cached.classification_tags)
                    .unwrap_or_default(),
                total_signatures: cached.total_signatures,
                ha_link: format!("https://hybrid-analysis.com/sample/{}", cached.sha256),
                wait_until: None,
                job_id: None,
                error_message: None,
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

        // Not in cache, query Hybrid Analysis API
        let mut loop_err = None;
        loop {
            if Instant::now() > max_wait_time {
                loop_err = Some("Timeout waiting for rate limit".to_string());
                break;
            }

            let wait_duration = {
                let mut limiter = rate_limiter.lock().unwrap();
                if last_was_cache && limiter.rate_limit_until.is_none() {
                    None
                } else {
                    limiter.check_wait_needed()
                }
            };

            if let Some(duration) = wait_duration {
                // If the wait duration would push us over the timeout, abort
                if Instant::now() + duration > max_wait_time {
                    loop_err = Some(format!("Waited too long for rate limit: {:?}", duration));
                    break;
                }

                log::debug!("Waiting {:?} before API request", duration);
                if let Err(e) = sleep_with_updates(
                    duration,
                    package_name,
                    idx + 1,
                    total_files,
                    "Rate limit",
                    state,
                    repaint_signal,
                    Some(max_wait_time),
                ) {
                    loop_err = Some(e);
                    break;
                }

                // Update status back to checking API
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
            } else {
                break;
            }
        }

        if let Some(e) = loop_err {
            log::warn!("Skipping file {} due to timeout: {}", file_path, e);
            last_error = Some(e);
            continue;
        }

        log::info!("Querying Hybrid Analysis API for SHA256: {}", sha256);

        // Final timeout check before request
        if Instant::now() > max_wait_time {
            last_error = Some("Timeout before API request".to_string());
            continue;
        }

        // Record the request BEFORE making the API call to reserve the slot
        {
            let mut limiter = rate_limiter.lock().unwrap();
            limiter.record_request();
        }

        last_was_cache = false;

        // Use a timeout logic for search_hash if possible, but the function block is blocking.
        // We'll update search_hash to use ureq timeout in api_hybridanalysis.rs later.
        match api_hybridanalysis::search_hash(sha256, api_key) {
            Ok(response) => {
                log::info!("Got Hybrid Analysis hash search result for {}", sha256);

                // Check if there are any reports
                if response.reports.is_empty() {
                    log::warn!("No reports found for SHA256: {}", sha256);

                    // If upload is allowed, upload the file
                    if allow_upload {
                        // Check timeout before upload
                        if Instant::now() > max_wait_time {
                            last_error = Some("Timeout before upload".to_string());
                            continue;
                        }

                        if let Err(e) = handle_file_upload(
                            package_name,
                            file_path,
                            sha256,
                            device_serial,
                            api_key,
                            &config,
                            rate_limiter,
                            &mut file_results,
                            state,
                            repaint_signal,
                            idx + 1,
                            total_files,
                            max_wait_time,
                        ) {
                            let err_msg = format!("Upload failed: {}", e);
                            log::warn!("Upload failed for {}: {}", file_path, e);
                            last_error = Some(err_msg);
                        }
                    }
                    continue;
                }

                // Get the first report (Android Static Analysis preferred)
                let report_info = response
                    .reports
                    .iter()
                    .find(|r| {
                        r.environment_description.as_deref() == Some("Android Static Analysis")
                    })
                    .or_else(|| response.reports.first())
                    .unwrap();

                // Get full report details
                let mut loop_err = None;
                loop {
                    if Instant::now() > max_wait_time {
                        loop_err = Some("Timeout waiting for report rate limit".to_string());
                        break;
                    }

                    let wait_duration = {
                        let mut limiter = rate_limiter.lock().unwrap();
                        limiter.check_wait_needed()
                    };

                    if let Some(duration) = wait_duration {
                        if Instant::now() + duration > max_wait_time {
                            loop_err = Some(format!(
                                "Waited too long for report rate limit: {:?}",
                                duration
                            ));
                            break;
                        }

                        log::debug!("Waiting {:?} before fetching report", duration);
                        if let Err(e) = sleep_with_updates(
                            duration,
                            package_name,
                            idx + 1,
                            total_files,
                            "Rate limit",
                            state,
                            repaint_signal,
                            Some(max_wait_time),
                        ) {
                            loop_err = Some(e);
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if let Some(e) = loop_err {
                    log::warn!("Skipping report for {} due to timeout: {}", file_path, e);
                    last_error = Some(e);
                    continue;
                }

                log::info!("Fetching report summary for report ID: {}", report_info.id);

                // Record the request BEFORE making the API call to reserve the slot
                {
                    let mut limiter = rate_limiter.lock().unwrap();
                    limiter.record_request();
                }

                match api_hybridanalysis::get_report_summary(&report_info.id, api_key) {
                    Ok(report) => {
                        log::info!("Got Hybrid Analysis report for {}", sha256);

                        // Save to database via queue
                        db_hybridanalysis::queue_upsert(
                            package_name.to_string(),
                            file_path.clone(),
                            sha256.clone(),
                            report.clone(),
                        )?;

                        file_results.push(FileScanResult {
                            file_path: file_path.clone(),
                            sha256: sha256.clone(),
                            verdict: report.verdict.clone(),
                            threat_score: report.threat_score,
                            threat_level: report.threat_level,
                            classification_tags: report.classification_tags.clone(),
                            total_signatures: report.total_signatures,
                            ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                            wait_until: None,
                            job_id: None,
                            error_message: None,
                        });
                    }
                    Err(HaError::RateLimit { retry_after }) => {
                        rate_limiter
                            .lock()
                            .unwrap()
                            .set_rate_limit(Duration::from_secs(retry_after));
                    }
                    Err(e) => {
                        let err_msg = format!("Error getting report: {}", e);
                        log::error!(
                            "Error getting Hybrid Analysis report for {} (pkg: {}, file: {}): {}",
                            sha256,
                            package_name,
                            file_path,
                            e
                        );
                        last_error = Some(err_msg);
                    }
                }
            }
            Err(HaError::NotFound) if allow_upload => {
                log::info!(
                    "File {} sha256 {} not found in Hybrid Analysis, uploading",
                    file_path,
                    sha256
                );

                // Check timeout before upload
                if Instant::now() > max_wait_time {
                    last_error = Some("Timeout before upload".to_string());
                    continue;
                }

                if let Err(e) = handle_file_upload(
                    package_name,
                    file_path,
                    sha256,
                    device_serial,
                    api_key,
                    &config,
                    rate_limiter,
                    &mut file_results,
                    state,
                    repaint_signal,
                    idx + 1,
                    total_files,
                    max_wait_time,
                ) {
                    let err_msg = format!("Upload failed: {}", e);
                    log::warn!("Upload failed for {}: {}", file_path, e);
                    last_error = Some(err_msg);
                }
            }
            Err(HaError::NotFound) => {
                log::warn!(
                    "Sha256 {} for file {} not found in Hybrid Analysis and upload is disabled or api limit reached.",
                    sha256,
                    file_path
                );

                // Create a 404 Not Found response to cache
                let not_found_response = api_hybridanalysis::HybridAnalysisReportResponse {
                    classification_tags: Vec::new(),
                    tags: Vec::new(),
                    submissions: Vec::new(),
                    warnings: Vec::new(),
                    job_id: "".to_string(),
                    environment_id: 0,
                    environment_description: "N/A".to_string(),
                    state: "not_found".to_string(),
                    error_type: None,
                    error_origin: None,
                    submit_name: "N/A".to_string(),
                    md5: "".to_string(),
                    sha1: "".to_string(),
                    sha256: sha256.to_string(),
                    sha512: None,
                    threat_score: None,
                    threat_level: None,
                    verdict: "404 Not Found".to_string(),
                    total_network_connections: None,
                    total_processes: None,
                    total_signatures: None,
                };

                // Save to database via queue
                if let Err(e) = db_hybridanalysis::queue_upsert(
                    package_name.to_string(),
                    file_path.clone(),
                    sha256.clone(),
                    not_found_response,
                ) {
                    log::error!("Failed to cache 404 for {}: {}", sha256, e);
                }

                // Add to results so it shows up in UI
                file_results.push(FileScanResult {
                    file_path: file_path.clone(),
                    sha256: sha256.clone(),
                    verdict: "404 Not Found".to_string(),
                    threat_score: None,
                    threat_level: None,
                    classification_tags: Vec::new(),
                    total_signatures: None,
                    ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                    wait_until: None,
                    job_id: None,
                    error_message: None,
                });
            }
            Err(HaError::RateLimit { retry_after }) => {
                rate_limiter
                    .lock()
                    .unwrap()
                    .set_rate_limit(Duration::from_secs(retry_after));
            }
            Err(e) => {
                let err_msg = e.to_string();
                log::error!(
                    "Error searching Hybrid Analysis for {} (pkg: {}, file: {}): {}",
                    sha256,
                    package_name,
                    file_path,
                    e
                );
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
            // No results but no specific error (maybe not found and upload disabled)
            // We still mark as completed with empty results
            let result = CalcHybridAnalysis { file_results };
            s.insert(package_name.to_string(), ScanStatus::Completed(result));
        }
    } else {
        let result = CalcHybridAnalysis { file_results };
        s.insert(package_name.to_string(), ScanStatus::Completed(result));
    }
    if let Some(signal) = repaint_signal {
        signal();
    }

    Ok(())
}

/// Handle file upload to Hybrid Analysis
fn handle_file_upload(
    package_name: &str,
    file_path: &str,
    sha256: &str,
    device_serial: &str,
    api_key: &str,
    config: &Config,
    rate_limiter: &SharedRateLimiter,
    file_results: &mut Vec<FileScanResult>,
    state: &ScannerState,
    repaint_signal: &Option<Arc<dyn Fn() + Send + Sync>>,
    idx: usize,
    total_files: usize,
    max_wait_time: Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!(
        "Starting file upload process for {} (sha256: {})",
        file_path,
        sha256
    );

    // Update status - pulling file
    {
        let mut s = state.lock().unwrap();
        s.insert(
            package_name.to_string(),
            ScanStatus::Scanning {
                scanned: idx,
                total: total_files,
                operation: "Pulling file".to_string(),
            },
        );
    }
    if let Some(signal) = repaint_signal {
        signal();
    }

    // Pull file to temp directory

    // Skip if the path appears to be a directory (doesn't have a file extension or ends with /)
    if file_path.ends_with('/') || !file_path.contains('.') {
        log::warn!("Skipping directory or invalid path: {}", file_path);
        return Err(format!("Path is a directory, not a file: {}", file_path).into());
    }

    // Only upload APK and SO files to Hybrid Analysis (skip .prof, .dm, .art, etc.)
    if !file_path.ends_with(".apk") && !file_path.ends_with(".so") {
        log::info!(
            "Skipping file for upload: {} (only .apk and .so files are uploaded)",
            file_path
        );
        return Err(format!("Not an APK or SO file, skipping upload: {}", file_path).into());
    }

    // Ensure tmp directory exists
    if let Err(e) = std::fs::create_dir_all(&config.tmp_dir) {
        log::error!("Failed to create tmp directory {:?}: {}", config.tmp_dir, e);
        return Err(format!("Failed to create tmp directory: {}", e).into());
    }

    let tmp_dir_str = config.tmp_dir.to_str().ok_or("Invalid tmp directory path")?;
    let expected_filename = format!("{}.apk", package_name.replace('.', "_"));
    let local_path = config.tmp_dir.join(&expected_filename);

    log::info!(
        "Pulling file from device: {} -> {}",
        file_path,
        local_path.display()
    );
    #[cfg(not(target_os = "android"))]
    if let Err(e) = adb::pull_file_to_temp(device_serial, file_path, tmp_dir_str, package_name) {
        log::error!("Failed to pull file {} from device: {}", file_path, e);
        return Err(Box::new(e));
    }
    #[cfg(target_os = "android")]
    {
        log::error!("File pulling is not supported on Android platform");
        return Err("File pulling not supported on Android".into());
    }
    
    // Verify the file was actually pulled
    if !local_path.exists() {
        let error_msg = format!("File was not pulled successfully: {} does not exist after pull", local_path.display());
        log::error!("{}", error_msg);
        return Err(error_msg.into());
    }
    
    // Check file size
    let file_size = std::fs::metadata(&local_path)
        .map(|m| m.len())
        .unwrap_or(0);
    
    log::info!("File pulled successfully: {} ({} bytes)", local_path.display(), file_size);

    // Update status - uploading
    {
        let mut s = state.lock().unwrap();
        s.insert(
            package_name.to_string(),
            ScanStatus::Scanning {
                scanned: idx,
                total: total_files,
                operation: "Uploading".to_string(),
            },
        );
    }
    if let Some(signal) = repaint_signal {
        signal();
    }

    // Wait for upload rate limits before uploading
    log::debug!("About to check upload rate limits");
    loop {
        if Instant::now() > max_wait_time {
            let _ = std::fs::remove_file(&local_path);
            return Err("Timeout waiting for upload rate limit".into());
        }

        let wait_duration = {
            let mut limiter = rate_limiter.lock().unwrap();
            log::debug!("Rate limiter lock acquired, checking upload limits");
            limiter.check_upload_wait_needed()
        };
        log::debug!("Rate limiter lock released");

        if let Some(duration) = wait_duration {
            if Instant::now() + duration > max_wait_time {
                let _ = std::fs::remove_file(&local_path);
                return Err("Waited too long for upload rate limit".into());
            }

            log::info!("Waiting {:?} before upload", duration);
            if let Err(e) = sleep_with_updates(
                duration,
                package_name,
                idx,
                total_files,
                "Upload rate limit",
                state,
                repaint_signal,
                Some(max_wait_time),
            ) {
                let _ = std::fs::remove_file(&local_path);
                return Err(e.into());
            }
        } else {
            break;
        }
    }

    log::debug!("Upload rate limit check complete");
    log::info!("Uploading file to Hybrid Analysis: {}", file_path);

    // Record the upload request BEFORE making the API call to reserve the slot
    {
        let mut limiter = rate_limiter.lock().unwrap();
        limiter.record_request();
    }

    match api_hybridanalysis::ha_submit_file(&local_path, api_key) {
        Ok(scan_response) => {
            log::info!(
                "Uploaded file {}, job_id: {}, submission_id: {}",
                sha256,
                scan_response.job_id,
                scan_response.submission_id
            );

            // Clean up temp file immediately after upload
            let _ = std::fs::remove_file(&local_path);

            let job_id = scan_response.job_id.clone();

            // Check job state once immediately after upload (non-blocking approach)
            // Check rate limits before polling
            {
                let mut limiter = rate_limiter.lock().unwrap();
                if let Some(wait) = limiter.check_wait_needed() {
                    drop(limiter);
                    thread::sleep(wait);
                }
            }

            // Record request and check job state
            {
                let mut limiter = rate_limiter.lock().unwrap();
                limiter.record_request();
            }

            log::info!("Checking job state for job_id: {}", job_id);
            match api_hybridanalysis::get_job_state(&job_id, api_key) {
                Ok(state_response) => {
                    log::info!("Job {} state: {}", job_id, state_response.state);

                    if state_response.state == "SUCCESS" {
                        // Job completed immediately, fetch report
                        log::info!("Job completed, fetching report for sha256: {}", sha256);

                        // Wait for rate limit
                        {
                            let mut limiter = rate_limiter.lock().unwrap();
                            if let Some(wait) = limiter.check_wait_needed() {
                                drop(limiter);
                                thread::sleep(wait);
                            }
                        }
                        {
                            let mut limiter = rate_limiter.lock().unwrap();
                            limiter.record_request();
                        }

                        // Search for the hash again to get report ID
                        match api_hybridanalysis::search_hash(sha256, api_key) {
                            Ok(hash_response) if !hash_response.reports.is_empty() => {
                                let report_info = hash_response
                                    .reports
                                    .iter()
                                    .find(|r| {
                                        r.environment_description.as_deref()
                                            == Some("Android Static Analysis")
                                    })
                                    .or_else(|| hash_response.reports.first())
                                    .unwrap();

                                // Wait for rate limit
                                {
                                    let mut limiter = rate_limiter.lock().unwrap();
                                    if let Some(wait) = limiter.check_wait_needed() {
                                        drop(limiter);
                                        thread::sleep(wait);
                                    }
                                }
                                {
                                    let mut limiter = rate_limiter.lock().unwrap();
                                    limiter.record_request();
                                }

                                // Get full report
                                match api_hybridanalysis::get_report_summary(
                                    &report_info.id,
                                    api_key,
                                ) {
                                    Ok(report) => {
                                        log::info!(
                                            "Got report for uploaded file {}",
                                            sha256
                                        );

                                        // Cache result
                                        let _ = db_hybridanalysis::queue_upsert(
                                            package_name.to_string(),
                                            file_path.to_string(),
                                            sha256.to_string(),
                                            report.clone(),
                                        );

                                        file_results.push(FileScanResult {
                                            file_path: file_path.to_string(),
                                            sha256: sha256.to_string(),
                                            verdict: report.verdict.clone(),
                                            threat_score: report.threat_score,
                                            threat_level: report.threat_level,
                                            classification_tags: report
                                                .classification_tags
                                                .clone(),
                                            total_signatures: report.total_signatures,
                                            ha_link: format!(
                                                "https://hybrid-analysis.com/sample/{}",
                                                sha256
                                            ),
                                            wait_until: None,
                                            job_id: None,
                                            error_message: None,
                                        });
                                        return Ok(());
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Failed to get report after upload: {}",
                                            e
                                        );
                                    }
                                }
                            }
                            Ok(_) => {
                                let error_msg = format!(
                                    "No report after job completion (job_id: {}). File may exceed HA upload size limit.",
                                    &job_id[..job_id.len().min(8)]
                                );
                                log::warn!(
                                    "No reports found after job completion for sha256: {} (pkg: {}, file: {}, job_id: {}). File may exceed HA size limit.",
                                    sha256, package_name, file_path, job_id
                                );

                                // Save error to database for persistence
                                {
                                    let mut conn = crate::db::establish_connection();
                                    if let Err(db_err) = crate::db_hybridanalysis::save_error_result(
                                        &mut conn,
                                        package_name,
                                        file_path,
                                        sha256,
                                        "upload_error",
                                        &error_msg,
                                    ) {
                                        log::error!("Failed to save error to database: {}", db_err);
                                    }
                                }

                                file_results.push(FileScanResult {
                                    file_path: file_path.to_string(),
                                    sha256: sha256.to_string(),
                                    verdict: "upload_error".to_string(),
                                    threat_score: None,
                                    threat_level: None,
                                    classification_tags: Vec::new(),
                                    total_signatures: None,
                                    ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                                    wait_until: None,
                                    job_id: None,
                                    error_message: Some(error_msg),
                                });
                                return Ok(());
                            }
                            Err(e) => {
                                let error_msg = format!(
                                    "Hash not found after job completion (job_id: {}): {}. File may exceed HA upload size limit.",
                                    &job_id[..job_id.len().min(8)], e
                                );
                                log::error!(
                                    "Failed to search hash after job completion for sha256: {} (pkg: {}, file: {}, job_id: {}): {}",
                                    sha256, package_name, file_path, job_id, e
                                );

                                // Save error to database for persistence
                                {
                                    let mut conn = crate::db::establish_connection();
                                    if let Err(db_err) = crate::db_hybridanalysis::save_error_result(
                                        &mut conn,
                                        package_name,
                                        file_path,
                                        sha256,
                                        "upload_error",
                                        &error_msg,
                                    ) {
                                        log::error!("Failed to save error to database: {}", db_err);
                                    }
                                }

                                file_results.push(FileScanResult {
                                    file_path: file_path.to_string(),
                                    sha256: sha256.to_string(),
                                    verdict: "upload_error".to_string(),
                                    threat_score: None,
                                    threat_level: None,
                                    classification_tags: Vec::new(),
                                    total_signatures: None,
                                    ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                                    wait_until: None,
                                    job_id: None,
                                    error_message: Some(error_msg),
                                });
                                return Ok(());
                            }
                        }
                    } else if state_response.state == "ERROR" {
                        let error_msg = format!(
                            "{}: {}",
                            state_response.error_type.as_deref().unwrap_or("Unknown error"),
                            state_response.error_origin.as_deref().unwrap_or("Unknown origin")
                        );
                        log::error!(
                            "Job {} failed with error: {}",
                            job_id,
                            error_msg
                        );

                        // Save error to database for persistence
                        {
                            let mut conn = crate::db::establish_connection();
                            if let Err(db_err) = crate::db_hybridanalysis::save_error_result(
                                &mut conn,
                                package_name,
                                file_path,
                                sha256,
                                "analysis_error",
                                &error_msg,
                            ) {
                                log::error!("Failed to save error to database: {}", db_err);
                            }
                        }

                        file_results.push(FileScanResult {
                            file_path: file_path.to_string(),
                            sha256: sha256.to_string(),
                            verdict: "analysis_error".to_string(),
                            threat_score: None,
                            threat_level: None,
                            classification_tags: Vec::new(),
                            total_signatures: None,
                            ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                            wait_until: None,
                            job_id: None,
                            error_message: Some(error_msg),
                        });
                        return Ok(());
                    }
                    // Otherwise state is IN_QUEUE or IN_PROGRESS - fall through to pending_analysis
                }
                Err(HaError::RateLimit { retry_after }) => {
                    rate_limiter
                        .lock()
                        .unwrap()
                        .set_rate_limit(Duration::from_secs(retry_after));
                    log::warn!("Rate limited while checking job state");
                    // Fall through to pending_analysis
                }
                Err(e) => {
                    log::error!("Error checking job state: {}", e);
                    // Fall through to pending_analysis
                }
            }

            // Job is still pending (IN_QUEUE or IN_PROGRESS) - add as pending_analysis
            // and move on to the next file/package. Will check back later.
            log::info!(
                "Job {} is pending (IN_QUEUE/IN_PROGRESS), will check back later",
                job_id
            );

            file_results.push(FileScanResult {
                file_path: file_path.to_string(),
                sha256: sha256.to_string(),
                verdict: "pending_analysis".to_string(),
                threat_score: None,
                threat_level: None,
                classification_tags: Vec::new(),
                total_signatures: None,
                ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                wait_until: None,
                job_id: Some(job_id.clone()),
                error_message: None,
            });
        }
        Err(HaError::RateLimit { retry_after: _ }) => {
            // For upload 429 errors, set 24 hour wait and show in UI
            rate_limiter.lock().unwrap().set_upload_rate_limit();
            log::error!("Upload rate limited (429), will wait 24 hours before next upload");

            use std::time::{SystemTime, UNIX_EPOCH};
            let wait_until_ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 86400; // 24 hours

            file_results.push(FileScanResult {
                file_path: file_path.to_string(),
                sha256: sha256.to_string(),
                verdict: "rate_limited".to_string(),
                threat_score: None,
                threat_level: None,
                classification_tags: Vec::new(),
                total_signatures: None,
                ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                wait_until: Some(wait_until_ts),
                job_id: None,
                error_message: None,
            });

            // Clean up temp file
            let _ = std::fs::remove_file(&local_path);
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            log::error!("Failed to upload file: {}", error_msg);

            // Save error to database for persistence
            {
                let mut conn = crate::db::establish_connection();
                if let Err(db_err) = crate::db_hybridanalysis::save_error_result(
                    &mut conn,
                    package_name,
                    file_path,
                    sha256,
                    "upload_error",
                    &error_msg,
                ) {
                    log::error!("Failed to save error to database: {}", db_err);
                }
            }

            // Add error result to show in UI
            file_results.push(FileScanResult {
                file_path: file_path.to_string(),
                sha256: sha256.to_string(),
                verdict: "upload_error".to_string(),
                threat_score: None,
                threat_level: None,
                classification_tags: Vec::new(),
                total_signatures: None,
                ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                wait_until: None,
                job_id: None,
                error_message: Some(error_msg),
            });

            // Clean up temp file
            let _ = std::fs::remove_file(&local_path);
        }
    }

    Ok(())
}

/// Check pending jobs across all packages and update their status
/// Returns the count of jobs still pending
pub fn check_pending_jobs(
    state: &ScannerState,
    rate_limiter: &SharedRateLimiter,
    api_key: &str,
    repaint_signal: &Option<Arc<dyn Fn() + Send + Sync>>,
) -> usize {
    let mut pending_count = 0;

    // Collect pending jobs from all completed packages
    let pending_jobs: Vec<(String, String, String, String)> = {
        let state_guard = state.lock().unwrap();
        let mut jobs = Vec::new();

        for (package_name, scan_status) in state_guard.iter() {
            if let ScanStatus::Completed(result) = scan_status {
                for file_result in &result.file_results {
                    if file_result.verdict == "pending_analysis" {
                        if let Some(ref job_id) = file_result.job_id {
                            jobs.push((
                                package_name.clone(),
                                file_result.file_path.clone(),
                                file_result.sha256.clone(),
                                job_id.clone(),
                            ));
                        }
                    }
                }
            }
        }

        jobs
    };

    log::info!("Checking {} pending jobs", pending_jobs.len());

    for (package_name, file_path, sha256, job_id) in pending_jobs {
        // Check rate limits before polling
        {
            let mut limiter = rate_limiter.lock().unwrap();
            if let Some(wait) = limiter.check_wait_needed() {
                drop(limiter);
                thread::sleep(wait);
            }
        }

        // Record request
        {
            let mut limiter = rate_limiter.lock().unwrap();
            limiter.record_request();
        }

        log::info!("Checking job state for job_id: {} (sha256: {})", job_id, sha256);

        match api_hybridanalysis::get_job_state(&job_id, api_key) {
            Ok(state_response) => {
                log::info!("Job {} state: {}", job_id, state_response.state);

                if state_response.state == "SUCCESS" {
                    // Job completed, fetch report
                    log::info!("Job {} completed, fetching report for sha256: {}", job_id, sha256);

                    // Wait for rate limit
                    {
                        let mut limiter = rate_limiter.lock().unwrap();
                        if let Some(wait) = limiter.check_wait_needed() {
                            drop(limiter);
                            thread::sleep(wait);
                        }
                    }
                    {
                        let mut limiter = rate_limiter.lock().unwrap();
                        limiter.record_request();
                    }

                    // Search for the hash to get report ID
                    match api_hybridanalysis::search_hash(&sha256, api_key) {
                        Ok(hash_response) if !hash_response.reports.is_empty() => {
                            let report_info = hash_response
                                .reports
                                .iter()
                                .find(|r| {
                                    r.environment_description.as_deref()
                                        == Some("Android Static Analysis")
                                })
                                .or_else(|| hash_response.reports.first())
                                .unwrap();

                            // Wait for rate limit
                            {
                                let mut limiter = rate_limiter.lock().unwrap();
                                if let Some(wait) = limiter.check_wait_needed() {
                                    drop(limiter);
                                    thread::sleep(wait);
                                }
                            }
                            {
                                let mut limiter = rate_limiter.lock().unwrap();
                                limiter.record_request();
                            }

                            // Get full report
                            match api_hybridanalysis::get_report_summary(&report_info.id, api_key) {
                                Ok(report) => {
                                    log::info!("Got report for job {}", job_id);

                                    // Cache result
                                    let _ = db_hybridanalysis::queue_upsert(
                                        package_name.clone(),
                                        file_path.clone(),
                                        sha256.clone(),
                                        report.clone(),
                                    );

                                    // Update the file result in state
                                    update_file_result(
                                        state,
                                        &package_name,
                                        &sha256,
                                        FileScanResult {
                                            file_path: file_path.clone(),
                                            sha256: sha256.clone(),
                                            verdict: report.verdict.clone(),
                                            threat_score: report.threat_score,
                                            threat_level: report.threat_level,
                                            classification_tags: report.classification_tags.clone(),
                                            total_signatures: report.total_signatures,
                                            ha_link: format!(
                                                "https://hybrid-analysis.com/sample/{}",
                                                sha256
                                            ),
                                            wait_until: None,
                                            job_id: None,
                                            error_message: None,
                                        },
                                    );

                                    if let Some(signal) = repaint_signal {
                                        signal();
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to get report for job {}: {}", job_id, e);
                                    pending_count += 1;
                                }
                            }
                        }
                        Ok(_) => {
                            log::warn!(
                                "No reports found after job completion for sha256: {} (pkg: {}, file: {}, job_id: {}). File may exceed HA size limit.",
                                sha256, package_name, file_path, job_id
                            );
                            // Stop retrying: mark as error instead of keeping pending
                            update_file_result(
                                state,
                                &package_name,
                                &sha256,
                                FileScanResult {
                                    file_path: file_path.clone(),
                                    sha256: sha256.clone(),
                                    verdict: "upload_error".to_string(),
                                    threat_score: None,
                                    threat_level: None,
                                    classification_tags: Vec::new(),
                                    total_signatures: None,
                                    ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                                    wait_until: None,
                                    job_id: None,
                                    error_message: Some(format!(
                                        "No report after job completion (job_id: {}). File may exceed HA upload size limit.",
                                        &job_id[..job_id.len().min(8)]
                                    )),
                                },
                            );
                            if let Some(signal) = repaint_signal {
                                signal();
                            }
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to search hash after job completion for sha256: {} (pkg: {}, file: {}, job_id: {}): {}",
                                sha256, package_name, file_path, job_id, e
                            );
                            // Stop retrying: mark as error instead of keeping pending
                            update_file_result(
                                state,
                                &package_name,
                                &sha256,
                                FileScanResult {
                                    file_path: file_path.clone(),
                                    sha256: sha256.clone(),
                                    verdict: "upload_error".to_string(),
                                    threat_score: None,
                                    threat_level: None,
                                    classification_tags: Vec::new(),
                                    total_signatures: None,
                                    ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                                    wait_until: None,
                                    job_id: None,
                                    error_message: Some(format!(
                                        "Hash not found after job completion (job_id: {}): {}. File may exceed HA upload size limit.",
                                        &job_id[..job_id.len().min(8)], e
                                    )),
                                },
                            );
                            if let Some(signal) = repaint_signal {
                                signal();
                            }
                        }
                    }
                } else if state_response.state == "ERROR" {
                    log::error!(
                        "Job {} failed with error: {:?} ({:?})",
                        job_id,
                        state_response.error_type,
                        state_response.error_origin
                    );

                    // Update the file result to show error
                    update_file_result(
                        state,
                        &package_name,
                        &sha256,
                        FileScanResult {
                            file_path: file_path.clone(),
                            sha256: sha256.clone(),
                            verdict: "analysis_error".to_string(),
                            threat_score: None,
                            threat_level: None,
                            classification_tags: Vec::new(),
                            total_signatures: None,
                            ha_link: format!("https://hybrid-analysis.com/sample/{}", sha256),
                            wait_until: None,
                            job_id: None,
                            error_message: None,
                        },
                    );

                    if let Some(signal) = repaint_signal {
                        signal();
                    }
                } else {
                    // Still IN_QUEUE or IN_PROGRESS
                    log::info!("Job {} still pending ({})", job_id, state_response.state);
                    pending_count += 1;
                }
            }
            Err(HaError::RateLimit { retry_after }) => {
                rate_limiter
                    .lock()
                    .unwrap()
                    .set_rate_limit(Duration::from_secs(retry_after));
                log::warn!("Rate limited while checking job state for {}", job_id);
                pending_count += 1;
            }
            Err(e) => {
                log::error!("Error checking job state for {}: {}", job_id, e);
                pending_count += 1;
            }
        }
    }

    pending_count
}

/// Helper function to update a specific file result in the scanner state
fn update_file_result(
    state: &ScannerState,
    package_name: &str,
    sha256: &str,
    new_result: FileScanResult,
) {
    let mut state_guard = state.lock().unwrap();

    if let Some(ScanStatus::Completed(result)) = state_guard.get_mut(package_name) {
        if let Some(file_result) = result
            .file_results
            .iter_mut()
            .find(|fr| fr.sha256 == sha256)
        {
            *file_result = new_result;
        }
    }
}

/// Run Hybrid Analysis scanning for a list of packages in a background thread.
/// This function initializes the scanner state and spawns a background thread
/// to scan all packages using the Hybrid Analysis API.
///
/// # Arguments
/// * `installed_packages` - List of packages to scan
/// * `device_serial` - Device serial number for ADB operations
/// * `api_key` - Hybrid Analysis API key
/// * `hybridanalysis_submit_enabled` - Whether to submit unknown files to Hybrid Analysis
/// * `package_risk_scores` - Risk scores for sorting packages by priority
/// * `ha_scan_progress` - Shared progress value for UI updates
/// * `ha_scan_cancelled` - Shared cancellation flag
///
/// # Returns
/// Returns the scanner state and rate limiter for tracking progress
pub fn run_hybridanalysis(
    installed_packages: Vec<crate::adb::PackageFingerprint>,
    device_serial: String,
    api_key: String,
    hybridanalysis_submit_enabled: bool,
    package_risk_scores: HashMap<String, i32>,
    ha_scan_progress: Arc<Mutex<Option<f32>>>,
    ha_scan_cancelled: Arc<Mutex<bool>>,
) -> (ScannerState, SharedRateLimiter) {
    let package_names: Vec<String> = installed_packages.iter().map(|p| p.pkg.clone()).collect();
    let scanner_state = init_scanner_state(&package_names);

    let rate_limiter = Arc::new(Mutex::new(RateLimiter::new(Duration::from_secs(3))));

    // Initialize progress
    if let Ok(mut p) = ha_scan_progress.lock() {
        *p = Some(0.0);
    }
    if let Ok(mut cancelled) = ha_scan_cancelled.lock() {
        *cancelled = false;
    }

    log::info!(
        "Starting HybridAnalysis scan for {} packages",
        installed_packages.len()
    );

    let scanner_state_clone = scanner_state.clone();
    let rate_limiter_clone = rate_limiter.clone();
    let ha_scan_progress_clone = ha_scan_progress;
    let ha_scan_cancelled_clone = ha_scan_cancelled;

    std::thread::spawn(move || {
        let mut effective_submit_enabled = hybridanalysis_submit_enabled;
        log::info!("Checking Hybrid Analysis API quota...");
        match crate::api_hybridanalysis::check_quota(&api_key) {
            Ok(quota) => {
                if let Some(detonation) = quota.detonation {
                    if detonation.quota_reached {
                        log::warn!("Hybrid Analysis detonation quota reached!");
                        effective_submit_enabled = false;
                    }
                    if let Some(apikey_info) = detonation.apikey {
                        if apikey_info.quota_reached {
                            log::warn!("Hybrid Analysis API key quota reached!");
                            effective_submit_enabled = false;
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to check Hybrid Analysis quota: {}", e);
            }
        }

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
            if let Ok(cancelled) = ha_scan_cancelled_clone.lock() {
                if *cancelled {
                    log::info!("Hybrid Analysis scan cancelled by user");
                    break;
                }
            }

            if let Ok(mut p) = ha_scan_progress_clone.lock() {
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
                                        "Failed to get sha256sums for {}: {}, using cached",
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
                    effective_submit_enabled,
                    &None,
                ) {
                    log::error!("Error analyzing package {}: {}", pkg_name, e);
                }
            } else {
                log::error!("Failed to get path and sha256 for package {}", pkg_name);
            }
        }

        log::info!(
            "Hybrid Analysis scan complete: {} cached, {} processed",
            skipped_cached,
            total - skipped_cached
        );

        // Second pass: poll pending jobs
        log::info!("Checking for pending jobs...");
        loop {
            if let Ok(cancelled) = ha_scan_cancelled_clone.lock() {
                if *cancelled {
                    log::info!("Hybrid Analysis scan cancelled during pending check");
                    break;
                }
            }

            let pending_count = check_pending_jobs(
                &scanner_state_clone,
                &rate_limiter_clone,
                &api_key,
                &None,
            );

            if pending_count == 0 {
                log::info!("All pending jobs completed");
                break;
            }

            log::info!("{} jobs still pending, waiting 30 seconds", pending_count);

            for _ in 0..30 {
                if let Ok(cancelled) = ha_scan_cancelled_clone.lock() {
                    if *cancelled {
                        log::info!("Hybrid Analysis scan cancelled during wait");
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

    (scanner_state, rate_limiter)
}

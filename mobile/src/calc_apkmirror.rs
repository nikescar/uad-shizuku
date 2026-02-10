use crate::api_apkmirror::{fetch_app_details, ApkMirrorAppInfo};
pub use crate::calc_apkmirror_stt::*;
use crate::db_apkmirror::{get_apkmirror_app, is_cache_stale, upsert_apkmirror_app};
use crate::is_valid_package_id;
use crate::models::ApkMirrorApp;
use anyhow::Result;
use diesel::prelude::*;
use egui_i18n::tr;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

impl ApkMirrorQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            results: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(Mutex::new(false)),
            email: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Set the email for APKMirror requests
    pub fn set_email(&self, email: String) {
        let mut email_lock = self.email.lock().unwrap();
        *email_lock = email;
    }

    /// Get the current email
    pub fn get_email(&self) -> String {
        let email_lock = self.email.lock().unwrap();
        email_lock.clone()
    }

    /// Add a package ID to the fetch queue
    pub fn enqueue(&self, package_id: String) {
        // Skip package IDs with less than 2 domain levels (e.g., com.android)
        if !is_valid_package_id(&package_id) {
            // Mark as error so it won't be re-queued repeatedly
            let mut results = self.results.lock().unwrap();
            if !results.contains_key(&package_id) {
                log::debug!(
                    "Skipping APKMirror fetch for invalid package ID: {}",
                    package_id
                );
                results.insert(
                    package_id,
                    ApkMirrorFetchStatus::Error(tr!("error-invalid-package-id")),
                );
            }
            return;
        }

        let mut queue = self.queue.lock().unwrap();
        let mut results = self.results.lock().unwrap();

        // Don't add if already in queue or being processed
        if !queue.contains(&package_id) && !results.contains_key(&package_id) {
            queue.push_back(package_id.clone());
            results.insert(package_id, ApkMirrorFetchStatus::Pending);
        }
    }

    /// Add multiple package IDs to the fetch queue
    pub fn enqueue_batch(&self, package_ids: Vec<String>) {
        for package_id in package_ids {
            self.enqueue(package_id);
        }
    }

    /// Get the status of a package fetch
    pub fn get_status(&self, package_id: &str) -> Option<ApkMirrorFetchStatus> {
        let results = self.results.lock().unwrap();
        results.get(package_id).cloned()
    }

    /// Get result if successfully fetched
    pub fn get_result(&self, package_id: &str) -> Option<ApkMirrorApp> {
        let results = self.results.lock().unwrap();
        if let Some(ApkMirrorFetchStatus::Success(app)) = results.get(package_id) {
            Some(app.clone())
        } else {
            None
        }
    }

    /// Start the background worker thread
    pub fn start_worker(&self, _db_path: String) {
        let mut is_running = self.is_running.lock().unwrap();

        if *is_running {
            log::warn!("APKMirror worker already running");
            return;
        }

        *is_running = true;
        drop(is_running);

        let queue = self.queue.clone();
        let results = self.results.clone();
        let is_running_clone = self.is_running.clone();
        let email_clone = self.email.clone();

        thread::spawn(move || {
            // Small delay to let the main thread's initial pre-fetch complete
            thread::sleep(Duration::from_millis(500));

            log::info!("APKMirror worker thread started");

            while *is_running_clone.lock().unwrap() {
                // Prioritize cached items: find a cached item first, otherwise take from front
                let package_id = {
                    let mut queue = queue.lock().unwrap();
                    if queue.is_empty() {
                        None
                    } else {
                        // Establish connection to check cache
                        let mut conn = crate::db::establish_connection();

                        // Find first cached item in queue
                        let mut cached_idx = None;
                        for (idx, pkg_id) in queue.iter().enumerate() {
                            if let Ok(Some(cached_app)) = get_apkmirror_app(&mut conn, pkg_id) {
                                if !is_cache_stale(&cached_app) {
                                    cached_idx = Some(idx);
                                    break;
                                }
                            }
                        }

                        // Remove and return cached item if found, otherwise pop front
                        if let Some(idx) = cached_idx {
                            queue.remove(idx)
                        } else {
                            queue.pop_front()
                        }
                    }
                };

                if let Some(pkg_id) = package_id {
                    let results_clone = results.clone();
                    let email_clone = email_clone.clone();
                    let pkg_id_clone = pkg_id.clone();

                    let result = std::panic::catch_unwind(move || -> Duration {
                        // APKMirror rate limits aggressively - use 30 second interval
                        // to stay well under their limits (observed 429 after ~4-6 requests at 15s)
                        let mut next_sleep = Duration::from_secs(30);

                        // Get current email
                        let email = {
                            let email_lock = email_clone.lock().unwrap();
                            email_lock.clone()
                        };

                        // if email.is_empty() {
                        //     log::warn!(
                        //         "APKMirror email not set, skipping fetch for: {}",
                        //         pkg_id_clone
                        //     );
                        //     let mut results = results_clone.lock().unwrap();
                        //     results.insert(
                        //         pkg_id_clone,
                        //         ApkMirrorFetchStatus::Error(tr!("error-email-not-configured")),
                        //     );
                        //     return next_sleep;
                        // }

                        // Update status to fetching
                        {
                            let mut results = results_clone.lock().unwrap();
                            results.insert(pkg_id_clone.clone(), ApkMirrorFetchStatus::Fetching);
                        }

                        log::info!("Processing APKMirror fetch for: {}", pkg_id_clone);

                        // Establish database connection
                        let mut conn = match crate::db::establish_connection() {
                            conn => conn,
                        };

                        // Check cache first
                        match get_apkmirror_app(&mut conn, &pkg_id_clone) {
                            Ok(Some(cached_app)) if !is_cache_stale(&cached_app) => {
                                if cached_app.raw_response == "404" {
                                    log::info!(
                                        "Using cached APKMirror 404 for: {}",
                                        pkg_id_clone
                                    );
                                    let mut results = results_clone.lock().unwrap();
                                    results.insert(
                                        pkg_id_clone,
                                        ApkMirrorFetchStatus::Error(format!(
                                            "{} (cached)",
                                            tr!("error-app-not-found")
                                        )),
                                    );
                                    // No need to rate limit when using cache
                                    return Duration::from_millis(50);
                                }
                                log::info!("Using cached APKMirror data for: {}", pkg_id_clone);
                                let mut results = results_clone.lock().unwrap();
                                results.insert(
                                    pkg_id_clone,
                                    ApkMirrorFetchStatus::Success(cached_app),
                                );
                                // No need to rate limit when using cache
                                return Duration::from_millis(50);
                            }
                            _ => {}
                        }

                        // Fetch from APKMirror
                        match fetch_app_details(&pkg_id_clone, &email) {
                            Ok(app_info) => {
                                // Check if we actually found a result (not just empty/default)
                                if app_info.title == "Unknown" && app_info.developer == "Unknown" {
                                    log::info!(
                                        "APKMirror returned no results for {}, caching as not found",
                                        pkg_id_clone
                                    );
                                    // Save "Not Found" to database
                                    let not_found_app = ApkMirrorAppInfo {
                                        package_id: pkg_id_clone.clone(),
                                        title: "Not Found".to_string(),
                                        developer: "Unknown".to_string(),
                                        version: None,
                                        icon_url: None,
                                        icon_base64: None,
                                        raw_response: "404".to_string(),
                                    };

                                    if let Ok(_) = save_to_db(&mut conn, &not_found_app) {
                                        log::info!("Cached 404 for {}", pkg_id_clone);
                                    }

                                    let mut results = results_clone.lock().unwrap();
                                    results.insert(
                                        pkg_id_clone,
                                        ApkMirrorFetchStatus::Error(tr!("error-app-not-found")),
                                    );
                                } else {
                                    // Save to database
                                    match save_to_db(&mut conn, &app_info) {
                                        Ok(saved_app) => {
                                            log::info!(
                                                "Successfully fetched and saved APKMirror app: {}",
                                                pkg_id_clone
                                            );
                                            let mut results = results_clone.lock().unwrap();
                                            results.insert(
                                                pkg_id_clone,
                                                ApkMirrorFetchStatus::Success(saved_app),
                                            );
                                        }
                                        Err(e) => {
                                            let error_msg = format!("Database save error: {}", e);
                                            log::error!("{}", error_msg);
                                            let mut results = results_clone.lock().unwrap();
                                            results.insert(
                                                pkg_id_clone,
                                                ApkMirrorFetchStatus::Error(error_msg),
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                // Check for specific HTTP errors
                                let mut is_404 = false;
                                let mut is_429 = false;

                                if let Some(ureq_err) = e.downcast_ref::<ureq::Error>() {
                                    match ureq_err {
                                        ureq::Error::Status(404, _) => is_404 = true,
                                        ureq::Error::Status(429, _) => is_429 = true,
                                        _ => {}
                                    }
                                }

                                if is_429 {
                                    // APKMirror 429 backoff: wait 120 seconds before retry
                                    // (60s was still triggering repeated 429s)
                                    log::warn!(
                                        "APKMirror rate limit reached (429) for {}. Waiting 120 seconds.",
                                        pkg_id_clone
                                    );
                                    next_sleep = Duration::from_secs(120);

                                    let mut results = results_clone.lock().unwrap();
                                    results.insert(
                                        pkg_id_clone,
                                        ApkMirrorFetchStatus::Error(tr!(
                                            "error-rate-limit-reached"
                                        )),
                                    );
                                } else if is_404 {
                                    log::info!(
                                        "APKMirror returned 404 for {}, caching as not found",
                                        pkg_id_clone
                                    );
                                    // Save "Not Found" to database
                                    let not_found_app = ApkMirrorAppInfo {
                                        package_id: pkg_id_clone.clone(),
                                        title: "Not Found".to_string(),
                                        developer: "Unknown".to_string(),
                                        version: None,
                                        icon_url: None,
                                        icon_base64: None,
                                        raw_response: "404".to_string(),
                                    };

                                    if let Ok(_) = save_to_db(&mut conn, &not_found_app) {
                                        log::info!("Cached 404 for {}", pkg_id_clone);
                                    }

                                    let mut results = results_clone.lock().unwrap();
                                    results.insert(
                                        pkg_id_clone,
                                        ApkMirrorFetchStatus::Error(tr!("error-app-not-found")),
                                    );
                                } else {
                                    let error_msg = format!("Fetch error: {}", e);
                                    log::warn!("{}", error_msg);
                                    let mut results = results_clone.lock().unwrap();
                                    results.insert(
                                        pkg_id_clone,
                                        ApkMirrorFetchStatus::Error(error_msg),
                                    );
                                }
                            }
                        }

                        next_sleep
                    });

                    let sleep_duration = match result {
                        Ok(duration) => duration,
                        Err(e) => {
                            let error_msg = if let Some(s) = e.downcast_ref::<&str>() {
                                format!("APKMirror worker panicked: {}", s)
                            } else if let Some(s) = e.downcast_ref::<String>() {
                                format!("APKMirror worker panicked: {}", s)
                            } else {
                                "APKMirror worker panicked with unknown error".to_string()
                            };
                            log::error!("{}", error_msg);

                            // Mark as error so it doesn't get stuck in Fetching
                            let mut results = results.lock().unwrap();
                            results.insert(pkg_id, ApkMirrorFetchStatus::Error(error_msg));

                            Duration::from_secs(30) // Default sleep on panic
                        }
                    };

                    // Rate limiting: wait between requests (APKMirror may rate limit)
                    thread::sleep(sleep_duration);
                } else {
                    // No work, sleep a bit
                    thread::sleep(Duration::from_millis(500));
                }
            }

            log::info!("APKMirror worker thread stopped");
        });
    }

    /// Stop the background worker thread
    pub fn stop_worker(&self) {
        let mut is_running = self.is_running.lock().unwrap();
        *is_running = false;
        log::info!("APKMirror worker stopping...");
    }

    /// Clear all pending items from queue
    pub fn clear_queue(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.clear();
        log::info!("APKMirror queue cleared");
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        let queue = self.queue.lock().unwrap();
        queue.len()
    }

    /// Get number of completed fetches
    pub fn completed_count(&self) -> usize {
        let results = self.results.lock().unwrap();
        results
            .values()
            .filter(|status| matches!(status, ApkMirrorFetchStatus::Success(_)))
            .count()
    }
}

/// Save APKMirror app info to database
fn save_to_db(conn: &mut SqliteConnection, app_info: &ApkMirrorAppInfo) -> Result<ApkMirrorApp> {
    upsert_apkmirror_app(
        conn,
        &app_info.package_id,
        &app_info.title,
        &app_info.developer,
        app_info.version.as_deref(),
        app_info.icon_url.as_deref(),
        app_info.icon_base64.as_deref(),
        &app_info.raw_response,
    )
}

impl Default for ApkMirrorQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// APKMirror Upload Queue Implementation
// ============================================================================

use crate::api_apkmirror::{check_apk_uploadable, upload_apk};
use crate::calc_apkmirror_stt::{ApkMirrorUploadItem, ApkMirrorUploadQueue, ApkMirrorUploadStatus};

impl ApkMirrorUploadQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            results: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(Mutex::new(false)),
            email: Arc::new(Mutex::new(String::new())),
            name: Arc::new(Mutex::new(String::new())),
            tmp_dir: Arc::new(Mutex::new(String::new())),
            rate_limit_until: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if currently rate limited
    pub fn is_rate_limited(&self) -> bool {
        let rate_limit = self.rate_limit_until.lock().unwrap();
        if let Some(until) = *rate_limit {
            std::time::Instant::now() < until
        } else {
            false
        }
    }

    /// Get remaining rate limit duration in seconds
    pub fn rate_limit_remaining_secs(&self) -> Option<u64> {
        let rate_limit = self.rate_limit_until.lock().unwrap();
        if let Some(until) = *rate_limit {
            let now = std::time::Instant::now();
            if now < until {
                Some((until - now).as_secs())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Set the email for APKMirror uploads
    pub fn set_email(&self, email: String) {
        let mut email_lock = self.email.lock().unwrap();
        *email_lock = email;
    }

    /// Set the name for APKMirror uploads
    pub fn set_name(&self, name: String) {
        let mut name_lock = self.name.lock().unwrap();
        *name_lock = name;
    }

    /// Set the temp directory for APK files
    pub fn set_tmp_dir(&self, tmp_dir: String) {
        let mut tmp_dir_lock = self.tmp_dir.lock().unwrap();
        *tmp_dir_lock = tmp_dir;
    }

    /// Get the current email
    pub fn get_email(&self) -> String {
        let email_lock = self.email.lock().unwrap();
        email_lock.clone()
    }

    /// Get the current name
    pub fn get_name(&self) -> String {
        let name_lock = self.name.lock().unwrap();
        name_lock.clone()
    }

    /// Add an item to the upload queue after checking version
    pub fn enqueue(&self, item: ApkMirrorUploadItem) {
        // Check if version comparison shows device version is newer
        if !is_version_newer(&item.device_version_name, item.apkmirror_version.as_deref()) {
            let mut results = self.results.lock().unwrap();
            results.insert(
                item.package_id.clone(),
                ApkMirrorUploadStatus::VersionNotNewer,
            );
            log::info!(
                "Skipping upload for {}: device version {} is not newer than APKMirror version {:?}",
                item.package_id,
                item.device_version_name,
                item.apkmirror_version
            );
            return;
        }

        let mut queue = self.queue.lock().unwrap();
        let mut results = self.results.lock().unwrap();

        // Don't add if already in queue or being processed
        if !results.contains_key(&item.package_id) {
            results.insert(item.package_id.clone(), ApkMirrorUploadStatus::Pending);
            queue.push_back(item);
        }
    }

    /// Get the status of an upload
    pub fn get_status(&self, package_id: &str) -> Option<ApkMirrorUploadStatus> {
        let results = self.results.lock().unwrap();
        results.get(package_id).cloned()
    }

    /// Start the background upload worker thread
    pub fn start_worker(&self) {
        let mut is_running = self.is_running.lock().unwrap();

        if *is_running {
            log::warn!("APKMirror upload worker already running");
            return;
        }

        *is_running = true;
        drop(is_running);

        let queue = self.queue.clone();
        let results = self.results.clone();
        let is_running_clone = self.is_running.clone();
        let email_clone = self.email.clone();
        let name_clone = self.name.clone();
        let tmp_dir_clone = self.tmp_dir.clone();
        let rate_limit_until_clone = self.rate_limit_until.clone();

        thread::spawn(move || {
            log::info!("APKMirror upload worker thread started");

            while *is_running_clone.lock().unwrap() {
                // Check if we're rate limited
                {
                    let rate_limit = rate_limit_until_clone.lock().unwrap();
                    if let Some(until) = *rate_limit {
                        if std::time::Instant::now() < until {
                            let remaining = (until - std::time::Instant::now()).as_secs();
                            if remaining % 300 == 0 {
                                // Log every 5 minutes
                                log::info!(
                                    "APKMirror upload rate limited. {} seconds remaining.",
                                    remaining
                                );
                            }
                            drop(rate_limit);
                            thread::sleep(Duration::from_secs(60)); // Check again in 1 minute
                            continue;
                        }
                    }
                }

                // Check if there's work to do
                let item = {
                    let mut queue = queue.lock().unwrap();
                    queue.pop_front()
                };

                if let Some(upload_item) = item {
                    let results_clone = results.clone();
                    let rate_limit_clone = rate_limit_until_clone.clone();
                    let email = {
                        let email_lock = email_clone.lock().unwrap();
                        email_lock.clone()
                    };
                    let name = {
                        let name_lock = name_clone.lock().unwrap();
                        name_lock.clone()
                    };
                    let tmp_dir = {
                        let tmp_dir_lock = tmp_dir_clone.lock().unwrap();
                        tmp_dir_lock.clone()
                    };

                    if email.is_empty() {
                        log::warn!(
                            "APKMirror upload email not set, skipping upload for: {}",
                            upload_item.package_id
                        );
                        let mut results = results_clone.lock().unwrap();
                        results.insert(
                            upload_item.package_id,
                            ApkMirrorUploadStatus::Error("Email not configured".to_string()),
                        );
                        continue;
                    }

                    let was_rate_limited = process_upload_item(
                        &upload_item,
                        &email,
                        &name,
                        &tmp_dir,
                        &results_clone,
                        &rate_limit_clone,
                    );

                    if was_rate_limited {
                        // Re-queue the item at the front so it will be retried
                        // after the rate limit expires
                        let mut queue = queue.lock().unwrap();
                        queue.push_front(upload_item);
                        // Mark all pending items in the queue as rate limited too
                        let mut results = results_clone.lock().unwrap();
                        for queued_item in queue.iter() {
                            if let Some(status) = results.get(&queued_item.package_id) {
                                if matches!(status, ApkMirrorUploadStatus::Pending) {
                                    results.insert(
                                        queued_item.package_id.clone(),
                                        ApkMirrorUploadStatus::RateLimited,
                                    );
                                }
                            }
                        }
                        continue;
                    }

                    // Rate limiting: wait between uploads
                    thread::sleep(Duration::from_secs(10));
                } else {
                    // No work, sleep a bit
                    thread::sleep(Duration::from_millis(500));
                }
            }

            log::info!("APKMirror upload worker thread stopped");
        });
    }

    /// Stop the background upload worker thread
    pub fn stop_worker(&self) {
        let mut is_running = self.is_running.lock().unwrap();
        *is_running = false;
        log::info!("APKMirror upload worker stopping...");
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        let queue = self.queue.lock().unwrap();
        queue.len()
    }

    /// Get number of successful uploads
    pub fn success_count(&self) -> usize {
        let results = self.results.lock().unwrap();
        results
            .values()
            .filter(|status| matches!(status, ApkMirrorUploadStatus::Success(_)))
            .count()
    }

    /// Clear all results (for UI refresh)
    pub fn clear_results(&self) {
        let mut results = self.results.lock().unwrap();
        results.clear();
    }
}

impl Default for ApkMirrorUploadQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Find APK files in a directory on the device
fn find_apk_files_in_directory(device_serial: &str, dir_path: &str) -> Vec<String> {
    use std::process::Command;

    let find_output = Command::new("adb")
        .arg("-s")
        .arg(device_serial)
        .arg("shell")
        .arg("find")
        .arg(dir_path)
        .arg("-maxdepth")
        .arg("1")
        .arg("-name")
        .arg("*.apk")
        .arg("-type")
        .arg("f")
        .output();

    match find_output {
        Ok(output) if output.status.success() => {
            let files_text = String::from_utf8_lossy(&output.stdout).to_string();
            files_text
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|path| !path.is_empty())
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Process a single upload item
/// Returns true if rate limited (caller should stop processing)
fn process_upload_item(
    item: &ApkMirrorUploadItem,
    email: &str,
    name: &str,
    tmp_dir: &str,
    results: &Arc<Mutex<HashMap<String, ApkMirrorUploadStatus>>>,
    rate_limit_until: &Arc<Mutex<Option<std::time::Instant>>>,
) -> bool {
    let pkg_id = &item.package_id;

    log::info!("Processing APKMirror upload for: {}", pkg_id);

    // Step 1: Determine the actual APK file path
    // The apk_path from PackageFingerprint.codePath is typically a directory path like
    // /data/app/~~xxx==/com.example.app-yyy==/ so we need to find APK files within it
    let device_apk_path = if item.apk_path.ends_with(".apk") {
        item.apk_path.clone()
    } else {
        let apk_files = find_apk_files_in_directory(&item.device_serial, &item.apk_path);
        if apk_files.is_empty() {
            let error_msg = format!("No APK files found in directory: {}", item.apk_path);
            log::error!("{}", error_msg);
            let mut results = results.lock().unwrap();
            results.insert(pkg_id.clone(), ApkMirrorUploadStatus::Error(error_msg));
            return false;
        }
        // Prefer .apk if it exists, otherwise use the first APK found
        apk_files
            .iter()
            .find(|p| p.ends_with(".apk"))
            .cloned()
            .unwrap_or_else(|| apk_files[0].clone())
    };

    log::info!("Using APK path for {}: {}", pkg_id, device_apk_path);

    // Step 2: Pull APK from device
    {
        let mut results = results.lock().unwrap();
        results.insert(pkg_id.clone(), ApkMirrorUploadStatus::PullingApk);
    }

    let local_apk_path = format!("{}/{}.apk", tmp_dir, pkg_id.replace('.', "_"));

    let pull_result =
        crate::adb::pull_file_to_temp(&item.device_serial, &device_apk_path, tmp_dir, pkg_id);

    #[cfg(target_os = "android")]
    let pull_result: std::io::Result<String> = {
        // On Android, we can copy the file directly if we have access
        match std::fs::copy(&device_apk_path, &local_apk_path) {
            Ok(_) => Ok(local_apk_path.clone()),
            Err(e) => Err(e),
        }
    };

    let local_path = match pull_result {
        Ok(path) => path,
        Err(e) => {
            let error_msg = format!("Failed to pull APK: {}", e);
            log::error!("{}", error_msg);
            let mut results = results.lock().unwrap();
            results.insert(pkg_id.clone(), ApkMirrorUploadStatus::Error(error_msg));
            return false;
        }
    };

    // Verify the pulled file is actually a file and not a directory
    let local_path_meta = std::fs::metadata(&local_path);
    let final_local_path = match local_path_meta {
        Ok(meta) if meta.is_file() => local_path,
        Ok(meta) if meta.is_dir() => {
            // adb pull created a directory, look for the APK inside
            let apk_found = std::fs::read_dir(&local_path)
                .ok()
                .and_then(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .find(|e| {
                            e.path()
                                .extension()
                                .map_or(false, |ext| ext.eq_ignore_ascii_case("apk"))
                                && e.path().is_file()
                        })
                        .map(|e| e.path().to_string_lossy().to_string())
                });
            if let Some(inner_path) = apk_found {
                log::info!("Found APK inside pulled directory: {}", inner_path);
                inner_path
            } else {
                let error_msg = format!(
                    "Pulled path is a directory but no APK found inside: {}",
                    local_path
                );
                log::error!("{}", error_msg);
                let mut results = results.lock().unwrap();
                results.insert(pkg_id.clone(), ApkMirrorUploadStatus::Error(error_msg));
                let _ = std::fs::remove_dir_all(&local_path);
                return false;
            }
        }
        _ => {
            let error_msg = format!(
                "Pulled file does not exist or is inaccessible: {}",
                local_path
            );
            log::error!("{}", error_msg);
            let mut results = results.lock().unwrap();
            results.insert(pkg_id.clone(), ApkMirrorUploadStatus::Error(error_msg));
            return false;
        }
    };

    // Step 3: Compute MD5 hash
    {
        let mut results = results.lock().unwrap();
        results.insert(pkg_id.clone(), ApkMirrorUploadStatus::ComputingHash);
    }

    let md5_hash = match compute_md5_hash(&final_local_path) {
        Ok(hash) => hash,
        Err(e) => {
            let error_msg = format!("Failed to compute MD5 hash: {}", e);
            log::error!("{}", error_msg);
            let mut results = results.lock().unwrap();
            results.insert(pkg_id.clone(), ApkMirrorUploadStatus::Error(error_msg));
            // Clean up temp file/directory
            let _ = std::fs::remove_file(&final_local_path);
            let _ = std::fs::remove_dir_all(&local_apk_path);
            return false;
        }
    };

    log::info!("Computed MD5 hash for {}: {}", pkg_id, md5_hash);

    // Step 4: Check if APK is uploadable
    {
        let mut results = results.lock().unwrap();
        results.insert(pkg_id.clone(), ApkMirrorUploadStatus::CheckingUploadable);
    }

    let is_uploadable = match check_apk_uploadable(&md5_hash, email) {
        Ok(uploadable) => uploadable,
        Err(e) => {
            let error_msg = format!("Failed to check uploadability: {}", e);
            log::error!("{}", error_msg);
            let mut results = results.lock().unwrap();
            results.insert(pkg_id.clone(), ApkMirrorUploadStatus::Error(error_msg));
            // Clean up temp file/directory
            let _ = std::fs::remove_file(&final_local_path);
            let _ = std::fs::remove_dir_all(&local_apk_path);
            return false;
        }
    };

    if !is_uploadable {
        log::info!(
            "APK {} already exists on APKMirror (MD5: {})",
            pkg_id,
            md5_hash
        );
        let mut results = results.lock().unwrap();
        results.insert(pkg_id.clone(), ApkMirrorUploadStatus::AlreadyExists);
        // Clean up temp file/directory
        let _ = std::fs::remove_file(&final_local_path);
        let _ = std::fs::remove_dir_all(&local_apk_path);
        return false;
    }

    // Step 5: Upload APK
    {
        let mut results = results.lock().unwrap();
        results.insert(pkg_id.clone(), ApkMirrorUploadStatus::Uploading);
    }

    let upload_name = if name.is_empty() { "Anonymous" } else { name };

    let is_rate_limited = match upload_apk(&final_local_path, upload_name, email) {
        Ok(result) => {
            if result.success {
                log::info!("Successfully uploaded {} to APKMirror", pkg_id);
                let mut results = results.lock().unwrap();
                results.insert(
                    pkg_id.clone(),
                    ApkMirrorUploadStatus::Success(result.message),
                );
                false
            } else if result.rate_limited {
                log::warn!(
                    "APKMirror rate limit reached for {}. Pausing uploads for 24 hours.",
                    pkg_id
                );
                // Set rate limit expiry to 24 hours from now
                {
                    let mut rate_limit = rate_limit_until.lock().unwrap();
                    *rate_limit =
                        Some(std::time::Instant::now() + Duration::from_secs(24 * 60 * 60));
                }
                let mut results = results.lock().unwrap();
                results.insert(pkg_id.clone(), ApkMirrorUploadStatus::RateLimited);
                true
            } else if result.already_exists {
                log::info!("APK {} already exists on APKMirror", pkg_id);
                let mut results = results.lock().unwrap();
                results.insert(pkg_id.clone(), ApkMirrorUploadStatus::AlreadyExists);
                false
            } else {
                let error_msg = format!("Upload failed: {}", result.message);
                log::error!("{}", error_msg);
                let mut results = results.lock().unwrap();
                results.insert(pkg_id.clone(), ApkMirrorUploadStatus::Error(error_msg));
                false
            }
        }
        Err(e) => {
            let error_msg = format!("Upload error: {}", e);
            log::error!("{}", error_msg);
            let mut results = results.lock().unwrap();
            results.insert(pkg_id.clone(), ApkMirrorUploadStatus::Error(error_msg));
            false
        }
    };

    // Clean up temp file/directory
    let _ = std::fs::remove_file(&final_local_path);
    let _ = std::fs::remove_dir_all(&local_apk_path);

    is_rate_limited
}

/// Compute MD5 hash of a file
fn compute_md5_hash(file_path: &str) -> Result<String> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(file_path)?;
    let mut hasher = md5::Context::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.consume(&buffer[..bytes_read]);
    }

    let digest = hasher.compute();
    Ok(format!("{:x}", digest))
}

/// Compare version strings to determine if device version is newer than APKMirror version.
/// Returns true if device_version is newer than apkmirror_version.
/// If apkmirror_version is None (not found on APKMirror), always returns true.
pub fn is_version_newer(device_version: &str, apkmirror_version: Option<&str>) -> bool {
    let apkmirror_ver = match apkmirror_version {
        Some(v) if !v.is_empty() => v,
        _ => return true, // If APKMirror doesn't have version info, consider device version as newer
    };

    // Parse versions into comparable components
    let device_parts = parse_version(device_version);
    let apkmirror_parts = parse_version(apkmirror_ver);

    // Compare each component
    for (d, a) in device_parts.iter().zip(apkmirror_parts.iter()) {
        match d.cmp(a) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => continue,
        }
    }

    // If device has more components, it might be newer (e.g., 1.2.3 vs 1.2)
    device_parts.len() > apkmirror_parts.len()
}

/// Parse version string into numeric components.
/// Handles versions like "1.2.3", "1.2.3-beta", "1.2.3.456", etc.
fn parse_version(version: &str) -> Vec<i64> {
    version
        .split(|c: char| c == '.' || c == '-' || c == '_' || c == ' ')
        .filter_map(|part| {
            // Extract leading numeric part from each component
            let numeric: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            if numeric.is_empty() {
                None
            } else {
                numeric.parse::<i64>().ok()
            }
        })
        .collect()
}

#[cfg(test)]
mod upload_tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        // Device newer
        assert!(is_version_newer("1.2.4", Some("1.2.3")));
        assert!(is_version_newer("2.0.0", Some("1.9.9")));
        assert!(is_version_newer("1.10.0", Some("1.9.0")));

        // Device older or same
        assert!(!is_version_newer("1.2.3", Some("1.2.4")));
        assert!(!is_version_newer("1.2.3", Some("1.2.3")));
        assert!(!is_version_newer("1.0.0", Some("2.0.0")));

        // APKMirror version None or empty
        assert!(is_version_newer("1.0.0", None));
        assert!(is_version_newer("1.0.0", Some("")));

        // Complex versions
        assert!(is_version_newer("1.2.3.4", Some("1.2.3")));
        assert!(is_version_newer("1.2.3-beta2", Some("1.2.2")));
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_version("1.2.3-beta"), vec![1, 2, 3]);
        assert_eq!(parse_version("1.2.3.456"), vec![1, 2, 3, 456]);
        assert_eq!(parse_version("25.47.63"), vec![25, 47, 63]);
    }
}

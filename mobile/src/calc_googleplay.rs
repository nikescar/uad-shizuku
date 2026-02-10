use crate::api_googleplay::{fetch_app_details, GooglePlayAppInfo};
pub use crate::calc_googleplay_stt::*;
use crate::db_googleplay::{get_google_play_app, is_cache_stale, upsert_google_play_app};
use crate::is_valid_package_id;
use crate::models::GooglePlayApp;
use anyhow::Result;
use diesel::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

impl GooglePlayQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            results: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Add a package ID to the fetch queue
    pub fn enqueue(&self, package_id: String) {
        // Skip package IDs with less than 2 domain levels (e.g., com.android)
        if !is_valid_package_id(&package_id) {
            // Mark as error so it won't be re-queued repeatedly
            let mut results = self.results.lock().unwrap();
            if !results.contains_key(&package_id) {
                log::debug!(
                    "Skipping Google Play fetch for invalid package ID: {}",
                    package_id
                );
                results.insert(
                    package_id,
                    FetchStatus::Error("Invalid package ID".to_string()),
                );
            }
            return;
        }

        let mut queue = self.queue.lock().unwrap();
        let mut results = self.results.lock().unwrap();

        // Don't add if already in queue or being processed
        if !queue.contains(&package_id) && !results.contains_key(&package_id) {
            queue.push_back(package_id.clone());
            results.insert(package_id, FetchStatus::Pending);
        }
    }

    /// Add multiple package IDs to the fetch queue
    pub fn enqueue_batch(&self, package_ids: Vec<String>) {
        for package_id in package_ids {
            self.enqueue(package_id);
        }
    }

    /// Get the status of a package fetch
    pub fn get_status(&self, package_id: &str) -> Option<FetchStatus> {
        let results = self.results.lock().unwrap();
        results.get(package_id).cloned()
    }

    /// Get result if successfully fetched
    pub fn get_result(&self, package_id: &str) -> Option<GooglePlayApp> {
        let results = self.results.lock().unwrap();
        if let Some(FetchStatus::Success(app)) = results.get(package_id) {
            Some(app.clone())
        } else {
            None
        }
    }

    /// Start the background worker thread
    pub fn start_worker(&self, _db_path: String) {
        let mut is_running = self.is_running.lock().unwrap();

        if *is_running {
            log::warn!("Google Play worker already running");
            return;
        }

        *is_running = true;
        drop(is_running);

        let queue = self.queue.clone();
        let results = self.results.clone();
        let is_running_clone = self.is_running.clone();

        thread::spawn(move || {
            // Small delay to let the main thread's initial pre-fetch complete
            thread::sleep(Duration::from_millis(500));

            log::info!("Google Play worker thread started");

            while *is_running_clone.lock().unwrap() {
                // Check if there's work to do
                let package_id = {
                    let mut queue = queue.lock().unwrap();
                    queue.pop_front()
                };

                if let Some(pkg_id) = package_id {
                    // Update status to fetching
                    {
                        let mut results = results.lock().unwrap();
                        results.insert(pkg_id.clone(), FetchStatus::Fetching);
                    }

                    log::info!("Processing Google Play fetch for: {}", pkg_id);

                    // Establish database connection
                    let mut conn = match crate::db::establish_connection() {
                        conn => conn,
                    };

                    // Check cache first
                    match get_google_play_app(&mut conn, &pkg_id) {
                        Ok(Some(cached_app)) if !is_cache_stale(&cached_app) => {
                            if cached_app.raw_response == "404" {
                                log::info!("Using cached Google Play 404 for: {}", pkg_id);
                                let mut results = results.lock().unwrap();
                                results.insert(
                                    pkg_id,
                                    FetchStatus::Error("App not found (cached)".to_string()),
                                );
                                continue;
                            }
                            log::info!("Using cached Google Play data for: {}", pkg_id);
                            let mut results = results.lock().unwrap();
                            results.insert(pkg_id, FetchStatus::Success(cached_app));
                            continue;
                        }
                        _ => {}
                    }

                    // Fetch from Google Play
                    match fetch_app_details(&pkg_id) {
                        Ok(app_info) => {
                            // Save to database
                            match save_to_db(&mut conn, &app_info) {
                                Ok(saved_app) => {
                                    log::info!("Successfully fetched and saved: {}", pkg_id);
                                    let mut results = results.lock().unwrap();
                                    results.insert(pkg_id, FetchStatus::Success(saved_app));
                                }
                                Err(e) => {
                                    let error_msg = format!("Database save error: {}", e);
                                    log::error!("{}", error_msg);
                                    let mut results = results.lock().unwrap();
                                    results.insert(pkg_id, FetchStatus::Error(error_msg));
                                }
                            }
                        }
                        Err(e) => {
                            // Check if it's a 404 error
                            let is_404 = if let Some(ureq_err) = e.downcast_ref::<ureq::Error>() {
                                matches!(ureq_err, ureq::Error::Status(404, _))
                            } else {
                                false
                            };

                            if is_404 {
                                log::info!(
                                    "Google Play returned 404 for {}, caching as not found",
                                    pkg_id
                                );
                                // Save "Not Found" to database
                                let not_found_app = crate::api_googleplay::GooglePlayAppInfo {
                                    package_id: pkg_id.clone(),
                                    title: "Not Found".to_string(),
                                    developer: "Unknown".to_string(),
                                    version: None,
                                    icon_base64: None,
                                    score: None,
                                    installs: None,
                                    updated: None,
                                    raw_response: "404".to_string(),
                                };

                                if let Ok(_) = save_to_db(&mut conn, &not_found_app) {
                                    log::info!("Cached 404 for {}", pkg_id);
                                }

                                let mut results = results.lock().unwrap();
                                results.insert(
                                    pkg_id,
                                    FetchStatus::Error("App not found".to_string()),
                                );
                            } else {
                                let error_msg = format!("Fetch error: {:?}", e);
                                log::warn!("{}", error_msg);
                                let mut results = results.lock().unwrap();
                                results.insert(pkg_id, FetchStatus::Error(error_msg));
                            }
                        }
                    }

                    // Rate limiting: wait between requests
                    thread::sleep(Duration::from_secs(2));
                } else {
                    // No work, sleep a bit
                    thread::sleep(Duration::from_millis(500));
                }
            }

            log::info!("Google Play worker thread stopped");
        });
    }

    /// Stop the background worker thread
    pub fn stop_worker(&self) {
        let mut is_running = self.is_running.lock().unwrap();
        *is_running = false;
        log::info!("Google Play worker stopping...");
    }

    /// Clear all pending items from queue
    pub fn clear_queue(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.clear();
        log::info!("Google Play queue cleared");
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
            .filter(|status| matches!(status, FetchStatus::Success(_)))
            .count()
    }
}

/// Save Google Play app info to database
fn save_to_db(conn: &mut SqliteConnection, app_info: &GooglePlayAppInfo) -> Result<GooglePlayApp> {
    upsert_google_play_app(
        conn,
        &app_info.package_id,
        &app_info.title,
        &app_info.developer,
        app_info.version.as_deref(),
        app_info.icon_base64.as_deref(),
        app_info.score,
        app_info.installs.as_deref(),
        app_info.updated,
        &app_info.raw_response,
    )
}

impl Default for GooglePlayQueue {
    fn default() -> Self {
        Self::new()
    }
}

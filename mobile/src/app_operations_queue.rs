pub use crate::app_operations_queue_stt::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

impl AppOperationsQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            results: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(Mutex::new(false)),
            progress: Arc::new(Mutex::new(None)),
            cancelled: Arc::new(Mutex::new(false)),
        }
    }

    /// Add an operation to the queue
    pub fn enqueue(&self, operation: OperationType) {
        let key = match &operation {
            OperationType::Install { app_name, .. } => app_name.clone(),
            OperationType::Uninstall { package_name, .. } => package_name.clone(),
        };

        let mut queue = self.queue.lock().unwrap();
        let mut results = self.results.lock().unwrap();

        // Don't add if already in queue or being processed
        if !results.contains_key(&key) {
            let item = OperationItem {
                operation: operation.clone(),
                status: OperationStatus::Pending,
            };
            queue.push_back(item);
            results.insert(key, OperationStatus::Pending);
        }
    }

    /// Add multiple operations to the queue
    pub fn enqueue_batch(&self, operations: Vec<OperationType>) {
        for operation in operations {
            self.enqueue(operation);
        }
    }

    /// Get the status of an operation
    pub fn get_status(&self, key: &str) -> Option<OperationStatus> {
        let results = self.results.lock().unwrap();
        results.get(key).cloned()
    }

    /// Get number of pending operations
    pub fn queue_size(&self) -> usize {
        let queue = self.queue.lock().unwrap();
        queue.len()
    }

    /// Get number of completed operations
    pub fn completed_count(&self) -> usize {
        let results = self.results.lock().unwrap();
        results
            .values()
            .filter(|status| matches!(status, OperationStatus::Success(_) | OperationStatus::Error(_)))
            .count()
    }

    /// Clear the queue
    pub fn clear_queue(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.clear();
        if let Ok(mut cancelled) = self.cancelled.lock() {
            *cancelled = true;
        }
        log::info!("App operations queue cleared");
    }

    /// Clear completed results (call after operations are done and refresh is complete)
    pub fn clear_results(&self) {
        let mut results = self.results.lock().unwrap();
        results.clear();
        log::info!("App operations results cleared");
    }

    /// Start the background worker thread
    #[cfg(not(target_os = "android"))]
    pub fn start_worker(&self, device: String, _cache_dir: std::path::PathBuf, tmp_dir: std::path::PathBuf) {
        let mut is_running = self.is_running.lock().unwrap();

        if *is_running {
            log::warn!("App operations worker already running");
            return;
        }

        *is_running = true;
        drop(is_running);

        // Reset progress and cancelled flag
        if let Ok(mut p) = self.progress.lock() {
            *p = Some(0.0);
        }
        if let Ok(mut c) = self.cancelled.lock() {
            *c = false;
        }

        let queue = self.queue.clone();
        let results = self.results.clone();
        let is_running_clone = self.is_running.clone();
        let progress_clone = self.progress.clone();
        let cancelled_clone = self.cancelled.clone();

        thread::spawn(move || {
            log::info!("App operations worker thread started");

            loop {
                // Check if cancelled
                if let Ok(cancelled) = cancelled_clone.lock() {
                    if *cancelled {
                        log::info!("App operations cancelled by user");
                        break;
                    }
                }

                // Get total count for progress calculation
                let total_count = {
                    let results = results.lock().unwrap();
                    results.len()
                };

                // Check if there's work to do
                let operation_item = {
                    let mut queue = queue.lock().unwrap();
                    queue.pop_front()
                };

                if let Some(item) = operation_item {
                    let key = match &item.operation {
                        OperationType::Install { app_name, .. } => app_name.clone(),
                        OperationType::Uninstall { package_name, .. } => package_name.clone(),
                    };

                    // Update status to processing
                    {
                        let mut results = results.lock().unwrap();
                        results.insert(key.clone(), OperationStatus::Processing);
                    }

                    // Process the operation
                    let result = match &item.operation {
                        OperationType::Install { app_name, download_url, link_type } => {
                            log::info!("Processing install for: {}", app_name);
                            Self::process_install(
                                app_name,
                                download_url,
                                link_type,
                                &device,
                                &tmp_dir,
                            )
                        }
                        OperationType::Uninstall { package_name, is_system } => {
                            log::info!("Processing uninstall for: {}", package_name);
                            Self::process_uninstall(package_name, *is_system, &device)
                        }
                    };

                    // Update results
                    {
                        let mut results = results.lock().unwrap();
                        results.insert(key, result);
                    }

                    // Update progress
                    if total_count > 0 {
                        let completed = {
                            let results = results.lock().unwrap();
                            results
                                .values()
                                .filter(|status| matches!(status, OperationStatus::Success(_) | OperationStatus::Error(_)))
                                .count()
                        };
                        let progress_value = completed as f32 / total_count as f32;
                        if let Ok(mut p) = progress_clone.lock() {
                            *p = Some(progress_value);
                        }
                    }

                    // Small delay between operations
                    thread::sleep(Duration::from_millis(100));
                } else {
                    // No more work, exit
                    break;
                }
            }

            // Clear progress when done
            if let Ok(mut p) = progress_clone.lock() {
                *p = None;
            }

            let mut is_running = is_running_clone.lock().unwrap();
            *is_running = false;
            log::info!("App operations worker thread stopped");
        });
    }

    #[cfg(not(target_os = "android"))]
    fn process_install(
        app_name: &str,
        download_url: &str,
        _link_type: &str,
        device: &str,
        tmp_dir: &std::path::Path,
    ) -> OperationStatus {
        use crate::adb;

        // Generate a safe filename from app name
        let safe_name: String = app_name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        let safe_name = if safe_name.is_empty() {
            "app".to_string()
        } else {
            safe_name
        };

        // Download the APK
        let apk_path = match Self::download_apk(download_url, &safe_name, tmp_dir) {
            Ok(path) => path,
            Err(e) => {
                return OperationStatus::Error(format!("Failed to download APK: {}", e));
            }
        };

        log::info!("APK downloaded to: {:?}", apk_path);

        // Install the APK using adb
        let apk_path_str = apk_path.to_string_lossy().to_string();
        let result = match adb::install_apk(&apk_path_str, device) {
            Ok(result) => {
                log::info!("APK installed successfully: {}", result);

                // Clean up the downloaded APK
                if let Err(e) = std::fs::remove_file(&apk_path) {
                    log::warn!("Failed to clean up APK file: {}", e);
                }

                OperationStatus::Success(format!("Installed: {}", app_name))
            }
            Err(e) => {
                // Clean up the downloaded APK even on failure
                if let Err(e2) = std::fs::remove_file(&apk_path) {
                    log::warn!("Failed to clean up APK file: {}", e2);
                }

                OperationStatus::Error(format!("Failed to install: {}", e))
            }
        };

        result
    }

    #[cfg(not(target_os = "android"))]
    fn download_apk(
        url: &str,
        app_name: &str,
        tmp_dir: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        use std::io::Write;

        let apk_filename = format!("{}.apk", app_name);
        let apk_path = tmp_dir.join(&apk_filename);

        // Download the file
        let response = ureq::get(url)
            .timeout(std::time::Duration::from_secs(300))
            .call()
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let mut file =
            std::fs::File::create(&apk_path).map_err(|e| format!("Failed to create file: {}", e))?;

        std::io::copy(&mut response.into_reader(), &mut file)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        file.flush()
            .map_err(|e| format!("Failed to flush file: {}", e))?;

        Ok(apk_path)
    }

    #[cfg(not(target_os = "android"))]
    fn process_uninstall(
        package_name: &str,
        is_system: bool,
        device: &str,
    ) -> OperationStatus {
        use crate::adb;

        let result = if is_system {
            adb::uninstall_app_user(package_name, device, None)
        } else {
            adb::uninstall_app(package_name, device)
        };

        match result {
            Ok(output) => {
                log::info!("App uninstalled successfully: {}", output);
                OperationStatus::Success(format!("Uninstalled: {}", package_name))
            }
            Err(e) => {
                log::error!("Failed to uninstall app({}): {}", package_name, e);
                OperationStatus::Error(format!("Failed to uninstall: {}", e))
            }
        }
    }

    #[cfg(target_os = "android")]
    pub fn start_worker(&self, _device: String, _cache_dir: std::path::PathBuf, _tmp_dir: std::path::PathBuf) {
        log::warn!("App operations worker not supported on Android");
    }
}

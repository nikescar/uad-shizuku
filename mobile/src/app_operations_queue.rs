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
                                cancelled_clone.clone(),
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
        cancelled: Arc<Mutex<bool>>,
    ) -> OperationStatus {
        use crate::adb;

        log::info!("Starting installation for: {} from {}", app_name, download_url);

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

        log::info!("Downloading APK for: {}", app_name);

        // Download the APK
        let apk_path = match Self::download_apk(download_url, &safe_name, tmp_dir, cancelled) {
            Ok(path) => {
                log::info!("APK downloaded successfully to: {:?}", path);
                path
            }
            Err(e) => {
                log::error!("Failed to download APK for {}: {}", app_name, e);
                return OperationStatus::Error(format!("Download failed: {}", e));
            }
        };

        // Install the APK using adb
        log::info!("Installing APK via ADB: {:?}", apk_path);
        let apk_path_str = apk_path.to_string_lossy().to_string();
        let result = match adb::install_apk(&apk_path_str, device) {
            Ok(output) => {
                log::info!("APK installed successfully: {}", output);

                // Clean up the downloaded APK
                if let Err(e) = std::fs::remove_file(&apk_path) {
                    log::warn!("Failed to clean up APK file: {}", e);
                } else {
                    log::info!("Cleaned up APK file: {:?}", apk_path);
                }

                OperationStatus::Success(format!("Installed: {}", app_name))
            }
            Err(e) => {
                log::error!("Failed to install APK for {}: {}", app_name, e);

                // Clean up the downloaded APK even on failure
                if let Err(e2) = std::fs::remove_file(&apk_path) {
                    log::warn!("Failed to clean up APK file: {}", e2);
                } else {
                    log::info!("Cleaned up APK file after failed install: {:?}", apk_path);
                }

                OperationStatus::Error(format!("Install failed: {}", e))
            }
        };

        result
    }

    #[cfg(not(target_os = "android"))]
    fn download_apk(
        url: &str,
        app_name: &str,
        tmp_dir: &std::path::Path,
        cancelled: Arc<Mutex<bool>>,
    ) -> Result<std::path::PathBuf, String> {
        use std::io::{BufWriter, Read, Write};

        let apk_filename = format!("{}.apk", app_name);
        let apk_path = tmp_dir.join(&apk_filename);
        let tmp_path = tmp_dir.join(format!("{}.tmp", app_name));

        log::info!("Downloading from URL: {}", url);
        log::info!("Target path: {:?}", apk_path);

        // Check if cancelled before starting
        if let Ok(c) = cancelled.lock() {
            if *c {
                log::info!("Download cancelled before start");
                return Err("Download cancelled".to_string());
            }
        }

        // Download the file
        log::info!("Sending HTTP request...");
        let response = ureq::get(url)
            .timeout(std::time::Duration::from_secs(300))
            .call()
            .map_err(|e| {
                let err_msg = format!("HTTP request failed: {}", e);
                log::error!("{}", err_msg);
                err_msg
            })?;

        // Get content length for progress tracking
        let content_length = response.header("Content-Length")
            .and_then(|s| s.parse::<u64>().ok());

        if let Some(size) = content_length {
            log::info!("Content-Length: {} bytes ({:.2} MB)", size, size as f64 / 1_048_576.0);
        } else {
            log::info!("Content-Length: unknown");
        }

        // Write to temp file first (atomic write pattern)
        let file = std::fs::File::create(&tmp_path).map_err(|e| {
            let err_msg = format!("Failed to create temp file: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;

        // Use buffered writer for better performance (128KB buffer)
        let mut writer = BufWriter::with_capacity(131072, file);
        let mut reader = response.into_reader();

        // Download in chunks with progress logging
        let mut buffer = vec![0u8; 131072]; // 128KB chunks
        let mut total_written = 0u64;
        let mut last_log_at = 0u64;
        let log_interval = 5_242_880; // Log every 5 MB (reduce logging overhead)

        loop {
            // Check if download was cancelled
            if let Ok(c) = cancelled.lock() {
                if *c {
                    log::info!("Download cancelled by user");
                    // Clean up temp file
                    let _ = std::fs::remove_file(&tmp_path);
                    return Err("Download cancelled by user".to_string());
                }
            }

            let bytes_read = reader.read(&mut buffer).map_err(|e| {
                let err_msg = format!("Failed to read from response: {}", e);
                log::error!("{}", err_msg);
                // Clean up temp file on error
                let _ = std::fs::remove_file(&tmp_path);
                err_msg
            })?;

            if bytes_read == 0 {
                break; // EOF
            }

            writer.write_all(&buffer[..bytes_read]).map_err(|e| {
                let err_msg = format!("Failed to write to file: {}", e);
                log::error!("{}", err_msg);
                // Clean up temp file on error
                let _ = std::fs::remove_file(&tmp_path);
                err_msg
            })?;

            total_written += bytes_read as u64;

            // Log progress every 5 MB to reduce overhead
            if total_written - last_log_at >= log_interval {
                if let Some(total) = content_length {
                    let progress = (total_written as f64 / total as f64) * 100.0;
                    log::info!("Downloaded: {:.1} MB / {:.1} MB ({:.0}%)",
                        total_written as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0,
                        progress);
                } else {
                    log::info!("Downloaded: {:.1} MB", total_written as f64 / 1_048_576.0);
                }
                last_log_at = total_written;
            }
        }

        // Flush buffered writer
        writer.flush().map_err(|e| {
            let err_msg = format!("Failed to flush file: {}", e);
            log::error!("{}", err_msg);
            // Clean up temp file on error
            let _ = std::fs::remove_file(&tmp_path);
            err_msg
        })?;

        // Explicitly drop writer to close file
        drop(writer);

        log::info!("Download complete: {:.2} MB ({} bytes)",
            total_written as f64 / 1_048_576.0, total_written);

        // Verify file size if Content-Length was provided
        if let Some(expected) = content_length {
            if total_written != expected {
                let err_msg = format!(
                    "Size mismatch: downloaded {} bytes, expected {} bytes",
                    total_written, expected
                );
                log::error!("{}", err_msg);
                // Clean up temp file
                let _ = std::fs::remove_file(&tmp_path);
                return Err(err_msg);
            }
        }

        // Atomic rename from temp to final path
        std::fs::rename(&tmp_path, &apk_path).map_err(|e| {
            let err_msg = format!("Failed to rename temp file: {}", e);
            log::error!("{}", err_msg);
            // Clean up temp file on error
            let _ = std::fs::remove_file(&tmp_path);
            err_msg
        })?;

        log::info!("APK saved: {:?}", apk_path);

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
    pub fn start_worker(&self, _device: String, cache_dir: std::path::PathBuf, _tmp_dir: std::path::PathBuf) {
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
            log::info!("App operations worker thread started (Android/Shizuku)");

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
                            Self::process_install_android(
                                app_name,
                                download_url,
                                link_type,
                                &cache_dir,
                                cancelled_clone.clone(),
                            )
                        }
                        OperationType::Uninstall { package_name, is_system } => {
                            log::info!("Processing uninstall for: {}", package_name);
                            Self::process_uninstall_android(package_name, *is_system)
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

    #[cfg(target_os = "android")]
    fn process_install_android(
        app_name: &str,
        download_url: &str,
        _link_type: &str,
        cache_dir: &std::path::Path,
        cancelled: Arc<Mutex<bool>>,
    ) -> OperationStatus {
        use crate::android_shizuku;

        log::info!("Starting installation for: {} from {}", app_name, download_url);

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

        log::info!("Downloading APK for: {}", app_name);

        // Download the APK
        let apk_path = match Self::download_apk(download_url, &safe_name, cache_dir, cancelled) {
            Ok(path) => {
                log::info!("APK downloaded successfully to: {:?}", path);
                path
            }
            Err(e) => {
                log::error!("Failed to download APK for {}: {}", app_name, e);
                return OperationStatus::Error(format!("Download failed: {}", e));
            }
        };

        // Install the APK using Shizuku
        log::info!("Installing APK via Shizuku: {:?}", apk_path);
        let apk_path_str = apk_path.to_string_lossy().to_string();
        let result = match android_shizuku::shizuku_install_apk(&apk_path_str) {
            Ok(status) => {
                log::info!("APK installed successfully via Shizuku (status: {})", status);

                // Clean up the downloaded APK
                if let Err(e) = std::fs::remove_file(&apk_path) {
                    log::warn!("Failed to clean up APK file: {}", e);
                } else {
                    log::info!("Cleaned up APK file: {:?}", apk_path);
                }

                OperationStatus::Success(format!("Installed: {}", app_name))
            }
            Err(e) => {
                log::error!("Failed to install APK for {}: {}", app_name, e);

                // Clean up the downloaded APK even on failure
                if let Err(e2) = std::fs::remove_file(&apk_path) {
                    log::warn!("Failed to clean up APK file: {}", e2);
                } else {
                    log::info!("Cleaned up APK file after failed install: {:?}", apk_path);
                }

                OperationStatus::Error(format!("Install failed: {}", e))
            }
        };

        result
    }

    #[cfg(target_os = "android")]
    fn download_apk(
        url: &str,
        app_name: &str,
        cache_dir: &std::path::Path,
        cancelled: Arc<Mutex<bool>>,
    ) -> Result<std::path::PathBuf, String> {
        use std::io::{BufWriter, Read, Write};

        let apk_filename = format!("{}.apk", app_name);
        let apk_path = cache_dir.join(&apk_filename);
        let tmp_path = cache_dir.join(format!("{}.tmp", app_name));

        log::info!("Downloading from URL: {}", url);
        log::info!("Target path: {:?}", apk_path);

        // Check if cancelled before starting
        if let Ok(c) = cancelled.lock() {
            if *c {
                log::info!("Download cancelled before start");
                return Err("Download cancelled".to_string());
            }
        }

        // Download the file
        log::info!("Sending HTTP request...");
        let response = ureq::get(url)
            .timeout(std::time::Duration::from_secs(300))
            .call()
            .map_err(|e| {
                let err_msg = format!("HTTP request failed: {}", e);
                log::error!("{}", err_msg);
                err_msg
            })?;

        // Get content length for progress tracking
        let content_length = response.header("Content-Length")
            .and_then(|s| s.parse::<u64>().ok());

        if let Some(size) = content_length {
            log::info!("Content-Length: {} bytes ({:.2} MB)", size, size as f64 / 1_048_576.0);
        } else {
            log::info!("Content-Length: unknown");
        }

        // Write to temp file first (atomic write pattern)
        let file = std::fs::File::create(&tmp_path).map_err(|e| {
            let err_msg = format!("Failed to create temp file: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;

        // Use buffered writer for better performance (128KB buffer)
        let mut writer = BufWriter::with_capacity(131072, file);
        let mut reader = response.into_reader();

        // Download in chunks with progress logging
        let mut buffer = vec![0u8; 131072]; // 128KB chunks
        let mut total_written = 0u64;
        let mut last_log_at = 0u64;
        let log_interval = 5_242_880; // Log every 5 MB (reduce logging overhead)

        loop {
            // Check if download was cancelled
            if let Ok(c) = cancelled.lock() {
                if *c {
                    log::info!("Download cancelled by user");
                    // Clean up temp file
                    let _ = std::fs::remove_file(&tmp_path);
                    return Err("Download cancelled by user".to_string());
                }
            }

            let bytes_read = reader.read(&mut buffer).map_err(|e| {
                let err_msg = format!("Failed to read from response: {}", e);
                log::error!("{}", err_msg);
                // Clean up temp file on error
                let _ = std::fs::remove_file(&tmp_path);
                err_msg
            })?;

            if bytes_read == 0 {
                break; // EOF
            }

            writer.write_all(&buffer[..bytes_read]).map_err(|e| {
                let err_msg = format!("Failed to write to file: {}", e);
                log::error!("{}", err_msg);
                // Clean up temp file on error
                let _ = std::fs::remove_file(&tmp_path);
                err_msg
            })?;

            total_written += bytes_read as u64;

            // Log progress every 5 MB to reduce overhead
            if total_written - last_log_at >= log_interval {
                if let Some(total) = content_length {
                    let progress = (total_written as f64 / total as f64) * 100.0;
                    log::info!("Downloaded: {:.1} MB / {:.1} MB ({:.0}%)",
                        total_written as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0,
                        progress);
                } else {
                    log::info!("Downloaded: {:.1} MB", total_written as f64 / 1_048_576.0);
                }
                last_log_at = total_written;
            }
        }

        // Flush buffered writer
        writer.flush().map_err(|e| {
            let err_msg = format!("Failed to flush file: {}", e);
            log::error!("{}", err_msg);
            // Clean up temp file on error
            let _ = std::fs::remove_file(&tmp_path);
            err_msg
        })?;

        // Explicitly drop writer to close file
        drop(writer);

        log::info!("Download complete: {:.2} MB ({} bytes)",
            total_written as f64 / 1_048_576.0, total_written);

        // Verify file size if Content-Length was provided
        if let Some(expected) = content_length {
            if total_written != expected {
                let err_msg = format!(
                    "Size mismatch: downloaded {} bytes, expected {} bytes",
                    total_written, expected
                );
                log::error!("{}", err_msg);
                // Clean up temp file
                let _ = std::fs::remove_file(&tmp_path);
                return Err(err_msg);
            }
        }

        // Atomic rename from temp to final path
        std::fs::rename(&tmp_path, &apk_path).map_err(|e| {
            let err_msg = format!("Failed to rename temp file: {}", e);
            log::error!("{}", err_msg);
            // Clean up temp file on error
            let _ = std::fs::remove_file(&tmp_path);
            err_msg
        })?;

        log::info!("APK saved: {:?}", apk_path);

        Ok(apk_path)
    }

    #[cfg(target_os = "android")]
    fn process_uninstall_android(
        package_name: &str,
        _is_system: bool,
    ) -> OperationStatus {
        use crate::android_shizuku;

        let result = android_shizuku::shizuku_uninstall_package(package_name);

        match result {
            Ok(_) => {
                log::info!("Package uninstalled successfully via Shizuku: {}", package_name);
                OperationStatus::Success(format!("Uninstalled: {}", package_name))
            }
            Err(e) => {
                log::error!("Failed to uninstall package({}): {}", package_name, e);
                OperationStatus::Error(format!("Failed to uninstall: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Run with: cargo test --release -- --ignored --nocapture
    fn test_download_fdroid_apk_performance() {
        // Initialize logger for test output
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        // Test network connectivity first
        println!("\n========================================");
        println!("Checking network connectivity...");
        match ureq::get("https://www.google.com").timeout(std::time::Duration::from_secs(5)).call() {
            Ok(_) => println!("✓ Network is available"),
            Err(e) => {
                println!("✗ Network check failed: {}", e);
                println!("Please check your network connection and DNS settings");
                println!("========================================\n");
                panic!("Network connectivity required for this test");
            }
        }

        let url = "https://f-droid.org/repo/com.machiav3lli.fdroid_1207.apk";
        let app_name = "fdroid_test";
        let tmp_dir = std::env::temp_dir();

        println!("\n========================================");
        println!("Download Performance Test");
        println!("========================================");
        println!("URL: {}", url);
        println!("Temp dir: {:?}", tmp_dir);
        println!("Starting download...\n");

        let start = std::time::Instant::now();

        #[cfg(not(target_os = "android"))]
        let result = {
            let cancelled = Arc::new(Mutex::new(false));
            AppOperationsQueue::download_apk(url, app_name, &tmp_dir, cancelled)
        };

        #[cfg(target_os = "android")]
        let result = {
            println!("Running on Android - using cache dir");
            // On Android, we'd need to use a proper cache directory
            Err("Test not supported on Android without proper setup".to_string())
        };

        let elapsed = start.elapsed();

        match result {
            Ok(path) => {
                println!("\n========================================");
                println!("✓ Download SUCCESSFUL");
                println!("========================================");
                println!("Time elapsed: {:.2}s", elapsed.as_secs_f64());
                println!("File path: {:?}", path);

                // Get file size
                if let Ok(metadata) = std::fs::metadata(&path) {
                    let size_mb = metadata.len() as f64 / 1_048_576.0;
                    let speed_mbps = size_mb / elapsed.as_secs_f64();
                    println!("File size: {:.2} MB", size_mb);
                    println!("Download speed: {:.2} MB/s", speed_mbps);
                    println!("========================================\n");

                    // Clean up test file
                    let _ = std::fs::remove_file(&path);
                    println!("Test file cleaned up");
                } else {
                    println!("Warning: Could not read file metadata");
                }
            }
            Err(e) => {
                println!("\n========================================");
                println!("✗ Download FAILED");
                println!("========================================");
                println!("Time elapsed: {:.2}s", elapsed.as_secs_f64());
                println!("Error: {}", e);
                println!("========================================\n");
                panic!("Download failed: {}", e);
            }
        }
    }

    #[test]
    #[ignore]
    fn test_download_fast_server_performance() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        // Test with a fast CDN server (Cloudflare)
        let url = "https://speed.cloudflare.com/__down?bytes=10000000"; // 10MB test file
        let app_name = "fast_test";
        let tmp_dir = std::env::temp_dir();

        println!("\n========================================");
        println!("Fast Server Download Test");
        println!("========================================");
        println!("URL: {}", url);
        println!("Note: Using Cloudflare CDN for accurate speed test");
        println!("");

        let start = std::time::Instant::now();

        #[cfg(not(target_os = "android"))]
        let result = {
            let cancelled = Arc::new(Mutex::new(false));
            AppOperationsQueue::download_apk(url, app_name, &tmp_dir, cancelled)
        };

        #[cfg(target_os = "android")]
        let result = Err("Not supported on Android".to_string());

        let elapsed = start.elapsed();

        match result {
            Ok(path) => {
                println!("\n========================================");
                println!("✓ Download SUCCESSFUL");
                println!("========================================");
                if let Ok(metadata) = std::fs::metadata(&path) {
                    let size_mb = metadata.len() as f64 / 1_048_576.0;
                    let speed_mbps = size_mb / elapsed.as_secs_f64();
                    println!("Time: {:.2}s", elapsed.as_secs_f64());
                    println!("Size: {:.2} MB", size_mb);
                    println!("Speed: {:.2} MB/s", speed_mbps);
                    println!("========================================\n");
                }
                let _ = std::fs::remove_file(&path);
            }
            Err(e) => {
                println!("✗ Failed: {}", e);
            }
        }
    }

    #[test]
    #[ignore]
    fn test_download_small_file_performance() {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Info)
            .try_init();

        // Test with a smaller file for quick testing
        let url = "https://httpbin.org/bytes/1048576"; // 1MB from httpbin
        let app_name = "small_test";
        let tmp_dir = std::env::temp_dir();

        println!("\n========================================");
        println!("Small File Download Test");
        println!("========================================");
        println!("URL: {}", url);

        let start = std::time::Instant::now();

        #[cfg(not(target_os = "android"))]
        let result = {
            let cancelled = Arc::new(Mutex::new(false));
            AppOperationsQueue::download_apk(url, app_name, &tmp_dir, cancelled)
        };

        #[cfg(target_os = "android")]
        let result = Err("Not supported on Android".to_string());

        let elapsed = start.elapsed();

        match result {
            Ok(path) => {
                println!("✓ Downloaded in {:.3}s", elapsed.as_secs_f64());
                if let Ok(metadata) = std::fs::metadata(&path) {
                    println!("Size: {} bytes", metadata.len());
                }
                let _ = std::fs::remove_file(&path);
            }
            Err(e) => {
                panic!("Download failed: {}", e);
            }
        }
    }
}

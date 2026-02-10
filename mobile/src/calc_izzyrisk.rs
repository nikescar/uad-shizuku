use crate::adb::PackageFingerprint;
use crate::models::PackageInfoCache;
use std::collections::HashMap;

// IzzyRisk Permission Risk Points
// Source: https://android.izzysoft.de/applists/perms
lazy_static::lazy_static! {
    static ref IZZY_RISK_POINTS: HashMap<&'static str, i32> = {
        let mut m = HashMap::new();
        m.insert("ACCESS_BACKGROUND_LOCATION", 3);
        m.insert("ACCESS_COARSE_LOCATION", 2);
        m.insert("ACCESS_FINE_LOCATION", 3);
        m.insert("ACCESS_LOCATION_EXTRA_COMMANDS", 3);
        m.insert("ACCESS_MEDIA_LOCATION", 2);
        m.insert("ACCESS_MOCK_LOCATION", 2);
        m.insert("ACCESS_NOTIFICATIONS", 2);
        m.insert("ACCESS_WIFI_STATE", 3);
        m.insert("ACTIVITY_RECOGNITION", 1);
        m.insert("AD_ID", 1);
        m.insert("ANSWER_PHONE_CALLS", 0);
        m.insert("BIND_ACCESSIBILITY_SERVICE", 3);
        m.insert("BLUETOOTH", 1);
        m.insert("BLUETOOTH_ADMIN", 2);
        m.insert("BODY_SENSORS", 2);
        m.insert("BRICK", 0);
        m.insert("CALL_PHONE", 3);
        m.insert("CAMERA", 4);
        m.insert("CAPTURE_AUDIO_OUTPUT", 3);
        m.insert("CHANGE_COMPONENT_ENABLED_STATE", 2);
        m.insert("CHANGE_CONFIGURATION", 3);
        m.insert("CHANGE_WIFI_MULTICAST_STATE", 0);
        m.insert("CLEAR_APP_CACHE", 0);
        m.insert("CONFIGURE_SIP", 3);
        m.insert("CONNECTIVITY_INTERNAL", 2);
        m.insert("DELETE_PACKAGES", 1);
        m.insert("DEVICE_POWER", 1);
        m.insert("DIAGNOSTIC", 0);
        m.insert("DISABLE_KEYGUARD", 3);
        m.insert("DOWNLOAD_WITHOUT_NOTIFICATION", 1);
        m.insert("DUMP", 4);
        m.insert("EXPAND_STATUS_BAR", 3);
        m.insert("FORCE_STOP_PACKAGES", 0);
        m.insert("GET_PACKAGE_SIZE", 1);
        m.insert("GET_TASKS", 3);
        m.insert("GET_TOP_ACTIVITY_INFO", 0);
        m.insert("GLOBAL_SEARCH", 0);
        m.insert("GLOBAL_SEARCH_CONTROL", 0);
        m.insert("GOOGLE_AUTH.mail", 1);
        m.insert("GOOGLE_AUTH.wise", 1);
        m.insert("GOOGLE_AUTH.writely", 1);
        m.insert("GOOGLE_PHOTOS", 1);
        m.insert("google.MAPS_RECEIVE", 2);
        m.insert("GTALK_SERVICE", 1);
        m.insert("HARDWARE_TEST", 0);
        m.insert("im.permission.READ_ONLY", 3);
        m.insert("INJECT_EVENTS", 0);
        m.insert("INSTALL_DRM", 3);
        m.insert("INSTALL_PACKAGES", 5);
        m.insert("INSTALL_SHORTCUT", 3);
        m.insert("INTERACT_ACROSS_USERS", 2);
        m.insert("INTERACT_ACROSS_USERS_FULL", 3);
        m.insert("INTERNAL_SYSTEM_WINDOW", 3);
        m.insert("INTERNET", 0);
        m.insert("k9.permission.DELETE_MESSAGES", 3);
        m.insert("k9.permission.READ_ATTACHMENT", 3);
        m.insert("k9.permission.READ_MESSAGES", 3);
        m.insert("KILL_BACKGROUND_PROCESSES", 4);
        m.insert("LOCATION_HARDWARE", 3);
        m.insert("MANAGE_EXTERNAL_STORAGE", 3);
        m.insert("MANAGE_USERS", 2);
        m.insert("MASTER_CLEAR", 1);
        m.insert("MODIFY_AUDIO_SETTINGS", 0);
        m.insert("MODIFY_PHONE_STATE", 4);
        m.insert("MOUNT_FORMAT_FILESYSTEMS", 3);
        m.insert("MOUNT_UNMOUNT_FILESYSTEMS", 4);
        m.insert("NEARBY_WIFI_DEVICES", 0);
        m.insert("NFC", 2);
        m.insert("PACKAGE_USAGE_STATS", 1);
        m.insert("PERSISTENT_ACTIVITY", 0);
        m.insert("PREVENT_POWER_KEY", 0);
        m.insert("PROCESS_OUTGOING_CALLS", 4);
        m.insert("QUERY_ALL_PACKAGES", 3);
        m.insert("READ_ATTACHMENT", 3);
        m.insert("READ_CALENDAR", 2);
        m.insert("READ_CALL_LOG", 2);
        m.insert("READ_CLIPBOARD", 3);
        m.insert("READ_CONTACTS", 2);
        m.insert("READ_CONTENT_PROVIDER", 0);
        m.insert("READ_EXTERNAL_STORAGE", 0);
        m.insert("READ_FRAME_BUFFER", 4);
        m.insert("READ_GMAIL", 1);
        m.insert("READ_GMAIL_PROVIDER", 1);
        m.insert("READ_GSERVICES", 1);
        m.insert("READ_HISTORY_BOOKMARKS", 3);
        m.insert("READ_INPUT_STATE", 4);
        m.insert("READ_LOGS", 3);
        m.insert("READ_MEDIA_AUDIO", 0);
        m.insert("READ_MEDIA_IMAGES", 0);
        m.insert("READ_MEDIA_VIDEO", 0);
        m.insert("READ_OWNER_DATA", 0);
        m.insert("READ_PHONE_NUMBERS", 0);
        m.insert("READ_PHONE_STATE", 0);
        m.insert("READ_PRIVILEGED_PHONE_STATE", 3);
        m.insert("READ_PROFILE", 3);
        m.insert("READ_SMS", 3);
        m.insert("READ_SOCIAL_STREAM", 3);
        m.insert("READ_SYNC_SETTINGS", 0);
        m.insert("READ_SYNC_STATS", 0);
        m.insert("READ_USER_DICTIONARY", 0);
        m.insert("RECEIVE_BOOT_COMPLETED", 0);
        m.insert("RECEIVE_MMS", 3);
        m.insert("RECEIVE_SENSITIVE_NOTIFICATIONS", 4);
        m.insert("RECEIVE_SMS", 3);
        m.insert("RECEIVE_WAP_PUSH", 0);
        m.insert("RECORD_AUDIO", 3);
        m.insert("REORDER_TASKS", 0);
        m.insert("REQUEST_IGNORE_BATTERY_OPTIMIZATIONS", 0);
        m.insert("RESTART_PACKAGES", 3);
        m.insert("SEND_SMS", 0);
        m.insert("SEND_SMS_NO_CONFIRMATION", 3);
        m.insert("SET_ACTIVITY_WATCHER", 0);
        m.insert("SET_ALARM", 0);
        m.insert("SET_ALWAYS_FINISH", 0);
        m.insert("SET_DEBUG_APP", 0);
        m.insert("SET_PREFERRED_APPLICATIONS", 2);
        m.insert("WRITE_APN_SETTINGS", 3);
        m.insert("WRITE_CALENDAR", 2);
        m.insert("WRITE_CALL_LOG", 2);
        m.insert("WRITE_CONTACTS", 2);
        m.insert("WRITE_EXTERNAL_STORAGE", 0);
        m.insert("WRITE_GSERVICES", 3);
        m.insert("WRITE_HISTORY_BOOKMARKS", 3);
        m.insert("WRITE_SECURE_SETTINGS", 4);
        m.insert("WRITE_SETTINGS", 3);
        m.insert("WRITE_SMS", 3);
        m.insert("WRITE_SYNC_SETTINGS", 2);
        m.insert("WRITE_USER_DICTIONARY", 2);
        m
    };
}

/// Calculate IzzyRisk score for a single package based on its runtime permissions
pub fn calculate_izzyrisk(package: &PackageFingerprint) -> i32 {
    let mut total_risk = 0;

    // Iterate through all users in the package
    for user in &package.users {
        // Check each runtime permission for the user
        for permission in &user.runtimePermissions {
            // Extract the permission name from the full permission string
            // Android permissions are typically formatted as "android.permission.PERMISSION_NAME: granted=true"
            // First, strip any metadata starting with ':'
            let clean_permission = permission
                .split(':')
                .next()
                .unwrap_or(permission.as_str())
                .trim();

            // Then extract the last part of the dot-separated string
            let permission_name = if let Some(last_part) = clean_permission.split('.').last() {
                last_part
            } else {
                clean_permission
            };

            // Look up the risk score in the IZZY_RISK_POINTS table
            if let Some(&risk_score) = IZZY_RISK_POINTS.get(permission_name) {
                total_risk += risk_score;
                log::trace!(
                    "Package {}: permission {} has risk score {}",
                    package.pkg,
                    permission_name,
                    risk_score
                );
            }
        }
    }

    log::debug!(
        "Package {} has total IzzyRisk score: {}",
        package.pkg,
        total_risk
    );
    total_risk
}

/// Calculate IzzyRisk score with database caching.
/// Returns the cached score if available, otherwise computes and persists it.
pub fn calculate_and_cache_izzyrisk(
    package: &PackageFingerprint,
    cached_pkg: Option<&PackageInfoCache>,
    device_serial: &str,
) -> i32 {
    // Check if cached score exists
    if let Some(cached) = cached_pkg {
        if let Some(score) = cached.izzyscore {
            log::debug!(
                "Package {} izzyrisk cache hit: {}",
                package.pkg,
                score
            );
            return score;
        }
    }

    // Cache miss: calculate from permissions
    let score = calculate_izzyrisk(package);

    // Persist to database
    if let Some(cached) = cached_pkg {
        // Update existing cache entry
        if let Err(e) = crate::db_package_cache::update_package_izzyscore(cached.id, score) {
            log::error!(
                "Failed to persist izzyrisk score for {}: {}",
                package.pkg,
                e
            );
        }
    } else {
        // Create new cache entry with izzyscore
        if let Err(e) = crate::db_package_cache::upsert_package_info_cache(
            &package.pkg,
            &package.pkgChecksum,
            &package.dumpText,
            &package.codePath,
            package.versionCode,
            &package.versionName,
            "",
            &package.lastUpdateTime,
            None,
            None,
            Some(score),
            device_serial,
        ) {
            log::error!(
                "Failed to create cache entry for {}: {}",
                package.pkg,
                e
            );
        }
    }

    score
}

/// Calculate risk scores for all packages in a vector
#[allow(dead_code)]
pub fn calculate_all_risk_scores(packages: &[PackageFingerprint]) -> HashMap<String, i32> {
    let mut package_risk_scores = HashMap::new();

    for package in packages {
        let risk_score = calculate_izzyrisk(package);
        package_risk_scores.insert(package.pkg.clone(), risk_score);
    }

    log::info!(
        "Calculated risk scores for {} packages",
        package_risk_scores.len()
    );
    package_risk_scores
}

/// Calculate risk scores for all packages in background thread with progress tracking
/// This version is used by TabScanControl to calculate scores asynchronously
pub fn calculate_all_risk_scores_async(
    installed_packages: Vec<PackageFingerprint>,
    device_serial: Option<String>,
    shared_scores: std::sync::Arc<std::sync::Mutex<HashMap<String, i32>>>,
    progress_clone: std::sync::Arc<std::sync::Mutex<Option<f32>>>,
    cancelled_clone: std::sync::Arc<std::sync::Mutex<bool>>,
) {
    use std::thread;

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
                    log::info!("IzzyRisk calculation cancelled by user");
                    break;
                }
            }

            // Update progress
            if let Ok(mut p) = progress_clone.lock() {
                *p = Some(i as f32 / total as f32);
            }

            let risk_score = if device_serial_str.is_empty() {
                // No device serial: calculate without caching
                calculate_izzyrisk(package)
            } else {
                let cached_pkg = cached_packages_map.get(&package.pkg);
                let score = calculate_and_cache_izzyrisk(
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

        log::info!(
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

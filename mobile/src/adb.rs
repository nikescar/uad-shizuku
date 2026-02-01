// interface for adb commands wrapper
// get_devices : retrun value of "adb devices"
// install_apk : install apk on connected device
// uninstall_app : uninstall app from connected device
// disable_app : disable app on connected device
// install_adb : install adb on host system (if not installed)
// get_installed_packages : get installed packages on connected device

pub use crate::adb_stt::{AdbPackageInfoUser, PackageFingerprint, UserInfo};
use tracing::{debug, error};

pub fn get_devices() -> std::io::Result<Vec<String>> {
    use std::process::Command;
    // Run adb root first to ensure root access
    // let _ = Command::new("adb").arg("root").output();
    let output = Command::new("adb").arg("devices").arg("-l").output()?;

    if output.status.success() {
        let devices = String::from_utf8_lossy(&output.stdout).to_string();

        let parsed: Vec<String> = devices
            .lines()
            .filter_map(|line| {
                // Skip empty lines and the header line
                if line.trim().is_empty() || line.starts_with("List of devices") {
                    return None;
                }

                // Check if line contains "device" status
                if line.contains("device") {
                    let first_token = line.split_whitespace().next()?.trim().to_string();
                    if !first_token.is_empty() {
                        Some(first_token)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(parsed)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

pub fn get_users(device: &str) -> std::io::Result<Vec<UserInfo>> {
    use std::process::Command;
    debug!("Getting users list for device: {}", device);
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("pm")
        .arg("list")
        .arg("users")
        .output()?;

    if output.status.success() {
        let users_text = String::from_utf8_lossy(&output.stdout).to_string();
        debug!("Received users data: {}", users_text);
        let users = parse_users(&users_text);
        debug!("Parsed {} users", users.len());
        Ok(users)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        error!("ADB command failed: {}", err);
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

fn parse_users(text: &str) -> Vec<UserInfo> {
    // Parse output like:
    // Users:
    //     UserInfo{0:null:4c13} running
    //     UserInfo{10:Work:4c10} running
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("UserInfo{") {
                // Extract content between braces
                if let Some(start) = trimmed.find('{') {
                    if let Some(end) = trimmed.find('}') {
                        let info = &trimmed[start + 1..end];
                        // Split by ':' to get user_id and name
                        let parts: Vec<&str> = info.split(':').collect();
                        if parts.len() >= 2 {
                            if let Ok(user_id) = parts[0].parse::<i32>() {
                                let name = if parts[1] == "null" {
                                    format!("User {}", user_id)
                                } else {
                                    parts[1].to_string()
                                };
                                return Some(UserInfo { user_id, name });
                            }
                        }
                    }
                }
            }
            None
        })
        .collect()
}

// pub fn get_installed_packages(device: &str) -> std::io::Result<Vec<AdbPackageInfo>> {
//     use std::process::Command;
//     debug!("Getting installed packages for device: {}", device);
//     let output = Command::new("adb")
//         .arg("-s")
//         .arg(device)
//         .arg("shell")
//         .arg("dumpsys")
//         .arg("package")
//         .arg("packages")
//         .output()?;

//     if output.status.success() {
//         let packages_text = String::from_utf8_lossy(&output.stdout).to_string();
//         debug!("Received {} bytes of package data", packages_text.len());
//         let packages = parse_package_info(&packages_text);
//         debug!("Parsed {} packages", packages.len());
//         Ok(packages)
//     } else {
//         let err = String::from_utf8_lossy(&output.stderr).to_string();
//         error!("ADB command failed: {}", err);
//         Err(std::io::Error::new(std::io::ErrorKind::Other, err))
//     }
// }

// /// Get all package paths (package_name, path) using pm list packages -f
// pub fn get_all_packages_paths(device: &str) -> std::io::Result<Vec<(String, String)>> {
//     use std::process::Command;
//     debug!("Getting all package paths for device: {}", device);

//     let output = Command::new("adb")
//         .arg("-s")
//         .arg(device)
//         .arg("shell")
//         .arg("pm")
//         .arg("list")
//         .arg("packages")
//         .arg("-f")
//         .output()?;

//     if output.status.success() {
//         let packages_text = String::from_utf8_lossy(&output.stdout).to_string();
//         debug!("Received {} bytes of package path data", packages_text.len());
//         let packages: Vec<(String, String)> = packages_text
//             .lines()
//             .filter_map(|line| {
//                 // Each line is like: package:/data/app/~~rC60QTcUvdtpvpaVzDP97Q==/pe.nikescar.uad_shizuku-uBRYvDyxInoZxF0y2MbCKw==/base.apk=pe.nikescar.uad_shizuku
//                 if let Some(rest) = line.strip_prefix("package:") {
//                     if let Some(pos) = rest.rfind('=') {
//                         let path = &rest[..pos];
//                         let pkg = &rest[pos + 1..];
//                         return Some((pkg.trim().to_string(), path.trim().to_string()));
//                     }
//                 }
//                 None
//             })
//             .collect();
//         debug!("Parsed {} package paths", packages.len());
//         Ok(packages)
//     } else {
//         let err = String::from_utf8_lossy(&output.stderr).to_string();
//         error!("ADB command failed: {}", err);
//         Err(std::io::Error::new(std::io::ErrorKind::Other, err))
//     }
// }

/// Get all package sha256sums and paths (package_name, sha256, path)
/// Output format: package_name|sha256|path
/// Cross-platform implementation (works on Windows, macOS, and Linux)
pub fn get_all_packages_sha256sum(device: &str) -> std::io::Result<Vec<(String, String, String)>> {
    use std::process::Command;
    debug!("Getting all package sha256sums for device: {}", device);

    // Step 1: Get package list with paths using adb shell pm list packages -f
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("pm")
        .arg("list")
        .arg("packages")
        .arg("-f")
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        error!("ADB pm list packages failed: {}", err);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
    }

    let packages_text = String::from_utf8_lossy(&output.stdout).to_string();
    debug!(
        "Received {} bytes of package list data",
        packages_text.len()
    );

    // Step 2: Parse the package list output
    // Each line is like: package:/data/app/~~rC60QTcUvdtpvpaVzDP97Q==/com.example.app-xxx/base.apk=com.example.app
    let package_paths: Vec<(String, String)> = packages_text
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("package:") {
                // Find the last '=' which separates the path from the package name
                if let Some(pos) = rest.rfind('=') {
                    let path = rest[..pos].trim();
                    let pkg = rest[pos + 1..].trim();
                    if !path.is_empty() && !pkg.is_empty() {
                        return Some((pkg.to_string(), path.to_string()));
                    }
                }
            }
            None
        })
        .collect();

    debug!("Parsed {} package paths", package_paths.len());

    // Step 3: Get sha256sum for each package file
    // We batch the sha256sum calls into one command for efficiency
    let mut results: Vec<(String, String, String)> = Vec::new();

    // Build a list of paths to hash
    let paths: Vec<&str> = package_paths.iter().map(|(_, p)| p.as_str()).collect();

    if paths.is_empty() {
        return Ok(results);
    }

    // Run sha256sum on all paths at once (more efficient than one call per file)
    // The command handles missing files gracefully by outputting errors to stderr
    let paths_arg = paths
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<_>>()
        .join(" ");

    let sha256_cmd = format!("sha256sum {} 2>/dev/null", paths_arg);
    let sha256_output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg(&sha256_cmd)
        .output()?;

    // Parse sha256sum output: each line is "hash  path"
    let sha256_text = String::from_utf8_lossy(&sha256_output.stdout).to_string();

    // Create a map of path -> hash for quick lookup
    let mut hash_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for line in sha256_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // sha256sum output format: "hash  path" (two spaces between hash and path)
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() >= 2 {
            let hash = parts[0].trim();
            // The path might have leading whitespace after the split
            let path = parts[1].trim();
            if !hash.is_empty() && !path.is_empty() && hash.len() == 64 {
                // SHA256 is 64 hex chars
                hash_map.insert(path.to_string(), hash.to_string());
            }
        }
    }

    // Step 4: Combine package info with hashes
    for (pkg, path) in package_paths {
        if let Some(hash) = hash_map.get(&path) {
            debug!("Parsed: {} -> {} ({})", pkg, hash, path);
            results.push((pkg, hash.clone(), path));
        } else {
            debug!("No hash found for package {} at path {}", pkg, path);
        }
    }

    debug!("Parsed {} package sha256sums", results.len());
    Ok(results)
}

/// Get fingerprints of all packages
/// Cross-platform implementation (works on Windows, macOS, and Linux)
pub fn get_all_packages_fingerprints(device: &str) -> std::io::Result<Vec<PackageFingerprint>> {
    use std::process::Command;
    debug!("Getting all package fingerprints for device: {}", device);

    // Step 1: Get full dumpsys package packages output
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("dumpsys")
        .arg("package")
        .arg("packages")
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        error!("ADB command failed: {}", err);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
    }

    let fingerprints_text = String::from_utf8_lossy(&output.stdout).to_string();
    debug!(
        "Received {} bytes of package fingerprint data",
        fingerprints_text.len()
    );

    // Step 2: Filter lines in Rust (equivalent to grep -e 'Pattern1' -e 'Pattern2' ...)
    // Patterns to match:
    // - 'Package ' - package header lines
    // - 'Path=' - codePath lines (note: original used 'Path=' but code expects 'codePath=')
    // - 'versionCode=' - version code
    // - 'versionName=' - version name
    // - 'lastUpdateTime=' - last update time
    // - 'firstInstallTime=' - first install time
    // - 'User ' - user info lines
    // - 'flags=' - package flags
    // - 'privateFlags=' - private flags
    // - 'permissions:' - permissions section headers
    // - 'permission.' - individual permission lines
    let fingerprints: Vec<String> = fingerprints_text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return false;
            }
            // Check if line contains any of the patterns
            line.contains("Package ")
                || line.contains("codePath=")
                || line.contains("versionCode=")
                || line.contains("versionName=")
                || line.contains("lastUpdateTime=")
                || line.contains("firstInstallTime=")
                || line.contains("User ")
                || line.contains("flags=")
                || line.contains("privateFlags=")
                || line.contains("permissions:")
                || line.contains("permission.")
        })
        .map(|line| line.to_string())
        .collect();

    debug!("Filtered to {} relevant lines", fingerprints.len());
    let parsed_fingerprints = parse_package_fingerprints(fingerprints);
    debug!("Parsed {} package fingerprints", parsed_fingerprints.len());
    Ok(parsed_fingerprints)
}

/// Parse package fingerprints from dumpsys output
pub fn parse_package_fingerprints(fingerprints: Vec<String>) -> Vec<PackageFingerprint> {
    use regex::Regex;

    let mut packages = Vec::new();
    let mut current_pkg: Option<String> = None;
    let mut current_pkg_checksum: Option<String> = None;
    let mut current_code_path: Option<String> = None;
    let mut current_version_code: Option<i32> = None;
    let mut current_version_name: Option<String> = None;
    let mut current_flags: Option<String> = None;
    let mut current_private_flags: Option<String> = None;
    let mut current_last_update_time: Option<String> = None;
    let mut current_users: Vec<AdbPackageInfoUser> = Vec::new();
    let mut current_dump_text: Vec<String> = Vec::new();
    let mut current_install_permissions: Vec<String> = Vec::new();
    let mut in_runtime_permissions: bool = false;

    let package_re = Regex::new(r"Package \[([^\]]+)\] \(([^)]+)\)").unwrap();
    let code_path_re = Regex::new(r"codePath=(.+)").unwrap();
    let version_code_re = Regex::new(r"versionCode=(\d+)").unwrap();
    let version_name_re = Regex::new(r"versionName=(.+)").unwrap();
    let flags_re = Regex::new(r"flags=\[\s*(.+?)\s*\]").unwrap();
    let private_flags_re = Regex::new(r"privateFlags=\[\s*(.+?)\s*\]").unwrap();
    let user_re = Regex::new(r"User (\d+):").unwrap();
    let first_install_re = Regex::new(r"firstInstallTime=(.+)").unwrap();
    let last_update_re = Regex::new(r"lastUpdateTime=(.+)").unwrap();

    let mut current_user: Option<AdbPackageInfoUser> = None;
    let mut in_install_permissions: bool = false;

    for line in fingerprints {
        // Check if this is a new package
        if let Some(caps) = package_re.captures(&line) {
            // Save current user if any
            if let Some(user) = current_user.take() {
                current_users.push(user);
            }

            // Save previous package if complete
            if let (Some(pkg), Some(path), Some(vc), Some(vn), Some(lut)) = (
                current_pkg.clone(),
                current_code_path.clone(),
                current_version_code,
                current_version_name.clone(),
                current_last_update_time.clone(),
            ) {
                // Don't include the new Package line in dump_text
                packages.push(PackageFingerprint {
                    pkg: pkg,
                    codePath: path,
                    versionCode: vc,
                    versionName: vn,
                    flags: current_flags.clone().unwrap_or_default(),
                    privateFlags: current_private_flags.clone().unwrap_or_default(),
                    installPermissions: current_install_permissions.clone(),
                    users: current_users.clone(),
                    lastUpdateTime: lut,
                    pkgChecksum: current_pkg_checksum.clone().unwrap_or_default(),
                    dumpText: current_dump_text.join("\n"),
                });
                current_dump_text.clear();
                current_users.clear();
                current_install_permissions.clear();
            }

            // Start new package - add the Package line to dump_text
            current_dump_text.push(line.clone());
            current_pkg = Some(caps.get(1).unwrap().as_str().to_string());
            current_pkg_checksum = Some(caps.get(2).unwrap().as_str().to_string());
            current_code_path = None;
            current_version_code = None;
            current_version_name = None;
            current_flags = None;
            current_private_flags = None;
            current_last_update_time = None;
            in_install_permissions = false;
            in_runtime_permissions = false;
            continue;
        }

        // Add non-Package lines to dump_text
        current_dump_text.push(line.clone());

        // Check if this is the start of install permissions section
        if line.trim() == "install permissions:" {
            in_install_permissions = true;
            continue;
        }

        // Check if we're in install permissions section
        if in_install_permissions {
            // Install permissions are indented with 6 spaces (or more)
            // Exit when we hit a line with same or less indentation (4 spaces or less)
            let leading_spaces = line.len() - line.trim_start().len();
            if leading_spaces <= 4 && !line.trim().is_empty() {
                in_install_permissions = false;
            } else if leading_spaces >= 6 && !line.trim().is_empty() {
                // Parse permission line: "android.permission.GET_PACKAGE_SIZE: granted=true"
                let trimmed = line.trim();
                current_install_permissions.push(trimmed.to_string());
                continue;
            }
        }

        // Check if this is the start of runtime permissions section
        if line.trim() == "runtime permissions:" {
            in_runtime_permissions = true;
            continue;
        }

        // Check if we're in runtime permissions section
        if in_runtime_permissions {
            // Runtime permissions are indented with 8 spaces (or more)
            // Exit when we hit a line with same or less indentation (6 spaces or less)
            let leading_spaces = line.len() - line.trim_start().len();
            if leading_spaces <= 6 && !line.trim().is_empty() {
                in_runtime_permissions = false;
            } else if leading_spaces >= 8 && !line.trim().is_empty() {
                // Parse permission line: "android.permission.POST_NOTIFICATIONS: granted=false, flags=[ ...]"
                let trimmed = line.trim();
                if let Some(ref mut user) = current_user {
                    user.runtimePermissions.push(trimmed.to_string());
                }
                continue;
            }
        }

        // Check if this is a User line
        if let Some(caps) = user_re.captures(&line) {
            // Save previous user if any
            if let Some(user) = current_user.take() {
                current_users.push(user);
            }

            // Reset runtime permissions flag when switching users
            in_runtime_permissions = false;

            // Start new user
            if let Ok(user_id) = caps.get(1).unwrap().as_str().parse::<i32>() {
                let mut user = AdbPackageInfoUser {
                    userId: user_id,
                    ceDataInode: 0,
                    deDataInode: 0,
                    installed: false,
                    hidden: false,
                    suspended: false,
                    distractionFlags: 0,
                    stopped: false,
                    notLaunched: false,
                    enabled: 0,
                    instant: false,
                    virtualField: false,
                    quarantined: false,
                    installReason: 0,
                    dataDir: String::new(),
                    firstInstallTime: String::new(),
                    uninstallReason: 0,
                    lastDisabledCaller: String::new(),
                    gids: Vec::new(),
                    runtimePermissions: Vec::new(),
                };

                // Parse key-value pairs on the same line
                for pair in line.split_whitespace() {
                    if let Some((key, value)) = pair.split_once('=') {
                        match key {
                            "ceDataInode" => user.ceDataInode = value.parse().unwrap_or(0),
                            "deDataInode" => user.deDataInode = value.parse().unwrap_or(0),
                            "installed" => user.installed = value == "true",
                            "hidden" => user.hidden = value == "true",
                            "suspended" => user.suspended = value == "true",
                            "distractionFlags" => {
                                user.distractionFlags = value.parse().unwrap_or(0)
                            }
                            "stopped" => user.stopped = value == "true",
                            "notLaunched" => user.notLaunched = value == "true",
                            "enabled" => user.enabled = value.parse().unwrap_or(0),
                            "instant" => user.instant = value == "true",
                            "virtual" => user.virtualField = value == "true",
                            "quarantined" => user.quarantined = value == "true",
                            _ => {}
                        }
                    }
                }
                current_user = Some(user);
            }
            continue;
        }

        // Parse other fields
        if let Some(caps) = code_path_re.captures(&line) {
            current_code_path = Some(caps.get(1).unwrap().as_str().to_string());
        } else if let Some(caps) = version_code_re.captures(&line) {
            if let Ok(vc) = caps.get(1).unwrap().as_str().parse::<i32>() {
                current_version_code = Some(vc);
            }
        } else if let Some(caps) = version_name_re.captures(&line) {
            current_version_name = Some(caps.get(1).unwrap().as_str().to_string());
        } else if let Some(caps) = flags_re.captures(&line) {
            current_flags = Some(caps.get(1).unwrap().as_str().to_string());
        } else if let Some(caps) = private_flags_re.captures(&line) {
            current_private_flags = Some(caps.get(1).unwrap().as_str().to_string());
        } else if let Some(caps) = first_install_re.captures(&line) {
            // This is a per-user field
            if let Some(ref mut user) = current_user {
                user.firstInstallTime = caps.get(1).unwrap().as_str().to_string();
            }
        } else if let Some(caps) = last_update_re.captures(&line) {
            current_last_update_time = Some(caps.get(1).unwrap().as_str().to_string());
        }
    }

    // Save current user if any
    if let Some(user) = current_user.take() {
        current_users.push(user);
    }

    // Don't forget the last package
    if let (Some(pkg), Some(path), Some(vc), Some(vn), Some(lut)) = (
        current_pkg,
        current_code_path,
        current_version_code,
        current_version_name,
        current_last_update_time,
    ) {
        packages.push(PackageFingerprint {
            pkg: pkg,
            codePath: path,
            versionCode: vc,
            versionName: vn,
            flags: current_flags.unwrap_or_default(),
            privateFlags: current_private_flags.unwrap_or_default(),
            installPermissions: current_install_permissions,
            users: current_users,
            lastUpdateTime: lut,
            pkgChecksum: current_pkg_checksum.unwrap_or_default(),
            dumpText: current_dump_text.join("\n"),
        });
    }

    debug!("Parsed {} package fingerprints before dedup", packages.len());

    // Deduplicate packages: merge users from duplicate package entries
    // This handles cases where dumpsys returns the same package multiple times
    // (e.g., for system packages with overlays or multi-user installations)
    let mut deduped: std::collections::HashMap<String, PackageFingerprint> =
        std::collections::HashMap::new();

    for pkg in packages {
        if let Some(existing) = deduped.get_mut(&pkg.pkg) {
            // Merge users from duplicate entry, avoiding duplicate user IDs
            for user in pkg.users {
                if !existing.users.iter().any(|u| u.userId == user.userId) {
                    existing.users.push(user);
                }
            }
            // Merge install permissions
            for perm in pkg.installPermissions {
                if !existing.installPermissions.contains(&perm) {
                    existing.installPermissions.push(perm);
                }
            }
        } else {
            deduped.insert(pkg.pkg.clone(), pkg);
        }
    }

    let result: Vec<PackageFingerprint> = deduped.into_values().collect();
    debug!("Parsed {} package fingerprints after dedup", result.len());
    result
}

/// Get single package files in codePath folder with their SHA256 sums
/// Returns space-separated sha256sums and space-separated file paths
/// Cross-platform implementation (works on Windows, macOS, and Linux)
pub fn get_single_package_sha256sum(
    device: &str,
    package_name: &str,
) -> std::io::Result<(String, String)> {
    use std::process::Command;
    debug!(
        "Getting package paths for {} on device: {}",
        package_name, device
    );

    // Step 1: Get package info using dumpsys package
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("dumpsys")
        .arg("package")
        .arg(package_name)
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        error!("ADB dumpsys package failed: {}", err);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
    }

    let dumpsys_text = String::from_utf8_lossy(&output.stdout).to_string();

    // Step 2: Parse codePath values from dumpsys output
    // Lines look like: "    codePath=/data/app/~~xxx==/com.example.app-yyy=="
    // or for overlays: "    codePath=/product/overlay/SomeOverlay.apk"
    let mut directory_paths: Vec<String> = Vec::new();
    let mut apk_file_paths: Vec<String> = Vec::new();

    for line in dumpsys_text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("codePath=") {
            let path = rest.trim();
            if !path.is_empty() {
                if path.ends_with(".apk") {
                    // This is a direct .apk file path (common for overlays)
                    apk_file_paths.push(path.to_string());
                } else {
                    // This is a directory path
                    directory_paths.push(path.to_string());
                }
            }
        }
    }

    debug!(
        "Found {} directory paths and {} apk file paths for {}",
        directory_paths.len(),
        apk_file_paths.len(),
        package_name
    );

    if directory_paths.is_empty() && apk_file_paths.is_empty() {
        return Ok((String::new(), String::new()));
    }

    // Step 3: Collect all files to hash
    let mut all_files: Vec<String> = Vec::new();

    // Add direct .apk files first
    all_files.extend(apk_file_paths);

    // Find all files in each directory path
    for code_path in &directory_paths {
        let mut found_files = false;

        // First try the find command
        let find_output = Command::new("adb")
            .arg("-s")
            .arg(device)
            .arg("shell")
            .arg("find")
            .arg(code_path)
            .arg("-type")
            .arg("f")
            .output()?;

        if find_output.status.success() {
            let files_text = String::from_utf8_lossy(&find_output.stdout).to_string();
            for line in files_text.lines() {
                let path = line.trim();
                if !path.is_empty() {
                    all_files.push(path.to_string());
                    found_files = true;
                }
            }
        } else {
            debug!(
                "find command failed for path {}: {}",
                code_path,
                String::from_utf8_lossy(&find_output.stderr)
            );
        }

        // If find failed or returned no results, try common APK locations directly
        // This is needed for user apps where find fails due to permission issues
        if !found_files {
            // Use ls to list all .apk files in the directory
            let ls_output = Command::new("adb")
                .arg("-s")
                .arg(device)
                .arg("shell")
                .arg("ls")
                .arg(format!("{}/*.apk", code_path))
                .output()?;

            if ls_output.status.success() {
                let files_text = String::from_utf8_lossy(&ls_output.stdout).to_string();
                for line in files_text.lines() {
                    let path = line.trim();
                    if !path.is_empty() && !path.contains("No such file") {
                        debug!("Found APK via ls: {}", path);
                        all_files.push(path.to_string());
                        found_files = true;
                    }
                }
            } else {
                debug!(
                    "ls command failed for path {}: {}",
                    code_path,
                    String::from_utf8_lossy(&ls_output.stderr)
                );
            }

            if !found_files {
                debug!(
                    "No APK files found in {} using fallback methods",
                    code_path
                );
            }
        }
    }

    // Deduplicate files to avoid scanning the same file twice
    // (e.g., base.apk found by stat and then again by ls *.apk)
    all_files.sort();
    all_files.dedup();

    debug!(
        "Found {} files to hash for {}",
        all_files.len(),
        package_name
    );

    if all_files.is_empty() {
        return Ok((String::new(), String::new()));
    }

    // Step 4: Get sha256sum for all files
    // Build quoted paths for the shell command
    let paths_arg = all_files
        .iter()
        .map(|p| format!("\"{}\"", p))
        .collect::<Vec<_>>()
        .join(" ");

    let sha256_cmd = format!("sha256sum {} 2>/dev/null", paths_arg);
    let sha256_output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg(&sha256_cmd)
        .output()?;

    let output_text = String::from_utf8_lossy(&sha256_output.stdout).to_string();
    debug!("SHA256 output length: {}", output_text.len());

    // Step 5: Parse sha256sum output
    let mut paths: Vec<String> = Vec::new();
    let mut sha256sums: Vec<String> = Vec::new();

    for line in output_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // sha256sum output format: "hash  path" (two spaces between hash and path)
        let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
        if parts.len() >= 2 {
            let sha256 = parts[0].trim();
            let path = parts[1].trim();

            // Validate hash length (SHA256 = 64 hex chars)
            if sha256.len() != 64 {
                debug!("Invalid hash length for line: {}", line);
                continue;
            }

            // Validate that this is a file path (not a directory)
            if path.ends_with('/') || path.is_empty() {
                debug!("Skipping invalid path: {}", path);
                continue;
            }

            paths.push(path.to_string());
            sha256sums.push(sha256.to_string());
        }
    }

    debug!(
        "Found {} files with hashes for {}",
        paths.len(),
        package_name
    );

    // Return space-separated paths and sha256sums
    let paths_str = paths.join(" ");
    let sha256sums_str = sha256sums.join(" ");

    // Save to cache if we have results
    if !paths_str.is_empty() && !sha256sums_str.is_empty() {
        if let Some(cached_pkg) =
            crate::db_package_cache::get_cached_package_info(package_name, device)
        {
            if let Err(e) = crate::db_package_cache::update_package_apk_info(
                cached_pkg.id,
                &paths_str,
                &sha256sums_str,
            ) {
                debug!("Failed to update package apk info cache: {}", e);
            }
        }
    }

    Ok((paths_str, sha256sums_str))
}

// pub fn get_package_info(device: &str, package_name: &str) -> std::io::Result<Option<AdbPackageInfo>> {
//     use std::process::Command;
//     debug!("Getting package info for {} on device: {}", package_name, device);
//     let output = Command::new("adb")
//         .arg("-s")
//         .arg(device)
//         .arg("shell")
//         .arg("dumpsys")
//         .arg("package")
//         .arg("packages")
//         .arg(package_name)
//         .output()?;

//     if output.status.success() {
//         let package_text = String::from_utf8_lossy(&output.stdout).to_string();
//         debug!("Received {} bytes of package data for {}", package_text.len(), package_name);
//         let mut packages = parse_package_info(&package_text);
//         debug!("Parsed {} packages", packages.len());

//         // Return the first package that matches the exact package name
//         Ok(packages.into_iter().find(|p| p.pkg == package_name))
//     } else {
//         let err = String::from_utf8_lossy(&output.stderr).to_string();
//         error!("ADB command failed: {}", err);
//         Err(std::io::Error::new(std::io::ErrorKind::Other, err))
//     }
// }

// /// Parse a single package info from dump text (used with fingerprints)
// pub fn parse_package_info_from_dump(dump_text: &str) -> Option<AdbPackageInfo> {
//     // Use the existing parse_package_info function and return the first result
//     let packages = parse_package_info(dump_text);
//     packages.into_iter().next()
// }

// fn parse_package_info(text: &str) -> Vec<AdbPackageInfo> {
//     use std::collections::HashMap;

//     let mut packages = Vec::new();
//     let lines: Vec<&str> = text.lines().collect();
//     let mut i = 0;
//     let mut package_count = 0;

//     debug!("Starting to parse {} lines", lines.len());

//     while i < lines.len() {
//         let line = lines[i].trim();

//         // Look for package start marker "Package ["
//         if line.starts_with("Package [") {
//             // Extract package name between "Package [" and "]"
//             if let Some(rest) = line.strip_prefix("Package [") {
//                 if let Some(end_bracket) = rest.find(']') {
//                     let pkg_name = &rest[..end_bracket];
//                     package_count += 1;
//                     let mut package_info = AdbPackageInfo::default();
//                     package_info.pkg = pkg_name.trim().to_string();

//                     i += 1;

//                     // Parse package fields
//                     while i < lines.len() {
//                         let field_line = lines[i];

//                         // Stop if we hit the next package
//                         if field_line.trim().starts_with("Package [") {
//                             break;
//                         }

//                         // Check if this is a User line
//                         if field_line.trim().starts_with("User ") {
//                             if let Some(user) = parse_user_info(&lines, &mut i) {
//                                 package_info.users.push(user);
//                             }
//                             continue;
//                         }

//                         // Parse multi-line lists
//                         if field_line.trim().starts_with("usesLibraries:") {
//                             i += 1;
//                             while i < lines.len() {
//                                 let raw_line = lines[i];
//                                 let trimmed_line = raw_line.trim();
//                                 let leading_spaces = raw_line.len() - raw_line.trim_start().len();

//                                 // Break if indentation is less than or equal to 4 spaces (parent level)
//                                 if leading_spaces <= 4 && !trimmed_line.is_empty() {
//                                     i -= 1;
//                                     break;
//                                 }

//                                 if !trimmed_line.is_empty() {
//                                     package_info.usesLibraries.push(trimmed_line.to_string());
//                                 }
//                                 i += 1;
//                             }
//                         } else if field_line.trim().starts_with("usesLibraryFiles:") {
//                             i += 1;
//                             while i < lines.len() {
//                                 let raw_line = lines[i];
//                                 let trimmed_line = raw_line.trim();
//                                 let leading_spaces = raw_line.len() - raw_line.trim_start().len();

//                                 // Break if indentation is less than or equal to 4 spaces (parent level)
//                                 if leading_spaces <= 4 && !trimmed_line.is_empty() {
//                                     i -= 1;
//                                     break;
//                                 }

//                                 if !trimmed_line.is_empty() {
//                                     package_info.usesLibraryFiles.push(trimmed_line.to_string());
//                                 }
//                                 i += 1;
//                             }
//                         } else if field_line.trim().starts_with("declared permissions:") {
//                             i += 1;
//                             while i < lines.len() {
//                                 let raw_line = lines[i];
//                                 let trimmed_line = raw_line.trim();
//                                 let leading_spaces = raw_line.len() - raw_line.trim_start().len();

//                                 // Break if indentation is less than or equal to 4 spaces (parent level)
//                                 if leading_spaces <= 4 && !trimmed_line.is_empty() {
//                                     i -= 1;
//                                     break;
//                                 }

//                                 if !trimmed_line.is_empty() {
//                                     // Extract permission name (before colon if present)
//                                     let perm_name = if let Some(colon_pos) = trimmed_line.find(':') {
//                                         trimmed_line[..colon_pos].trim().to_string()
//                                     } else {
//                                         trimmed_line.to_string()
//                                     };
//                                     package_info.declaredPermissions.push(perm_name);
//                                 }
//                                 i += 1;
//                             }
//                         } else if field_line.trim().starts_with("install permissions:") {
//                             // Collect all install permissions into a single string
//                             let mut install_perms = Vec::new();
//                             i += 1;
//                             while i < lines.len() {
//                                 let raw_line = lines[i];
//                                 let trimmed_line = raw_line.trim();
//                                 let leading_spaces = raw_line.len() - raw_line.trim_start().len();

//                                 // Break if indentation is less than or equal to 4 spaces (parent level)
//                                 if leading_spaces <= 4 && !trimmed_line.is_empty() {
//                                     i -= 1;
//                                     break;
//                                 }

//                                 if !trimmed_line.is_empty() {
//                                     install_perms.push(trimmed_line.to_string());
//                                 }
//                                 i += 1;
//                             }
//                             package_info.installPermissions = install_perms.join("\n");
//                         }

//                         // Parse individual fields
//                         if let Some((key, value)) = parse_field(field_line) {
//                             match key {
//                                 "codePath" => package_info.codePath = value,
//                                 "resourcePath" => package_info.resourcePath = value,
//                                 "versionCode" => package_info.versionCode = value.parse().unwrap_or(0),
//                                 "versionName" => package_info.versionName = value,
//                                 "targetSdk" => package_info.targetSdk = value.parse().unwrap_or(0),
//                                 "minSdk" => package_info.minSdk = value.parse().unwrap_or(0),
//                                 "timeStamp" => package_info.timeStamp = value,
//                                 "lastUpdateTime" => package_info.lastUpdateTime = value,
//                                 "installerPackageName" => package_info.installerPackageName = value,
//                                 "flags" => package_info.flags = value,
//                                 "privateFlags" => package_info.privateFlags = value,
//                                 "pkgFlags" => package_info.pkgFlags = value,
//                                 "privatePkgFlags" => package_info.privatePkgFlags = value,
//                                 _ => {}
//                             }
//                         }

//                         i += 1;
//                     }

//                     packages.push(package_info);

//                     continue;
//                 }
//             }
//         }

//         i += 1;
//     }

//     debug!("Finished parsing. Total packages before deduplication: {}", packages.len());

//     // Deduplicate packages: keep only the latest version of each package
//     let mut package_map: HashMap<String, AdbPackageInfo> = HashMap::new();
//     let mut version_names: HashMap<String, Vec<String>> = HashMap::new();

//     for pkg in packages {
//         let pkg_name = pkg.pkg.clone();

//         if let Some(existing) = package_map.get_mut(&pkg_name) {
//             // Package already exists, check which version is newer
//             if pkg.versionCode > existing.versionCode {
//                 // New package is newer, add existing versionName to the list first (it's older)
//                 if !existing.versionName.is_empty() {
//                     version_names.entry(pkg_name.clone())
//                         .or_insert_with(Vec::new)
//                         .push(existing.versionName.clone());
//                 }
//                 // Replace with the newer package
//                 *existing = pkg;
//             } else {
//                 // Existing package is newer or same version, just add this version name to the list
//                 if !pkg.versionName.is_empty() {
//                     version_names.entry(pkg_name.clone())
//                         .or_insert_with(Vec::new)
//                         .push(pkg.versionName.clone());
//                 }
//             }
//         } else {
//             // First occurrence of this package
//             package_map.insert(pkg_name.clone(), pkg);
//         }
//     }

//     // Update versionName field to include all versions (space-separated, newest first)
//     let mut result: Vec<AdbPackageInfo> = package_map.into_iter().map(|(pkg_name, mut pkg)| {
//         if let Some(mut old_versions) = version_names.remove(&pkg_name) {
//             // Prepend the current (newest) version name
//             let mut all_versions = vec![pkg.versionName.clone()];
//             all_versions.append(&mut old_versions);
//             // Filter out empty version names and join with space
//             pkg.versionName = all_versions.into_iter()
//                 .filter(|v| !v.is_empty())
//                 .collect::<Vec<_>>()
//                 .join(" ");
//         }
//         pkg
//     }).collect();

//     // Sort by package name for consistent ordering
//     result.sort_by(|a, b| a.pkg.cmp(&b.pkg));

//     debug!("Finished deduplication. Total unique packages: {}", result.len());
//     result
// }

// fn parse_user_info(lines: &[&str], i: &mut usize) -> Option<AdbPackageInfoUser> {
//     let line = lines[*i].trim();

//     // Parse "User 0: ceDataInode=3927 deDataInode=1303 installed=true ..."
//     if !line.starts_with("User ") {
//         return None;
//     }

//     let mut user = AdbPackageInfoUser {
//         userId: 0,
//         ceDataInode: 0,
//         deDataInode: 0,
//         installed: false,
//         hidden: false,
//         suspended: false,
//         distractionFlags: 0,
//         stopped: false,
//         notLaunched: false,
//         enabled: 0,
//         instant: false,
//         virtualField: false,
//         quarantined: false,
//         installReason: 0,
//         dataDir: String::new(),
//         firstInstallTime: String::new(),
//         uninstallReason: 0,
//         // overlayPaths: Vec::new(),
//         // legacyOverlayPaths: Vec::new(),
//         lastDisabledCaller: String::new(),
//         gids: Vec::new(),
//         runtimePermissions: Vec::new(),
//     };

//     // Parse userId
//     if let Some(user_id_str) = line.strip_prefix("User ").and_then(|s| s.split(':').next()) {
//         user.userId = user_id_str.trim().parse().unwrap_or(0);
//     }

//     // Parse key-value pairs on the same line
//     for pair in line.split_whitespace() {
//         if let Some((key, value)) = pair.split_once('=') {
//             match key {
//                 "ceDataInode" => user.ceDataInode = value.parse().unwrap_or(0),
//                 "deDataInode" => user.deDataInode = value.parse().unwrap_or(0),
//                 "installed" => user.installed = value == "true",
//                 "hidden" => user.hidden = value == "true",
//                 "suspended" => user.suspended = value == "true",
//                 "distractionFlags" => user.distractionFlags = value.parse().unwrap_or(0),
//                 "stopped" => user.stopped = value == "true",
//                 "notLaunched" => user.notLaunched = value == "true",
//                 "enabled" => user.enabled = value.parse().unwrap_or(0),
//                 "instant" => user.instant = value == "true",
//                 "virtual" => user.virtualField = value == "true",
//                 "quarantined" => user.quarantined = value == "true",
//                 _ => {}
//             }
//         }
//     }

//     // Parse subsequent indented fields
//     *i += 1;
//     while *i < lines.len() {
//         let field_line = lines[*i];

//         // Stop if we hit another User or Package or non-indented line
//         if field_line.trim().starts_with("User ")
//             || field_line.trim().starts_with("Package [")
//             || (!field_line.starts_with("      ") && !field_line.trim().is_empty()) {
//             *i -= 1; // Step back so the outer loop can process this line
//             break;
//         }

//         // Parse runtime permissions
//         if field_line.trim().starts_with("runtime permissions:") {
//             *i += 1;
//             while *i < lines.len() {
//                 let field_line_inner = lines[*i];
//                 let perm_line = field_line_inner.trim();

//                 // Runtime permissions are indented with 8 spaces
//                 // Stop if we hit a line with same or less indentation (6 spaces or less)
//                 let leading_spaces = field_line_inner.len() - field_line_inner.trim_start().len();
//                 if leading_spaces <= 6 && !perm_line.is_empty() {
//                     *i -= 1;
//                     break;
//                 }

//                 // Store the full permission line including granted status and flags
//                 // Only process lines that are properly indented (8+ spaces) and non-empty
//                 if leading_spaces >= 8 && !perm_line.is_empty() {
//                     user.runtimePermissions.push(perm_line.to_string());
//                 }

//                 *i += 1;
//             }
//         }

//         // Parse individual fields
//         if let Some((key, value)) = parse_field(field_line) {
//             match key {
//                 "installReason" => user.installReason = value.parse().unwrap_or(0),
//                 "dataDir" => user.dataDir = value,
//                 "firstInstallTime" => user.firstInstallTime = value,
//                 "uninstallReason" => user.uninstallReason = value.parse().unwrap_or(0),
//                 _ => {}
//             }
//         }

//         *i += 1;
//     }

//     Some(user)
// }

// fn parse_field(line: &str) -> Option<(&str, String)> {
//     let trimmed = line.trim();
//     if let Some(eq_pos) = trimmed.find('=') {
//         let key = trimmed[..eq_pos].trim();
//         let value = trimmed[eq_pos + 1..].trim().to_string();
//         Some((key, value))
//     } else {
//         None
//     }
// }

#[allow(dead_code)]
pub fn install_apk(apk_path: &str, device: &str) -> std::io::Result<String> {
    use std::process::Command;
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("install")
        .arg(apk_path)
        .output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

pub fn uninstall_app(package_name: &str, device: &str) -> std::io::Result<String> {
    use std::process::Command;
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("uninstall")
        .arg(package_name)
        .output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

pub fn uninstall_app_user(
    package_name: &str,
    device: &str,
    user_id: Option<&str>,
) -> std::io::Result<String> {
    use std::process::Command;

    let user = user_id.unwrap_or("0");
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("pm")
        .arg("uninstall")
        .arg("--user")
        .arg(user)
        .arg(package_name)
        .output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

pub fn disable_app_current_user(
    package_name: &str,
    device: &str,
    user_id: Option<&str>,
) -> std::io::Result<String> {
    use std::process::Command;

    let user = user_id.unwrap_or("0");
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("pm")
        .arg("disable-user")
        .arg("--user")
        .arg(user)
        .arg(package_name)
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

pub fn enable_app(package_name: &str, device: &str) -> std::io::Result<String> {
    use std::process::Command;
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("pm")
        .arg("enable")
        .arg(package_name)
        .output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

pub fn pull_file_to_temp(
    device_serial: &str,
    file_path: &str,
    tmp_dir: &str,
    package_id: &str,
) -> std::io::Result<String> {
    use std::process::Command;

    // Construct the target filename using package_id
    let filename = format!("{}.apk", package_id.replace('.', "_"));
    let local_path = std::path::PathBuf::from(tmp_dir).join(filename);
    
    // Convert to absolute path to ensure adb uses the right location
    let absolute_path = local_path;
    
    let absolute_path_str = absolute_path.to_str().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path encoding")
    })?;

    debug!("Executing: adb -s {} pull {} {}", device_serial, file_path, absolute_path_str);
    
    // Verify parent directory exists before pull
    if let Some(parent) = absolute_path.parent() {
        if !parent.exists() {
            debug!("Creating parent directory: {:?}", parent);
            std::fs::create_dir_all(parent)?;
        }
        debug!("Parent directory exists: {:?}", parent);
    }
    
    let output = Command::new("adb")
        .arg("-s")
        .arg(device_serial)
        .arg("pull")
        .arg(file_path)
        .arg(absolute_path_str)
        .current_dir(tmp_dir)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    debug!("adb pull stdout: {}", stdout);
    if !stderr.is_empty() {
        debug!("adb pull stderr: {}", stderr);
    }

    if output.status.success() {
        // Verify the file was pulled to the correct location
        if absolute_path.exists() {
            let file_size = std::fs::metadata(&absolute_path)?.len();
            debug!("File successfully pulled to: {} ({} bytes)", absolute_path_str, file_size);
            Ok(absolute_path_str.to_string())
        } else {
            // Debug: check what files exist in tmp_dir
            let original_filename = std::path::Path::new(file_path)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("");
            let original_in_tmp = std::path::PathBuf::from(tmp_dir).join(original_filename);

            debug!("File not at expected path. Checking tmp_dir contents:");
            if let Ok(entries) = std::fs::read_dir(tmp_dir) {
                for entry in entries.flatten() {
                    debug!("  Found: {:?}", entry.path());
                }
            }
            debug!("Checking if original filename exists at: {:?} -> {}", original_in_tmp, original_in_tmp.exists());

            let err_msg = format!(
                "adb pull reported success but file does not exist at {}. stdout: {}, stderr: {}",
                absolute_path_str, stdout, stderr
            );
            error!("{}", err_msg);
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, err_msg))
        }
    } else {
        let err = format!("adb pull failed: {} {}", stderr, stdout);
        error!("{}", err);
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

#[allow(dead_code)]
pub fn install_existing_app(package_name: &str, device: &str) -> std::io::Result<String> {
    use std::process::Command;
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("cmd")
        .arg("package")
        .arg("install-existing")
        .arg(package_name)
        .output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

pub fn usagestats_history(device: &str) -> std::io::Result<String> {
    use std::process::Command;
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("dumpsys")
        .arg("usagestats")
        .arg("-history")
        .output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

#[allow(dead_code)]
pub fn extract_apk(device: &str, user_id: Option<&str>) -> std::io::Result<String> {
    use std::process::Command;
    let user = user_id.unwrap_or("0");
    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("pm")
        .arg("path")
        .arg("--user")
        .arg(user)
        .output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

#[allow(dead_code)]
pub fn kill_server() -> std::io::Result<String> {
    use std::process::Command;
    let output = Command::new("adb").arg("kill-server").output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

#[allow(dead_code)]
pub fn root_get_permission() -> std::io::Result<String> {
    use std::process::Command;
    let output = Command::new("adb").arg("root").output()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

/// Get device CPU ABI list (e.g., "arm64-v8a,armeabi-v7a,armeabi")
/// Returns a vector of supported ABIs in priority order
pub fn get_cpu_abi_list(device: &str) -> std::io::Result<Vec<String>> {
    use std::process::Command;
    debug!("Getting CPU ABI list for device: {}", device);

    let output = Command::new("adb")
        .arg("-s")
        .arg(device)
        .arg("shell")
        .arg("getprop")
        .arg("ro.product.cpu.abilist")
        .output()?;

    if output.status.success() {
        let abi_list = String::from_utf8_lossy(&output.stdout).to_string();
        let abis: Vec<String> = abi_list
            .trim()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        debug!("Device ABIs: {:?}", abis);
        Ok(abis)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        error!("Failed to get CPU ABI list: {}", err);
        Err(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

// adb
// Mac https://dl.google.com/android/repository/platform-tools-latest-darwin.zip
// Linux https://dl.google.com/android/repository/platform-tools-latest-linux.zip
// Windows https://dl.google.com/android/repository/platform-tools-latest-windows.zip

// ============================= https://gist.github.com/Pulimet/5013acf2cd5b28e55036c82c91bd56d8
// adb help // List all comands

// == Adb Server
// adb kill-server
// adb start-server

// == Adb Reboot
// adb reboot
// adb reboot recovery
// adb reboot-bootloader
// adb root //restarts adb with root permissions

// == Shell
// adb shell    // Open or run commands in a terminal on the host Android device.

// == Devices
// adb usb
// adb devices   //show devices attached
// adb devices -l //devices (product/model)
// adb connect ip_address_of_device

// == Get device android version
// adb shell getprop ro.build.version.release

// == LogCat
// adb logcat
// adb logcat -c // clear // The parameter -c will clear the current logs on the device.
// adb logcat -d > [path_to_file] // Save the logcat output to a file on the local system.
// adb bugreport > [path_to_file] // Will dump the whole device information like dumpstate, dumpsys and logcat output.

// == Files
// adb push [source] [destination]    // Copy files from your computer to your phone.
// adb pull [device file location] [local file location] // Copy files from your phone to your computer.

// == App install
// adb -e install path/to/app.apk

// -d                        - directs command to the only connected USB device...
// -e                        - directs command to the only running emulator...
// -s <serial number>        ...
// -p <product name or path> ...
// The flag you decide to use has to come before the actual adb command:

// adb devices | tail -n +2 | cut -sf 1 | xargs -IX adb -s X install -r com.myAppPackage // Install the given app on all connected devices.

// == Uninstalling app from device
// adb uninstall com.myAppPackage
// adb uninstall <app .apk name>
// adb uninstall -k <app .apk name> -> "Uninstall .apk withour deleting data"

// adb shell pm uninstall com.example.MyApp
// adb shell pm clear [package] // Deletes all data associated with a package.

// adb devices | tail -n +2 | cut -sf 1 | xargs -IX adb -s X uninstall com.myAppPackage //Uninstall the given app from all connected devices

// == Update app
// adb install -r yourApp.apk  //  -r means re-install the app and keep its data on the device.
// adb install –k <.apk file path on computer>

// == Home button
// adb shell am start -W -c android.intent.category.HOME -a android.intent.action.MAIN

// == Activity Manager
// adb shell am start -a android.intent.action.VIEW
// adb shell am broadcast -a 'my_action'

// adb shell am start -a android.intent.action.CALL -d tel:+972527300294 // Make a call

// // Open send sms screen with phone number and the message:
// adb shell am start -a android.intent.action.SENDTO -d sms:+972527300294   --es  sms_body "Test --ez exit_on_sent false

// // Reset permissions
// adb shell pm reset-permissions -p your.app.package
// adb shell pm grant [packageName] [ Permission]  // Grant a permission to an app.
// adb shell pm revoke [packageName] [ Permission]   // Revoke a permission from an app.

// // Emulate device
// adb shell wm size 2048x1536
// adb shell wm density 288
// // And reset to default
// adb shell wm size reset
// adb shell wm density reset

// == Print text
// adb shell input text 'Wow, it so cool feature'

// == Screenshot
// adb shell screencap -p /sdcard/screenshot.png

// $ adb shell
// shell@ $ screencap /sdcard/screen.png
// shell@ $ exit
// $ adb pull /sdcard/screen.png

// ---
// adb shell screenrecord /sdcard/NotAbleToLogin.mp4

// $ adb shell
// shell@ $ screenrecord --verbose /sdcard/demo.mp4
// (press Control + C to stop)
// shell@ $ exit
// $ adb pull /sdcard/demo.mp4

// == Key event
// adb shell input keyevent 3 // Home btn
// adb shell input keyevent 4 // Back btn
// adb shell input keyevent 5 // Call
// adb shell input keyevent 6 // End call
// adb shell input keyevent 26  // Turn Android device ON and OFF. It will toggle device to on/off status.
// adb shell input keyevent 27 // Camera
// adb shell input keyevent 64 // Open browser
// adb shell input keyevent 66 // Enter
// adb shell input keyevent 67 // Delete (backspace)
// adb shell input keyevent 207 // Contacts
// adb shell input keyevent 220 / 221 // Brightness down/up
// adb shell input keyevent 277 / 278 /279 // Cut/Copy/Paste

// 0 -->  "KEYCODE_0"
// 1 -->  "KEYCODE_SOFT_LEFT"
// 2 -->  "KEYCODE_SOFT_RIGHT"
// 3 -->  "KEYCODE_HOME"
// 4 -->  "KEYCODE_BACK"
// 5 -->  "KEYCODE_CALL"
// 6 -->  "KEYCODE_ENDCALL"
// 7 -->  "KEYCODE_0"
// 8 -->  "KEYCODE_1"
// 9 -->  "KEYCODE_2"
// 10 -->  "KEYCODE_3"
// 11 -->  "KEYCODE_4"
// 12 -->  "KEYCODE_5"
// 13 -->  "KEYCODE_6"
// 14 -->  "KEYCODE_7"
// 15 -->  "KEYCODE_8"
// 16 -->  "KEYCODE_9"
// 17 -->  "KEYCODE_STAR"
// 18 -->  "KEYCODE_POUND"
// 19 -->  "KEYCODE_DPAD_UP"
// 20 -->  "KEYCODE_DPAD_DOWN"
// 21 -->  "KEYCODE_DPAD_LEFT"
// 22 -->  "KEYCODE_DPAD_RIGHT"
// 23 -->  "KEYCODE_DPAD_CENTER"
// 24 -->  "KEYCODE_VOLUME_UP"
// 25 -->  "KEYCODE_VOLUME_DOWN"
// 26 -->  "KEYCODE_POWER"
// 27 -->  "KEYCODE_CAMERA"
// 28 -->  "KEYCODE_CLEAR"
// 29 -->  "KEYCODE_A"
// 30 -->  "KEYCODE_B"
// 31 -->  "KEYCODE_C"
// 32 -->  "KEYCODE_D"
// 33 -->  "KEYCODE_E"
// 34 -->  "KEYCODE_F"
// 35 -->  "KEYCODE_G"
// 36 -->  "KEYCODE_H"
// 37 -->  "KEYCODE_I"
// 38 -->  "KEYCODE_J"
// 39 -->  "KEYCODE_K"
// 40 -->  "KEYCODE_L"
// 41 -->  "KEYCODE_M"
// 42 -->  "KEYCODE_N"
// 43 -->  "KEYCODE_O"
// 44 -->  "KEYCODE_P"
// 45 -->  "KEYCODE_Q"
// 46 -->  "KEYCODE_R"
// 47 -->  "KEYCODE_S"
// 48 -->  "KEYCODE_T"
// 49 -->  "KEYCODE_U"
// 50 -->  "KEYCODE_V"
// 51 -->  "KEYCODE_W"
// 52 -->  "KEYCODE_X"
// 53 -->  "KEYCODE_Y"
// 54 -->  "KEYCODE_Z"
// 55 -->  "KEYCODE_COMMA"
// 56 -->  "KEYCODE_PERIOD"
// 57 -->  "KEYCODE_ALT_LEFT"
// 58 -->  "KEYCODE_ALT_RIGHT"
// 59 -->  "KEYCODE_SHIFT_LEFT"
// 60 -->  "KEYCODE_SHIFT_RIGHT"
// 61 -->  "KEYCODE_TAB"
// 62 -->  "KEYCODE_SPACE"
// 63 -->  "KEYCODE_SYM"
// 64 -->  "KEYCODE_EXPLORER"
// 65 -->  "KEYCODE_ENVELOPE"
// 66 -->  "KEYCODE_ENTER"
// 67 -->  "KEYCODE_DEL"
// 68 -->  "KEYCODE_GRAVE"
// 69 -->  "KEYCODE_MINUS"
// 70 -->  "KEYCODE_EQUALS"
// 71 -->  "KEYCODE_LEFT_BRACKET"
// 72 -->  "KEYCODE_RIGHT_BRACKET"
// 73 -->  "KEYCODE_BACKSLASH"
// 74 -->  "KEYCODE_SEMICOLON"
// 75 -->  "KEYCODE_APOSTROPHE"
// 76 -->  "KEYCODE_SLASH"
// 77 -->  "KEYCODE_AT"
// 78 -->  "KEYCODE_NUM"
// 79 -->  "KEYCODE_HEADSETHOOK"
// 80 -->  "KEYCODE_FOCUS"
// 81 -->  "KEYCODE_PLUS"
// 82 -->  "KEYCODE_MENU"
// 83 -->  "KEYCODE_NOTIFICATION"
// 84 -->  "KEYCODE_SEARCH"
// 85 -->  "KEYCODE_MEDIA_PLAY_PAUSE"
// 86 -->  "KEYCODE_MEDIA_STOP"
// 87 -->  "KEYCODE_MEDIA_NEXT"
// 88 -->  "KEYCODE_MEDIA_PREVIOUS"
// 89 -->  "KEYCODE_MEDIA_REWIND"
// 90 -->  "KEYCODE_MEDIA_FAST_FORWARD"
// 91 -->  "KEYCODE_MUTE"
// 92 -->  "KEYCODE_PAGE_UP"
// 93 -->  "KEYCODE_PAGE_DOWN"
// 94 -->  "KEYCODE_PICTSYMBOLS"
// ...
// 122 -->  "KEYCODE_MOVE_HOME"
// 123 -->  "KEYCODE_MOVE_END"
// // https://developer.android.com/reference/android/view/KeyEvent.html

// == ShPref
// # replace org.example.app with your application id

// # Add a value to default shared preferences.
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key key_name --es value "hello world!"'

// # Remove a value to default shared preferences.
// adb shell 'am broadcast -a org.example.app.sp.REMOVE --es key key_name'

// # Clear all default shared preferences.
// adb shell 'am broadcast -a org.example.app.sp.CLEAR --es key key_name'

// # It's also possible to specify shared preferences file.
// adb shell 'am broadcast -a org.example.app.sp.PUT --es name Game --es key level --ei value 10'

// # Data types
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key string --es value "hello world!"'
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key boolean --ez value true'
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key float --ef value 3.14159'
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key int --ei value 2015'
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key long --el value 9223372036854775807'

// # Restart application process after making changes
// adb shell 'am broadcast -a org.example.app.sp.CLEAR --ez restart true'

// == Monkey
// adb shell monkey -p com.myAppPackage -v 10000 -s 100 // monkey tool is generating 10.000 random events on the real device

// == Paths
// /data/data/<package>/databases (app databases)
// /data/data/<package>/shared_prefs/ (shared preferences)
// /data/app (apk installed by user)
// /system/app (pre-installed APK files)
// /mmt/asec (encrypted apps) (App2SD)
// /mmt/emmc (internal SD Card)
// /mmt/adcard (external/Internal SD Card)
// /mmt/adcard/external_sd (external SD Card)

// adb shell ls (list directory contents)
// adb shell ls -s (print size of each file)
// adb shell ls -R (list subdirectories recursively)

// == Device onformation
// adb get-statе (print device state)
// adb get-serialno (get the serial number)
// adb shell dumpsys iphonesybinfo (get the IMEI)
// adb shell netstat (list TCP connectivity)
// adb shell pwd (print current working directory)
// adb shell dumpsys battery (battery status)
// adb shell pm list features (list phone features)
// adb shell service list (list all services)
// adb shell dumpsys activity <package>/<activity> (activity info)
// adb shell ps (print process status)
// adb shell wm size (displays the current screen resolution)
// dumpsys window windows | grep -E 'mCurrentFocus|mFocusedApp' (print current app's opened activity)

// == Package info
// adb shell pm list packages
// adb shell dumpsys package com.android.telephony.imsmedia
// adb shell list packages (list package names)
// adb shell list packages -r (list package name + path to apks)
// adb shell list packages -3 (list third party package names)
// adb shell list packages -s (list only system packages)
// adb shell list packages -u (list package names + uninstalled)
// adb shell dumpsys package packages (list info on all apps)
// adb shell dump <name> (list info on one package)
// adb shell path <package> (path to the apk file)
// adb shell dumpsys usagestats -history (app usage statistics)

// ==Configure Settings Commands
// adb shell dumpsys battery set level <n> (change the level from 0 to 100)
// adb shell dumpsys battery set status<n> (change the level to unknown, charging, discharging, not charging or full)
// adb shell dumpsys battery reset (reset the battery)
// adb shell dumpsys battery set usb <n> (change the status of USB connection. ON or OFF)
// adb shell wm size WxH (sets the resolution to WxH)

// == Device Related Commands
// adb reboot-recovery (reboot device into recovery mode)
// adb reboot fastboot (reboot device into recovery mode)
// adb shell screencap -p "/path/to/screenshot.png" (capture screenshot)
// adb shell screenrecord "/path/to/record.mp4" (record device screen)
// adb backup -apk -all -f backup.ab (backup settings and apps)
// adb backup -apk -shared -all -f backup.ab (backup settings, apps and shared storage)
// adb backup -apk -nosystem -all -f backup.ab (backup only non-system apps)
// adb restore backup.ab (restore a previous backup)
// adb shell am start|startservice|broadcast <INTENT>[<COMPONENT>]
// -a <ACTION> e.g. android.intent.action.VIEW
// -c <CATEGORY> e.g. android.intent.category.LAUNCHER (start activity intent)

// adb shell am start -a android.intent.action.VIEW -d URL (open URL)
// adb shell am start -t image/* -a android.intent.action.VIEW (opens gallery)

// == Logs
// adb logcat [options] [filter] [filter] (view device log)
// adb bugreport (print bug reports)

// == Other
// adb backup // Create a full backup of your phone and save to the computer.
// adb restore // Restore a backup to your phone.
// adb sideload //  Push and flash custom ROMs and zips from your computer.

// fastboot devices
// // Check connection and get basic information about devices connected to the computer.
// // This is essentially the same command as adb devices from earlier.
// //However, it works in the bootloader, which ADB does not. Handy for ensuring that you have properly established a connection.

// --------------------------------------------------------------------------------
// Shared Preferences

// # replace org.example.app with your application id

// # Add a value to default shared preferences.
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key key_name --es value "hello world!"'

// # Remove a value to default shared preferences.
// adb shell 'am broadcast -a org.example.app.sp.REMOVE --es key key_name'

// # Clear all default shared preferences.
// adb shell 'am broadcast -a org.example.app.sp.CLEAR --es key key_name'

// # It's also possible to specify shared preferences file.
// adb shell 'am broadcast -a org.example.app.sp.PUT --es name Game --es key level --ei value 10'

// # Data types
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key string --es value "hello world!"'
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key boolean --ez value true'
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key float --ef value 3.14159'
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key int --ei value 2015'
// adb shell 'am broadcast -a org.example.app.sp.PUT --es key long --el value 9223372036854775807'

// # Restart application process after making changes
// adb shell 'am broadcast -a org.example.app.sp.CLEAR --ez restart true'
// --------------------------------------------------------------------------------

// === Few bash snippets ===
// @Source (https://jonfhancock.com/bash-your-way-to-better-android-development-1169bc3e0424)

// === Using tail -n
// //Use tail to remove the first line. Actually two lines. The first one is just a newline. The second is “List of devices attached.”
// $ adb devices | tail -n +2

// === Using cut -sf
// // Cut the last word and any white space off the end of each line.
// $ adb devices | tail -n +2 | cut -sf -1

// === Using xargs -I
// // Given the -I option, xargs will perform an action for each line of text that we feed into it.
// // We can give the line a variable name to use in commands that xargs can execute.
// $ adb devices | tail -n +2 | cut -sf -1 | xargs -I X echo X aw yiss

// === Three options below together
// // Will print android version of all connected devices
// adb devices | tail -n +2 | cut -sf -1 | xargs -I X adb -s X shell getprop ro.build.version.release

// === Using alias
// -- Example 1
// alias tellMeMore=echo
// tellMeMore "hi there"
// Output => hi there
// -- Example 2
// // Define alias
// alias apkinstall="adb devices | tail -n +2 | cut -sf 1 | xargs -I X adb -s X install -r $1"
// // And you can use it later
// apkinstall ~/Downloads/MyAppRelease.apk  // Install an apk on all devices
// -- Example 3
// alias rmapp="adb devices | tail -n +2 | cut -sf 1 | xargs -I X adb -s X uninstall $1"
// rmapp com.example.myapp // Uninstall a package from all devices
// -- Example 4
// alias clearapp="adb devices | tail -n +2 | cut -sf 1 | xargs -I X adb -s X shell pm clear $1"
// clearapp com.example.myapp  // Clear data on all devices (leave installed)
// -- Example 5
// alias startintent="adb devices | tail -n +2 | cut -sf 1 | xargs -I X adb -s X shell am start $1"
// startintent https://twitter.com/JonFHancock // Launch a deep link on all devices

// Setting up your .bash_profile
// Finally, to make this all reusable even after rebooting your computer (aliases only last through the current session), we have to add these to your .bash_profile. You might or might not already have a .bash_profile, so let’s make sure we append to it rather than overwriting it. Just open a terminal, and run the following command

// touch .bash_profile && open .bash_profile

// This will create it if it doesn’t already exist, and open it in a text editor either way. Now just copy and paste all of the aliases into it, save, and close.

// alias startintent="adb devices | tail -n +2 | cut -sf 1 | xargs -I X adb -s X shell am start $1"
// alias apkinstall="adb devices | tail -n +2 | cut -sf 1 | xargs -I X adb -s X install -r $1"
// alias rmapp="adb devices | tail -n +2 | cut -sf 1 | xargs -I X adb -s X uninstall $1"
// alias clearapp="adb devices | tail -n +2 | cut -sf 1 | xargs -I X adb -s X shell pm clear $1"
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    // NOTE: This test is commented out because parse_package_info function is commented out
    // #[test]
    // fn test_parse_reference_dump() {
    //     // Locate the reference file relative to the crate root
    //     let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    //     path.pop(); // Go up to workspace root
    //     path.push("reference");
    //     path.push("adb_dumpsys_package_packages");
    //
    //     println!("Reading reference file from: {:?}", path);
    //     let content = fs::read_to_string(&path).expect("Failed to read reference file");
    //
    //     let packages = parse_package_info(&content);
    //     println!("Parsed {} packages", packages.len());
    //
    //     // Verify we parsed some packages
    //     assert!(!packages.is_empty(), "Should parse at least one package");
    //
    //     // Find specific packages to verify permissions
    //
    //     // 1. com.google.android.apps.carrier.carrierwifi (System app with install permissions)
    //     let carrier_wifi = packages
    //         .iter()
    //         .find(|p| p.pkg == "com.google.android.apps.carrier.carrierwifi")
    //         .expect("Should find carrierwifi package");
    //
    //     println!(
    //         "CarrierWifi install permissions:\n{}",
    //         carrier_wifi.installPermissions
    //     );
    //     assert!(
    //         !carrier_wifi.installPermissions.is_empty(),
    //         "CarrierWifi should have install permissions"
    //     );
    //     assert!(
    //         carrier_wifi
    //             .installPermissions
    //             .contains("android.permission.INTERNET"),
    //         "Should contain INTERNET permission"
    //     );
    //     assert!(
    //         carrier_wifi.installPermissions.contains("granted=true"),
    //         "Should contain granted status"
    //     );
    //
    //     // Check runtime permissions for user 0
    //     let user0 = carrier_wifi
    //         .users
    //         .iter()
    //         .find(|u| u.userId == 0)
    //         .expect("Should find user 0");
    //     println!(
    //         "CarrierWifi user 0 runtime permissions: {:?}",
    //         user0.runtimePermissions
    //     );
    //     assert!(
    //         !user0.runtimePermissions.is_empty(),
    //         "CarrierWifi should have runtime permissions"
    //     );
    //     assert!(
    //         user0.runtimePermissions[0].contains("android.permission.ACTIVITY_RECOGNITION"),
    //         "Should contain ACTIVITY_RECOGNITION"
    //     );
    //
    //     // 2. com.google.android.apps.aiwallpapers (App with both install and runtime permissions)
    //     let wallpaper = packages
    //         .iter()
    //         .find(|p| p.pkg == "com.google.android.apps.aiwallpapers")
    //         .expect("Should find aiwallpapers package");
    //
    //     println!(
    //         "Wallpaper install permissions:\n{}",
    //         wallpaper.installPermissions
    //     );
    //     assert!(
    //         !wallpaper.installPermissions.is_empty(),
    //         "Wallpaper should have install permissions"
    //     );
    //     assert!(
    //         wallpaper
    //             .installPermissions
    //             .contains("android.permission.SET_WALLPAPER"),
    //         "Should contain SET_WALLPAPER"
    //     );
    //
    //     let user0 = wallpaper
    //         .users
    //         .iter()
    //         .find(|u| u.userId == 0)
    //         .expect("Should find user 0");
    //     println!(
    //         "Wallpaper user 0 runtime permissions: {:?}",
    //         user0.runtimePermissions
    //     );
    //     assert!(
    //         !user0.runtimePermissions.is_empty(),
    //         "Wallpaper should have runtime permissions"
    //     );
    //     assert!(
    //         user0
    //             .runtimePermissions
    //             .iter()
    //             .any(|p: &String| p.contains("android.permission.GET_ACCOUNTS")),
    //         "Should contain GET_ACCOUNTS"
    //     );
    // }

    #[test]
    fn test_parse_package_fingerprints() {
        // Locate the reference file relative to the crate root
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop(); // Go up to workspace root
        path.push("reference");
        path.push("adb_dumpsys_package_packages");

        println!("Reading reference file from: {:?}", path);
        let content = fs::read_to_string(&path).expect("Failed to read reference file");

        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let fingerprints = parse_package_fingerprints(lines);
        println!("Parsed {} fingerprints", fingerprints.len());

        assert!(
            !fingerprints.is_empty(),
            "Should parse at least one fingerprint"
        );

        // 1. com.google.android.apps.carrier.carrierwifi
        let carrier_wifi = fingerprints
            .iter()
            .find(|p| p.pkg == "com.google.android.apps.carrier.carrierwifi")
            .expect("Should find carrierwifi fingerprint");

        println!(
            "CarrierWifi install permissions (fingerprint):\n{:?}",
            carrier_wifi.installPermissions
        );
        assert!(
            !carrier_wifi.installPermissions.is_empty(),
            "CarrierWifi should have install permissions"
        );
        assert!(
            carrier_wifi
                .installPermissions
                .iter()
                .any(|p| p.contains("android.permission.INTERNET")),
            "Should contain INTERNET permission"
        );

        // Runtime permissions check (Fingerprint struct includes users)
        let user0 = carrier_wifi
            .users
            .iter()
            .find(|u| u.userId == 0)
            .expect("Should find user 0");
        println!(
            "CarrierWifi user 0 runtime permissions (fingerprint): {:?}",
            user0.runtimePermissions
        );
        assert!(
            !user0.runtimePermissions.is_empty(),
            "CarrierWifi should have runtime permissions"
        );
        assert!(
            user0
                .runtimePermissions
                .iter()
                .any(|p| p.contains("android.permission.ACTIVITY_RECOGNITION")),
            "Should contain ACTIVITY_RECOGNITION"
        );

        // 2. com.google.android.apps.aiwallpapers
        let wallpaper = fingerprints
            .iter()
            .find(|p| p.pkg == "com.google.android.apps.aiwallpapers")
            .expect("Should find aiwallpapers fingerprint");

        println!(
            "Wallpaper install permissions (fingerprint):\n{:?}",
            wallpaper.installPermissions
        );
        assert!(
            !wallpaper.installPermissions.is_empty(),
            "Wallpaper should have install permissions"
        );

        // Note: install permissions in PackageFingerprint are Vec<String>, not String joined by \n
        assert!(
            wallpaper
                .installPermissions
                .iter()
                .any(|p| p.contains("android.permission.SET_WALLPAPER")),
            "Should contain SET_WALLPAPER"
        );

        let user0 = wallpaper
            .users
            .iter()
            .find(|u| u.userId == 0)
            .expect("Should find user 0");
        println!(
            "Wallpaper user 0 runtime permissions (fingerprint): {:?}",
            user0.runtimePermissions
        );
        assert!(
            !user0.runtimePermissions.is_empty(),
            "Wallpaper should have runtime permissions"
        );
        assert!(
            user0
                .runtimePermissions
                .iter()
                .any(|p| p.contains("android.permission.GET_ACCOUNTS")),
            "Should contain GET_ACCOUNTS"
        );
    }
    #[test]
    fn test_parse_package_fingerprints_user_flags() {
        let lines = vec![
            "Package [com.example.app] (deadbeef)".to_string(),
            "    userId=10060".to_string(),
            "    codePath=/data/app/com.example.app-1".to_string(),
            "    versionCode=123".to_string(),
            "    versionName=1.2.3".to_string(),
            "    lastUpdateTime=2023-01-01 12:00:00".to_string(),
            "    User 0: ceDataInode=123 installed=true hidden=false suspended=false stopped=true enabled=0".to_string(),
            "    User 10: ceDataInode=456 installed=false hidden=false suspended=false stopped=false enabled=3".to_string(),
        ];

        let fingerprints = parse_package_fingerprints(lines);
        assert_eq!(fingerprints.len(), 1);
        let pkg = &fingerprints[0];

        // Check User 0
        let user0 = pkg
            .users
            .iter()
            .find(|u| u.userId == 0)
            .expect("Should have user 0");
        assert!(user0.installed, "User 0 should be installed");
        assert!(user0.stopped, "User 0 should be stopped");
        assert_eq!(user0.enabled, 0, "User 0 should be enabled (state 0)");

        // Check User 10
        let user10 = pkg
            .users
            .iter()
            .find(|u| u.userId == 10)
            .expect("Should have user 10");
        assert!(!user10.installed, "User 10 should not be installed");
        assert_eq!(
            user10.enabled, 3,
            "User 10 should be disabled user (state 3)"
        );
    }

    #[test]
    #[ignore] // This test requires actual adb device connection
    fn test_pull_file_to_temp_velvet_apk() {
        // This test simulates the real scenario from the logs:
        // - Device: 43151JEKB07226
        // - File: /product/priv-app/Velvet/Velvet.apk
        // - Package: com.google.android.googlequicksearchbox
        // - Expected: /home/spot/.config/uad_shizuku/tmp/com_google_android_googlequicksearchbox.apk
        
        let device_serial = "43151JEKB07226";
        let file_path = "/product/priv-app/Velvet/Velvet.apk";
        let tmp_dir = "/tmp/test_pull_file";
        let package_id = "com.google.android.googlequicksearchbox";
        
        // Create tmp directory
        std::fs::create_dir_all(tmp_dir).expect("Failed to create tmp dir");
        
        // Attempt to pull file
        let result = pull_file_to_temp(device_serial, file_path, tmp_dir, package_id);
        
        match result {
            Ok(pulled_path) => {
                println!("File successfully pulled to: {}", pulled_path);
                
                // Verify the file exists
                assert!(std::path::Path::new(&pulled_path).exists(), 
                    "File should exist at pulled path: {}", pulled_path);
                
                // Verify it's the expected path
                let expected_path = format!("{}/com_google_android_googlequicksearchbox.apk", tmp_dir);
                assert_eq!(pulled_path, expected_path, 
                    "Pulled path should match expected path");
                
                // Clean up
                let _ = std::fs::remove_file(&pulled_path);
            }
            Err(e) => {
                println!("Pull failed with error: {}", e);
                
                // Check if file was pulled with original device filename
                let original_name_path = format!("{}/Velvet.apk", tmp_dir);
                if std::path::Path::new(&original_name_path).exists() {
                    println!("ISSUE FOUND: File was pulled with original device name: {}", original_name_path);
                    println!("Expected: {}/com_google_android_googlequicksearchbox.apk", tmp_dir);
                    
                    // Clean up
                    let _ = std::fs::remove_file(&original_name_path);
                    
                    panic!("File naming mismatch: adb pulled file with original device name (Velvet.apk) instead of package-based name (com_google_android_googlequicksearchbox.apk)");
                } else {
                    // File doesn't exist anywhere
                    panic!("File pull failed and file not found: {}", e);
                }
            }
        }
        
        // Clean up directory
        let _ = std::fs::remove_dir(tmp_dir);
    }
}

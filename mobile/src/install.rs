//! Installation management for desktop platforms (Windows/Linux/macOS)
//!
//! Provides functionality for:
//! - Checking installation status
//! - Installing/uninstalling the application
//! - Checking for updates
//! - Downloading and applying updates

use crate::install_stt::{GitHubRelease, InstallPaths, InstallResult, InstallStatus, UpdateInfo};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const APP_NAME: &str = "uad-shizuku";
const GITHUB_REPO: &str = "nikescar/uad-shizuku";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get versioned app name (e.g., "uad-shizuku-1.0.0")
fn get_versioned_app_name() -> String {
    format!("{}-{}", APP_NAME, CURRENT_VERSION)
}

/// Move a file or directory to trash (cross-platform)
fn move_to_trash<P: AsRef<Path>>(path: P) -> Result<(), String> {
    let path = path.as_ref();

    log::debug!("Moving to trash: {}", path.display());

    if !path.exists() {
        log::debug!("Path does not exist, nothing to move");
        return Ok(()); // Nothing to move
    }

    // Use the trash crate which properly handles all platforms:
    // - Windows: Uses IFileOperation COM interface (Recycle Bin)
    // - macOS: Uses FSMoveObjectToTrashSync (Finder Trash)
    // - Linux: Uses freedesktop.org Trash specification
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    {
        trash::delete(path)
            .map_err(|e| {
                let err_msg = format!("Failed to move to trash: {}", e);
                log::error!("{}", err_msg);
                err_msg
            })?;

        log::debug!("Successfully moved to trash");
    }

    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    {
        // On Android/WASM, just delete the file
        log::debug!("Platform does not support trash, deleting file directly");
        if path.is_dir() {
            fs::remove_dir_all(path)
                .map_err(|e| format!("Failed to remove directory: {}", e))?;
        } else {
            fs::remove_file(path)
                .map_err(|e| format!("Failed to remove file: {}", e))?;
        }
    }

    Ok(())
}

/// Remove old versions of binaries and shortcuts
/// If `keep_version` is provided, keep that version instead of the current running version
fn cleanup_old_installations(paths: &InstallPaths, keep_version: Option<&str>) -> Result<(), String> {
    let version_to_keep = if let Some(ver) = keep_version {
        format!("{}-{}", APP_NAME, ver)
    } else {
        get_versioned_app_name()
    };

    log::info!("Cleaning up old installations, keeping: {}", version_to_keep);

    // Clean up old binaries in bin_dir
    if paths.bin_dir.exists() {
        log::debug!("Scanning bin directory: {}", paths.bin_dir.display());
        if let Ok(entries) = fs::read_dir(&paths.bin_dir) {
            let mut removed_count = 0;
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = entry.file_name();
                let name_str = file_name.to_string_lossy();

                // Check if it's an old version of our app
                #[cfg(target_os = "windows")]
                let is_old_binary = name_str.starts_with(&format!("{}-", APP_NAME))
                    && name_str.ends_with(".exe")
                    && !name_str.starts_with(&version_to_keep);

                #[cfg(not(target_os = "windows"))]
                let is_old_binary = name_str.starts_with(&format!("{}-", APP_NAME))
                    && !name_str.starts_with(&version_to_keep)
                    && !name_str.contains("-bin"); // Don't remove the -bin helper on macOS

                if is_old_binary {
                    log::info!("Moving old binary to trash: {}", path.display());
                    match move_to_trash(&path) {
                        Ok(_) => {
                            removed_count += 1;
                            log::debug!("Successfully removed: {}", name_str);
                        }
                        Err(e) => log::warn!("Failed to remove old binary {}: {}", name_str, e),
                    }
                }
            }
            if removed_count > 0 {
                log::info!("Removed {} old binary(ies)", removed_count);
            } else {
                log::debug!("No old binaries found to remove");
            }
        } else {
            log::warn!("Could not read bin directory");
        }
    } else {
        log::debug!("Bin directory does not exist: {}", paths.bin_dir.display());
    }

    // Clean up old shortcuts
    #[cfg(not(target_os = "macos"))]
    {
        log::debug!("Cleaning up old shortcuts...");
        let mut shortcuts_removed = 0;

        // Clean start menu shortcuts
        if let Some(ref start_menu) = paths.start_menu_entry {
            if let Some(parent) = start_menu.parent() {
                if parent.exists() {
                    log::debug!("Scanning start menu directory: {}", parent.display());
                    if let Ok(entries) = fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let file_name = entry.file_name();
                            let name_str = file_name.to_string_lossy();

                            #[cfg(target_os = "linux")]
                            let is_old_shortcut = name_str == format!("{}.desktop", APP_NAME);

                            #[cfg(target_os = "windows")]
                            let is_old_shortcut = name_str == format!("{}.lnk", APP_NAME);

                            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
                            let is_old_shortcut = false;

                            if is_old_shortcut && path != *start_menu {
                                log::info!("Moving old start menu shortcut to trash: {}", path.display());
                                match move_to_trash(&path) {
                                    Ok(_) => {
                                        shortcuts_removed += 1;
                                        log::debug!("Successfully removed shortcut: {}", name_str);
                                    }
                                    Err(e) => log::warn!("Failed to remove shortcut {}: {}", name_str, e),
                                }
                            }
                        }
                    }
                }
            }
        }

        // Clean desktop shortcuts
        if let Some(ref desktop) = paths.desktop_shortcut {
            if let Some(parent) = desktop.parent() {
                if parent.exists() {
                    log::debug!("Scanning desktop directory: {}", parent.display());
                    if let Ok(entries) = fs::read_dir(parent) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let file_name = entry.file_name();
                            let name_str = file_name.to_string_lossy();

                            #[cfg(target_os = "linux")]
                            let is_old_shortcut = name_str == format!("{}.desktop", APP_NAME);

                            #[cfg(target_os = "windows")]
                            let is_old_shortcut = name_str == format!("{}.lnk", APP_NAME);

                            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
                            let is_old_shortcut = false;

                            if is_old_shortcut && path != *desktop {
                                log::info!("Moving old desktop shortcut to trash: {}", path.display());
                                match move_to_trash(&path) {
                                    Ok(_) => {
                                        shortcuts_removed += 1;
                                        log::debug!("Successfully removed shortcut: {}", name_str);
                                    }
                                    Err(e) => log::warn!("Failed to remove shortcut {}: {}", name_str, e),
                                }
                            }
                        }
                    }
                }
            }
        }

        if shortcuts_removed > 0 {
            log::info!("Removed {} old shortcut(s)", shortcuts_removed);
        } else {
            log::debug!("No old shortcuts found to remove");
        }
    }

    // Clean up old macOS app bundles
    #[cfg(target_os = "macos")]
    {
        log::debug!("Cleaning up old macOS app bundles...");
        if paths.bin_dir.exists() {
            if let Ok(entries) = fs::read_dir(&paths.bin_dir) {
                let mut removed_count = 0;
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();

                    let is_old_app = name_str.starts_with(&format!("{}-", APP_NAME))
                        && name_str.ends_with(".app")
                        && !name_str.starts_with(&version_to_keep);

                    if is_old_app {
                        log::info!("Moving old app bundle to trash: {}", path.display());
                        match move_to_trash(&path) {
                            Ok(_) => {
                                removed_count += 1;
                                log::debug!("Successfully removed: {}", name_str);
                            }
                            Err(e) => log::warn!("Failed to remove old app bundle {}: {}", name_str, e),
                        }
                    }
                }
                if removed_count > 0 {
                    log::info!("Removed {} old app bundle(s)", removed_count);
                } else {
                    log::debug!("No old app bundles found to remove");
                }
            }
        }
    }

    log::info!("Cleanup completed");
    Ok(())
}

/// Get platform-specific installation paths
pub fn get_install_paths() -> InstallPaths {
    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        InstallPaths {
            bin_dir: home.join(".local").join("bin"),
            desktop_shortcut: Some(home.join("Desktop").join(format!("{}.desktop", APP_NAME))),
            start_menu_entry: Some(
                home.join(".local")
                    .join("share")
                    .join("applications")
                    .join(format!("{}.desktop", APP_NAME)),
            ),
            uninstall_key: None,
        }
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        InstallPaths {
            bin_dir: home.join("Applications"),
            desktop_shortcut: None, // macOS doesn't use desktop shortcuts
            start_menu_entry: None, // App bundle is self-contained
            uninstall_key: None,
        }
    }

    #[cfg(target_os = "windows")]
    {
        let user_profile = dirs::home_dir().unwrap_or_else(|| PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\Default".to_string())));
        let local_app_data =
            dirs::data_local_dir().unwrap_or_else(|| user_profile.join("AppData").join("Local"));
        let start_menu = dirs::data_dir()
            .unwrap_or_else(|| local_app_data.clone())
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs");
        let desktop = dirs::desktop_dir().unwrap_or_else(|| user_profile.join("Desktop"));

        InstallPaths {
            bin_dir: local_app_data.join("Programs").join(APP_NAME),
            desktop_shortcut: Some(desktop.join(format!("{}.lnk", APP_NAME))),
            start_menu_entry: Some(start_menu.join(format!("{}.lnk", APP_NAME))),
            uninstall_key: Some(format!(
                "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}",
                APP_NAME
            )),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        InstallPaths {
            bin_dir: PathBuf::from("/tmp"),
            desktop_shortcut: None,
            start_menu_entry: None,
            uninstall_key: None,
        }
    }
}

/// Check if the application is installed
pub fn check_install() -> InstallStatus {
    let paths = get_install_paths();

    #[cfg(target_os = "linux")]
    {
        let binary_path = paths.bin_dir.join(get_versioned_app_name());
        let desktop_file_exists = paths
            .start_menu_entry
            .as_ref()
            .is_some_and(|p| p.exists());

        if binary_path.exists() && desktop_file_exists {
            return InstallStatus::Installed;
        }
    }

    #[cfg(target_os = "macos")]
    {
        let app_bundle = paths.bin_dir.join(format!("{}.app", get_versioned_app_name()));
        if app_bundle.exists() {
            return InstallStatus::Installed;
        }
    }

    #[cfg(target_os = "windows")]
    {
        let binary_path = paths.bin_dir.join(format!("{}.exe", get_versioned_app_name()));
        if binary_path.exists() {
            // Also check registry
            if check_windows_registry(&paths) {
                return InstallStatus::Installed;
            }
        }
    }

    InstallStatus::NotInstalled
}

#[cfg(target_os = "windows")]
fn check_windows_registry(paths: &InstallPaths) -> bool {
    use std::process::Command;

    if let Some(key) = &paths.uninstall_key {
        let output = Command::new("reg")
            .args(["query", key])
            .output();

        matches!(output, Ok(o) if o.status.success())
    } else {
        false
    }
}

#[cfg(not(target_os = "windows"))]
fn check_windows_registry(_paths: &InstallPaths) -> bool {
    false
}

/// Install the application
pub fn do_install() -> InstallResult {
    let paths = get_install_paths();
    let current_exe = match env::current_exe() {
        Ok(p) => p,
        Err(e) => return InstallResult::Error(format!("Failed to get current executable: {}", e)),
    };

    // Create installation directory
    if let Err(e) = fs::create_dir_all(&paths.bin_dir) {
        return InstallResult::Error(format!(
            "Failed to create installation directory: {}",
            e
        ));
    }

    #[cfg(target_os = "linux")]
    {
        match install_linux(&paths, &current_exe) {
            Ok(msg) => InstallResult::Success(msg),
            Err(e) => InstallResult::Error(e),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match install_macos(&paths, &current_exe) {
            Ok(msg) => InstallResult::Success(msg),
            Err(e) => InstallResult::Error(e),
        }
    }

    #[cfg(target_os = "windows")]
    {
        match install_windows(&paths, &current_exe) {
            Ok(msg) => InstallResult::Success(msg),
            Err(e) => InstallResult::Error(e),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        InstallResult::Error("Unsupported platform".to_string())
    }
}

#[cfg(target_os = "linux")]
fn install_linux(paths: &InstallPaths, current_exe: &PathBuf) -> Result<String, String> {
    log::info!("Starting Linux installation...");
    log::info!("Current exe: {}", current_exe.display());
    log::info!("Target directory: {}", paths.bin_dir.display());

    // Clean up old installations
    log::info!("Cleaning up old installations...");
    cleanup_old_installations(paths, None)?;

    let binary_dest = paths.bin_dir.join(get_versioned_app_name());
    log::info!("Installing to: {}", binary_dest.display());

    // Copy binary
    log::info!("Copying binary...");
    fs::copy(current_exe, &binary_dest)
        .map_err(|e| format!("Failed to copy binary: {}", e))?;
    log::info!("Binary copied successfully");

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        log::info!("Setting executable permissions (0755)...");
        let mut perms = fs::metadata(&binary_dest)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_dest, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
        log::debug!("Permissions set successfully");
    }

    // Create applications directory if needed
    if let Some(ref start_menu) = paths.start_menu_entry {
        if let Some(parent) = start_menu.parent() {
            log::info!("Creating applications directory: {}", parent.display());
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create applications directory: {}", e))?;
        }
    }

    // Create .desktop file for applications menu
    log::info!("Creating .desktop files...");
    let desktop_content = format!(
        r#"[Desktop Entry]
Name=UAD-Shizuku {}
Comment=Universal Android Debloater with Shizuku support
Exec={}
Icon={}
Terminal=false
Type=Application
Categories=Utility;Development;
Keywords=android;debloat;shizuku;adb;
"#,
        CURRENT_VERSION,
        binary_dest.display(),
        binary_dest.display() // TODO: Add proper icon path
    );

    if let Some(ref start_menu) = paths.start_menu_entry {
        log::info!("Creating applications menu entry: {}", start_menu.display());
        fs::write(start_menu, &desktop_content)
            .map_err(|e| format!("Failed to create .desktop file: {}", e))?;
        log::debug!("Applications menu entry created");
    }

    // Optionally create desktop shortcut
    if let Some(ref desktop) = paths.desktop_shortcut {
        if desktop.parent().is_some_and(|p| p.exists()) {
            log::info!("Creating desktop shortcut: {}", desktop.display());
            match fs::write(desktop, &desktop_content) {
                Ok(_) => log::debug!("Desktop shortcut created"),
                Err(e) => log::warn!("Failed to create desktop shortcut (non-critical): {}", e),
            }
        }
    }

    log::info!("Installation completed successfully");
    Ok(format!("Successfully installed to {}", binary_dest.display()))
}

#[cfg(target_os = "macos")]
fn install_macos(paths: &InstallPaths, current_exe: &PathBuf) -> Result<String, String> {
    log::info!("Starting macOS installation...");
    log::info!("Current exe: {}", current_exe.display());
    log::info!("Target directory: {}", paths.bin_dir.display());

    // Clean up old installations
    log::info!("Cleaning up old installations...");
    cleanup_old_installations(paths, None)?;

    let app_bundle = paths.bin_dir.join(format!("{}.app", get_versioned_app_name()));
    let contents_dir = app_bundle.join("Contents");
    let macos_dir = contents_dir.join("MacOS");

    // Create app bundle structure
    log::info!("Creating app bundle structure...");
    fs::create_dir_all(&macos_dir)
        .map_err(|e| format!("Failed to create app bundle: {}", e))?;

    // Copy binary
    let binary_dest = macos_dir.join(get_versioned_app_name());
    log::info!("Copying binary to: {}", binary_dest.display());
    fs::copy(current_exe, &binary_dest)
        .map_err(|e| format!("Failed to copy binary: {}", e))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        log::info!("Setting executable permissions...");
        let mut perms = fs::metadata(&binary_dest)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_dest, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Create Info.plist
    log::info!("Creating Info.plist...");
    let versioned_name = get_versioned_app_name();
    let info_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>{}</string>
    <key>CFBundleIdentifier</key>
    <string>pe.nikescar.uad-shizuku</string>
    <key>CFBundleName</key>
    <string>UAD-Shizuku {}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>{}</string>
    <key>CFBundleVersion</key>
    <string>{}</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
"#,
        versioned_name, CURRENT_VERSION, CURRENT_VERSION, CURRENT_VERSION
    );

    fs::write(contents_dir.join("Info.plist"), info_plist)
        .map_err(|e| format!("Failed to create Info.plist: {}", e))?;

    log::info!("Installation completed successfully");
    Ok(format!("Successfully installed to {}", app_bundle.display()))
}

#[cfg(target_os = "windows")]
fn install_windows(paths: &InstallPaths, current_exe: &PathBuf) -> Result<String, String> {
    use std::process::Command;

    log::info!("Starting Windows installation...");
    log::info!("Current exe: {}", current_exe.display());
    log::info!("Target directory: {}", paths.bin_dir.display());

    // Clean up old installations
    log::info!("Cleaning up old installations...");
    cleanup_old_installations(paths, None)?;

    let binary_dest = paths.bin_dir.join(format!("{}.exe", get_versioned_app_name()));
    log::info!("Installing to: {}", binary_dest.display());

    // Copy binary
    log::info!("Copying binary...");
    fs::copy(current_exe, &binary_dest)
        .map_err(|e| format!("Failed to copy binary: {}", e))?;
    log::info!("Binary copied successfully");

    // Add uninstall registry entry
    if let Some(ref key) = paths.uninstall_key {
        log::info!("Adding registry entries for uninstaller...");

        let reg_commands = [
            format!(r#"reg add "{}" /v DisplayName /t REG_SZ /d "UAD-Shizuku" /f"#, key),
            format!(r#"reg add "{}" /v DisplayVersion /t REG_SZ /d "{}" /f"#, key, CURRENT_VERSION),
            format!(r#"reg add "{}" /v Publisher /t REG_SZ /d "nikescar" /f"#, key),
            format!(r#"reg add "{}" /v UninstallString /t REG_SZ /d "\"{}\" --uninstall" /f"#, key, binary_dest.display()),
            format!(r#"reg add "{}" /v InstallLocation /t REG_SZ /d "{}" /f"#, key, paths.bin_dir.display()),
            format!(r#"reg add "{}" /v NoModify /t REG_DWORD /d 1 /f"#, key),
            format!(r#"reg add "{}" /v NoRepair /t REG_DWORD /d 1 /f"#, key),
        ];

        for cmd in &reg_commands {
            let _ = Command::new("cmd")
                .args(["/C", cmd])
                .output();
        }
        log::debug!("Registry entries added");
    }

    // Create Start Menu shortcut using PowerShell
    if let Some(ref start_menu) = paths.start_menu_entry {
        if let Some(parent) = start_menu.parent() {
            let _ = fs::create_dir_all(parent);
        }
        log::info!("Creating Start Menu shortcut...");
        create_windows_shortcut(&binary_dest, start_menu)?;
    }

    // Create Desktop shortcut
    if let Some(ref desktop) = paths.desktop_shortcut {
        log::info!("Creating Desktop shortcut...");
        let _ = create_windows_shortcut(&binary_dest, desktop);
    }

    log::info!("Installation completed successfully");
    Ok(format!("Successfully installed to {}", binary_dest.display()))
}

#[cfg(target_os = "windows")]
fn create_windows_shortcut(target: &PathBuf, shortcut_path: &PathBuf) -> Result<(), String> {
    use std::process::Command;

    let ps_script = format!(
        r#"$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut('{}'); $Shortcut.TargetPath = '{}'; $Shortcut.WorkingDirectory = '{}'; $Shortcut.Description = 'UAD-Shizuku - Universal Android Debloater'; $Shortcut.Save()"#,
        shortcut_path.display(),
        target.display(),
        target.parent().map(|p| p.display().to_string()).unwrap_or_default()
    );

    Command::new("powershell")
        .args(["-Command", &ps_script])
        .output()
        .map_err(|e| format!("Failed to create shortcut: {}", e))?;

    Ok(())
}

/// Uninstall the application
pub fn do_uninstall() -> InstallResult {
    let paths = get_install_paths();

    #[cfg(target_os = "linux")]
    {
        match uninstall_linux(&paths) {
            Ok(msg) => InstallResult::Success(msg),
            Err(e) => InstallResult::Error(e),
        }
    }

    #[cfg(target_os = "macos")]
    {
        match uninstall_macos(&paths) {
            Ok(msg) => InstallResult::Success(msg),
            Err(e) => InstallResult::Error(e),
        }
    }

    #[cfg(target_os = "windows")]
    {
        match uninstall_windows(&paths) {
            Ok(msg) => InstallResult::Success(msg),
            Err(e) => InstallResult::Error(e),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        InstallResult::Error("Unsupported platform".to_string())
    }
}

#[cfg(target_os = "linux")]
fn uninstall_linux(paths: &InstallPaths) -> Result<String, String> {
    let binary_path = paths.bin_dir.join(get_versioned_app_name());

    // Remove binary
    if binary_path.exists() {
        fs::remove_file(&binary_path)
            .map_err(|e| format!("Failed to remove binary: {}", e))?;
    }

    // Remove .desktop files
    if let Some(ref start_menu) = paths.start_menu_entry {
        let _ = fs::remove_file(start_menu);
    }
    if let Some(ref desktop) = paths.desktop_shortcut {
        let _ = fs::remove_file(desktop);
    }

    Ok("Successfully uninstalled UAD-Shizuku".to_string())
}

#[cfg(target_os = "macos")]
fn uninstall_macos(paths: &InstallPaths) -> Result<String, String> {
    let app_bundle = paths.bin_dir.join(format!("{}.app", get_versioned_app_name()));

    if app_bundle.exists() {
        fs::remove_dir_all(&app_bundle)
            .map_err(|e| format!("Failed to remove app bundle: {}", e))?;
    }

    Ok("Successfully uninstalled UAD-Shizuku".to_string())
}

#[cfg(target_os = "windows")]
fn uninstall_windows(paths: &InstallPaths) -> Result<String, String> {
    use std::process::Command;

    let binary_path = paths.bin_dir.join(format!("{}.exe", get_versioned_app_name()));

    // Remove shortcuts
    if let Some(ref start_menu) = paths.start_menu_entry {
        let _ = fs::remove_file(start_menu);
    }
    if let Some(ref desktop) = paths.desktop_shortcut {
        let _ = fs::remove_file(desktop);
    }

    // Remove registry entry
    if let Some(ref key) = paths.uninstall_key {
        let _ = Command::new("reg")
            .args(["delete", key, "/f"])
            .output();
    }

    // Remove binary and installation directory
    if binary_path.exists() {
        // On Windows, we can't delete a running executable, so schedule deletion on reboot
        // or use a helper batch script
        let batch_script = paths.bin_dir.join("uninstall.bat");
        let script_content = format!(
            r#"@echo off
:retry
del "{}" > nul 2>&1
if exist "{}" (
    timeout /t 1 /nobreak > nul
    goto retry
)
rmdir /s /q "{}"
del "%~f0"
"#,
            binary_path.display(),
            binary_path.display(),
            paths.bin_dir.display()
        );

        fs::write(&batch_script, script_content)
            .map_err(|e| format!("Failed to create uninstall script: {}", e))?;

        Command::new("cmd")
            .args(["/C", "start", "/min", "", &batch_script.display().to_string()])
            .spawn()
            .map_err(|e| format!("Failed to run uninstall script: {}", e))?;
    }

    Ok("Uninstallation initiated. The application will be fully removed after exit.".to_string())
}

/// Check for updates from GitHub releases
pub fn check_update() -> Result<UpdateInfo, String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    log::info!("Checking for updates from: {}", url);

    let response = ureq::get(&url)
        .set("User-Agent", &format!("{}/{}", APP_NAME, CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    let body = response
        .into_string()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let release: GitHubRelease = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    let current_version = CURRENT_VERSION.to_string();

    // Construct direct download URL based on platform and architecture
    let download_url = get_platform_download_url()
        .ok_or_else(|| {
            log::error!("No compatible release for platform {} arch {}",
                std::env::consts::OS, std::env::consts::ARCH);
            "No compatible release found for your platform".to_string()
        })?;

    let available = is_newer_version(&current_version, &latest_version);

    log::info!("Update check: current={}, latest={}, available={}, url={}",
        current_version, latest_version, available, download_url);

    Ok(UpdateInfo {
        available,
        current_version,
        latest_version,
        download_url,
        release_notes: release.body.unwrap_or_default(),
    })
}

/// Compare versions to determine if latest is newer
fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let current_parts = parse_version(current);
    let latest_parts = parse_version(latest);

    log::debug!("Version comparison: current={} ({:?}) vs latest={} ({:?})",
        current, current_parts, latest, latest_parts);

    for (c, l) in current_parts.iter().zip(latest_parts.iter()) {
        if l > c {
            log::info!("Update available: {} > {}", latest, current);
            return true;
        } else if c > l {
            log::info!("Current version is newer: {} > {}", current, latest);
            return false;
        }
    }

    let result = latest_parts.len() > current_parts.len();
    log::info!("Version comparison result: {} (length check)", result);
    result
}

/// Get the direct download URL for the current platform and architecture
fn get_platform_download_url() -> Option<String> {
    let target_arch = std::env::consts::ARCH; // "x86_64", "aarch64", etc.
    let target_os = std::env::consts::OS;

    log::debug!("Constructing download URL for OS: {}, Architecture: {}", target_os, target_arch);

    // Build filename based on platform and architecture
    let filename = match (target_os, target_arch) {
        // Windows x86_64: uad-shizuku.exe
        ("windows", "x86_64") => "uad-shizuku.exe",

        // macOS x86_64: uad-shizuku-x86_64-apple-darwin.tar.gz
        ("macos", "x86_64") => "uad-shizuku-x86_64-apple-darwin.tar.gz",

        // macOS aarch64: uad-shizuku-aarch64-apple-darwin.tar.gz
        ("macos", "aarch64") => "uad-shizuku-aarch64-apple-darwin.tar.gz",

        // Linux x86_64: uad-shizuku-x86_64-unknown-linux-musl.tar.gz
        ("linux", "x86_64") => "uad-shizuku-x86_64-unknown-linux-musl.tar.gz",

        // Linux aarch64: uad-shizuku-aarch64-unknown-linux-musl.tar.gz
        ("linux", "aarch64") => "uad-shizuku-aarch64-unknown-linux-musl.tar.gz",

        // Android: uad-shizuku-all-signed.apk
        ("android", _) => "uad-shizuku-all-signed.apk",

        // Unsupported platform
        (os, arch) => {
            log::warn!("Unsupported platform: {} {}", os, arch);
            return None;
        }
    };

    let download_url = format!(
        "https://github.com/{}/releases/latest/download/{}",
        GITHUB_REPO, filename
    );

    log::info!("Constructed download URL: {}", download_url);
    Some(download_url)
}

/// Download and apply update
pub fn do_update(download_url: &str, latest_version: &str, tmp_dir: &PathBuf) -> InstallResult {
    log::info!("=== Starting update process ===");
    log::info!("Download URL: {}", download_url);
    log::info!("Target version: {}", latest_version);
    log::info!("Temp directory: {}", tmp_dir.display());

    let paths = get_install_paths();

    // Download the update
    log::info!("Step 1: Downloading update...");
    let downloaded_file = match download_update(download_url, tmp_dir) {
        Ok(path) => {
            log::info!("Download completed: {}", path.display());
            path
        }
        Err(e) => {
            log::error!("Download failed: {}", e);
            return InstallResult::Error(format!("Download failed: {}", e));
        }
    };

    // Extract if archive
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    let binary_path = if downloaded_file.extension().is_some_and(|ext| ext == "gz" || ext == "tar") {
        log::info!("Step 2: Extracting tar.gz archive: {}", downloaded_file.display());
        match extract_tar_gz(&downloaded_file, tmp_dir) {
            Ok(path) => {
                log::info!("Extraction successful, binary at: {}", path.display());
                path
            }
            Err(e) => {
                log::error!("Extraction failed: {}", e);
                return InstallResult::Error(format!("Extraction failed: {}", e));
            }
        }
    } else if downloaded_file.extension().is_some_and(|ext| ext == "zip") {
        log::info!("Step 2: Extracting zip archive: {}", downloaded_file.display());
        match extract_zip(&downloaded_file, tmp_dir) {
            Ok(path) => {
                log::info!("Extraction successful, binary at: {}", path.display());
                path
            }
            Err(e) => {
                log::error!("Extraction failed: {}", e);
                return InstallResult::Error(format!("Extraction failed: {}", e));
            }
        }
    } else {
        log::info!("Step 2: No extraction needed, using downloaded file directly: {}", downloaded_file.display());
        downloaded_file
    };

    #[cfg(any(target_os = "android", target_arch = "wasm32"))]
    let binary_path = downloaded_file;

    // Replace current binary
    log::info!("Step 3: Installing new binary...");
    match replace_binary(&binary_path, &paths, Some(latest_version)) {
        Ok(msg) => {
            log::info!("=== Update completed successfully ===");
            InstallResult::Success(msg)
        }
        Err(e) => {
            log::error!("=== Update failed ===");
            log::error!("Error: {}", e);
            InstallResult::Error(e)
        }
    }
}

fn download_update(url: &str, tmp_dir: &PathBuf) -> Result<PathBuf, String> {
    let filename = url.split('/').last().unwrap_or("update");
    let dest_path = tmp_dir.join(filename);

    // Create tmp directory if it doesn't exist
    fs::create_dir_all(tmp_dir)
        .map_err(|e| format!("Failed to create tmp directory: {}", e))?;

    log::info!("Starting download from: {}", url);
    log::info!("Download destination: {}", dest_path.display());

    let response = ureq::get(url)
        .set("User-Agent", &format!("{}/{}", APP_NAME, CURRENT_VERSION))
        .call()
        .map_err(|e| format!("Download request failed: {}", e))?;

    let mut file = fs::File::create(&dest_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut reader = response.into_reader();
    io::copy(&mut reader, &mut file)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    log::info!("Download completed successfully");
    Ok(dest_path)
}

fn extract_tar_gz(archive_path: &PathBuf, dest_dir: &PathBuf) -> Result<PathBuf, String> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let file = fs::File::open(archive_path)
        .map_err(|e| format!("Failed to open archive: {}", e))?;

    let tar = GzDecoder::new(file);
    let mut archive = Archive::new(tar);

    archive.unpack(dest_dir)
        .map_err(|e| format!("Failed to extract archive: {}", e))?;

    // Find the binary in extracted files
    find_binary_in_dir(dest_dir)
}

fn extract_zip(archive_path: &PathBuf, dest_dir: &PathBuf) -> Result<PathBuf, String> {
    let file = fs::File::open(archive_path)
        .map_err(|e| format!("Failed to open archive: {}", e))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip: {}", e))?;

    archive.extract(dest_dir)
        .map_err(|e| format!("Failed to extract zip: {}", e))?;

    find_binary_in_dir(dest_dir)
}

fn find_binary_in_dir(dir: &PathBuf) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let binary_name = format!("{}.exe", get_versioned_app_name());
    #[cfg(not(target_os = "windows"))]
    let binary_name = get_versioned_app_name();

    for entry in walkdir(dir) {
        if let Ok(entry) = entry {
            if entry.file_name().to_string_lossy() == binary_name {
                return Ok(entry.path().to_path_buf());
            }
        }
    }

    Err(format!("Binary '{}' not found in archive", binary_name))
}

fn walkdir(dir: &PathBuf) -> impl Iterator<Item = Result<fs::DirEntry, io::Error>> {
    let mut stack = vec![dir.clone()];
    std::iter::from_fn(move || {
        while let Some(current_dir) = stack.pop() {
            if let Ok(entries) = fs::read_dir(&current_dir) {
                let mut items: Vec<_> = entries.collect();
                for entry in items.drain(..) {
                    if let Ok(ref e) = entry {
                        if e.path().is_dir() {
                            stack.push(e.path());
                        }
                    }
                    return Some(entry);
                }
            }
        }
        None
    })
}

fn replace_binary(new_binary: &PathBuf, paths: &InstallPaths, version: Option<&str>) -> Result<String, String> {
    log::info!("=== Starting binary replacement ===");
    log::debug!("Source binary: {}", new_binary.display());
    log::debug!("Install paths bin_dir: {}", paths.bin_dir.display());

    // Check if source binary exists
    if !new_binary.exists() {
        let err = format!("Source binary does not exist: {}", new_binary.display());
        log::error!("{}", err);
        return Err(err);
    }

    // Always install to the standard installation directory (e.g., ~/.local/bin on Linux)
    // Use versioned naming - use provided version (for updates) or current version (for installs)
    let versioned_name = if let Some(ver) = version {
        log::info!("Installing version: {}", ver);
        format!("{}-{}", APP_NAME, ver)
    } else {
        log::info!("Installing current version: {}", CURRENT_VERSION);
        get_versioned_app_name()
    };

    #[cfg(target_os = "windows")]
    let dest = paths.bin_dir.join(format!("{}.exe", versioned_name));
    #[cfg(not(target_os = "windows"))]
    let dest = paths.bin_dir.join(versioned_name);

    log::info!("Target installation path: {}", dest.display());

    // Ensure bin directory exists
    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            log::info!("Creating bin directory: {}", parent.display());
            fs::create_dir_all(parent)
                .map_err(|e| {
                    log::error!("Failed to create bin directory: {}", e);
                    format!("Failed to create bin directory: {}", e)
                })?;
        } else {
            log::debug!("Bin directory already exists");
        }
    }

    // Rename current binary before replacing (works on both Windows and Unix)
    // This allows updating a running executable
    if dest.exists() {
        #[cfg(target_os = "windows")]
        let backup = dest.with_extension("exe.old");
        #[cfg(not(target_os = "windows"))]
        let backup = dest.with_extension("old");

        log::info!("Existing binary found, backing up to: {}", backup.display());
        fs::rename(&dest, &backup)
            .map_err(|e| {
                log::error!("Failed to backup existing binary: {}", e);
                format!("Failed to backup existing binary: {}", e)
            })?;
        log::debug!("Backup completed");
    } else {
        log::debug!("No existing binary to backup");
    }

    log::info!("Copying new binary to destination...");
    fs::copy(new_binary, &dest)
        .map_err(|e| {
            log::error!("Failed to copy new binary: {}", e);
            format!("Failed to copy new binary: {}", e)
        })?;
    log::info!("Binary copied successfully");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        log::info!("Setting executable permissions (0755)...");
        let mut perms = fs::metadata(&dest)
            .map_err(|e| {
                log::error!("Failed to get permissions: {}", e);
                format!("Failed to get permissions: {}", e)
            })?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms)
            .map_err(|e| {
                log::error!("Failed to set permissions: {}", e);
                format!("Failed to set permissions: {}", e)
            })?;
        log::debug!("Permissions set successfully");
    }

    // Clean up old installations if we're installing a specific version (i.e., during update)
    if let Some(ver) = version {
        log::info!("Performing post-installation cleanup for version: {}", ver);

        log::info!("Cleaning up old installations...");
        match cleanup_old_installations(paths, Some(ver)) {
            Ok(_) => log::debug!("Old installations cleaned up successfully"),
            Err(e) => log::warn!("Failed to clean up old installations (non-critical): {}", e),
        }

        // Update shortcuts to point to new binary
        #[cfg(target_os = "windows")]
        {
            log::info!("Updating Windows shortcuts...");

            if let Some(ref start_menu) = paths.start_menu_entry {
                log::info!("Updating Start Menu shortcut: {}", start_menu.display());
                match create_windows_shortcut(&dest, start_menu) {
                    Ok(_) => log::info!("Start Menu shortcut updated successfully"),
                    Err(e) => {
                        log::error!("Failed to update Start Menu shortcut: {}", e);
                        return Err(format!("Failed to update Start Menu shortcut: {}", e));
                    }
                }
            }

            if let Some(ref desktop) = paths.desktop_shortcut {
                log::info!("Updating Desktop shortcut: {}", desktop.display());
                match create_windows_shortcut(&dest, desktop) {
                    Ok(_) => log::info!("Desktop shortcut updated successfully"),
                    Err(e) => log::warn!("Failed to update Desktop shortcut (non-critical): {}", e),
                }
            }

            log::info!("Shortcuts updated successfully");
        }

        #[cfg(target_os = "linux")]
        {
            log::info!("Updating Linux .desktop files...");
            let desktop_content = format!(
                r#"[Desktop Entry]
Name=UAD-Shizuku {}
Comment=Universal Android Debloater with Shizuku support
Exec={}
Icon={}
Terminal=false
Type=Application
Categories=Utility;Development;
Keywords=android;debloat;shizuku;adb;
"#,
                ver,
                dest.display(),
                dest.display()
            );

            if let Some(ref start_menu) = paths.start_menu_entry {
                log::info!("Updating applications menu entry: {}", start_menu.display());
                match fs::write(start_menu, &desktop_content) {
                    Ok(_) => log::debug!("Applications menu entry updated"),
                    Err(e) => log::warn!("Failed to update applications menu entry: {}", e),
                }
            }

            if let Some(ref desktop) = paths.desktop_shortcut {
                if desktop.parent().is_some_and(|p| p.exists()) {
                    log::info!("Updating desktop entry: {}", desktop.display());
                    match fs::write(desktop, &desktop_content) {
                        Ok(_) => log::debug!("Desktop entry updated"),
                        Err(e) => log::warn!("Failed to update desktop entry: {}", e),
                    }
                }
            }

            log::info!(".desktop files updated successfully");
        }
    }

    log::info!("=== Binary replacement completed successfully ===");
    log::info!("Installed to: {}", dest.display());
    Ok(format!("Successfully updated to new version. Please restart the application."))
}

/// Get current version
pub fn get_current_version() -> &'static str {
    CURRENT_VERSION
}

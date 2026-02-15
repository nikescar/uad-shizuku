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
use std::path::PathBuf;

const APP_NAME: &str = "uad-shizuku";
const GITHUB_REPO: &str = "nikescar/uad-shizuku";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        let binary_path = paths.bin_dir.join(APP_NAME);
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
        let app_bundle = paths.bin_dir.join(format!("{}.app", APP_NAME));
        if app_bundle.exists() {
            return InstallStatus::Installed;
        }
    }

    #[cfg(target_os = "windows")]
    {
        let binary_path = paths.bin_dir.join(format!("{}.exe", APP_NAME));
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
    let binary_dest = paths.bin_dir.join(APP_NAME);

    // Copy binary
    fs::copy(current_exe, &binary_dest)
        .map_err(|e| format!("Failed to copy binary: {}", e))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_dest)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_dest, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Create applications directory if needed
    if let Some(ref start_menu) = paths.start_menu_entry {
        if let Some(parent) = start_menu.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create applications directory: {}", e))?;
        }
    }

    // Create .desktop file for applications menu
    let desktop_content = format!(
        r#"[Desktop Entry]
Name=UAD-Shizuku
Comment=Universal Android Debloater with Shizuku support
Exec={}
Icon={}
Terminal=false
Type=Application
Categories=Utility;Development;
Keywords=android;debloat;shizuku;adb;
"#,
        binary_dest.display(),
        binary_dest.display() // TODO: Add proper icon path
    );

    if let Some(ref start_menu) = paths.start_menu_entry {
        fs::write(start_menu, &desktop_content)
            .map_err(|e| format!("Failed to create .desktop file: {}", e))?;
    }

    // Optionally create desktop shortcut
    if let Some(ref desktop) = paths.desktop_shortcut {
        if desktop.parent().is_some_and(|p| p.exists()) {
            let _ = fs::write(desktop, &desktop_content);
        }
    }

    Ok(format!("Successfully installed to {}", binary_dest.display()))
}

#[cfg(target_os = "macos")]
fn install_macos(paths: &InstallPaths, current_exe: &PathBuf) -> Result<String, String> {
    let app_bundle = paths.bin_dir.join(format!("{}.app", APP_NAME));
    let contents_dir = app_bundle.join("Contents");
    let macos_dir = contents_dir.join("MacOS");

    // Create app bundle structure
    fs::create_dir_all(&macos_dir)
        .map_err(|e| format!("Failed to create app bundle: {}", e))?;

    // Copy binary
    let binary_dest = macos_dir.join(APP_NAME);
    fs::copy(current_exe, &binary_dest)
        .map_err(|e| format!("Failed to copy binary: {}", e))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_dest)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_dest, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Create Info.plist
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
    <string>UAD-Shizuku</string>
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
        APP_NAME, CURRENT_VERSION, CURRENT_VERSION
    );

    fs::write(contents_dir.join("Info.plist"), info_plist)
        .map_err(|e| format!("Failed to create Info.plist: {}", e))?;

    Ok(format!("Successfully installed to {}", app_bundle.display()))
}

#[cfg(target_os = "windows")]
fn install_windows(paths: &InstallPaths, current_exe: &PathBuf) -> Result<String, String> {
    use std::process::Command;

    let binary_dest = paths.bin_dir.join(format!("{}.exe", APP_NAME));

    // Copy binary
    fs::copy(current_exe, &binary_dest)
        .map_err(|e| format!("Failed to copy binary: {}", e))?;

    // Add uninstall registry entry
    if let Some(ref key) = paths.uninstall_key {
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
    }

    // Create Start Menu shortcut using PowerShell
    if let Some(ref start_menu) = paths.start_menu_entry {
        if let Some(parent) = start_menu.parent() {
            let _ = fs::create_dir_all(parent);
        }
        create_windows_shortcut(&binary_dest, start_menu)?;
    }

    // Create Desktop shortcut
    if let Some(ref desktop) = paths.desktop_shortcut {
        let _ = create_windows_shortcut(&binary_dest, desktop);
    }

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
    let binary_path = paths.bin_dir.join(APP_NAME);

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
    let app_bundle = paths.bin_dir.join(format!("{}.app", APP_NAME));

    if app_bundle.exists() {
        fs::remove_dir_all(&app_bundle)
            .map_err(|e| format!("Failed to remove app bundle: {}", e))?;
    }

    Ok("Successfully uninstalled UAD-Shizuku".to_string())
}

#[cfg(target_os = "windows")]
fn uninstall_windows(paths: &InstallPaths) -> Result<String, String> {
    use std::process::Command;

    let binary_path = paths.bin_dir.join(format!("{}.exe", APP_NAME));

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

    let response = ureq::get(&url)
        .set("User-Agent", &format!("{}/{}", APP_NAME, CURRENT_VERSION))
        .call()
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    let body = response
        .into_string()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let release: GitHubRelease = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    let current_version = CURRENT_VERSION.to_string();

    // Find appropriate download URL based on platform
    let download_url = find_platform_asset(&release.assets);

    let available = is_newer_version(&current_version, &latest_version);

    Ok(UpdateInfo {
        available,
        current_version,
        latest_version,
        download_url: download_url.unwrap_or_default(),
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

    for (c, l) in current_parts.iter().zip(latest_parts.iter()) {
        if l > c {
            return true;
        } else if c > l {
            return false;
        }
    }

    latest_parts.len() > current_parts.len()
}

/// Find the appropriate asset for the current platform
fn find_platform_asset(assets: &[crate::install_stt::GitHubAsset]) -> Option<String> {
    #[cfg(target_os = "linux")]
    let patterns = ["linux", "Linux", "x86_64-unknown-linux"];

    #[cfg(target_os = "macos")]
    let patterns = ["macos", "darwin", "osx", "apple"];

    #[cfg(target_os = "windows")]
    let patterns = ["windows", "Windows", "win64", "x86_64-pc-windows"];

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let patterns: [&str; 0] = [];

    for asset in assets {
        for pattern in &patterns {
            if asset.name.contains(pattern) {
                return Some(asset.browser_download_url.clone());
            }
        }
    }

    None
}

/// Download and apply update
pub fn do_update(download_url: &str, tmp_dir: &PathBuf) -> InstallResult {
    let paths = get_install_paths();

    // Download the update
    let downloaded_file = match download_update(download_url, tmp_dir) {
        Ok(path) => path,
        Err(e) => return InstallResult::Error(format!("Download failed: {}", e)),
    };

    // Extract if archive
    let binary_path = if downloaded_file.extension().is_some_and(|ext| ext == "gz" || ext == "tar") {
        match extract_tar_gz(&downloaded_file, tmp_dir) {
            Ok(path) => path,
            Err(e) => return InstallResult::Error(format!("Extraction failed: {}", e)),
        }
    } else if downloaded_file.extension().is_some_and(|ext| ext == "zip") {
        match extract_zip(&downloaded_file, tmp_dir) {
            Ok(path) => path,
            Err(e) => return InstallResult::Error(format!("Extraction failed: {}", e)),
        }
    } else {
        downloaded_file
    };

    // Replace current binary
    match replace_binary(&binary_path, &paths) {
        Ok(msg) => InstallResult::Success(msg),
        Err(e) => InstallResult::Error(e),
    }
}

fn download_update(url: &str, tmp_dir: &PathBuf) -> Result<PathBuf, String> {
    let filename = url.split('/').last().unwrap_or("update");
    let dest_path = tmp_dir.join(filename);

    let response = ureq::get(url)
        .set("User-Agent", &format!("{}/{}", APP_NAME, CURRENT_VERSION))
        .call()
        .map_err(|e| format!("Download request failed: {}", e))?;

    let mut file = fs::File::create(&dest_path)
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut reader = response.into_reader();
    io::copy(&mut reader, &mut file)
        .map_err(|e| format!("Failed to write file: {}", e))?;

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
    let binary_name = format!("{}.exe", APP_NAME);
    #[cfg(not(target_os = "windows"))]
    let binary_name = APP_NAME.to_string();

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

fn replace_binary(new_binary: &PathBuf, paths: &InstallPaths) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let dest = paths.bin_dir.join(format!("{}.exe", APP_NAME));
    #[cfg(not(target_os = "windows"))]
    let dest = paths.bin_dir.join(APP_NAME);

    // On Windows, rename current binary before replacing
    #[cfg(target_os = "windows")]
    {
        let backup = dest.with_extension("exe.old");
        if dest.exists() {
            let _ = fs::rename(&dest, &backup);
        }
    }

    fs::copy(new_binary, &dest)
        .map_err(|e| format!("Failed to copy new binary: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest)
            .map_err(|e| format!("Failed to get permissions: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    Ok(format!("Successfully updated to new version. Please restart the application."))
}

/// Get current version
pub fn get_current_version() -> &'static str {
    CURRENT_VERSION
}

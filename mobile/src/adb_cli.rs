// adb_client crate wrapper — fallback when `adb` binary is not in PATH.
// Connects to the ADB server daemon over TCP (127.0.0.1:5037).

use adb_client::{ADBDeviceExt, ADBServer};
use log::debug;
use std::path::Path;

fn to_io_error(e: adb_client::RustADBError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, format!("{e}"))
}

fn get_device(server: &mut ADBServer, device: &str) -> std::io::Result<adb_client::ADBServerDevice> {
    server.get_device_by_name(device).map_err(to_io_error)
}

/// Execute a shell command on the device via adb_client.
/// Equivalent to: `adb -s <device> shell <command>`
pub fn shell_exec(device: &str, command: &str) -> std::io::Result<String> {
    debug!("adb_cli: shell_exec device={} command={}", device, command);
    let mut server = ADBServer::default();
    let mut dev = get_device(&mut server, device)?;
    let args: Vec<&str> = command.split_whitespace().collect();
    let mut output = Vec::new();
    dev.shell_command(&args, &mut output).map_err(to_io_error)?;
    Ok(String::from_utf8_lossy(&output).to_string())
}

/// List connected devices.
/// Equivalent to: `adb devices -l`
/// Returns a vector of device identifier strings (matching adb.rs format).
pub fn get_devices() -> std::io::Result<Vec<String>> {
    debug!("adb_cli: get_devices");
    let mut server = ADBServer::default();
    let devices = server.devices_long().map_err(to_io_error)?;
    let identifiers: Vec<String> = devices
        .iter()
        .map(|d| d.identifier.clone())
        .collect();
    Ok(identifiers)
}

/// Install an APK on the device.
/// Equivalent to: `adb -s <device> install <apk_path>`
pub fn install_apk(apk_path: &str, device: &str) -> std::io::Result<String> {
    debug!("adb_cli: install_apk device={} apk_path={}", device, apk_path);
    let mut server = ADBServer::default();
    let mut dev = get_device(&mut server, device)?;
    dev.install(&Path::new(apk_path)).map_err(to_io_error)?;
    Ok("Success".to_string())
}

/// Uninstall a package from the device.
/// Equivalent to: `adb -s <device> uninstall <package_name>`
pub fn uninstall_app(package_name: &str, device: &str) -> std::io::Result<String> {
    debug!("adb_cli: uninstall_app device={} package={}", device, package_name);
    let mut server = ADBServer::default();
    let mut dev = get_device(&mut server, device)?;
    dev.uninstall(package_name).map_err(to_io_error)?;
    Ok("Success".to_string())
}

/// Pull a file from the device to a local path.
/// Equivalent to: `adb -s <device> pull <file_path> <local_path>`
pub fn pull_file(device_serial: &str, file_path: &str, local_path: &str) -> std::io::Result<String> {
    debug!("adb_cli: pull_file device={} src={} dst={}", device_serial, file_path, local_path);
    let mut server = ADBServer::default();
    let mut dev = get_device(&mut server, device_serial)?;
    let mut file = std::fs::File::create(local_path)?;
    dev.pull(&file_path, &mut file).map_err(to_io_error)?;
    Ok(local_path.to_string())
}

/// Kill the ADB server.
/// Equivalent to: `adb kill-server`
pub fn kill_server() -> std::io::Result<String> {
    debug!("adb_cli: kill_server");
    let mut server = ADBServer::default();
    server.kill().map_err(to_io_error)?;
    Ok(String::new())
}

/// Request root permission (best-effort via shell).
/// Equivalent to: `adb root`
/// Note: adb_client has no direct `root` command, so we run `su -c id` as a probe.
pub fn root_get_permission() -> std::io::Result<String> {
    debug!("adb_cli: root_get_permission");
    let mut server = ADBServer::default();
    let mut dev = server.get_device().map_err(to_io_error)?;
    let mut output = Vec::new();
    dev.shell_command(&["su", "-c", "id"], &mut output).map_err(to_io_error)?;
    Ok(String::from_utf8_lossy(&output).to_string())
}

/// Find files in a directory on the device via shell find command.
/// Equivalent to: `adb -s <device> shell find <dir_path> -maxdepth 1 -name "<pattern>" -type f`
pub fn find_files_in_directory(device_serial: &str, dir_path: &str, pattern: &str, max_depth: u32) -> Vec<String> {
    debug!("adb_cli: find_files_in_directory device={} dir={} pattern={}", device_serial, dir_path, pattern);
    let mut server = ADBServer::default();
    let mut dev = match get_device(&mut server, device_serial) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let depth_str = max_depth.to_string();
    let args = vec!["find", dir_path, "-maxdepth", &depth_str, "-name", pattern, "-type", "f"];
    let mut output = Vec::new();
    if dev.shell_command(&args, &mut output).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output).to_string();
    text.lines()
        .map(|line| line.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect()
}

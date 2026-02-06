//! Installation state structures for desktop platforms (Windows/Linux/macOS)

use serde::Deserialize;

/// Installation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallStatus {
    #[default]
    Unknown,
    Installed,
    NotInstalled,
}

/// Update check result
#[derive(Debug, Clone, Default)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub release_notes: String,
}

/// GitHub release asset
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// GitHub release response
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: Option<String>,
    pub assets: Vec<GitHubAsset>,
}

/// Installation operation result
#[derive(Debug, Clone)]
pub enum InstallResult {
    Success(String),
    Error(String),
}

/// Installation paths for each platform
#[derive(Debug, Clone)]
pub struct InstallPaths {
    /// Binary installation directory
    pub bin_dir: std::path::PathBuf,
    /// Desktop shortcut path
    pub desktop_shortcut: Option<std::path::PathBuf>,
    /// Start menu/applications entry
    pub start_menu_entry: Option<std::path::PathBuf>,
    /// Uninstall registry key (Windows only)
    pub uninstall_key: Option<String>,
}

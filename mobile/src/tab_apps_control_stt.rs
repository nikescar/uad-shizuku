use crate::adb::PackageFingerprint;
use crate::dlg_uninstall_confirm::DlgUninstallConfirm;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppListSource {
    pub name: String,
    pub info_url: String,
    pub contents_url: String,
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub category: String,
    pub name: String,
    pub links: Vec<(String, String)>, // (url, type) where type is "fdroid", "izzy", "home", "source"
    pub package_name: Option<String>,
}

pub struct TabAppsControl {
    pub open: bool,
    pub installed_packages: Vec<PackageFingerprint>,
    pub app_lists: Vec<AppListSource>,
    pub selected_app_list: Option<usize>,
    pub app_entries: Vec<AppEntry>,
    pub refresh_pending: bool,
    pub cache_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub installing_apps: HashMap<String, String>, // app_name -> status message
    pub selected_device: Option<String>,
    pub previous_app_list: Option<usize>, // Track previous selection to detect changes
    pub recently_installed_apps: HashSet<String>, // Track app names that were just installed (for GitHub apps where package name isn't in URL)
    pub show_only_installable: bool, // Filter to show only apps with downloadable links
    pub disable_github_install: bool, // Option to disable GitHub installations
    pub text_filter: String, // Text filter for searching all visible text in the table
    pub sort_column: Option<usize>, // Sort column for mobile view
    pub sort_ascending: bool, // Sort direction
    // Uninstall confirmation dialog
    pub uninstall_confirm_dialog: DlgUninstallConfirm,
}

use crate::adb::PackageFingerprint;
use crate::dlg_package_details::DlgPackageDetails;
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

/// Prepared row data for Apps Control - cached to avoid recomputing on every frame
#[derive(Clone)]
pub struct PreparedAppsRowData {
    pub idx: usize,
    pub app: AppEntry,
    pub downloadable_link: Option<(String, String)>,
    pub is_installed: bool,
    pub installed_pkg_info: Option<(String, bool, String)>, // (pkg_name, is_system, enabled_state)
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
    pub text_filter: String,         // Text filter for searching all visible text in the table
    pub sort_column: Option<usize>,  // Sort column for mobile view
    pub sort_ascending: bool,        // Sort direction
    // Background operations queue
    pub operations_queue: Option<std::sync::Arc<crate::app_operations_queue::AppOperationsQueue>>,
    // Track if operations just completed to trigger refresh
    pub was_worker_running: bool,
    pub pending_refresh_after_operations: bool,
    // Bumped on every update_packages() call so the cached prepared-row table
    // (keyed on this) is invalidated whenever installed_packages changes.
    pub packages_version: u64,
    // Dialogs
    pub package_details_dialog: DlgPackageDetails,
    pub uninstall_confirm_dialog: DlgUninstallConfirm,
}

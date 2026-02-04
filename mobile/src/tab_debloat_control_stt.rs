use crate::dlg_package_details::DlgPackageDetails;
use std::collections::HashSet;

pub enum AdbResult {
    Success(String), // package name
    Failure,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DebloatFilter {
    All,
    Recommended,
    Advanced,
    Expert,
    Unsafe,
    Unknown,
}

pub struct TabDebloatControl {
    pub open: bool,
    // NOTE: installed_packages, uad_ng_lists, textures, and cached apps are now in shared_store_stt::SharedStore
    // Access via: crate::shared_store_stt::get_shared_store()

    // Selection state - using package names as keys for stability across sorting
    pub selected_packages: HashSet<String>,
    // Package details dialog
    pub package_details_dialog: DlgPackageDetails,
    // Filter state
    pub active_filter: DebloatFilter,
    // Sort state
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
    // Device selection
    pub selected_device: Option<String>,
    // Version counter for table ID - incremented when packages are reloaded
    pub table_version: u64,

    // Filter toggles
    pub show_only_enabled: bool,
    pub hide_system_app: bool,
}

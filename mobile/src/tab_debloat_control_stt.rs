use crate::adb::PackageFingerprint;
use crate::gui::UadNgLists;
use crate::models::{ApkMirrorApp, FDroidApp, GooglePlayApp};
use crate::win_package_details_dialog::PackageDetailsDialog;
use eframe::egui;
use std::collections::{HashMap, HashSet};

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
    // Store reference to installed packages
    pub installed_packages: Vec<PackageFingerprint>,
    pub uad_ng_lists: Option<UadNgLists>,
    // Selection state - using package names as keys for stability across sorting
    pub selected_packages: HashSet<String>,
    // Package details dialog
    pub package_details_dialog: PackageDetailsDialog,
    // Filter state
    pub active_filter: DebloatFilter,
    // Sort state
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
    // Device selection
    pub selected_device: Option<String>,
    // Version counter for table ID - incremented when packages are reloaded
    pub table_version: u64,

    // Texture caches (package_id -> TextureHandle)
    pub google_play_textures: HashMap<String, egui::TextureHandle>,
    pub fdroid_textures: HashMap<String, egui::TextureHandle>,
    pub apkmirror_textures: HashMap<String, egui::TextureHandle>,

    // Cached app info from database (package_id -> app data)
    pub cached_google_play_apps: HashMap<String, GooglePlayApp>,
    pub cached_fdroid_apps: HashMap<String, FDroidApp>,
    pub cached_apkmirror_apps: HashMap<String, ApkMirrorApp>,

    // Filter toggles
    pub show_only_enabled: bool,
    pub hide_system_app: bool,
}

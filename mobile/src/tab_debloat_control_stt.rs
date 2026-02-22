use crate::dlg_package_details::DlgPackageDetails;
use crate::dlg_uninstall_confirm::DlgUninstallConfirm;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

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

/// Cached category counts to avoid recomputing every frame
#[derive(Default, Clone)]
pub struct CachedCategoryCounts {
    pub recommended: (usize, usize), // (enabled, total)
    pub advanced: (usize, usize),
    pub expert: (usize, usize),
    pub unsafe_count: (usize, usize),
    pub unknown: (usize, usize),
    pub version: u64, // tracks when cache was computed
}

/// State machine for batch uninstall operations
/// Uses the same pattern as ScanStateMachine for tracking async operation state
#[derive(Default)]
pub struct BatchUninstallState {
    /// Progress value (0.0 - 1.0) for batch uninstalls
    pub progress: Option<f32>,
    /// Whether uninstall is currently running
    pub is_running: bool,
    /// Whether uninstall was cancelled
    pub is_cancelled: bool,
}

impl BatchUninstallState {
    pub fn start(&mut self) {
        self.is_running = true;
        self.is_cancelled = false;
        self.progress = Some(0.0);
    }

    pub fn cancel(&mut self) {
        self.is_cancelled = true;
        self.is_running = false;
        self.progress = None;
    }

    pub fn complete(&mut self) {
        self.is_running = false;
        self.progress = None;
    }

    pub fn update_progress(&mut self, value: f32) {
        self.progress = Some(value);
    }
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

    // Cached category counts to avoid recomputing every frame
    pub cached_counts: CachedCategoryCounts,

    // Text filter for searching all visible text in the table
    pub text_filter: String,

    // Safety setting: when false, prevent uninstall of Unsafe category apps
    pub unsafe_app_remove: bool,

    // Uninstall confirmation dialog
    pub uninstall_confirm_dialog: DlgUninstallConfirm,

    // Batch uninstall state machine
    pub batch_uninstall_state: BatchUninstallState,
    // Progress for batch uninstall background task (for thread communication)
    pub batch_uninstall_progress: Arc<Mutex<Option<f32>>>,
    // Cancellation flag for batch uninstall
    pub batch_uninstall_cancelled: Arc<Mutex<bool>>,

    // Batch disable state machine
    pub batch_disable_state: BatchUninstallState,
    // Progress for batch disable background task (for thread communication)
    pub batch_disable_progress: Arc<Mutex<Option<f32>>>,
    // Cancellation flag for batch disable
    pub batch_disable_cancelled: Arc<Mutex<bool>>,

    // Batch enable state machine
    pub batch_enable_state: BatchUninstallState,
    // Progress for batch enable background task (for thread communication)
    pub batch_enable_progress: Arc<Mutex<Option<f32>>>,
    // Cancellation flag for batch enable
    pub batch_enable_cancelled: Arc<Mutex<bool>>,
}

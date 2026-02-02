use crate::adb::PackageFingerprint;
use crate::calc_hybridanalysis::{
    ScannerState as HaScannerState, SharedRateLimiter as HaSharedRateLimiter,
};
use crate::calc_virustotal::{
    ScannerState as VtScannerState, SharedRateLimiter as VtSharedRateLimiter,
};
use crate::uad_shizuku_app::UadNgLists;
use crate::models::{ApkMirrorApp, FDroidApp, GooglePlayApp};
use crate::win_package_details_dialog::PackageDetailsDialog;
use eframe::egui;
use egui_async::Bind;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// State machine for scan operations
/// Uses egui-async Bind pattern for tracking async operation state
#[derive(Default)]
pub struct ScanStateMachine {
    /// Progress value (0.0 - 1.0) for batch scans
    pub progress: Option<f32>,
    /// Whether scan is currently running
    pub is_running: bool,
    /// Whether scan was cancelled
    pub is_cancelled: bool,
}

impl ScanStateMachine {
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

pub struct TabScanControl {
    pub open: bool,
    // Store reference to installed packages
    pub installed_packages: Vec<PackageFingerprint>,
    pub uad_ng_lists: Option<UadNgLists>,
    // Selection state
    pub selected_packages: Vec<bool>,
    // Risk score cache: package_name -> risk_score
    pub package_risk_scores: HashMap<String, i32>,
    // Bind for IzzyRisk calculation (calculates all scores asynchronously)
    pub izzyrisk_bind: Bind<HashMap<String, i32>, String>,
    // Package details dialog
    pub package_details_dialog: PackageDetailsDialog,
    // VirusTotal scanner state
    pub vt_scanner_state: Option<VtScannerState>,
    // Shared rate limiter for VirusTotal API
    pub vt_rate_limiter: Option<VtSharedRateLimiter>,
    // Package paths cache for faster scanning (path, sha256 hash)
    pub vt_package_paths_cache:
        Option<std::sync::Arc<std::sync::Mutex<HashMap<String, Vec<(String, String)>>>>>,
    // VirusTotal scan state machine
    pub vt_scan_state: ScanStateMachine,
    // Hybrid Analysis scanner state
    pub ha_scanner_state: Option<HaScannerState>,
    // Shared rate limiter for Hybrid Analysis API
    pub ha_rate_limiter: Option<HaSharedRateLimiter>,
    // Package paths cache for faster scanning (path, sha256 hash)
    pub ha_package_paths_cache:
        Option<std::sync::Arc<std::sync::Mutex<HashMap<String, Vec<(String, String)>>>>>,
    // HybridAnalysis scan state machine
    pub ha_scan_state: ScanStateMachine,
    // IzzyRisk scan state machine
    pub izzyrisk_scan_state: ScanStateMachine,
    // Progress for IzzyRisk scan background task (for thread communication)
    pub izzyrisk_scan_progress: Arc<Mutex<Option<f32>>>,
    // Cancellation flag for IzzyRisk scan
    pub izzyrisk_scan_cancelled: Arc<Mutex<bool>>,
    // Shared risk scores from background thread
    pub shared_package_risk_scores: Arc<Mutex<HashMap<String, i32>>>,
    // Config for API keys and device serial
    pub vt_api_key: Option<String>,
    pub ha_api_key: Option<String>,
    pub device_serial: Option<String>,
    pub virustotal_submit_enabled: bool,
    pub hybridanalysis_submit_enabled: bool,
    // Sort state
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
    // Filter state
    pub active_vt_filter: VtFilter,
    pub active_ha_filter: HaFilter,

    // Progress for VirusTotal scan background task (for thread communication)
    pub vt_scan_progress: Arc<Mutex<Option<f32>>>,
    // Cancellation flag for VirusTotal scan
    pub vt_scan_cancelled: Arc<Mutex<bool>>,
    // Progress for HybridAnalysis scan background task
    pub ha_scan_progress: Arc<Mutex<Option<f32>>>,
    // Cancellation flag for HybridAnalysis scan
    pub ha_scan_cancelled: Arc<Mutex<bool>>,

    // Cached app info from database (shared with TabDebloatControl)
    // Loaded once when packages are updated, used for fast lookup in UI
    pub cached_google_play_apps: HashMap<String, GooglePlayApp>,
    pub cached_fdroid_apps: HashMap<String, FDroidApp>,
    pub cached_apkmirror_apps: HashMap<String, ApkMirrorApp>,
    // Cache for app icons (package_id -> TextureHandle)
    pub app_textures: HashMap<String, egui::TextureHandle>,
    // Filter to show only enabled (green) packages
    pub show_only_enabled: bool,
    // Filter to hide system apps
    pub hide_system_app: bool,
    // Renderer settings (synced from Settings)
    pub google_play_renderer_enabled: bool,
    pub fdroid_renderer_enabled: bool,
    pub apkmirror_renderer_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VtFilter {
    All,
    Malicious,
    Suspicious,
    Safe,
    NotScanned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HaFilter {
    All,
    Malicious,
    Suspicious,
    Safe,
    NotScanned,
}

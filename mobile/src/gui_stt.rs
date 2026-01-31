use crate::adb::PackageFingerprint;
use crate::adb::UserInfo;
use crate::tab_apps_control::TabAppsControl;
use crate::tab_debloat_control::TabDebloatControl;
use crate::tab_scan_control::TabScanControl;
use crate::tab_usage_control::TabUsageControl;
use crate::Config;
use crate::LogLevel;
use crate::Settings;
use eframe::egui::Rect;
use egui_material3::menu::{Corner, FocusState, Positioning};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State machine for renderer lifecycle management
#[derive(Default)]
pub struct RendererStateMachine {
    /// Whether the renderer is currently enabled
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LogSettings {
    pub show_logs: bool,
    pub log_level: LogLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UadNgLists {
    #[serde(flatten)]
    pub apps: HashMap<String, AppEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub list: String,
    pub description: String,
    pub dependencies: Vec<String>,
    #[serde(rename = "neededBy")]
    pub needed_by: Vec<String>,
    pub labels: Vec<String>,
    pub removal: String,
}

#[doc(hidden)]
pub struct UadShizukuApp {
    pub config: Option<Config>,
    pub current_view: AppView,
    pub shizuku_connected: bool,
    // top app bar state
    pub title_text: String,
    pub show_navigation: bool,
    pub show_actions: bool,
    pub is_scrolled: bool,
    pub custom_height: f32,
    pub use_custom_height: bool,
    //
    pub custom_selected: usize,
    // menu control
    pub items_button_rect: Option<Rect>,
    pub standard_menu_open: bool,
    // Knob options
    pub anchor_corner: Corner,
    pub menu_corner: Corner,
    pub default_focus: FocusState,
    pub positioning: Positioning,
    pub quick: bool,
    pub has_overflow: bool,
    pub stay_open_on_outside_click: bool,
    pub stay_open_on_focusout: bool,
    pub skip_restore_focus: bool,
    pub x_offset: f32,
    pub y_offset: f32,
    pub no_horizontal_flip: bool,
    pub no_vertical_flip: bool,
    pub typeahead_delay: f32,
    pub list_tab_index: i32,

    pub disabled: bool,

    pub adb_devices: Vec<String>,
    pub selected_device: Option<String>,
    pub current_device: Option<String>,

    pub adb_users: Vec<UserInfo>,
    pub selected_user: Option<i32>, // None means "All Users"
    pub current_user: Option<i32>,

    pub installed_packages: Vec<PackageFingerprint>,
    pub uad_ng_lists: Option<UadNgLists>,

    pub tab_debloat_control: TabDebloatControl,
    pub tab_scan_control: TabScanControl,
    pub tab_usage_control: TabUsageControl,
    pub tab_apps_control: TabAppsControl,

    // Settings dialog state
    pub settings_dialog_open: bool,
    pub settings: Settings,
    pub settings_virustotal_apikey: String,
    pub settings_hybridanalysis_apikey: String,
    pub settings_invalidate_cache: bool,
    // Flush checkboxes for each service
    pub settings_flush_virustotal: bool,
    pub settings_flush_hybridanalysis: bool,
    pub settings_flush_googleplay: bool,
    pub settings_flush_fdroid: bool,
    pub settings_flush_apkmirror: bool,

    // Progress tracking for background tasks
    pub package_load_progress: std::sync::Arc<std::sync::Mutex<Option<f32>>>,

    // ADB installation dialog state
    pub adb_install_dialog_open: bool,

    // Disclaimer dialog state
    pub disclaimer_dialog_open: bool,

    // Font selector state
    pub system_fonts: Vec<(String, String)>,
    pub system_fonts_loaded: bool,
    pub selected_font_display: String,

    // Renderer state machines
    pub google_play_renderer: RendererStateMachine,
    pub fdroid_renderer: RendererStateMachine,
    pub apkmirror_renderer: RendererStateMachine,

    // Background worker queues for fetching app data
    pub google_play_queue: Option<std::sync::Arc<crate::calc_googleplay::GooglePlayQueue>>,
    pub fdroid_queue: Option<std::sync::Arc<crate::calc_fdroid::FDroidQueue>>,
    pub apkmirror_queue: Option<std::sync::Arc<crate::calc_apkmirror::ApkMirrorQueue>>,

    // Package loading state
    pub package_loading_thread: Option<std::thread::JoinHandle<(Vec<crate::adb::PackageFingerprint>, Option<UadNgLists>)>>,
    pub package_loading_dialog_open: bool,
    pub package_loading_status: String,
}

pub enum AppView {
    Debloat,
    Scan,
    Apps,
    Usage,
}

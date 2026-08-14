use crate::adb::UserInfo;
#[cfg(not(target_os = "android"))]
use crate::install_stt::InstallStatus;
use crate::tab_apps_control::TabAppsControl;
use crate::tab_debloat::TabDebloat;
use crate::tab_debloat_control::TabDebloatControl; // TRANSITIONAL: Being phased out
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

#[derive(Debug, Clone)]
pub struct UadNgLists {
    pub apps: HashMap<String, AppEntry>,
}

// UAD-NG's uad_lists.json is a top-level JSON object keyed by package id
// (https://github.com/0x192/universal-android-debloater resources/assets/uad_lists.json),
// not an array of entries with an embedded id field.
impl<'de> Deserialize<'de> for UadNgLists {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let apps = HashMap::<String, AppEntry>::deserialize(deserializer)?;
        Ok(UadNgLists { apps })
    }
}

impl Serialize for UadNgLists {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.apps.serialize(serializer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub list: String,
    pub description: String,
    #[serde(default, deserialize_with = "null_to_empty_vec")]
    pub dependencies: Vec<String>,
    #[serde(rename = "neededBy", default, deserialize_with = "null_to_empty_vec")]
    pub needed_by: Vec<String>,
    #[serde(default, deserialize_with = "null_to_empty_vec")]
    pub labels: Vec<String>,
    pub removal: String,
}

fn null_to_empty_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<Vec<String>>::deserialize(deserializer).map(|opt| opt.unwrap_or_default())
}

#[cfg(test)]
mod uad_ng_lists_tests {
    use super::UadNgLists;

    // UAD-NG's uad_lists.json is a top-level JSON object keyed by package id,
    // e.g. https://github.com/0x192/universal-android-debloater resources/assets/uad_lists.json
    const SAMPLE_MAP_JSON: &str = r#"{
        "org.lineageos.jelly": {
            "list": "Oem",
            "description": "LineageOS Browser App.",
            "dependencies": [],
            "neededBy": [],
            "labels": [],
            "removal": "Recommended"
        }
    }"#;

    #[test]
    fn deserializes_upstream_map_format() {
        let lists: UadNgLists = serde_json::from_str(SAMPLE_MAP_JSON).unwrap();
        assert_eq!(lists.apps.len(), 1);
        let entry = lists.apps.get("org.lineageos.jelly").unwrap();
        assert_eq!(entry.list, "Oem");
        assert_eq!(entry.removal, "Recommended");
    }
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

    // NOTE: installed_packages and uad_ng_lists are now in shared_store_stt::SharedStore
    // Access via: crate::shared_store_stt::get_shared_store()
    pub tab_debloat: TabDebloat, // REFACTORED: New MVVM-based tab
    pub tab_debloat_control: TabDebloatControl, // TRANSITIONAL: Being phased out
    pub tab_scan_control: TabScanControl,
    pub tab_usage_control: TabUsageControl,
    pub tab_apps_control: TabAppsControl,

    // Settings
    pub settings: Settings,

    // Dialog states
    pub dlg_settings: crate::dlg_settings_stt::DlgSettings,

    // Progress tracking for background tasks
    pub package_load_progress: std::sync::Arc<std::sync::Mutex<Option<f32>>>,

    pub dlg_adb_install: crate::dlg_adb_install_stt::DlgAdbInstall,

    // Disclaimer dialog state
    pub disclaimer_dialog_open: bool,

    pub dlg_about: crate::dlg_about_stt::DlgAbout,
    pub dlg_update: crate::dlg_update_stt::DlgUpdate,
    pub dlg_dashcounter_details: crate::dlg_dashcounter_details_stt::DlgDashCounterDetails,

    // Installation status (desktop only)
    #[cfg(not(target_os = "android"))]
    pub install_status: InstallStatus,
    #[cfg(not(target_os = "android"))]
    pub install_dialog_open: bool,
    #[cfg(not(target_os = "android"))]
    pub install_message: String,

    // Update status (both desktop and Android)
    pub update_status: String,
    pub update_available: bool,
    pub update_checking: bool,

    // Renderer state machines
    pub google_play_renderer: RendererStateMachine,
    pub fdroid_renderer: RendererStateMachine,
    pub apkmirror_renderer: RendererStateMachine,

    // Background worker queues for fetching app data
    pub google_play_queue: Option<std::sync::Arc<crate::calc_googleplay::GooglePlayQueue>>,
    pub fdroid_queue: Option<std::sync::Arc<crate::calc_fdroid::FDroidQueue>>,
    pub apkmirror_queue: Option<std::sync::Arc<crate::calc_apkmirror::ApkMirrorQueue>>,

    // Package loading state
    pub package_loading_thread:
        Option<std::thread::JoinHandle<(Vec<crate::adb::PackageFingerprint>, Option<UadNgLists>)>>,
    pub package_loading_dialog_open: bool,
    pub package_loading_status: String,

    // First-run initialization flag
    pub first_update_done: bool,

    // Shizuku state tracking (Android)
    pub shizuku_init_done: bool,
    pub shizuku_permission_requested: bool,
    pub shizuku_bind_requested: bool,
    pub shizuku_error_message: Option<String>,

    // Pinch-to-zoom state (Android)
    pub zoom_factor: f32,

    // Dashboard counter scroll offsets
    pub dash_scroll_debloat: f32,
    pub dash_scroll_stalkerware: f32,
    pub dash_scroll_izzyrisk: f32,
    pub dash_scroll_virustotal: f32,
    pub dash_scroll_hybridanalysis: f32,
    pub dash_scroll_offa: f32,
    pub dash_scroll_fmhy: f32,

    // Installer package name (Android) - cached for UI decisions
    pub installer_package_name: Option<String>,

    // Debloat tab performance optimization
    pub debloat_last_enqueued_version: u64,
    pub debloat_last_result_load_time: std::time::Instant,

    // Tab controller state (shared between mobile and desktop UI)
    pub show_apps_tab: bool,

    // ViewModel for MVVM architecture (lazy initialization)
    pub viewmodel: Option<crate::viewmodel::ViewModel>,
}

pub enum AppView {
    Debloat,
    Scan,
    Apps,
    Usage,
}

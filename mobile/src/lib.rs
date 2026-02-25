#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

use eframe::egui;
use sys_locale;

mod adb;
pub mod adb_stt;
mod android_packagemanager;
pub mod android_shizuku;
mod android_inputmethod;
mod android_activity;
mod android_clipboard;
mod android_contexttheme;
mod clipboard_popup;
mod tab_apps_control;
pub mod tab_apps_control_stt;
mod tab_debloat_control;
pub mod tab_debloat_control_stt;
mod tab_scan_control;
pub mod tab_scan_control_stt;
mod tab_usage_control;
pub mod tab_usage_control_stt;
mod dlg_package_details;
pub mod dlg_package_details_stt;
mod dlg_uninstall_confirm;
pub mod dlg_uninstall_confirm_stt;
mod dlg_settings;
pub mod dlg_settings_stt;
mod dlg_adb_install;
pub mod dlg_adb_install_stt;
mod dlg_update;
pub mod dlg_update_stt;
mod dlg_about;
pub mod dlg_about_stt;

pub mod api_apkmirror;
pub mod api_apkmirror_stt;
pub mod api_fdroid;
pub mod api_fdroid_stt;
mod api_googleplay;
pub mod api_googleplay_stt;
mod app_operations_queue;
pub mod app_operations_queue_stt;
pub mod api_hybridanalysis;
pub mod api_hybridanalysis_stt;
mod api_virustotal;
pub mod api_virustotal_stt;
pub mod calc_androidpackage;
mod calc_apkmirror;
pub mod calc_apkmirror_stt;
mod calc_fdroid;
pub mod calc_fdroid_stt;
mod calc_googleplay;
pub mod calc_googleplay_stt;
mod calc_hybridanalysis;
pub mod calc_hybridanalysis_stt;
mod calc_izzyrisk;
mod calc_stalkerware;
pub mod calc_stalkerware_stt;
mod calc_virustotal;
pub mod calc_virustotal_stt;
pub mod db;
pub mod db_apkmirror;
pub mod db_fdroid;
pub mod db_googleplay;
pub mod db_hybridanalysis;
pub mod db_package_cache;
pub mod db_virustotal;
mod models;
mod schema;

pub mod log_capture;
// mod android_wallpaper;
// mod android_screensize;

// Export modules for external use
pub use uad_shizuku_app::{UadShizukuApp as GuiApp, View};
pub mod uad_shizuku_app;
pub mod uad_shizuku_app_stt;
pub mod shared_store_stt;
mod shared_store;
pub mod svg_stt;
pub mod material_symbol_icons;

// Installation management for desktop platforms
// (check_update function is available on all platforms)
pub mod install;
pub mod install_stt;

#[cfg(target_os = "android")]
mod main_android;

use anyhow::{Context, Result};
#[cfg(not(target_os = "android"))]
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub download_dir: PathBuf,
    pub db_dir: PathBuf,
    pub tmp_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub virustotal_apikey: String,
    pub hybridanalysis_apikey: String,
    #[serde(default)]
    pub virustotal_submit: bool,
    #[serde(default)]
    pub hybridanalysis_submit: bool,
    #[serde(default = "default_hybridanalysis_tag_ignorelist")]
    pub hybridanalysis_tag_ignorelist: String,
    #[serde(default)]
    pub show_logs: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_theme_mode")]
    pub theme_mode: String,
    #[serde(default = "default_contrast_level")]
    pub contrast_level: String,
    #[serde(default = "default_display_size")]
    pub display_size: String,
    #[serde(default)]
    pub google_play_renderer: bool,
    #[serde(default)]
    pub fdroid_renderer: bool,
    #[serde(default)]
    pub apkmirror_renderer: bool,
    #[serde(default)]
    pub apkmirror_email: String,
    #[serde(default)]
    pub apkmirror_name: String,
    #[serde(default)]
    pub apkmirror_auto_upload: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_font_path")]
    pub font_path: String,
    #[serde(default = "default_override_text_style")]
    pub override_text_style: String,
    #[serde(default = "default_theme_name")]
    pub theme_name: String,
    #[serde(default)]
    pub unsafe_app_remove: bool,
    #[serde(default)]
    pub autoupdate: bool,
}

#[allow(dead_code)]
fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "Auto".to_string()
}

fn default_font_path() -> String {
    String::new()
}

fn default_override_text_style() -> String {
    String::new()
}

fn default_log_level() -> String {
    "Error".to_string()
}

fn default_theme_mode() -> String {
    "Auto".to_string()
}

fn default_contrast_level() -> String {
    "Normal".to_string()
}

fn default_theme_name() -> String {
    "default".to_string()
}

fn default_display_size() -> String {
    "Desktop (1024x768)".to_string()
}

fn default_hybridanalysis_tag_ignorelist() -> String {
    "rat, jrat".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            virustotal_apikey: String::new(),
            hybridanalysis_apikey: String::new(),
            virustotal_submit: false,
            hybridanalysis_submit: false,
            hybridanalysis_tag_ignorelist: default_hybridanalysis_tag_ignorelist(),
            show_logs: false,
            log_level: default_log_level(),
            theme_mode: default_theme_mode(),
            contrast_level: default_contrast_level(),
            display_size: default_display_size(),
            google_play_renderer: false,
            fdroid_renderer: false,
            apkmirror_renderer: false,
            apkmirror_email: String::new(),
            apkmirror_name: String::new(),
            apkmirror_auto_upload: false,
            language: default_language(),
            font_path: default_font_path(),
            override_text_style: default_override_text_style(),
            theme_name: default_theme_name(),
            unsafe_app_remove: false,
            autoupdate: false,
        }
    }
}

impl Config {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "android")]
        {
            // Android-specific paths
            let config_dir = PathBuf::from("/data/data/pe.nikescar.uad_shizuku/files");
            let cache_dir = PathBuf::from("/data/data/pe.nikescar.uad_shizuku/cache");
            let download_dir = PathBuf::from("/data/data/pe.nikescar.uad_shizuku/downloads");
            let db_dir = PathBuf::from("/data/data/pe.nikescar.uad_shizuku/dbs");
            let tmp_dir = PathBuf::from("/data/data/pe.nikescar.uad_shizuku/tmp");

            log::info!("Android config paths - config_dir: {:?}, cache_dir: {:?}, download_dir: {:?}, db_dir: {:?}, tmp_dir: {:?}", config_dir, cache_dir, download_dir, db_dir, tmp_dir);

            // Create directories if they don't exist
            match fs::create_dir_all(&config_dir) {
                Ok(()) => log::info!("Successfully created config_dir: {:?}", config_dir),
                Err(e) => log::error!(
                    "Failed to create config_dir: {:?} - Error: {}",
                    config_dir,
                    e
                ),
            }
            match fs::create_dir_all(&cache_dir) {
                Ok(()) => log::info!("Successfully created cache_dir: {:?}", cache_dir),
                Err(e) => log::error!("Failed to create cache_dir: {:?} - Error: {}", cache_dir, e),
            }
            match fs::create_dir_all(&download_dir) {
                Ok(()) => log::info!("Successfully created download_dir: {:?}", download_dir),
                Err(e) => log::error!(
                    "Failed to create download_dir: {:?} - Error: {}",
                    download_dir,
                    e
                ),
            }
            match fs::create_dir_all(&db_dir) {
                Ok(()) => log::info!("Successfully created db_dir: {:?}", db_dir),
                Err(e) => log::error!("Failed to create db_dir: {:?} - Error: {}", db_dir, e),
            }
            match fs::create_dir_all(&tmp_dir) {
                Ok(()) => log::info!("Successfully created tmp_dir: {:?}", tmp_dir),
                Err(e) => log::error!("Failed to create tmp_dir: {:?} - Error: {}", tmp_dir, e),
            }

            Ok(Config {
                config_dir,
                cache_dir,
                download_dir,
                db_dir,
                tmp_dir,
            })
        }

        #[cfg(not(target_os = "android"))]
        {
            let proj_dirs = ProjectDirs::from("pe", "nikescar", "uad_shizuku")
                .context("Failed to get project directories")?;

            let config_dir = proj_dirs.config_dir().to_path_buf();
            let cache_dir = config_dir.join("cache");
            let download_dir = config_dir.join("downloads");
            let db_dir = config_dir.join("dbs");
            let tmp_dir = config_dir.join("tmp");

            // Create directories if they don't exist
            fs::create_dir_all(&config_dir)?;
            fs::create_dir_all(&cache_dir)?;
            fs::create_dir_all(&download_dir)?;
            fs::create_dir_all(&db_dir)?;
            fs::create_dir_all(&tmp_dir)?;

            Ok(Config {
                config_dir,
                cache_dir,
                download_dir,
                db_dir,
                tmp_dir,
            })
        }
    }

    pub fn load_settings(&self) -> Result<Settings> {
        let settings_path = self.config_dir.join("settings.txt");

        if !settings_path.exists() {
            return Ok(Settings::default());
        }

        let contents =
            fs::read_to_string(&settings_path).context("Failed to read settings file")?;

        let settings: Settings =
            serde_json::from_str(&contents).context("Failed to parse settings JSON")?;

        Ok(settings)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        let settings_path = self.config_dir.join("settings.txt");

        let json =
            serde_json::to_string_pretty(settings).context("Failed to serialize settings")?;

        fs::write(&settings_path, json).context("Failed to write settings file")?;

        log::info!("Settings saved to {:?}", settings_path);
        Ok(())
    }
}

pub fn init_i18n() {
    let en_us = String::from_utf8_lossy(include_bytes!("../assets/languages/fluent/en-US.ftl"));
    let ko_kr = String::from_utf8_lossy(include_bytes!("../assets/languages/fluent/ko-KR.ftl"));

    egui_i18n::load_translations_from_text("en-US", en_us).unwrap();
    egui_i18n::load_translations_from_text("ko-KR", ko_kr).unwrap();

    // Detect system language instead of hardcoding en-US
    let system_language = match sys_locale::get_locale().as_deref() {
        Some("ko_KR") | Some("ko-KR") | Some("ko") => "ko-KR",
        Some("en_US") | Some("en-US") | Some("en_GB") | Some("en-GB") | Some("en") | _ => "en-US",
    };
    egui_i18n::set_language(system_language);
    egui_i18n::set_fallback("en-US");
}

/// Detect narrow screens. This is used to show a simpler UI on mobile devices,
/// especially for the web demo at <https://egui.rs>.
pub fn is_mobile(ctx: &egui::Context) -> bool {
    let screen_size = ctx.content_rect().size();
    screen_size.x < 1081.0
}

/// Minimum viewport width for desktop table view
pub const DESKTOP_MIN_WIDTH: f32 = 1008.0;

/// Base table width for calculating column ratios
pub const BASE_TABLE_WIDTH: f32 = 1024.0;

/// Check if a package ID has at least 2 domain levels (e.g., com.example)
/// Package IDs with less than 2 levels (e.g., com.android) are typically system
/// packages that won't be found on app stores or malware databases.
pub fn is_valid_package_id(package_id: &str) -> bool {
    package_id.split('.').count() >= 2
}

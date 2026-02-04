#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

use eframe::egui;

mod adb;
pub mod adb_stt;
mod android_packagemanager;
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

pub mod api_apkmirror;
pub mod api_apkmirror_stt;
pub mod api_fdroid;
pub mod api_fdroid_stt;
mod api_googleplay;
pub mod api_googleplay_stt;
pub mod api_hybridanalysis;
pub mod api_hybridanalysis_stt;
mod api_virustotal;
pub mod api_virustotal_stt;
mod calc_apkmirror;
pub mod calc_apkmirror_stt;
mod calc_fdroid;
pub mod calc_fdroid_stt;
mod calc_googleplay;
pub mod calc_googleplay_stt;
mod calc_hybridanalysis;
pub mod calc_hybridanalysis_stt;
mod calc_izzyrisk;
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
pub mod svg_stt;

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
}

#[allow(dead_code)]
fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "en-US".to_string()
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

fn default_display_size() -> String {
    "Desktop (1024x768)".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            virustotal_apikey: String::new(),
            hybridanalysis_apikey: String::new(),
            virustotal_submit: false,
            hybridanalysis_submit: false,
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

            log::info!("Android config paths - config_dir: {:?}, cache_dir: {:?} download_dir: {:?}, db_dir: {:?}, tmp_dir: {:?}", config_dir, cache_dir, download_dir, db_dir, tmp_dir);

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

        tracing::info!("Settings saved to {:?}", settings_path);
        Ok(())
    }
}

pub fn init_i18n() {
    let en_us = String::from_utf8_lossy(include_bytes!("../assets/languages/fluent/en-US.ftl"));
    let ko_kr = String::from_utf8_lossy(include_bytes!("../assets/languages/fluent/ko-KR.ftl"));

    egui_i18n::load_translations_from_text("en-US", en_us).unwrap();
    egui_i18n::load_translations_from_text("ko-KR", ko_kr).unwrap();

    egui_i18n::set_language("en-US");
    egui_i18n::set_fallback("en-US");
}

/// Detect narrow screens. This is used to show a simpler UI on mobile devices,
/// especially for the web demo at <https://egui.rs>.
pub fn is_mobile(ctx: &egui::Context) -> bool {
    let screen_size = ctx.screen_rect().size();
    screen_size.x < 1081.0
}

/// Check if a package ID has at least 2 domain levels (e.g., com.example)
/// Package IDs with less than 2 levels (e.g., com.android) are typically system
/// packages that won't be found on app stores or malware databases.
pub fn is_valid_package_id(package_id: &str) -> bool {
    package_id.split('.').count() >= 2
}

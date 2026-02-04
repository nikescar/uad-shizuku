#![doc(hidden)]

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Flag to track when "Retry Detection" button is clicked in ADB install dialog
static ADB_RETRY_REQUESTED: AtomicBool = AtomicBool::new(false);

use eframe::egui;
use egui_i18n::tr;
use egui_material3::menu::{Corner, FocusState, Positioning};
use egui_material3::{assist_chip, dialog, menu, menu_item, tabs_primary, MaterialButton};
use egui_material3::{get_global_theme, ContrastLevel, MaterialThemeContext, ThemeMode};

use crate::db::{
    flush_apkmirror, flush_fdroid, flush_googleplay, flush_hybridanalysis, flush_virustotal,
    invalidate_cache,
};
use crate::db_package_cache::get_cached_packages_with_apk;
use crate::models::PackageInfoCache;
use crate::svg_stt::*;

use crate::adb::{get_devices, get_users, UserInfo};
// use crate::android_packagemanager::get_installed_packages;
use crate::tab_apps_control::TabAppsControl;
use crate::tab_debloat_control::TabDebloatControl;
use crate::tab_scan_control::TabScanControl;
use crate::tab_usage_control::TabUsageControl;
use crate::LogLevel;

pub use crate::uad_shizuku_app_stt::*;
use crate::{Config, Settings};

use eframe::egui::Context;
use egui_material3::theme::{
    load_fonts, load_themes, setup_local_fonts, setup_local_fonts_from_bytes, setup_local_theme,
    update_window_background,
};
use std::sync::OnceLock;

/// Initialize common app components (database, i18n).
/// Call this early in main() before creating the app.
pub fn init_common() {
    // Set up database path before initializing anything that uses the database
    if let Ok(config) = Config::new() {
        let db_path = config.db_dir.join("uad.db");
        crate::db::set_db_path(db_path.to_string_lossy().to_string());
    }

    // Initialize VirusTotal database upsert queue
    // This must be called AFTER setting the database path
    crate::db_virustotal::init_upsert_queue();

    // Initialize Hybrid Analysis database upsert queue
    crate::db_hybridanalysis::init_upsert_queue();

    // Initialize i18n
    crate::init_i18n();
}

/// Initialize egui context with fonts, themes, and image loaders.
/// Call this in the eframe app creation callback.
pub fn init_egui(ctx: &Context) {
    setup_local_theme(Some("resources/material-theme.json"));
    setup_local_fonts_from_bytes("NotoSansKr", include_bytes!("../resources/noto-sans-kr.ttf"));
    egui_extras::install_image_loaders(ctx);
    load_fonts(ctx);
    load_themes();
    update_window_background(ctx);

    // Restore saved custom font if configured
    if let Ok(config) = Config::new() {
        if let Ok(settings) = config.load_settings() {
            if !settings.font_path.is_empty() {
                setup_local_fonts(Some(&settings.font_path));
                load_fonts(ctx);
            }
        }
    }
}

static LOG_BUFFER: OnceLock<Arc<Mutex<String>>> = OnceLock::new();
static LOG_SETTINGS: OnceLock<Arc<Mutex<LogSettings>>> = OnceLock::new();

// Get or initialize the log buffer
fn get_log_buffer() -> &'static Arc<Mutex<String>> {
    LOG_BUFFER.get_or_init(|| Arc::new(Mutex::new(String::new())))
}

// Get or initialize log settings
fn get_log_settings() -> &'static Arc<Mutex<LogSettings>> {
    LOG_SETTINGS.get_or_init(|| {
        Arc::new(Mutex::new(LogSettings {
            show_logs: false,
            log_level: LogLevel::Info,
        }))
    })
}

// Update log settings
pub fn update_log_settings(settings: LogSettings) {
    if let Ok(mut log_settings) = get_log_settings().lock() {
        *log_settings = settings;
    }
}

// Function to append to log buffer
pub fn append_log(level: &str, message: String) {
    // Check if this log level should be captured
    // Logic: Show messages at the selected level and all higher priority levels
    // Priority order: ERROR > WARN > INFO > DEBUG > TRACE
    let should_log = if let Ok(settings) = get_log_settings().lock() {
        let message_level = match level {
            "ERROR" => LogLevel::Error,
            "WARN" => LogLevel::Warn,
            "INFO" => LogLevel::Info,
            "DEBUG" => LogLevel::Debug,
            "TRACE" => LogLevel::Trace,
            _ => return, // Skip unknown levels
        };

        // Check if message level is at or above the selected log level
        let level_priority = |lvl: LogLevel| -> i32 {
            match lvl {
                LogLevel::Error => 0,
                LogLevel::Warn => 1,
                LogLevel::Info => 2,
                LogLevel::Debug => 3,
                LogLevel::Trace => 4,
            }
        };

        level_priority(message_level) <= level_priority(settings.log_level)
    } else {
        false
    };

    if !should_log {
        return;
    }

    if let Ok(mut buffer) = get_log_buffer().lock() {
        buffer.push_str(&message);
        buffer.push('\n');

        // Keep only last 10000 characters to prevent memory issues
        if buffer.len() > 10000 {
            *buffer = buffer.chars().skip(buffer.len() - 10000).collect();
        }
    }
}

pub trait View {
    fn ui(&mut self, ui: &mut egui::Ui);
}

impl Default for UadShizukuApp {
    fn default() -> Self {
        //

        // Log basic system info at app start
        tracing::info!("=== System Information ===");
        tracing::info!("OS: {}", std::env::consts::OS);
        tracing::info!("Architecture: {}", std::env::consts::ARCH);
        tracing::info!("Family: {}", std::env::consts::FAMILY);

        let adb_devices = Vec::<String>::new();
        // #[cfg(not(target_os = "android"))]
        // {
        //     let _adb_available = which::which("adb").is_ok();
        //     if !_adb_available {
        //         // Dialog will be shown automatically via adb_install_dialog_open field
        //         tracing::warn!("ADB is not available. Installation dialog will be shown.");
        //     } else {
        //         let _adb_devices = get_devices().unwrap_or_default();
        //     }
        // }

        let config = Config::new().ok();
        if let Some(ref cfg) = config {
            let db_path = cfg.db_dir.join("uad.db");
            crate::db::set_db_path(db_path.to_string_lossy().to_string());
        }
        let settings = if let Some(ref cfg) = config {
            cfg.load_settings().unwrap_or_default()
        } else {
            Settings::default()
        };
        let cache_dir = if let Some(ref cfg) = config {
            cfg.cache_dir.clone()
        } else {
            std::path::PathBuf::from("./cache")
        };
        let tmp_dir = if let Some(ref cfg) = config {
            cfg.tmp_dir.clone()
        } else {
            std::path::PathBuf::from("./tmp")
        };

        let mut app = Self {
            config: config,
            current_view: AppView::Debloat,
            shizuku_connected: false,

            title_text: "UAD-Shizuku".to_string(),
            show_navigation: false,
            show_actions: true,
            is_scrolled: false,
            custom_height: 64.0,
            use_custom_height: false,

            custom_selected: 0,

            items_button_rect: None,
            standard_menu_open: false,

            anchor_corner: Corner::BottomLeft,
            menu_corner: Corner::TopLeft,
            default_focus: FocusState::None,
            positioning: Positioning::Absolute,
            quick: false,
            has_overflow: false,
            stay_open_on_outside_click: false,
            stay_open_on_focusout: false,
            skip_restore_focus: false,
            x_offset: 0.0,
            y_offset: 0.0,
            no_horizontal_flip: false,
            no_vertical_flip: false,
            typeahead_delay: 200.0,
            list_tab_index: -1,

            disabled: false,

            adb_devices: adb_devices,
            selected_device: None,
            current_device: None,

            adb_users: Vec::<UserInfo>::new(),
            selected_user: None,
            current_user: None,

            // NOTE: installed_packages and uad_ng_lists are now in shared_store_stt::SharedStore

            tab_debloat_control: TabDebloatControl::default(),
            tab_scan_control: TabScanControl::default(),
            tab_usage_control: TabUsageControl::default(),
            tab_apps_control: TabAppsControl::new(cache_dir, tmp_dir),

            // Settings dialog
            settings_dialog_open: false,
            settings_virustotal_apikey: settings.virustotal_apikey.clone(),
            settings_hybridanalysis_apikey: settings.hybridanalysis_apikey.clone(),
            settings_invalidate_cache: false,
            settings_flush_virustotal: false,
            settings_flush_hybridanalysis: false,
            settings_flush_googleplay: false,
            settings_flush_fdroid: false,
            settings_flush_apkmirror: false,
            // Temporary settings for dialog (applied only on Save)
            settings_google_play_renderer: settings.google_play_renderer,
            settings_fdroid_renderer: settings.fdroid_renderer,
            settings_apkmirror_renderer: settings.apkmirror_renderer,
            settings_virustotal_submit: settings.virustotal_submit,
            settings_hybridanalysis_submit: settings.hybridanalysis_submit,
            settings: settings,

            package_load_progress: Arc::new(Mutex::new(None)),

            // ADB installation dialog (opens automatically if ADB not found)
            adb_install_dialog_open: which::which("adb").is_err(),

            // Disclaimer dialog (shows on startup)
            disclaimer_dialog_open: true,

            // About dialog
            about_dialog_open: false,

            // Font selector state
            system_fonts: Vec::new(),
            system_fonts_loaded: false,
            selected_font_display: String::new(),

            // Renderer state machines
            google_play_renderer: RendererStateMachine::default(),
            fdroid_renderer: RendererStateMachine::default(),
            apkmirror_renderer: RendererStateMachine::default(),
            google_play_queue: None,
            fdroid_queue: None,
            apkmirror_queue: None,

            // Package loading state
            package_loading_thread: None,
            package_loading_dialog_open: false,
            package_loading_status: String::new(),
        };

        // Apply persisted theme preferences
        app.apply_saved_theme_preferences();

        // Apply persisted language preferences
        app.apply_saved_language();

        // Initialize log settings from loaded settings
        update_log_settings(LogSettings {
            show_logs: app.settings.show_logs,
            log_level: Self::string_to_log_level(&app.settings.log_level),
        });

        // refresh adb devices list
        app.retrieve_adb_devices();
        
        app
    }
}

impl UadShizukuApp {
    fn apply_saved_theme_preferences(&self) {
        if let Ok(mut theme) = get_global_theme().lock() {
            theme.theme_mode = Self::string_to_theme_mode(&self.settings.theme_mode);
            theme.contrast_level = Self::string_to_contrast_level(&self.settings.contrast_level);
        }
    }

    fn apply_saved_language(&self) {
        if !self.settings.language.is_empty() {
            egui_i18n::set_language(&self.settings.language);
        }
    }

    fn apply_saved_text_style(&self, ctx: &egui::Context) {
        if !self.settings.override_text_style.is_empty() {
            let text_style = Self::string_to_text_style(&self.settings.override_text_style);
            ctx.style_mut(|s| {
                s.override_text_style = text_style;
            });
        }
    }

    /// Enumerate system TTF/OTF fonts by scanning platform-specific directories.
    /// Returns a sorted Vec of (display_name, file_path) tuples.
    fn enumerate_system_fonts() -> Vec<(String, String)> {
        let mut fonts: Vec<(String, String)> = Vec::new();

        let mut font_dirs: Vec<std::path::PathBuf> = Vec::new();

        if cfg!(target_os = "android") {
            font_dirs.push(std::path::PathBuf::from("/system/fonts"));
        } else if cfg!(target_os = "macos") {
            font_dirs.push(std::path::PathBuf::from("/Library/Fonts"));
            font_dirs.push(std::path::PathBuf::from("/System/Library/Fonts"));
            if let Some(home) = std::env::var_os("HOME") {
                font_dirs.push(std::path::PathBuf::from(home).join("Library/Fonts"));
            }
        } else if cfg!(target_os = "windows") {
            font_dirs.push(std::path::PathBuf::from("C:\\Windows\\Fonts"));
        } else {
            // Linux and other Unix
            font_dirs.push(std::path::PathBuf::from("/usr/share/fonts"));
            font_dirs.push(std::path::PathBuf::from("/usr/local/share/fonts"));
            if let Some(home) = std::env::var_os("HOME") {
                let home = std::path::PathBuf::from(home);
                font_dirs.push(home.join(".fonts"));
                font_dirs.push(home.join(".local/share/fonts"));
            }
        }

        for dir in &font_dirs {
            if dir.is_dir() {
                Self::scan_font_dir(dir, &mut fonts);
            }
        }

        fonts.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        fonts.dedup_by(|a, b| a.0 == b.0);
        fonts
    }

    /// Recursively scan a directory for TTF/OTF files.
    fn scan_font_dir(dir: &std::path::Path, fonts: &mut Vec<(String, String)>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::scan_font_dir(&path, fonts);
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if ext_lower == "ttf" || ext_lower == "otf" {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let display_name = stem.replace('-', " ").replace('_', " ");
                            fonts.push((display_name, path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }
    }

    /// Ensure system fonts are enumerated (lazy, cached).
    fn ensure_system_fonts_loaded(&mut self) {
        if !self.system_fonts_loaded {
            self.system_fonts = Self::enumerate_system_fonts();
            self.system_fonts_loaded = true;

            if self.settings.font_path.is_empty() {
                self.selected_font_display = "Default (NotoSansKr)".to_string();
            } else {
                self.selected_font_display = self
                    .system_fonts
                    .iter()
                    .find(|(_, path)| path == &self.settings.font_path)
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| "Default (NotoSansKr)".to_string());
            }
        }
    }

    fn string_to_theme_mode(value: &str) -> ThemeMode {
        match value {
            "Light" => ThemeMode::Light,
            "Dark" => ThemeMode::Dark,
            _ => ThemeMode::Auto,
        }
    }

    fn theme_mode_to_string(mode: ThemeMode) -> String {
        match mode {
            ThemeMode::Light => "Light".to_string(),
            ThemeMode::Dark => "Dark".to_string(),
            ThemeMode::Auto => "Auto".to_string(),
        }
    }

    fn string_to_contrast_level(value: &str) -> ContrastLevel {
        match value {
            "High" => ContrastLevel::High,
            "Medium" => ContrastLevel::Medium,
            _ => ContrastLevel::Normal,
        }
    }

    fn contrast_level_to_string(level: ContrastLevel) -> String {
        match level {
            ContrastLevel::High => "High".to_string(),
            ContrastLevel::Medium => "Medium".to_string(),
            ContrastLevel::Normal => "Normal".to_string(),
        }
    }

    fn string_to_log_level(value: &str) -> LogLevel {
        match value {
            "Error" => LogLevel::Error,
            "Warn" => LogLevel::Warn,
            "Info" => LogLevel::Info,
            "Debug" => LogLevel::Debug,
            "Trace" => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }

    fn log_level_to_string(level: LogLevel) -> String {
        match level {
            LogLevel::Error => "Error".to_string(),
            LogLevel::Warn => "Warn".to_string(),
            LogLevel::Info => "Info".to_string(),
            LogLevel::Debug => "Debug".to_string(),
            LogLevel::Trace => "Trace".to_string(),
        }
    }

    fn string_to_text_style(value: &str) -> Option<egui::TextStyle> {
        if value.is_empty() {
            return None;
        }
        match value {
            "Small" => Some(egui::TextStyle::Small),
            "Body" => Some(egui::TextStyle::Body),
            "Button" => Some(egui::TextStyle::Button),
            "Heading" => Some(egui::TextStyle::Heading),
            "Monospace" => Some(egui::TextStyle::Monospace),
            _ => None,
        }
    }

    fn text_style_to_string(style: &Option<egui::TextStyle>) -> String {
        match style {
            None => String::new(),
            Some(s) => s.to_string(),
        }
    }

    pub fn update(&mut self, _ctx: &egui::Context, _frame: &eframe::Frame) {
        tracing::debug!("update function is called.");
    }

    fn get_theme(&self) -> MaterialThemeContext {
        if let Ok(theme) = get_global_theme().lock() {
            theme.clone()
        } else {
            MaterialThemeContext::default()
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let mut theme = self.get_theme();

        let mut visuals = match theme.theme_mode {
            ThemeMode::Light => egui::Visuals::light(),
            ThemeMode::Dark => egui::Visuals::dark(),
            ThemeMode::Auto => {
                // Use system preference or default to light
                if ctx.style().visuals.dark_mode {
                    theme.theme_mode = ThemeMode::Dark; // Resolve Auto to Dark for color lookup
                    egui::Visuals::dark()
                } else {
                    theme.theme_mode = ThemeMode::Light; // Resolve Auto to Light for color lookup
                    egui::Visuals::light()
                }
            }
        };

        // Apply Material Design 3 colors if theme is loaded
        let primary_color = theme.get_primary_color();
        let on_primary = theme.get_on_primary_color();
        let surface = theme.get_surface_color(visuals.dark_mode);
        let on_surface = theme.get_color_by_name("onSurface");

        // Apply colors to visuals
        visuals.selection.bg_fill = primary_color;
        visuals.selection.stroke.color = primary_color;
        visuals.hyperlink_color = primary_color;

        // Button and widget colors
        visuals.widgets.noninteractive.bg_fill = surface;

        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_unmultiplied(
            primary_color.r(),
            primary_color.g(),
            primary_color.b(),
            20,
        );

        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_unmultiplied(
            primary_color.r(),
            primary_color.g(),
            primary_color.b(),
            40,
        );

        visuals.widgets.active.bg_fill = primary_color;
        visuals.widgets.active.fg_stroke.color = on_primary;

        // Window background
        visuals.window_fill = surface;
        visuals.panel_fill = theme.get_color_by_name("surfaceContainer");

        // Text colors
        visuals.override_text_color = Some(on_surface);

        // Apply surface colors
        visuals.extreme_bg_color = theme.get_color_by_name("surfaceContainerLowest");

        ctx.set_visuals(visuals);
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Apply theme at the start of UI rendering
        self.apply_theme(ui.ctx());
        
        // Apply saved text style
        self.apply_saved_text_style(ui.ctx());

        // === top app bar area start
        ui.horizontal(|ui| {
            let items_button = ui.add(MaterialButton::filled("☰"));
            self.items_button_rect = Some(items_button.rect);
            if items_button.clicked() {
                // Toggle menu instead of just opening
                self.standard_menu_open = !self.standard_menu_open;
                // ui.ctx().request_repaint(); // Repaint when menu state changes
            }
            self.show_menus(ui.ctx());

            ui.vertical(|ui| {
                ui.heading(tr!("app-title"));
                ui.label(tr!("app-description"));
            });

            {
                ui.label(tr!("devices"));
                let selected_text = self
                    .selected_device
                    .clone()
                    .unwrap_or_else(|| tr!("select-device"));

                let combo_response = egui::ComboBox::from_label("")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        for (_i, device) in self.adb_devices.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.selected_device,
                                Some(device.clone()),
                                device,
                            );
                        }
                    });

                if self.adb_devices.is_empty() && combo_response.response.clicked() {
                    self.retrieve_adb_devices();
                }

                // Update users list when device selection changes
                if self.selected_device != self.current_device {
                    tracing::debug!("device selection changed to {:?}", self.selected_device);
                    self.current_device = self.selected_device.clone();
                    self.retrieve_adb_users();
                    // Reset user selection when device changes
                    self.selected_user = None;
                    self.current_user = None;
                    self.retrieve_installed_packages();
                }

                // User selection ComboBox
                ui.label(tr!("users"));
                let user_selected_text = if let Some(user_id) = self.selected_user {
                    if let Some(user_info) = self.adb_users.iter().find(|u| u.user_id == user_id) {
                        format!("User {} ({})", user_id, user_info.name)
                    } else {
                        format!("User {}", user_id)
                    }
                } else {
                    tr!("all-users")
                };

                egui::ComboBox::from_label(" ")
                    .selected_text(user_selected_text)
                    .show_ui(ui, |ui| {
                        // Add "All Users" option
                        ui.selectable_value(&mut self.selected_user, None, tr!("all-users"));

                        // Add individual users
                        for user in &self.adb_users {
                            let label = format!("User {} ({})", user.user_id, user.name);
                            ui.selectable_value(&mut self.selected_user, Some(user.user_id), label);
                        }
                    });

                // Retrieve installed packages when user selection changes
                if self.selected_user != self.current_user {
                    tracing::debug!("user selection changed to {:?}", self.selected_user);
                    self.current_user = self.selected_user;
                    self.retrieve_installed_packages();
                }

                // Update device list on button click
                let refresh_chip = assist_chip("")
                    .leading_icon_svg(REFRESH_SVG)
                    .elevated(false);

                if ui
                    .add(refresh_chip.on_click(|| {
                        tracing::info!("Refresh devices clicked");
                    }))
                    .clicked()
                {
                    self.retrieve_adb_devices();
                }

                // Show progress bar if packages are loading
                let debloat_progress_value =
                    if let Ok(debloat_progress) = self.package_load_progress.lock() {
                        *debloat_progress
                    } else {
                        None
                    };

                if let Some(p) = debloat_progress_value {
                    let debloat_progress_bar = egui::ProgressBar::new(p)
                        .show_percentage()
                        .desired_width(100.0)
                        .animate(true);
                    ui.add(debloat_progress_bar)
                        .on_hover_text(tr!("loading-packages"));
                }
            }
        });
        // === top app bar area end

        // === notification render progress area start
        ui.horizontal(|ui| {
            // Google Play renderer progress
            if self.google_play_renderer.is_enabled {
                if let Some(queue) = &self.google_play_queue {
                    let pending = queue.queue_size();
                    let completed = queue.completed_count();
                    if pending > 0 {
                        let total = pending + completed;
                        let progress = completed as f32 / total as f32;
                        let progress_bar = egui::ProgressBar::new(progress)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("rendering-google-play"));
                        ui.add(progress_bar)
                            .on_hover_text(tr!("google-play-renderer"));
                        if ui.button(tr!("stop")).clicked() {
                            tracing::info!("Stop Google Play renderer clicked");
                            queue.clear_queue();
                        }
                    }
                }
            }

            // F-Droid renderer progress
            if self.fdroid_renderer.is_enabled {
                if let Some(queue) = &self.fdroid_queue {
                    let pending = queue.queue_size();
                    let completed = queue.completed_count();
                    if pending > 0 {
                        let total = pending + completed;
                        let progress = completed as f32 / total as f32;
                        let progress_bar = egui::ProgressBar::new(progress)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("rendering-fdroid"));
                        ui.add(progress_bar)
                            .on_hover_text(tr!("fdroid-renderer"));
                        if ui.button(tr!("stop")).clicked() {
                            tracing::info!("Stop F-Droid renderer clicked");
                            queue.clear_queue();
                        }
                    }
                }
            }

            // APKMirror renderer progress
            if self.apkmirror_renderer.is_enabled {
                if let Some(queue) = &self.apkmirror_queue {
                    let pending = queue.queue_size();
                    let completed = queue.completed_count();
                    if pending > 0 {
                        let total = pending + completed;
                        let progress = completed as f32 / total as f32;
                        let progress_bar = egui::ProgressBar::new(progress)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("rendering-apkmirror"));
                        ui.add(progress_bar)
                            .on_hover_text(tr!("apkmirror-renderer"));
                        if ui.button(tr!("stop")).clicked() {
                            tracing::info!("Stop APKMirror renderer clicked");
                            queue.clear_queue();
                        }
                    }
                }
            }
        });
        // === notification render progress area end

        // === tab area start
        self.render_custom_tabs(ui);
        // === tab area end

        // === logs area start
        if self.settings.show_logs {
            // Add vertical spacer to push logs to bottom
            self.render_logs(ui);
        }
        // === logs area end

        // === settings dialog
        self.show_settings_dialog(ui.ctx());
        // === settings dialog end

        // === ADB installation dialog
        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "windows"
        ))]
        self.show_adb_install_dialog(ui.ctx());
        // === ADB installation dialog end

        // === Disclaimer dialog
        self.show_disclaimer_dialog(ui.ctx());
        // === Disclaimer dialog end

        // === About dialog
        self.show_about_dialog(ui.ctx());
        // === About dialog end

        // === Package loading dialog
        self.show_package_loading_dialog(ui.ctx());
        self.handle_package_loading_result();
        // === Package loading dialog end

        
    }

    fn show_menus(&mut self, ctx: &egui::Context) {
        // Standard Menu with Items - opens below button (default positioning)
        if self.standard_menu_open {
            let close_menu = Cell::new(false);
            let should_exit = Cell::new(false);
            let open_settings = Cell::new(false);
            let open_about = Cell::new(false);
            let settings_text = tr!("settings");
            let about_text = tr!("about");
            let exit_text = tr!("exit");
            let settings_item = self.create_menu_item(&settings_text, "settings", || {
                println!("Settings clicked!");
                open_settings.set(true);
                close_menu.set(true);
            });
            let about_item = self.create_menu_item(&about_text, "info", || {
                println!("About clicked!");
                open_about.set(true);
                close_menu.set(true);
            });
            let exit_item = self.create_menu_item(&exit_text, "exit", || {
                close_menu.set(true);
                should_exit.set(true);
                println!("Exit clicked!");
            });

            let mut menu_builder = menu("standard_menu", &mut self.standard_menu_open)
                .item(settings_item)
                .item(about_item)
                .item(exit_item)
                .anchor_corner(self.anchor_corner)
                .menu_corner(self.menu_corner)
                .default_focus(self.default_focus)
                .positioning(self.positioning)
                .quick(self.quick)
                .has_overflow(self.has_overflow)
                .stay_open_on_outside_click(self.stay_open_on_outside_click)
                .stay_open_on_focusout(self.stay_open_on_focusout)
                .skip_restore_focus(self.skip_restore_focus)
                .x_offset(self.x_offset)
                .y_offset(self.y_offset)
                .no_horizontal_flip(self.no_horizontal_flip)
                .no_vertical_flip(self.no_vertical_flip)
                .typeahead_delay(self.typeahead_delay)
                .list_tab_index(self.list_tab_index);

            if let Some(rect) = self.items_button_rect {
                menu_builder = menu_builder.anchor_rect(rect);
            }

            menu_builder.show(ctx);

            if close_menu.get() {
                self.standard_menu_open = false;
            }

            if open_settings.get() {
                // Sync temporary settings from current settings when opening dialog
                self.settings_google_play_renderer = self.settings.google_play_renderer;
                self.settings_fdroid_renderer = self.settings.fdroid_renderer;
                self.settings_apkmirror_renderer = self.settings.apkmirror_renderer;
                self.settings_virustotal_submit = self.settings.virustotal_submit;
                self.settings_hybridanalysis_submit = self.settings.hybridanalysis_submit;
                self.settings_dialog_open = true;
            }

            if open_about.get() {
                self.about_dialog_open = true;
            }

            if should_exit.get() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn create_menu_item<'a, F>(
        &self,
        text: &'a str,
        _id: &str,
        on_click: F,
    ) -> egui_material3::MenuItem<'a>
    where
        F: Fn() + 'a,
    {
        let mut item = menu_item(text).on_click(on_click);
        if self.disabled {
            item = item.enabled(false);
        }
        item
    }

    fn render_custom_tabs(&mut self, ui: &mut egui::Ui) {
        // Custom themed tabs
        let _previous_tab = self.custom_selected;
        ui.add(
            tabs_primary(&mut self.custom_selected)
                .id_salt("custom_primary")
                .tab(tr!("debloat"))
                .tab(tr!("scan"))
                .tab(tr!("apps")),
                //.tab(tr!("usage")),
        );

        // Enhanced content with custom styling
        ui.add_space(10.0);

        // Calculate max height: leave 200px for log box if enabled
        let reserved_space = if self.settings.show_logs { 200.0 } else { 0.0 };
        let max_height = ui.available_height() - reserved_space;

        match self.custom_selected {
            0 => {
                ui.label(tr!("debloat-description"));

                ui.add_space(8.0);

                egui::ScrollArea::both()
                    .id_salt("debloat_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        self.render_debloat_tab(ui);
                    });
            }
            1 => {
                ui.horizontal(|ui| {
                    ui.label(tr!("scan-description"));
                    ui.add_space(8.0);
                    if self.settings.virustotal_apikey.is_empty()
                        || self.settings.hybridanalysis_apikey.is_empty()
                    {
                        ui.label(tr!("set-api-keys"));
                    }
                });
                ui.add_space(8.0);

                egui::ScrollArea::both()
                    .id_salt("scan_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        self.render_scan_tab(ui);
                    });
            }
            2 => {
                // ui.colored_label(egui::Color32::from_rgb(103, 80, 164), "Apps");
                ui.label(tr!("apps-description"));
                ui.add_space(8.0);

                egui::ScrollArea::both()
                    .id_salt("apps_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        self.render_apps_tab(ui);
                    });
            }
            3 => {
                // ui.colored_label(egui::Color32::from_rgb(103, 80, 164), "Usage");
                ui.label(tr!("usage-description"));
                ui.add_space(8.0);

                egui::ScrollArea::both()
                    .id_salt("usage_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        self.render_usage_tab(ui);
                    });
            }
            _ => {
                ui.colored_label(egui::Color32::from_rgb(103, 80, 164), "");
            }
        }
    }

    fn render_debloat_tab(&mut self, ui: &mut egui::Ui) {
        use crate::tab_debloat_control::AdbResult;

        // Manage Google Play renderer state machine
        self.google_play_renderer.is_enabled = self.settings.google_play_renderer;

        // Manage F-Droid renderer state machine
        self.fdroid_renderer.is_enabled = self.settings.fdroid_renderer;

        // Manage APKMirror renderer state machine
        self.apkmirror_renderer.is_enabled = self.settings.apkmirror_renderer;

        // Get renderer enabled flags for UI
        let google_play_enabled = self.google_play_renderer.is_enabled;
        let fdroid_enabled = self.fdroid_renderer.is_enabled;
        let apkmirror_enabled = self.apkmirror_renderer.is_enabled;

        // Initialize and start worker queues if renderers are enabled
        let db_path = self.config.as_ref().map(|c| c.db_dir.to_string_lossy().to_string()).unwrap_or_default();
        
        if google_play_enabled && self.google_play_queue.is_none() {
            let queue = std::sync::Arc::new(crate::calc_googleplay::GooglePlayQueue::new());
            queue.start_worker(db_path.clone());
            self.google_play_queue = Some(queue);
        }
        if fdroid_enabled && self.fdroid_queue.is_none() {
            let queue = std::sync::Arc::new(crate::calc_fdroid::FDroidQueue::new());
            queue.start_worker(db_path.clone());
            self.fdroid_queue = Some(queue);
        }
        if apkmirror_enabled && self.apkmirror_queue.is_none() {
            let queue = std::sync::Arc::new(crate::calc_apkmirror::ApkMirrorQueue::new());
            queue.set_email(self.settings.apkmirror_email.clone());
            queue.start_worker(db_path.clone());
            self.apkmirror_queue = Some(queue);
        }

        // Enqueue visible packages for fetching
        self.enqueue_visible_packages_for_debloat(google_play_enabled, fdroid_enabled, apkmirror_enabled);

        // Load results from workers and populate caches
        self.load_renderer_results_to_debloat_cache();

        if let Some(result) = self.tab_debloat_control.ui(
            ui,
            google_play_enabled,
            fdroid_enabled,
            apkmirror_enabled,
        ) {
            match result {
                AdbResult::Success(_pkg_name) => {
                    // Package already removed in tab_debloat_control
                }
                AdbResult::Failure => {
                    // Open log box if it's closed
                    if !self.settings.show_logs {
                        self.settings.show_logs = true;
                        // Update global log settings
                        update_log_settings(LogSettings {
                            show_logs: true,
                            log_level: Self::string_to_log_level(&self.settings.log_level),
                        });
                    }
                }
            }
        }

        // NOTE: Cached app info is now in shared_store_stt::SharedStore
        // Both tabs access the same shared cache, no need to sync
    }

    fn enqueue_visible_packages_for_debloat(
        &mut self,
        google_play_enabled: bool,
        fdroid_enabled: bool,
        apkmirror_enabled: bool,
    ) {
        use crate::shared_store_stt::get_shared_store;
        let store = get_shared_store();

        // Separate packages into system and non-system
        let mut non_system_packages = Vec::new();
        let mut system_packages = Vec::new();

        let installed_packages = store.get_installed_packages();
        for package in &installed_packages {
            if package.flags.contains("SYSTEM") {
                system_packages.push(package.pkg.clone());
            } else {
                non_system_packages.push(package.pkg.clone());
            }
        }

        // Enqueue non-system packages for Google Play and F-Droid
        for pkg_id in non_system_packages {
            // Skip if already cached
            if google_play_enabled && store.get_cached_google_play_app(&pkg_id).is_none() {
                if let Some(queue) = &self.google_play_queue {
                    queue.enqueue(pkg_id.clone());
                }
            }
            if fdroid_enabled && store.get_cached_fdroid_app(&pkg_id).is_none() {
                if let Some(queue) = &self.fdroid_queue {
                    queue.enqueue(pkg_id.clone());
                }
            }
        }

        // Enqueue system packages for APKMirror
        if apkmirror_enabled {
            for pkg_id in system_packages {
                if store.get_cached_apkmirror_app(&pkg_id).is_none() {
                    if let Some(queue) = &self.apkmirror_queue {
                        queue.enqueue(pkg_id);
                    }
                }
            }
        }
    }

    fn load_renderer_results_to_debloat_cache(&mut self) {
        use crate::shared_store_stt::get_shared_store;
        let store = get_shared_store();

        // Collect visible packages to check
        let mut visible_packages = Vec::new();
        let installed_packages = store.get_installed_packages();
        for package in &installed_packages {
            visible_packages.push((package.pkg.clone(), package.flags.contains("SYSTEM")));
        }

        // Check Google Play results for non-system packages
        if let Some(queue) = &self.google_play_queue {
            for (pkg_id, is_system) in &visible_packages {
                if *is_system || store.get_cached_google_play_app(pkg_id).is_some() {
                    continue;
                }
                if let Some(status) = queue.get_status(pkg_id) {
                    match status {
                        crate::calc_googleplay_stt::FetchStatus::Success(app) => {
                            self.tab_debloat_control.update_cached_google_play(pkg_id.clone(), app);
                        }
                        crate::calc_googleplay_stt::FetchStatus::Error(_) => {
                            // Cache 404 placeholder
                            use crate::models::GooglePlayApp;
                            let placeholder = GooglePlayApp {
                                id: 0,
                                package_id: pkg_id.clone(),
                                title: String::new(),
                                developer: String::new(),
                                version: None,
                                icon_base64: None,
                                score: None,
                                installs: None,
                                updated: None,
                                raw_response: "404".to_string(),
                                created_at: 0,
                                updated_at: 0,
                            };
                            self.tab_debloat_control.update_cached_google_play(pkg_id.clone(), placeholder);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Check F-Droid results for non-system packages
        if let Some(queue) = &self.fdroid_queue {
            for (pkg_id, is_system) in &visible_packages {
                if *is_system || store.get_cached_fdroid_app(pkg_id).is_some() {
                    continue;
                }
                if let Some(status) = queue.get_status(pkg_id) {
                    match status {
                        crate::calc_fdroid_stt::FDroidFetchStatus::Success(app) => {
                            self.tab_debloat_control.update_cached_fdroid(pkg_id.clone(), app);
                        }
                        crate::calc_fdroid_stt::FDroidFetchStatus::Error(_) => {
                            // Cache 404 placeholder
                            use crate::models::FDroidApp;
                            let placeholder = FDroidApp {
                                id: 0,
                                package_id: pkg_id.clone(),
                                title: String::new(),
                                developer: String::new(),
                                version: None,
                                icon_base64: None,
                                description: None,
                                license: None,
                                updated: None,
                                raw_response: "404".to_string(),
                                created_at: 0,
                                updated_at: 0,
                            };
                            self.tab_debloat_control.update_cached_fdroid(pkg_id.clone(), placeholder);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Check APKMirror results for system packages
        if let Some(queue) = &self.apkmirror_queue {
            for (pkg_id, is_system) in &visible_packages {
                if !is_system || store.get_cached_apkmirror_app(pkg_id).is_some() {
                    continue;
                }
                if let Some(status) = queue.get_status(pkg_id) {
                    match status {
                        crate::calc_apkmirror_stt::ApkMirrorFetchStatus::Success(app) => {
                            self.tab_debloat_control.update_cached_apkmirror(pkg_id.clone(), app);
                        }
                        crate::calc_apkmirror_stt::ApkMirrorFetchStatus::Error(_) => {
                            // Cache 404 placeholder
                            use crate::models::ApkMirrorApp;
                            let placeholder = ApkMirrorApp {
                                id: 0,
                                package_id: pkg_id.clone(),
                                title: String::new(),
                                developer: String::new(),
                                version: None,
                                icon_url: None,
                                icon_base64: None,
                                raw_response: "404".to_string(),
                                created_at: 0,
                                updated_at: 0,
                            };
                            self.tab_debloat_control.update_cached_apkmirror(pkg_id.clone(), placeholder);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn render_scan_tab(&mut self, ui: &mut egui::Ui) {
        // Sync renderer settings from Settings to TabScanControl
        self.tab_scan_control.google_play_renderer_enabled = self.settings.google_play_renderer;
        self.tab_scan_control.fdroid_renderer_enabled = self.settings.fdroid_renderer;
        self.tab_scan_control.apkmirror_renderer_enabled = self.settings.apkmirror_renderer;

        self.tab_scan_control.ui(ui);
    }

    fn render_apps_tab(&mut self, ui: &mut egui::Ui) {
        // Check if packages need to be refreshed (e.g., after install)
        // Only refresh tab_apps_control, not other tabs (to avoid triggering scans)
        if self.tab_apps_control.refresh_pending {
            self.tab_apps_control.refresh_pending = false;
            self.refresh_apps_tab_packages();
        }
        let has_error = self.tab_apps_control.ui(ui);

        // Open log window automatically if an error occurred
        if has_error && !self.settings.show_logs {
            self.settings.show_logs = true;
            update_log_settings(LogSettings {
                show_logs: true,
                log_level: Self::string_to_log_level(&self.settings.log_level),
            });
        }
    }

    /// Lightweight refresh that only updates tab_apps_control's package list
    /// Does not trigger updates to tab_scan_control or tab_debloat_control
    fn refresh_apps_tab_packages(&mut self) {
        {
            use crate::adb::get_all_packages_fingerprints;
            use crate::shared_store_stt::get_shared_store;

            if let Some(ref device) = self.selected_device {
                tracing::debug!("Refreshing packages for apps tab only...");
                match get_all_packages_fingerprints(device) {
                    Ok(packages) => {
                        let store = get_shared_store();
                        store.set_installed_packages(packages.clone());
                        self.tab_apps_control.update_packages(packages);
                        tracing::debug!("Apps tab packages refreshed");
                    }
                    Err(e) => {
                        tracing::error!("Failed to refresh packages for apps tab: {}", e);
                    }
                }
            }
        }
    }

    fn render_usage_tab(&mut self, ui: &mut egui::Ui) {
        self.tab_usage_control.ui(ui);
    }

    fn render_logs(&mut self, ui: &mut egui::Ui) {
        // put blank space before logs
        // Calculate max height: leave 200px for log box if enabled
        let reserved_space = if self.settings.show_logs { 200.0 } else { 0.0 };
        let max_height = ui.available_height() - reserved_space;

        // put blank space if self.installed_packages is empty
        if max_height > 0.0 {
            ui.add_space(max_height);
        }

        // Read from global log buffer
        let log_text = if let Ok(buffer) = get_log_buffer().lock() {
            buffer.clone()
        } else {
            String::from("Unable to access logs")
        };

        // Use top_down layout within this section to keep content in correct order
        ui.label(tr!("logs"));
        // Create a scrollable text area for logs
        egui::ScrollArea::vertical()
            .id_salt("logs_scroll")
            .max_height(150.0)
            .min_scrolled_height(150.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut log_text.as_str())
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .interactive(false)
                        .desired_rows(10),
                );
            });
    }

    fn retrieve_adb_devices(&mut self) {
        {
            // clear current selections
            self.selected_device = None;
            self.current_device = None;
            self.adb_users.clear();
            self.selected_user = None;
            self.current_user = None;
            {
                use crate::shared_store_stt::get_shared_store;
                get_shared_store().set_installed_packages(Vec::new());
            }
            self.tab_debloat_control.update_packages(Vec::new());
            self.tab_debloat_control.update_uad_ng_lists(UadNgLists {
                apps: HashMap::new(),
            });
            self.tab_scan_control.update_packages(Vec::new());
            self.tab_scan_control.update_uad_ng_lists(UadNgLists {
                apps: HashMap::new(),
            });
            self.tab_apps_control.update_packages(Vec::new());

            // match kill_server() {
            //     Ok(output) => {
            //         tracing::debug!("ADB server killed: {}", output);
            //     }
            //     Err(e) => {
            //         tracing::error!("Failed to kill ADB server: {}", e);
            //     }
            // }

            // root_get_permission();

            match get_devices() {
                Ok(devices) => {
                    self.adb_devices = devices;

                    self.retrieve_adb_users();
                }
                Err(e) => {
                    tracing::error!("[ERROR] Failed to get ADB devices: {}", e);
                    self.adb_devices.clear();
                }
            }
        }
    }

    fn retrieve_adb_users(&mut self) {
        if let Some(ref device) = self.selected_device {
            tracing::debug!("Retrieving users for device: {}", device);
            match get_users(device) {
                Ok(users) => {
                    tracing::debug!("Successfully retrieved {} users", users.len());
                    self.adb_users = users;

                    self.retrieve_installed_packages();
                }
                Err(e) => {
                    tracing::error!("Failed to get users: {}", e);
                    self.adb_users.clear();
                }
            }
        } else {
            tracing::debug!("No device selected, skipping user retrieval");
            self.adb_users.clear();
        }
    }

    fn retrieve_installed_packages(&mut self) {
        // Load uad_ng_lists after struct is constructed
        self.retrieve_uad_ng_lists();

        let Some(device) = self.selected_device.clone() else {
            tracing::debug!("No device selected, skipping package retrieval");
            return;
        };

        // Open loading dialog
        self.package_loading_dialog_open = true;
        self.package_loading_status = tr!("loading-packages");

        // Clone necessary data for the async task
        let selected_user = self.selected_user;
        let debloat_progress = self.package_load_progress.clone();
        let shared_store = crate::shared_store_stt::get_shared_store();
        let uad_ng_lists = shared_store.uad_ng_lists.lock().unwrap().clone();

        // Start background thread
        let handle = std::thread::spawn(move || {
            use crate::adb::get_all_packages_fingerprints;
            use crate::db_package_cache::upsert_package_info_cache;

            tracing::debug!("Retrieving installed packages for device: {}", device);

            // Step 1: Get package fingerprints (lightweight)
            let parsed_packages = match get_all_packages_fingerprints(&device) {
                Ok(fp) => fp,
                Err(e) => {
                    tracing::error!("Failed to get package fingerprints: {}", e);
                    return (Vec::new(), None);
                }
            };
            tracing::debug!("Retrieved {} package fingerprints", parsed_packages.len());

            // Step 2: load all contents from get_cached_packages_with_apk, db_package_cache
            let cached_packages: Vec<PackageInfoCache> = get_cached_packages_with_apk(&device);
            tracing::debug!(
                "Loaded {} cached packages from database",
                cached_packages.len()
            );

            // Step 3: fill apk path and sha256sum using background worker
            let parsed_packages_for_thread = parsed_packages.clone();
            let device_for_thread = device.to_string();
            let debloat_progress_clone = debloat_progress.clone();

            // Initialize debloat_progress
            if let Ok(mut p) = debloat_progress_clone.lock() {
                *p = Some(0.0);
            }

            std::thread::spawn(move || {
                tracing::info!("fill apk path and sha256sum from all packages -f");
                if cached_packages.len() < parsed_packages_for_thread.len() / 2 {
                    match crate::adb::get_all_packages_sha256sum(&device_for_thread) {
                        Ok(package_data) => {
                            tracing::info!(
                                "Retrieved sha256 sums for {} packages",
                                package_data.len()
                            );
                            // Convert Vec<(String, String, String)> to HashMap for easier lookup
                            let sha256_map: std::collections::HashMap<
                                String,
                                (String, String),
                            > = package_data
                                .into_iter()
                                .map(|(pkg, sha256, path)| (pkg, (sha256, path)))
                                .collect();

                            let total = parsed_packages_for_thread.len();
                            for (i, pkg) in parsed_packages_for_thread.iter().enumerate() {
                                // Update debloat_progress
                                if let Ok(mut p) = debloat_progress_clone.lock() {
                                    *p = Some(i as f32 / total as f32);
                                }

                                if let Some((sha256, apk_path)) = sha256_map.get(&pkg.pkg) {
                                    // insert into db
                                    match upsert_package_info_cache(
                                        &pkg.pkg,
                                        &pkg.pkgChecksum,
                                        &pkg.dumpText,
                                        &pkg.codePath,
                                        pkg.versionCode,
                                        &pkg.versionName,
                                        "", // first_install_time - not available from this data
                                        &pkg.lastUpdateTime,
                                        Some(apk_path.as_str()),
                                        Some(sha256.as_str()),
                                        None, // izzyscore - calculated separately
                                        &device_for_thread,
                                    ) {
                                        Ok(_) => {
                                            tracing::debug!(
                                                "Cached package info for {}: {} ({})",
                                                pkg.pkg,
                                                sha256,
                                                apk_path
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to cache package info for {}: {}",
                                                pkg.pkg,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to get package sha256 sums: {}", e);
                        }
                    }
                }
                // Clear progress when done
                if let Ok(mut p) = debloat_progress_clone.lock() {
                    *p = None;
                }
            });

            // use package
            let mut packages = parsed_packages;

            // Filter packages by selected user if a specific user is selected
            if let Some(user_id) = selected_user {
                tracing::debug!("Filtering packages for user: {}", user_id);
                packages
                    .retain(|pkg| pkg.users.iter().any(|u| u.userId == user_id && u.installed));
                tracing::debug!(
                    "Filtered to {} packages for user {}",
                    packages.len(),
                    user_id
                );
            } else {
                tracing::debug!("Showing all users' packages");
            }

            tracing::debug!("Package retrieval complete");
            (packages, uad_ng_lists)
        });

        self.package_loading_thread = Some(handle);
    }

    fn handle_package_loading_result(&mut self) {
        // Check if thread is complete
        let should_check = self.package_loading_thread.is_some();
        if !should_check {
            return;
        }

        // Try to take the thread handle and check if it's finished
        if let Some(handle) = self.package_loading_thread.take() {
            if handle.is_finished() {
                // Thread is complete, get the result
                match handle.join() {
                    Ok((packages, uad_lists)) => {
                        // Loading complete, update UI
                        tracing::debug!("Applying loaded packages to UI");
                        
                        let shared_store = crate::shared_store_stt::get_shared_store();
                        {
                            let mut installed_pkgs = shared_store.installed_packages.lock().unwrap();
                            *installed_pkgs = packages.clone();
                        }
                        self.tab_debloat_control.update_packages(packages.clone());
                        
                        if let Some(lists) = uad_lists {
                            self.tab_debloat_control.update_uad_ng_lists(lists.clone());
                            self.tab_scan_control.update_uad_ng_lists(lists);
                        }
                        
                        self.tab_debloat_control
                            .set_selected_device(self.selected_device.clone());

                        // Update TabScanControl with API key, device serial, and settings
                        self.tab_scan_control.vt_api_key = Some(self.settings.virustotal_apikey.clone());
                        self.tab_scan_control.ha_api_key =
                            Some(self.settings.hybridanalysis_apikey.clone());
                        self.tab_scan_control.device_serial = self.selected_device.clone();
                        self.tab_scan_control.virustotal_submit_enabled = self.settings.virustotal_submit;
                        self.tab_scan_control.hybridanalysis_submit_enabled =
                            self.settings.hybridanalysis_submit;
                        tracing::info!(
                            "Synced hybridanalysis_submit_enabled={} to tab_scan_control",
                            self.settings.hybridanalysis_submit
                        );

                        let installed_packages = shared_store.installed_packages.lock().unwrap().clone();
                        self.tab_scan_control
                            .update_packages(installed_packages.clone());

                        self.tab_apps_control
                            .update_packages(installed_packages.clone());
                        self.tab_apps_control
                            .set_selected_device(self.selected_device.clone());
                        tracing::debug!("Updated tab controls with packages");

                        // Close dialog
                        self.package_loading_dialog_open = false;
                    }
                    Err(e) => {
                        tracing::error!("Package loading thread panicked: {:?}", e);
                        self.package_loading_dialog_open = false;
                    }
                }
            } else {
                // Thread not finished yet, put it back
                self.package_loading_thread = Some(handle);
            }
        }
    }

    // another lists https://github.com/MuntashirAkon/android-debloat-list
    fn retrieve_uad_ng_lists(&mut self) {
        const UAD_LISTS_URL: &str = "https://raw.githubusercontent.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation/refs/heads/main/resources/assets/uad_lists.json";
        const UAD_LISTS_FILENAME: &str = "uad_lists.json";

        // Get cache directory from config
        let cache_dir = match &self.config {
            Some(config) => config.cache_dir.clone(),
            None => {
                tracing::error!("Config not available, cannot retrieve UAD lists");
                return;
            }
        };

        let cache_file_path = cache_dir.join(UAD_LISTS_FILENAME);

        // Check if file exists in cache or is older than 7 days
        let should_download = !cache_file_path.exists() || {
            cache_file_path
                .metadata()
                .and_then(|m| m.modified())
                .map(|modified| {
                    modified
                        .elapsed()
                        .map(|elapsed| elapsed.as_secs() > 7 * 24 * 60 * 60)
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        };

        if should_download {
            tracing::info!(
                "UAD lists not found in cache or older than 7 days, downloading from {}",
                UAD_LISTS_URL
            );

            // Download the file
            let request = ehttp::Request::get(UAD_LISTS_URL);
            let (sender, receiver) = std::sync::mpsc::channel();

            ehttp::fetch(request, move |result| {
                sender.send(result).ok();
            });

            // Wait for the response (blocking)
            match receiver.recv() {
                Ok(Ok(response)) => {
                    if response.ok {
                        // Save to cache
                        match std::fs::write(&cache_file_path, &response.bytes) {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully downloaded and cached UAD lists to {:?}",
                                    cache_file_path
                                );
                            }
                            Err(e) => {
                                tracing::error!("Failed to write UAD lists to cache: {}", e);
                                return;
                            }
                        }
                    } else {
                        tracing::error!("Failed to download UAD lists: HTTP {}", response.status);
                        return;
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!("Failed to download UAD lists: {}", e);
                    return;
                }
                Err(e) => {
                    tracing::error!("Failed to receive download response: {}", e);
                    return;
                }
            }
        } else {
            tracing::info!("UAD lists found in cache at {:?}", cache_file_path);
        }

        // Load and parse the JSON file
        match std::fs::read_to_string(&cache_file_path) {
            Ok(json_content) => match serde_json::from_str::<UadNgLists>(&json_content) {
                Ok(uad_lists) => {
                    tracing::info!(
                        "Successfully parsed UAD lists with {} apps",
                        uad_lists.apps.len()
                    );
                    let shared_store = crate::shared_store_stt::get_shared_store();
                    {
                        let mut lists = shared_store.uad_ng_lists.lock().unwrap();
                        *lists = Some(uad_lists);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse UAD lists JSON: {}", e);
                }
            },
            Err(e) => {
                tracing::error!("Failed to read UAD lists from cache: {}", e);
            }
        }
    }

    // Flags : https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/content/pm/ApplicationInfo.java
    // Permissions : https://developer.android.com/reference/android/Manifest.permission
    // Stalkerware IOC : https://github.com/AssoEchap/stalkerware-indicators
    fn show_settings_dialog(&mut self, ctx: &egui::Context) {
        if self.settings_dialog_open {
            // Lazy-load system fonts before dialog borrows self.settings_dialog_open
            self.ensure_system_fonts_loaded();

            let save_clicked = Cell::new(false);

            dialog(
                "settings_dialog",
                "Settings",
                &mut self.settings_dialog_open,
            )
            .content(|ui| {
                ui.vertical(|ui| {
                    ui.add_space(8.0);

                    // Language Selector
                    ui.horizontal(|ui| {
                        ui.label(tr!("language"));
                        let current_lang = egui_i18n::get_language();
                        let mut selected_lang = current_lang.to_string();

                        egui::ComboBox::from_label("   ")
                            .selected_text(match selected_lang.as_str() {
                                "en-US" => "English",
                                "ko-KR" => "Korean",
                                _ => &selected_lang,
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut selected_lang,
                                    "en-US".to_string(),
                                    "English",
                                );
                                ui.selectable_value(
                                    &mut selected_lang,
                                    "ko-KR".to_string(),
                                    "Korean",
                                );
                            });

                        if selected_lang != current_lang {
                            egui_i18n::set_language(&selected_lang);
                            self.settings.language = selected_lang;
                        }
                    
                        ui.add_space(8.0);
                        // Font Selector
                    
                        ui.label(tr!("font"));

                        let mut selected = self.selected_font_display.clone();

                        egui::ComboBox::from_id_salt("font_selector")
                            .selected_text(&selected)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut selected,
                                    "Default (NotoSansKr)".to_string(),
                                    "Default (NotoSansKr)",
                                );
                                for (display_name, _path) in &self.system_fonts {
                                    ui.selectable_value(
                                        &mut selected,
                                        display_name.clone(),
                                        display_name.as_str(),
                                    );
                                }
                            });

                        if selected != self.selected_font_display {
                            self.selected_font_display = selected.clone();

                            if selected == "Default (NotoSansKr)" {
                                self.settings.font_path = String::new();
                            } else if let Some((_, path)) = self
                                .system_fonts
                                .iter()
                                .find(|(name, _)| name == &selected)
                            {
                                self.settings.font_path = path.clone();
                            }

                            // Apply font immediately using free functions
                            use egui_material3::theme::{
                                load_fonts, setup_local_fonts, setup_local_fonts_from_bytes,
                            };
                            if self.settings.font_path.is_empty() {
                                setup_local_fonts_from_bytes(
                                    "NotoSansKr",
                                    include_bytes!("../resources/noto-sans-kr.ttf"),
                                );
                            } else {
                                setup_local_fonts(Some(&self.settings.font_path));
                            }
                            load_fonts(ui.ctx());
                        }

                        ui.add_space(8.0);
                        // Text Style Override Selector

                        ui.horizontal(|ui|{
                            let mut override_text_style = ui.style().override_text_style.clone();
                            ui.label(tr!("text-style"));
                            egui::ComboBox::from_id_salt("override_text_style")
                                .selected_text(match &override_text_style {
                                    None => "None".to_owned(),
                                    Some(s) => s.to_string(),
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut override_text_style,
                                        None,
                                        "None",
                                    );
                                    let all_text_styles = ui.style().text_styles();
                                    for style in all_text_styles {
                                        let text = egui::RichText::new(style.to_string())
                                            .text_style(style.clone());
                                        ui.selectable_value(
                                            &mut override_text_style,
                                            Some(style),
                                            text,
                                        );
                                    }
                                });
                            let text_style = override_text_style.clone();
                            ui.ctx().style_mut(|s| {
                                s.override_text_style = text_style.clone();
                            });
                            
                            // Save to settings when changed
                            let style_string = match text_style {
                                None => String::new(),
                                Some(s) => s.to_string(),
                            };
                            if style_string != self.settings.override_text_style {
                                self.settings.override_text_style = style_string;
                            }
                        });
                    });
                    ui.add_space(8.0);

                    // Display Size Selector
                    ui.horizontal(|ui| {
                        ui.label(tr!("display-size"));
                        let display_sizes = vec![
                            // ("Phone (412x732)", (412.0, 732.0)),
                            // ("Tablet (768x1024)", (768.0, 1024.0)),
                            ("Desktop (1024x768)", (1024.0, 768.0)),
                            ("1080p (1920x1080)", (1920.0, 1080.0)),
                        ];
                        let mut selected_size = self.settings.display_size.clone();
                        egui::ComboBox::from_label("  ")
                            .selected_text(&selected_size)
                            .show_ui(ui, |ui| {
                                for (label, _size) in &display_sizes {
                                    ui.selectable_value(
                                        &mut selected_size,
                                        label.to_string(),
                                        *label,
                                    );
                                }
                            });

                        if selected_size != self.settings.display_size {
                            self.settings.display_size = selected_size.clone();

                            // Find the corresponding size and resize the window
                            if let Some((_, size)) = display_sizes
                                .iter()
                                .find(|(label, _)| *label == selected_size)
                            {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                    egui::vec2(size.0, size.1),
                                ));
                                tracing::info!("Window resized to {}x{}", size.0, size.1);
                            }
                        }
                        ui.add_space(8.0);

                        // Color Mode Selector
                        ui.label(tr!("color-mode"));
                        if let Ok(mut theme) = get_global_theme().lock() {
                            // Light mode button
                            let light_selected = theme.theme_mode == ThemeMode::Light;
                            let light_button =
                                ui.selectable_label(light_selected, tr!("light-mode"));
                            if light_button.clicked() {
                                theme.theme_mode = ThemeMode::Light;
                                self.settings.theme_mode =
                                    Self::theme_mode_to_string(ThemeMode::Light);
                            }

                            // Auto mode button
                            let auto_selected = theme.theme_mode == ThemeMode::Auto;
                            let auto_button = ui.selectable_label(auto_selected, tr!("auto-mode"));
                            if auto_button.clicked() {
                                theme.theme_mode = ThemeMode::Auto;
                                self.settings.theme_mode =
                                    Self::theme_mode_to_string(ThemeMode::Auto);
                            }

                            // Dark mode button
                            let dark_selected = theme.theme_mode == ThemeMode::Dark;
                            let dark_button = ui.selectable_label(dark_selected, tr!("dark-mode"));
                            if dark_button.clicked() {
                                theme.theme_mode = ThemeMode::Dark;
                                self.settings.theme_mode =
                                    Self::theme_mode_to_string(ThemeMode::Dark);
                            }
                        }

                        ui.add_space(8.0);

                        // Contrast Level Selector
                        ui.label(tr!("contrast"));
                        if let Ok(mut theme) = get_global_theme().lock() {
                            // High contrast button
                            let high_selected = theme.contrast_level == ContrastLevel::High;
                            let high_button =
                                ui.selectable_label(high_selected, tr!("contrast-high"));
                            if high_button.clicked() {
                                theme.contrast_level = ContrastLevel::High;
                                self.settings.contrast_level =
                                    Self::contrast_level_to_string(ContrastLevel::High);
                            }

                            // Medium contrast button
                            let medium_selected = theme.contrast_level == ContrastLevel::Medium;
                            let medium_button =
                                ui.selectable_label(medium_selected, tr!("contrast-medium"));
                            if medium_button.clicked() {
                                theme.contrast_level = ContrastLevel::Medium;
                                self.settings.contrast_level =
                                    Self::contrast_level_to_string(ContrastLevel::Medium);
                            }

                            // Normal contrast button
                            let normal_selected = theme.contrast_level == ContrastLevel::Normal;
                            let normal_button =
                                ui.selectable_label(normal_selected, tr!("contrast-normal"));
                            if normal_button.clicked() {
                                theme.contrast_level = ContrastLevel::Normal;
                                self.settings.contrast_level =
                                    Self::contrast_level_to_string(ContrastLevel::Normal);
                            }
                        }
                    });
                    
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(tr!("virustotal-api-key"));
                        ui.text_edit_singleline(&mut self.settings_virustotal_apikey);
                        ui.hyperlink_to(
                            tr!("get-api-key"),
                            "https://www.virustotal.com/gui/my-apikey",
                        );
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.settings_virustotal_submit,
                            tr!("allow-virustotal-upload"),
                        );
                        ui.label(tr!("virustotal-upload-desc"));
                        ui.checkbox(&mut self.settings_flush_virustotal, tr!("flush"));
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(tr!("hybridanalysis-api-key"));
                        ui.text_edit_singleline(&mut self.settings_hybridanalysis_apikey);
                        ui.hyperlink_to(
                            tr!("get-api-key"),
                            "https://hybrid-analysis.com/my-account",
                        );
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.settings_hybridanalysis_submit,
                            tr!("allow-hybridanalysis-upload"),
                        );
                        ui.label(tr!("hybridanalysis-upload-desc"));
                        ui.checkbox(&mut self.settings_flush_hybridanalysis, tr!("flush"));
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.settings_google_play_renderer,
                            tr!("google-play-renderer"),
                        );
                        ui.label(tr!("google-play-renderer-desc"));
                        ui.checkbox(&mut self.settings_flush_googleplay, tr!("flush"));
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.settings_fdroid_renderer, tr!("fdroid-renderer"));
                        ui.label(tr!("fdroid-renderer-desc"));
                        ui.checkbox(&mut self.settings_flush_fdroid, tr!("flush"));
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.settings_apkmirror_renderer,
                            tr!("apkmirror-renderer"),
                        );
                        ui.label(tr!("apkmirror-renderer-desc"));
                        ui.checkbox(&mut self.settings_flush_apkmirror, tr!("flush"));
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.settings.apkmirror_auto_upload,
                            tr!("apkmirror-auto-upload"),
                        );
                        ui.label(tr!("apkmirror-auto-upload-desc"));
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(tr!("apkmirror-email"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings.apkmirror_email)
                                .desired_width(200.0)
                                .hint_text(tr!("email-hint")),
                        );
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(tr!("apkmirror-name"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings.apkmirror_name)
                                .desired_width(200.0)
                                .hint_text(tr!("name-hint")),
                        );
                    });                    

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.settings_invalidate_cache, tr!("invalidate-cache"));
                        ui.label(tr!("invalidate-cache-desc"));
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(tr!("show-logs"));
                        ui.checkbox(&mut self.settings.show_logs, tr!("show"));

                        let current_level = Self::string_to_log_level(&self.settings.log_level);

                        let error_selected = current_level == LogLevel::Error;
                        let error_button = ui.selectable_label(error_selected, "ERROR");
                        if error_button.clicked() {
                            self.settings.log_level = Self::log_level_to_string(LogLevel::Error);
                        }

                        let warn_selected = current_level == LogLevel::Warn;
                        let warn_button = ui.selectable_label(warn_selected, "WARN");
                        if warn_button.clicked() {
                            self.settings.log_level = Self::log_level_to_string(LogLevel::Warn);
                        }

                        let info_selected = current_level == LogLevel::Info;
                        let info_button = ui.selectable_label(info_selected, "INFO");
                        if info_button.clicked() {
                            self.settings.log_level = Self::log_level_to_string(LogLevel::Info);
                        }

                        let debug_selected = current_level == LogLevel::Debug;
                        let debug_button = ui.selectable_label(debug_selected, "DEBUG");
                        if debug_button.clicked() {
                            self.settings.log_level = Self::log_level_to_string(LogLevel::Debug);
                        }

                        let trace_selected = current_level == LogLevel::Trace;
                        let trace_button = ui.selectable_label(trace_selected, "TRACE");
                        if trace_button.clicked() {
                            self.settings.log_level = Self::log_level_to_string(LogLevel::Trace);
                        }
                    });

                    ui.add_space(8.0);
                });
            })
            .action(tr!("cancel"), || {
                tracing::info!("Settings dialog Cancel clicked!");
            })
            .primary_action(tr!("save"), || {
                tracing::info!("Settings dialog Save clicked!");
                save_clicked.set(true);
            })
            .show(ctx);

            // Handle save after dialog is shown
            if save_clicked.get() {
                self.save_settings();
            }
        }
    }

    fn show_package_loading_dialog(&mut self, ctx: &egui::Context) {
        if self.package_loading_dialog_open {
            dialog(
                "package_loading_dialog",
                &tr!("loading-packages"),
                &mut self.package_loading_dialog_open,
            )
            .content(|ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(400.0);
                });
            })
            .show(ctx);

            // Request repaint to keep dialog updating
            ctx.request_repaint();
        }
    }

    fn show_adb_install_dialog(&mut self, ctx: &egui::Context) {
        // Handle retry request from previous frame
        if ADB_RETRY_REQUESTED.swap(false, Ordering::SeqCst) {
            if which::which("adb").is_err() {
                // ADB still not found, reopen dialog
                self.adb_install_dialog_open = true;
            } else {
                // ADB found after retry
                tracing::info!("ADB detected after retry");
                self.retrieve_adb_devices();
                self.retrieve_adb_users();
                self.retrieve_installed_packages();
            }
        }

        if self.adb_install_dialog_open {
            dialog(
                "adb_install_dialog",
                "ADB Not Found - Installation Instructions",
                &mut self.adb_install_dialog_open,
            )
            .content(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(400.0);

                    ui.add_space(8.0);

                    // Platform detection
                    let os = std::env::consts::OS;
                    let platform_name = match os {
                        "windows" => "Windows",
                        "macos" => "macOS",
                        "linux" => "Linux",
                        _ => os,
                    };

                    ui.label(format!("Detected platform: {}", platform_name));
                    ui.add_space(8.0);
                    ui.label("ADB (Android Debug Bridge) is required but not found in your system PATH.");
                    ui.add_space(16.0);

                    ui.label("Please follow the installation guide to install ADB:");
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.hyperlink_to(
                            "Installation Guide (English)",
                            "https://uad-shizuku.pages.dev/docs/installation",
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.hyperlink_to(
                            "설치 가이드 (한국어)",
                            "https://uad-shizuku.pages.dev/docs/kr/docs/installation",
                        );
                    });

                    ui.add_space(16.0);
                });
            })
            .action(tr!("close"), || {})
            .primary_action("Retry Detection", || {
                ADB_RETRY_REQUESTED.store(true, Ordering::SeqCst);
            })
            .show(ctx);

            // Check if ADB became available (user may have installed it)
            if which::which("adb").is_ok() {
                self.adb_install_dialog_open = false;
                tracing::info!("ADB detected, closing installation dialog");
                self.retrieve_adb_devices();
                self.retrieve_adb_users();
                self.retrieve_installed_packages();
            }
        }
    }

    fn show_disclaimer_dialog(&mut self, ctx: &egui::Context) {
        // Disclaimer dialog
        if self.disclaimer_dialog_open {
            let disclaimer_title = tr!("disclaimer-title");
            let disclaimer_no_user_data = tr!("disclaimer-no-user-data");
            let disclaimer_uninstall_warning = tr!("disclaimer-uninstall-warning");

            dialog(
                "disclaimer_dialog",
                &disclaimer_title,
                &mut self.disclaimer_dialog_open,
            )
            .content(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(400.0);

                    ui.add_space(8.0);

                    ui.label(&disclaimer_no_user_data);

                    ui.add_space(8.0);

                    ui.label(&disclaimer_uninstall_warning);

                    ui.add_space(16.0);
                });
            })
            .action(tr!("ok"), || {})
            .show(ctx);
        }
    }

    fn show_about_dialog(&mut self, ctx: &egui::Context) {
        if self.about_dialog_open {
            let about_title = tr!("about");
            let version = env!("CARGO_PKG_VERSION");
            let description = tr!("about-description");
            let website_label = tr!("about-website");
            let credits_label = tr!("about-credits");

            dialog("about_dialog", &about_title, &mut self.about_dialog_open)
                .content(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(400.0);

                        ui.add_space(8.0);

                        // App name and version
                        ui.heading("UAD-Shizuku");
                        ui.label(format!("Version: {}", version));

                        ui.add_space(12.0);

                        // Description
                        ui.label(&description);

                        ui.add_space(12.0);

                        // Website
                        ui.horizontal(|ui| {
                            ui.label(format!("{}: ", website_label));
                            ui.hyperlink("https://uad-shizuku.pages.dev");
                        });

                        ui.add_space(12.0);

                        // Credits section
                        ui.label(egui::RichText::new(&credits_label).strong());
                        ui.add_space(4.0);

                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                ui.label("Reference Projects:");
                                ui.label("  - Universal Android Debloater Next Generation (GPL-3.0)");
                                ui.label("  - bevy_game_template (MIT/Apache-2.0)");
                                ui.label("  - android-activity (MIT/Apache-2.0)");
                                ui.label("  - ai-rules (Apache-2.0)");

                                ui.add_space(8.0);

                                ui.label("Key Libraries:");
                                ui.label("  - egui/eframe (MIT/Apache-2.0)");
                                ui.label("  - diesel (MIT/Apache-2.0)");
                                ui.label("  - serde (MIT/Apache-2.0)");
                                ui.label("  - tracing (MIT)");

                                ui.add_space(8.0);

                                ui.label("Assets:");
                                ui.label("  - Icons from SVG Repo (CC Attribution)");
                            });

                        ui.add_space(16.0);
                    });
                })
                .action(tr!("ok"), || {})
                .show(ctx);
        }
    }

    fn save_settings(&mut self) {
        // Sync theme selections into settings before persisting
        if let Ok(theme) = get_global_theme().lock() {
            self.settings.theme_mode = Self::theme_mode_to_string(theme.theme_mode);
            self.settings.contrast_level = Self::contrast_level_to_string(theme.contrast_level);
        }

        // Store old values for comparison
        let old_vt_apikey = self.settings.virustotal_apikey.clone();
        let old_ha_apikey = self.settings.hybridanalysis_apikey.clone();
        let old_vt_submit = self.settings.virustotal_submit;
        let old_ha_submit = self.settings.hybridanalysis_submit;
        let old_google_play_renderer = self.settings.google_play_renderer;
        let old_fdroid_renderer = self.settings.fdroid_renderer;
        let old_apkmirror_renderer = self.settings.apkmirror_renderer;
        let old_apkmirror_auto_upload = self.settings.apkmirror_auto_upload;

        // Update settings struct from temporary values
        self.settings.virustotal_apikey = self.settings_virustotal_apikey.clone();
        self.settings.hybridanalysis_apikey = self.settings_hybridanalysis_apikey.clone();
        self.settings.virustotal_submit = self.settings_virustotal_submit;
        self.settings.hybridanalysis_submit = self.settings_hybridanalysis_submit;
        self.settings.google_play_renderer = self.settings_google_play_renderer;
        self.settings.fdroid_renderer = self.settings_fdroid_renderer;
        self.settings.apkmirror_renderer = self.settings_apkmirror_renderer;

        // Sync submit settings to tab_scan_control
        self.tab_scan_control.virustotal_submit_enabled = self.settings.virustotal_submit;
        self.tab_scan_control.hybridanalysis_submit_enabled = self.settings.hybridanalysis_submit;

        // Check if VirusTotal API key was removed -> stop running scans
        if !old_vt_apikey.is_empty() && self.settings.virustotal_apikey.is_empty() {
            tracing::info!("VirusTotal API key removed, cancelling running scans");
            if let Ok(mut cancelled) = self.tab_scan_control.vt_scan_cancelled.lock() {
                *cancelled = true;
            }
        }

        // Check if HybridAnalysis API key was removed -> stop running scans
        if !old_ha_apikey.is_empty() && self.settings.hybridanalysis_apikey.is_empty() {
            tracing::info!("HybridAnalysis API key removed, cancelling running scans");
            if let Ok(mut cancelled) = self.tab_scan_control.ha_scan_cancelled.lock() {
                *cancelled = true;
            }
        }

        // Check if VirusTotal upload was disabled -> stop uploading
        if old_vt_submit && !self.settings.virustotal_submit {
            tracing::info!("VirusTotal upload disabled, cancelling uploads");
            if let Ok(mut cancelled) = self.tab_scan_control.vt_scan_cancelled.lock() {
                *cancelled = true;
            }
        }

        // Check if HybridAnalysis upload was disabled -> stop uploading
        if old_ha_submit && !self.settings.hybridanalysis_submit {
            tracing::info!("HybridAnalysis upload disabled, cancelling uploads");
            if let Ok(mut cancelled) = self.tab_scan_control.ha_scan_cancelled.lock() {
                *cancelled = true;
            }
        }

        // Check if Google Play renderer was disabled -> clear caches
        if old_google_play_renderer && !self.settings.google_play_renderer {
            tracing::info!("Google Play renderer disabled, clearing caches");
            self.google_play_renderer.is_enabled = false;
            let shared_store = crate::shared_store_stt::get_shared_store();
            shared_store.google_play_textures.lock().unwrap().clear();
            self.tab_scan_control.google_play_renderer_enabled = false;
        }

        // Check if F-Droid renderer was disabled -> clear caches
        if old_fdroid_renderer && !self.settings.fdroid_renderer {
            tracing::info!("F-Droid renderer disabled, clearing caches");
            self.fdroid_renderer.is_enabled = false;
            let shared_store = crate::shared_store_stt::get_shared_store();
            shared_store.fdroid_textures.lock().unwrap().clear();
            self.tab_scan_control.fdroid_renderer_enabled = false;
        }

        // Check if APKMirror renderer was disabled -> clear caches
        if old_apkmirror_renderer && !self.settings.apkmirror_renderer {
            tracing::info!("APKMirror renderer disabled, clearing caches");
            self.apkmirror_renderer.is_enabled = false;
            let shared_store = crate::shared_store_stt::get_shared_store();
            shared_store.apkmirror_textures.lock().unwrap().clear();
            self.tab_scan_control.apkmirror_renderer_enabled = false;
        }

        // Check if APKMirror auto upload was disabled
        if old_apkmirror_auto_upload && !self.settings.apkmirror_auto_upload {
            tracing::info!("APKMirror auto upload disabled");
        }

        // Check if Google Play renderer was enabled -> enable renderer
        if !old_google_play_renderer && self.settings.google_play_renderer {
            tracing::info!("Google Play renderer enabled");
            self.google_play_renderer.is_enabled = true;
            self.tab_scan_control.google_play_renderer_enabled = true;
        }

        // Check if F-Droid renderer was enabled -> enable renderer
        if !old_fdroid_renderer && self.settings.fdroid_renderer {
            tracing::info!("F-Droid renderer enabled");
            self.fdroid_renderer.is_enabled = true;
            self.tab_scan_control.fdroid_renderer_enabled = true;
        }

        // Check if APKMirror renderer was enabled -> enable renderer
        if !old_apkmirror_renderer && self.settings.apkmirror_renderer {
            tracing::info!("APKMirror renderer enabled");
            self.apkmirror_renderer.is_enabled = true;
            self.tab_scan_control.apkmirror_renderer_enabled = true;
        }

        // Check if VirusTotal API key was added -> start scan
        if old_vt_apikey.is_empty() && !self.settings.virustotal_apikey.is_empty() {
            tracing::info!("VirusTotal API key added, starting scan");
            self.tab_scan_control.vt_api_key = Some(self.settings.virustotal_apikey.clone());
            // Reset cancelled flag and trigger scan start via update_packages
            if let Ok(mut cancelled) = self.tab_scan_control.vt_scan_cancelled.lock() {
                *cancelled = false;
            }
            // Re-trigger scan by calling update_packages if packages are already loaded
            let shared_store = crate::shared_store_stt::get_shared_store();
            let installed_packages = shared_store.installed_packages.lock().unwrap().clone();
            if !installed_packages.is_empty() {
                self.tab_scan_control.update_packages(installed_packages);
            }
        }

        // Check if HybridAnalysis API key was added -> start scan
        if old_ha_apikey.is_empty() && !self.settings.hybridanalysis_apikey.is_empty() {
            tracing::info!("HybridAnalysis API key added, starting scan");
            self.tab_scan_control.ha_api_key = Some(self.settings.hybridanalysis_apikey.clone());
            // Reset cancelled flag and trigger scan start via update_packages
            if let Ok(mut cancelled) = self.tab_scan_control.ha_scan_cancelled.lock() {
                *cancelled = false;
            }
            // Re-trigger scan by calling update_packages if packages are already loaded
            let shared_store = crate::shared_store_stt::get_shared_store();
            let installed_packages = shared_store.installed_packages.lock().unwrap().clone();
            if !installed_packages.is_empty() {
                self.tab_scan_control.update_packages(installed_packages);
            }
        }

        if self.settings_invalidate_cache {
            invalidate_cache();
            self.settings_invalidate_cache = false;
        }

        // Flush individual tables if requested
        if self.settings_flush_virustotal {
            flush_virustotal();
            self.settings_flush_virustotal = false;
        }
        if self.settings_flush_hybridanalysis {
            flush_hybridanalysis();
            self.settings_flush_hybridanalysis = false;
        }
        if self.settings_flush_googleplay {
            flush_googleplay();
            self.settings_flush_googleplay = false;
        }
        if self.settings_flush_fdroid {
            flush_fdroid();
            self.settings_flush_fdroid = false;
        }
        if self.settings_flush_apkmirror {
            flush_apkmirror();
            self.settings_flush_apkmirror = false;
        }

        // Update log settings for in-app log display
        update_log_settings(LogSettings {
            show_logs: self.settings.show_logs,
            log_level: Self::string_to_log_level(&self.settings.log_level),
        });

        // Update tracing log level in real-time
        crate::log_capture::update_tracing_level(&self.settings.log_level.to_lowercase());

        // Save to file
        if let Some(ref config) = self.config {
            match config.save_settings(&self.settings) {
                Ok(_) => {
                    tracing::info!("Settings saved successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to save settings: {}", e);
                }
            }
        } else {
            tracing::error!("Config not available, cannot save settings");
        }
    }
}

impl View for UadShizukuApp {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.ui(ui);
    }
}

impl eframe::App for UadShizukuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }
}

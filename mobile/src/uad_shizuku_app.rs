#![doc(hidden)]

use std::cell::Cell;
use std::collections::HashMap;
use std::process;
use std::sync::{Arc, Mutex};
use sys_locale::get_locale;

use eframe::egui;
use egui_i18n::tr;
use egui_material3::menu::{Corner, FocusState, Positioning};
use egui_material3::{
    dashcounter, dialog, icon_button_standard, menu, menu_item, tabs_primary, MaterialButton,
};
use egui_material3::{get_global_theme, ContrastLevel, ThemeMode};

use crate::db::{
    flush_apkmirror, flush_fdroid, flush_googleplay, flush_hybridanalysis, flush_virustotal,
    invalidate_cache,
};
use crate::db_package_cache::get_cached_packages_with_apk;
use crate::material_symbol_icons::{ICON_INFO, ICON_REFRESH};
use crate::models::PackageInfoCache;

#[cfg(not(target_os = "android"))]
use crate::adb::get_devices;
use crate::adb::{get_users, UserInfo};
// use crate::android_packagemanager::get_installed_packages;
use crate::dlg_dashcounter_details::DlgDashCounterDetails;
use crate::tab_apps_control::TabAppsControl;
use crate::tab_debloat::TabDebloat;
use crate::tab_debloat_control::TabDebloatControl; // TRANSITIONAL: Being phased out
use crate::tab_scan_control::TabScanControl;
use crate::tab_usage_control::TabUsageControl;
use crate::LogLevel;

pub use crate::uad_shizuku_app_stt::*;
use crate::viewmodel::ViewModel;
use crate::{Config, Settings, BASE_TABLE_WIDTH, DESKTOP_MIN_WIDTH};

use crate::install;
#[cfg(not(target_os = "android"))]
use crate::install_stt::InstallStatus;

use eframe::egui::Context;
use egui_material3::theme::{
    apply_theme, load_fonts, load_theme_from_json_str, load_themes, set_contrast_level,
    set_theme_mode, setup_local_fonts, setup_local_fonts_from_bytes, setup_local_theme,
    update_window_background,
};
use std::sync::OnceLock;

// Embedded theme data
const THEME_GREEN: &str = include_str!("../resources/material-theme-green.json");
const THEME_LIGHTBLUE: &str = include_str!("../resources/material-theme-lightblue.json");
const THEME_LIGHTPINK: &str = include_str!("../resources/material-theme-lightpink.json");
const THEME_YELLOW: &str = include_str!("../resources/material-theme-yellow.json");

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
    // material icon fonts https://github.com/google/material-design-icons
    setup_local_fonts_from_bytes(
        "MaterialSymbolsOutlined",
        include_bytes!("../resources/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf"),
    );
    setup_local_fonts_from_bytes(
        "NotoSansKr",
        include_bytes!("../resources/noto-sans-kr.ttf"),
    );
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
        log::info!("=== System Information ===");
        log::info!("OS: {}", std::env::consts::OS);
        log::info!("Architecture: {}", std::env::consts::ARCH);
        log::info!("Family: {}", std::env::consts::FAMILY);

        let adb_devices = Vec::<String>::new();
        // #[cfg(not(target_os = "android"))]
        // {
        //     let _adb_available = which::which("adb").is_ok();
        //     if !_adb_available {
        //         // Dialog will be shown automatically via adb_install_dialog_open field
        //         log::warn!("ADB is not available. Installation dialog will be shown.");
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
            tab_debloat: TabDebloat::default(), // REFACTORED: New MVVM-based tab
            tab_debloat_control: TabDebloatControl::default(), // TRANSITIONAL: Being phased out
            tab_scan_control: TabScanControl::default(),
            tab_usage_control: TabUsageControl::default(),
            tab_apps_control: TabAppsControl::new(cache_dir, tmp_dir),

            settings: settings.clone(),

            // Dialog states
            dlg_settings: crate::dlg_settings_stt::DlgSettings {
                virustotal_apikey: settings.virustotal_apikey.clone(),
                hybridanalysis_apikey: settings.hybridanalysis_apikey.clone(),
                google_play_renderer: settings.google_play_renderer,
                fdroid_renderer: settings.fdroid_renderer,
                apkmirror_renderer: settings.apkmirror_renderer,
                virustotal_submit: settings.virustotal_submit,
                hybridanalysis_submit: settings.hybridanalysis_submit,
                hybridanalysis_tag_ignorelist: settings.hybridanalysis_tag_ignorelist.clone(),
                unsafe_app_remove: settings.unsafe_app_remove,
                expert_app_remove: settings.expert_app_remove,
                autoupdate: settings.autoupdate,
                ..Default::default()
            },

            package_load_progress: Arc::new(Mutex::new(None)),

            dlg_adb_install: crate::dlg_adb_install_stt::DlgAdbInstall {
                open: which::which("adb").is_err(),
                ..Default::default()
            },

            // Disclaimer dialog (shows on startup)
            disclaimer_dialog_open: true,

            dlg_about: crate::dlg_about_stt::DlgAbout::default(),
            dlg_update: crate::dlg_update_stt::DlgUpdate::default(),
            dlg_dashcounter_details: DlgDashCounterDetails::default(),

            // Installation status (desktop only)
            #[cfg(not(target_os = "android"))]
            install_status: install::check_install(),
            #[cfg(not(target_os = "android"))]
            install_dialog_open: false,
            #[cfg(not(target_os = "android"))]
            install_message: String::new(),

            // Update status (both desktop and Android)
            update_status: String::new(),
            update_available: false,
            update_checking: false,

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

            // First-run initialization flag
            first_update_done: false,

            // Shizuku state tracking
            shizuku_init_done: false,
            shizuku_permission_requested: false,
            shizuku_bind_requested: false,
            shizuku_error_message: None,

            // Pinch-to-zoom state
            zoom_factor: 1.0,

            // Dashboard counter scroll offsets
            dash_scroll_debloat: 0.0,
            dash_scroll_stalkerware: 0.0,
            dash_scroll_izzyrisk: 0.0,
            dash_scroll_virustotal: 0.0,
            dash_scroll_hybridanalysis: 0.0,
            dash_scroll_offa: 0.0,
            dash_scroll_fmhy: 0.0,

            // Installer package name - detect on Android
            #[cfg(target_os = "android")]
            installer_package_name: crate::android_packagemanager::get_installer_package_name(
                "pe.nikescar.uad_shizuku",
            )
            .ok()
            .flatten(),
            #[cfg(not(target_os = "android"))]
            installer_package_name: None,

            // Debloat tab performance optimization
            debloat_last_enqueued_version: 0,
            debloat_last_result_load_time: std::time::Instant::now(),

            // Tab controller state (shared between mobile and desktop UI)
            show_apps_tab: true,

            viewmodel: None, // Lazy initialization in update()
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

        // Don't call retrieve_adb_devices() here on Android - it will be called
        // on first update when the Android context is fully ready
        #[cfg(not(target_os = "android"))]
        crate::calc::retrieve_adb_devices(&mut app);

        app
    }
}

impl UadShizukuApp {
    fn apply_saved_theme_preferences(&self) {
        // Set theme mode and contrast level using utility functions
        set_theme_mode(self.settings.theme_mode.parse().unwrap_or(ThemeMode::Auto));
        set_contrast_level(
            self.settings
                .contrast_level
                .parse()
                .unwrap_or(ContrastLevel::Normal),
        );

        // Apply saved theme if not default
        if !self.settings.theme_name.is_empty() && self.settings.theme_name != "default" {
            self.apply_theme_by_name(&self.settings.theme_name);
        }
    }

    /// Detect system language using sys_locale and map to supported languages
    fn detect_system_language() -> String {
        match get_locale().as_deref() {
            Some("ko_KR") | Some("ko-KR") | Some("ko") => "ko-KR".to_string(),
            Some("en_US") | Some("en-US") | Some("en_GB") | Some("en-GB") | Some("en") | _ => {
                "en-US".to_string()
            }
        }
    }

    fn apply_saved_language(&self) {
        let language_to_apply =
            if self.settings.language == "Auto" || self.settings.language.is_empty() {
                Self::detect_system_language()
            } else {
                self.settings.language.clone()
            };
        egui_i18n::set_language(&language_to_apply);
    }

    fn apply_saved_text_style(&self, ctx: &egui::Context) {
        if !self.settings.override_text_style.is_empty() {
            let text_style = Self::string_to_text_style(&self.settings.override_text_style);
            ctx.style_mut(|s| {
                s.override_text_style = text_style;
            });
        }
    }

    /// Detect OS theme preference using dark-light crate (desktop) or Android JNI (Android)
    fn detect_os_theme() -> ThemeMode {
        #[cfg(target_os = "android")]
        {
            // Use Android JNI to get system theme mode
            match crate::android_contexttheme::get_ui_theme_mode() {
                Ok(crate::android_contexttheme::UiThemeMode::Dark) => {
                    log::debug!("Android system theme detected: Dark");
                    ThemeMode::Dark
                }
                Ok(crate::android_contexttheme::UiThemeMode::Light) => {
                    log::debug!("Android system theme detected: Light");
                    ThemeMode::Light
                }
                Ok(crate::android_contexttheme::UiThemeMode::Unspecified) | Err(_) => {
                    log::debug!("Android system theme unspecified or error, defaulting to Light");
                    ThemeMode::Light
                }
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            // Use dark-light crate for desktop platforms
            match dark_light::detect() {
                dark_light::Mode::Dark => ThemeMode::Dark,
                dark_light::Mode::Light => ThemeMode::Light,
                dark_light::Mode::Default => ThemeMode::Light, // Default to Light if unspecified
            }
        }
    }

    /// Get theme data by name
    fn get_theme_data_by_name(name: &str) -> Option<&'static str> {
        match name {
            "green" => Some(THEME_GREEN),
            "lightblue" => Some(THEME_LIGHTBLUE),
            "lightpink" => Some(THEME_LIGHTPINK),
            "yellow" => Some(THEME_YELLOW),
            _ => None,
        }
    }

    /// Apply theme by name
    fn apply_theme_by_name(&self, name: &str) {
        if let Some(theme_data) = Self::get_theme_data_by_name(name) {
            if let Err(e) = load_theme_from_json_str(theme_data) {
                log::error!("Failed to load theme '{}': {}", name, e);
            }
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
        log::debug!("update function is called.");
    }

    /// Sync scan progress from background threads to state machines
    /// This must be called before rendering progress bars to ensure they hide immediately
    fn sync_scan_progress(&mut self) {
        // Sync VirusTotal progress
        if let Ok(progress) = self.tab_scan_control.vt_scan_progress.lock() {
            if let Some(p) = *progress {
                self.tab_scan_control.vt_scan_state.update_progress(p);
            } else if self.tab_scan_control.vt_scan_state.is_running {
                self.tab_scan_control.vt_scan_state.complete();
            }
        }
        // Sync Hybrid Analysis progress
        if let Ok(progress) = self.tab_scan_control.ha_scan_progress.lock() {
            if let Some(p) = *progress {
                self.tab_scan_control.ha_scan_state.update_progress(p);
            } else if self.tab_scan_control.ha_scan_state.is_running {
                self.tab_scan_control.ha_scan_state.complete();
            }
        }
        // Sync IzzyRisk progress
        if let Ok(progress) = self.tab_scan_control.izzyrisk_scan_progress.lock() {
            if let Some(p) = *progress {
                self.tab_scan_control.izzyrisk_scan_state.update_progress(p);
            } else if self.tab_scan_control.izzyrisk_scan_state.is_running {
                self.tab_scan_control.izzyrisk_scan_state.complete();
            }
        }
        // Sync batch uninstall progress
        if let Ok(progress) = self.tab_debloat_control.batch_uninstall_progress.lock() {
            if let Some(p) = *progress {
                self.tab_debloat_control
                    .batch_uninstall_state
                    .update_progress(p);
            } else if self.tab_debloat_control.batch_uninstall_state.is_running {
                self.tab_debloat_control.batch_uninstall_state.complete();
            }
        }
        // Sync batch disable progress
        if let Ok(progress) = self.tab_debloat_control.batch_disable_progress.lock() {
            if let Some(p) = *progress {
                self.tab_debloat_control
                    .batch_disable_state
                    .update_progress(p);
            } else if self.tab_debloat_control.batch_disable_state.is_running {
                self.tab_debloat_control.batch_disable_state.complete();
            }
        }
        // Sync batch enable progress
        if let Ok(progress) = self.tab_debloat_control.batch_enable_progress.lock() {
            if let Some(p) = *progress {
                self.tab_debloat_control
                    .batch_enable_state
                    .update_progress(p);
            } else if self.tab_debloat_control.batch_enable_state.is_running {
                self.tab_debloat_control.batch_enable_state.complete();
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let available_width = ui.ctx().content_rect().width();
        let is_desktop = available_width >= crate::DESKTOP_MIN_WIDTH;
        // Apply theme at the start of UI rendering
        apply_theme(ui.ctx(), Some(Self::detect_os_theme));

        // Apply saved text style
        self.apply_saved_text_style(ui.ctx());

        // Sync scan progress states before rendering progress bars
        self.sync_scan_progress();

        // === top app bar area start
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            let items_button = ui.add(MaterialButton::filled("☰"));
            self.items_button_rect = Some(items_button.rect);
            if items_button.clicked() {
                // Toggle menu instead of just opening
                self.standard_menu_open = !self.standard_menu_open;
                // ui.ctx().request_repaint(); // Repaint when menu state changes
            }
            self.show_menus(ui.ctx());
            ui.horizontal_wrapped(|ui| {
                egui::ScrollArea::horizontal()
                    .id_salt(format!("top_app_bar_area"))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.heading(tr!("app-title"));
                            if is_desktop {
                                ui.label(tr!("app-description"));
                            }
                        });

                        {
                            ui.label(tr!("devices"));
                            let selected_text = self
                                .selected_device
                                .clone()
                                .unwrap_or_else(|| tr!("select-device"));

                            let combo_response = egui::ComboBox::from_label("   ")
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
                                crate::calc::retrieve_adb_devices(self);
                            }

                            // Update users list when device selection changes
                            if self.selected_device != self.current_device {
                                log::debug!(
                                    "device selection changed to {:?}",
                                    self.selected_device
                                );
                                self.current_device = self.selected_device.clone();
                                crate::calc::retrieve_adb_users(self);
                                // Reset user selection when device changes
                                self.selected_user = None;
                                self.current_user = None;
                                crate::calc::retrieve_installed_packages(self);
                            }

                            // User selection ComboBox
                            ui.label(tr!("users"));
                            let user_selected_text = if let Some(user_id) = self.selected_user {
                                if let Some(user_info) =
                                    self.adb_users.iter().find(|u| u.user_id == user_id)
                                {
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
                                    ui.selectable_value(
                                        &mut self.selected_user,
                                        None,
                                        tr!("all-users"),
                                    );

                                    // Add individual users
                                    for user in &self.adb_users {
                                        let label =
                                            format!("User {} ({})", user.user_id, user.name);
                                        ui.selectable_value(
                                            &mut self.selected_user,
                                            Some(user.user_id),
                                            label,
                                        );
                                    }
                                });

                            // Retrieve installed packages when user selection changes
                            if self.selected_user != self.current_user {
                                log::debug!("user selection changed to {:?}", self.selected_user);
                                self.current_user = self.selected_user;
                                crate::calc::retrieve_installed_packages(self);
                            }

                            // Update device list on button click
                            let refresh_button = egui::Button::new(ICON_REFRESH.to_string())
                                .min_size(egui::vec2(20.0, 20.0));
                            if ui
                                .add(refresh_button)
                                .on_hover_text(tr!("refresh-list"))
                                .clicked()
                            {
                                crate::calc::retrieve_adb_devices(self);
                            }
                        }
                    });
            });
        });
        // === top app bar area end

        // === notification render progress area start
        ui.horizontal(|ui| {
            egui::ScrollArea::horizontal()
                .id_salt(format!("notification_render_progress_area"))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // Package loading progress
                    let debloat_progress_value =
                        if let Ok(debloat_progress) = self.package_load_progress.lock() {
                            *debloat_progress
                        } else {
                            None
                        };

                    if let Some(p) = debloat_progress_value {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("loading-packages"));
                        ui.add(progress_bar).on_hover_text(tr!("loading-packages"));
                    }

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
                                    log::info!("Stop Google Play renderer clicked");
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
                                ui.add(progress_bar).on_hover_text(tr!("fdroid-renderer"));
                                if ui.button(tr!("stop")).clicked() {
                                    log::info!("Stop F-Droid renderer clicked");
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
                                    log::info!("Stop APKMirror renderer clicked");
                                    queue.clear_queue();
                                }
                            }
                        }
                    }

                    // VirusTotal scan progress
                    if let Some(p) = self.tab_scan_control.vt_scan_state.progress {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("virustotal-filter"));
                        ui.horizontal(|ui| {
                            ui.add(progress_bar).on_hover_text(tr!("scanning-packages"));

                            if ui.button(tr!("stop")).clicked() {
                                log::info!("Stop Virustotal scan clicked");
                                self.tab_scan_control.vt_scan_state.cancel();
                                if let Ok(mut cancelled) =
                                    self.tab_scan_control.vt_scan_cancelled.lock()
                                {
                                    *cancelled = true;
                                }
                                if let Ok(mut progress) =
                                    self.tab_scan_control.vt_scan_progress.lock()
                                {
                                    *progress = None;
                                }
                            }
                        });
                    }

                    // Hybrid Analysis scan progress
                    if let Some(p) = self.tab_scan_control.ha_scan_state.progress {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("hybrid-analysis-filter"));
                        ui.horizontal(|ui| {
                            ui.add(progress_bar).on_hover_text(tr!("scanning-packages"));

                            if ui.button(tr!("stop")).clicked() {
                                log::info!("Stop Hybrid Analysis scan clicked");
                                self.tab_scan_control.ha_scan_state.cancel();
                                if let Ok(mut cancelled) =
                                    self.tab_scan_control.ha_scan_cancelled.lock()
                                {
                                    *cancelled = true;
                                }
                                if let Ok(mut progress) =
                                    self.tab_scan_control.ha_scan_progress.lock()
                                {
                                    *progress = None;
                                }
                            }
                        });
                    }

                    // IzzyRisk calculation progress
                    if let Some(p) = self.tab_scan_control.izzyrisk_scan_state.progress {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("izzyrisk-calculation"));
                        ui.horizontal(|ui| {
                            ui.add(progress_bar)
                                .on_hover_text(tr!("calculating-risk-scores"));

                            if ui.button(tr!("stop")).clicked() {
                                log::info!("Stop IzzyRisk calculation clicked");
                                self.tab_scan_control.izzyrisk_scan_state.cancel();
                                if let Ok(mut cancelled) =
                                    self.tab_scan_control.izzyrisk_scan_cancelled.lock()
                                {
                                    *cancelled = true;
                                }
                                if let Ok(mut progress) =
                                    self.tab_scan_control.izzyrisk_scan_progress.lock()
                                {
                                    *progress = None;
                                }
                            }
                        });
                    }

                    // Batch uninstall progress
                    if let Some(p) = self.tab_debloat_control.batch_uninstall_state.progress {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("batch-uninstall"));
                        ui.horizontal(|ui| {
                            ui.add(progress_bar)
                                .on_hover_text(tr!("uninstalling-packages"));

                            if ui.button(tr!("stop")).clicked() {
                                log::info!("Stop batch uninstall clicked");
                                self.tab_debloat_control.batch_uninstall_state.cancel();
                                if let Ok(mut cancelled) =
                                    self.tab_debloat_control.batch_uninstall_cancelled.lock()
                                {
                                    *cancelled = true;
                                }
                                if let Ok(mut progress) =
                                    self.tab_debloat_control.batch_uninstall_progress.lock()
                                {
                                    *progress = None;
                                }
                            }
                        });
                    }

                    // Batch disable progress
                    if let Some(p) = self.tab_debloat_control.batch_disable_state.progress {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("batch-disable"));
                        ui.horizontal(|ui| {
                            ui.add(progress_bar)
                                .on_hover_text(tr!("disabling-packages"));

                            if ui.button(tr!("stop")).clicked() {
                                log::info!("Stop batch disable clicked");
                                self.tab_debloat_control.batch_disable_state.cancel();
                                if let Ok(mut cancelled) =
                                    self.tab_debloat_control.batch_disable_cancelled.lock()
                                {
                                    *cancelled = true;
                                }
                                if let Ok(mut progress) =
                                    self.tab_debloat_control.batch_disable_progress.lock()
                                {
                                    *progress = None;
                                }
                            }
                        });
                    }

                    // Batch enable progress
                    if let Some(p) = self.tab_debloat_control.batch_enable_state.progress {
                        let progress_bar = egui::ProgressBar::new(p)
                            .show_percentage()
                            .desired_width(100.0)
                            .animate(true);
                        ui.label(tr!("batch-enable"));
                        ui.horizontal(|ui| {
                            ui.add(progress_bar).on_hover_text(tr!("enabling-packages"));

                            if ui.button(tr!("stop")).clicked() {
                                log::info!("Stop batch enable clicked");
                                self.tab_debloat_control.batch_enable_state.cancel();
                                if let Ok(mut cancelled) =
                                    self.tab_debloat_control.batch_enable_cancelled.lock()
                                {
                                    *cancelled = true;
                                }
                                if let Ok(mut progress) =
                                    self.tab_debloat_control.batch_enable_progress.lock()
                                {
                                    *progress = None;
                                }
                            }
                        });
                    }

                    // App operations progress (install/uninstall)
                    if let Some(queue) = &self.tab_apps_control.operations_queue {
                        if let Ok(progress) = queue.progress.lock() {
                            if let Some(p) = *progress {
                                let pending = queue.queue_size();
                                let completed = queue.completed_count();
                                let total = pending + completed;

                                let progress_bar = egui::ProgressBar::new(p)
                                    .show_percentage()
                                    .desired_width(100.0)
                                    .animate(true);
                                ui.label("App Operations");
                                ui.horizontal(|ui| {
                                    ui.add(progress_bar).on_hover_text(format!(
                                        "{}/{} operations",
                                        completed, total
                                    ));

                                    if ui.button(tr!("stop")).clicked() {
                                        log::info!("Stop app operations clicked");
                                        queue.clear_queue();
                                    }
                                });
                            }
                        }
                    }
                });
        });
        // === notification render progress area end

        // === tab controller (shared logic for mobile and desktop)
        self.prepare_tabs_controller();

        // === tab area start
        let is_desktop = available_width >= DESKTOP_MIN_WIDTH;
        if is_desktop {
            self.render_desktop_tabs(ui);
        } else {
            // mobile
            self.render_mobile_dashboards(ui);
        }
        // === tab area end

        // === logs area start
        if self.settings.show_logs {
            // Add vertical spacer to push logs to bottom
            self.render_logs(ui);
        }
        // === logs area end

        // === settings dialog
        self.dlg_settings.show(ui.ctx(), &mut self.settings);
        // Handle save and theme changes from settings dialog
        if self.dlg_settings.save_clicked {
            self.dlg_settings.save_clicked = false; // Reset flag to prevent repeated saves
            self.save_settings();
        }
        if let Some(theme_name) = self.dlg_settings.theme_to_apply.take() {
            if theme_name == "default" {
                setup_local_theme(Some("resources/material-theme.json"));
                load_themes();
            } else {
                self.apply_theme_by_name(&theme_name);
            }
        }
        // === settings dialog end

        // === ADB installation dialog (all platforms including Android)
        self.dlg_adb_install.show(ui.ctx());
        // Handle retry request from ADB install dialog
        if self.dlg_adb_install.retry_requested {
            self.dlg_adb_install.retry_requested = false;

            #[cfg(target_os = "android")]
            {
                use crate::android_shizuku;
                if android_shizuku::shizuku_is_available() {
                    log::info!("Shizuku detected after retry");
                    crate::calc::retrieve_adb_devices(self);
                } else {
                    self.dlg_adb_install.open();
                }
            }

            #[cfg(not(target_os = "android"))]
            {
                if which::which("adb").is_err() {
                    self.dlg_adb_install.open();
                } else {
                    log::info!("ADB detected after retry");
                    crate::calc::retrieve_adb_devices(self);
                    crate::calc::retrieve_adb_users(self);
                    crate::calc::retrieve_installed_packages(self);
                }
            }
        }
        // === ADB installation dialog end

        // === Disclaimer dialog
        // TODO: implement disclaimer dialog if needed
        // === Disclaimer dialog end

        // === Update dialog (both desktop and Android)
        self.dlg_update.show(ui.ctx());
        // Handle update request from update dialog
        if self.dlg_update.do_update {
            self.perform_update();
        }
        // === Update dialog end

        // === About dialog
        self.dlg_about.show(
            ui.ctx(),
            self.update_checking,
            self.update_available,
            &self.update_status,
        );
        // Handle check update and perform update from about dialog
        if self.dlg_about.do_check_update {
            self.check_for_update();
        }
        if self.dlg_about.do_perform_update {
            self.perform_update();
        }
        // === About dialog end

        // === Install dialog (desktop only)
        #[cfg(not(target_os = "android"))]
        self.show_install_dialog(ui.ctx());
        // === Install dialog end

        // === Package loading dialog
        self.show_package_loading_dialog(ui.ctx());
        crate::calc::handle_package_loading_result(self);
        // === Package loading dialog end

        // === Dashcounter details dialog
        // Check if a dashcounter was clicked
        if let Some((dashboard_type, index)) = ui
            .ctx()
            .data(|data| data.get_temp::<(&str, usize)>(egui::Id::new("dashcounter_clicked")))
        {
            use crate::dlg_dashcounter_details_stt::DashCounterCategory;

            // Update cached counts before using them (needed when viewing from dashboard)
            let shared_store = crate::shared_store_stt::get_shared_store();
            let installed_packages = shared_store.get_installed_packages();
            let uad_ng_lists = shared_store.get_uad_ng_lists();
            self.tab_debloat_control
                .update_cached_counts(&installed_packages, uad_ng_lists.as_ref());

            // Get counts from the dashboard
            let cached_counts = &self.tab_debloat_control.cached_counts;
            let cached_scan_counts = &self.tab_scan_control.cached_scan_counts;

            let (category, count_enabled, count_total) = match (dashboard_type, index) {
                ("debloat", 0) => (
                    Some(DashCounterCategory::DebloatRecommend),
                    cached_counts.recommended.0,
                    cached_counts.recommended.1,
                ),
                ("debloat", 1) => (
                    Some(DashCounterCategory::DebloatAdvanced),
                    cached_counts.advanced.0,
                    cached_counts.advanced.1,
                ),
                ("debloat", 2) => (
                    Some(DashCounterCategory::DebloatExpert),
                    cached_counts.expert.0,
                    cached_counts.expert.1,
                ),
                ("debloat", 3) => (
                    Some(DashCounterCategory::DebloatUnsafe),
                    cached_counts.unsafe_count.0,
                    cached_counts.unsafe_count.1,
                ),
                ("debloat", 4) => (
                    Some(DashCounterCategory::DebloatUnknown),
                    cached_counts.unknown.0,
                    cached_counts.unknown.1,
                ),
                ("stalkerware", 0) => {
                    let shared_store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = shared_store.get_installed_packages();
                    let stalkerware_indicators = shared_store.get_stalkerware_indicators();

                    let is_pkg_enabled = |pkg: &crate::adb::PackageFingerprint| -> bool {
                        let is_system = pkg.flags.contains("SYSTEM");
                        pkg.users
                            .first()
                            .map(|u| {
                                let enabled_str = match u.enabled {
                                    0 => {
                                        if !u.installed && is_system {
                                            "REMOVED_USER"
                                        } else {
                                            "DEFAULT"
                                        }
                                    }
                                    1 => "ENABLED",
                                    2 => "DISABLED",
                                    3 => "DISABLED_USER",
                                    _ => "UNKNOWN",
                                };
                                enabled_str == "ENABLED"
                                    || enabled_str == "DEFAULT"
                                    || enabled_str == "UNKNOWN"
                            })
                            .unwrap_or(false)
                    };

                    let (enabled, total) = if let Some(indicators) = &stalkerware_indicators {
                        let detected = installed_packages
                            .iter()
                            .filter(|pkg| indicators.is_stalkerware(&pkg.pkg))
                            .count();
                        let enabled_detected = installed_packages
                            .iter()
                            .filter(|pkg| {
                                indicators.is_stalkerware(&pkg.pkg) && is_pkg_enabled(pkg)
                            })
                            .count();
                        (enabled_detected, detected)
                    } else {
                        (0, 0)
                    };
                    (
                        Some(DashCounterCategory::StalkerwareDetected),
                        enabled,
                        total,
                    )
                }
                ("stalkerware", 1) => {
                    let shared_store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = shared_store.get_installed_packages();
                    let stalkerware_indicators = shared_store.get_stalkerware_indicators();

                    let is_pkg_enabled = |pkg: &crate::adb::PackageFingerprint| -> bool {
                        let is_system = pkg.flags.contains("SYSTEM");
                        pkg.users
                            .first()
                            .map(|u| {
                                let enabled_str = match u.enabled {
                                    0 => {
                                        if !u.installed && is_system {
                                            "REMOVED_USER"
                                        } else {
                                            "DEFAULT"
                                        }
                                    }
                                    1 => "ENABLED",
                                    2 => "DISABLED",
                                    3 => "DISABLED_USER",
                                    _ => "UNKNOWN",
                                };
                                enabled_str == "ENABLED"
                                    || enabled_str == "DEFAULT"
                                    || enabled_str == "UNKNOWN"
                            })
                            .unwrap_or(false)
                    };

                    let (enabled, total) = if let Some(indicators) = &stalkerware_indicators {
                        let undetected = installed_packages
                            .iter()
                            .filter(|pkg| !indicators.is_stalkerware(&pkg.pkg))
                            .count();
                        let enabled_undetected = installed_packages
                            .iter()
                            .filter(|pkg| {
                                !indicators.is_stalkerware(&pkg.pkg) && is_pkg_enabled(pkg)
                            })
                            .count();
                        (enabled_undetected, undetected)
                    } else {
                        (installed_packages.len(), installed_packages.len())
                    };
                    (
                        Some(DashCounterCategory::StalkerwareUndetected),
                        enabled,
                        total,
                    )
                }
                ("izzyrisk", 0) => {
                    let shared_store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = shared_store.get_installed_packages();
                    let package_risk_scores = &self.tab_scan_control.package_risk_scores;

                    let is_pkg_enabled = |pkg: &crate::adb::PackageFingerprint| -> bool {
                        let is_system = pkg.flags.contains("SYSTEM");
                        pkg.users
                            .first()
                            .map(|u| {
                                let enabled_str = match u.enabled {
                                    0 => {
                                        if !u.installed && is_system {
                                            "REMOVED_USER"
                                        } else {
                                            "DEFAULT"
                                        }
                                    }
                                    1 => "ENABLED",
                                    2 => "DISABLED",
                                    3 => "DISABLED_USER",
                                    _ => "UNKNOWN",
                                };
                                enabled_str == "ENABLED"
                                    || enabled_str == "DEFAULT"
                                    || enabled_str == "UNKNOWN"
                            })
                            .unwrap_or(false)
                    };

                    let total = installed_packages
                        .iter()
                        .filter(|pkg| {
                            package_risk_scores
                                .get(&pkg.pkg)
                                .map_or(false, |&score| score > 20)
                        })
                        .count();
                    let enabled = installed_packages
                        .iter()
                        .filter(|pkg| {
                            package_risk_scores
                                .get(&pkg.pkg)
                                .map_or(false, |&score| score > 20)
                                && is_pkg_enabled(pkg)
                        })
                        .count();
                    (Some(DashCounterCategory::IzzyRiskHigh), enabled, total)
                }
                ("izzyrisk", 1) => {
                    let shared_store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = shared_store.get_installed_packages();
                    let package_risk_scores = &self.tab_scan_control.package_risk_scores;

                    let is_pkg_enabled = |pkg: &crate::adb::PackageFingerprint| -> bool {
                        let is_system = pkg.flags.contains("SYSTEM");
                        pkg.users
                            .first()
                            .map(|u| {
                                let enabled_str = match u.enabled {
                                    0 => {
                                        if !u.installed && is_system {
                                            "REMOVED_USER"
                                        } else {
                                            "DEFAULT"
                                        }
                                    }
                                    1 => "ENABLED",
                                    2 => "DISABLED",
                                    3 => "DISABLED_USER",
                                    _ => "UNKNOWN",
                                };
                                enabled_str == "ENABLED"
                                    || enabled_str == "DEFAULT"
                                    || enabled_str == "UNKNOWN"
                            })
                            .unwrap_or(false)
                    };

                    let total = installed_packages
                        .iter()
                        .filter(|pkg| {
                            package_risk_scores
                                .get(&pkg.pkg)
                                .map_or(false, |&score| score >= 11 && score <= 20)
                        })
                        .count();
                    let enabled = installed_packages
                        .iter()
                        .filter(|pkg| {
                            package_risk_scores
                                .get(&pkg.pkg)
                                .map_or(false, |&score| score >= 11 && score <= 20)
                                && is_pkg_enabled(pkg)
                        })
                        .count();
                    (Some(DashCounterCategory::IzzyRiskModerate), enabled, total)
                }
                ("izzyrisk", 2) => {
                    let shared_store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = shared_store.get_installed_packages();
                    let package_risk_scores = &self.tab_scan_control.package_risk_scores;

                    let is_pkg_enabled = |pkg: &crate::adb::PackageFingerprint| -> bool {
                        let is_system = pkg.flags.contains("SYSTEM");
                        pkg.users
                            .first()
                            .map(|u| {
                                let enabled_str = match u.enabled {
                                    0 => {
                                        if !u.installed && is_system {
                                            "REMOVED_USER"
                                        } else {
                                            "DEFAULT"
                                        }
                                    }
                                    1 => "ENABLED",
                                    2 => "DISABLED",
                                    3 => "DISABLED_USER",
                                    _ => "UNKNOWN",
                                };
                                enabled_str == "ENABLED"
                                    || enabled_str == "DEFAULT"
                                    || enabled_str == "UNKNOWN"
                            })
                            .unwrap_or(false)
                    };

                    let total = installed_packages
                        .iter()
                        .filter(|pkg| {
                            package_risk_scores
                                .get(&pkg.pkg)
                                .map_or(false, |&score| score >= 1 && score <= 10)
                        })
                        .count();
                    let enabled = installed_packages
                        .iter()
                        .filter(|pkg| {
                            package_risk_scores
                                .get(&pkg.pkg)
                                .map_or(false, |&score| score >= 1 && score <= 10)
                                && is_pkg_enabled(pkg)
                        })
                        .count();
                    (Some(DashCounterCategory::IzzyRiskNormal), enabled, total)
                }
                ("izzyrisk", 3) => {
                    let shared_store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = shared_store.get_installed_packages();
                    let package_risk_scores = &self.tab_scan_control.package_risk_scores;

                    let is_pkg_enabled = |pkg: &crate::adb::PackageFingerprint| -> bool {
                        let is_system = pkg.flags.contains("SYSTEM");
                        pkg.users
                            .first()
                            .map(|u| {
                                let enabled_str = match u.enabled {
                                    0 => {
                                        if !u.installed && is_system {
                                            "REMOVED_USER"
                                        } else {
                                            "DEFAULT"
                                        }
                                    }
                                    1 => "ENABLED",
                                    2 => "DISABLED",
                                    3 => "DISABLED_USER",
                                    _ => "UNKNOWN",
                                };
                                enabled_str == "ENABLED"
                                    || enabled_str == "DEFAULT"
                                    || enabled_str == "UNKNOWN"
                            })
                            .unwrap_or(false)
                    };

                    let total = installed_packages
                        .iter()
                        .filter(|pkg| {
                            package_risk_scores
                                .get(&pkg.pkg)
                                .map_or(false, |&score| score == 0)
                        })
                        .count();
                    let enabled = installed_packages
                        .iter()
                        .filter(|pkg| {
                            package_risk_scores
                                .get(&pkg.pkg)
                                .map_or(false, |&score| score == 0)
                                && is_pkg_enabled(pkg)
                        })
                        .count();
                    (Some(DashCounterCategory::IzzyRiskSafe), enabled, total)
                }
                ("virustotal", 0) => (
                    Some(DashCounterCategory::VirusTotalMalicious),
                    cached_scan_counts.vt_counts.1 .0,
                    cached_scan_counts.vt_counts.1 .1,
                ),
                ("virustotal", 1) => (
                    Some(DashCounterCategory::VirusTotalSuspicious),
                    cached_scan_counts.vt_counts.2 .0,
                    cached_scan_counts.vt_counts.2 .1,
                ),
                ("virustotal", 2) => (
                    Some(DashCounterCategory::VirusTotalSafe),
                    cached_scan_counts.vt_counts.3 .0,
                    cached_scan_counts.vt_counts.3 .1,
                ),
                ("virustotal", 3) => (
                    Some(DashCounterCategory::VirusTotalNotScanned),
                    cached_scan_counts.vt_counts.4 .0,
                    cached_scan_counts.vt_counts.4 .1,
                ),
                ("hybridanalysis", 0) => (
                    Some(DashCounterCategory::HybridAnalysisMalicious),
                    cached_scan_counts.ha_counts.1 .0,
                    cached_scan_counts.ha_counts.1 .1,
                ),
                ("hybridanalysis", 1) => (
                    Some(DashCounterCategory::HybridAnalysisMaliciousIgnored),
                    cached_scan_counts.ha_counts.2 .0,
                    cached_scan_counts.ha_counts.2 .1,
                ),
                ("hybridanalysis", 2) => (
                    Some(DashCounterCategory::HybridAnalysisSuspicious),
                    cached_scan_counts.ha_counts.3 .0,
                    cached_scan_counts.ha_counts.3 .1,
                ),
                ("hybridanalysis", 3) => (
                    Some(DashCounterCategory::HybridAnalysisSafe),
                    cached_scan_counts.ha_counts.4 .0,
                    cached_scan_counts.ha_counts.4 .1,
                ),
                ("hybridanalysis", 4) => (
                    Some(DashCounterCategory::HybridAnalysisNotScanned),
                    cached_scan_counts.ha_counts.5 .0,
                    cached_scan_counts.ha_counts.5 .1,
                ),
                ("offa", idx) => {
                    // Retrieve category name and apps from temp storage
                    let category_name = ui.ctx().data(|data| {
                        data.get_temp::<String>(egui::Id::new(format!("offa_category_{}", idx)))
                    });
                    let apps_data = ui.ctx().data(|data| {
                        data.get_temp::<Vec<crate::tab_apps_control_stt::AppEntry>>(egui::Id::new(
                            format!("offa_apps_{}", idx),
                        ))
                    });

                    if let (Some(category), Some(apps)) = (category_name, apps_data) {
                        // Store apps in dialog for rendering
                        self.dlg_dashcounter_details.offa_apps = apps.clone();

                        // Helper to check if an app is installed
                        let is_app_installed =
                            |app_entry: &crate::tab_apps_control_stt::AppEntry| -> bool {
                                // Extract package name from links if not explicitly set
                                let package_name = app_entry.package_name.clone().or_else(|| {
                                    // Try to extract from F-Droid or IzzyOnDroid link
                                    for (url, _link_type) in &app_entry.links {
                                        // F-Droid format: https://f-droid.org/packages/com.example.app
                                        if url.contains("f-droid.org") && url.contains("/packages/")
                                        {
                                            if let Some(start) = url.find("/packages/") {
                                                let after = &url[start + 10..];
                                                let end = after.find('/').unwrap_or(after.len());
                                                let pkg = after[..end].trim();
                                                if !pkg.is_empty() && pkg.contains('.') {
                                                    return Some(pkg.to_string());
                                                }
                                            }
                                        }
                                        // IzzyOnDroid format: https://apt.izzysoft.de/fdroid/index/apk/com.example.app
                                        else if url.contains("izzysoft.de")
                                            && url.contains("/apk/")
                                        {
                                            if let Some(start) = url.find("/apk/") {
                                                let after = &url[start + 5..];
                                                let end = after.find('/').unwrap_or(after.len());
                                                let pkg = after[..end].trim();
                                                if !pkg.is_empty() && pkg.contains('.') {
                                                    return Some(pkg.to_string());
                                                }
                                            }
                                        }
                                    }
                                    None
                                });

                                // Check if app is installed - only use exact package name matching
                                if let Some(ref pkg_name) = package_name {
                                    installed_packages.iter().any(|pkg| &pkg.pkg == pkg_name)
                                } else {
                                    false
                                }
                            };

                        let total = apps.len();
                        let installed = apps.iter().filter(|app| is_app_installed(app)).count();

                        (
                            Some(DashCounterCategory::OffaCategory(category)),
                            installed,
                            total,
                        )
                    } else {
                        (None, 0, 0)
                    }
                }
                ("fmhy", idx) => {
                    // Retrieve category name and apps from temp storage
                    let category_name = ui.ctx().data(|data| {
                        data.get_temp::<String>(egui::Id::new(format!("fmhy_category_{}", idx)))
                    });
                    let apps_data = ui.ctx().data(|data| {
                        data.get_temp::<Vec<crate::tab_apps_control_stt::AppEntry>>(egui::Id::new(
                            format!("fmhy_apps_{}", idx),
                        ))
                    });

                    if let (Some(category), Some(apps)) = (category_name, apps_data) {
                        // Store apps in dialog for rendering
                        self.dlg_dashcounter_details.offa_apps = apps.clone();

                        // Helper to check if an app is installed
                        let is_app_installed =
                            |app_entry: &crate::tab_apps_control_stt::AppEntry| -> bool {
                                // Extract package name from links if not explicitly set
                                let package_name = app_entry.package_name.clone().or_else(|| {
                                    // Try to extract from F-Droid or IzzyOnDroid link
                                    for (url, _link_type) in &app_entry.links {
                                        // F-Droid format: https://f-droid.org/packages/com.example.app
                                        if url.contains("f-droid.org") && url.contains("/packages/")
                                        {
                                            if let Some(start) = url.find("/packages/") {
                                                let after = &url[start + 10..];
                                                let end = after.find('/').unwrap_or(after.len());
                                                let pkg = after[..end].trim();
                                                if !pkg.is_empty() && pkg.contains('.') {
                                                    return Some(pkg.to_string());
                                                }
                                            }
                                        }
                                        // IzzyOnDroid format: https://apt.izzysoft.de/fdroid/index/apk/com.example.app
                                        else if url.contains("izzysoft.de")
                                            && url.contains("/apk/")
                                        {
                                            if let Some(start) = url.find("/apk/") {
                                                let after = &url[start + 5..];
                                                let end = after.find('/').unwrap_or(after.len());
                                                let pkg = after[..end].trim();
                                                if !pkg.is_empty() && pkg.contains('.') {
                                                    return Some(pkg.to_string());
                                                }
                                            }
                                        }
                                    }
                                    None
                                });

                                // Check if app is installed - only use exact package name matching
                                if let Some(ref pkg_name) = package_name {
                                    installed_packages.iter().any(|pkg| &pkg.pkg == pkg_name)
                                } else {
                                    false
                                }
                            };

                        let total = apps.len();
                        let installed = apps.iter().filter(|app| is_app_installed(app)).count();

                        (
                            Some(DashCounterCategory::FmhyCategory(category)),
                            installed,
                            total,
                        )
                    } else {
                        (None, 0, 0)
                    }
                }
                _ => (None, 0, 0),
            };
            if let Some(cat) = category {
                self.dlg_dashcounter_details
                    .open(cat, count_enabled, count_total);
            }
            // Clear the temp data
            ui.ctx().data_mut(|data| {
                data.remove::<(&str, usize)>(egui::Id::new("dashcounter_clicked"));
            });
        }

        // Show dashcounter details dialog
        let shared_store = crate::shared_store_stt::get_shared_store();
        let installed_packages = shared_store.get_installed_packages();
        let uad_ng_lists = shared_store.get_uad_ng_lists();
        let stalkerware_indicators = shared_store.get_stalkerware_indicators();
        let package_risk_scores = &self.tab_scan_control.package_risk_scores;

        self.dlg_dashcounter_details.show(
            ui.ctx(),
            &installed_packages,
            &uad_ng_lists,
            &stalkerware_indicators,
            package_risk_scores,
            self.tab_debloat_control.unsafe_app_remove,
            self.tab_debloat_control.expert_app_remove,
            &self.settings.hybridanalysis_tag_ignorelist,
        );

        // Handle action button events from dashcounter details dialog
        // These need to be handled here because the tabs may not be rendered when viewing dashboard

        // Handle info button - open package details dialog
        if let Some(pkg_id) = ui
            .ctx()
            .data(|data| data.get_temp::<String>(egui::Id::new("info_clicked_package")))
        {
            ui.ctx().data_mut(|data| {
                data.remove::<String>(egui::Id::new("info_clicked_package"));
            });

            if let Some(idx) = installed_packages.iter().position(|p| p.pkg == pkg_id) {
                // Use debloat control dialog by default (works from any view)
                match self.custom_selected {
                    1 => self.tab_debloat_control.package_details_dialog.open(idx),
                    2 => self.tab_scan_control.package_details_dialog.open(idx),
                    3 => self.tab_apps_control.package_details_dialog.open(idx),
                    _ => self.tab_debloat_control.package_details_dialog.open(idx), // Dashboard or other views
                }
            }
        }

        // Handle settings button - open settings dialog
        if ui
            .ctx()
            .data(|data| data.get_temp::<bool>(egui::Id::new("settings_button_clicked")))
            .unwrap_or(false)
        {
            ui.ctx().data_mut(|data| {
                data.remove::<bool>(egui::Id::new("settings_button_clicked"));
            });

            // Sync temporary settings from current settings when opening dialog
            self.dlg_settings.google_play_renderer = self.settings.google_play_renderer;
            self.dlg_settings.fdroid_renderer = self.settings.fdroid_renderer;
            self.dlg_settings.apkmirror_renderer = self.settings.apkmirror_renderer;
            self.dlg_settings.virustotal_apikey = self.settings.virustotal_apikey.clone();
            self.dlg_settings.hybridanalysis_apikey = self.settings.hybridanalysis_apikey.clone();
            self.dlg_settings.virustotal_submit = self.settings.virustotal_submit;
            self.dlg_settings.hybridanalysis_submit = self.settings.hybridanalysis_submit;
            self.dlg_settings.hybridanalysis_tag_ignorelist =
                self.settings.hybridanalysis_tag_ignorelist.clone();
            self.dlg_settings.unsafe_app_remove = self.settings.unsafe_app_remove;
            self.dlg_settings.expert_app_remove = self.settings.expert_app_remove;
            self.dlg_settings.autoupdate = self.settings.autoupdate;
            self.dlg_settings.open();
        }

        // Handle enable/disable/uninstall/refresh actions from dialog
        let mut enable_package: Option<String> = None;
        let mut disable_package: Option<String> = None;
        let mut uninstall_package: Option<String> = None;
        let mut uninstall_is_system = false;
        let mut refresh_package: Option<String> = None;

        ui.ctx().data_mut(|data| {
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("enable_clicked_package")) {
                enable_package = Some(pkg);
                data.remove::<String>(egui::Id::new("enable_clicked_package"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("disable_clicked_package")) {
                disable_package = Some(pkg);
                data.remove::<String>(egui::Id::new("disable_clicked_package"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("uninstall_clicked_package")) {
                uninstall_package = Some(pkg);
                uninstall_is_system = data
                    .get_temp::<bool>(egui::Id::new("uninstall_clicked_is_system"))
                    .unwrap_or(false);
                data.remove::<String>(egui::Id::new("uninstall_clicked_package"));
                data.remove::<bool>(egui::Id::new("uninstall_clicked_is_system"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("refresh_clicked_package")) {
                refresh_package = Some(pkg);
                data.remove::<String>(egui::Id::new("refresh_clicked_package"));
            }
        });

        // Perform enable action
        if let Some(pkg_name) = enable_package {
            if let Some(ref device) = self.tab_debloat_control.selected_device {
                match crate::adb::enable_app(&pkg_name, device) {
                    Ok(output) => {
                        log::info!("App enabled successfully: {}", output);
                        let mut packages = shared_store.get_installed_packages();
                        if let Some(pkg) = packages.iter_mut().find(|p| p.pkg == pkg_name) {
                            for user in pkg.users.iter_mut() {
                                user.enabled = 1;
                                user.installed = true;
                            }
                        }
                        shared_store.set_installed_packages(packages);
                        self.tab_debloat_control.table_version += 1;
                    }
                    Err(e) => {
                        log::error!("Failed to enable app: {}", e);
                    }
                }
            }
        }

        // Perform disable action
        if let Some(pkg_name) = disable_package {
            if let Some(ref device) = self.tab_debloat_control.selected_device {
                match crate::adb::disable_app_current_user(&pkg_name, device, None) {
                    Ok(output) => {
                        log::info!("App disabled successfully: {}", output);
                        let mut packages = shared_store.get_installed_packages();
                        if let Some(pkg) = packages.iter_mut().find(|p| p.pkg == pkg_name) {
                            for user in pkg.users.iter_mut() {
                                user.enabled = 3;
                            }
                        }
                        shared_store.set_installed_packages(packages);
                        self.tab_debloat_control.table_version += 1;
                    }
                    Err(e) => {
                        log::error!("Failed to disable app: {}", e);
                    }
                }
            }
        }

        // Open uninstall confirmation dialog
        if let Some(pkg_name) = uninstall_package {
            self.tab_debloat_control
                .uninstall_confirm_dialog
                .open_single(pkg_name, uninstall_is_system);
        }

        // Show uninstall confirm dialog (needed when viewing from dashboard)
        if self
            .tab_debloat_control
            .uninstall_confirm_dialog
            .show(ui.ctx())
        {
            let pkgs =
                std::mem::take(&mut self.tab_debloat_control.uninstall_confirm_dialog.packages);
            let sys_flags =
                std::mem::take(&mut self.tab_debloat_control.uninstall_confirm_dialog.is_system);
            self.tab_debloat_control.uninstall_confirm_dialog.reset();

            if let Some(ref device) = self.tab_debloat_control.selected_device {
                // Use tab's batch uninstall to handle the operation properly
                self.tab_debloat_control.start_batch_uninstall(
                    self.viewmodel.as_mut(),
                    pkgs,
                    sys_flags,
                    device.clone(),
                    uad_ng_lists.as_ref(),
                );
            } else {
                log::error!("No device selected for uninstall");
            }
        }

        // Handle install button from offa dashcounter details
        if let Some(app) = ui.ctx().data(|data| {
            data.get_temp::<crate::tab_apps_control_stt::AppEntry>(egui::Id::new(
                "install_clicked_app",
            ))
        }) {
            ui.ctx().data_mut(|data| {
                data.remove::<crate::tab_apps_control_stt::AppEntry>(egui::Id::new(
                    "install_clicked_app",
                ));
            });

            // Use tab_apps_control's install_app method
            if let Err(e) = self.tab_apps_control.install_app(&app) {
                log::error!("Failed to install app: {}", e);
            }
        }

        // Perform refresh (delete scan results and re-scan)
        if let Some(pkg_name) = refresh_package {
            log::info!("Refreshing scan results for: {}", pkg_name);

            // Delete from database
            let mut conn = crate::db::establish_connection();
            if let Err(e) = crate::db_virustotal::delete_results_by_package(&mut conn, &pkg_name) {
                log::error!(
                    "Failed to delete VirusTotal results for {}: {}",
                    pkg_name,
                    e
                );
            } else {
                log::info!("Deleted VirusTotal results for: {}", pkg_name);
            }

            if let Err(e) =
                crate::db_hybridanalysis::delete_results_by_package(&mut conn, &pkg_name)
            {
                log::error!(
                    "Failed to delete HybridAnalysis results for {}: {}",
                    pkg_name,
                    e
                );
            } else {
                log::info!("Deleted HybridAnalysis results for: {}", pkg_name);
            }

            // Get package info for scanning
            let installed_packages = shared_store.installed_packages.lock().unwrap();
            let package_info = installed_packages
                .iter()
                .find(|p| p.pkg == pkg_name)
                .cloned();

            if let Some(package) = package_info {
                // Get hashes for the package
                let device_serial = self.tab_scan_control.device_serial.clone();
                let cached_packages = if let Some(ref serial) = device_serial {
                    crate::db_package_cache::get_cached_packages_with_apk(serial)
                } else {
                    vec![]
                };
                let cached_pkg = cached_packages.iter().find(|cp| cp.pkg_id == pkg_name);

                let mut paths_str = String::new();
                let mut sha256sums_str = String::new();

                if let Some(cp) = cached_pkg {
                    if let (Some(path), Some(sha256)) = (&cp.apk_path, &cp.apk_sha256sum) {
                        paths_str = path.clone();
                        sha256sums_str = sha256.clone();
                    }
                }

                if paths_str.is_empty() || sha256sums_str.is_empty() {
                    paths_str = package.codePath.clone();
                    sha256sums_str = package.pkgChecksum.clone();
                }

                // Get proper hashes if needed
                if let Some(ref serial) = device_serial {
                    let paths: Vec<&str> = paths_str.split(' ').collect();
                    let sha256sums: Vec<&str> = sha256sums_str.split(' ').collect();
                    let needs_directory_scan = paths.iter().any(|p| !p.ends_with(".apk"));
                    let has_invalid_hashes = sha256sums.iter().any(|s| s.len() != 64);

                    if needs_directory_scan || has_invalid_hashes {
                        if let Ok((new_paths, new_sha256sums)) =
                            crate::adb::get_single_package_sha256sum(serial, &pkg_name)
                        {
                            if !new_paths.is_empty() && !new_sha256sums.is_empty() {
                                paths_str = new_paths;
                                sha256sums_str = new_sha256sums;
                            }
                        }
                    }
                }

                let final_paths: Vec<&str> = paths_str.split(' ').collect();
                let final_sha256sums: Vec<&str> = sha256sums_str.split(' ').collect();
                let hashes: Vec<(String, String)> = final_paths
                    .iter()
                    .zip(final_sha256sums.iter())
                    .filter(|(p, s)| !p.is_empty() && s.len() == 64)
                    .map(|(p, s)| (p.to_string(), s.to_string()))
                    .collect();

                // TODO: Re-implement using ViewModel commands (RunVirusTotal)
                // Old direct SharedStore access code removed during migration
                log::warn!("VirusTotal refresh not yet re-implemented with ViewModel");

                // TODO: Re-implement using ViewModel commands (RunHybridAnalysis)
                // Old direct SharedStore access code removed during migration
                log::warn!("HybridAnalysis refresh not yet re-implemented with ViewModel");
            }
        }

        // Show package details dialogs (needed when viewing from dashboard)
        let packages_for_dialog = shared_store.get_installed_packages();
        let uad_lists_for_dialog = shared_store.get_uad_ng_lists();

        match self.custom_selected {
            1 => self.tab_debloat_control.package_details_dialog.show(
                ui.ctx(),
                &packages_for_dialog,
                &uad_lists_for_dialog,
            ),
            2 => self.tab_scan_control.package_details_dialog.show(
                ui.ctx(),
                &packages_for_dialog,
                &uad_lists_for_dialog,
            ),
            3 => self.tab_apps_control.package_details_dialog.show(
                ui.ctx(),
                &packages_for_dialog,
                &uad_lists_for_dialog,
            ),
            _ => self.tab_debloat_control.package_details_dialog.show(
                ui.ctx(),
                &packages_for_dialog,
                &uad_lists_for_dialog,
            ), // Dashboard or other views
        };

        // === Dashcounter details dialog end
    }

    fn show_menus(&mut self, ctx: &egui::Context) {
        // Standard Menu with Items - opens below button (default positioning)
        if self.standard_menu_open {
            let close_menu = Cell::new(false);
            let should_exit = Cell::new(false);
            let open_settings = Cell::new(false);
            let open_about = Cell::new(false);
            #[cfg(not(target_os = "android"))]
            let do_install_action = Cell::new(false);
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

            // Install/Uninstall menu item (desktop only)
            #[cfg(not(target_os = "android"))]
            let install_text = if self.install_status == InstallStatus::Installed {
                tr!("uninstall")
            } else {
                tr!("install")
            };
            #[cfg(not(target_os = "android"))]
            let install_item = self.create_menu_item(&install_text, "install", || {
                do_install_action.set(true);
                close_menu.set(true);
            });

            let exit_item = self.create_menu_item(&exit_text, "exit", || {
                close_menu.set(true);
                should_exit.set(true);
                println!("Exit clicked!");
            });

            #[cfg(not(target_os = "android"))]
            let menu_builder = menu("standard_menu", &mut self.standard_menu_open)
                .item(settings_item)
                .item(install_item)
                .item(about_item)
                .item(exit_item);

            #[cfg(target_os = "android")]
            let menu_builder = menu("standard_menu", &mut self.standard_menu_open)
                .item(settings_item)
                .item(about_item)
                .item(exit_item);

            let mut menu_builder = menu_builder
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
                self.dlg_settings.google_play_renderer = self.settings.google_play_renderer;
                self.dlg_settings.fdroid_renderer = self.settings.fdroid_renderer;
                self.dlg_settings.apkmirror_renderer = self.settings.apkmirror_renderer;
                self.dlg_settings.virustotal_apikey = self.settings.virustotal_apikey.clone();
                self.dlg_settings.hybridanalysis_apikey =
                    self.settings.hybridanalysis_apikey.clone();
                self.dlg_settings.virustotal_submit = self.settings.virustotal_submit;
                self.dlg_settings.hybridanalysis_submit = self.settings.hybridanalysis_submit;
                self.dlg_settings.hybridanalysis_tag_ignorelist =
                    self.settings.hybridanalysis_tag_ignorelist.clone();
                self.dlg_settings.unsafe_app_remove = self.settings.unsafe_app_remove;
                self.dlg_settings.expert_app_remove = self.settings.expert_app_remove;
                self.dlg_settings.expert_app_remove = self.settings.expert_app_remove;
                self.dlg_settings.autoupdate = self.settings.autoupdate;
                self.dlg_settings.open();
            }

            if open_about.get() {
                self.dlg_about.open();
            }

            #[cfg(not(target_os = "android"))]
            if do_install_action.get() {
                self.perform_install_action();
            }

            if should_exit.get() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                process::exit(1);
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

    /// Perform install or uninstall action based on current status
    #[cfg(not(target_os = "android"))]
    fn perform_install_action(&mut self) {
        use crate::install_stt::InstallResult;

        let result = if self.install_status == InstallStatus::Installed {
            install::do_uninstall()
        } else {
            install::do_install()
        };

        match result {
            InstallResult::Success(msg) => {
                self.install_message = msg;
                // Refresh install status
                self.install_status = install::check_install();
            }
            InstallResult::Error(err) => {
                self.install_message = format!("Error: {}", err);
            }
        }
        self.install_dialog_open = true;
    }

    /// Show installation result dialog
    #[cfg(not(target_os = "android"))]
    fn show_install_dialog(&mut self, ctx: &egui::Context) {
        if self.install_dialog_open {
            let title = if self.install_message.starts_with("Error") {
                tr!("install-error")
            } else if self.install_status == InstallStatus::Installed {
                tr!("install-success")
            } else {
                tr!("uninstall-success")
            };

            let mut should_close = false;

            dialog("install_dialog", &title, &mut self.install_dialog_open)
                .content(|ui| {
                    ui.label(&self.install_message);
                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if ui.add(MaterialButton::filled(tr!("ok"))).clicked() {
                            should_close = true;
                        }
                    });
                })
                .show(ctx);

            if should_close {
                self.install_dialog_open = false;
            }
        }
    }

    /// Check for updates from GitHub
    fn check_for_update(&mut self) {
        self.update_checking = true;
        self.update_status.clear();
        self.update_available = false;

        match install::check_update() {
            Ok(info) => {
                self.update_checking = false;
                if info.available {
                    self.update_available = true;
                    self.dlg_update.download_url = info.download_url;
                    self.dlg_update.current_version = info.current_version;
                    self.dlg_update.latest_version = info.latest_version.clone();
                    self.dlg_update.release_notes = info.release_notes;
                    self.update_status = format!(
                        "{} {} → {}",
                        tr!("update-available"),
                        self.dlg_update.current_version,
                        info.latest_version
                    );
                    // Open update dialog automatically
                    self.dlg_update.open();
                } else {
                    self.update_status = tr!("up-to-date").to_string();
                }
            }
            Err(e) => {
                self.update_checking = false;
                self.update_status = format!("{}: {}", tr!("update-error"), e);
            }
        }
    }

    /// Perform update
    fn perform_update(&mut self) {
        #[cfg(not(target_os = "android"))]
        {
            use crate::install_stt::InstallResult;

            let tmp_dir = if let Some(ref cfg) = self.config {
                cfg.tmp_dir.clone()
            } else {
                std::path::PathBuf::from("./tmp")
            };

            match install::do_update(
                &self.dlg_update.download_url,
                &self.dlg_update.latest_version,
                &tmp_dir,
            ) {
                InstallResult::Success(msg) => {
                    self.install_message = msg;
                    self.update_available = false;
                    self.update_status.clear();
                    self.dlg_update.close();
                }
                InstallResult::Error(err) => {
                    self.install_message = format!("Error: {}", err);
                }
            }
            self.install_dialog_open = true;
        }

        #[cfg(target_os = "android")]
        {
            // On Android, open browser to download page
            if !self.dlg_update.download_url.is_empty() {
                if let Err(e) = webbrowser::open(&self.dlg_update.download_url) {
                    log::error!("Failed to open browser for update download: {}", e);
                    self.update_status = format!("Failed to open browser: {}", e);
                } else {
                    log::info!("Opened browser for update download");
                    self.dlg_update.close();
                }
            }
        }
    }

    // Controller method: prepare tab state (shared by both mobile and desktop UI)
    fn prepare_tabs_controller(&mut self) {
        // On Android: hide apps tab if installed from Google Play Store
        // On other platforms: always show apps tab
        #[cfg(target_os = "android")]
        {
            self.show_apps_tab = !matches!(
                self.installer_package_name.as_deref(),
                Some("com.android.vending")
            );
        }
        #[cfg(not(target_os = "android"))]
        {
            self.show_apps_tab = true;
        }

        // Prepare debloat tab controller state
        self.prepare_debloat_tab_controller();

        // Prepare scan tab controller state
        self.prepare_scan_tab_controller();

        // Prepare apps tab controller state
        self.prepare_apps_tab_controller();
    }

    fn prepare_debloat_tab_controller(&mut self) {
        // Platform-specific renderer flags
        #[cfg(not(target_os = "android"))]
        {
            self.google_play_renderer.is_enabled = self.settings.google_play_renderer;
            self.fdroid_renderer.is_enabled = self.settings.fdroid_renderer;
            self.apkmirror_renderer.is_enabled = self.settings.apkmirror_renderer;

            let google_play_enabled = self.google_play_renderer.is_enabled;
            let fdroid_enabled = self.fdroid_renderer.is_enabled;
            let apkmirror_enabled = self.apkmirror_renderer.is_enabled;

            // Initialize and start worker queues if renderers are enabled (desktop only)
            let db_path = self
                .config
                .as_ref()
                .map(|c| c.db_dir.to_string_lossy().to_string())
                .unwrap_or_default();

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

            // Only enqueue packages when the table version changes (packages updated)
            let current_version = self.tab_debloat_control.table_version;
            if current_version != self.debloat_last_enqueued_version {
                self.enqueue_visible_packages_for_debloat(
                    google_play_enabled,
                    fdroid_enabled,
                    apkmirror_enabled,
                );
                self.debloat_last_enqueued_version = current_version;
            }

            // Only load results periodically (every 500ms) instead of every frame
            let now = std::time::Instant::now();
            if now
                .duration_since(self.debloat_last_result_load_time)
                .as_millis()
                >= 500
            {
                self.load_renderer_results_to_debloat_cache();
                self.debloat_last_result_load_time = now;
            }
        }

        // Sync unsafe_app_remove and expert_app_remove setting
        self.tab_debloat_control.unsafe_app_remove = self.settings.unsafe_app_remove;
        self.tab_debloat_control.expert_app_remove = self.settings.expert_app_remove;
    }

    fn prepare_scan_tab_controller(&mut self) {
        // Sync renderer settings from Settings to TabScanControl
        #[cfg(target_os = "android")]
        {
            self.tab_scan_control.google_play_renderer_enabled = false;
            self.tab_scan_control.fdroid_renderer_enabled = false;
            self.tab_scan_control.apkmirror_renderer_enabled = false;
            self.tab_scan_control.android_package_renderer_enabled = true;
        }
        #[cfg(not(target_os = "android"))]
        {
            self.tab_scan_control.google_play_renderer_enabled = self.settings.google_play_renderer;
            self.tab_scan_control.fdroid_renderer_enabled = self.settings.fdroid_renderer;
            self.tab_scan_control.apkmirror_renderer_enabled = self.settings.apkmirror_renderer;
            self.tab_scan_control.android_package_renderer_enabled = false;
        }
        self.tab_scan_control.unsafe_app_remove = self.settings.unsafe_app_remove;
        self.tab_scan_control.expert_app_remove = self.settings.expert_app_remove;
    }

    fn prepare_apps_tab_controller(&mut self) {
        // Check if operations queue has completed and trigger refresh
        if let Some(ref queue) = self.tab_apps_control.operations_queue {
            let was_running = {
                let is_running = queue.is_running.lock().unwrap();
                *is_running
            };

            // If queue was running and is now done, trigger refresh
            if !was_running {
                let has_completed_ops = queue.completed_count() > 0;
                if has_completed_ops {
                    // Check if any operation completed successfully
                    if let Ok(results) = queue.results.lock() {
                        let has_success = results.values().any(|status| {
                            matches!(
                                status,
                                crate::app_operations_queue_stt::OperationStatus::Success(_)
                            )
                        });
                        if has_success && !self.tab_apps_control.refresh_pending {
                            self.tab_apps_control.refresh_pending = true;
                        }
                    }
                }
            }
        }

        // Check if packages need to be refreshed (e.g., after install)
        // Only refresh tab_apps_control, not other tabs (to avoid triggering scans)
        if self.tab_apps_control.refresh_pending {
            self.tab_apps_control.refresh_pending = false;
            self.refresh_apps_tab_packages();

            // Clear operation results after refresh (so progress notification disappears)
            if let Some(ref queue) = self.tab_apps_control.operations_queue {
                let is_running = queue.is_running.lock().unwrap();
                if !*is_running && queue.completed_count() > 0 {
                    drop(is_running);
                    queue.clear_results();
                }
            }
        }
    }

    fn render_desktop_tabs(&mut self, ui: &mut egui::Ui) {
        // Custom themed tabs — compact when content is scrolled down
        let _previous_tab = self.custom_selected;

        // Use pre-calculated show_apps_tab from controller
        let show_apps_tab = self.show_apps_tab;

        let mut tabs = tabs_primary(&mut self.custom_selected)
            .id_salt("custom_primary")
            .tab(tr!("debloat"))
            .tab(tr!("scan"));

        if show_apps_tab {
            tabs = tabs.tab(tr!("apps"));
        }
        //.tab(tr!("usage"));

        if self.is_scrolled {
            tabs = tabs.height(24.0);
        }
        ui.add(tabs);

        // Enhanced content with custom styling
        ui.add_space(10.0);

        // Calculate max height: leave 200px for log box if enabled
        let reserved_space = if self.settings.show_logs { 200.0 } else { 0.0 };
        let max_height = ui.available_height() - reserved_space;

        // Tab content rendering - adjust indices based on whether apps tab is shown
        match self.custom_selected {
            0 => {
                // Debloat tab
                ui.label(tr!("debloat-description"));

                ui.add_space(8.0);

                let scroll_output = egui::ScrollArea::both()
                    .id_salt("debloat_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        self.render_debloat_tab(ui);
                    });
                self.is_scrolled = scroll_output.state.offset.y > 0.0;
            }
            1 => {
                // Scan tab
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

                let scroll_output = egui::ScrollArea::both()
                    .id_salt("scan_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        self.render_scan_tab(ui);
                    });
                self.is_scrolled = scroll_output.state.offset.y > 0.0;
            }
            2 if show_apps_tab => {
                // Apps tab (only shown if not installed from Google Play on Android)
                // ui.colored_label(egui::Color32::from_rgb(103, 80, 164), "Apps");
                ui.label(tr!("apps-description"));
                ui.add_space(8.0);

                let scroll_output = egui::ScrollArea::both()
                    .id_salt("apps_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        self.render_apps_tab(ui);
                    });
                self.is_scrolled = scroll_output.state.offset.y > 0.0;
            }
            3 => {
                // Usage tab (currently commented out in tab list)
                // ui.colored_label(egui::Color32::from_rgb(103, 80, 164), "Usage");
                ui.label(tr!("usage-description"));
                ui.add_space(8.0);

                let scroll_output = egui::ScrollArea::both()
                    .id_salt("usage_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        self.render_usage_tab(ui);
                    });
                self.is_scrolled = scroll_output.state.offset.y > 0.0;
            }
            _ => {
                ui.colored_label(egui::Color32::from_rgb(103, 80, 164), "");
            }
        }
    }

    /// Load and parse app entries from an app list (used for dashboard independent loading)
    fn load_app_list_entries(
        &self,
        app_list: &crate::tab_apps_control_stt::AppListSource,
    ) -> Vec<crate::tab_apps_control_stt::AppEntry> {
        let cache_file = self
            .tab_apps_control
            .cache_dir
            .join(format!("{}.md", app_list.name.replace(" ", "_")));

        // Try to load from cache first
        if let Ok(content) = std::fs::read_to_string(&cache_file) {
            return self.parse_app_list_content(&content);
        }

        // If cache doesn't exist, try to download it
        log::info!(
            "Cache file not found for {}, downloading from: {}",
            app_list.name,
            app_list.contents_url
        );

        match ureq::get(&app_list.contents_url).call() {
            Ok(response) => {
                match response.into_string() {
                    Ok(content) => {
                        log::info!(
                            "Successfully downloaded {} ({} bytes)",
                            app_list.name,
                            content.len()
                        );

                        // Save to cache
                        if let Err(e) = std::fs::write(&cache_file, &content) {
                            log::error!("Failed to save cache file: {}", e);
                        } else {
                            log::info!("Successfully saved {} to cache", app_list.name);
                        }

                        return self.parse_app_list_content(&content);
                    }
                    Err(e) => {
                        log::error!("Failed to read response for {}: {}", app_list.name, e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to download {}: {}", app_list.name, e);
            }
        }

        // Return empty if download failed
        Vec::new()
    }

    /// Parse markdown content into app entries (helper for dashboard)
    fn parse_app_list_content(&self, content: &str) -> Vec<crate::tab_apps_control_stt::AppEntry> {
        let mut app_entries = Vec::new();
        let mut current_category = String::from("Uncategorized");

        for line in content.lines() {
            // Check if line is a markdown header (category)
            if line.starts_with('#') {
                let header_text = line.trim_start_matches('#').trim();
                if !header_text.is_empty() {
                    current_category = header_text
                        .chars()
                        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                        .collect::<String>()
                        .trim()
                        .to_string();
                }
            } else if line.contains("f-droid.org")
                || line.contains("izzysoft.de")
                || line.contains("github.com")
                || line.contains("gitlab.com")
            {
                // Skip iOS categories
                if current_category.contains("iOS") {
                    continue;
                }

                // Extract app name (simplified version)
                let name = if let Some(start) = line.find('[') {
                    if let Some(end) = line.find(']') {
                        if end > start {
                            line[start + 1..end].trim().to_string()
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                // Extract links
                let mut links = Vec::new();
                let mut search_pos = 0;
                while let Some(link_start) = line[search_pos..].find("http") {
                    let actual_start = search_pos + link_start;
                    let remaining = &line[actual_start..];
                    let end_pos = remaining
                        .find(|c: char| c.is_whitespace() || c == ')' || c == ']')
                        .unwrap_or(remaining.len());
                    let url = remaining[..end_pos].to_string();

                    let link_type = if url.contains("f-droid.org") {
                        "fdroid"
                    } else if url.contains("izzysoft.de") {
                        "izzy"
                    } else if url.contains("github.com") {
                        "github"
                    } else if url.contains("gitlab.com") {
                        "gitlab"
                    } else {
                        "home"
                    };

                    links.push((url, link_type.to_string()));
                    search_pos = actual_start + end_pos;
                }

                if !name.is_empty() && !links.is_empty() {
                    app_entries.push(crate::tab_apps_control_stt::AppEntry {
                        category: current_category.clone(),
                        name,
                        links,
                        package_name: None,
                    });
                }
            }
        }

        app_entries
    }

    fn render_mobile_dashboards(&mut self, ui: &mut egui::Ui) {
        use crate::shared_store_stt::get_shared_store;

        // Sync risk scores from background thread before displaying
        self.tab_scan_control.sync_risk_scores();

        let shared_store = get_shared_store();
        let installed_packages = shared_store.get_installed_packages();

        // Update cached debloat counts
        let uad_ng_lists = shared_store.get_uad_ng_lists();
        self.tab_debloat_control
            .update_cached_counts(&installed_packages, uad_ng_lists.as_ref());

        // Update cached scan counts for VT and HA
        let vt_scanner_state = shared_store.get_vt_scanner_state();
        let ha_scanner_state = shared_store.get_ha_scanner_state();
        let ha_tag_ignorelist = &self.settings.hybridanalysis_tag_ignorelist;
        self.tab_scan_control.update_cached_scan_counts(
            &installed_packages,
            &vt_scanner_state,
            &ha_scanner_state,
            ha_tag_ignorelist,
        );

        // Calculate reserved space for logs
        let reserved_space = if self.settings.show_logs { 200.0 } else { 0.0 };
        let max_height = ui.available_height() - reserved_space;

        egui::ScrollArea::vertical()
            .id_salt("mobile_dashboards_scroll")
            .max_height(max_height)
            .show(ui, |ui| {
                // 1. Debloat Dashboard
                let cached_counts = &self.tab_debloat_control.cached_counts;
                let ctx_clone = ui.ctx().clone();
                ui.add(
                    dashcounter("Debloat", &mut self.dash_scroll_debloat)
                        .id_salt("dash_debloat")
                        .title_ui(|ui| {
                            if ui.add(icon_button_standard(ICON_INFO.to_string())).clicked() {
                                let _ = webbrowser::open("https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation");
                            }
                        })
                        .card_with_description(
                            "Recommend",
                            cached_counts.recommended.0,
                            cached_counts.recommended.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Advanced",
                            cached_counts.advanced.0,
                            cached_counts.advanced.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Expert",
                            cached_counts.expert.0,
                            cached_counts.expert.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Unsafe",
                            cached_counts.unsafe_count.0,
                            cached_counts.unsafe_count.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Unknown",
                            cached_counts.unknown.0,
                            cached_counts.unknown.1,
                            "enabled",
                            "all"
                        )
                        .on_click(move |index| {
                            ctx_clone.data_mut(|data| {
                                data.insert_temp(egui::Id::new("dashcounter_clicked"), ("debloat", index));
                            });
                        })
                );
                ui.add_space(20.0);

                // 2. Stalkerware Dashboard
                let stalkerware_indicators = shared_store.get_stalkerware_indicators();
                let ctx_clone = ui.ctx().clone();

                // Helper closure to check if a package is enabled (matches tab_debloat_control logic)
                let is_pkg_enabled = |pkg: &crate::adb::PackageFingerprint| -> bool {
                    let is_system = pkg.flags.contains("SYSTEM");
                    pkg.users.first().map(|u| {
                        let enabled_str = match u.enabled {
                            0 => if !u.installed && is_system { "REMOVED_USER" } else { "DEFAULT" },
                            1 => "ENABLED",
                            2 => "DISABLED",
                            3 => "DISABLED_USER",
                            _ => "UNKNOWN",
                        };
                        enabled_str == "ENABLED" || enabled_str == "DEFAULT" || enabled_str == "UNKNOWN"
                    }).unwrap_or(false)
                };

                let (stalkerware_detected, stalkerware_undetected) = if let Some(indicators) = &stalkerware_indicators {
                    let detected = installed_packages.iter()
                        .filter(|pkg| indicators.is_stalkerware(&pkg.pkg))
                        .count();
                    let enabled_detected = installed_packages.iter()
                        .filter(|pkg| {
                            indicators.is_stalkerware(&pkg.pkg) && is_pkg_enabled(pkg)
                        })
                        .count();
                    let undetected = installed_packages.len() - detected;
                    let enabled_undetected = installed_packages.iter()
                        .filter(|pkg| {
                            !indicators.is_stalkerware(&pkg.pkg) && is_pkg_enabled(pkg)
                        })
                        .count();
                    ((enabled_detected, detected), (enabled_undetected, undetected))
                } else {
                    ((0, 0), (0, installed_packages.len()))
                };

                ui.add(
                    dashcounter("Stalkerware", &mut self.dash_scroll_stalkerware)
                        .id_salt("dash_stalkerware")
                        .title_ui(|ui| {
                            if ui.add(icon_button_standard(ICON_INFO.to_string())).clicked() {
                                let _ = webbrowser::open("https://github.com/AssoEchap/stalkerware-indicators");
                            }
                        })
                        .card_with_description(
                            "Detected",
                            stalkerware_detected.0,
                            stalkerware_detected.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Undetected",
                            stalkerware_undetected.0,
                            stalkerware_undetected.1,
                            "enabled",
                            "all"
                        )
                        .category_color(egui::Color32::from_rgb(244, 67, 54))
                        .counter_color(egui::Color32::from_rgb(183, 28, 28))
                        .description_color(egui::Color32::from_rgb(239, 154, 154))
                        .on_click(move |index| {
                            ctx_clone.data_mut(|data| {
                                data.insert_temp(egui::Id::new("dashcounter_clicked"), ("stalkerware", index));
                            });
                        })
                );
                ui.add_space(20.0);

                // 3. IzzyRisk Dashboard
                let ctx_clone = ui.ctx().clone();
                let package_risk_scores = &self.tab_scan_control.package_risk_scores;

                // Helper closure to check if a package is enabled (matches tab_debloat_control logic)
                let is_pkg_enabled = |pkg: &crate::adb::PackageFingerprint| -> bool {
                    let is_system = pkg.flags.contains("SYSTEM");
                    pkg.users.first().map(|u| {
                        let enabled_str = match u.enabled {
                            0 => if !u.installed && is_system { "REMOVED_USER" } else { "DEFAULT" },
                            1 => "ENABLED",
                            2 => "DISABLED",
                            3 => "DISABLED_USER",
                            _ => "UNKNOWN",
                        };
                        enabled_str == "ENABLED" || enabled_str == "DEFAULT" || enabled_str == "UNKNOWN"
                    }).unwrap_or(false)
                };

                // Count total packages in each risk category
                let risk_0 = installed_packages.iter()
                    .filter(|pkg| {
                        if let Some(&score) = package_risk_scores.get(&pkg.pkg) {
                            score == 0
                        } else {
                            false
                        }
                    })
                    .count();
                let risk_1_10 = installed_packages.iter()
                    .filter(|pkg| {
                        if let Some(&score) = package_risk_scores.get(&pkg.pkg) {
                            score >= 1 && score <= 10
                        } else {
                            false
                        }
                    })
                    .count();
                let risk_11_20 = installed_packages.iter()
                    .filter(|pkg| {
                        if let Some(&score) = package_risk_scores.get(&pkg.pkg) {
                            score >= 11 && score <= 20
                        } else {
                            false
                        }
                    })
                    .count();
                let risk_20_plus = installed_packages.iter()
                    .filter(|pkg| {
                        if let Some(&score) = package_risk_scores.get(&pkg.pkg) {
                            score > 20
                        } else {
                            false
                        }
                    })
                    .count();

                // Count enabled packages in each risk category
                let enabled_risk_0 = installed_packages.iter()
                    .filter(|pkg| {
                        if let Some(&score) = package_risk_scores.get(&pkg.pkg) {
                            score == 0 && is_pkg_enabled(pkg)
                        } else {
                            false
                        }
                    })
                    .count();
                let enabled_risk_1_10 = installed_packages.iter()
                    .filter(|pkg| {
                        if let Some(&score) = package_risk_scores.get(&pkg.pkg) {
                            score >= 1 && score <= 10 && is_pkg_enabled(pkg)
                        } else {
                            false
                        }
                    })
                    .count();
                let enabled_risk_11_20 = installed_packages.iter()
                    .filter(|pkg| {
                        if let Some(&score) = package_risk_scores.get(&pkg.pkg) {
                            score >= 11 && score <= 20 && is_pkg_enabled(pkg)
                        } else {
                            false
                        }
                    })
                    .count();
                let enabled_risk_20_plus = installed_packages.iter()
                    .filter(|pkg| {
                        if let Some(&score) = package_risk_scores.get(&pkg.pkg) {
                            score > 20 && is_pkg_enabled(pkg)
                        } else {
                            false
                        }
                    })
                    .count();

                ui.add(
                    dashcounter("IzzyRisk", &mut self.dash_scroll_izzyrisk)
                        .id_salt("dash_izzyrisk")
                        .title_ui(|ui| {
                            if ui.add(icon_button_standard(ICON_INFO.to_string())).clicked() {
                                let _ = webbrowser::open("https://android.izzysoft.de/applists.php?lang=en;topic=perms");
                            }
                        })
                        .card_with_description("20+(high)", enabled_risk_20_plus, risk_20_plus, "enabled", "all")
                        .card_with_description("11-20(moderate)", enabled_risk_11_20, risk_11_20, "enabled", "all")
                        .card_with_description("1-10(normal)", enabled_risk_1_10, risk_1_10, "enabled", "all")
                        .card_with_description("0(safe)", enabled_risk_0, risk_0, "enabled", "all")
                        .category_color(egui::Color32::from_rgb(255, 152, 0))
                        .counter_color(egui::Color32::from_rgb(230, 81, 0))
                        .description_color(egui::Color32::from_rgb(255, 183, 77))
                        .on_click(move |index| {
                            ctx_clone.data_mut(|data| {
                                data.insert_temp(egui::Id::new("dashcounter_clicked"), ("izzyrisk", index));
                            });
                        })
                );
                ui.add_space(20.0);

                // 4. VirusTotal Dashboard
                let ctx_clone = ui.ctx().clone();
                let vt_counts = &self.tab_scan_control.cached_scan_counts.vt_counts;
                ui.add(
                    dashcounter("VirusTotal", &mut self.dash_scroll_virustotal)
                        .id_salt("dash_virustotal")
                        .title_ui(|ui| {
                            if ui.add(icon_button_standard(ICON_INFO.to_string())).clicked() {
                                let _ = webbrowser::open("https://www.virustotal.com/gui/my-apikey");
                            }
                        })
                        .card_with_description(
                            "Malicious",
                            vt_counts.1.0,
                            vt_counts.1.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Suspicious",
                            vt_counts.2.0,
                            vt_counts.2.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Safe",
                            vt_counts.3.0,
                            vt_counts.3.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Unscan",
                            vt_counts.4.0,
                            vt_counts.4.1,
                            "enabled",
                            "all"
                        )
                        .on_click(move |index| {
                            ctx_clone.data_mut(|data| {
                                data.insert_temp(egui::Id::new("dashcounter_clicked"), ("virustotal", index));
                            });
                        })
                );
                ui.add_space(20.0);

                // 5. HybridAnalysis Dashboard
                let ctx_clone = ui.ctx().clone();
                let ha_counts = &self.tab_scan_control.cached_scan_counts.ha_counts;
                ui.add(
                    dashcounter("HybridAnalysis", &mut self.dash_scroll_hybridanalysis)
                        .id_salt("dash_hybridanalysis")
                        .title_ui(|ui| {
                            if ui.add(icon_button_standard(ICON_INFO.to_string())).clicked() {
                                let _ = webbrowser::open("https://hybrid-analysis.com/my-account");
                            }
                        })
                        .card_with_description(
                            "Malicious",
                            ha_counts.1.0,
                            ha_counts.1.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Mal-Ignored",
                            ha_counts.2.0,
                            ha_counts.2.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Suspicious",
                            ha_counts.3.0,
                            ha_counts.3.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Undetected",
                            ha_counts.4.0,
                            ha_counts.4.1,
                            "enabled",
                            "all"
                        )
                        .card_with_description(
                            "Unscan",
                            ha_counts.5.0,
                            ha_counts.5.1,
                            "enabled",
                            "all"
                        )
                        .on_click(move |index| {
                            ctx_clone.data_mut(|data| {
                                data.insert_temp(egui::Id::new("dashcounter_clicked"), ("hybridanalysis", index));
                            });
                        })
                );
                ui.add_space(20.0);

                // 6. FOSS/OFFA Dashboard (show if offa list exists and not installed via Google Play)
                let show_foss_dashboards = {
                    #[cfg(target_os = "android")]
                    {
                        !matches!(
                            self.installer_package_name.as_deref(),
                            Some("com.android.vending")
                        )
                    }
                    #[cfg(not(target_os = "android"))]
                    {
                        true
                    }
                };

                if show_foss_dashboards {
                    // Find OFFA list
                    if let Some((offa_idx, offa_list)) = self.tab_apps_control.app_lists.iter()
                        .enumerate()
                        .find(|(_, list)| list.name.to_lowercase().contains("offa"))
                    {
                        // Parse apps from OFFA list if needed
                        let offa_apps = if let Some(sel_idx) = self.tab_apps_control.selected_app_list {
                            if sel_idx == offa_idx {
                                // Already loaded
                                self.tab_apps_control.app_entries.clone()
                            } else {
                                // Need to load OFFA list temporarily
                                self.load_app_list_entries(offa_list)
                            }
                        } else {
                            // No list selected, load OFFA
                            self.load_app_list_entries(offa_list)
                        };

                        if !offa_apps.is_empty() {
                            let ctx_clone = ui.ctx().clone();

                            // Helper to check if an app is installed - using exact package namespace matching only
                            let is_app_installed = |app_entry: &crate::tab_apps_control_stt::AppEntry| -> bool {
                                // Extract package name from links if not explicitly set
                                let package_name = app_entry.package_name.clone().or_else(|| {
                                    // Try to extract from F-Droid or IzzyOnDroid link
                                    for (url, _link_type) in &app_entry.links {
                                        // F-Droid format: https://f-droid.org/packages/com.example.app
                                        if url.contains("f-droid.org") && url.contains("/packages/") {
                                            if let Some(start) = url.find("/packages/") {
                                                let after = &url[start + 10..];
                                                let end = after.find('/').unwrap_or(after.len());
                                                let pkg = after[..end].trim();
                                                if !pkg.is_empty() && pkg.contains('.') {
                                                    return Some(pkg.to_string());
                                                }
                                            }
                                        }
                                        // IzzyOnDroid format: https://apt.izzysoft.de/fdroid/index/apk/com.example.app
                                        else if url.contains("izzysoft.de") && url.contains("/apk/") {
                                            if let Some(start) = url.find("/apk/") {
                                                let after = &url[start + 5..];
                                                let end = after.find('/').unwrap_or(after.len());
                                                let pkg = after[..end].trim();
                                                if !pkg.is_empty() && pkg.contains('.') {
                                                    return Some(pkg.to_string());
                                                }
                                            }
                                        }
                                    }
                                    None
                                });

                                // Check if app is installed - only use exact package name matching
                                if let Some(ref pkg_name) = package_name {
                                    installed_packages.iter().any(|pkg| &pkg.pkg == pkg_name)
                                } else {
                                    false
                                }
                            };

                            // Group apps by category and count installed/total
                            let mut category_counts: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
                            for app in &offa_apps {
                                let entry = category_counts.entry(app.category.clone()).or_insert((0, 0));
                                entry.1 += 1; // total
                                if is_app_installed(app) {
                                    entry.0 += 1; // installed
                                }
                            }

                            // Sort categories alphabetically
                            let mut sorted_categories: Vec<_> = category_counts.iter().collect();
                            sorted_categories.sort_by(|a, b| a.0.cmp(b.0));

                            let mut dashboard = dashcounter("FOSS/OFFA", &mut self.dash_scroll_offa)
                                .id_salt("dash_offa")
                                .title_ui(|ui| {
                                    if ui.add(icon_button_standard(ICON_INFO.to_string())).clicked() {
                                        let _ = webbrowser::open(&offa_list.info_url);
                                    }
                                });

                            // Add a card for each category
                            for (idx, (category, (installed, total))) in sorted_categories.iter().enumerate() {
                                dashboard = dashboard.card_with_description(
                                    category.as_str(),
                                    *installed,
                                    *total,
                                    "installed",
                                    "all"
                                );

                                // Store category name and apps for click handler
                                let category_clone = (*category).clone();
                                let apps_in_category: Vec<crate::tab_apps_control_stt::AppEntry> = offa_apps.iter()
                                    .filter(|app| &app.category == *category)
                                    .cloned()
                                    .collect();

                                let ctx_for_storage = ctx_clone.clone();
                                ctx_for_storage.data_mut(|data| {
                                    data.insert_temp(
                                        egui::Id::new(format!("offa_category_{}", idx)),
                                        category_clone
                                    );
                                    data.insert_temp(
                                        egui::Id::new(format!("offa_apps_{}", idx)),
                                        apps_in_category
                                    );
                                });
                            }

                            ui.add(
                                dashboard.on_click(move |index| {
                                    ctx_clone.data_mut(|data| {
                                        data.insert_temp(egui::Id::new("dashcounter_clicked"), ("offa", index));
                                    });
                                })
                            );
                            ui.add_space(20.0);
                        }
                    }

                    // 7. FOSS/FMHY Dashboard (show if fmhy list exists)
                    // Find FMHY list
                    if let Some((fmhy_idx, fmhy_list)) = self.tab_apps_control.app_lists.iter()
                        .enumerate()
                        .find(|(_, list)| list.name.to_lowercase().contains("fmhy"))
                    {
                        // Parse apps from FMHY list if needed
                        let fmhy_apps = if let Some(sel_idx) = self.tab_apps_control.selected_app_list {
                            if sel_idx == fmhy_idx {
                                // Already loaded
                                self.tab_apps_control.app_entries.clone()
                            } else {
                                // Need to load FMHY list temporarily
                                self.load_app_list_entries(fmhy_list)
                            }
                        } else {
                            // No list selected, load FMHY
                            self.load_app_list_entries(fmhy_list)
                        };

                        if !fmhy_apps.is_empty() {
                            let ctx_clone = ui.ctx().clone();

                            // Helper to check if an app is installed - using exact package namespace matching only
                            let is_app_installed = |app_entry: &crate::tab_apps_control_stt::AppEntry| -> bool {
                                // Extract package name from links if not explicitly set
                                let package_name = app_entry.package_name.clone().or_else(|| {
                                    // Try to extract from F-Droid or IzzyOnDroid link
                                    for (url, _link_type) in &app_entry.links {
                                        // F-Droid format: https://f-droid.org/packages/com.example.app
                                        if url.contains("f-droid.org") && url.contains("/packages/") {
                                            if let Some(start) = url.find("/packages/") {
                                                let after = &url[start + 10..];
                                                let end = after.find('/').unwrap_or(after.len());
                                                let pkg = after[..end].trim();
                                                if !pkg.is_empty() && pkg.contains('.') {
                                                    return Some(pkg.to_string());
                                                }
                                            }
                                        }
                                        // IzzyOnDroid format: https://apt.izzysoft.de/fdroid/index/apk/com.example.app
                                        else if url.contains("izzysoft.de") && url.contains("/apk/") {
                                            if let Some(start) = url.find("/apk/") {
                                                let after = &url[start + 5..];
                                                let end = after.find('/').unwrap_or(after.len());
                                                let pkg = after[..end].trim();
                                                if !pkg.is_empty() && pkg.contains('.') {
                                                    return Some(pkg.to_string());
                                                }
                                            }
                                        }
                                    }
                                    None
                                });

                                // Check if app is installed - only use exact package name matching
                                if let Some(ref pkg_name) = package_name {
                                    installed_packages.iter().any(|pkg| &pkg.pkg == pkg_name)
                                } else {
                                    false
                                }
                            };

                            // Group apps by category and count installed/total
                            let mut category_counts: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
                            for app in &fmhy_apps {
                                let entry = category_counts.entry(app.category.clone()).or_insert((0, 0));
                                entry.1 += 1; // total
                                if is_app_installed(app) {
                                    entry.0 += 1; // installed
                                }
                            }

                            // Sort categories alphabetically
                            let mut sorted_categories: Vec<_> = category_counts.iter().collect();
                            sorted_categories.sort_by(|a, b| a.0.cmp(b.0));

                            let mut dashboard = dashcounter("FOSS/FMHY", &mut self.dash_scroll_fmhy)
                                .id_salt("dash_fmhy")
                                .title_ui(|ui| {
                                    if ui.add(icon_button_standard(ICON_INFO.to_string())).clicked() {
                                        let _ = webbrowser::open(&fmhy_list.info_url);
                                    }
                                });

                            // Add a card for each category
                            for (idx, (category, (installed, total))) in sorted_categories.iter().enumerate() {
                                dashboard = dashboard.card_with_description(
                                    category.as_str(),
                                    *installed,
                                    *total,
                                    "installed",
                                    "all"
                                );

                                // Store category name and apps for click handler
                                let category_clone = (*category).clone();
                                let apps_in_category: Vec<crate::tab_apps_control_stt::AppEntry> = fmhy_apps.iter()
                                    .filter(|app| &app.category == *category)
                                    .cloned()
                                    .collect();

                                let ctx_for_storage = ctx_clone.clone();
                                ctx_for_storage.data_mut(|data| {
                                    data.insert_temp(
                                        egui::Id::new(format!("fmhy_category_{}", idx)),
                                        category_clone
                                    );
                                    data.insert_temp(
                                        egui::Id::new(format!("fmhy_apps_{}", idx)),
                                        apps_in_category
                                    );
                                });
                            }

                            ui.add(
                                dashboard.on_click(move |index| {
                                    ctx_clone.data_mut(|data| {
                                        data.insert_temp(egui::Id::new("dashcounter_clicked"), ("fmhy", index));
                                    });
                                })
                            );
                            ui.add_space(20.0);
                        }
                    }
                }
            });
    }

    fn render_debloat_tab(&mut self, ui: &mut egui::Ui) {
        // Get available width for responsive layout
        let available_width = ui.ctx().content_rect().width();

        // Get renderer settings
        let google_play_enabled = self.google_play_renderer.is_enabled;
        let fdroid_enabled = self.fdroid_renderer.is_enabled;
        let apkmirror_enabled = self.apkmirror_renderer.is_enabled;
        // Android package renderer only available on Android platform
        #[cfg(target_os = "android")]
        let android_package_enabled = true;
        #[cfg(not(target_os = "android"))]
        let android_package_enabled = false;

        // Render the refactored debloat tab with MVVM architecture
        if let Some(ref viewmodel) = self.viewmodel {
            // Use the new refactored TabDebloat with responsive width-based routing
            self.tab_debloat.render(
                ui,
                &viewmodel.state,
                available_width,
                viewmodel,
                google_play_enabled,
                fdroid_enabled,
                apkmirror_enabled,
                android_package_enabled,
            );
        } else {
            ui.label("Error: ViewModel not initialized");
        }
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
                            self.tab_debloat_control
                                .update_cached_google_play(pkg_id.clone(), app);
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
                            self.tab_debloat_control
                                .update_cached_google_play(pkg_id.clone(), placeholder);
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
                            self.tab_debloat_control
                                .update_cached_fdroid(pkg_id.clone(), app);
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
                            self.tab_debloat_control
                                .update_cached_fdroid(pkg_id.clone(), placeholder);
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
                            self.tab_debloat_control
                                .update_cached_apkmirror(pkg_id.clone(), app);
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
                            self.tab_debloat_control
                                .update_cached_apkmirror(pkg_id.clone(), placeholder);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn render_scan_tab(&mut self, ui: &mut egui::Ui) {
        // Renderer settings already synced in controller
        self.tab_scan_control.ui(
            self.viewmodel.as_mut(),
            ui,
            &self.settings.hybridanalysis_tag_ignorelist,
        );
    }

    fn render_apps_tab(&mut self, ui: &mut egui::Ui) {
        // Operations queue and refresh already handled in controller
        let has_error = self.tab_apps_control.ui(self.viewmodel.as_mut(), ui);

        // Check if we need to refresh after operations completed
        if self.tab_apps_control.pending_refresh_after_operations {
            self.tab_apps_control.pending_refresh_after_operations = false;
            self.refresh_apps_tab_packages();

            // Clear operation results after refresh so buttons return to normal state
            if let Some(ref queue) = self.tab_apps_control.operations_queue {
                queue.clear_results();
                log::info!("App list refreshed and operation results cleared");
            }
        }

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
                log::debug!("Refreshing packages for apps tab only...");
                match get_all_packages_fingerprints(device) {
                    Ok(packages) => {
                        let store = get_shared_store();
                        store.set_installed_packages(packages.clone());
                        self.tab_apps_control.update_packages(packages);
                        log::debug!("Apps tab packages refreshed");
                    }
                    Err(e) => {
                        log::error!("Failed to refresh packages for apps tab: {}", e);
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

    // another lists https://github.com/MuntashirAkon/android-debloat-list
    pub fn retrieve_uad_ng_lists(&mut self) {
        // Get cache directory from config
        let cache_dir = match &self.config {
            Some(config) => config.cache_dir.clone(),
            None => {
                log::error!("Config not available, cannot retrieve UAD lists");
                return;
            }
        };

        if let Some(uad_lists) = crate::calc_uad_lists::load_uad_ng_lists_blocking(&cache_dir) {
            let shared_store = crate::shared_store_stt::get_shared_store();
            let mut lists = shared_store.uad_ng_lists.lock().unwrap();
            *lists = Some(uad_lists);
        }
    }

    // Stalkerware IOC : https://github.com/AssoEchap/stalkerware-indicators
    pub fn retrieve_stalkerware_indicators(&mut self) {
        const IOC_URL: &str =
            "https://fastly.jsdelivr.net/gh/AssoEchap/stalkerware-indicators@master/ioc.yaml";
        const IOC_FILENAME: &str = "stalkerware_ioc.yaml";
        // Embedded fallback file (pre-downloaded at build time)
        const IOC_FALLBACK: &[u8] = include_bytes!("../resources/stalkerware_ioc.yaml");

        // Get cache directory from config
        let cache_dir = match &self.config {
            Some(config) => config.cache_dir.clone(),
            None => {
                log::error!("Config not available, cannot retrieve stalkerware indicators");
                return;
            }
        };

        let cache_file_path = cache_dir.join(IOC_FILENAME);

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
            log::info!(
                "Stalkerware IoC not found in cache or older than 7 days, downloading from {}",
                IOC_URL
            );

            // Download the file
            let mut request = ehttp::Request::get(IOC_URL);
            request.headers.insert(
                "User-Agent".to_string(),
                format!("uad-shizuku/{}", env!("CARGO_PKG_VERSION")),
            );
            let (sender, receiver) = std::sync::mpsc::channel();

            ehttp::fetch(request, move |result| {
                sender.send(result).ok();
            });

            // Wait for the response (blocking)
            match receiver.recv() {
                Ok(Ok(response)) => {
                    if response.ok {
                        // jsdelivr CDN returns raw YAML, not GitHub HTML
                        // Save the response directly to cache
                        match std::fs::write(&cache_file_path, &response.bytes) {
                            Ok(_) => {
                                log::info!(
                                    "Successfully downloaded and cached stalkerware IoC to {:?}",
                                    cache_file_path
                                );
                            }
                            Err(e) => {
                                log::error!("Failed to write stalkerware IoC to cache: {}", e);
                                // Use embedded fallback
                                match std::fs::write(&cache_file_path, IOC_FALLBACK) {
                                    Ok(_) => {
                                        log::info!(
                                            "Successfully wrote stalkerware IoC fallback to cache"
                                        );
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Failed to write stalkerware IoC fallback to cache: {}",
                                            e
                                        );
                                        return;
                                    }
                                }
                            }
                        }
                    } else {
                        log::error!(
                            "Failed to download stalkerware IoC: HTTP {}, using fallback",
                            response.status
                        );
                        // Use embedded fallback
                        match std::fs::write(&cache_file_path, IOC_FALLBACK) {
                            Ok(_) => {
                                log::info!("Successfully wrote stalkerware IoC fallback to cache");
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to write stalkerware IoC fallback to cache: {}",
                                    e
                                );
                                return;
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    log::error!("Failed to download stalkerware IoC: {}, using fallback", e);
                    // Use embedded fallback
                    match std::fs::write(&cache_file_path, IOC_FALLBACK) {
                        Ok(_) => {
                            log::info!("Successfully wrote stalkerware IoC fallback to cache");
                        }
                        Err(e) => {
                            log::error!("Failed to write stalkerware IoC fallback to cache: {}", e);
                            return;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to receive download response: {}, using fallback", e);
                    // Use embedded fallback
                    match std::fs::write(&cache_file_path, IOC_FALLBACK) {
                        Ok(_) => {
                            log::info!("Successfully wrote stalkerware IoC fallback to cache");
                        }
                        Err(e) => {
                            log::error!("Failed to write stalkerware IoC fallback to cache: {}", e);
                            return;
                        }
                    }
                }
            }
        } else {
            log::info!("Stalkerware IoC found in cache at {:?}", cache_file_path);
        }

        // Load and parse the YAML file
        match std::fs::read_to_string(&cache_file_path) {
            Ok(yaml_content) => {
                match crate::calc_stalkerware::parse_stalkerware_yaml(&yaml_content) {
                    Ok(indicators) => {
                        log::info!("Successfully parsed stalkerware IoC");
                        let shared_store = crate::shared_store_stt::get_shared_store();
                        shared_store.set_stalkerware_indicators(Some(indicators));
                    }
                    Err(e) => {
                        log::error!("Failed to parse stalkerware IoC YAML: {}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to read stalkerware IoC from cache: {}", e);
            }
        }
    }

    /// Extracts embedded data from GitHub HTML response
    /// Data is contained in: <script type="application/json" data-target="react-app.embeddedData">{data}</script>
    fn extract_github_embedded_data(html: &str) -> Option<String> {
        // Find the script tag with embedded data
        let script_start =
            html.find(r#"<script type="application/json" data-target="react-app.embeddedData">"#)?;
        let data_start = script_start
            + r#"<script type="application/json" data-target="react-app.embeddedData">"#.len();
        let data_end = html[data_start..].find("</script>")?;
        let json_str = &html[data_start..data_start + data_end];

        // Parse the embedded JSON to extract the actual file content
        let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;

        // Navigate to the payload > codeViewBlobLayoutRoute.StyledBlob > rawLines
        let raw_lines = parsed
            .get("payload")?
            .get("codeViewBlobLayoutRoute.StyledBlob")?
            .get("rawLines")?
            .as_array()?;

        // Join the lines to reconstruct the file content
        let content: Vec<String> = raw_lines
            .iter()
            .filter_map(|line| line.as_str().map(|s| s.to_string()))
            .collect();

        Some(content.join("\n"))
    }

    // Flags : https://android.googlesource.com/platform/frameworks/base/+/master/core/java/android/content/pm/ApplicationInfo.java
    // Permissions : https://developer.android.com/reference/android/Manifest.permission
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

    fn save_settings(&mut self) {
        // Sync theme selections into settings before persisting
        if let Ok(theme) = get_global_theme().lock() {
            self.settings.theme_mode = theme.theme_mode.to_string();
            self.settings.contrast_level = theme.contrast_level.to_string();
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
        self.settings.virustotal_apikey = self.dlg_settings.virustotal_apikey.clone();
        self.settings.hybridanalysis_apikey = self.dlg_settings.hybridanalysis_apikey.clone();
        self.settings.virustotal_submit = self.dlg_settings.virustotal_submit;
        self.settings.hybridanalysis_submit = self.dlg_settings.hybridanalysis_submit;
        self.settings.hybridanalysis_tag_ignorelist =
            self.dlg_settings.hybridanalysis_tag_ignorelist.clone();
        self.settings.google_play_renderer = self.dlg_settings.google_play_renderer;
        self.settings.fdroid_renderer = self.dlg_settings.fdroid_renderer;
        self.settings.apkmirror_renderer = self.dlg_settings.apkmirror_renderer;
        self.settings.unsafe_app_remove = self.dlg_settings.unsafe_app_remove;
        self.settings.expert_app_remove = self.dlg_settings.expert_app_remove;
        self.settings.autoupdate = self.dlg_settings.autoupdate;

        // Sync unsafe_app_remove and expert_app_remove to tab controls
        self.tab_debloat_control.unsafe_app_remove = self.settings.unsafe_app_remove;
        self.tab_debloat_control.expert_app_remove = self.settings.expert_app_remove;
        self.tab_scan_control.unsafe_app_remove = self.settings.unsafe_app_remove;
        self.tab_scan_control.expert_app_remove = self.settings.expert_app_remove;

        // Sync submit settings to tab_scan_control
        self.tab_scan_control.virustotal_submit_enabled = self.settings.virustotal_submit;
        self.tab_scan_control.hybridanalysis_submit_enabled = self.settings.hybridanalysis_submit;

        // Sync renderer enabled flags to both tabs
        self.tab_scan_control.google_play_renderer_enabled = self.settings.google_play_renderer;
        self.tab_scan_control.fdroid_renderer_enabled = self.settings.fdroid_renderer;
        self.tab_scan_control.apkmirror_renderer_enabled = self.settings.apkmirror_renderer;
        self.tab_debloat_control.google_play_renderer_enabled = self.settings.google_play_renderer;
        self.tab_debloat_control.fdroid_renderer_enabled = self.settings.fdroid_renderer;
        self.tab_debloat_control.apkmirror_renderer_enabled = self.settings.apkmirror_renderer;

        // Check if VirusTotal API key was removed -> stop running scans
        if !old_vt_apikey.is_empty() && self.settings.virustotal_apikey.is_empty() {
            log::info!("VirusTotal API key removed, cancelling running scans");
            if let Ok(mut cancelled) = self.tab_scan_control.vt_scan_cancelled.lock() {
                *cancelled = true;
            }
        }

        // Check if HybridAnalysis API key was removed -> stop running scans
        if !old_ha_apikey.is_empty() && self.settings.hybridanalysis_apikey.is_empty() {
            log::info!("HybridAnalysis API key removed, cancelling running scans");
            if let Ok(mut cancelled) = self.tab_scan_control.ha_scan_cancelled.lock() {
                *cancelled = true;
            }
        }

        // Check if VirusTotal upload was disabled -> stop uploading
        if old_vt_submit && !self.settings.virustotal_submit {
            log::info!("VirusTotal upload disabled, cancelling uploads");
            if let Ok(mut cancelled) = self.tab_scan_control.vt_scan_cancelled.lock() {
                *cancelled = true;
            }
        }

        // Check if HybridAnalysis upload was disabled -> stop uploading
        if old_ha_submit && !self.settings.hybridanalysis_submit {
            log::info!("HybridAnalysis upload disabled, cancelling uploads");
            if let Ok(mut cancelled) = self.tab_scan_control.ha_scan_cancelled.lock() {
                *cancelled = true;
            }
        }

        // Check if Google Play renderer was disabled -> clear caches
        if old_google_play_renderer && !self.settings.google_play_renderer {
            log::info!("Google Play renderer disabled, clearing caches");
            self.google_play_renderer.is_enabled = false;
            let shared_store = crate::shared_store_stt::get_shared_store();
            shared_store.google_play_textures.lock().unwrap().clear();
            self.tab_scan_control.google_play_renderer_enabled = false;
        }

        // Check if F-Droid renderer was disabled -> clear caches
        if old_fdroid_renderer && !self.settings.fdroid_renderer {
            log::info!("F-Droid renderer disabled, clearing caches");
            self.fdroid_renderer.is_enabled = false;
            let shared_store = crate::shared_store_stt::get_shared_store();
            shared_store.fdroid_textures.lock().unwrap().clear();
            self.tab_scan_control.fdroid_renderer_enabled = false;
        }

        // Check if APKMirror renderer was disabled -> clear caches
        if old_apkmirror_renderer && !self.settings.apkmirror_renderer {
            log::info!("APKMirror renderer disabled, clearing caches");
            self.apkmirror_renderer.is_enabled = false;
            let shared_store = crate::shared_store_stt::get_shared_store();
            shared_store.apkmirror_textures.lock().unwrap().clear();
            self.tab_scan_control.apkmirror_renderer_enabled = false;
        }

        // Check if APKMirror auto upload was disabled
        if old_apkmirror_auto_upload && !self.settings.apkmirror_auto_upload {
            log::info!("APKMirror auto upload disabled");
        }

        // Check if Google Play renderer was enabled -> enable renderer and start crawling
        if !old_google_play_renderer && self.settings.google_play_renderer {
            log::info!("Google Play renderer enabled, starting metadata crawling");
            self.google_play_renderer.is_enabled = true;
            self.tab_scan_control.google_play_renderer_enabled = true;

            // Start worker queue and enqueue packages for metadata fetching
            if let Some(ref queue) = self.google_play_queue {
                let shared_store = crate::shared_store_stt::get_shared_store();
                let installed_packages = shared_store.installed_packages.lock().unwrap().clone();
                if !installed_packages.is_empty() {
                    log::info!("Enqueueing {} packages for Google Play metadata fetch", installed_packages.len());
                    let package_ids: Vec<String> = installed_packages.iter().map(|p| p.pkg.clone()).collect();
                    queue.enqueue_batch(package_ids);

                    // Start worker if not already running
                    if let Ok(config) = crate::Config::new() {
                        let db_path = config.db_dir.join("uad.db");
                        queue.start_worker(db_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        // Check if F-Droid renderer was enabled -> enable renderer and start crawling
        if !old_fdroid_renderer && self.settings.fdroid_renderer {
            log::info!("F-Droid renderer enabled, starting metadata crawling");
            self.fdroid_renderer.is_enabled = true;
            self.tab_scan_control.fdroid_renderer_enabled = true;

            // Start worker queue and enqueue packages for metadata fetching
            if let Some(ref queue) = self.fdroid_queue {
                let shared_store = crate::shared_store_stt::get_shared_store();
                let installed_packages = shared_store.installed_packages.lock().unwrap().clone();
                if !installed_packages.is_empty() {
                    log::info!("Enqueueing {} packages for F-Droid metadata fetch", installed_packages.len());
                    let package_ids: Vec<String> = installed_packages.iter().map(|p| p.pkg.clone()).collect();
                    queue.enqueue_batch(package_ids);

                    // Start worker if not already running
                    if let Ok(config) = crate::Config::new() {
                        let db_path = config.db_dir.join("uad.db");
                        queue.start_worker(db_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        // Check if APKMirror renderer was enabled -> enable renderer and start crawling
        if !old_apkmirror_renderer && self.settings.apkmirror_renderer {
            log::info!("APKMirror renderer enabled, starting metadata crawling");
            self.apkmirror_renderer.is_enabled = true;
            self.tab_scan_control.apkmirror_renderer_enabled = true;

            // Start worker queue and enqueue packages for metadata fetching
            if let Some(ref queue) = self.apkmirror_queue {
                let shared_store = crate::shared_store_stt::get_shared_store();
                let installed_packages = shared_store.installed_packages.lock().unwrap().clone();
                if !installed_packages.is_empty() {
                    log::info!("Enqueueing {} packages for APKMirror metadata fetch", installed_packages.len());
                    let package_ids: Vec<String> = installed_packages.iter().map(|p| p.pkg.clone()).collect();
                    queue.enqueue_batch(package_ids);

                    // Start worker if not already running
                    if let Ok(config) = crate::Config::new() {
                        let db_path = config.db_dir.join("uad.db");
                        queue.start_worker(db_path.to_string_lossy().to_string());
                    }
                }
            }
        }

        // Check if VirusTotal API key was added -> start scan
        if old_vt_apikey.is_empty() && !self.settings.virustotal_apikey.is_empty() {
            log::info!("VirusTotal API key added, starting scan");
            self.tab_scan_control.vt_api_key = Some(self.settings.virustotal_apikey.clone());
            // Reset cancelled flag and trigger scan start via update_packages
            if let Ok(mut cancelled) = self.tab_scan_control.vt_scan_cancelled.lock() {
                *cancelled = false;
            }
            // Re-trigger scan by calling update_packages if packages are already loaded
            let shared_store = crate::shared_store_stt::get_shared_store();
            let installed_packages = shared_store.installed_packages.lock().unwrap().clone();
            if !installed_packages.is_empty() {
                self.tab_scan_control
                    .update_packages(installed_packages, self.viewmodel.as_ref());
            }
        }

        // Check if HybridAnalysis API key was added -> start scan
        if old_ha_apikey.is_empty() && !self.settings.hybridanalysis_apikey.is_empty() {
            log::info!("HybridAnalysis API key added, starting scan");
            self.tab_scan_control.ha_api_key = Some(self.settings.hybridanalysis_apikey.clone());
            // Reset cancelled flag and trigger scan start via update_packages
            if let Ok(mut cancelled) = self.tab_scan_control.ha_scan_cancelled.lock() {
                *cancelled = false;
            }
            // Re-trigger scan by calling update_packages if packages are already loaded
            let shared_store = crate::shared_store_stt::get_shared_store();
            let installed_packages = shared_store.installed_packages.lock().unwrap().clone();
            if !installed_packages.is_empty() {
                self.tab_scan_control
                    .update_packages(installed_packages, self.viewmodel.as_ref());
            }
        }

        if self.dlg_settings.invalidate_cache {
            invalidate_cache();
            self.dlg_settings.invalidate_cache = false;
        }

        // Flush individual tables if requested
        if self.dlg_settings.flush_virustotal {
            flush_virustotal();
            self.dlg_settings.flush_virustotal = false;
        }
        if self.dlg_settings.flush_hybridanalysis {
            flush_hybridanalysis();
            self.dlg_settings.flush_hybridanalysis = false;
        }
        if self.dlg_settings.flush_googleplay {
            flush_googleplay();
            self.dlg_settings.flush_googleplay = false;
        }
        if self.dlg_settings.flush_fdroid {
            flush_fdroid();
            self.dlg_settings.flush_fdroid = false;
        }
        if self.dlg_settings.flush_apkmirror {
            flush_apkmirror();
            self.dlg_settings.flush_apkmirror = false;
        }

        // Update log settings for in-app log display
        update_log_settings(LogSettings {
            show_logs: self.settings.show_logs,
            log_level: Self::string_to_log_level(&self.settings.log_level),
        });

        // Update log level in real-time
        log::info!("Updating log level to: {}", self.settings.log_level);
        crate::log_capture::update_log_level(&self.settings.log_level);

        // Save to file
        if let Some(ref config) = self.config {
            match config.save_settings(&self.settings) {
                Ok(_) => {
                    log::info!("Settings saved successfully");
                }
                Err(e) => {
                    log::error!("Failed to save settings: {}", e);
                }
            }
        } else {
            log::error!("Config not available, cannot save settings");
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
        // Lazy initialize ViewModel on first update (when we have real context)
        if self.viewmodel.is_none() {
            log::info!("Initializing ViewModel with real egui context");
            self.viewmodel = Some(ViewModel::new(ctx.clone()));
        }

        // Poll ViewModel events
        if let Some(ref mut vm) = self.viewmodel {
            let _vm_events = vm.poll_events(ctx);
            // TODO: Handle events when actors are implemented
        }

        // On first update, initialize device list (Android only)
        // This happens after Android context is fully initialized
        #[cfg(target_os = "android")]
        if !self.first_update_done {
            self.first_update_done = true;

            // Set UI context for background threads to request repaints
            let shared_store = crate::shared_store_stt::get_shared_store();
            shared_store.set_ui_context(ctx.clone());

            log::info!("First update - initializing Shizuku");
            crate::calc::retrieve_adb_devices(self);

            // Check for updates if autoupdate is enabled
            if self.settings.autoupdate {
                log::info!("Autoupdate enabled - checking for updates");
                self.check_for_update();
            }
        }

        #[cfg(not(target_os = "android"))]
        if !self.first_update_done {
            self.first_update_done = true;

            // Set UI context for background threads to request repaints
            let shared_store = crate::shared_store_stt::get_shared_store();
            shared_store.set_ui_context(ctx.clone());

            // Check for updates if autoupdate is enabled
            if self.settings.autoupdate {
                log::info!("Autoupdate enabled - checking for updates");
                self.check_for_update();
            }
        }

        // Poll Shizuku state: auto-retry when permission granted or service bound
        #[cfg(target_os = "android")]
        let needs_shizuku_polling = {
            let mut needs_polling = false;
            if self.shizuku_permission_requested && self.adb_devices.is_empty() {
                let perm_state = crate::android_shizuku::shizuku_get_permission_state();
                if perm_state == 2 {
                    // Permission granted, retry device detection
                    self.shizuku_permission_requested = false;
                    crate::calc::retrieve_adb_devices(self);
                } else {
                    needs_polling = true;
                }
            }
            if self.shizuku_bind_requested && self.adb_devices.is_empty() {
                let bind_state = crate::android_shizuku::shizuku_get_bind_state();
                if bind_state == 2 {
                    // Service bound, retry device detection
                    self.shizuku_bind_requested = false;
                    crate::calc::retrieve_adb_devices(self);
                } else if bind_state == 3 {
                    // Bind failed, stop polling
                    self.shizuku_bind_requested = false;
                } else {
                    needs_polling = true;
                }
            }
            needs_polling
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            #[cfg(target_os = "android")]
            {
                if let Some(multi_touch) = ctx.input(|i| i.multi_touch()) {
                    if multi_touch.num_touches >= 2 {
                        self.zoom_factor =
                            (self.zoom_factor * multi_touch.zoom_delta).clamp(0.25, 3.0);
                    }
                }
                ctx.set_zoom_factor(self.zoom_factor);
            }
            self.ui(ui);
        });

        // Use reactive mode: only repaint when actually needed
        // Only poll for Shizuku state changes if needed (Android only)
        #[cfg(target_os = "android")]
        if needs_shizuku_polling {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}

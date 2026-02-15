use crate::Settings;
use crate::LogLevel;
pub use crate::dlg_settings_stt::*;
use eframe::egui;
use egui_i18n::tr;
use egui_material3::{get_global_theme, ThemeMode, MaterialButton};
use sys_locale::get_locale;

impl DlgSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.save_clicked = false;
        self.theme_to_apply = None;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    fn detect_system_language() -> String {
        match get_locale().as_deref() {
            Some("ko_KR") | Some("ko-KR") | Some("ko") => "ko-KR".to_string(),
            Some("en_US") | Some("en-US") | Some("en_GB") | Some("en-GB") | Some("en") | _ => "en-US".to_string(),
        }
    }

    fn enumerate_system_fonts() -> Vec<(String, String)> {
        let mut fonts: Vec<(String, String)> = Vec::new();

        let font_dirs: Vec<&str> = if cfg!(target_os = "linux") {
            vec!["/usr/share/fonts", "/usr/local/share/fonts"]
        } else if cfg!(target_os = "macos") {
            vec!["/System/Library/Fonts", "/Library/Fonts"]
        } else if cfg!(target_os = "windows") {
            vec!["C:\\Windows\\Fonts"]
        } else {
            vec![]
        };

        for dir in font_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "ttf" || ext_lower == "otf" {
                            if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                                fonts.push((name.to_string(), path.to_string_lossy().to_string()));
                            }
                        }
                    }
                    // Also scan subdirectories one level deep
                    if path.is_dir() {
                        if let Ok(sub_entries) = std::fs::read_dir(&path) {
                            for sub_entry in sub_entries.flatten() {
                                let sub_path = sub_entry.path();
                                if let Some(ext) = sub_path.extension().and_then(|e| e.to_str()) {
                                    let ext_lower = ext.to_lowercase();
                                    if ext_lower == "ttf" || ext_lower == "otf" {
                                        if let Some(name) = sub_path.file_stem().and_then(|n| n.to_str()) {
                                            fonts.push((name.to_string(), sub_path.to_string_lossy().to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        fonts.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        fonts
    }

    pub fn ensure_system_fonts_loaded(&mut self, settings: &Settings) {
        if !self.system_fonts_loaded {
            self.system_fonts = Self::enumerate_system_fonts();
            self.system_fonts_loaded = true;

            if settings.font_path.is_empty() {
                self.selected_font_display = "Default (NotoSansKr)".to_string();
            } else {
                self.selected_font_display = self
                    .system_fonts
                    .iter()
                    .find(|(_, path)| path == &settings.font_path)
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| "Default (NotoSansKr)".to_string());
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

    fn theme_mode_to_string(mode: ThemeMode) -> String {
        match mode {
            ThemeMode::Light => "Light".to_string(),
            ThemeMode::Dark => "Dark".to_string(),
            ThemeMode::Auto => "Auto".to_string(),
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        settings: &mut Settings,
    ) {
        if !self.open {
            return;
        }

        self.ensure_system_fonts_loaded(settings);
        self.save_clicked = false;
        self.theme_to_apply = None;

        let mut close_clicked = false;
        let mut save_clicked = false;

        egui::Window::new("Settings")
            .id(egui::Id::new("settings_window"))
            .title_bar(false)
            .resizable(true)
            .collapsible(false)
            .scroll([false, false])
            .min_width(600.0)
            .min_height(400.0)
            .resize(|r| {
                r.default_size([ctx.screen_rect().width() - 40.0, ctx.screen_rect().height() - 40.0])
                    .max_size([ctx.screen_rect().width() - 40.0, ctx.screen_rect().height() - 40.0])
            })
            .show(ctx, |ui| {
                ui.heading("Settings");
                ui.add_space(8.0);

                let max_height = ui.available_height() - 50.0;

                egui::ScrollArea::both()
                    .id_salt("settings_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        // Language + Font + Text Style
                        ui.horizontal_wrapped(|ui| {
                            // Language Selector
                            ui.label(tr!("language"));
                            let mut selected_lang = settings.language.clone();

                            egui::ComboBox::from_label("   ")
                                .selected_text(match selected_lang.as_str() {
                                    "Auto" => "Auto",
                                    "en-US" => "English",
                                    "ko-KR" => "Korean",
                                    _ => &selected_lang,
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut selected_lang, "Auto".to_string(), "Auto");
                                    ui.selectable_value(&mut selected_lang, "en-US".to_string(), "English");
                                    ui.selectable_value(&mut selected_lang, "ko-KR".to_string(), "Korean");
                                });

                            if selected_lang != settings.language {
                                settings.language = selected_lang.clone();
                                let language_to_apply = if selected_lang == "Auto" {
                                    Self::detect_system_language()
                                } else {
                                    selected_lang
                                };
                                egui_i18n::set_language(&language_to_apply);
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
                                    settings.font_path = String::new();
                                } else if let Some((_, path)) = self
                                    .system_fonts
                                    .iter()
                                    .find(|(name, _)| name == &selected)
                                {
                                    settings.font_path = path.clone();
                                }

                                use egui_material3::theme::{
                                    load_fonts, setup_local_fonts, setup_local_fonts_from_bytes,
                                };
                                if settings.font_path.is_empty() {
                                    setup_local_fonts_from_bytes(
                                        "NotoSansKr",
                                        include_bytes!("../resources/noto-sans-kr.ttf"),
                                    );
                                } else {
                                    setup_local_fonts(Some(&settings.font_path));
                                }
                                load_fonts(ui.ctx());
                            }

                            ui.add_space(8.0);

                            // Text Style Override Selector
                            let mut override_text_style = ui.style().override_text_style.clone();
                            ui.label(tr!("text-style"));
                            egui::ComboBox::from_id_salt("override_text_style")
                                .selected_text(match &override_text_style {
                                    None => "None".to_owned(),
                                    Some(s) => s.to_string(),
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut override_text_style, None, "None");
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

                            let style_string = match text_style {
                                None => String::new(),
                                Some(s) => s.to_string(),
                            };
                            if style_string != settings.override_text_style {
                                settings.override_text_style = style_string;
                            }
                        });
                        ui.add_space(8.0);

                        // Display Size + Color Mode + Theme
                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr!("display-size"));
                            let display_sizes = vec![
                                ("Phone (412x732)", (412.0, 732.0)),
                                ("Tablet (768x1024)", (768.0, 1024.0)),
                                ("Desktop (1024x768)", (1024.0, 768.0)),
                                ("1080p (1920x1080)", (1920.0, 1080.0)),
                            ];
                            let mut selected_size = settings.display_size.clone();
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

                            if selected_size != settings.display_size {
                                settings.display_size = selected_size.clone();
                                if let Some((_, size)) = display_sizes
                                    .iter()
                                    .find(|(label, _)| *label == selected_size)
                                {
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                        egui::vec2(size.0, size.1),
                                    ));
                                    log::info!("Window resized to {}x{}", size.0, size.1);
                                }
                            }
                            ui.add_space(8.0);

                            // Color Mode Selector
                            ui.label(tr!("color-mode"));
                            if let Ok(mut theme) = get_global_theme().lock() {
                                let light_selected = theme.theme_mode == ThemeMode::Light;
                                if ui.selectable_label(light_selected, tr!("light-mode")).clicked() {
                                    theme.theme_mode = ThemeMode::Light;
                                    settings.theme_mode = Self::theme_mode_to_string(ThemeMode::Light);
                                }

                                let auto_selected = theme.theme_mode == ThemeMode::Auto;
                                if ui.selectable_label(auto_selected, tr!("auto-mode")).clicked() {
                                    theme.theme_mode = ThemeMode::Auto;
                                    settings.theme_mode = Self::theme_mode_to_string(ThemeMode::Auto);
                                }

                                let dark_selected = theme.theme_mode == ThemeMode::Dark;
                                if ui.selectable_label(dark_selected, tr!("dark-mode")).clicked() {
                                    theme.theme_mode = ThemeMode::Dark;
                                    settings.theme_mode = Self::theme_mode_to_string(ThemeMode::Dark);
                                }
                            }

                            ui.label(tr!("theme-mode"));
                            let mut selected_theme = settings.theme_name.clone();
                            egui::ComboBox::from_id_salt("theme_selector")
                                .selected_text(match selected_theme.as_str() {
                                    "green" => "Green",
                                    "lightblue" => "Light Blue",
                                    "lightpink" => "Light Pink",
                                    "yellow" => "Yellow",
                                    _ => "Default",
                                })
                                .show_ui(ui, |ui| {
                                    for (value, label) in [
                                        ("default", "Default"),
                                        ("green", "Green"),
                                        ("lightblue", "Light Blue"),
                                        ("lightpink", "Light Pink"),
                                        ("yellow", "Yellow"),
                                    ] {
                                        if ui.selectable_value(&mut selected_theme, value.to_string(), label).clicked() {
                                            settings.theme_name = value.to_string();
                                            self.theme_to_apply = Some(value.to_string());
                                        }
                                    }
                                });
                        });

                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(&mut self.unsafe_app_remove, tr!("allow-unsafe-app-remove"));
                        });

                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(&mut self.autoupdate, tr!("autoupdate"));
                            ui.label(tr!("autoupdate-desc"));
                        });

                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr!("virustotal-api-key"));
                            let response = ui.text_edit_singleline(&mut self.virustotal_apikey);
                            #[cfg(target_os = "android")]
                            {
                                if response.gained_focus() {
                                    let _ = crate::android_inputmethod::show_soft_input();
                                }
                                if response.lost_focus() {
                                    let _ = crate::android_inputmethod::hide_soft_input();
                                }
                            }
                            crate::clipboard_popup::show_clipboard_popup(ui, &response, &mut self.virustotal_apikey);
                            if ui.button(tr!("get-api-key")).clicked() {
                                if let Err(e) = webbrowser::open("https://www.virustotal.com/gui/my-apikey") {
                                    log::error!("Failed to open VirusTotal API key URL: {}", e);
                                }
                            }
                        });

                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(&mut self.virustotal_submit, tr!("allow-virustotal-upload"));
                            ui.label(tr!("virustotal-upload-desc"));
                            ui.checkbox(&mut self.flush_virustotal, tr!("flush"));
                        });

                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr!("hybridanalysis-api-key"));
                            let response = ui.text_edit_singleline(&mut self.hybridanalysis_apikey);
                            #[cfg(target_os = "android")]
                            {
                                if response.gained_focus() {
                                    let _ = crate::android_inputmethod::show_soft_input();
                                }
                                if response.lost_focus() {
                                    let _ = crate::android_inputmethod::hide_soft_input();
                                }
                            }
                            crate::clipboard_popup::show_clipboard_popup(ui, &response, &mut self.hybridanalysis_apikey);
                            if ui.button(tr!("get-api-key")).clicked() {
                                if let Err(e) = webbrowser::open("https://hybrid-analysis.com/my-account") {
                                    log::error!("Failed to open HybridAnalysis API key URL: {}", e);
                                }
                            }
                        });

                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(&mut self.hybridanalysis_submit, tr!("allow-hybridanalysis-upload"));
                            ui.label(tr!("hybridanalysis-upload-desc"));
                            ui.checkbox(&mut self.flush_hybridanalysis, tr!("flush"));
                        });

                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr!("hybridanalysis-tag-ignorelist"));
                            let response = ui.text_edit_singleline(&mut self.hybridanalysis_tag_ignorelist);
                            #[cfg(target_os = "android")]
                            {
                                if response.gained_focus() {
                                    let _ = crate::android_inputmethod::show_soft_input();
                                }
                                if response.lost_focus() {
                                    let _ = crate::android_inputmethod::hide_soft_input();
                                }
                            }
                            crate::clipboard_popup::show_clipboard_popup(ui, &response, &mut self.hybridanalysis_tag_ignorelist);
                        });

                        ui.add_space(8.0);

                        #[cfg(not(target_os = "android"))]
                        {
                            ui.horizontal_wrapped(|ui| {
                                ui.checkbox(&mut self.google_play_renderer, tr!("google-play-renderer"));
                                ui.label(tr!("google-play-renderer-desc"));
                                ui.checkbox(&mut self.flush_googleplay, tr!("flush"));
                            });

                            ui.add_space(8.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.checkbox(&mut self.fdroid_renderer, tr!("fdroid-renderer"));
                                ui.label(tr!("fdroid-renderer-desc"));
                                ui.checkbox(&mut self.flush_fdroid, tr!("flush"));
                            });

                            ui.add_space(8.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.checkbox(&mut self.apkmirror_renderer, tr!("apkmirror-renderer"));
                                ui.label(tr!("apkmirror-renderer-desc"));
                                ui.checkbox(&mut self.flush_apkmirror, tr!("flush"));
                            });

                            ui.add_space(8.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.checkbox(&mut settings.apkmirror_auto_upload, tr!("apkmirror-auto-upload"));
                                ui.label(tr!("apkmirror-auto-upload-desc"));
                            });

                            ui.add_space(8.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.label(tr!("apkmirror-email"));
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut settings.apkmirror_email)
                                        .desired_width(200.0)
                                        .hint_text(tr!("email-hint")),
                                );
                                crate::clipboard_popup::show_clipboard_popup(ui, &response, &mut settings.apkmirror_email);
                            });

                            ui.add_space(8.0);

                            ui.horizontal_wrapped(|ui| {
                                ui.label(tr!("apkmirror-name"));
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut settings.apkmirror_name)
                                        .desired_width(200.0)
                                        .hint_text(tr!("name-hint")),
                                );
                                crate::clipboard_popup::show_clipboard_popup(ui, &response, &mut settings.apkmirror_name);
                            });

                            ui.add_space(8.0);
                        }

                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(&mut self.invalidate_cache, tr!("invalidate-cache"));
                            ui.label(tr!("invalidate-cache-desc"));
                        });

                        ui.add_space(8.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label(tr!("show-logs"));
                            ui.checkbox(&mut settings.show_logs, tr!("show"));

                            let current_level = Self::string_to_log_level(&settings.log_level);

                            for (level, label) in [
                                (LogLevel::Error, "ERROR"),
                                (LogLevel::Warn, "WARN"),
                                (LogLevel::Info, "INFO"),
                                (LogLevel::Debug, "DEBUG"),
                                (LogLevel::Trace, "TRACE"),
                            ] {
                                let selected = current_level == level;
                                if ui.selectable_label(selected, label).clicked() {
                                    settings.log_level = Self::log_level_to_string(level);
                                    crate::log_capture::update_log_level(&settings.log_level);
                                }
                            }
                        });

                        ui.add_space(8.0);
                    });

                ui.add_space(8.0);

                // Action buttons
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(MaterialButton::filled(tr!("save"))).clicked() {
                            log::info!("Settings dialog Save clicked!");
                            save_clicked = true;
                        }
                        if ui.add(MaterialButton::outlined(tr!("cancel"))).clicked() {
                            log::info!("Settings dialog Cancel clicked!");
                            close_clicked = true;
                        }
                    });
                });
            });

        if save_clicked {
            self.save_clicked = true;
            self.close();
        }
        if close_clicked {
            self.close();
        }
    }
}

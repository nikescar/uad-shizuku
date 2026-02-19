pub use crate::dlg_adb_install_stt::*;
use eframe::egui;
use egui_i18n::tr;
use egui_material3::MaterialButton;

impl DlgAdbInstall {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        let os = std::env::consts::OS;

        #[cfg(target_os = "android")]
        let title = tr!("install-dlg-shizuku-title");

        #[cfg(not(target_os = "android"))]
        let title = tr!("install-dlg-adb-title");

        let mut close_clicked = false;
        let mut retry_clicked = false;

        egui::Window::new(&title)
            .id(egui::Id::new("adb_install_window"))
            .title_bar(false)
            .resizable(true)
            .collapsible(false)
            .scroll([false, false])
            .min_width(500.0)
            .min_height(400.0)
            .resize(|r| {
                r.default_size([ctx.content_rect().width() - 40.0, ctx.content_rect().height() - 40.0])
                    .max_size([ctx.content_rect().width() - 40.0, ctx.content_rect().height() - 40.0])
            })
            .show(ctx, |ui| {
                ui.heading(&title);
                ui.add_space(8.0);

                let max_height = ui.available_height() - 50.0;

                egui::ScrollArea::both()
                    .id_salt("adb_install_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        #[cfg(target_os = "android")]
                        {
                            ui.label(tr!("install-dlg-platform-android"));
                            ui.add_space(8.0);
                            ui.label(tr!("install-dlg-shizuku-required"));
                            ui.add_space(16.0);

                            ui.label(tr!("install-dlg-follow-steps"));
                            ui.add_space(8.0);

                            ui.label(tr!("install-dlg-step1"));
                            ui.add_space(4.0);

                            if ui.button(tr!("install-dlg-open-play-store")).clicked() {
                                if let Err(e) = webbrowser::open("https://play.google.com/store/apps/details?id=moe.shizuku.privileged.api") {
                                    log::error!("Failed to open Google Play Store URL: {}", e);
                                }
                            }

                            ui.add_space(8.0);
                            ui.add(egui::Label::new(tr!("install-dlg-step2")).wrap());
                            ui.add_space(4.0);
                            if ui.button(tr!("install-dlg-open-build-number")).clicked() {
                                crate::android_activity::open_build_number_settings();
                            }
                            ui.add_space(4.0);
                            ui.add(egui::Label::new(tr!("install-dlg-step3")).wrap());
                            ui.add_space(4.0);
                            if ui.button(tr!("install-dlg-open-wireless-debug")).clicked() {
                                crate::android_activity::open_wireless_debugging_settings();
                            }
                            ui.add_space(4.0);
                            ui.add(egui::Label::new(tr!("install-dlg-step4")).wrap());
                            ui.add_space(4.0);
                            if ui.button(tr!("install-dlg-open-shizuku")).clicked() {
                                crate::android_activity::open_shizuku_app();
                            }
                            ui.add_space(4.0);
                            ui.add(egui::Label::new(tr!("install-dlg-step5")).wrap());
                            ui.add_space(16.0);

                            ui.label(tr!("install-dlg-detailed-instructions"));
                            ui.add_space(8.0);

                            let guide_url = tr!("install-dlg-guide-url");
                            if ui.button(tr!("install-dlg-guide")).clicked() {
                                if let Err(e) = webbrowser::open(&guide_url) {
                                    log::error!("Failed to open installation guide URL: {}", e);
                                }
                            }

                        }

                        #[cfg(not(target_os = "android"))]
                        {
                            let platform_name = match os {
                                "windows" => "Windows",
                                "macos" => "macOS",
                                "linux" => "Linux",
                                _ => os,
                            };

                            ui.label(format!("Detected platform: {}", platform_name));
                            ui.add_space(8.0);
                            ui.add(egui::Label::new(tr!("install-dlg-adb-not-found")).wrap());
                            ui.add_space(16.0);

                            ui.label(tr!("install-dlg-follow-guide"));
                            ui.add_space(8.0);

                            let guide_url = tr!("install-dlg-guide-url");
                            if ui.button(tr!("install-dlg-guide")).clicked() {
                                if let Err(e) = webbrowser::open(&guide_url) {
                                    log::error!("Failed to open installation guide URL: {}", e);
                                }
                            }

                        }
                    });

                ui.add_space(8.0);

                // Action buttons
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(MaterialButton::filled(tr!("install-dlg-retry"))).clicked() {
                            retry_clicked = true;
                        }
                        if ui.add(MaterialButton::outlined(tr!("close"))).clicked() {
                            close_clicked = true;
                        }
                    });
                });
            });

        if retry_clicked {
            self.retry_requested = true;
            self.close();
        }
        if close_clicked {
            self.close();
        }

        // Check if requirements became available
        #[cfg(target_os = "android")]
        {
            use crate::android_shizuku;
            if android_shizuku::shizuku_is_available() {
                log::info!("Shizuku detected, closing installation dialog");
                self.close();
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            if which::which("adb").is_ok() {
                log::info!("ADB detected, closing installation dialog");
                self.close();
            }
        }
    }
}

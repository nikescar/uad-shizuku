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
        let title = "Shizuku Not Found - Installation Instructions";

        #[cfg(not(target_os = "android"))]
        let title = "ADB Not Found - Installation Instructions";

        let mut close_clicked = false;
        let mut retry_clicked = false;

        egui::Window::new(title)
            .id(egui::Id::new("adb_install_window"))
            .title_bar(false)
            .resizable(true)
            .collapsible(false)
            .scroll([false, false])
            .default_width(ctx.screen_rect().width() - 40.0)
            .default_height(ctx.screen_rect().height() - 40.0)
            .max_width(ctx.screen_rect().width() - 40.0)
            .max_height(ctx.screen_rect().height() - 40.0)
            .show(ctx, |ui| {
                ui.heading(title);
                ui.add_space(8.0);

                let max_height = ui.available_height() - 50.0;

                egui::ScrollArea::both()
                    .id_salt("adb_install_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        #[cfg(target_os = "android")]
                        {
                            ui.label("Detected platform: Android");
                            ui.add_space(8.0);
                            ui.label("Shizuku is required to provide ADB functionality on Android devices.");
                            ui.add_space(16.0);

                            ui.label("Please follow these steps:");
                            ui.add_space(8.0);

                            ui.label("1. Install Shizuku app from Google Play:");
                            ui.add_space(4.0);

                            if ui.button("Open Google Play Store").clicked() {
                                if let Err(e) = webbrowser::open("https://play.google.com/store/apps/details?id=moe.shizuku.privileged.api") {
                                    log::error!("Failed to open Google Play Store URL: {}", e);
                                }
                            }

                            ui.add_space(8.0);
                            ui.add(egui::Label::new("2. Enable Developer Mode (Settings > About > tap Build number 7 times)").wrap());
                            ui.add_space(4.0);
                            ui.add(egui::Label::new("3. Enable Wireless Debugging (Settings > Developer options)").wrap());
                            ui.add_space(4.0);
                            ui.add(egui::Label::new("4. Open Shizuku app and start the service").wrap());
                            ui.add_space(4.0);
                            ui.add(egui::Label::new("5. Return to UAD-Shizuku and tap 'Retry Detection'").wrap());
                            ui.add_space(16.0);

                            ui.label("For detailed instructions:");
                            ui.add_space(8.0);

                            if ui.button("Installation Guide (English)").clicked() {
                                if let Err(e) = webbrowser::open("https://uad-shizuku.pages.dev/docs/installation") {
                                    log::error!("Failed to open installation guide URL: {}", e);
                                }
                            }

                            if ui.button("설치 가이드 (한국어)").clicked() {
                                if let Err(e) = webbrowser::open("https://uad-shizuku.pages.dev/docs/kr/docs/installation") {
                                    log::error!("Failed to open Korean installation guide URL: {}", e);
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
                            ui.add(egui::Label::new("ADB (Android Debug Bridge) is required but not found in your system PATH.").wrap());
                            ui.add_space(16.0);

                            ui.label("Please follow the installation guide to install ADB:");
                            ui.add_space(8.0);

                            if ui.button("Installation Guide (English)").clicked() {
                                if let Err(e) = webbrowser::open("https://uad-shizuku.pages.dev/docs/installation") {
                                    log::error!("Failed to open installation guide URL: {}", e);
                                }
                            }

                            if ui.button("설치 가이드 (한국어)").clicked() {
                                if let Err(e) = webbrowser::open("https://uad-shizuku.pages.dev/docs/kr/docs/installation") {
                                    log::error!("Failed to open Korean installation guide URL: {}", e);
                                }
                            }
                        }
                    });

                ui.add_space(8.0);

                // Action buttons
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(MaterialButton::filled("Retry Detection")).clicked() {
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

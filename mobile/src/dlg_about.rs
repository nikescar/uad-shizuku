pub use crate::dlg_about_stt::*;
use eframe::egui;
use egui_i18n::tr;
use egui_material3::MaterialButton;

impl DlgAbout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.do_check_update = false;
        self.do_perform_update = false;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        update_checking: bool,
        update_available: bool,
        update_status: &str,
    ) {
        if !self.open {
            return;
        }

        self.do_check_update = false;
        self.do_perform_update = false;

        let version = env!("CARGO_PKG_VERSION");
        let description = tr!("about-description");
        let website_label = tr!("about-website");
        let credits_label = tr!("about-credits");

        let mut close_clicked = false;

        egui::Window::new(tr!("about"))
            .id(egui::Id::new("about_window"))
            .title_bar(false)
            .resizable(true)
            .collapsible(false)
            .scroll([false, false])
            .default_width(ctx.screen_rect().width() - 40.0)
            .default_height(ctx.screen_rect().height() - 40.0)
            .max_width(ctx.screen_rect().width() - 40.0)
            .max_height(ctx.screen_rect().height() - 40.0)
            .show(ctx, |ui| {
                ui.heading("UAD-Shizuku");
                ui.add_space(8.0);

                let max_height = ui.available_height() - 50.0;

                egui::ScrollArea::both()
                    .id_salt("about_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("Version: {}", version));

                            ui.add_space(8.0);

                            if update_checking {
                                ui.spinner();
                                ui.label(tr!("checking-update"));
                            } else if update_available {
                                ui.label(update_status);
                                if ui.button(tr!("update-now")).clicked() {
                                    self.do_perform_update = true;
                                }
                            } else if !update_status.is_empty() {
                                ui.label(update_status);
                            } else if ui.button(tr!("check-update")).clicked() {
                                self.do_check_update = true;
                            }
                        });

                        ui.add_space(12.0);

                        // Description
                        ui.add(egui::Label::new(&description).wrap());

                        ui.add_space(12.0);

                        // Website
                        ui.horizontal_wrapped(|ui| {
                            ui.label(format!("{}: ", website_label));
                            if ui.button("https://uad-shizuku.pages.dev").clicked() {
                                if let Err(e) = webbrowser::open("https://uad-shizuku.pages.dev") {
                                    log::error!("Failed to open website URL: {}", e);
                                }
                            }
                        });

                        ui.add_space(12.0);

                        // Credits section
                        ui.label(egui::RichText::new(&credits_label).strong());
                        ui.add_space(4.0);

                        ui.heading("Reference Projects");
                        ui.add_space(4.0);
                        ui.add(egui::Label::new("• Universal Android Debloater Next Generation").wrap());
                        ui.add(egui::Label::new("  Cross-platform GUI written in Rust using ADB to debloat non-rooted Android devices.").wrap());
                        ui.label("  License: GPL-3.0");
                        ui.add_space(2.0);
                        ui.label("• bevy_game_template");
                        ui.label("  Template for Bevy game projects");
                        ui.label("  License: MIT/Apache-2.0");
                        ui.add_space(2.0);
                        ui.label("• chatGPTBox");
                        ui.label("  ChatGPT browser extension");
                        ui.label("  License: MIT");
                        ui.add_space(2.0);
                        ui.label("• android-activity");
                        ui.label("  Android activity glue crate");
                        ui.label("  License: MIT/Apache-2.0");
                        ui.add_space(2.0);
                        ui.label("• ai-rules");
                        ui.label("  AI rules configuration");
                        ui.label("  License: Apache-2.0");
                        ui.add_space(2.0);
                        ui.label("• aShell");
                        ui.add(egui::Label::new("  A local ADB shell for Shizuku powered Android devices").wrap());
                        ui.label("  License: GPL-3.0");

                        ui.add_space(12.0);
                        ui.heading("Rust Libraries");
                        ui.add_space(4.0);
                        ui.add(egui::Label::new("• log - Lightweight logging facade (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• tracing - Application-level tracing framework (MIT)").wrap());
                        ui.add(egui::Label::new("• tracing-subscriber - Utilities for tracing subscribers (MIT)").wrap());
                        ui.add(egui::Label::new("• lazy_static - Macro for declaring lazily evaluated statics (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• egui - Immediate mode GUI library (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• eframe - Framework for egui applications (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• egui_extras - Extra functionality for egui (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• serde - Serialization framework (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• serde_json - JSON serialization/deserialization (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• wgpu - Cross-platform graphics API (MIT/Apache-2.0)").wrap());
                        ui.label("• egui-i18n - Internationalization for egui (MIT)");
                        ui.label("• which - Locate installed executables (MIT)");
                        ui.add(egui::Label::new("• ehttp - Minimal HTTP client (MIT/Apache-2.0)").wrap());
                        ui.label("• zip - ZIP archive reading and writing (MIT)");
                        ui.add(egui::Label::new("• anyhow - Flexible error handling (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• ureq - Simple HTTP request library (MIT/Apache-2.0)").wrap());
                        ui.label("• ureq_multipart - Multipart form support for ureq (MIT)");
                        ui.add(egui::Label::new("• regex - Regular expressions (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• chrono - Date and time library (MIT/Apache-2.0)").wrap());
                        ui.label("• jsonpath_lib - JSONPath implementation (MIT)");
                        ui.add(egui::Label::new("• base64 - Base64 encoding/decoding (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• image - Image processing library (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• md5 - MD5 hash function (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• xee-xpath - XPath implementation (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• diesel - Safe, extensible ORM and query builder (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• diesel_migrations - Database migrations for Diesel (MIT/Apache-2.0)").wrap());
                        ui.label("• libsqlite3-sys - Native SQLite3 bindings (MIT)");
                        ui.add(egui::Label::new("• serde-wasm-bindgen - Serde integration for wasm-bindgen (MIT)").wrap());
                        ui.add(egui::Label::new("• wasm-bindgen - WebAssembly interop with JavaScript (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• wasm-bindgen-futures - Async/await support for wasm-bindgen (MIT/Apache-2.0)").wrap());

                        ui.add_space(12.0);
                        ui.heading("Android-Specific Libraries");
                        ui.add_space(4.0);
                        ui.add(egui::Label::new("• ndk-context - Android NDK context access (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• jni - Rust bindings for JNI (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• android_logger - Android logging for Rust (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• android-activity - Glue for building Android applications (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• ndk-sys - Raw FFI bindings to Android NDK (MIT/Apache-2.0)").wrap());

                        ui.add_space(12.0);
                        ui.heading("iOS-Specific Libraries");
                        ui.add_space(4.0);
                        ui.add(egui::Label::new("• bevy - Data-driven game engine (MIT/Apache-2.0)").wrap());
                        ui.label("• bevy_egui - Egui integration for Bevy (MIT)");
                        ui.add(egui::Label::new("• objc2-avf-audio - Rust bindings for AVFAudio framework (MIT)").wrap());

                        ui.add_space(12.0);
                        ui.heading("Linux-Specific Libraries");
                        ui.add_space(4.0);
                        ui.label("• gtk - Rust bindings for GTK 3 (MIT)");
                        ui.label("• gdk - Rust bindings for GDK 3 (MIT)");

                        ui.add_space(12.0);
                        ui.heading("Desktop Libraries");
                        ui.add_space(4.0);
                        ui.add(egui::Label::new("• directories - Platform-specific directory paths (MIT/Apache-2.0)").wrap());
                        ui.add(egui::Label::new("• open - Open files and URLs with default programs (MIT)").wrap());

                        ui.add_space(12.0);
                        ui.heading("WebAssembly Libraries");
                        ui.add_space(4.0);
                        ui.add(egui::Label::new("• sqlite-wasm-vfs - SQLite VFS for WebAssembly (MIT)").wrap());
                        ui.add(egui::Label::new("• sqlite-wasm-rs - SQLite for WebAssembly (MIT/Apache-2.0)").wrap());

                        ui.add_space(12.0);
                        ui.heading("Assets");
                        ui.add_space(4.0);
                        ui.label("• Icons from SVG Repo (CC Attribution)");
                    });

                ui.add_space(8.0);

                // Close button
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(MaterialButton::filled(tr!("ok"))).clicked() {
                            close_clicked = true;
                        }
                    });
                });
            });

        if close_clicked {
            self.close();
        }
    }
}

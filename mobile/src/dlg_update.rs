pub use crate::dlg_update_stt::*;
use eframe::egui;
use egui_i18n::tr;
use egui_material3::MaterialButton;

impl DlgUpdate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.do_update = false;
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        self.do_update = false;

        let mut close_clicked = false;
        let mut update_clicked = false;

        egui::Window::new(tr!("update-available-title"))
            .id(egui::Id::new("update_window"))
            .title_bar(false)
            .resizable(true)
            .collapsible(false)
            .scroll([false, false])
            .default_width(ctx.screen_rect().width() - 40.0)
            .default_height(ctx.screen_rect().height() - 40.0)
            .max_width(ctx.screen_rect().width() - 40.0)
            .max_height(ctx.screen_rect().height() - 40.0)
            .show(ctx, |ui| {
                ui.heading(tr!("update-available-title"));
                ui.add_space(8.0);

                let max_height = ui.available_height() - 50.0;

                egui::ScrollArea::both()
                    .id_salt("update_scroll")
                    .max_height(max_height)
                    .show(ui, |ui| {
                        // Show version info
                        ui.add(egui::Label::new(format!(
                            "{} {} → {}",
                            tr!("update-available-message"),
                            self.current_version,
                            self.latest_version
                        )).wrap());

                        ui.add_space(12.0);

                        // Show release notes if available
                        if !self.release_notes.is_empty() {
                            ui.label(tr!("release-notes"));
                            ui.add_space(4.0);

                            egui::ScrollArea::vertical()
                                .id_salt("update_release_notes_scroll")
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    ui.add(egui::Label::new(&self.release_notes).wrap());
                                });

                            ui.add_space(12.0);
                        }

                        // Platform-specific instructions
                        #[cfg(target_os = "android")]
                        {
                            ui.add(egui::Label::new(tr!("update-android-instruction")).wrap());
                        }

                        #[cfg(not(target_os = "android"))]
                        {
                            ui.add(egui::Label::new(tr!("update-desktop-instruction")).wrap());
                        }
                    });

                ui.add_space(8.0);

                // Action buttons
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(MaterialButton::filled(tr!("update-now"))).clicked() {
                            update_clicked = true;
                        }
                        if ui.add(MaterialButton::outlined(tr!("cancel"))).clicked() {
                            close_clicked = true;
                        }
                    });
                });
            });

        if update_clicked {
            self.do_update = true;
        }
        if close_clicked {
            self.close();
        }
    }
}

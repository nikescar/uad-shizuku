pub use crate::dlg_uninstall_confirm_stt::*;
use eframe::egui;
use egui_i18n::tr;
use egui_material3::dialog;
use std::cell::Cell;

impl DlgUninstallConfirm {
    pub fn open_single(&mut self, pkg: String, is_system: bool) {
        self.packages = vec![pkg];
        self.is_system = vec![is_system];
        self.confirmed = false;
        self.open = true;
    }

    pub fn open_batch(&mut self, packages: Vec<String>, is_system: Vec<bool>) {
        self.packages = packages;
        self.is_system = is_system;
        self.confirmed = false;
        self.open = true;
    }

    pub fn reset(&mut self) {
        self.open = false;
        self.packages.clear();
        self.is_system.clear();
        self.confirmed = false;
        self.app_names.clear();
    }

    /// Renders the dialog. Returns true if the user confirmed uninstall this frame.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        if !self.open {
            return false;
        }

        let do_confirm = Cell::new(false);
        let count = self.packages.len();

        let title = tr!("uninstall-confirm-title");

        dialog(
            "uninstall_confirm_dialog",
            &title,
            &mut self.open,
        )
        .content(|ui| {
            ui.set_width(300.0);
            if count == 1 {
                ui.label(tr!("uninstall-confirm-single", { name: self.packages[0].clone() }));
            } else {
                ui.label(tr!("uninstall-confirm-batch", { count: count }));
            }
        })
        .action(tr!("cancel"), || {})
        .primary_action(tr!("uninstall"), || {
            do_confirm.set(true);
        })
        .show(ctx);

        if do_confirm.get() {
            self.confirmed = true;
            self.open = false;
            return true;
        }

        false
    }
}

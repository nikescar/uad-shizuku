//! Mobile list dialog implementation
//!
//! This module implements a reusable full-screen mobile dialog that displays
//! card-based package lists from different tabs. Currently supports debloat tab,
//! designed to be extended for scan/apps tabs in the future.

use crate::viewmodel::ViewModelState;
pub use crate::dlg_mobile_list_stt::*;
use eframe::egui;

impl DlgMobileList {
    /// Create a new mobile list dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog with specified view type and category filter
    ///
    /// # Arguments
    /// * `view_type` - Which tab's view to display (Debloat, Scan, Apps)
    /// * `category_filter` - Optional category filter (e.g., "recommended", "advanced")
    pub fn open(&mut self, view_type: MobileListViewType, category_filter: Option<String>) {
        self.view_type = view_type;
        self.category_filter = category_filter;
        self.open = true;
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Show the mobile list dialog
    ///
    /// This renders a full-screen window containing the appropriate mobile view
    /// based on the selected view_type. The window is optimized for touch interaction.
    ///
    /// # Arguments
    /// * `ctx` - egui context for rendering
    /// * `vm_state` - ViewModel state (read-only access to packages)
    /// * `tab_debloat_state` - Debloat tab state (mutable for selection, filters)
    /// * `google_play_enabled` - Whether Google Play metadata renderer is enabled
    /// * `fdroid_enabled` - Whether F-Droid metadata renderer is enabled
    /// * `apkmirror_enabled` - Whether APKMirror metadata renderer is enabled
    /// * `android_package_enabled` - Whether Android package metadata renderer is enabled
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        vm_state: &ViewModelState,
        tab_debloat_state: &mut crate::tab_debloat::TabDebloatState,
        google_play_enabled: bool,
        fdroid_enabled: bool,
        apkmirror_enabled: bool,
        android_package_enabled: bool,
    ) {
        if !self.open {
            return;
        }

        let window_title = match &self.view_type {
            MobileListViewType::Debloat => {
                if let Some(ref category) = self.category_filter {
                    format!("Debloat - {}", capitalize_first(category))
                } else {
                    "Debloat Packages".to_string()
                }
            }
        };

        egui::Window::new(window_title)
            .id(egui::Id::new("mobile_list_window"))
            .title_bar(true)
            .resizable(true)
            .collapsible(false)
            .scroll([false, false])
            .resize(|r| {
                r.default_size([
                    ctx.content_rect().width() - 40.0,
                    ctx.content_rect().height() - 40.0,
                ])
                .max_size([
                    ctx.content_rect().width() - 40.0,
                    ctx.content_rect().height() - 40.0,
                ])
            })
            .show(ctx, |ui| {
                // Render appropriate view based on view_type
                match self.view_type {
                    MobileListViewType::Debloat => {
                        crate::tab_debloat::view_mobile::render(
                            ui,
                            vm_state,
                            tab_debloat_state,
                            google_play_enabled,
                            fdroid_enabled,
                            apkmirror_enabled,
                            android_package_enabled,
                        );
                    }
                    // Future: Add Scan, Apps views here
                }

                // Close button at bottom
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        self.close();
                    }
                });
            });
    }
}

/// Capitalize first letter of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_dialog() {
        let dialog = DlgMobileList::new();
        assert!(!dialog.open);
    }

    #[test]
    fn test_open_close() {
        let mut dialog = DlgMobileList::new();

        dialog.open(MobileListViewType::Debloat, Some("recommended".to_string()));
        assert!(dialog.open);
        assert_eq!(dialog.view_type, MobileListViewType::Debloat);
        assert_eq!(dialog.category_filter, Some("recommended".to_string()));

        dialog.close();
        assert!(!dialog.open);
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("recommended"), "Recommended");
        assert_eq!(capitalize_first("advanced"), "Advanced");
        assert_eq!(capitalize_first(""), "");
    }
}

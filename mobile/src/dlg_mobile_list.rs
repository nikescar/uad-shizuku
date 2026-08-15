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
        // Check viewport width and auto-close if >800px
        let current_width = ctx.screen_rect().width();
        if current_width > 800.0 {
            // Close dialog when viewport exceeds mobile threshold
            if self.open {
                log::info!("[MOBILE_LIST] Auto-closing dialog: viewport width {} > 800px", current_width);
                self.close();
                return;
            }
        }
        self.last_width = Some(current_width);

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
            .resizable(false)  // Prevent manual resizing
            .collapsible(false)
            .scroll([false, false])
            .fixed_size([ctx.screen_rect().width(), ctx.screen_rect().height()])
            .default_pos([0.0, 0.0])
            .show(ctx, |ui| {
                // Top-right close button
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.button("✕").clicked() {
                        self.close();
                    }
                });

                ui.add_space(8.0);

                // Render appropriate view based on view_type
                log::info!(
                    "[MOBILE_LIST] Renderer flags - GP: {}, FD: {}, APK: {}, AP: {}",
                    google_play_enabled,
                    fdroid_enabled,
                    apkmirror_enabled,
                    android_package_enabled
                );
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

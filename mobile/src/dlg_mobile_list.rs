//! Mobile list dialog implementation
//!
//! This module implements a reusable full-screen mobile dialog that displays
//! card-based package lists from different tabs. Currently supports debloat tab,
//! designed to be extended for scan/apps tabs in the future.

use crate::adb::PackageFingerprint;
pub use crate::dlg_mobile_list_stt::*;
use crate::viewmodel::ViewModelState;
use eframe::egui;
use std::collections::HashMap;

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
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        vm_state: &ViewModelState,
        tab_debloat_state: &mut crate::tab_debloat::TabDebloatState,
        viewmodel: &crate::viewmodel::ViewModel,
        google_play_enabled: bool,
        fdroid_enabled: bool,
        apkmirror_enabled: bool,
        android_package_enabled: bool,
        installed_packages: &[PackageFingerprint],
        package_risk_scores: &HashMap<String, i32>,
        unsafe_app_remove: bool,
        expert_app_remove: bool,
        hybridanalysis_tag_ignorelist: &str,
    ) {
        // Check viewport width and auto-close if >1010px
        let current_width = ctx.screen_rect().width();
        if current_width > 1010.0 {
            // Close dialog when viewport exceeds mobile threshold
            if self.open {
                log::info!(
                    "[MOBILE_LIST] Auto-closing dialog: viewport width {} > 1010px",
                    current_width
                );
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
            MobileListViewType::Stalkerware | MobileListViewType::IzzyRisk => {
                match &self.risk_state.category {
                    Some(category) => crate::dlg_mobile_risk::window_title(
                        category,
                        self.risk_state.count_enabled,
                        self.risk_state.count_total,
                    ),
                    None => "Details".to_string(),
                }
            }
            MobileListViewType::VirusTotal | MobileListViewType::HybridAnalysis => {
                match &self.scan_state.category {
                    Some(category) => crate::dlg_mobile_scan::window_title(
                        category,
                        self.scan_state.count_enabled,
                        self.scan_state.count_total,
                    ),
                    None => "Details".to_string(),
                }
            }
        };

        let mut close_requested = false;
        let title = window_title.clone();

        egui::Window::new(window_title)
            .id(egui::Id::new("mobile_list_window"))
            .title_bar(false)  // Disable default title bar
            .resizable(true)
            .collapsible(false)
            .scroll([false, false])
            .resize(|r| {
                r.default_size(ctx.screen_rect().size())
                    .max_size(ctx.screen_rect().size())
            })
            .show(ctx, |ui| {
                // Custom title bar with heading and close button
                ui.horizontal(|ui| {
                    ui.heading(&title);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close_requested = true;
                        }
                    });
                });
                ui.separator();

                ui.add_space(8.0);

                // Sync filter state from dialog to tab state
                if let Some(ref category) = self.category_filter {
                    if tab_debloat_state.active_filter.category_filter.as_deref() != Some(category)
                    {
                        log::info!("[MOBILE_LIST] Syncing category filter to: {}", category);
                        tab_debloat_state.active_filter.category_filter = Some(category.clone());

                        // Apply filter via ViewModel
                        let text_filter = if tab_debloat_state.applied_filter_text.is_empty() {
                            None
                        } else {
                            Some(tab_debloat_state.applied_filter_text.clone())
                        };

                        if let Err(e) = viewmodel.filter_packages(
                            text_filter,
                            Some(category.clone()),
                            tab_debloat_state.active_filter.show_only_enabled,
                            tab_debloat_state.active_filter.hide_system_apps,
                        ) {
                            log::error!("[MOBILE_LIST] Failed to apply category filter: {}", e);
                        }

                        ui.ctx().request_repaint();
                    }
                } else if tab_debloat_state.active_filter.category_filter.is_some() {
                    // Dialog has no category filter but tab state does - clear it
                    log::info!("[MOBILE_LIST] Clearing category filter");
                    tab_debloat_state.active_filter.category_filter = None;

                    let text_filter = if tab_debloat_state.applied_filter_text.is_empty() {
                        None
                    } else {
                        Some(tab_debloat_state.applied_filter_text.clone())
                    };

                    if let Err(e) = viewmodel.filter_packages(
                        text_filter,
                        None,
                        tab_debloat_state.active_filter.show_only_enabled,
                        tab_debloat_state.active_filter.hide_system_apps,
                    ) {
                        log::error!("[MOBILE_LIST] Failed to clear category filter: {}", e);
                    }

                    ui.ctx().request_repaint();
                }

                // Render appropriate view based on view_type
                log::debug!(
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
                            viewmodel,
                            google_play_enabled,
                            fdroid_enabled,
                            apkmirror_enabled,
                            android_package_enabled,
                        );
                    }
                    MobileListViewType::Stalkerware | MobileListViewType::IzzyRisk => {
                        crate::dlg_mobile_risk::view_mobile::render(
                            ui,
                            ctx,
                            vm_state,
                            &mut self.risk_state,
                            installed_packages,
                            package_risk_scores,
                            unsafe_app_remove,
                            expert_app_remove,
                            google_play_enabled,
                            fdroid_enabled,
                            apkmirror_enabled,
                            android_package_enabled,
                        );
                    }
                    MobileListViewType::VirusTotal | MobileListViewType::HybridAnalysis => {
                        crate::dlg_mobile_scan::view_mobile::render(
                            ui,
                            ctx,
                            vm_state,
                            &mut self.scan_state,
                            installed_packages,
                            hybridanalysis_tag_ignorelist,
                            unsafe_app_remove,
                            expert_app_remove,
                            google_play_enabled,
                            fdroid_enabled,
                            apkmirror_enabled,
                            android_package_enabled,
                        );
                    }
                }
            });

        if close_requested {
            self.close();
            return;
        }
    }
}

fn capitalize_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
    }
}

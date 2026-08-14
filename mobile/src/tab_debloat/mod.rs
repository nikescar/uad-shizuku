//! Debloat tab module
//!
//! This module implements the refactored debloat tab with MVVM architecture:
//! - `state.rs` - UI state (selection, filters, sorting, dialogs)
//! - `components/mod.rs` - Reusable UI components (desktop, mobile)
//! - `view_desktop.rs` - Desktop layout (800px+) with sidebar
//! - `view_mobile.rs` - Mobile layout (<800px) with card layout
//!
//! The main `TabDebloat` struct implements width-based responsive routing:
//! - Desktop view (800px+): Full-featured data table with all controls
//! - Mobile view (<800px): Card-based list view for small screens

pub mod components;
pub mod state;
pub mod view_desktop;
pub mod view_mobile;

pub use state::{
    BatchUninstallState, CachedCategoryCounts, DebloatFilter, SortColumn, TabDebloatState,
};

use eframe::egui;

/// Width threshold (pixels) for switching between desktop and mobile views
const RESPONSIVE_WIDTH_THRESHOLD: f32 = 800.0;

/// Filter debounce duration (milliseconds) - prevents excessive filtering while typing
const FILTER_DEBOUNCE_MS: u64 = 300;

/// Debloat tab controller - coordinates UI rendering and state management
///
/// This struct is responsible for:
/// 1. Rendering the appropriate view based on screen width
/// 2. Handling user interactions (filtering, sorting, selection)
/// 3. Coordinating with ViewModel for data access
/// 4. Managing dialog states (package details, uninstall confirmation)
///
/// The controller uses width-based routing to switch between:
/// - Desktop view (800px+): Full data table with virtual scrolling
/// - Mobile view (<800px): Simplified list interface
#[derive(Debug)]
pub struct TabDebloat {
    /// Tab UI state
    pub state: TabDebloatState,
}

impl Default for TabDebloat {
    fn default() -> Self {
        Self {
            state: TabDebloatState::default(),
        }
    }
}

impl TabDebloat {
    /// Create a new debloat tab controller
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the debloat tab with responsive width-based routing
    ///
    /// This method determines which view to render based on available width:
    /// - If width >= 800px: render desktop view with full data table
    /// - If width < 800px: render mobile view with simplified interface
    ///
    /// It also handles filter debouncing (300ms) to prevent excessive filtering
    /// when the user is typing in the search box.
    ///
    /// # Arguments
    /// * `ui` - egui context for rendering
    /// * `vm_state` - ViewModel state (read-only access to packages and UAD lists)
    /// * `available_width` - available width in pixels for layout
    /// * `viewmodel` - ViewModel reference for sending filter commands
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        vm_state: &crate::viewmodel::ViewModelState,
        available_width: f32,
        viewmodel: &crate::viewmodel::ViewModel,
        google_play_enabled: bool,
        fdroid_enabled: bool,
        apkmirror_enabled: bool,
        android_package_enabled: bool,
    ) {
        // Check if filter debounce has elapsed and we need to apply the filter
        if let Some(last_input_time) = self.state.last_filter_input {
            let elapsed = last_input_time.elapsed();
            if elapsed.as_millis() >= FILTER_DEBOUNCE_MS as u128
                && self.state.pending_filter_text != self.state.applied_filter_text
            {
                // Debounce elapsed, apply the pending filter
                let text_filter = if self.state.pending_filter_text.is_empty() {
                    None
                } else {
                    Some(self.state.pending_filter_text.clone())
                };

                if let Err(e) = viewmodel.filter_packages(
                    text_filter,
                    self.state.active_filter.category_filter.clone(),
                    self.state.active_filter.show_only_enabled,
                    self.state.active_filter.hide_system_apps,
                ) {
                    log::error!("Failed to send filter command: {}", e);
                } else {
                    // Mark filter as applied
                    self.state.applied_filter_text = self.state.pending_filter_text.clone();
                    self.state.last_filter_input = None;
                }
            }
        }

        // Check if non-text filters changed (category, checkboxes)
        let current_filter_snapshot = DebloatFilter {
            text_filter: self.state.applied_filter_text.clone(),
            category_filter: self.state.active_filter.category_filter.clone(),
            show_only_enabled: self.state.active_filter.show_only_enabled,
            hide_system_apps: self.state.active_filter.hide_system_apps,
        };

        if current_filter_snapshot != self.state.last_applied_filter {
            // Filter changed, apply immediately
            let text_filter = if current_filter_snapshot.text_filter.is_empty() {
                None
            } else {
                Some(current_filter_snapshot.text_filter.clone())
            };

            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                current_filter_snapshot.category_filter.clone(),
                current_filter_snapshot.show_only_enabled,
                current_filter_snapshot.hide_system_apps,
            ) {
                log::error!("Failed to send filter command: {}", e);
            } else {
                log::debug!(
                    "Applied filter update: category={:?}, enabled={}, hide_system={}",
                    self.state.active_filter.category_filter,
                    self.state.active_filter.show_only_enabled,
                    self.state.active_filter.hide_system_apps
                );
                self.state.last_applied_filter = current_filter_snapshot;
            }
        }

        self.state.cached_counts =
            compute_category_counts(&vm_state.packages, vm_state.uad_ng_lists.as_ref());

        if available_width >= RESPONSIVE_WIDTH_THRESHOLD {
            self.render_desktop(
                ui,
                vm_state,
                viewmodel,
                google_play_enabled,
                fdroid_enabled,
                apkmirror_enabled,
                android_package_enabled,
            );
        } else {
            self.render_mobile(
                ui,
                vm_state,
                google_play_enabled,
                fdroid_enabled,
                apkmirror_enabled,
                android_package_enabled,
            );
        }
    }

    /// Render desktop view (800px+)
    ///
    /// Full-featured interface with:
    /// - Left sidebar: Category filters, options, advanced settings
    /// - Main content: Search bar, batch actions, error banner, virtual table
    /// - Virtual scrolling data table for performance
    fn render_desktop(
        &mut self,
        ui: &mut egui::Ui,
        vm_state: &crate::viewmodel::ViewModelState,
        viewmodel: &crate::viewmodel::ViewModel,
        google_play_enabled: bool,
        fdroid_enabled: bool,
        apkmirror_enabled: bool,
        android_package_enabled: bool,
    ) {
        view_desktop::render(
            ui,
            vm_state,
            &mut self.state,
            viewmodel,
            google_play_enabled,
            fdroid_enabled,
            apkmirror_enabled,
            android_package_enabled,
        );

        // Render dialogs on top of content
        self.state.package_details_dialog.show(
            ui.ctx(),
            &vm_state.filtered_packages,
            &vm_state.uad_ng_lists,
        );

        self.state.uninstall_confirm_dialog.show(ui.ctx());
    }

    /// Render mobile view (<800px)
    ///
    /// Simplified interface with:
    /// - Stacked vertical layout
    /// - Collapsible filter section
    /// - Card-based list (48px minimum per card)
    /// - Batch actions at bottom
    fn render_mobile(
        &mut self,
        ui: &mut egui::Ui,
        vm_state: &crate::viewmodel::ViewModelState,
        google_play_enabled: bool,
        fdroid_enabled: bool,
        apkmirror_enabled: bool,
        android_package_enabled: bool,
    ) {
        view_mobile::render(
            ui,
            vm_state,
            &mut self.state,
            google_play_enabled,
            fdroid_enabled,
            apkmirror_enabled,
            android_package_enabled,
        );

        // Render dialogs on top of content
        self.state.package_details_dialog.show(
            ui.ctx(),
            &vm_state.filtered_packages,
            &vm_state.uad_ng_lists,
        );

        self.state.uninstall_confirm_dialog.show(ui.ctx());
    }
}

/// Check if a package is enabled (not disabled, not removed)
///
/// Uses the same logic as scan table (tab_scan_control.rs:1189-1209):
/// A package is disabled if:
/// - enabled == 2 (disabled)
/// - enabled == 3 (disabled-user)
/// - enabled == 0 && !installed && is_system (removed system user)
fn is_package_enabled(package: &crate::adb::PackageFingerprint) -> bool {
    package.users.first().map(|user| {
        let enabled = user.enabled;
        let installed = user.installed;
        let is_system = package.flags.contains("SYSTEM");

        let is_removed_user = enabled == 0 && !installed && is_system;
        let is_disabled = enabled == 2;
        let is_disabled_user = enabled == 3;

        !(is_removed_user || is_disabled || is_disabled_user)
    }).unwrap_or(false)
}

/// Compute per-category package counts from the UAD-NG lists (matches
/// `AppEntry.removal`: "Recommended" / "Advanced" / "Unsafe" / "Expert").
fn compute_category_counts(
    packages: &[crate::adb::PackageFingerprint],
    uad_ng_lists: Option<&crate::uad_shizuku_app::UadNgLists>,
) -> CachedCategoryCounts {
    let mut counts = CachedCategoryCounts {
        all: packages.len(),
        all_enabled: 0,
        recommended: 0,
        recommended_enabled: 0,
        advanced: 0,
        advanced_enabled: 0,
        unsafe_apps: 0,
        unsafe_apps_enabled: 0,
        expert: 0,
        expert_enabled: 0,
    };

    // Count enabled packages using correct logic
    for pkg in packages {
        if is_package_enabled(pkg) {
            counts.all_enabled += 1;
        }
    }

    if let Some(lists) = uad_ng_lists {
        for pkg in packages {
            let is_enabled = is_package_enabled(pkg);
            match lists.apps.get(&pkg.pkg).map(|app| app.removal.as_str()) {
                Some("Recommended") => {
                    counts.recommended += 1;
                    if is_enabled {
                        counts.recommended_enabled += 1;
                    }
                }
                Some("Advanced") => {
                    counts.advanced += 1;
                    if is_enabled {
                        counts.advanced_enabled += 1;
                    }
                }
                Some("Unsafe") => {
                    counts.unsafe_apps += 1;
                    if is_enabled {
                        counts.unsafe_apps_enabled += 1;
                    }
                }
                Some("Expert") => {
                    counts.expert += 1;
                    if is_enabled {
                        counts.expert_enabled += 1;
                    }
                }
                _ => {}
            }
        }
    }

    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tab_debloat() {
        let tab = TabDebloat::new();
        assert!(!tab.state.open);
    }

    #[test]
    fn test_responsive_width_threshold() {
        assert_eq!(RESPONSIVE_WIDTH_THRESHOLD, 800.0);
    }

    #[test]
    fn test_default_tab_debloat() {
        let tab = TabDebloat::default();
        assert_eq!(tab.state.table_version, 0);
        assert!(tab.state.selected_packages.is_empty());
    }
}

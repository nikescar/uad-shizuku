//! Debloat tab UI state management
//!
//! This module defines the state for the debloat tab, including selection,
//! filtering, sorting, and dialog management.

use crate::dlg_package_details::DlgPackageDetails;
use crate::dlg_uninstall_confirm::DlgUninstallConfirm;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Simple confirmation dialog for batch toggle operations
#[derive(Default)]
pub struct DlgBatchToggleConfirm {
    pub open: bool,
    pub is_enabling: bool,
    pub package_ids: Vec<String>,
    pub device: String,
}

impl DlgBatchToggleConfirm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(
        &mut self,
        is_enabled: bool,
        package_ids: std::collections::HashSet<String>,
        device: String,
    ) {
        self.is_enabling = !is_enabled;
        self.package_ids = package_ids.into_iter().collect();
        self.device = device;
        self.open = true;
    }

    pub fn show(
        &mut self,
        ctx: &eframe::egui::Context,
        viewmodel: &crate::viewmodel::ViewModel,
    ) -> bool {
        if !self.open {
            return false;
        }

        let mut confirmed = false;
        let mut cancelled = false;

        let action = if self.is_enabling { "Enable" } else { "Disable" };

        eframe::egui::Window::new(format!("Confirm Batch {}", action))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Are you sure you want to {} {} packages?",
                    action.to_lowercase(),
                    self.package_ids.len()
                ));

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        cancelled = true;
                    }
                    if ui.button("Confirm").clicked() {
                        confirmed = true;
                    }
                });
            });

        if confirmed {
            if self.is_enabling {
                if let Err(e) =
                    viewmodel.batch_enable(self.package_ids.clone(), self.device.clone())
                {
                    log::error!("Failed to batch enable: {}", e);
                }
            } else {
                if let Err(e) =
                    viewmodel.batch_disable(self.package_ids.clone(), self.device.clone())
                {
                    log::error!("Failed to batch disable: {}", e);
                }
            }
            self.open = false;
        }

        if cancelled {
            self.open = false;
        }

        confirmed
    }
}

/// Sort column options for the package table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortColumn {
    PackageName,
    LastUpdate,
    VersionCode,
}

/// Batch operation state for tracking progress
#[derive(Clone, Default)]
pub struct BatchUninstallState {
    pub package_count: usize,
    pub current_index: usize,
    pub current_package: Option<String>,
    pub status_message: String,
}

impl std::fmt::Debug for BatchUninstallState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchUninstallState")
            .field("package_count", &self.package_count)
            .field("current_index", &self.current_index)
            .field("current_package", &self.current_package)
            .field("status_message", &self.status_message)
            .finish()
    }
}

/// Filter options for package display
#[derive(Clone, PartialEq)]
pub struct DebloatFilter {
    /// Text search filter (case-insensitive, searches package name)
    pub text_filter: String,

    /// Show only enabled packages
    pub show_only_enabled: bool,

    /// Hide system apps from view
    pub hide_system_apps: bool,

    /// Active category filter (placeholder for UAD integration)
    pub category_filter: Option<String>,
}

impl std::fmt::Debug for DebloatFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DebloatFilter")
            .field("text_filter", &self.text_filter)
            .field("show_only_enabled", &self.show_only_enabled)
            .field("hide_system_apps", &self.hide_system_apps)
            .field("category_filter", &self.category_filter)
            .finish()
    }
}

impl Default for DebloatFilter {
    fn default() -> Self {
        Self {
            text_filter: String::new(),
            show_only_enabled: false,
            hide_system_apps: false,
            category_filter: None,
        }
    }
}

/// Debloat tab UI state
///
/// This struct holds all UI-specific state for the debloat tab:
/// - Selection state (which packages are selected)
/// - Filter state (text filters, category filters, toggles)
/// - Sorting state (column, direction)
/// - Dialog state (package details, uninstall confirmation)
/// - Batch operation progress
/// - Error handling
///
/// The ViewModel provides the underlying data (packages, UAD lists),
/// while this struct manages UI presentation and user interaction state.
pub struct TabDebloatState {
    /// Open/closed state of the tab
    pub open: bool,

    /// Selected package names (for multi-select actions)
    pub selected_packages: HashSet<String>,

    /// Current active filter
    pub active_filter: DebloatFilter,

    /// Current sort column (None = unsorted)
    pub sort_column: Option<SortColumn>,

    /// Sort direction (true = ascending)
    pub sort_ascending: bool,

    /// Selected device name
    pub selected_device: Option<String>,

    /// Filter debounce: timestamp of last text input change (for 300ms debounce)
    pub last_filter_input: Option<std::time::Instant>,

    /// Filter debounce: pending filter text (user is still typing)
    pub pending_filter_text: String,

    /// Filter debounce: last applied filter text (already sent to ViewModel)
    pub applied_filter_text: String,

    /// Package details dialog state
    pub package_details_dialog: DlgPackageDetails,

    /// Uninstall confirmation dialog state
    pub uninstall_confirm_dialog: DlgUninstallConfirm,

    /// Mobile package info dialog
    pub mobile_info_dialog: crate::dlg_package_info_mobile::DlgPackageInfoMobile,

    /// Batch toggle confirmation dialog
    pub batch_toggle_confirm: DlgBatchToggleConfirm,

    /// Cached category counts for quick display
    pub cached_counts: CachedCategoryCounts,

    /// Unsafe removal allowed (expert mode)
    pub unsafe_app_remove: bool,

    /// Expert mode enabled (allow dangerous operations)
    pub expert_app_remove: bool,

    /// Batch uninstall operation state
    pub batch_uninstall_state: BatchUninstallState,

    /// Progress tracking for batch uninstall (wrapped for thread safety)
    pub batch_uninstall_progress: Arc<Mutex<Option<f32>>>,

    /// Cancellation flag for batch uninstall
    pub batch_uninstall_cancelled: Arc<Mutex<bool>>,

    /// Batch disable operation state
    pub batch_disable_state: BatchUninstallState,

    /// Progress tracking for batch disable
    pub batch_disable_progress: Arc<Mutex<Option<f32>>>,

    /// Cancellation flag for batch disable
    pub batch_disable_cancelled: Arc<Mutex<bool>>,

    /// Batch enable operation state
    pub batch_enable_state: BatchUninstallState,

    /// Progress tracking for batch enable
    pub batch_enable_progress: Arc<Mutex<Option<f32>>>,

    /// Cancellation flag for batch enable
    pub batch_enable_cancelled: Arc<Mutex<bool>>,
}

impl std::fmt::Debug for TabDebloatState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabDebloatState")
            .field("open", &self.open)
            .field("selected_packages_count", &self.selected_packages.len())
            .field("sort_column", &self.sort_column)
            .field("sort_ascending", &self.sort_ascending)
            .field("selected_device", &self.selected_device)
            .field("unsafe_app_remove", &self.unsafe_app_remove)
            .field("expert_app_remove", &self.expert_app_remove)
            .finish()
    }
}

impl Default for TabDebloatState {
    fn default() -> Self {
        Self {
            open: false,
            selected_packages: HashSet::new(),
            active_filter: DebloatFilter::default(),
            sort_column: None,
            sort_ascending: true,
            selected_device: None,
            last_filter_input: None,
            pending_filter_text: String::new(),
            applied_filter_text: String::new(),
            package_details_dialog: DlgPackageDetails::new(),
            uninstall_confirm_dialog: DlgUninstallConfirm::default(),
            mobile_info_dialog: crate::dlg_package_info_mobile::DlgPackageInfoMobile::new(),
            batch_toggle_confirm: DlgBatchToggleConfirm::new(),
            cached_counts: CachedCategoryCounts::default(),
            unsafe_app_remove: false,
            expert_app_remove: false,
            batch_uninstall_state: BatchUninstallState::default(),
            batch_uninstall_progress: Arc::new(Mutex::new(None)),
            batch_uninstall_cancelled: Arc::new(Mutex::new(false)),
            batch_disable_state: BatchUninstallState::default(),
            batch_disable_progress: Arc::new(Mutex::new(None)),
            batch_disable_cancelled: Arc::new(Mutex::new(false)),
            batch_enable_state: BatchUninstallState::default(),
            batch_enable_progress: Arc::new(Mutex::new(None)),
            batch_enable_cancelled: Arc::new(Mutex::new(false)),
        }
    }
}

/// Cached category counts for performance
#[derive(Debug, Clone, Default)]
pub struct CachedCategoryCounts {
    pub all: usize,
    pub all_enabled: usize,
    pub recommended: usize,
    pub recommended_enabled: usize,
    pub advanced: usize,
    pub advanced_enabled: usize,
    pub unsafe_apps: usize,
    pub unsafe_apps_enabled: usize,
    pub expert: usize,
    pub expert_enabled: usize,
    pub unknown_apps: usize,
    pub unknown_apps_enabled: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = TabDebloatState::default();
        assert!(!state.open);
        assert!(state.selected_packages.is_empty());
        assert_eq!(state.sort_column, None);
        assert!(state.sort_ascending);
    }

    #[test]
    fn test_default_filter() {
        let filter = DebloatFilter::default();
        assert!(filter.text_filter.is_empty());
        assert!(!filter.show_only_enabled);
        assert!(!filter.hide_system_apps);
        assert!(filter.category_filter.is_none());
    }

    #[test]
    fn test_batch_operation_state_default() {
        let state = BatchUninstallState::default();
        assert_eq!(state.package_count, 0);
        assert_eq!(state.current_index, 0);
        assert!(state.current_package.is_none());
    }

    #[test]
    fn test_filter_equality() {
        let filter1 = DebloatFilter {
            text_filter: "test".to_string(),
            category_filter: Some("recommended".to_string()),
            show_only_enabled: true,
            hide_system_apps: false,
        };

        let filter2 = filter1.clone();
        assert_eq!(filter1, filter2);

        let filter3 = DebloatFilter {
            category_filter: Some("unsafe".to_string()),
            ..filter1.clone()
        };
        assert_ne!(filter1, filter3);
    }

    #[test]
    fn test_filter_change_detection() {
        let filter1 = DebloatFilter::default();
        let mut filter2 = filter1.clone();

        // Same filter should be equal
        assert_eq!(filter1, filter2);

        // Change category
        filter2.category_filter = Some("recommended".to_string());
        assert_ne!(filter1, filter2);

        // Change checkbox
        let mut filter3 = filter1.clone();
        filter3.show_only_enabled = true;
        assert_ne!(filter1, filter3);

        // Change text
        let mut filter4 = filter1.clone();
        filter4.text_filter = "search".to_string();
        assert_ne!(filter1, filter4);
    }
}

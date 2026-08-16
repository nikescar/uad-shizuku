//! State for the mobile VirusTotal/HybridAnalysis scan table.

use crate::dlg_mobile_scan::ScanCategory;
use crate::dlg_package_details::DlgPackageDetails;
use crate::dlg_uninstall_confirm::DlgUninstallConfirm;

/// UI state for a VirusTotal/HybridAnalysis drill-down: which category is active, the local
/// filters, and the two dialogs it owns (package info, uninstall confirmation). Lives as
/// long as `dlg_mobile_list.rs`'s `MobileListViewType::VirusTotal`/`HybridAnalysis` is open.
pub struct ScanTableState {
    pub category: Option<ScanCategory>,
    pub count_enabled: usize,
    pub count_total: usize,
    pub show_only_enabled: bool,
    pub hide_system_app: bool,
    pub text_filter: String,
    pub mobile_info_dialog: DlgPackageDetails,
    pub uninstall_confirm_dialog: DlgUninstallConfirm,
}

impl Default for ScanTableState {
    fn default() -> Self {
        Self {
            category: None,
            count_enabled: 0,
            count_total: 0,
            show_only_enabled: false,
            hide_system_app: false,
            text_filter: String::new(),
            mobile_info_dialog: DlgPackageDetails::new(),
            uninstall_confirm_dialog: DlgUninstallConfirm::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = ScanTableState::default();
        assert!(state.category.is_none());
        assert_eq!(state.count_enabled, 0);
        assert_eq!(state.count_total, 0);
        assert!(!state.show_only_enabled);
        assert!(!state.hide_system_app);
        assert!(state.text_filter.is_empty());
    }
}

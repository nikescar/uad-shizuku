//! Mobile list dialog state
//!
//! This module defines the state for a reusable mobile list dialog that can display
//! card-based package lists from different tabs (debloat, scan, apps).

pub struct DlgMobileList {
    /// Whether the dialog is currently open
    pub open: bool,

    /// Category filter to apply (e.g., "recommended", "advanced", "expert", "unsafe")
    pub category_filter: Option<String>,

    /// Which tab's view to display
    pub view_type: MobileListViewType,

    /// Track last viewport width for resize detection
    pub last_width: Option<f32>,

    /// State for the IzzyRisk/Stalkerware drill-down (category, filters, owned dialogs)
    pub risk_state: crate::dlg_mobile_risk::RiskTableState,

    /// State for the VirusTotal/HybridAnalysis drill-down (category, filters, owned dialogs)
    pub scan_state: crate::dlg_mobile_scan::ScanTableState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MobileListViewType {
    /// Show debloat tab mobile view
    Debloat,
    /// Show stalkerware dashcounter drill-down (detected/undetected)
    Stalkerware,
    /// Show IzzyRisk dashcounter drill-down (safe/normal/moderate/high)
    IzzyRisk,
    /// Show VirusTotal dashcounter drill-down (malicious/suspicious/safe/not-scanned)
    VirusTotal,
    /// Show HybridAnalysis dashcounter drill-down (malicious/malicious-ignored/suspicious/safe/not-scanned)
    HybridAnalysis,
    // Future: Apps, Usage
}

impl Default for DlgMobileList {
    fn default() -> Self {
        Self {
            open: false,
            category_filter: None,
            view_type: MobileListViewType::Debloat,
            last_width: None,
            risk_state: crate::dlg_mobile_risk::RiskTableState::default(),
            scan_state: crate::dlg_mobile_scan::ScanTableState::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = DlgMobileList::default();
        assert!(!state.open);
        assert!(state.category_filter.is_none());
        assert_eq!(state.view_type, MobileListViewType::Debloat);
    }

    #[test]
    fn test_view_type_equality() {
        assert_eq!(MobileListViewType::Debloat, MobileListViewType::Debloat);
    }

    #[test]
    fn test_stalkerware_and_izzyrisk_view_types_are_distinct() {
        assert_ne!(
            MobileListViewType::Stalkerware,
            MobileListViewType::IzzyRisk
        );
        assert_ne!(MobileListViewType::Stalkerware, MobileListViewType::Debloat);
        assert_ne!(MobileListViewType::IzzyRisk, MobileListViewType::Debloat);
    }

    #[test]
    fn test_virustotal_and_hybridanalysis_view_types_are_distinct() {
        assert_ne!(
            MobileListViewType::VirusTotal,
            MobileListViewType::HybridAnalysis
        );
        assert_ne!(MobileListViewType::VirusTotal, MobileListViewType::Debloat);
        assert_ne!(
            MobileListViewType::HybridAnalysis,
            MobileListViewType::Stalkerware
        );
    }
}

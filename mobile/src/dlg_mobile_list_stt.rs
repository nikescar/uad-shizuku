//! Mobile list dialog state
//!
//! This module defines the state for a reusable mobile list dialog that can display
//! card-based package lists from different tabs (debloat, scan, apps).

#[derive(Debug, Clone)]
pub struct DlgMobileList {
    /// Whether the dialog is currently open
    pub open: bool,

    /// Category filter to apply (e.g., "recommended", "advanced", "expert", "unsafe")
    pub category_filter: Option<String>,

    /// Which tab's view to display
    pub view_type: MobileListViewType,

    /// Track last viewport width for resize detection
    pub last_width: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MobileListViewType {
    /// Show debloat tab mobile view
    Debloat,
    // Future: Scan, Apps, Usage
}

impl Default for DlgMobileList {
    fn default() -> Self {
        Self {
            open: false,
            category_filter: None,
            view_type: MobileListViewType::Debloat,
            last_width: None,
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
}

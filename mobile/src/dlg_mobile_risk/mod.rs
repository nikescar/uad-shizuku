//! Mobile IzzyRisk/Stalkerware drill-down module.
//!
//! Reimplements the mobile-only rendering for these two dashboard drill-downs without
//! depending on `dlg_dashcounter_details.rs`. See
//! `docs/superpowers/specs/2026-08-16-mobile-risk-table-design.md`.

pub mod components;
pub mod filter_logic;
pub mod state;
pub mod view_mobile;

pub use state::RiskTableState;

/// Category identity for the mobile risk drill-down. Independent of
/// `dlg_dashcounter_details_stt::DashCounterCategory` — the desktop dialog keeps using that
/// enum for its own (unmodified) rendering path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskCategory {
    IzzyRiskSafe,
    IzzyRiskNormal,
    IzzyRiskModerate,
    IzzyRiskHigh,
    StalkerwareDetected,
    StalkerwareUndetected,
}

/// Window title text, matching `DlgDashCounterDetails::get_window_title`'s existing strings
/// for these categories (dlg_dashcounter_details.rs:1138-1141) so the title doesn't change
/// for the user.
pub fn window_title(category: &RiskCategory, count_enabled: usize, count_total: usize) -> String {
    let base = match category {
        RiskCategory::IzzyRiskSafe => "IzzyRisk: Safe (0)",
        RiskCategory::IzzyRiskNormal => "IzzyRisk: Normal (1-10)",
        RiskCategory::IzzyRiskModerate => "IzzyRisk: Moderate (11-20)",
        RiskCategory::IzzyRiskHigh => "IzzyRisk: High (20+)",
        RiskCategory::StalkerwareDetected => "Stalkerware: Detected",
        RiskCategory::StalkerwareUndetected => "Stalkerware: Undetected",
    };
    format!("{} ({}/{})", base, count_enabled, count_total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_category_variants_are_distinct() {
        assert_ne!(RiskCategory::IzzyRiskSafe, RiskCategory::IzzyRiskNormal);
        assert_ne!(
            RiskCategory::StalkerwareDetected,
            RiskCategory::StalkerwareUndetected
        );
        assert_ne!(
            RiskCategory::IzzyRiskHigh,
            RiskCategory::StalkerwareDetected
        );
    }

    #[test]
    fn test_window_title_izzyrisk_safe() {
        assert_eq!(
            window_title(&RiskCategory::IzzyRiskSafe, 3, 5),
            "IzzyRisk: Safe (0) (3/5)"
        );
    }

    #[test]
    fn test_window_title_stalkerware_detected() {
        assert_eq!(
            window_title(&RiskCategory::StalkerwareDetected, 1, 2),
            "Stalkerware: Detected (1/2)"
        );
    }
}

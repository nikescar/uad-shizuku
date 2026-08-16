//! Mobile VirusTotal/HybridAnalysis drill-down module.
//!
//! Reimplements the mobile-only rendering for these two dashboard drill-downs without
//! depending on `dlg_dashcounter_details.rs`. See
//! `docs/superpowers/specs/2026-08-16-mobile-scan-table-design.md`.

pub mod components;
pub mod filter_logic;
pub mod state;
pub mod view_mobile;

pub use state::ScanTableState;

/// Category identity for the mobile scan drill-down. Independent of
/// `dlg_dashcounter_details_stt::DashCounterCategory` — the desktop dialog keeps using that
/// enum for its own (unmodified) rendering path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanCategory {
    VirusTotalMalicious,
    VirusTotalSuspicious,
    VirusTotalSafe,
    VirusTotalNotScanned,
    HybridAnalysisMalicious,
    HybridAnalysisMaliciousIgnored,
    HybridAnalysisSuspicious,
    HybridAnalysisSafe,
    HybridAnalysisNotScanned,
}

/// True for the 4 VirusTotal variants.
pub fn is_virustotal(category: &ScanCategory) -> bool {
    matches!(
        category,
        ScanCategory::VirusTotalMalicious
            | ScanCategory::VirusTotalSuspicious
            | ScanCategory::VirusTotalSafe
            | ScanCategory::VirusTotalNotScanned
    )
}

/// True for the 5 HybridAnalysis variants.
pub fn is_hybridanalysis(category: &ScanCategory) -> bool {
    !is_virustotal(category)
}

/// Window title text, matching `DlgDashCounterDetails::get_window_title`'s existing strings
/// for these categories (dlg_dashcounter_details.rs:1142-1152) so the title doesn't change
/// for the user, with `" ({enabled}/{total})"` appended (matching `dlg_mobile_risk::window_title`'s
/// convention — the desktop title shows no count, the mobile one does).
pub fn window_title(category: &ScanCategory, count_enabled: usize, count_total: usize) -> String {
    let base = match category {
        ScanCategory::VirusTotalMalicious => "VirusTotal: Malicious",
        ScanCategory::VirusTotalSuspicious => "VirusTotal: Suspicious",
        ScanCategory::VirusTotalSafe => "VirusTotal: Safe",
        ScanCategory::VirusTotalNotScanned => "VirusTotal: Not Scanned",
        ScanCategory::HybridAnalysisMalicious => "HybridAnalysis: Malicious",
        ScanCategory::HybridAnalysisMaliciousIgnored => "HybridAnalysis: Malicious (Ignored)",
        ScanCategory::HybridAnalysisSuspicious => "HybridAnalysis: Suspicious",
        ScanCategory::HybridAnalysisSafe => "HybridAnalysis: Safe",
        ScanCategory::HybridAnalysisNotScanned => "HybridAnalysis: Not Scanned",
    };
    format!("{} ({}/{})", base, count_enabled, count_total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_category_variants_are_distinct() {
        assert_ne!(ScanCategory::VirusTotalMalicious, ScanCategory::VirusTotalSuspicious);
        assert_ne!(
            ScanCategory::HybridAnalysisMalicious,
            ScanCategory::HybridAnalysisMaliciousIgnored
        );
        assert_ne!(ScanCategory::VirusTotalSafe, ScanCategory::HybridAnalysisSafe);
    }

    #[test]
    fn test_is_virustotal() {
        assert!(is_virustotal(&ScanCategory::VirusTotalMalicious));
        assert!(is_virustotal(&ScanCategory::VirusTotalNotScanned));
        assert!(!is_virustotal(&ScanCategory::HybridAnalysisMalicious));
    }

    #[test]
    fn test_is_hybridanalysis() {
        assert!(is_hybridanalysis(&ScanCategory::HybridAnalysisSafe));
        assert!(is_hybridanalysis(&ScanCategory::HybridAnalysisMaliciousIgnored));
        assert!(!is_hybridanalysis(&ScanCategory::VirusTotalSafe));
    }

    #[test]
    fn test_window_title_virustotal_malicious() {
        assert_eq!(
            window_title(&ScanCategory::VirusTotalMalicious, 2, 5),
            "VirusTotal: Malicious (2/5)"
        );
    }

    #[test]
    fn test_window_title_hybridanalysis_malicious_ignored() {
        assert_eq!(
            window_title(&ScanCategory::HybridAnalysisMaliciousIgnored, 1, 3),
            "HybridAnalysis: Malicious (Ignored) (1/3)"
        );
    }
}

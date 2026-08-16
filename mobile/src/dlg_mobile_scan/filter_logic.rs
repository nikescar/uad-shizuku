//! Category-matching predicates for the mobile VirusTotal/HybridAnalysis scan table.
//!
//! Ported from `DlgDashCounterDetails::render_virustotal_table`'s filter closure
//! (dlg_dashcounter_details.rs:1772-1821) and `render_hybridanalysis_table`'s
//! (dlg_dashcounter_details.rs:1992-2066), rebased onto an already-resolved `Option<&ScanStatus>`
//! instead of locking `shared_store`'s scanner state inline — callers (`view_mobile.rs`,
//! `components/package_table_mobile.rs`) do that lookup once per package against
//! `ViewModelState::vt_scanner_state`/`ha_scanner_state`.
//!
//! `should_show_package`/`matches_text_filter` are reused from `dlg_mobile_risk::filter_logic`
//! (already category-agnostic) rather than duplicated a third time.

use crate::dlg_mobile_scan::ScanCategory;

/// VirusTotal category bucketing. Mirrors the filter closure in
/// `render_virustotal_table` (dlg_dashcounter_details.rs:1772-1821).
pub fn matches_virustotal_category(
    category: &ScanCategory,
    scan_status: Option<&crate::calc_virustotal_stt::ScanStatus>,
) -> bool {
    use crate::calc_virustotal_stt::ScanStatus;

    match scan_status {
        Some(ScanStatus::Completed(result)) => {
            let has_not_found = result.file_results.iter().any(|fr| fr.not_found);
            let has_skipped = result.file_results.iter().any(|fr| fr.skipped);
            let has_error = result.file_results.iter().any(|fr| fr.error.is_some());

            if has_not_found || has_skipped || has_error {
                matches!(category, ScanCategory::VirusTotalNotScanned)
            } else {
                match category {
                    ScanCategory::VirusTotalMalicious => {
                        result.file_results.iter().any(|f| f.malicious > 0)
                    }
                    ScanCategory::VirusTotalSuspicious => result
                        .file_results
                        .iter()
                        .any(|f| f.suspicious > 0 && f.malicious == 0),
                    ScanCategory::VirusTotalSafe => result
                        .file_results
                        .iter()
                        .all(|f| f.malicious == 0 && f.suspicious == 0),
                    ScanCategory::VirusTotalNotScanned => false,
                    _ => false,
                }
            }
        }
        // Pending, Scanning, Error, or None (not in scanner_state)
        _ => matches!(category, ScanCategory::VirusTotalNotScanned),
    }
}

/// Whether all of a HybridAnalysis file result's `classification_tags` are present in the
/// (comma-split, lowercased) ignorelist. Empty tags count as fully ignored.
fn ha_all_tags_ignored(
    file_result: &crate::calc_hybridanalysis_stt::FileScanResult,
    tag_ignorelist: &str,
) -> bool {
    let ignorelist_tags: Vec<String> = tag_ignorelist
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    if file_result.classification_tags.is_empty() {
        true
    } else {
        file_result
            .classification_tags
            .iter()
            .all(|tag| ignorelist_tags.contains(&tag.to_lowercase()))
    }
}

/// HybridAnalysis category bucketing. Mirrors the filter closure in
/// `render_hybridanalysis_table` (dlg_dashcounter_details.rs:1992-2066).
pub fn matches_hybridanalysis_category(
    category: &ScanCategory,
    scan_status: Option<&crate::calc_hybridanalysis_stt::ScanStatus>,
    tag_ignorelist: &str,
) -> bool {
    use crate::calc_hybridanalysis_stt::ScanStatus;

    match scan_status {
        Some(ScanStatus::Completed(result)) => {
            let has_non_scan = result.file_results.iter().any(|fr| {
                fr.verdict == "404 Not Found"
                    || fr.verdict.is_empty()
                    || fr.verdict == "upload_error"
                    || fr.verdict == "analysis_error"
            });

            if has_non_scan {
                matches!(category, ScanCategory::HybridAnalysisNotScanned)
            } else {
                let has_malicious_ignored = result
                    .file_results
                    .iter()
                    .any(|fr| fr.verdict == "malicious" && ha_all_tags_ignored(fr, tag_ignorelist));
                let has_malicious_normal = result.file_results.iter().any(|fr| {
                    fr.verdict == "malicious" && !ha_all_tags_ignored(fr, tag_ignorelist)
                });

                match category {
                    ScanCategory::HybridAnalysisMalicious => has_malicious_normal,
                    ScanCategory::HybridAnalysisMaliciousIgnored => {
                        has_malicious_ignored && !has_malicious_normal
                    }
                    ScanCategory::HybridAnalysisSuspicious => result
                        .file_results
                        .iter()
                        .any(|f| f.verdict.to_lowercase().contains("suspicious")),
                    ScanCategory::HybridAnalysisSafe => result.file_results.iter().all(|f| {
                        !f.verdict.to_lowercase().contains("malicious")
                            && !f.verdict.to_lowercase().contains("suspicious")
                    }),
                    ScanCategory::HybridAnalysisNotScanned => false,
                    _ => false,
                }
            }
        }
        _ => matches!(category, ScanCategory::HybridAnalysisNotScanned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc_hybridanalysis_stt::{
        CalcHybridAnalysis, FileScanResult as HaFileScanResult, ScanStatus as HaScanStatus,
    };
    use crate::calc_virustotal_stt::{
        CalcVirustotal, FileScanResult as VtFileScanResult, ScanStatus as VtScanStatus,
    };

    fn vt_file(
        malicious: i32,
        suspicious: i32,
        not_found: bool,
        skipped: bool,
    ) -> VtFileScanResult {
        VtFileScanResult {
            file_path: "base.apk".to_string(),
            sha256: "abc".to_string(),
            malicious,
            suspicious,
            undetected: 0,
            harmless: 0,
            dex_count: None,
            reputation: 0,
            vt_link: "https://virustotal.com/x".to_string(),
            not_found,
            skipped,
            error: None,
        }
    }

    fn ha_file(verdict: &str, tags: Vec<&str>) -> HaFileScanResult {
        HaFileScanResult {
            file_path: "base.apk".to_string(),
            sha256: "abc".to_string(),
            verdict: verdict.to_string(),
            threat_score: None,
            threat_level: None,
            classification_tags: tags.into_iter().map(|s| s.to_string()).collect(),
            total_signatures: None,
            ha_link: "https://hybrid-analysis.com/x".to_string(),
            wait_until: None,
            job_id: None,
            error_message: None,
        }
    }

    #[test]
    fn test_vt_not_scanned_when_no_status() {
        assert!(matches_virustotal_category(
            &ScanCategory::VirusTotalNotScanned,
            None
        ));
        assert!(!matches_virustotal_category(
            &ScanCategory::VirusTotalMalicious,
            None
        ));
    }

    #[test]
    fn test_vt_not_scanned_when_pending() {
        let status = VtScanStatus::Pending;
        assert!(matches_virustotal_category(
            &ScanCategory::VirusTotalNotScanned,
            Some(&status)
        ));
    }

    #[test]
    fn test_vt_malicious() {
        let status = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(1, 0, false, false)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(
            &ScanCategory::VirusTotalMalicious,
            Some(&status)
        ));
        assert!(!matches_virustotal_category(
            &ScanCategory::VirusTotalSafe,
            Some(&status)
        ));
    }

    #[test]
    fn test_vt_suspicious_only_when_no_malicious() {
        let status = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(0, 2, false, false)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(
            &ScanCategory::VirusTotalSuspicious,
            Some(&status)
        ));
    }

    #[test]
    fn test_vt_safe() {
        let status = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(0, 0, false, false)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(
            &ScanCategory::VirusTotalSafe,
            Some(&status)
        ));
    }

    #[test]
    fn test_vt_not_found_or_skipped_buckets_to_not_scanned() {
        let not_found = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(1, 0, true, false)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(
            &ScanCategory::VirusTotalNotScanned,
            Some(&not_found)
        ));
        assert!(!matches_virustotal_category(
            &ScanCategory::VirusTotalMalicious,
            Some(&not_found)
        ));

        let skipped = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(0, 0, false, true)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(
            &ScanCategory::VirusTotalNotScanned,
            Some(&skipped)
        ));
    }

    #[test]
    fn test_ha_not_scanned_when_no_status() {
        assert!(matches_hybridanalysis_category(
            &ScanCategory::HybridAnalysisNotScanned,
            None,
            ""
        ));
    }

    #[test]
    fn test_ha_malicious_without_ignored_tags() {
        let status = HaScanStatus::Completed(CalcHybridAnalysis {
            file_results: vec![ha_file("malicious", vec!["banker"])],
        });
        assert!(matches_hybridanalysis_category(
            &ScanCategory::HybridAnalysisMalicious,
            Some(&status),
            "adware"
        ));
        assert!(!matches_hybridanalysis_category(
            &ScanCategory::HybridAnalysisMaliciousIgnored,
            Some(&status),
            "adware"
        ));
    }

    #[test]
    fn test_ha_malicious_ignored_when_all_tags_in_ignorelist() {
        let status = HaScanStatus::Completed(CalcHybridAnalysis {
            file_results: vec![ha_file("malicious", vec!["adware", "Adware"])],
        });
        assert!(matches_hybridanalysis_category(
            &ScanCategory::HybridAnalysisMaliciousIgnored,
            Some(&status),
            "adware, banker"
        ));
        assert!(!matches_hybridanalysis_category(
            &ScanCategory::HybridAnalysisMalicious,
            Some(&status),
            "adware, banker"
        ));
    }

    #[test]
    fn test_ha_suspicious() {
        let status = HaScanStatus::Completed(CalcHybridAnalysis {
            file_results: vec![ha_file("suspicious", vec![])],
        });
        assert!(matches_hybridanalysis_category(
            &ScanCategory::HybridAnalysisSuspicious,
            Some(&status),
            ""
        ));
    }

    #[test]
    fn test_ha_safe() {
        let status = HaScanStatus::Completed(CalcHybridAnalysis {
            file_results: vec![ha_file("whitelisted", vec![])],
        });
        assert!(matches_hybridanalysis_category(
            &ScanCategory::HybridAnalysisSafe,
            Some(&status),
            ""
        ));
    }

    #[test]
    fn test_ha_non_scan_verdicts_bucket_to_not_scanned() {
        let status = HaScanStatus::Completed(CalcHybridAnalysis {
            file_results: vec![ha_file("404 Not Found", vec![])],
        });
        assert!(matches_hybridanalysis_category(
            &ScanCategory::HybridAnalysisNotScanned,
            Some(&status),
            ""
        ));
        assert!(!matches_hybridanalysis_category(
            &ScanCategory::HybridAnalysisSafe,
            Some(&status),
            ""
        ));
    }
}

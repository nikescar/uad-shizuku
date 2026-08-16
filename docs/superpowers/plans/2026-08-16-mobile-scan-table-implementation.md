# Mobile VirusTotal & HybridAnalysis Table Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the mobile VirusTotal/HybridAnalysis drill-down tables' dependency on `dlg_dashcounter_details.rs`'s laggy `data_table()`/`shared_store`-based rendering with a dedicated `TableBuilder`-based, ViewModel-backed module mirroring `dlg_mobile_risk`'s pattern.

**Architecture:** New module `mobile/src/dlg_mobile_scan/` (a `ScanCategory` enum covering both VT and HA categories + state + filter predicates + a 3-column virtualized table + a render entry point) is wired into `dlg_mobile_list.rs` in place of the current fallthrough to `DlgDashCounterDetails`, and the 9 VT/HA dashboard-counter click handlers in `uad_shizuku_app.rs` are rerouted the same way the stalkerware/izzyrisk ones already are. Enable/disable/uninstall reuse the existing app-wide `ctx.data_mut` temp-key convention (not `viewmodel`), matching the risk table.

**Tech Stack:** Rust, egui/eframe, `egui_extras::TableBuilder`, existing ViewModel actor system.

**Spec:** `docs/superpowers/specs/2026-08-16-mobile-scan-table-design.md`

## Global Constraints

- No file under `mobile/src/dlg_mobile_scan/` imports from `dlg_dashcounter_details.rs` or `dlg_dashcounter_details_stt.rs`.
- Scan results come from `ViewModelState::vt_scanner_state`/`ha_scanner_state` (already ViewModel-resident), **not** `shared_store` — unlike the risk table's `package_risk_scores`, this is not a workaround, it's the correct source per the spec's Goal 2.
- `installed_packages` stays sourced from `shared_store.get_installed_packages()` (spec Non-Goals — same as the risk table).
- Icons/titles come from `crate::app_metadata_renderer::prepare_app_info_for_display` (ViewModel-backed).
- Desktop `dlg_dashcounter_details.rs` rendering (the >1010px dialog) is not modified.
- No column-click sorting, no expandable UAD description drawer, no checkbox/batch-select column (spec Non-Goals).
- Report-link opening (`webbrowser::open`) stays `#[cfg(not(target_os = "android"))]`-gated, unchanged from today (spec Non-Goals).
- Every `cargo build -p mobile` (or workspace `cargo build`) and `cargo test -p mobile` must pass at the end of each task before moving to the next.

---

### Task 1: `ScanCategory` enum and module skeleton

**Files:**
- Create: `mobile/src/dlg_mobile_scan/mod.rs`
- Create: `mobile/src/dlg_mobile_scan/components/mod.rs`
- Modify: `mobile/src/lib.rs` (add module registration, next to the existing `dlg_mobile_risk` line)

**Interfaces:**
- Produces: `pub enum ScanCategory { VirusTotalMalicious, VirusTotalSuspicious, VirusTotalSafe, VirusTotalNotScanned, HybridAnalysisMalicious, HybridAnalysisMaliciousIgnored, HybridAnalysisSuspicious, HybridAnalysisSafe, HybridAnalysisNotScanned }` (derives `Debug, Clone, PartialEq, Eq`), `pub fn is_virustotal(category: &ScanCategory) -> bool`, `pub fn is_hybridanalysis(category: &ScanCategory) -> bool`, and `pub fn window_title(category: &ScanCategory, count_enabled: usize, count_total: usize) -> String`, all in `crate::dlg_mobile_scan`.

- [ ] **Step 1: Write the failing tests**

Create `mobile/src/dlg_mobile_scan/mod.rs`:

```rust
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
```

Create `mobile/src/dlg_mobile_scan/components/mod.rs`:

```rust
pub mod package_table_mobile;
```

This will fail to compile because `state`, `filter_logic`, and `view_mobile` don't exist yet — that's expected; Tasks 2-5 create them. To verify just this file's logic in isolation, the module tree needs stub files first.

- [ ] **Step 2: Create empty stub modules so the crate compiles**

Create `mobile/src/dlg_mobile_scan/state.rs`:

```rust
//! Placeholder — filled in by Task 2.
```

Create `mobile/src/dlg_mobile_scan/filter_logic.rs`:

```rust
//! Placeholder — filled in by Task 3.
```

Create `mobile/src/dlg_mobile_scan/view_mobile.rs`:

```rust
//! Placeholder — filled in by Task 5.
```

Create `mobile/src/dlg_mobile_scan/components/package_table_mobile.rs`:

```rust
//! Placeholder — filled in by Task 4.
```

- [ ] **Step 3: Register the module in `lib.rs`**

In `mobile/src/lib.rs`, find the existing block (around line 85-87):

```rust
pub mod dlg_mobile_list;
pub mod dlg_mobile_list_stt;
pub mod dlg_mobile_risk;
```

Change to:

```rust
pub mod dlg_mobile_list;
pub mod dlg_mobile_list_stt;
pub mod dlg_mobile_risk;
pub mod dlg_mobile_scan;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p mobile dlg_mobile_scan::tests -- --nocapture`
Expected: 5 tests pass (`test_scan_category_variants_are_distinct`, `test_is_virustotal`, `test_is_hybridanalysis`, `test_window_title_virustotal_malicious`, `test_window_title_hybridanalysis_malicious_ignored`).

- [ ] **Step 5: Commit**

```bash
git add mobile/src/dlg_mobile_scan/ mobile/src/lib.rs
git commit -m "feat(mobile): add ScanCategory enum and dlg_mobile_scan module skeleton"
```

---

### Task 2: `ScanTableState`

**Files:**
- Modify: `mobile/src/dlg_mobile_scan/state.rs` (replace placeholder)

**Interfaces:**
- Consumes: `crate::dlg_mobile_scan::ScanCategory` (Task 1), `crate::dlg_package_details::DlgPackageDetails::new()`, `crate::dlg_uninstall_confirm::DlgUninstallConfirm::default()`.
- Produces: `pub struct ScanTableState { pub category: Option<ScanCategory>, pub count_enabled: usize, pub count_total: usize, pub show_only_enabled: bool, pub hide_system_app: bool, pub text_filter: String, pub mobile_info_dialog: DlgPackageDetails, pub uninstall_confirm_dialog: DlgUninstallConfirm }` implementing `Default`, re-exported as `crate::dlg_mobile_scan::ScanTableState`.

- [ ] **Step 1: Write the failing test**

Replace `mobile/src/dlg_mobile_scan/state.rs`:

```rust
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
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p mobile dlg_mobile_scan::state::tests -- --nocapture`
Expected: `test_default_state` passes; crate compiles (view_mobile.rs/components still placeholders but `mod.rs` no longer references undefined items — `pub mod view_mobile;`/`pub mod filter_logic;`/`pub mod components;` just need the files to parse as valid empty-ish Rust, which the placeholders already do).

- [ ] **Step 3: Commit**

```bash
git add mobile/src/dlg_mobile_scan/state.rs
git commit -m "feat(mobile): add ScanTableState"
```

---

### Task 3: Filter predicates

**Files:**
- Modify: `mobile/src/dlg_mobile_scan/filter_logic.rs` (replace placeholder)

**Interfaces:**
- Consumes: `crate::dlg_mobile_scan::ScanCategory` (Task 1), `crate::calc_virustotal_stt::{ScanStatus as VtScanStatus, CalcVirustotal, FileScanResult as VtFileScanResult}`, `crate::calc_hybridanalysis_stt::{ScanStatus as HaScanStatus, CalcHybridAnalysis, FileScanResult as HaFileScanResult}`.
- Produces: `pub fn matches_virustotal_category(category: &ScanCategory, scan_status: Option<&VtScanStatus>) -> bool` and `pub fn matches_hybridanalysis_category(category: &ScanCategory, scan_status: Option<&HaScanStatus>, tag_ignorelist: &str) -> bool`, both pure functions (no locking — callers resolve the `Arc<Mutex<HashMap>>` lookup first). `should_show_package`/`matches_text_filter`/`get_display_name` are **not** redefined here — Task 5 imports them from `crate::dlg_mobile_risk::filter_logic`, which is already category-agnostic.

- [ ] **Step 1: Write the failing tests**

Replace `mobile/src/dlg_mobile_scan/filter_logic.rs`:

```rust
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

    fn vt_file(malicious: i32, suspicious: i32, not_found: bool, skipped: bool) -> VtFileScanResult {
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
        assert!(matches_virustotal_category(&ScanCategory::VirusTotalNotScanned, None));
        assert!(!matches_virustotal_category(&ScanCategory::VirusTotalMalicious, None));
    }

    #[test]
    fn test_vt_not_scanned_when_pending() {
        let status = VtScanStatus::Pending;
        assert!(matches_virustotal_category(&ScanCategory::VirusTotalNotScanned, Some(&status)));
    }

    #[test]
    fn test_vt_malicious() {
        let status = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(1, 0, false, false)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(&ScanCategory::VirusTotalMalicious, Some(&status)));
        assert!(!matches_virustotal_category(&ScanCategory::VirusTotalSafe, Some(&status)));
    }

    #[test]
    fn test_vt_suspicious_only_when_no_malicious() {
        let status = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(0, 2, false, false)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(&ScanCategory::VirusTotalSuspicious, Some(&status)));
    }

    #[test]
    fn test_vt_safe() {
        let status = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(0, 0, false, false)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(&ScanCategory::VirusTotalSafe, Some(&status)));
    }

    #[test]
    fn test_vt_not_found_or_skipped_buckets_to_not_scanned() {
        let not_found = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(1, 0, true, false)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(&ScanCategory::VirusTotalNotScanned, Some(&not_found)));
        assert!(!matches_virustotal_category(&ScanCategory::VirusTotalMalicious, Some(&not_found)));

        let skipped = VtScanStatus::Completed(CalcVirustotal {
            file_results: vec![vt_file(0, 0, false, true)],
            files_attempted: 1,
            files_skipped_invalid_hash: 0,
        });
        assert!(matches_virustotal_category(&ScanCategory::VirusTotalNotScanned, Some(&skipped)));
    }

    #[test]
    fn test_ha_not_scanned_when_no_status() {
        assert!(matches_hybridanalysis_category(&ScanCategory::HybridAnalysisNotScanned, None, ""));
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
        assert!(matches_hybridanalysis_category(&ScanCategory::HybridAnalysisSuspicious, Some(&status), ""));
    }

    #[test]
    fn test_ha_safe() {
        let status = HaScanStatus::Completed(CalcHybridAnalysis {
            file_results: vec![ha_file("whitelisted", vec![])],
        });
        assert!(matches_hybridanalysis_category(&ScanCategory::HybridAnalysisSafe, Some(&status), ""));
    }

    #[test]
    fn test_ha_non_scan_verdicts_bucket_to_not_scanned() {
        let status = HaScanStatus::Completed(CalcHybridAnalysis {
            file_results: vec![ha_file("404 Not Found", vec![])],
        });
        assert!(matches_hybridanalysis_category(&ScanCategory::HybridAnalysisNotScanned, Some(&status), ""));
        assert!(!matches_hybridanalysis_category(&ScanCategory::HybridAnalysisSafe, Some(&status), ""));
    }
}
```

- [ ] **Step 2: Run the tests and verify they fail first (TDD check), then pass**

Run: `cargo test -p mobile dlg_mobile_scan::filter_logic::tests -- --nocapture`

Since this is written as a single step with both test and implementation already present above (the predicates are pure ports of known-correct existing logic, not exploratory), run it once implementation is in place:

Expected: all 12 tests pass (`test_vt_not_scanned_when_no_status`, `test_vt_not_scanned_when_pending`, `test_vt_malicious`, `test_vt_suspicious_only_when_no_malicious`, `test_vt_safe`, `test_vt_not_found_or_skipped_buckets_to_not_scanned`, `test_ha_not_scanned_when_no_status`, `test_ha_malicious_without_ignored_tags`, `test_ha_malicious_ignored_when_all_tags_in_ignorelist`, `test_ha_suspicious`, `test_ha_safe`, `test_ha_non_scan_verdicts_bucket_to_not_scanned`).

If any fail, compare the failing predicate against `dlg_dashcounter_details.rs:1772-1821` (VT) or `:1992-2066` (HA) line-by-line — the ported logic must match exactly.

- [ ] **Step 3: Commit**

```bash
git add mobile/src/dlg_mobile_scan/filter_logic.rs
git commit -m "feat(mobile): add VT/HA category-matching predicates"
```

---

### Task 4: Mobile scan table component (3-column `TableBuilder`)

**Files:**
- Modify: `mobile/src/dlg_mobile_scan/components/package_table_mobile.rs` (replace placeholder)

**Interfaces:**
- Consumes: `crate::dlg_mobile_scan::{is_virustotal, ScanCategory}` (Task 1), `crate::adb_stt::PackageFingerprint`, `crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_TOGGLE_OFF, ICON_TOGGLE_ON}`, `egui_material3::icon_button_standard`, `crate::tab_debloat::components::package_table_mobile::AppDisplayData` (`HashMap<String, (Option<egui::TextureHandle>, String)>`), `crate::uad_shizuku_app::UadNgLists`, `crate::calc_virustotal_stt::ScannerState` (`Arc<Mutex<HashMap<String, ScanStatus>>>`), `crate::calc_hybridanalysis_stt::ScannerState`, `egui_i18n::tr`.
- Produces: `pub fn render_scan_table_mobile(...)` (signature below), called by `view_mobile.rs` (Task 5).

- [ ] **Step 1: Write the implementation**

Replace `mobile/src/dlg_mobile_scan/components/package_table_mobile.rs`:

```rust
//! Mobile-optimized scan table component (VirusTotal / HybridAnalysis).
//!
//! 3 columns: Name/Status + Scan Result (chips) + Tasks. Modeled on
//! `dlg_mobile_risk::components::package_table_mobile`, with an extra Scan Result column since
//! VT/HA results are interactive colored chips (click-to-open-report, hover tooltip, multiple
//! per row for multi-file packages) that don't compress into a text line.

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use egui_i18n::tr;

use crate::adb_stt::PackageFingerprint;
use crate::calc_hybridanalysis_stt::ScannerState as HaScannerState;
use crate::calc_virustotal_stt::ScannerState as VtScannerState;
use crate::dlg_mobile_scan::{is_virustotal, ScanCategory};
use crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::tab_debloat::components::package_table_mobile::AppDisplayData;
use crate::uad_shizuku_app::UadNgLists;
use egui_material3::icon_button_standard;

const ROW_HEIGHT: f32 = 56.0;
const SCAN_RESULT_COLUMN_WIDTH: f32 = 260.0;
const TASKS_COLUMN_WIDTH: f32 = 200.0;
const MOBILE_BUTTON_SPACING: f32 = 16.0;
const MOBILE_TOUCH_TARGET: f32 = 40.0;

fn is_row_enabled(package: &PackageFingerprint) -> bool {
    package.users.first().map_or(false, |user| {
        let enabled = user.enabled;
        let installed = user.installed;
        let is_system = package.flags.contains("SYSTEM");
        !(enabled == 0 && !installed && is_system || enabled == 2 || enabled == 3)
    })
}

/// Whether the delete/uninstall button should show for this package, gated by the
/// Unsafe/Expert removal toggles — mirrors `render_action_buttons_static`'s gating
/// (dlg_dashcounter_details.rs:806-808).
fn show_delete_button(
    pkg_id: &str,
    uad_ng_lists: Option<&UadNgLists>,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
) -> bool {
    let category = uad_ng_lists
        .and_then(|lists| lists.apps.get(pkg_id))
        .map(|app| app.removal.as_str());

    match category {
        Some("Unsafe") => unsafe_app_remove,
        Some("Expert") => expert_app_remove,
        _ => true,
    }
}

/// Get HybridAnalysis display text (ported from `DlgDashCounterDetails::get_ha_display_text`,
/// dlg_dashcounter_details.rs:376-497).
fn get_ha_display_text(file_result: &crate::calc_hybridanalysis_stt::FileScanResult) -> String {
    if file_result.verdict == "upload_error" || file_result.verdict == "analysis_error" {
        if let Some(ref error_msg) = file_result.error_message {
            if error_msg.contains("File too large") {
                if let Some(mb_pos) = error_msg.find(" MB ") {
                    if let Some(start) =
                        error_msg[..mb_pos].rfind(|c: char| !c.is_numeric() && c != '.')
                    {
                        let size = &error_msg[start + 1..mb_pos + 3];
                        return tr!("ha-file-too-large", { size: size.to_string() });
                    } else {
                        return tr!("ha-file-too-large-default");
                    }
                } else {
                    return tr!("ha-file-too-large-default");
                }
            } else if error_msg.contains("No such file or directory") {
                return tr!("ha-pull-failed");
            } else if error_msg.contains("Failed to create tmp directory") {
                return tr!("ha-temp-dir-error");
            } else if file_result.verdict == "upload_error" {
                return tr!("ha-upload-error");
            } else {
                return tr!("ha-analysis-error");
            }
        } else if file_result.verdict == "upload_error" {
            return tr!("ha-upload-error");
        } else {
            return tr!("ha-analysis-error");
        }
    }

    let has_tags = !file_result.classification_tags.is_empty();
    let base_text = if has_tags {
        let tags_str = file_result.classification_tags.join(", ");
        match file_result.verdict.as_str() {
            "malicious" => tr!("ha-malicious-tags", { tags: tags_str }),
            "suspicious" => tr!("ha-suspicious-tags", { tags: tags_str }),
            "whitelisted" => tr!("ha-whitelisted-tags", { tags: tags_str }),
            "no specific threat" => tr!("ha-no-specific-threat-tags", { tags: tags_str }),
            _ => match file_result.verdict.as_str() {
                "no-result" => tr!("ha-no-result"),
                "rate_limited" => tr!("ha-rate-limited"),
                "submitted" => tr!("ha-submitted"),
                "pending_analysis" => tr!("ha-pending-analysis"),
                "404 Not Found" => tr!("ha-404"),
                "" => tr!("ha-skipped"),
                _ => file_result.verdict.clone(),
            },
        }
    } else if let Some(score) = file_result.threat_score {
        match file_result.verdict.as_str() {
            "malicious" => tr!("ha-malicious-score", { score: score }),
            "suspicious" => tr!("ha-suspicious-score", { score: score }),
            "whitelisted" => tr!("ha-whitelisted-score", { score: score }),
            "no specific threat" => tr!("ha-no-specific-threat-score", { score: score }),
            _ => match file_result.verdict.as_str() {
                "no-result" => tr!("ha-no-result"),
                "rate_limited" => tr!("ha-rate-limited"),
                "submitted" => tr!("ha-submitted"),
                "pending_analysis" => tr!("ha-pending-analysis"),
                "404 Not Found" => tr!("ha-404"),
                "" => tr!("ha-skipped"),
                _ => file_result.verdict.clone(),
            },
        }
    } else {
        match file_result.verdict.as_str() {
            "malicious" => tr!("ha-malicious"),
            "suspicious" => tr!("ha-suspicious"),
            "whitelisted" => tr!("ha-whitelisted"),
            "no specific threat" => tr!("ha-no-specific-threat"),
            "no-result" => tr!("ha-no-result"),
            "rate_limited" => tr!("ha-rate-limited"),
            "submitted" => tr!("ha-submitted"),
            "pending_analysis" => {
                if let Some(ref job_id) = file_result.job_id {
                    let short_id = if job_id.len() > 8 { &job_id[..8] } else { job_id };
                    tr!("ha-pending", { jobid: short_id.to_string() })
                } else {
                    tr!("ha-pending-analysis")
                }
            }
            "404 Not Found" => tr!("ha-404"),
            "" => tr!("ha-skipped"),
            _ => file_result.verdict.clone(),
        }
    };

    if let Some(wait_until) = file_result.wait_until {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if wait_until > now {
            let remaining_secs = wait_until - now;
            let hours = remaining_secs / 3600;
            let mins = (remaining_secs % 3600) / 60;
            if hours > 0 {
                tr!("ha-wait-hours", { text: base_text, hours: hours, mins: mins })
            } else if mins > 0 {
                tr!("ha-wait-mins", { text: base_text, mins: mins })
            } else {
                tr!("ha-wait-less-than-min", { text: base_text })
            }
        } else {
            base_text
        }
    } else {
        base_text
    }
}

/// Renders the VirusTotal chip row for one package. Ported from
/// `DlgDashCounterDetails::render_vt_cell` (dlg_dashcounter_details.rs:190-265), converted from
/// a `DataTableCell::widget` closure to a direct `ui` call (TableBuilder's `row.col` already
/// hands the cell its own `ui`).
fn render_vt_chips(
    ui: &mut egui::Ui,
    vt_result: Option<&crate::calc_virustotal_stt::ScanStatus>,
    idx: usize,
) {
    egui::ScrollArea::horizontal()
        .id_salt(format!("vt_scroll_mobile_{}", idx))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                match vt_result {
                    None => {
                        ui.label(tr!("scan-not-initialized"));
                    }
                    Some(crate::calc_virustotal_stt::ScanStatus::Pending) => {
                        ui.label(tr!("scan-not-scanned"));
                    }
                    Some(crate::calc_virustotal_stt::ScanStatus::Scanning { scanned, total, .. }) => {
                        ui.label(tr!("scan-scanning", { scanned: scanned, total: total }));
                    }
                    Some(crate::calc_virustotal_stt::ScanStatus::Completed(result)) => {
                        for (i, file_result) in result.file_results.iter().enumerate() {
                            let (text, bg_color) = if file_result.error.is_some() {
                                (tr!("scan-error"), egui::Color32::from_rgb(211, 47, 47))
                            } else if file_result.skipped {
                                (tr!("scan-skip"), egui::Color32::from_rgb(128, 128, 128))
                            } else if file_result.not_found {
                                (tr!("scan-404"), egui::Color32::from_rgb(128, 128, 128))
                            } else if file_result.malicious > 0 {
                                (
                                    tr!("scan-malicious", { count: file_result.malicious + file_result.suspicious, total: file_result.total() }),
                                    egui::Color32::from_rgb(211, 47, 47),
                                )
                            } else if file_result.suspicious > 0 {
                                (
                                    tr!("scan-suspicious", { count: file_result.suspicious, total: file_result.total() }),
                                    egui::Color32::from_rgb(255, 152, 0),
                                )
                            } else {
                                (
                                    tr!("scan-clean", { count: file_result.total(), total: file_result.total() }),
                                    egui::Color32::from_rgb(56, 142, 60),
                                )
                            };

                            let inner_response = egui::Frame::new()
                                .fill(bg_color)
                                .corner_radius(8.0)
                                .inner_margin(egui::Margin::symmetric(12, 6))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(12.0))
                                });

                            let response = ui.interact(
                                inner_response.response.rect,
                                ui.id().with(format!("vt_chip_mobile_{}_{}", idx, i)),
                                egui::Sense::click(),
                            );

                            if let Some(ref err) = file_result.error {
                                response.on_hover_text(format!("{}\n{}", file_result.file_path, err));
                            } else {
                                if response.clicked() {
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        if let Err(err) = webbrowser::open(&file_result.vt_link) {
                                            log::error!("Failed to open VirusTotal link: {}", err);
                                        }
                                    }
                                }
                                response.on_hover_text(&file_result.file_path);
                            }
                        }
                    }
                    Some(crate::calc_virustotal_stt::ScanStatus::Error(e)) => {
                        ui.label(tr!("scan-error-msg", { message: e.clone() }));
                    }
                }
            });
        });
}

/// Renders the HybridAnalysis chip row for one package. Ported from
/// `DlgDashCounterDetails::render_ha_cell` (dlg_dashcounter_details.rs:268-373).
fn render_ha_chips(
    ui: &mut egui::Ui,
    ha_result: Option<&crate::calc_hybridanalysis_stt::ScanStatus>,
    idx: usize,
    ha_tag_ignorelist: &str,
) {
    egui::ScrollArea::horizontal()
        .id_salt(format!("ha_scroll_mobile_{}", idx))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                match ha_result {
                    None => {
                        ui.label(tr!("scan-not-initialized"));
                    }
                    Some(crate::calc_hybridanalysis_stt::ScanStatus::Pending) => {
                        ui.label(tr!("scan-not-scanned"));
                    }
                    Some(crate::calc_hybridanalysis_stt::ScanStatus::Scanning { scanned, total, .. }) => {
                        ui.label(tr!("scan-scanning", { scanned: scanned, total: total }));
                    }
                    Some(crate::calc_hybridanalysis_stt::ScanStatus::Completed(result)) => {
                        if result.file_results.is_empty() {
                            ui.label(tr!("scan-no-results"));
                        }
                        for (i, file_result) in result.file_results.iter().enumerate() {
                            let text = get_ha_display_text(file_result);

                            let ignorelist_tags: Vec<String> = ha_tag_ignorelist
                                .split(',')
                                .map(|s| s.trim().to_lowercase())
                                .filter(|s| !s.is_empty())
                                .collect();

                            let all_tags_ignored = if file_result.classification_tags.is_empty() {
                                true
                            } else {
                                file_result
                                    .classification_tags
                                    .iter()
                                    .all(|tag| ignorelist_tags.contains(&tag.to_lowercase()))
                            };

                            let bg_color = match file_result.verdict.as_str() {
                                "malicious" => {
                                    if all_tags_ignored {
                                        egui::Color32::from_rgb(128, 128, 128)
                                    } else {
                                        egui::Color32::from_rgb(211, 47, 47)
                                    }
                                }
                                "suspicious" => egui::Color32::from_rgb(255, 152, 0),
                                "whitelisted" => egui::Color32::from_rgb(56, 142, 60),
                                "no specific threat" => egui::Color32::from_rgb(0, 150, 136),
                                "no-result" => egui::Color32::from_rgb(158, 158, 158),
                                "rate_limited" => egui::Color32::from_rgb(156, 39, 176),
                                "submitted" => egui::Color32::from_rgb(33, 150, 243),
                                "pending_analysis" => egui::Color32::from_rgb(33, 150, 243),
                                "upload_error" | "analysis_error" => egui::Color32::from_rgb(211, 47, 47),
                                "404 Not Found" => egui::Color32::from_rgb(158, 158, 158),
                                "" => egui::Color32::from_rgb(158, 158, 158),
                                _ => egui::Color32::from_rgb(158, 158, 158),
                            };

                            let inner_response = egui::Frame::new()
                                .fill(bg_color)
                                .corner_radius(8.0)
                                .inner_margin(egui::Margin::symmetric(12, 6))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(12.0))
                                });

                            let response = ui.interact(
                                inner_response.response.rect,
                                ui.id().with(format!("ha_chip_mobile_{}_{}", idx, i)),
                                egui::Sense::click(),
                            );

                            if let Some(ref error_msg) = file_result.error_message {
                                response.on_hover_text(format!("{}\n{}", file_result.file_path, error_msg));
                            } else {
                                if response.clicked() {
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        if !file_result.ha_link.is_empty() {
                                            if let Err(err) = webbrowser::open(&file_result.ha_link) {
                                                log::error!("Failed to open HybridAnalysis link: {}", err);
                                            }
                                        }
                                    }
                                }
                                response.on_hover_text(&file_result.file_path);
                            }
                        }
                    }
                    Some(crate::calc_hybridanalysis_stt::ScanStatus::Error(e)) => {
                        ui.label(tr!("scan-error-msg", { message: e.clone() }));
                    }
                }
            });
        });
}

#[allow(clippy::too_many_arguments)]
pub fn render_scan_table_mobile(
    ui: &mut egui::Ui,
    packages: &[&PackageFingerprint],
    category: &ScanCategory,
    vt_scanner_state: Option<&VtScannerState>,
    ha_scanner_state: Option<&HaScannerState>,
    hybridanalysis_tag_ignorelist: &str,
    uad_ng_lists: Option<&UadNgLists>,
    app_display_data: &AppDisplayData,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
    on_info_clicked: &mut dyn FnMut(&str),
    on_toggle_clicked: &mut dyn FnMut(&str, bool),
    on_delete_clicked: &mut dyn FnMut(&str),
) {
    let show_vt = is_virustotal(category);
    let header_label = if show_vt {
        tr!("col-virustotal")
    } else {
        tr!("col-hybrid-analysis")
    };

    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::remainder())
        .column(Column::exact(SCAN_RESULT_COLUMN_WIDTH))
        .column(Column::exact(TASKS_COLUMN_WIDTH))
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.label("Name");
            });
            header.col(|ui| {
                ui.label(header_label);
            });
            header.col(|ui| {
                ui.label("Tasks");
            });
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, packages.len(), |mut row| {
                let package = packages[row.index()];

                // Column 1: Name/Status
                row.col(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let (texture_handle, app_title) = app_display_data
                            .get(&package.pkg)
                            .map(|(tex, title)| (tex.as_ref(), Some(title.as_str())))
                            .unwrap_or((None, None));

                        if let Some(tex) = texture_handle {
                            ui.image((tex.id(), egui::vec2(38.0, 38.0)));
                        }

                        ui.vertical(|ui| {
                            ui.style_mut().spacing.item_spacing.y = 2.0;

                            if let Some(title) = app_title {
                                let text_color = ui.style().visuals.text_color();
                                ui.label(egui::RichText::new(title).strong().color(text_color));
                                ui.label(egui::RichText::new(&package.pkg).small().weak());
                            } else {
                                ui.label(&package.pkg);
                            }

                            let (status_text, status_color) = if package.users.is_empty() {
                                ("Uninstalled", egui::Color32::from_rgb(128, 128, 128))
                            } else {
                                let user = &package.users[0];
                                let is_system = package.flags.contains("SYSTEM");
                                if user.enabled == 0 && !user.installed && is_system {
                                    ("Removed", egui::Color32::from_rgb(158, 158, 158))
                                } else if user.enabled == 2 {
                                    ("Disabled", egui::Color32::from_rgb(211, 47, 47))
                                } else if user.enabled == 3 {
                                    ("Disabled-User", egui::Color32::from_rgb(244, 67, 54))
                                } else {
                                    ("Enabled", egui::Color32::from_rgb(56, 142, 60))
                                }
                            };
                            ui.label(egui::RichText::new(status_text).color(status_color));
                        });
                    });
                });

                // Column 2: Scan Result (VT or HA chips)
                row.col(|ui| {
                    if show_vt {
                        let vt_status = vt_scanner_state
                            .and_then(|state| state.lock().ok())
                            .and_then(|locked| locked.get(&package.pkg).cloned());
                        render_vt_chips(ui, vt_status.as_ref(), row.index());
                    } else {
                        let ha_status = ha_scanner_state
                            .and_then(|state| state.lock().ok())
                            .and_then(|locked| locked.get(&package.pkg).cloned());
                        render_ha_chips(ui, ha_status.as_ref(), row.index(), hybridanalysis_tag_ignorelist);
                    }
                });

                // Column 3: Tasks (info / toggle / uninstall)
                row.col(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = MOBILE_BUTTON_SPACING;
                        ui.style_mut().spacing.interact_size =
                            egui::vec2(MOBILE_TOUCH_TARGET, MOBILE_TOUCH_TARGET);

                        if ui
                            .add(icon_button_standard(ICON_INFO.to_string()))
                            .on_hover_text("Package details")
                            .clicked()
                        {
                            on_info_clicked(&package.pkg);
                        }

                        let is_enabled = is_row_enabled(package);
                        let toggle_icon = if is_enabled { ICON_TOGGLE_ON } else { ICON_TOGGLE_OFF };
                        let toggle_text = if is_enabled { "Disable" } else { "Enable" };

                        if ui
                            .add(icon_button_standard(toggle_icon.to_string()))
                            .on_hover_text(toggle_text)
                            .clicked()
                        {
                            on_toggle_clicked(&package.pkg, is_enabled);
                        }

                        if show_delete_button(&package.pkg, uad_ng_lists, unsafe_app_remove, expert_app_remove) {
                            if ui
                                .add(icon_button_standard(ICON_DELETE.to_string()))
                                .on_hover_text("Uninstall package")
                                .clicked()
                            {
                                on_delete_clicked(&package.pkg);
                            }
                        }
                    });
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(ROW_HEIGHT, 56.0);
        assert_eq!(SCAN_RESULT_COLUMN_WIDTH, 260.0);
        assert_eq!(TASKS_COLUMN_WIDTH, 200.0);
        assert_eq!(MOBILE_BUTTON_SPACING, 16.0);
        assert_eq!(MOBILE_TOUCH_TARGET, 40.0);
    }

    #[test]
    fn test_show_delete_button_defaults_true_with_no_uad_lists() {
        assert!(show_delete_button("com.example.app", None, false, false));
        assert!(show_delete_button("com.example.app", None, true, true));
    }
}
```

Note: `FileScanResult::total()` (used in `render_vt_chips`) must already exist on `crate::calc_virustotal_stt::FileScanResult` — it's called the same way in the current `render_vt_cell` (dlg_dashcounter_details.rs:222,226). If `cargo build` reports it missing, check `calc_virustotal_stt.rs` for a method named `total` on `FileScanResult` (likely `malicious + suspicious + undetected + harmless`) and confirm the import path; it is not being redefined by this task, only reused.

- [ ] **Step 2: Build and run the tests**

Run: `cargo build -p mobile`
Expected: compiles clean (fix any import path mismatches against what Tasks 1-3 actually produced).

Run: `cargo test -p mobile dlg_mobile_scan::components::package_table_mobile::tests -- --nocapture`
Expected: `test_constants` and `test_show_delete_button_defaults_true_with_no_uad_lists` pass.

- [ ] **Step 3: Commit**

```bash
git add mobile/src/dlg_mobile_scan/components/package_table_mobile.rs
git commit -m "feat(mobile): add 3-column VT/HA scan table component"
```

---

### Task 5: Render entry point (`view_mobile.rs`)

**Files:**
- Modify: `mobile/src/dlg_mobile_scan/view_mobile.rs` (replace placeholder)

**Interfaces:**
- Consumes: `crate::dlg_mobile_scan::{ScanCategory, ScanTableState}` (Tasks 1-2), `crate::dlg_mobile_scan::filter_logic::{matches_virustotal_category, matches_hybridanalysis_category}` (Task 3), `crate::dlg_mobile_scan::components::package_table_mobile::render_scan_table_mobile` (Task 4), `crate::dlg_mobile_risk::filter_logic::{should_show_package, matches_text_filter}` (existing, reused), `crate::app_metadata_renderer::prepare_app_info_for_display`, `crate::viewmodel::ViewModelState`.
- Produces: `pub fn render(ui, ctx, vm_state, local_state: &mut ScanTableState, installed_packages, hybridanalysis_tag_ignorelist: &str, unsafe_app_remove, expert_app_remove, google_play_enabled, fdroid_enabled, apkmirror_enabled, android_package_enabled)`, called by `dlg_mobile_list.rs` (Task 6).

- [ ] **Step 1: Write the implementation**

Replace `mobile/src/dlg_mobile_scan/view_mobile.rs`:

```rust
//! Render entry point for the mobile VirusTotal/HybridAnalysis drill-down.
//!
//! Mirrors `dlg_mobile_risk::view_mobile`'s shape: filter row, then a virtualized table, then
//! the dialogs it owns. Filtering is local/synchronous, same as the risk table.

use eframe::egui;
use std::collections::HashSet;

use super::components::package_table_mobile::render_scan_table_mobile;
use super::filter_logic::{matches_hybridanalysis_category, matches_virustotal_category};
use super::state::ScanTableState;
use crate::adb::PackageFingerprint;
use crate::dlg_mobile_risk::filter_logic::{matches_text_filter, should_show_package};
use crate::viewmodel::ViewModelState;

#[allow(clippy::too_many_arguments)]
pub fn render(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    vm_state: &ViewModelState,
    local_state: &mut ScanTableState,
    installed_packages: &[PackageFingerprint],
    hybridanalysis_tag_ignorelist: &str,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) {
    let Some(category) = local_state.category.clone() else {
        ui.label("No category selected");
        return;
    };

    render_filter_row(ui, local_state);
    ui.add_space(8.0);

    let is_vt = super::is_virustotal(&category);

    let filtered_packages: Vec<&PackageFingerprint> = installed_packages
        .iter()
        .filter(|pkg| {
            let matches_category = if is_vt {
                let vt_status = vm_state
                    .vt_scanner_state
                    .as_ref()
                    .and_then(|state| state.lock().ok())
                    .and_then(|locked| locked.get(&pkg.pkg).cloned());
                matches_virustotal_category(&category, vt_status.as_ref())
            } else {
                let ha_status = vm_state
                    .ha_scanner_state
                    .as_ref()
                    .and_then(|state| state.lock().ok())
                    .and_then(|locked| locked.get(&pkg.pkg).cloned());
                matches_hybridanalysis_category(&category, ha_status.as_ref(), hybridanalysis_tag_ignorelist)
            };

            matches_category
                && should_show_package(pkg, local_state.show_only_enabled, local_state.hide_system_app)
                && matches_text_filter(&local_state.text_filter, pkg, vm_state)
        })
        .collect();

    let package_ids: Vec<String> = filtered_packages.iter().map(|p| p.pkg.clone()).collect();
    let system_packages: HashSet<String> = installed_packages
        .iter()
        .filter(|p| p.flags.contains("SYSTEM"))
        .map(|p| p.pkg.clone())
        .collect();

    let full_metadata = crate::app_metadata_renderer::prepare_app_info_for_display(
        ctx,
        &package_ids,
        &system_packages,
        vm_state,
        google_play_enabled,
        fdroid_enabled,
        apkmirror_enabled,
        android_package_enabled,
    );
    let app_metadata: crate::tab_debloat::components::package_table_mobile::AppDisplayData =
        full_metadata
            .iter()
            .map(|(pkg_id, (texture, title, _developer, _version))| {
                (pkg_id.clone(), (texture.clone(), title.clone()))
            })
            .collect();

    egui::ScrollArea::vertical()
        .id_salt("scan_table_mobile_scroll")
        .show(ui, |ui| {
            render_scan_table_mobile(
                ui,
                &filtered_packages,
                &category,
                vm_state.vt_scanner_state.as_ref(),
                vm_state.ha_scanner_state.as_ref(),
                hybridanalysis_tag_ignorelist,
                vm_state.uad_ng_lists.as_ref(),
                &app_metadata,
                unsafe_app_remove,
                expert_app_remove,
                &mut |pkg_id| {
                    if let Some(idx) = installed_packages.iter().position(|p| p.pkg == pkg_id) {
                        local_state.mobile_info_dialog.open(idx);
                    }
                },
                &mut |pkg_id, is_enabled| {
                    let key = if is_enabled {
                        "disable_clicked_package"
                    } else {
                        "enable_clicked_package"
                    };
                    ctx.data_mut(|data| {
                        data.insert_temp(egui::Id::new(key), pkg_id.to_string());
                    });
                },
                &mut |pkg_id| {
                    if let Some(package) = installed_packages.iter().find(|p| p.pkg == pkg_id) {
                        let is_system = package.flags.contains("SYSTEM");
                        local_state
                            .uninstall_confirm_dialog
                            .open_single(pkg_id.to_string(), is_system);
                    }
                },
            );
        });

    local_state
        .mobile_info_dialog
        .show(ctx, installed_packages, &vm_state.uad_ng_lists);

    if local_state.uninstall_confirm_dialog.show(ctx) {
        if let (Some(pkg_id), Some(&is_system)) = (
            local_state.uninstall_confirm_dialog.packages.first().cloned(),
            local_state.uninstall_confirm_dialog.is_system.first(),
        ) {
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("uninstall_clicked_package"), pkg_id);
                data.insert_temp(egui::Id::new("uninstall_clicked_is_system"), is_system);
            });
        }
    }
}

fn render_filter_row(ui: &mut egui::Ui, local_state: &mut ScanTableState) {
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut local_state.show_only_enabled, "Show only enabled");
        ui.add_space(10.0);
        ui.checkbox(&mut local_state.hide_system_app, "Hide system apps");
        ui.add_space(10.0);
        ui.label("Filter:");
        ui.add(
            egui::TextEdit::singleline(&mut local_state.text_filter)
                .hint_text("Search...")
                .desired_width(200.0),
        );
        if !local_state.text_filter.is_empty() && ui.button("X").clicked() {
            local_state.text_filter.clear();
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_view_mobile_module_compiles() {
        assert!(true);
    }
}
```

- [ ] **Step 2: Build and run tests**

Run: `cargo build -p mobile`
Expected: compiles clean. If `matches_text_filter`/`should_show_package` aren't `pub` on `dlg_mobile_risk::filter_logic`, check their visibility — they must be `pub fn` (not `pub(crate)`) for this cross-module import to work; `dlg_mobile_risk/filter_logic.rs` already declares them `pub fn`, so no change needed there.

Run: `cargo test -p mobile dlg_mobile_scan -- --nocapture`
Expected: all tests from Tasks 1-5 pass (mod.rs: 5, state.rs: 1, filter_logic.rs: 12, components: 2, view_mobile: 1 — 21 total).

- [ ] **Step 3: Commit**

```bash
git add mobile/src/dlg_mobile_scan/view_mobile.rs
git commit -m "feat(mobile): add VT/HA mobile render entry point"
```

---

### Task 6: Wire into `dlg_mobile_list`

**Files:**
- Modify: `mobile/src/dlg_mobile_list_stt.rs`
- Modify: `mobile/src/dlg_mobile_list.rs`

**Interfaces:**
- Consumes: `crate::dlg_mobile_scan::{ScanTableState, ScanCategory, window_title}` (Tasks 1-2), `crate::dlg_mobile_scan::view_mobile::render` (Task 5).
- Produces: `MobileListViewType::VirusTotal` / `MobileListViewType::HybridAnalysis` variants; `DlgMobileList::show()` gains one new trailing parameter `hybridanalysis_tag_ignorelist: &str`.

- [ ] **Step 1: Add `scan_state` field and new `MobileListViewType` variants**

In `mobile/src/dlg_mobile_list_stt.rs`, change:

```rust
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MobileListViewType {
    /// Show debloat tab mobile view
    Debloat,
    /// Show stalkerware dashcounter drill-down (detected/undetected)
    Stalkerware,
    /// Show IzzyRisk dashcounter drill-down (safe/normal/moderate/high)
    IzzyRisk,
    // Future: Scan, Apps, Usage
}

impl Default for DlgMobileList {
    fn default() -> Self {
        Self {
            open: false,
            category_filter: None,
            view_type: MobileListViewType::Debloat,
            last_width: None,
            risk_state: crate::dlg_mobile_risk::RiskTableState::default(),
        }
    }
}
```

to:

```rust
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
```

Also update the existing test module in the same file — find:

```rust
    #[test]
    fn test_stalkerware_and_izzyrisk_view_types_are_distinct() {
        assert_ne!(
            MobileListViewType::Stalkerware,
            MobileListViewType::IzzyRisk
        );
        assert_ne!(MobileListViewType::Stalkerware, MobileListViewType::Debloat);
        assert_ne!(MobileListViewType::IzzyRisk, MobileListViewType::Debloat);
    }
```

and add a new test immediately after it (inside the same `mod tests` block):

```rust
    #[test]
    fn test_virustotal_and_hybridanalysis_view_types_are_distinct() {
        assert_ne!(
            MobileListViewType::VirusTotal,
            MobileListViewType::HybridAnalysis
        );
        assert_ne!(MobileListViewType::VirusTotal, MobileListViewType::Debloat);
        assert_ne!(MobileListViewType::HybridAnalysis, MobileListViewType::Stalkerware);
    }
```

- [ ] **Step 2: Dispatch and window title in `dlg_mobile_list.rs`**

In `mobile/src/dlg_mobile_list.rs`, change the `show()` signature — find:

```rust
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
    ) {
```

to:

```rust
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
```

Find the window-title `match`:

```rust
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
        };
```

Change to:

```rust
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
```

Find the render dispatch `match`:

```rust
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
                }
```

Change to:

```rust
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
```

- [ ] **Step 3: Build and run tests**

Run: `cargo build -p mobile`
Expected: fails at this point with a type error at `DlgMobileList::show()`'s call site in `uad_shizuku_app.rs` (missing the new argument) — that's expected; Task 7 fixes the call site.

Run: `cargo test -p mobile dlg_mobile_list_stt::tests -- --nocapture`
Expected: this specific test target still compiles and passes independently (`dlg_mobile_list_stt.rs` doesn't depend on the call site) — `test_default_state`, `test_view_type_equality`, `test_stalkerware_and_izzyrisk_view_types_are_distinct`, `test_virustotal_and_hybridanalysis_view_types_are_distinct` all pass. If `cargo test` refuses to run due to the workspace-wide build error from `uad_shizuku_app.rs`, proceed to Task 7 first, then return to confirm these tests pass as part of Task 7's Step 3.

- [ ] **Step 4: Commit**

```bash
git add mobile/src/dlg_mobile_list_stt.rs mobile/src/dlg_mobile_list.rs
git commit -m "feat(mobile): wire dlg_mobile_list to dlg_mobile_scan"
```

---

### Task 7: Reroute VT/HA dashboard-counter click handlers

**Files:**
- Modify: `mobile/src/uad_shizuku_app.rs`

**Interfaces:**
- Consumes: `crate::dlg_mobile_scan::ScanCategory` (Task 1), `MobileListViewType::VirusTotal`/`HybridAnalysis` (Task 6), `self.dlg_mobile_list.scan_state` (Task 6), `self.tab_scan_control.cached_scan_counts.{vt_counts, ha_counts}` (existing `CachedScanCounts` — `tab_scan_control_stt.rs:52-78`: `vt_counts: ((usize,usize); 5)`-shaped tuple, `ha_counts`: 6-tuple), `self.settings.hybridanalysis_tag_ignorelist: String` (existing).

- [ ] **Step 1: Add the `ScanCategory` import**

In `mobile/src/uad_shizuku_app.rs`, find (around line 30):

```rust
use crate::dlg_mobile_risk::RiskCategory;
```

Change to:

```rust
use crate::dlg_mobile_risk::RiskCategory;
use crate::dlg_mobile_scan::ScanCategory;
```

- [ ] **Step 2: Reroute the 9 VT/HA click-handler arms**

Find (around line 1388-1432):

```rust
                ("virustotal", 0) => (
                    Some(DashCounterCategory::VirusTotalMalicious),
                    cached_scan_counts.vt_counts.1 .0,
                    cached_scan_counts.vt_counts.1 .1,
                ),
                ("virustotal", 1) => (
                    Some(DashCounterCategory::VirusTotalSuspicious),
                    cached_scan_counts.vt_counts.2 .0,
                    cached_scan_counts.vt_counts.2 .1,
                ),
                ("virustotal", 2) => (
                    Some(DashCounterCategory::VirusTotalSafe),
                    cached_scan_counts.vt_counts.3 .0,
                    cached_scan_counts.vt_counts.3 .1,
                ),
                ("virustotal", 3) => (
                    Some(DashCounterCategory::VirusTotalNotScanned),
                    cached_scan_counts.vt_counts.4 .0,
                    cached_scan_counts.vt_counts.4 .1,
                ),
                ("hybridanalysis", 0) => (
                    Some(DashCounterCategory::HybridAnalysisMalicious),
                    cached_scan_counts.ha_counts.1 .0,
                    cached_scan_counts.ha_counts.1 .1,
                ),
                ("hybridanalysis", 1) => (
                    Some(DashCounterCategory::HybridAnalysisMaliciousIgnored),
                    cached_scan_counts.ha_counts.2 .0,
                    cached_scan_counts.ha_counts.2 .1,
                ),
                ("hybridanalysis", 2) => (
                    Some(DashCounterCategory::HybridAnalysisSuspicious),
                    cached_scan_counts.ha_counts.3 .0,
                    cached_scan_counts.ha_counts.3 .1,
                ),
                ("hybridanalysis", 3) => (
                    Some(DashCounterCategory::HybridAnalysisSafe),
                    cached_scan_counts.ha_counts.4 .0,
                    cached_scan_counts.ha_counts.4 .1,
                ),
                ("hybridanalysis", 4) => (
                    Some(DashCounterCategory::HybridAnalysisNotScanned),
                    cached_scan_counts.ha_counts.5 .0,
                    cached_scan_counts.ha_counts.5 .1,
                ),
```

Change to:

```rust
                ("virustotal", 0) => {
                    self.dlg_mobile_list.scan_state.category = Some(ScanCategory::VirusTotalMalicious);
                    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.vt_counts.1 .0;
                    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.vt_counts.1 .1;
                    self.dlg_mobile_list
                        .open(crate::dlg_mobile_list::MobileListViewType::VirusTotal, None);
                    (None, 0, 0)
                }
                ("virustotal", 1) => {
                    self.dlg_mobile_list.scan_state.category = Some(ScanCategory::VirusTotalSuspicious);
                    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.vt_counts.2 .0;
                    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.vt_counts.2 .1;
                    self.dlg_mobile_list
                        .open(crate::dlg_mobile_list::MobileListViewType::VirusTotal, None);
                    (None, 0, 0)
                }
                ("virustotal", 2) => {
                    self.dlg_mobile_list.scan_state.category = Some(ScanCategory::VirusTotalSafe);
                    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.vt_counts.3 .0;
                    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.vt_counts.3 .1;
                    self.dlg_mobile_list
                        .open(crate::dlg_mobile_list::MobileListViewType::VirusTotal, None);
                    (None, 0, 0)
                }
                ("virustotal", 3) => {
                    self.dlg_mobile_list.scan_state.category = Some(ScanCategory::VirusTotalNotScanned);
                    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.vt_counts.4 .0;
                    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.vt_counts.4 .1;
                    self.dlg_mobile_list
                        .open(crate::dlg_mobile_list::MobileListViewType::VirusTotal, None);
                    (None, 0, 0)
                }
                ("hybridanalysis", 0) => {
                    self.dlg_mobile_list.scan_state.category = Some(ScanCategory::HybridAnalysisMalicious);
                    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.ha_counts.1 .0;
                    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.ha_counts.1 .1;
                    self.dlg_mobile_list
                        .open(crate::dlg_mobile_list::MobileListViewType::HybridAnalysis, None);
                    (None, 0, 0)
                }
                ("hybridanalysis", 1) => {
                    self.dlg_mobile_list.scan_state.category =
                        Some(ScanCategory::HybridAnalysisMaliciousIgnored);
                    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.ha_counts.2 .0;
                    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.ha_counts.2 .1;
                    self.dlg_mobile_list
                        .open(crate::dlg_mobile_list::MobileListViewType::HybridAnalysis, None);
                    (None, 0, 0)
                }
                ("hybridanalysis", 2) => {
                    self.dlg_mobile_list.scan_state.category = Some(ScanCategory::HybridAnalysisSuspicious);
                    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.ha_counts.3 .0;
                    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.ha_counts.3 .1;
                    self.dlg_mobile_list
                        .open(crate::dlg_mobile_list::MobileListViewType::HybridAnalysis, None);
                    (None, 0, 0)
                }
                ("hybridanalysis", 3) => {
                    self.dlg_mobile_list.scan_state.category = Some(ScanCategory::HybridAnalysisSafe);
                    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.ha_counts.4 .0;
                    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.ha_counts.4 .1;
                    self.dlg_mobile_list
                        .open(crate::dlg_mobile_list::MobileListViewType::HybridAnalysis, None);
                    (None, 0, 0)
                }
                ("hybridanalysis", 4) => {
                    self.dlg_mobile_list.scan_state.category = Some(ScanCategory::HybridAnalysisNotScanned);
                    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.ha_counts.5 .0;
                    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.ha_counts.5 .1;
                    self.dlg_mobile_list
                        .open(crate::dlg_mobile_list::MobileListViewType::HybridAnalysis, None);
                    (None, 0, 0)
                }
```

- [ ] **Step 3: Pass the new `hybridanalysis_tag_ignorelist` argument to `DlgMobileList::show()`**

Find (around line 1625-1638):

```rust
            self.dlg_mobile_list.show(
                ui.ctx(),
                &viewmodel.state,
                &mut self.tab_debloat.state,
                viewmodel,
                google_play_enabled,
                fdroid_enabled,
                apkmirror_enabled,
                android_package_enabled,
                &installed_packages,
                package_risk_scores,
                self.settings.unsafe_app_remove,
                self.settings.expert_app_remove,
            );
```

Change to:

```rust
            self.dlg_mobile_list.show(
                ui.ctx(),
                &viewmodel.state,
                &mut self.tab_debloat.state,
                viewmodel,
                google_play_enabled,
                fdroid_enabled,
                apkmirror_enabled,
                android_package_enabled,
                &installed_packages,
                package_risk_scores,
                self.settings.unsafe_app_remove,
                self.settings.expert_app_remove,
                &self.settings.hybridanalysis_tag_ignorelist,
            );
```

- [ ] **Step 4: Build and run the full test suite**

Run: `cargo build -p mobile`
Expected: compiles clean, no more missing-argument or unresolved-import errors.

Run: `cargo test -p mobile`
Expected: full suite passes, including every test added in Tasks 1-6 plus all pre-existing tests (no regressions).

Run: `cargo clippy -p mobile -- -D warnings`
Expected: no new warnings from the added code (matches the `#[allow(clippy::too_many_arguments)]` already applied where needed).

- [ ] **Step 5: Commit**

```bash
git add mobile/src/uad_shizuku_app.rs
git commit -m "feat(mobile): route VT/HA dashcounter clicks through dlg_mobile_scan"
```

---

### Task 8: Manual verification

**Files:** None (verification only).

**Interfaces:** None — this task exercises the running app, not new code.

- [ ] **Step 1: Full workspace build**

Run: `cargo build` (from repo root)
Expected: entire workspace builds clean.

- [ ] **Step 2: Run the app on desktop at a narrow window width**

Run: `cargo run -p mobile`

Resize the window to ≤1010px wide (the `dlg_mobile_list.rs` mobile threshold). With a device connected (or test fixture data) that has VirusTotal and HybridAnalysis scan results for at least one package with multiple files:

- Tap each of the 4 VirusTotal dashboard counters (Malicious / Suspicious / Safe / Not Scanned) and each of the 5 HybridAnalysis counters (Malicious / Malicious Ignored / Suspicious / Safe / Not Scanned).
- For each: confirm the table renders promptly with no visible lag versus the old `data_table()` dialog; confirm the scan-result chips show correct color/text; confirm clicking a chip opens the report link in a browser (desktop only); confirm hovering a chip shows the tooltip; confirm the "Show only enabled"/"Hide system apps" checkboxes and the text filter work; confirm info/toggle/uninstall buttons work.
- Confirm the HybridAnalysis "Malicious (Ignored)" counter correctly excludes packages whose malicious verdict has at least one tag *not* in `self.settings.hybridanalysis_tag_ignorelist`.

- [ ] **Step 3: Confirm desktop path is untouched**

Widen the window above 1010px and tap a VirusTotal/HybridAnalysis counter that was previously reachable on desktop through some other path (e.g. the Scan tab's own drill-down UI, if one exists outside the dashboard counters) — confirm `dlg_dashcounter_details.rs`'s desktop rendering still works unmodified for any path that doesn't go through `dashcounter_clicked`.

- [ ] **Step 4: No commit needed**

This task is verification-only; if issues are found, fix them in the relevant task's files and re-commit there (do not create a separate "fixes" commit — amend forward with a new commit referencing the task, per the project's normal git workflow).

# Mobile VirusTotal & HybridAnalysis Table Design

**Date:** 2026-08-16
**Author:** Claude Sonnet 5
**Status:** Design Approved

## Overview

Reimplement the VirusTotal and HybridAnalysis mobile drill-down tables (reached by tapping a
dashboard counter) as a dedicated mobile module, instead of hosting
`dlg_dashcounter_details.rs`'s desktop-oriented `egui_material3::data_table()` rendering
inside `dlg_mobile_list.rs`. This mirrors the prior session's IzzyRisk/Stalkerware port (see
`docs/superpowers/specs/2026-08-16-mobile-risk-table-design.md` and `dlg_mobile_risk/`) applied
to the two remaining dashcounter categories that still fall back to the laggy desktop dialog on
mobile.

## Current State

Today, tapping a `("virustotal", 0-3)` or `("hybridanalysis", 0-4)` dashboard counter (see
`uad_shizuku_app.rs:1388-1432`) sets `self.dlg_dashcounter_details.category` to a
`DashCounterCategory::VirusTotal*`/`HybridAnalysis*` variant and calls
`self.dlg_dashcounter_details.open(cat, count_enabled, count_total)`. Unlike
`"stalkerware"`/`"izzyrisk"` (already routed through `dlg_mobile_list` last session), VT/HA
still render via `DlgDashCounterDetails::show()`'s `min_width(800.0)` window — laggy and
poorly suited to narrow viewports — regardless of actual screen width, because that dialog's
`egui_material3::data_table()` is not virtualized (see `2026-08-15-mobile-table-datatable-design.md`
for the perf comparison against `TableBuilder`) and sources icons from `shared_store` rather
than the ViewModel.

`render_virustotal_table`/`render_hybridanalysis_table` (dlg_dashcounter_details.rs:1757-2266)
render per-file colored chips (`render_vt_cell`/`render_ha_cell`, :190-373) that are clickable
(opens the VirusTotal/HybridAnalysis report link via `webbrowser::open`, non-Android only) and
show hover tooltips with file path / error text.

## Goals

1. Mobile VirusTotal/HybridAnalysis tables render via `egui_extras::TableBuilder` (virtualized),
   matching debloat's and the risk table's mobile performance characteristics.
2. Scan-result data sourced from `ViewModelState::vt_scanner_state`/`ha_scanner_state`
   (already populated there per the June 2026 MVVM migration — see CLAUDE.md §4) instead of
   `shared_store`. Icons/titles sourced from the ViewModel's `app_metadata_renderer`, matching
   debloat/IzzyRisk.
3. No file under the new module imports from `dlg_dashcounter_details.rs`.
4. Desktop VirusTotal/HybridAnalysis rendering (`dlg_dashcounter_details.rs`'s `.show()`, used
   for the >1010px dashcounter details window) is untouched.
5. Chip interactivity (color coding, click-to-open-report, hover tooltip, multiple chips per
   package for multi-file APKs) is preserved, not degraded to plain text.

## Non-Goals

- Migrating `installed_packages` (still `shared_store.get_installed_packages()`) into the
  ViewModel — same non-goal as the risk table design; a separate, larger effort.
- Changing Offa/Fmhy dashboard-counter behavior — unaffected by this change.
- Adding column-click sorting or the expandable description drawer to the mobile table (same
  rationale as the risk table: debloat's mobile table, the pattern being matched, has neither).
- Batch selection / checkbox column — neither current VT nor HA table has batch actions.
- Android report-link opening. `webbrowser::open` is already `#[cfg(not(target_os =
  "android"))]`-gated in the source being ported; this design keeps that gate as-is rather than
  adding an Android intent-based alternative.

## Architecture

### File structure

```
mobile/src/dlg_mobile_scan/
├── mod.rs                       # ScanCategory enum, module declarations, re-exports
├── state.rs                     # ScanTableState: filters + owned dialogs
├── view_mobile.rs               # render entry: filter row + table + dialogs
├── filter_logic.rs              # matches_virustotal_category / matches_hybridanalysis_category
└── components/
    ├── mod.rs
    └── package_table_mobile.rs  # TableBuilder, 3 columns
```

### Component hierarchy

```
dlg_mobile_list.rs  (unchanged shape: owns window chrome, dispatches by MobileListViewType)
  └─> MobileListViewType::VirusTotal | HybridAnalysis
        └─> dlg_mobile_scan::view_mobile::render(...)
              └─> dlg_mobile_scan::components::package_table_mobile::render(...)
                    └─> egui_extras::TableBuilder (virtual scrolling)
```

`dlg_dashcounter_details.rs` is untouched — its desktop `.show()` path and
`render_virustotal_table`/`render_hybridanalysis_table` continue to serve the >1010px
dashcounter details window exactly as today, reachable only for the categories that still call
`dlg_dashcounter_details.open()` (Offa/Fmhy — VT/HA no longer will, matching the earlier
stalkerware/izzyrisk precedent).

## Decoupling from `DlgDashCounterDetails`

### Category identity

New enum in `dlg_mobile_scan/mod.rs`:

```rust
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
```

`DashCounterCategory` keeps all its VT/HA variants for the desktop dialog. The two enums are
independent; `ScanCategory` is not derived from or convertible to `DashCounterCategory` — same
relationship `RiskCategory` has to it.

### Window title

`dlg_mobile_scan::window_title(category, count_enabled, count_total)` produces strings using
the same base text as `DlgDashCounterDetails::get_window_title` (dlg_dashcounter_details.rs:1142-1152)
— e.g. `"VirusTotal: Malicious"`, `"HybridAnalysis: Malicious (Ignored)"` — with `" ({}/{})"`
appended, matching the risk table's `window_title` convention (an enhancement over the desktop
title, which shows no count).

### Dashboard-counter click handlers

`uad_shizuku_app.rs`'s `("virustotal", 0-3)` / `("hybridanalysis", 0-4)` arms
(:1388-1432) currently produce a `(DashCounterCategory, count_enabled, count_total)` tuple that
flows into `self.dlg_dashcounter_details.open(cat, count_enabled, count_total)`. These change to
match the existing `("stalkerware", _)`/`("izzyrisk", _)` arms' shape:

```rust
("virustotal", 0) => {
    self.dlg_mobile_list.scan_state.category = Some(ScanCategory::VirusTotalMalicious);
    self.dlg_mobile_list.scan_state.count_enabled = cached_scan_counts.vt_counts.1 .0;
    self.dlg_mobile_list.scan_state.count_total = cached_scan_counts.vt_counts.1 .1;
    self.dlg_mobile_list
        .open(crate::dlg_mobile_list::MobileListViewType::VirusTotal, None);
    (None, 0, 0)
}
```

(and similarly for the other 3 VT + 5 HA arms), reusing the already-computed
`cached_scan_counts.vt_counts`/`ha_counts` tuples rather than recomputing counts manually (VT/HA
don't need the manual recomputation stalkerware/izzyrisk do, since `cached_scan_counts` already
has these figures on hand). Returning `(None, 0, 0)` means the later
`if let Some(cat) = category { self.dlg_dashcounter_details.open(...) }` is skipped for these 9
categories, exactly as it already is for stalkerware/izzyrisk.

This makes `dlg_mobile_list.open(...)` unconditional on tap (not gated by screen width) for
VT/HA too — the same behavior class stalkerware/izzyrisk already have. `dlg_mobile_list.rs`'s
own `current_width > 1010.0` auto-close check is what actually governs whether the window
renders; no new width logic is introduced here.

### Row click → package info, enable/disable/uninstall

Identical to the risk table's approach — no deviation:

- `mobile_info_dialog: DlgPackageDetails` owned by `ScanTableState`, opened by index on row-info
  click, rendered by `view_mobile::render` itself.
- Toggle/uninstall go through the existing `ctx.data_mut` temp-key convention
  (`enable_clicked_package` / `disable_clicked_package` / `uninstall_clicked_package` +
  `uninstall_clicked_is_system`), handled by the same `uad_shizuku_app.rs` handler that already
  services the risk table (~lines 1700-1765). No `viewmodel.batch_*` calls — `installed_packages`
  is still a `shared_store`-sourced param (Non-Goals).

## State

```rust
// dlg_mobile_scan/state.rs
pub struct ScanTableState {
    pub category: Option<ScanCategory>,
    pub count_enabled: usize,
    pub count_total: usize,
    pub show_only_enabled: bool,
    pub hide_system_app: bool,
    pub text_filter: String,
    pub mobile_info_dialog: crate::dlg_package_details::DlgPackageDetails,
    pub uninstall_confirm_dialog: crate::dlg_uninstall_confirm::DlgUninstallConfirm,
}
```

No `sort_column`/`sort_ascending`/`cache_key`/`cached_rows` — same rationale as `RiskTableState`:
those are `data_table()`/`shared_store`-era concerns that don't apply once metadata comes from
`app_metadata_renderer::prepare_app_info_for_display` and scan state comes straight from the
ViewModel each frame.

`ScanTableState` is owned by `DlgMobileList` (new field `scan_state`), same lifetime as
`risk_state` — only matters while a VirusTotal/HybridAnalysis drill-down is open.

## Data source

| Data | Source |
|---|---|
| VT scan results | `vm_state.vt_scanner_state: Option<calc_virustotal_stt::ScannerState>` — already in `ViewModelState` (viewmodel/mod.rs:77) |
| HA scan results | `vm_state.ha_scanner_state: Option<calc_hybridanalysis_stt::ScannerState>` — already in `ViewModelState` (viewmodel/mod.rs:78) |
| Icons / titles | `app_metadata_renderer::prepare_app_info_for_display(...)` — same call debloat's and the risk table's mobile views already make |
| `uad_ng_lists` | `vm_state.uad_ng_lists` |
| `installed_packages` | Unchanged: `shared_store.get_installed_packages()`, passed as a plain `&[PackageFingerprint]` param (Non-Goals) |
| `hybridanalysis_tag_ignorelist` | New plain `&str` param on `DlgMobileList::show()`, sourced from `self.settings.hybridanalysis_tag_ignorelist` (same value already threaded to `dlg_dashcounter_details.show()` today) |

Unlike the risk table's `package_risk_scores` (which stayed on `shared_store`/`tab_scan_control`
because migrating it was explicitly out of scope), VT/HA scanner state is *already*
ViewModel-resident — this design uses `vm_state` directly rather than reintroducing a
`shared_store` dependency for it.

## Filtering

`dlg_mobile_scan/filter_logic.rs` ports the category-matching predicates verbatim from
`render_virustotal_table` (dlg_dashcounter_details.rs:1772-1821) and
`render_hybridanalysis_table` (:1992-2066), rebased onto `vm_state.vt_scanner_state`/
`ha_scanner_state` instead of `get_shared_store().get_vt_scanner_state()`/`get_ha_scanner_state()`:

- **VirusTotal**: a package's `ScanStatus::Completed(result)` buckets to `NotScanned` if any
  `file_result` has `not_found`/`skipped`/`error`; otherwise `Malicious` (any `malicious > 0`),
  `Suspicious` (any `suspicious > 0` with no malicious), or `Safe` (all clean). Non-`Completed`
  states (`Pending`/`Scanning`/`Error`/absent) bucket to `NotScanned`.
- **HybridAnalysis**: a package's `ScanStatus::Completed(result)` buckets to `NotScanned` if any
  `file_result.verdict` is `"404 Not Found"`/`""`/`"upload_error"`/`"analysis_error"`; otherwise
  `Malicious`/`MaliciousIgnored` split by whether *all* of a malicious file's
  `classification_tags` are present in the (comma-split, lowercased)
  `hybridanalysis_tag_ignorelist`, `Suspicious` (any verdict containing `"suspicious"`), or
  `Safe` (no malicious/suspicious verdicts). Non-`Completed` states bucket to `NotScanned`.

`should_show_package`/`matches_text_filter` are reused from `dlg_mobile_risk::filter_logic`
(already category-agnostic) rather than duplicated a third time.

## Table layout

`components/package_table_mobile.rs` — `egui_extras::TableBuilder`, 3 columns (unlike the risk
table's 2 — VT/HA chips are interactive and don't compress into a text line):

| Column | Width | Content |
|---|---|---|
| Name/Status | remainder | icon + title + package id + enabled/disabled status — identical cell to debloat/risk table's |
| Scan Result | ~260px fixed | Per-`file_result` colored chips (`egui::Frame`), ported from `render_vt_cell`/`render_ha_cell`, wrapped in a horizontal `egui::ScrollArea` so row height stays fixed regardless of file count. Click → `webbrowser::open(&file_result.vt_link / ha_link)` (`#[cfg(not(target_os = "android"))]`, unchanged gate); hover → tooltip (file path, or path+error/error message when present). HA chip text ported from `get_ha_display_text` (verdict/tags/score/wait-time formatting); HA chip color additionally depends on `hybridanalysis_tag_ignorelist` tag matching (malicious-but-all-tags-ignored renders gray, not red). |
| Tasks | 200px fixed | info / toggle / uninstall icon buttons — same `icon_button_standard` widgets and 40px touch targets as debloat/risk table |

Row height: fixed 56.0 (debloat's height — no secondary text line here, unlike IzzyRisk's 64.0).

`is_virustotal(category)` / `is_hybridanalysis(category)` helpers select which chip-rendering
branch runs and which column header (`tr!("col-virustotal")` vs `tr!("col-hybrid-analysis")`,
reusing existing translation keys) is shown.

Dropped relative to the current desktop-style table: column-click sorting and the expandable
UAD description drawer (see Non-Goals — same as the risk table).

## Integration points

1. **`dlg_mobile_list_stt.rs`**:
   - Add `scan_state: dlg_mobile_scan::ScanTableState` field to `DlgMobileList`.
   - Add `MobileListViewType::VirusTotal` and `MobileListViewType::HybridAnalysis` variants.
2. **`dlg_mobile_list.rs`**:
   - `show()` signature: add one new parameter, `hybridanalysis_tag_ignorelist: &str`. No other
     parameters change (VT/HA don't need `package_risk_scores`, but the existing param stays for
     the `Stalkerware`/`IzzyRisk` arms that still use it).
   - `VirusTotal`/`HybridAnalysis` arms call `dlg_mobile_scan::view_mobile::render(...)` instead
     of falling through to `dlg_dashcounter_details`.
   - Window title match gains arms calling `dlg_mobile_scan::window_title(...)`.
3. **`uad_shizuku_app.rs`**:
   - The 9 VT/HA dashboard-counter click arms set `self.dlg_mobile_list.scan_state.*` and call
     `self.dlg_mobile_list.open(MobileListViewType::VirusTotal | HybridAnalysis, None)` instead
     of building a `DashCounterCategory` tuple for `dlg_dashcounter_details.open()` (see
     Dashboard-counter click handlers above).
   - `DlgMobileList::show()` call site passes `&self.settings.hybridanalysis_tag_ignorelist` as
     the new argument.

## Testing

- Unit tests for `dlg_mobile_scan/filter_logic.rs`'s category predicates: VT
  malicious/suspicious/safe/not-scanned bucketing (incl. not-found/skipped/error →
  not-scanned), HA malicious/malicious-ignored/suspicious/safe/not-scanned bucketing (incl.
  tag-ignorelist splitting malicious from malicious-ignored) — mirroring
  `dlg_mobile_risk/filter_logic.rs`'s existing test coverage.
- `ScanCategory` equality/distinctness tests, mirroring `RiskCategory`'s.
- Manual verification on a device/emulator with 1000+ packages, several with multi-file scan
  results: tap each of the 9 dashboard counters, confirm the table renders promptly (no visible
  lag versus today's `data_table()` dialog), confirm chip coloring/click-to-open-report/hover
  tooltip, confirm tag-ignorelist correctly splits HA malicious vs. malicious-ignored, confirm
  filter toggles, info tap, enable/disable, and uninstall-confirm all work.
- No automated UI/perf test exists for egui rendering in this repo (per CLAUDE.md §11,
  "UI tests are manual") — this stays manual, consistent with debloat's and the risk table's
  rollout.

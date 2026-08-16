# Mobile IzzyRisk & Stalkerware Table Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the mobile IzzyRisk/Stalkerware drill-down tables' dependency on `dlg_dashcounter_details.rs`'s laggy `data_table()`/`shared_store`-based rendering with a dedicated `TableBuilder`-based, ViewModel-backed module mirroring `tab_debloat`'s mobile pattern.

**Architecture:** New module `mobile/src/dlg_mobile_risk/` (enum + state + filter predicates + a 2-column virtualized table + a render entry point) is wired into `dlg_mobile_list.rs` in place of the current calls into `DlgDashCounterDetails`. Enable/disable/uninstall reuse the existing app-wide `ctx.data_mut` temp-key convention (not `viewmodel`) so `shared_store` — this table's package-list source — stays in sync, exactly like today's working enable/disable path.

**Tech Stack:** Rust, egui/eframe, `egui_extras::TableBuilder`, existing ViewModel actor system.

**Spec:** `docs/superpowers/specs/2026-08-16-mobile-risk-table-design.md`

## Global Constraints

- No file under `mobile/src/dlg_mobile_risk/` imports from `dlg_dashcounter_details.rs` or `dlg_dashcounter_details_stt.rs`.
- `installed_packages` stays sourced from `shared_store.get_installed_packages()` and `package_risk_scores` stays sourced from `self.tab_scan_control.package_risk_scores` — neither source changes (spec Non-Goals).
- Icons/titles come from `crate::app_metadata_renderer::prepare_app_info_for_display` (ViewModel-backed), never from direct `shared_store` metadata lookups.
- Desktop `dlg_dashcounter_details.rs` rendering (the >1010px dialog) is not modified.
- No column-click sorting, no expandable description drawer, no checkbox/batch-select column on the new table (spec Non-Goals).
- Every `cargo build -p mobile` (or workspace `cargo build`) and `cargo test -p mobile` must pass at the end of each task before moving to the next.

---

### Task 1: `RiskCategory` enum and module skeleton

**Files:**
- Create: `mobile/src/dlg_mobile_risk/mod.rs`
- Create: `mobile/src/dlg_mobile_risk/components/mod.rs`
- Modify: `mobile/src/lib.rs` (add module registration)

**Interfaces:**
- Produces: `pub enum RiskCategory { IzzyRiskSafe, IzzyRiskNormal, IzzyRiskModerate, IzzyRiskHigh, StalkerwareDetected, StalkerwareUndetected }` (derives `Debug, Clone, PartialEq, Eq`), and `pub fn window_title(category: &RiskCategory, count_enabled: usize, count_total: usize) -> String`, both in `crate::dlg_mobile_risk`.

- [ ] **Step 1: Write the failing tests**

Create `mobile/src/dlg_mobile_risk/mod.rs`:

```rust
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
        assert_ne!(RiskCategory::StalkerwareDetected, RiskCategory::StalkerwareUndetected);
        assert_ne!(RiskCategory::IzzyRiskHigh, RiskCategory::StalkerwareDetected);
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
```

This references `components`, `filter_logic`, `state`, `view_mobile` modules that don't exist
yet — the crate won't compile until Steps 2-5 below stub them out. Create minimal stubs now
so `cargo test` can run for this task in isolation:

Create `mobile/src/dlg_mobile_risk/components/mod.rs`:

```rust
//! UI components for the mobile risk drill-down table.

pub mod package_table_mobile;
```

Create `mobile/src/dlg_mobile_risk/components/package_table_mobile.rs` (placeholder, replaced
in Task 4):

```rust
// Placeholder — implemented in Task 4.
```

Create `mobile/src/dlg_mobile_risk/filter_logic.rs` (placeholder, replaced in Task 2):

```rust
// Placeholder — implemented in Task 2.
```

Create `mobile/src/dlg_mobile_risk/state.rs` (placeholder, replaced in Task 3):

```rust
// Placeholder — implemented in Task 3.
```

Create `mobile/src/dlg_mobile_risk/view_mobile.rs` (placeholder, replaced in Task 5):

```rust
// Placeholder — implemented in Task 5.
```

- [ ] **Step 2: Register the module in `lib.rs`**

In `mobile/src/lib.rs`, next to the existing `pub mod dlg_mobile_list;` declaration (around
line 85), add:

```rust
pub mod dlg_mobile_risk;
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p mobile dlg_mobile_risk:: -- --nocapture`
Expected: 3 tests pass (`test_risk_category_variants_are_distinct`,
`test_window_title_izzyrisk_safe`, `test_window_title_stalkerware_detected`).

- [ ] **Step 4: Run full build to verify no regressions**

Run: `cargo build -p mobile`
Expected: builds cleanly (placeholders compile as empty modules).

- [ ] **Step 5: Commit**

```bash
git add mobile/src/lib.rs mobile/src/dlg_mobile_risk/
git commit -m "feat(mobile): add dlg_mobile_risk module skeleton with RiskCategory enum"
```

---

### Task 2: Filter predicates (`filter_logic.rs`)

**Files:**
- Modify: `mobile/src/dlg_mobile_risk/filter_logic.rs` (replace Task 1 placeholder)
- Modify: `mobile/src/tab_debloat/mod.rs:224` (visibility change)
- Test: inline `#[cfg(test)] mod tests` in `filter_logic.rs`

**Interfaces:**
- Consumes: `crate::adb_stt::PackageFingerprint`, `crate::viewmodel::ViewModelState` (specifically `vm_state.cached_metadata: MetadataCache` with methods `get_android_package(&str) -> Option<&AndroidPackageInfo>`, `get_fdroid(&str) -> Option<&FDroidApp>`, `get_google_play(&str) -> Option<&GooglePlayApp>`, `get_apkmirror(&str) -> Option<&ApkMirrorApp>`, all with a `.title`/`.label` `String` field), `crate::dlg_mobile_risk::RiskCategory`.
- Produces: `pub fn should_show_package(package: &PackageFingerprint, show_only_enabled: bool, hide_system_app: bool) -> bool`, `pub fn get_display_name(pkg_id: &str, vm_state: &ViewModelState) -> String`, `pub fn matches_text_filter(text_filter: &str, package: &PackageFingerprint, vm_state: &ViewModelState) -> bool`, `pub fn matches_izzyrisk_category(category: &RiskCategory, score: i32) -> bool`. Re-exports `crate::tab_debloat::is_package_enabled` as `pub use`.

- [ ] **Step 1: Make `is_package_enabled` crate-visible**

In `mobile/src/tab_debloat/mod.rs:224`, change:

```rust
fn is_package_enabled(package: &crate::adb::PackageFingerprint) -> bool {
```

to:

```rust
pub(crate) fn is_package_enabled(package: &crate::adb::PackageFingerprint) -> bool {
```

This function was already being duplicated (a copy also exists inline as `get_enabled_status`
in `dlg_dashcounter_details.rs` and as an inline closure `is_pkg_enabled` in 6 places in
`uad_shizuku_app.rs`). Making the debloat copy `pub(crate)` lets the new module reuse it
instead of adding a fourth copy.

- [ ] **Step 2: Write the failing tests**

Replace `mobile/src/dlg_mobile_risk/filter_logic.rs` with:

```rust
//! Filter predicates for the mobile IzzyRisk/Stalkerware risk table.
//!
//! Ported from `DlgDashCounterDetails::should_show_package` /
//! `matches_text_filter` / `get_display_name` (dlg_dashcounter_details.rs:2229-2665), with
//! `get_display_name` rebased onto the ViewModel's `cached_metadata` instead of the legacy
//! `shared_store`, per the design spec's decoupling goal.

use crate::adb_stt::PackageFingerprint;
use crate::dlg_mobile_risk::RiskCategory;
use crate::viewmodel::ViewModelState;

pub use crate::tab_debloat::is_package_enabled;

/// Applies the `show_only_enabled` / `hide_system_app` toggles.
pub fn should_show_package(
    package: &PackageFingerprint,
    show_only_enabled: bool,
    hide_system_app: bool,
) -> bool {
    let is_system = package.flags.contains("SYSTEM");
    let is_enabled = is_package_enabled(package);

    if show_only_enabled && !is_enabled {
        return false;
    }
    if hide_system_app && is_system {
        return false;
    }
    true
}

/// Display name for text-filter matching, sourced from the ViewModel's metadata cache.
/// Same priority order as the original `shared_store`-based version: Android package label,
/// then FDroid/GooglePlay/APKMirror title, then the package id itself.
pub fn get_display_name(pkg_id: &str, vm_state: &ViewModelState) -> String {
    if let Some(android_app) = vm_state.cached_metadata.get_android_package(pkg_id) {
        if !android_app.label.is_empty() {
            return android_app.label.to_lowercase();
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        if let Some(fdroid_app) = vm_state.cached_metadata.get_fdroid(pkg_id) {
            if !fdroid_app.title.is_empty() {
                return fdroid_app.title.to_lowercase();
            }
        }
        if let Some(gp_app) = vm_state.cached_metadata.get_google_play(pkg_id) {
            if !gp_app.title.is_empty() {
                return gp_app.title.to_lowercase();
            }
        }
        if let Some(am_app) = vm_state.cached_metadata.get_apkmirror(pkg_id) {
            if !am_app.title.is_empty() {
                return am_app.title.to_lowercase();
            }
        }
    }

    pkg_id.to_lowercase()
}

/// Case-insensitive substring match against package id, display name, and version.
pub fn matches_text_filter(
    text_filter: &str,
    package: &PackageFingerprint,
    vm_state: &ViewModelState,
) -> bool {
    if text_filter.is_empty() {
        return true;
    }
    let filter_lower = text_filter.to_lowercase();

    if package.pkg.to_lowercase().contains(&filter_lower) {
        return true;
    }
    if get_display_name(&package.pkg, vm_state).contains(&filter_lower) {
        return true;
    }
    if package.versionName.to_lowercase().contains(&filter_lower) {
        return true;
    }
    false
}

/// IzzyRisk risk-score bucket predicate. Mirrors the filter closure in
/// `render_izzyrisk_table` (dlg_dashcounter_details.rs:1555-1565).
pub fn matches_izzyrisk_category(category: &RiskCategory, score: i32) -> bool {
    match category {
        RiskCategory::IzzyRiskSafe => score == 0,
        RiskCategory::IzzyRiskNormal => (1..=10).contains(&score),
        RiskCategory::IzzyRiskModerate => (11..=20).contains(&score),
        RiskCategory::IzzyRiskHigh => score > 20,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adb_stt::{AdbPackageInfoUser, PackageFingerprint};

    fn make_package(pkg: &str, flags: &str, enabled: i32, installed: bool) -> PackageFingerprint {
        PackageFingerprint {
            pkg: pkg.to_string(),
            codePath: String::new(),
            versionCode: 1,
            versionName: "1.0".to_string(),
            flags: flags.to_string(),
            privateFlags: String::new(),
            installPermissions: Vec::new(),
            users: vec![AdbPackageInfoUser {
                userId: 0,
                ceDataInode: 0,
                deDataInode: 0,
                installed,
                hidden: false,
                suspended: false,
                distractionFlags: 0,
                stopped: false,
                notLaunched: false,
                enabled,
            }],
            lastUpdateTime: String::new(),
            pkgChecksum: String::new(),
            dumpText: String::new(),
        }
    }

    #[test]
    fn test_should_show_package_no_filters() {
        let pkg = make_package("com.example.app", "", 1, true);
        assert!(should_show_package(&pkg, false, false));
    }

    #[test]
    fn test_should_show_package_hides_disabled_when_show_only_enabled() {
        let pkg = make_package("com.example.app", "", 2, true); // enabled=2 => disabled
        assert!(!should_show_package(&pkg, true, false));
        assert!(should_show_package(&pkg, false, false));
    }

    #[test]
    fn test_should_show_package_hides_system_when_hide_system_app() {
        let pkg = make_package("com.example.app", "SYSTEM", 1, true);
        assert!(!should_show_package(&pkg, false, true));
        assert!(should_show_package(&pkg, false, false));
    }

    #[test]
    fn test_matches_text_filter_empty_filter_matches_everything() {
        let pkg = make_package("com.example.app", "", 1, true);
        let vm_state = ViewModelState::default();
        assert!(matches_text_filter("", &pkg, &vm_state));
    }

    #[test]
    fn test_matches_text_filter_matches_package_id() {
        let pkg = make_package("com.example.app", "", 1, true);
        let vm_state = ViewModelState::default();
        assert!(matches_text_filter("example", &pkg, &vm_state));
        assert!(matches_text_filter("EXAMPLE", &pkg, &vm_state));
        assert!(!matches_text_filter("nomatch", &pkg, &vm_state));
    }

    #[test]
    fn test_matches_text_filter_matches_version() {
        let pkg = make_package("com.example.app", "", 1, true);
        let vm_state = ViewModelState::default();
        assert!(matches_text_filter("1.0", &pkg, &vm_state));
    }

    #[test]
    fn test_get_display_name_falls_back_to_pkg_id_when_no_metadata() {
        let vm_state = ViewModelState::default();
        assert_eq!(get_display_name("com.example.App", &vm_state), "com.example.app");
    }

    #[test]
    fn test_matches_izzyrisk_category_buckets() {
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskSafe, 0));
        assert!(!matches_izzyrisk_category(&RiskCategory::IzzyRiskSafe, 1));
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskNormal, 1));
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskNormal, 10));
        assert!(!matches_izzyrisk_category(&RiskCategory::IzzyRiskNormal, 11));
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskModerate, 11));
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskModerate, 20));
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskHigh, 21));
        assert!(!matches_izzyrisk_category(&RiskCategory::IzzyRiskHigh, 20));
        assert!(!matches_izzyrisk_category(&RiskCategory::StalkerwareDetected, 5));
    }
}
```

Note: full priority-cascade coverage of `get_display_name` (Android/FDroid/GooglePlay/
APKMirror branches) isn't unit-testable in isolation — `MetadataCache`'s fields are private
to `viewmodel::mod` with no test constructor, so only the empty-cache fallback path is
covered here. The cascade logic itself is a straight port of the already-shipped
`shared_store` version; the spec's Testing section covers the rest via manual device
verification.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p mobile dlg_mobile_risk::filter_logic:: -- --nocapture`
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add mobile/src/dlg_mobile_risk/filter_logic.rs mobile/src/tab_debloat/mod.rs
git commit -m "feat(mobile): add risk table filter predicates ported off shared_store"
```

---

### Task 3: `RiskTableState`

**Files:**
- Modify: `mobile/src/dlg_mobile_risk/state.rs` (replace Task 1 placeholder)

**Interfaces:**
- Consumes: `crate::dlg_package_details::DlgPackageDetails` (`::new() -> Self`, `.open(idx: usize)`, `.show(ctx, installed_packages: &[PackageFingerprint], uad_ng_lists: &Option<UadNgLists>)`), `crate::dlg_uninstall_confirm::DlgUninstallConfirm` (`Default`, `.open_single(pkg: String, is_system: bool)`, `.show(ctx) -> bool`, fields `packages: Vec<String>`, `is_system: Vec<bool>`), `crate::dlg_mobile_risk::RiskCategory`.
- Produces: `pub struct RiskTableState { pub category: Option<RiskCategory>, pub count_enabled: usize, pub count_total: usize, pub show_only_enabled: bool, pub hide_system_app: bool, pub text_filter: String, pub mobile_info_dialog: DlgPackageDetails, pub uninstall_confirm_dialog: DlgUninstallConfirm }`, implementing `Default`.

- [ ] **Step 1: Write the failing test**

Replace `mobile/src/dlg_mobile_risk/state.rs` with:

```rust
//! State for the mobile IzzyRisk/Stalkerware risk table.

use crate::dlg_mobile_risk::RiskCategory;
use crate::dlg_package_details::DlgPackageDetails;
use crate::dlg_uninstall_confirm::DlgUninstallConfirm;

/// UI state for a Stalkerware/IzzyRisk drill-down: which category is active, the local
/// filters, and the two dialogs it owns (package info, uninstall confirmation). Lives as
/// long as `dlg_mobile_list.rs`'s `MobileListViewType::Stalkerware`/`IzzyRisk` is open.
pub struct RiskTableState {
    pub category: Option<RiskCategory>,
    pub count_enabled: usize,
    pub count_total: usize,
    pub show_only_enabled: bool,
    pub hide_system_app: bool,
    pub text_filter: String,
    pub mobile_info_dialog: DlgPackageDetails,
    pub uninstall_confirm_dialog: DlgUninstallConfirm,
}

impl Default for RiskTableState {
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
        let state = RiskTableState::default();
        assert!(state.category.is_none());
        assert_eq!(state.count_enabled, 0);
        assert_eq!(state.count_total, 0);
        assert!(!state.show_only_enabled);
        assert!(!state.hide_system_app);
        assert!(state.text_filter.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p mobile dlg_mobile_risk::state:: -- --nocapture`
Expected: `test_default_state` passes.

- [ ] **Step 3: Commit**

```bash
git add mobile/src/dlg_mobile_risk/state.rs
git commit -m "feat(mobile): add RiskTableState"
```

---

### Task 4: Mobile table component (`components/package_table_mobile.rs`)

**Files:**
- Modify: `mobile/src/dlg_mobile_risk/components/package_table_mobile.rs` (replace Task 1 placeholder)

**Interfaces:**
- Consumes: `crate::adb_stt::PackageFingerprint`, `crate::dlg_mobile_risk::RiskCategory`, `crate::tab_debloat::components::package_table_mobile::AppDisplayData` (`= HashMap<String, (Option<egui::TextureHandle>, String)>`), `crate::uad_shizuku_app::UadNgLists`, `crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_TOGGLE_OFF, ICON_TOGGLE_ON}`, `egui_material3::icon_button_standard`.
- Produces: `pub fn render_risk_table_mobile(ui: &mut egui::Ui, packages: &[&PackageFingerprint], category: &RiskCategory, package_risk_scores: &HashMap<String, i32>, uad_ng_lists: Option<&UadNgLists>, app_display_data: &AppDisplayData, unsafe_app_remove: bool, expert_app_remove: bool, on_info_clicked: &mut dyn FnMut(&str), on_toggle_clicked: &mut dyn FnMut(&str, bool), on_delete_clicked: &mut dyn FnMut(&str))`.

- [ ] **Step 1: Write the failing test**

Replace `mobile/src/dlg_mobile_risk/components/package_table_mobile.rs` with:

```rust
//! Mobile-optimized risk table component (IzzyRisk / Stalkerware).
//!
//! 2 columns: Name/Status (+ risk score/permissions for IzzyRisk) + Tasks.
//! Modeled on `tab_debloat::components::package_table_mobile`, minus the checkbox column
//! (neither risk table has batch-select) and with an extra secondary line for IzzyRisk rows.

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::HashMap;

use crate::adb_stt::PackageFingerprint;
use crate::dlg_mobile_risk::RiskCategory;
use crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::tab_debloat::components::package_table_mobile::AppDisplayData;
use crate::uad_shizuku_app::UadNgLists;
use egui_material3::icon_button_standard;

// Taller than debloat's 56.0: IzzyRisk rows carry a third text line (risk score/permissions).
const ROW_HEIGHT: f32 = 64.0;
const TASKS_COLUMN_WIDTH: f32 = 200.0;
const MOBILE_BUTTON_SPACING: f32 = 16.0;
const MOBILE_TOUCH_TARGET: f32 = 40.0;

fn is_izzyrisk(category: &RiskCategory) -> bool {
    matches!(
        category,
        RiskCategory::IzzyRiskSafe
            | RiskCategory::IzzyRiskNormal
            | RiskCategory::IzzyRiskModerate
            | RiskCategory::IzzyRiskHigh
    )
}

/// Secondary text line shown under the title for IzzyRisk rows only.
fn izzyrisk_secondary_line(
    pkg_id: &str,
    package_risk_scores: &HashMap<String, i32>,
    permissions_count: usize,
) -> String {
    let risk_score = package_risk_scores.get(pkg_id).copied().unwrap_or(0);
    format!("Risk {} \u{b7} {} perms", risk_score, permissions_count)
}

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

#[allow(clippy::too_many_arguments)]
pub fn render_risk_table_mobile(
    ui: &mut egui::Ui,
    packages: &[&PackageFingerprint],
    category: &RiskCategory,
    package_risk_scores: &HashMap<String, i32>,
    uad_ng_lists: Option<&UadNgLists>,
    app_display_data: &AppDisplayData,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
    on_info_clicked: &mut dyn FnMut(&str),
    on_toggle_clicked: &mut dyn FnMut(&str, bool),
    on_delete_clicked: &mut dyn FnMut(&str),
) {
    let show_risk_line = is_izzyrisk(category);

    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::remainder())
        .column(Column::exact(TASKS_COLUMN_WIDTH))
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.label("Name");
            });
            header.col(|ui| {
                ui.label("Tasks");
            });
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, packages.len(), |mut row| {
                let package = packages[row.index()];

                // Column 1: Name/Status (+ risk line for IzzyRisk)
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

                            if show_risk_line {
                                let permissions_count = package.installPermissions.len();
                                ui.label(
                                    egui::RichText::new(izzyrisk_secondary_line(
                                        &package.pkg,
                                        package_risk_scores,
                                        permissions_count,
                                    ))
                                    .small()
                                    .weak(),
                                );
                            }
                        });
                    });
                });

                // Column 2: Tasks (info / toggle / uninstall)
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

                        if show_delete_button(
                            &package.pkg,
                            uad_ng_lists,
                            unsafe_app_remove,
                            expert_app_remove,
                        ) {
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
        assert_eq!(ROW_HEIGHT, 64.0);
        assert_eq!(TASKS_COLUMN_WIDTH, 200.0);
        assert_eq!(MOBILE_BUTTON_SPACING, 16.0);
        assert_eq!(MOBILE_TOUCH_TARGET, 40.0);
    }

    #[test]
    fn test_is_izzyrisk() {
        assert!(is_izzyrisk(&RiskCategory::IzzyRiskSafe));
        assert!(is_izzyrisk(&RiskCategory::IzzyRiskHigh));
        assert!(!is_izzyrisk(&RiskCategory::StalkerwareDetected));
        assert!(!is_izzyrisk(&RiskCategory::StalkerwareUndetected));
    }

    #[test]
    fn test_izzyrisk_secondary_line_formatting() {
        let mut scores = HashMap::new();
        scores.insert("com.example.app".to_string(), 15);
        assert_eq!(
            izzyrisk_secondary_line("com.example.app", &scores, 4),
            "Risk 15 \u{b7} 4 perms"
        );
    }

    #[test]
    fn test_izzyrisk_secondary_line_defaults_to_zero_score() {
        let scores = HashMap::new();
        assert_eq!(
            izzyrisk_secondary_line("com.unknown.app", &scores, 0),
            "Risk 0 \u{b7} 0 perms"
        );
    }

    #[test]
    fn test_show_delete_button_defaults_true_with_no_uad_lists() {
        assert!(show_delete_button("com.example.app", None, false, false));
        assert!(show_delete_button("com.example.app", None, true, true));
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p mobile dlg_mobile_risk::components:: -- --nocapture`
Expected: all tests pass.

- [ ] **Step 3: Commit**

```bash
git add mobile/src/dlg_mobile_risk/components/package_table_mobile.rs
git commit -m "feat(mobile): add TableBuilder-based risk table component"
```

---

### Task 5: Render entry point (`view_mobile.rs`)

**Files:**
- Modify: `mobile/src/dlg_mobile_risk/view_mobile.rs` (replace Task 1 placeholder)

**Interfaces:**
- Consumes: everything produced by Tasks 1-4 (`RiskCategory`, `RiskTableState`, `filter_logic::*`, `components::package_table_mobile::render_risk_table_mobile`), plus `crate::app_metadata_renderer::prepare_app_info_for_display(ctx, package_ids: &[String], system_packages: &HashSet<String>, vm_state: &ViewModelState, google_play_enabled: bool, fdroid_enabled: bool, apkmirror_enabled: bool, android_package_enabled: bool) -> AppMetadataMap` where `AppMetadataMap = HashMap<String, (Option<egui::TextureHandle>, String, String, Option<String>)>`.
- Produces: `pub fn render(ui: &mut egui::Ui, ctx: &egui::Context, vm_state: &ViewModelState, local_state: &mut RiskTableState, installed_packages: &[PackageFingerprint], package_risk_scores: &HashMap<String, i32>, unsafe_app_remove: bool, expert_app_remove: bool, google_play_enabled: bool, fdroid_enabled: bool, apkmirror_enabled: bool, android_package_enabled: bool)`. Writes to `ctx` temp keys `enable_clicked_package`, `disable_clicked_package`, `uninstall_clicked_package`, `uninstall_clicked_is_system` (existing app-wide convention, handled in Task 7).

- [ ] **Step 1: Implement (no isolated unit test — this is an egui integration point; correctness is verified by the crate-wide build plus Task 7's manual verification pass)**

Replace `mobile/src/dlg_mobile_risk/view_mobile.rs` with:

```rust
//! Render entry point for the mobile IzzyRisk/Stalkerware drill-down.
//!
//! Mirrors `tab_debloat::view_mobile`'s shape: filter row, then a virtualized table, then
//! the dialogs it owns. Unlike debloat, filtering here is local/synchronous (no ViewModel
//! filter command) since IzzyRisk/Stalkerware aren't part of the debloat filter pipeline.

use eframe::egui;
use std::collections::{HashMap, HashSet};

use super::components::package_table_mobile::render_risk_table_mobile;
use super::filter_logic;
use super::state::RiskTableState;
use super::RiskCategory;
use crate::adb::PackageFingerprint;
use crate::viewmodel::ViewModelState;

#[allow(clippy::too_many_arguments)]
pub fn render(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    vm_state: &ViewModelState,
    local_state: &mut RiskTableState,
    installed_packages: &[PackageFingerprint],
    package_risk_scores: &HashMap<String, i32>,
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

    let is_izzyrisk = matches!(
        category,
        RiskCategory::IzzyRiskSafe
            | RiskCategory::IzzyRiskNormal
            | RiskCategory::IzzyRiskModerate
            | RiskCategory::IzzyRiskHigh
    );

    let filtered_packages: Vec<&PackageFingerprint> = installed_packages
        .iter()
        .filter(|pkg| {
            let matches_category = if is_izzyrisk {
                match package_risk_scores.get(&pkg.pkg) {
                    Some(&score) => filter_logic::matches_izzyrisk_category(&category, score),
                    None => false,
                }
            } else {
                match &vm_state.stalkerware_indicators {
                    Some(indicators) => {
                        let is_stalkerware = indicators.is_stalkerware(&pkg.pkg);
                        (category == RiskCategory::StalkerwareDetected) == is_stalkerware
                    }
                    None => false,
                }
            };

            matches_category
                && filter_logic::should_show_package(
                    pkg,
                    local_state.show_only_enabled,
                    local_state.hide_system_app,
                )
                && filter_logic::matches_text_filter(&local_state.text_filter, pkg, vm_state)
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
        .id_salt("risk_table_mobile_scroll")
        .show(ui, |ui| {
            render_risk_table_mobile(
                ui,
                &filtered_packages,
                &category,
                package_risk_scores,
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

fn render_filter_row(ui: &mut egui::Ui, local_state: &mut RiskTableState) {
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

- [ ] **Step 2: Run full build**

Run: `cargo build -p mobile`
Expected: builds cleanly.

- [ ] **Step 3: Run tests**

Run: `cargo test -p mobile dlg_mobile_risk:: -- --nocapture`
Expected: all `dlg_mobile_risk` tests (Tasks 1-5) pass.

- [ ] **Step 4: Commit**

```bash
git add mobile/src/dlg_mobile_risk/view_mobile.rs
git commit -m "feat(mobile): add risk table render entry point"
```

---

### Task 6: Wire `dlg_mobile_list.rs` to the new module

**Files:**
- Modify: `mobile/src/dlg_mobile_list_stt.rs`
- Modify: `mobile/src/dlg_mobile_list.rs`

**Interfaces:**
- Consumes: `crate::dlg_mobile_risk::{RiskTableState, RiskCategory, window_title, view_mobile::render}`.
- Produces: `DlgMobileList` gains a `pub risk_state: RiskTableState` field; `DlgMobileList::show()`'s signature drops the `dlg_dashcounter_details: &mut DlgDashCounterDetails` and `stalkerware_indicators: &Option<StalkerwareIndicators>` parameters.

- [ ] **Step 1: Add `risk_state` field to `DlgMobileList`**

In `mobile/src/dlg_mobile_list_stt.rs`, change the struct (currently `#[derive(Debug, Clone)]`,
lines 6-19) to:

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
```

Drop `#[derive(Debug, Clone)]` from the struct — `RiskTableState` owns `DlgPackageDetails`/
`DlgUninstallConfirm`, which aren't `Clone`, and nothing in the codebase clones/debug-prints
`DlgMobileList` (confirm with `grep -rn "dlg_mobile_list.clone()\|{:?}.*dlg_mobile_list" mobile/src/`
before removing — expect no matches).

Update `impl Default for DlgMobileList` (lines 32-41) to include the new field:

```rust
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

- [ ] **Step 2: Run test to verify existing tests still compile/pass**

Run: `cargo test -p mobile dlg_mobile_list_stt:: -- --nocapture`
Expected: `test_default_state`, `test_view_type_equality`,
`test_stalkerware_and_izzyrisk_view_types_are_distinct` still pass.

- [ ] **Step 3: Rewire `dlg_mobile_list.rs`**

In `mobile/src/dlg_mobile_list.rs`, remove these two imports (lines 8-9):

```rust
use crate::calc_stalkerware_stt::StalkerwareIndicators;
use crate::dlg_dashcounter_details::DlgDashCounterDetails;
```

Change the `show()` signature (lines 52-68) from:

```rust
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
        dlg_dashcounter_details: &mut DlgDashCounterDetails,
        installed_packages: &[PackageFingerprint],
        stalkerware_indicators: &Option<StalkerwareIndicators>,
        package_risk_scores: &HashMap<String, i32>,
        unsafe_app_remove: bool,
        expert_app_remove: bool,
    ) {
```

to:

```rust
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

Change the window-title `match` (lines 88-102) from:

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
                match &dlg_dashcounter_details.category {
                    Some(category) => dlg_dashcounter_details.get_window_title(category),
                    None => "Details".to_string(),
                }
            }
        };
```

to:

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

Change the `Stalkerware`/`IzzyRisk` render arms (lines 207-244) from:

```rust
                    MobileListViewType::Stalkerware => {
                        if let Some(category) = dlg_dashcounter_details.category.clone() {
                            dlg_dashcounter_details.render_filter_controls(ui);
                            ui.add_space(8.0);
                            dlg_dashcounter_details.render_stalkerware_table(
                                ui,
                                ctx,
                                installed_packages,
                                stalkerware_indicators,
                                &category,
                                clicked_package_idx.clone(),
                                unsafe_app_remove,
                                expert_app_remove,
                                &vm_state.uad_ng_lists,
                            );
                        } else {
                            ui.label("No category selected");
                        }
                    }
                    MobileListViewType::IzzyRisk => {
                        if let Some(category) = dlg_dashcounter_details.category.clone() {
                            dlg_dashcounter_details.render_filter_controls(ui);
                            ui.add_space(8.0);
                            dlg_dashcounter_details.render_izzyrisk_table(
                                ui,
                                ctx,
                                installed_packages,
                                package_risk_scores,
                                &category,
                                clicked_package_idx.clone(),
                                unsafe_app_remove,
                                expert_app_remove,
                                &vm_state.uad_ng_lists,
                            );
                        } else {
                            ui.label("No category selected");
                        }
                    }
```

to:

```rust
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
```

Remove the now-unused `clicked_package_idx` (line 106, only used by the old Stalkerware/
IzzyRisk arms and the trailing `handle_package_click` block — still needed by nothing else in
this file, since `MobileListViewType::Debloat`'s arm never used it) and the trailing
`handle_package_click` block (lines 248-257):

```rust
        if matches!(
            self.view_type,
            MobileListViewType::Stalkerware | MobileListViewType::IzzyRisk
        ) {
            dlg_dashcounter_details.handle_package_click(
                ctx,
                installed_packages,
                &clicked_package_idx,
            );
        }
```

Delete this block entirely — `RiskTableState.mobile_info_dialog` now handles info clicks
directly inside `dlg_mobile_risk::view_mobile::render`.

Also remove the now-unused `use std::sync::{Arc, Mutex};` import (line 14) if nothing else in
the file uses `Arc`/`Mutex` (confirm with `grep -n "Arc<\|Mutex<" mobile/src/dlg_mobile_list.rs`
after the edits above — expect no matches).

- [ ] **Step 4: Build to check for compile errors from this file alone**

Run: `cargo build -p mobile 2>&1 | grep -A5 "dlg_mobile_list.rs"`
Expected: no errors from `dlg_mobile_list.rs` itself (errors from `uad_shizuku_app.rs`'s
still-unfixed call site are expected and fixed in Task 7).

- [ ] **Step 5: Commit**

```bash
git add mobile/src/dlg_mobile_list_stt.rs mobile/src/dlg_mobile_list.rs
git commit -m "feat(mobile): wire dlg_mobile_list to dlg_mobile_risk, drop dashcounter dependency"
```

---

### Task 7: Wire `uad_shizuku_app.rs` and revive the uninstall handler

**Files:**
- Modify: `mobile/src/uad_shizuku_app.rs` (import block, 6 dashboard-counter handler arms, the `dlg_mobile_list.show()` call site, and the dead uninstall handler)

**Interfaces:**
- Consumes: `crate::dlg_mobile_risk::RiskCategory`, `crate::adb::uninstall_app(package_name: &str, device: &str) -> std::io::Result<String>`.

- [ ] **Step 1: Add the `RiskCategory` import**

In `mobile/src/uad_shizuku_app.rs`, next to the existing `use crate::dlg_dashcounter_details::DlgDashCounterDetails;` (line 29), add:

```rust
use crate::dlg_mobile_risk::RiskCategory;
```

- [ ] **Step 2: Update the 6 dashboard-counter handler arms**

Each arm currently sets `self.dlg_dashcounter_details.category`/`count_enabled`/
`count_total` right before `self.dlg_mobile_list.open(...)`. Change each to set
`self.dlg_mobile_list.risk_state.*` instead. The count-computation code above each of these
lines is unchanged.

At `uad_shizuku_app.rs:1108-1111` (`("stalkerware", 0)` arm), change:

```rust
                    self.dlg_dashcounter_details.category =
                        Some(DashCounterCategory::StalkerwareDetected);
                    self.dlg_dashcounter_details.count_enabled = enabled;
                    self.dlg_dashcounter_details.count_total = total;
```

to:

```rust
                    self.dlg_mobile_list.risk_state.category =
                        Some(RiskCategory::StalkerwareDetected);
                    self.dlg_mobile_list.risk_state.count_enabled = enabled;
                    self.dlg_mobile_list.risk_state.count_total = total;
```

At `uad_shizuku_app.rs:1163-1166` (`("stalkerware", 1)` arm), change:

```rust
                    self.dlg_dashcounter_details.category =
                        Some(DashCounterCategory::StalkerwareUndetected);
                    self.dlg_dashcounter_details.count_enabled = enabled;
                    self.dlg_dashcounter_details.count_total = total;
```

to:

```rust
                    self.dlg_mobile_list.risk_state.category =
                        Some(RiskCategory::StalkerwareUndetected);
                    self.dlg_mobile_list.risk_state.count_enabled = enabled;
                    self.dlg_mobile_list.risk_state.count_total = total;
```

At `uad_shizuku_app.rs:1220-1222` (`("izzyrisk", 0)` arm), change:

```rust
                    self.dlg_dashcounter_details.category = Some(DashCounterCategory::IzzyRiskHigh);
                    self.dlg_dashcounter_details.count_enabled = enabled;
                    self.dlg_dashcounter_details.count_total = total;
```

to:

```rust
                    self.dlg_mobile_list.risk_state.category = Some(RiskCategory::IzzyRiskHigh);
                    self.dlg_mobile_list.risk_state.count_enabled = enabled;
                    self.dlg_mobile_list.risk_state.count_total = total;
```

At `uad_shizuku_app.rs:1274-1277` (`("izzyrisk", 1)` arm), change:

```rust
                    self.dlg_dashcounter_details.category =
                        Some(DashCounterCategory::IzzyRiskModerate);
                    self.dlg_dashcounter_details.count_enabled = enabled;
                    self.dlg_dashcounter_details.count_total = total;
```

to:

```rust
                    self.dlg_mobile_list.risk_state.category =
                        Some(RiskCategory::IzzyRiskModerate);
                    self.dlg_mobile_list.risk_state.count_enabled = enabled;
                    self.dlg_mobile_list.risk_state.count_total = total;
```

At `uad_shizuku_app.rs:1329-1332` (`("izzyrisk", 2)` arm), change:

```rust
                    self.dlg_dashcounter_details.category =
                        Some(DashCounterCategory::IzzyRiskNormal);
                    self.dlg_dashcounter_details.count_enabled = enabled;
                    self.dlg_dashcounter_details.count_total = total;
```

to:

```rust
                    self.dlg_mobile_list.risk_state.category =
                        Some(RiskCategory::IzzyRiskNormal);
                    self.dlg_mobile_list.risk_state.count_enabled = enabled;
                    self.dlg_mobile_list.risk_state.count_total = total;
```

At `uad_shizuku_app.rs:1384-1386` (`("izzyrisk", 3)` arm), change:

```rust
                    self.dlg_dashcounter_details.category = Some(DashCounterCategory::IzzyRiskSafe);
                    self.dlg_dashcounter_details.count_enabled = enabled;
                    self.dlg_dashcounter_details.count_total = total;
```

to:

```rust
                    self.dlg_mobile_list.risk_state.category = Some(RiskCategory::IzzyRiskSafe);
                    self.dlg_mobile_list.risk_state.count_enabled = enabled;
                    self.dlg_mobile_list.risk_state.count_total = total;
```

None of the six `self.dlg_mobile_list.open(crate::dlg_mobile_list::MobileListViewType::...)`
calls immediately below each of these blocks change.

- [ ] **Step 3: Update the `dlg_mobile_list.show()` call site**

At `uad_shizuku_app.rs:1628-1643`, change:

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
                &mut self.dlg_dashcounter_details,
                &installed_packages,
                &stalkerware_indicators,
                package_risk_scores,
                self.settings.unsafe_app_remove,
                self.settings.expert_app_remove,
            );
```

to:

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

The `stalkerware_indicators` local variable at line 1594 is still used by the unmodified
`self.dlg_dashcounter_details.show(...)` call at line 1597 — do not remove that local
binding, only its use as an argument here.

- [ ] **Step 4: Revive the dead uninstall handler**

At `uad_shizuku_app.rs:1768`, the line `// REMOVED: Uninstall confirmation dialog
(tab_debloat_control phased out)` marks where `uninstall_clicked_package`/
`uninstall_clicked_is_system` are written (by the new risk table, via
`dlg_mobile_risk::view_mobile::render`) but never read. Add a real handler here, mirroring
the enable/disable handlers immediately above it (same file, ~lines 1721-1765): direct `adb`
call, then patch `shared_store`'s package list so the table's next-frame render reflects the
uninstall immediately.

Replace:

```rust
        // REMOVED: Uninstall confirmation dialog (tab_debloat_control phased out)
```

with:

```rust
        // Perform uninstall action (revived for the mobile IzzyRisk/Stalkerware risk table —
        // this path was dead since tab_debloat_control was phased out; see
        // docs/superpowers/specs/2026-08-16-mobile-risk-table-design.md)
        if let Some(pkg_name) = uninstall_package {
            if let Some(ref device) = self.selected_device {
                match crate::adb::uninstall_app(&pkg_name, device) {
                    Ok(output) => {
                        log::info!("App uninstalled successfully: {}", output);
                        let mut packages = shared_store.get_installed_packages();
                        if uninstall_is_system {
                            if let Some(pkg) = packages.iter_mut().find(|p| p.pkg == pkg_name) {
                                for user in pkg.users.iter_mut() {
                                    user.enabled = 0;
                                    user.installed = false;
                                }
                            }
                        } else {
                            packages.retain(|p| p.pkg != pkg_name);
                        }
                        shared_store.set_installed_packages(packages);
                    }
                    Err(e) => {
                        log::error!("Failed to uninstall app: {}", e);
                    }
                }
            }
        }
```

This uses the `uninstall_package: Option<String>` and `uninstall_is_system: bool` locals
already extracted from the temp keys earlier in the same function (see the
`ui.ctx().data_mut(...)` block a few dozen lines above, which already populates both — this
was previously computed but unused, hence the dead-code state).

- [ ] **Step 5: Full workspace build**

Run: `cargo build -p mobile`
Expected: builds cleanly with zero errors. If `DashCounterCategory` import warnings appear
(unused import) because no remaining code path constructs
`DashCounterCategory::Stalkerware*`/`IzzyRisk*`, leave the import — it's still used by
`DashCounterCategory`'s other variants (Debloat/VT/HA/Offa/Fmhy) referenced elsewhere in this
same file's `match` block.

- [ ] **Step 6: Run full test suite**

Run: `cargo test -p mobile`
Expected: all tests pass, including the pre-existing `dlg_mobile_list_stt` tests and all new
`dlg_mobile_risk` tests from Tasks 1-5.

- [ ] **Step 7: Manual verification (per spec's Testing section)**

On a device/emulator with a nontrivial package count (spec target: 1000+), narrow the window
to mobile width (<1010px) and tap each of the 6 dashboard counters (Stalkerware Detected,
Stalkerware Undetected, IzzyRisk Safe/Normal/Moderate/High). For each:
- Table renders promptly (no visible lag versus debloat's mobile table).
- `Show only enabled` / `Hide system apps` / text filter all narrow the list correctly.
- Tapping the info icon opens package details.
- Tapping the toggle icon enables/disables the package and the row updates without needing
  to close/reopen the drill-down.
- Tapping the uninstall icon opens a confirmation, and confirming actually uninstalls the app
  and removes/updates the row (this previously did nothing — verify it now works).

- [ ] **Step 8: Commit**

```bash
git add mobile/src/uad_shizuku_app.rs
git commit -m "feat(mobile): route dashboard risk counters through dlg_mobile_risk, revive uninstall handler"
```

---

## Post-implementation cleanup (optional, not required for this feature to work)

`DlgDashCounterDetails::render_stalkerware_table`, `render_izzyrisk_table`,
`render_filter_controls`, `handle_package_click`, and `get_window_title` were made
`pub(crate)` specifically so `dlg_mobile_list.rs` could call them (prior session). After this
plan, nothing outside `dlg_dashcounter_details.rs` calls them anymore. Reverting their
visibility to private is safe dead-code cleanup but isn't required for correctness — leaving
it as a follow-up rather than a task here, since it touches only visibility modifiers with no
behavior change and doesn't block anything in this plan.

# Mobile IzzyRisk & Stalkerware Table Design

**Date:** 2026-08-16
**Author:** Claude Sonnet 5
**Status:** Design Approved

## Overview

Reimplement the IzzyRisk and Stalkerware mobile drill-down tables (reached by tapping a
dashboard counter) as a dedicated mobile module, instead of hosting
`dlg_dashcounter_details.rs`'s desktop-oriented `egui_material3::data_table()` rendering
inside `dlg_mobile_list.rs`. The current approach is laggy on mobile and depends on the
legacy `shared_store` for icons instead of the ViewModel.

## Current State

As of the prior session ("reuse, don't rebuild"), `dlg_mobile_list.rs` hosts Stalkerware/
IzzyRisk drill-downs by calling `pub(crate)` methods on `DlgDashCounterDetails`
(`render_stalkerware_table`, `render_izzyrisk_table`, `render_filter_controls`,
`handle_package_click`, `get_window_title`) inside a viewport-sized window. These methods:

- Render via `egui_material3::data_table()` — not virtualized like `egui_extras::TableBuilder`,
  which is the likely source of the mobile lag (debloat's `TableBuilder`-based table handles
  1000-2000 packages in <300ms; see `2026-08-15-mobile-table-datatable-design.md`).
- Source icons/titles from `shared_store` (`get_cached_android_package_app`,
  `get_*_texture`, etc.) rather than the ViewModel's `cached_metadata` /
  `app_metadata_renderer::prepare_app_info_for_display`.
- Route enable/disable/uninstall/info-click through `ui.data_mut`/`ctx.data_mut` temp-key
  writes (`enable_clicked_package`, `disable_clicked_package`, `uninstall_clicked_package`,
  `info_clicked_package`) that are polled elsewhere in `uad_shizuku_app.rs`.

Debloat's mobile view (`tab_debloat/view_mobile.rs` +
`tab_debloat/components/package_table_mobile.rs`) already solves both the performance and
data-source problems: `TableBuilder` virtual scrolling, ViewModel-backed metadata, and direct
callbacks into `viewmodel.batch_enable/batch_disable/batch_uninstall` instead of temp-key
polling. This design ports that pattern to IzzyRisk/Stalkerware.

## Goals

1. Mobile IzzyRisk/Stalkerware tables render via `egui_extras::TableBuilder` (virtualized),
   matching debloat's mobile performance characteristics.
2. Icons/titles sourced from the ViewModel (`app_metadata_renderer`), not `shared_store`.
3. No file under the new module imports from `dlg_dashcounter_details.rs`.
4. Desktop IzzyRisk/Stalkerware rendering (`dlg_dashcounter_details.rs`'s `.show()`, used
   for the >1010px dashcounter details window) is untouched.

## Non-Goals

- Migrating `installed_packages` (still `shared_store.get_installed_packages()`) or
  `package_risk_scores` (still owned by `tab_scan_control`) into the ViewModel. Both remain
  plain parameters, unchanged in origin. Migrating those sources touches VT/HA/Offa/Fmhy
  dash-counters too and is a separate, larger effort.
- Changing VT/HA/Offa/Fmhy dashboard-counter behavior — they continue to open the desktop
  `dlg_dashcounter_details` dialog even on mobile, unaffected by this change.
- Adding column-click sorting or the expandable description drawer to the mobile table.
  Debloat's mobile table has neither; matching it keeps the new table simple rather than a
  partial port of desktop features.
- Batch selection / checkbox column. Neither current IzzyRisk nor Stalkerware table has
  batch actions — each row's tasks are single-package operations.

## Architecture

### File structure

```
mobile/src/dlg_mobile_risk/
├── mod.rs                       # RiskCategory enum, module declarations, re-exports
├── state.rs                     # RiskTableState: filters + owned dialogs
├── view_mobile.rs               # render entry: filter row + table + dialogs
├── filter_logic.rs              # should_show_package / matches_text_filter
└── components/
    ├── mod.rs
    └── package_table_mobile.rs  # TableBuilder, 2 columns
```

### Component hierarchy

```
dlg_mobile_list.rs  (unchanged: owns window chrome, dispatches by MobileListViewType)
  └─> MobileListViewType::Stalkerware | IzzyRisk
        └─> dlg_mobile_risk::view_mobile::render(...)
              └─> dlg_mobile_risk::components::package_table_mobile::render(...)
                    └─> egui_extras::TableBuilder (virtual scrolling)
```

`dlg_dashcounter_details.rs` is untouched by this change — its desktop `.show()` path and
`render_stalkerware_table`/`render_izzyrisk_table` continue to serve the >1010px dashcounter
details window exactly as today. The `pub(crate)` visibility added to those methods last
session for mobile's benefit can be reverted to private once `dlg_mobile_list.rs` no longer
calls them (dead-code cleanup, not required for this feature to work).

## Decoupling from `DlgDashCounterDetails`

### Category identity

New enum in `dlg_mobile_risk/mod.rs`:

```rust
pub enum RiskCategory {
    IzzyRiskSafe,
    IzzyRiskNormal,
    IzzyRiskModerate,
    IzzyRiskHigh,
    StalkerwareDetected,
    StalkerwareUndetected,
}
```

`DashCounterCategory` (in `dlg_dashcounter_details_stt.rs`) keeps all its variants —
including the IzzyRisk/Stalkerware ones, since the desktop dialog still needs them. The two
enums are independent; `RiskCategory` is not derived from or convertible to
`DashCounterCategory`.

`uad_shizuku_app.rs`'s dashboard-counter click handlers for `("stalkerware", 0|1)` and
`("izzyrisk", 0|1|2|3)` currently do:

```rust
self.dlg_dashcounter_details.category = Some(DashCounterCategory::StalkerwareDetected);
self.dlg_dashcounter_details.count_enabled = enabled;
self.dlg_dashcounter_details.count_total = total;
self.dlg_mobile_list.open(MobileListViewType::Stalkerware, None);
```

This changes to set a `RiskTableState` (owned by `DlgMobileList` — see State below) instead
of touching `self.dlg_dashcounter_details` at all:

```rust
self.dlg_mobile_list.risk_state.category = Some(RiskCategory::StalkerwareDetected);
self.dlg_mobile_list.risk_state.count_enabled = enabled;
self.dlg_mobile_list.risk_state.count_total = total;
self.dlg_mobile_list.open(MobileListViewType::Stalkerware, None);
```

### Window title

`dlg_mobile_list.rs` already inlines its own title logic for `MobileListViewType::Debloat`
(`"Debloat Packages"`) rather than calling out to another module. The `Stalkerware`/
`IzzyRisk` arms get the same treatment — a small local `match` on `RiskCategory` producing
strings like `"IzzyRisk: Safe (0)"`, replacing the current call to
`dlg_dashcounter_details.get_window_title(category)`.

### Row click → package info

Replaces the `ctx.data_mut` "info_clicked_package" + `handle_package_click` indirection.
The table takes an `on_info_clicked: &mut dyn FnMut(&str)` callback (debloat's pattern).
`RiskTableState` owns `mobile_info_dialog: DlgPackageDetails`; `view_mobile::render` opens it
by index on click and renders it itself:

```rust
local_state.mobile_info_dialog.show(ctx, vm_state, &vm_state.uad_ng_lists);
```

### Enable / disable / uninstall

**Correction found during plan-writing:** the original draft of this section proposed
`viewmodel.batch_enable`/`batch_disable`/`batch_uninstall`. That's wrong for this table
specifically — `viewmodel.*` mutates ViewModel state (`vm_state.packages`), but this table's
`installed_packages` comes from `shared_store` (Non-Goals: that source isn't migrating).
`DebloatActor`'s batch methods never touch `shared_store`, so a toggle would succeed on
device but the row would stay stale until some unrelated `shared_store` refresh happened.

**Actual approach:** keep the *existing* app-wide `ctx.data_mut` temp-key convention
(`enable_clicked_package`, `disable_clicked_package`, `uninstall_clicked_package` +
`uninstall_clicked_is_system`). This isn't owned by `dlg_dashcounter_details.rs` — `calc.rs`
writes to the sibling `info_clicked_package` key too — and its handler in
`uad_shizuku_app.rs` (~lines 1700-1765) already performs direct `adb::enable_app` /
`adb::disable_app_current_user` + `shared_store.set_installed_packages(...)`, which is
exactly what keeps this table's next-frame render correct.

The table still exposes `on_toggle_clicked: &mut dyn FnMut(&str, bool)` and
`on_delete_clicked: &mut dyn FnMut(&str)` callbacks (debloat's pattern), but their
implementation in `view_mobile.rs` writes to the temp keys instead of calling `viewmodel`:

- Toggle → `ctx.data_mut(|data| data.insert_temp(Id::new("enable_clicked_package"), pkg_id))`
  (or `disable_clicked_package`).
- Delete → `local_state.uninstall_confirm_dialog.open_single(pkg_id, is_system)`, then on
  confirm (`.show(ctx)` returns `true`), write `uninstall_clicked_package` +
  `uninstall_clicked_is_system`.

**`uninstall_clicked_package` is currently dead code** — its handler was removed when
`tab_debloat_control` was phased out (see the `// REMOVED: Uninstall confirmation dialog`
comment at `uad_shizuku_app.rs:1766`), so desktop IzzyRisk/Stalkerware uninstall is silently
broken today. This plan adds a real handler there, mirroring the enable/disable pattern:
`adb::uninstall_app(pkg, device)`, then on success patch `shared_store`'s package list — for
a system app, set `enabled = 0, installed = false` on all users (matches the existing
"Removed"/`REMOVED_USER` status display already used elsewhere); for a non-system app,
remove the package entry from the list entirely.

No new parameters on `DlgMobileList::show()` are needed for this — `viewmodel` and a device
string are not required at this layer.

## State

```rust
// dlg_mobile_risk/state.rs
pub struct RiskTableState {
    pub category: Option<RiskCategory>,
    pub count_enabled: usize,
    pub count_total: usize,
    pub show_only_enabled: bool,
    pub hide_system_app: bool,
    pub text_filter: String,
    pub mobile_info_dialog: crate::dlg_package_details::DlgPackageDetails,
    pub uninstall_confirm_dialog: crate::dlg_uninstall_confirm::DlgUninstallConfirm,
}
```

No `sort_column`/`sort_ascending`/`cache_key`/`cached_rows` — sorting and the row-cache
scheme (`prepare_row_cache`, `generate_cache_key`, `should_refresh_cache`) are
`data_table()`/`shared_store`-era concerns that don't apply once metadata comes from
`app_metadata_renderer::prepare_app_info_for_display` (already computed once per frame from
the ViewModel, no separate row cache needed — same as debloat's mobile view).

`RiskTableState` is owned by `DlgMobileList` (field `risk_state`), not by
`UadShizukuApp` directly — it only matters while a Stalkerware/IzzyRisk drill-down is open,
same lifetime as `MobileListViewType`.

## Data source

| Data | Source |
|---|---|
| Icons / titles | `app_metadata_renderer::prepare_app_info_for_display(ctx, &package_ids, &system_packages, vm_state, google_play_enabled, fdroid_enabled, apkmirror_enabled, android_package_enabled)` — same call debloat's mobile view already makes |
| `stalkerware_indicators` | `vm_state.stalkerware_indicators` (already populated there) instead of the separately-threaded param |
| `uad_ng_lists` | `vm_state.uad_ng_lists` |
| `installed_packages` | Unchanged: `shared_store.get_installed_packages()`, passed as a plain `&[PackageFingerprint]` param, same as today |
| `package_risk_scores` | Unchanged: `&self.tab_scan_control.package_risk_scores`, passed as a plain param, same as today |

## Filtering

`dlg_mobile_risk/filter_logic.rs` ports the two predicate methods currently on
`DlgDashCounterDetails` (lines ~2623–2665), as free functions or `RiskTableState` methods —
same semantics, no behavior change:

- `should_show_package`: applies `show_only_enabled` (via the enabled-status derivation
  already duplicated in `tab_debloat/mod.rs::is_package_enabled` — reuse that instead of
  re-deriving it a third time) and `hide_system_app` (checks `flags.contains("SYSTEM")`).
- `matches_text_filter`: case-insensitive substring match against package id, display name,
  and version.
- Category predicate: for IzzyRisk, `package_risk_scores.get(&pkg.pkg)` bucketed into
  Safe(0)/Normal(1-10)/Moderate(11-20)/High(20+); for Stalkerware,
  `stalkerware_indicators.is_stalkerware(&pkg.pkg)`. Both match the current logic in
  `render_izzyrisk_table`/`render_stalkerware_table` exactly.

## Table layout

`components/package_table_mobile.rs` — `egui_extras::TableBuilder`, modeled directly on
`tab_debloat/components/package_table_mobile.rs` but with 2 columns instead of 3 (no
checkbox — neither table has batch-select):

| Column | Width | Content |
|---|---|---|
| Name/Status | remainder | icon + title + package id + enabled/disabled status (debloat's existing cell). For IzzyRisk only: risk score and permission count appended as small secondary text under the title. Stalkerware: no extra line. |
| Tasks | 200px fixed | info / toggle (enable-disable) / uninstall icon buttons — same `icon_button_standard` widgets and 40px touch targets as debloat's mobile table |

Dropped relative to the current desktop-style table: separate Risk Score / Permissions
columns (folded into the name cell), the expandable description drawer, and column-click
sorting (see Non-Goals).

## Integration points

1. **`dlg_mobile_list_stt.rs`**: add `risk_state: dlg_mobile_risk::RiskTableState` field to
   `DlgMobileList`.
2. **`dlg_mobile_list.rs`**:
   - `show()` signature: drop the `dlg_dashcounter_details: &mut DlgDashCounterDetails`
     parameter and the separately-threaded `stalkerware_indicators` parameter (now read from
     `vm_state.stalkerware_indicators`); `package_risk_scores` stays. No new parameters are
     added — enable/disable/uninstall go through the existing temp-key convention (see
     Enable / disable / uninstall above), not `viewmodel`.
   - `Stalkerware`/`IzzyRisk` arms call `dlg_mobile_risk::view_mobile::render(...)` instead
     of `dlg_dashcounter_details.render_stalkerware_table`/`render_izzyrisk_table`.
   - Window title match gains local arms for `RiskCategory` (see Window title above).
   - Drop the post-window `dlg_dashcounter_details.handle_package_click(...)` call for these
     two view types (superseded by `mobile_info_dialog` inside `view_mobile::render`).
3. **`uad_shizuku_app.rs`**: the 6 dashboard-counter click handlers for `"stalkerware"` /
   `"izzyrisk"` set `self.dlg_mobile_list.risk_state.*` instead of
   `self.dlg_dashcounter_details.*`; the `DlgMobileList::show()` call site updates for the
   new/dropped parameters.

## Testing

- Unit tests for `filter_logic.rs` predicates (should_show_package, matches_text_filter,
  category bucketing) — pure functions, easy to test in isolation, mirroring
  `tab_debloat/filter_logic.rs`'s existing test coverage.
- `RiskCategory` equality/distinctness tests, mirroring the existing
  `test_stalkerware_and_izzyrisk_view_types_are_distinct` pattern in
  `dlg_mobile_list_stt.rs`.
- Manual verification on a device/emulator with 1000+ packages: tap each of the 6 dashboard
  counters, confirm the table renders promptly (no visible lag versus debloat's mobile
  table), confirm filter toggles, info tap, enable/disable, and uninstall-confirm all work.
- No automated UI/perf test exists for egui rendering in this repo (per CLAUDE.md §11,
  "UI tests are manual") — this stays manual, consistent with debloat's mobile rollout.

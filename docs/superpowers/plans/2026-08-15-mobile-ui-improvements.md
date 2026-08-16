# Mobile UI Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix mobile UI issues including auto-close on resize, fullsize windows, extracted filter logic, and broken app icon rendering.

**Architecture:** Five independent fixes to egui dialogs and mobile view. Extract shared filter logic to common module, add window resize detection to mobile list dialog, update window sizing for fullsize display, and fix metadata renderer flags propagation.

**Tech Stack:** Rust, egui 0.29, egui_extras, Material3 components

## Global Constraints

- All code must follow Rust 2021 edition conventions
- Use `smol` async runtime (NOT tokio)
- Maintain MVVM architecture pattern (ViewModel + actors)
- Follow naming: `snake_case` for functions/variables, `CamelCase` for types
- Error handling: `anyhow` for application code
- No `.unwrap()` or `.expect()` in production code
- Test coverage: 80% minimum
- Immutable by default, `mut` only when necessary

---

## Task 1: Auto-Close Mobile List Dialog on Resize >800px

**Files:**
- Modify: `mobile/src/dlg_mobile_list.rs:46-111`
- Modify: `mobile/src/dlg_mobile_list_stt.rs` (add last_width field)

**Interfaces:**
- Consumes: `egui::Context::screen_rect()` - screen dimensions
- Produces: Auto-close behavior when viewport width exceeds 800px

- [ ] **Step 1: Add last_width tracking field to state**

```rust
// mobile/src/dlg_mobile_list_stt.rs
#[derive(Debug, Clone, PartialEq)]
pub struct DlgMobileList {
    pub open: bool,
    pub view_type: MobileListViewType,
    pub category_filter: Option<String>,
    pub last_width: Option<f32>,  // ADD THIS LINE
}

impl Default for DlgMobileList {
    fn default() -> Self {
        Self {
            open: false,
            view_type: MobileListViewType::Debloat,
            category_filter: None,
            last_width: None,  // ADD THIS LINE
        }
    }
}
```

Expected: Compilation succeeds

- [ ] **Step 2: Add resize detection in show() method**

```rust
// mobile/src/dlg_mobile_list.rs - Insert BEFORE the early return check at line 56
pub fn show(
    &mut self,
    ctx: &egui::Context,
    vm_state: &ViewModelState,
    tab_debloat_state: &mut crate::tab_debloat::TabDebloatState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) {
    // Check viewport width and auto-close if >800px
    let current_width = ctx.screen_rect().width();
    if current_width > 800.0 {
        // Close dialog when viewport exceeds mobile threshold
        if self.open {
            log::info!("[MOBILE_LIST] Auto-closing dialog: viewport width {} > 800px", current_width);
            self.close();
            return;
        }
    }
    self.last_width = Some(current_width);

    if !self.open {
        return;
    }
    // ... rest of existing code
}
```

Expected: Dialog closes when window resized >800px

- [ ] **Step 3: Test resize behavior**

Run: `cargo run` (desktop build)
Test steps:
1. Open app, resize window to <800px width
2. Open debloat tab, click a category card
3. Mobile list dialog should open
4. Resize window to >800px width
5. Dialog should auto-close immediately

Expected: Dialog closes when crossing 800px threshold

- [ ] **Step 4: Commit**

```bash
git add mobile/src/dlg_mobile_list.rs mobile/src/dlg_mobile_list_stt.rs
git commit -m "feat(mobile): auto-close mobile list dialog on resize >800px

- Add last_width tracking to DlgMobileList state
- Detect viewport width changes in show() method
- Auto-close when width exceeds 800px threshold
- Prevents mobile UI showing on desktop viewport

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Make Mobile List Dialog Fullsize with Top-Right Close Button

**Files:**
- Modify: `mobile/src/dlg_mobile_list.rs:70-111`

**Interfaces:**
- Consumes: `egui::Window` API, `egui::Context::screen_rect()`
- Produces: Fullsize window with close button in top-right corner

- [ ] **Step 1: Update window configuration for fullsize**

```rust
// mobile/src/dlg_mobile_list.rs - Replace lines 70-85
egui::Window::new(window_title)
    .id(egui::Id::new("mobile_list_window"))
    .title_bar(true)
    .resizable(false)  // Prevent manual resizing
    .collapsible(false)
    .scroll([false, false])
    .fixed_size([ctx.screen_rect().width(), ctx.screen_rect().height()])
    .default_pos([0.0, 0.0])
    .show(ctx, |ui| {
```

Expected: Window fills entire screen

- [ ] **Step 2: Add top-right close button**

```rust
// mobile/src/dlg_mobile_list.rs - Insert AFTER line 86 (inside window)
        // Top-right close button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if ui.button("✕").clicked() {
                self.close();
            }
        });
        
        ui.add_space(8.0);
```

Expected: Close button appears in top-right corner

- [ ] **Step 3: Remove bottom close button**

```rust
// mobile/src/dlg_mobile_list.rs - DELETE lines 103-109 (bottom close button)
// DELETE THIS BLOCK:
//                 // Close button at bottom
//                 ui.separator();
//                 ui.horizontal(|ui| {
//                     if ui.button("Close").clicked() {
//                         self.close();
//                     }
//                 });
```

Expected: Only top-right close button remains

- [ ] **Step 4: Test fullsize layout**

Run: `cargo run`
Test steps:
1. Resize window to <800px width
2. Open mobile list dialog
3. Verify dialog fills entire viewport
4. Verify close button is in top-right corner
5. Click close button
6. Verify dialog closes

Expected: PASS - Fullsize window with functional close button

- [ ] **Step 5: Commit**

```bash
git add mobile/src/dlg_mobile_list.rs
git commit -m "feat(mobile): make mobile list dialog fullsize with top-right close

- Set fixed size to match screen_rect dimensions
- Position at (0,0) to fill viewport
- Add close button in top-right corner
- Remove redundant bottom close button
- Prevent manual resizing for consistent mobile UX

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Make Package Details Dialog Fullsize

**Files:**
- Modify: `mobile/src/dlg_package_details.rs:67-84`

**Interfaces:**
- Consumes: `egui::Window` API, `egui::Context::screen_rect()`
- Produces: Fullsize package details window

- [ ] **Step 1: Update window configuration**

```rust
// mobile/src/dlg_package_details.rs - Replace lines 67-84
egui::Window::new(format!("Package Details: {}", pkg_id))
    .id(egui::Id::new("package_details_window"))
    .title_bar(true)
    .resizable(false)
    .collapsible(false)
    .scroll([false, false])
    .fixed_size([ctx.screen_rect().width(), ctx.screen_rect().height()])
    .default_pos([0.0, 0.0])
    .show(ctx, |ui| {
```

Expected: Package details window fills entire screen

- [ ] **Step 2: Add top-right close button**

```rust
// mobile/src/dlg_package_details.rs - Insert AFTER the window.show(ctx, |ui| { line (around line 85)
        // Top-right close button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if ui.add(egui_material3::MaterialButton::filled("✕")).clicked() {
                close_clicked = true;
            }
        });

        ui.add_space(8.0);
```

Expected: Close button appears in top-right corner

- [ ] **Step 3: Keep existing bottom close button**

No changes needed - bottom close button at line 199-205 provides redundant close option for accessibility

- [ ] **Step 4: Test fullsize package details**

Run: `cargo run`
Test steps:
1. Open package list
2. Click info button on any package
3. Verify package details dialog fills viewport
4. Verify close button in top-right works
5. Verify bottom close button still works
6. Test tab switching between pkg/UAD/GooglePlay/etc

Expected: PASS - Both close buttons functional, fullsize layout

- [ ] **Step 5: Commit**

```bash
git add mobile/src/dlg_package_details.rs
git commit -m "feat(mobile): make package details dialog fullsize

- Set fixed size to match screen_rect dimensions
- Position at (0,0) to fill viewport
- Add close button in top-right corner
- Keep bottom close button for accessibility
- Prevent manual resizing for consistent mobile UX

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Extract Common Filter Logic

**Files:**
- Create: `mobile/src/tab_debloat/filter_logic.rs`
- Modify: `mobile/src/tab_debloat/mod.rs` (add module export)
- Modify: `mobile/src/tab_debloat/view_desktop.rs:67-189` (use common logic)
- Modify: `mobile/src/tab_debloat/view_mobile.rs:85-193` (use common logic)

**Interfaces:**
- Consumes: `TabDebloatState`, `ViewModelState`
- Produces: `pub fn render_category_filters(ui, local_state)`, `pub fn render_options_checkboxes(ui, local_state)`, `pub fn render_advanced_settings(ui, local_state)`, `pub fn render_package_counts(ui, vm_state)`

- [ ] **Step 1: Create filter_logic.rs module**

```rust
// mobile/src/tab_debloat/filter_logic.rs
//! Shared filter logic for desktop and mobile debloat views

use eframe::egui;

use super::state::TabDebloatState;
use crate::viewmodel::ViewModelState;

/// Render category filter buttons (All, Recommended, Advanced, Expert, Unsafe)
///
/// Updates `local_state.active_filter.category_filter` and increments `table_version`
/// when selection changes.
pub fn render_category_filters(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
    ui.label("Category");

    ui.horizontal_wrapped(|ui| {
        if ui
            .selectable_label(
                local_state.active_filter.category_filter.is_none(),
                format!("All ({}/{})", local_state.cached_counts.all_enabled, local_state.cached_counts.all),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = None;
            local_state.table_version += 1;
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("recommended"),
                format!("Recommended ({}/{})", local_state.cached_counts.recommended_enabled, local_state.cached_counts.recommended),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("recommended".to_string());
            local_state.table_version += 1;
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("advanced"),
                format!("Advanced ({}/{})", local_state.cached_counts.advanced_enabled, local_state.cached_counts.advanced),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("advanced".to_string());
            local_state.table_version += 1;
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("expert"),
                format!("Expert ({}/{})", local_state.cached_counts.expert_enabled, local_state.cached_counts.expert),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("expert".to_string());
            local_state.table_version += 1;
        }

        if ui
            .selectable_label(
                local_state.active_filter.category_filter.as_deref() == Some("unsafe"),
                format!("Unsafe ({}/{})", local_state.cached_counts.unsafe_apps_enabled, local_state.cached_counts.unsafe_apps),
            )
            .clicked()
        {
            local_state.active_filter.category_filter = Some("unsafe".to_string());
            local_state.table_version += 1;
        }
    });
}

/// Render options checkboxes (Show only enabled, Hide system apps)
pub fn render_options_checkboxes(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
    ui.label("Options");

    if ui
        .checkbox(
            &mut local_state.active_filter.show_only_enabled,
            "Show only enabled",
        )
        .changed()
    {
        local_state.table_version += 1;
    }

    if ui
        .checkbox(
            &mut local_state.active_filter.hide_system_apps,
            "Hide system apps",
        )
        .changed()
    {
        local_state.table_version += 1;
    }
}

/// Render advanced settings checkboxes (Unsafe removal, Expert removal)
pub fn render_advanced_settings(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
    ui.label("Advanced");

    ui.checkbox(&mut local_state.unsafe_app_remove, "Unsafe removal");
    ui.checkbox(&mut local_state.expert_app_remove, "Expert removal");
}

/// Render device info and package counts
pub fn render_package_counts(ui: &mut egui::Ui, vm_state: &ViewModelState, local_state: &TabDebloatState) {
    // Device info (if available)
    if let Some(device) = &local_state.selected_device {
        ui.separator();
        ui.label("Device");
        ui.label(device);
        ui.add_space(8.0);
    }

    // Package counts
    ui.separator();
    ui.label(format!("Total packages: {}", vm_state.packages.len()));
    ui.label(format!("Filtered: {}", vm_state.filtered_packages.len()));
}
```

Expected: Module compiles successfully

- [ ] **Step 2: Export filter_logic module**

```rust
// mobile/src/tab_debloat/mod.rs - Add this line with other modules
pub mod filter_logic;
```

Expected: cargo check passes

- [ ] **Step 3: Update view_desktop.rs to use common logic**

```rust
// mobile/src/tab_debloat/view_desktop.rs - Add import at top
use super::filter_logic;

// Replace render_sidebar function (lines 67-189) with:
fn render_sidebar(ui: &mut egui::Ui, vm_state: &ViewModelState, local_state: &mut TabDebloatState) {
    ui.vertical(|ui| {
        ui.heading("Filters");
        ui.separator();

        // Category filters
        filter_logic::render_category_filters(ui, local_state);

        ui.add_space(16.0);

        // Options
        ui.separator();
        ui.heading("Options");
        filter_logic::render_options_checkboxes(ui, local_state);

        ui.add_space(16.0);

        // Advanced settings
        ui.separator();
        ui.heading("Advanced");
        filter_logic::render_advanced_settings(ui, local_state);

        // Device info and package counts
        ui.add_space(16.0);
        filter_logic::render_package_counts(ui, vm_state, local_state);
    });
}
```

Expected: Desktop view renders filters correctly

- [ ] **Step 4: Update view_mobile.rs to use common logic**

```rust
// mobile/src/tab_debloat/view_mobile.rs - Add import at top
use super::filter_logic;

// Replace render_filter_section function (lines 85-193) with:
fn render_filter_section(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
) {
    egui::CollapsingHeader::new("Filters")
        .default_open(false)
        .show(ui, |ui| {
            // Category filters
            filter_logic::render_category_filters(ui, local_state);

            ui.add_space(8.0);

            // Options
            ui.separator();
            filter_logic::render_options_checkboxes(ui, local_state);

            ui.add_space(8.0);

            // Advanced settings
            ui.separator();
            filter_logic::render_advanced_settings(ui, local_state);

            // Device info and package counts
            ui.add_space(8.0);
            filter_logic::render_package_counts(ui, vm_state, local_state);
        });
}
```

Expected: Mobile view renders filters correctly

- [ ] **Step 5: Test filter functionality**

Run: `cargo run`
Test steps:
1. Test desktop view (>800px width):
   - Click each category filter (All, Recommended, Advanced, Expert, Unsafe)
   - Verify package list updates
   - Toggle "Show only enabled" and "Hide system apps"
   - Verify package counts update
2. Test mobile view (<800px width):
   - Expand "Filters" collapsing header
   - Click each category filter
   - Verify filters work identically to desktop

Expected: PASS - Filters work identically in both views

- [ ] **Step 6: Commit**

```bash
git add mobile/src/tab_debloat/filter_logic.rs mobile/src/tab_debloat/mod.rs mobile/src/tab_debloat/view_desktop.rs mobile/src/tab_debloat/view_mobile.rs
git commit -m "refactor(mobile): extract common filter logic to shared module

- Create filter_logic.rs with reusable filter functions
- Extract render_category_filters() for All/Recommended/Advanced/Expert/Unsafe
- Extract render_options_checkboxes() for Show only enabled/Hide system apps
- Extract render_advanced_settings() for Unsafe/Expert removal toggles
- Extract render_package_counts() for device info and counts
- Update view_desktop.rs to use shared functions
- Update view_mobile.rs to use shared functions
- Eliminates code duplication between desktop and mobile views

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Fix App Icon Rendering

**Files:**
- Modify: `mobile/src/dlg_mobile_list.rs:46-111`

**Interfaces:**
- Consumes: Renderer enable flags (google_play_enabled, fdroid_enabled, apkmirror_enabled, android_package_enabled)
- Produces: Icons rendered in mobile list dialog

- [ ] **Step 1: Verify renderer flags are passed correctly**

Check that `dlg_mobile_list.rs` show() method passes renderer flags to view_mobile::render():

```rust
// mobile/src/dlg_mobile_list.rs - Verify lines 90-98 look like this:
match self.view_type {
    MobileListViewType::Debloat => {
        crate::tab_debloat::view_mobile::render(
            ui,
            vm_state,
            tab_debloat_state,
            google_play_enabled,
            fdroid_enabled,
            apkmirror_enabled,
            android_package_enabled,
        );
    }
    // Future: Add Scan, Apps views here
}
```

Expected: Flags are passed correctly

- [ ] **Step 2: Add diagnostic logging to identify issue**

```rust
// mobile/src/dlg_mobile_list.rs - Add BEFORE match self.view_type (around line 87)
log::info!(
    "[MOBILE_LIST] Renderer flags - GP: {}, FD: {}, APK: {}, AP: {}",
    google_play_enabled,
    fdroid_enabled,
    apkmirror_enabled,
    android_package_enabled
);
```

Expected: Log output shows renderer flags

- [ ] **Step 3: Test with logging**

Run: `RUST_LOG=info cargo run`
Test steps:
1. Resize to <800px width
2. Open mobile list dialog
3. Check console logs for "[MOBILE_LIST] Renderer flags"
4. Check console logs for "[DEBLOAT] Renderer flags" (from view_mobile.rs line 206)
5. Check console logs for "[DEBLOAT] Got metadata for X packages" (view_mobile.rs line 238)

Expected output:
```
[MOBILE_LIST] Renderer flags - GP: true, FD: true, APK: false, AP: false
[DEBLOAT] Renderer flags - GP: true, FD: true, APK: false, AP: false
[DEBLOAT] Got metadata for X packages
```

- [ ] **Step 4: If flags are false, trace caller**

If all flags show `false`, check `uad_shizuku_app.rs` where mobile_list_dialog.show() is called.

Find the call site and verify it's passing the correct boolean values:

```bash
grep -n "mobile_list_dialog.show" mobile/src/uad_shizuku_app.rs
```

Expected: Should show line number where show() is called with renderer flags

- [ ] **Step 5: Fix if flags are incorrectly passed**

If the call site has hardcoded `false` values:

```rust
// WRONG (example - fix if found):
self.mobile_list_dialog.show(
    ctx,
    vm_state,
    &mut self.tab_debloat_state,
    false,  // WRONG - should be self.google_play_enabled
    false,  // WRONG - should be self.fdroid_enabled
    false,  // WRONG - should be self.apkmirror_enabled
    false,  // WRONG - should be self.android_package_enabled
);

// CORRECT:
self.mobile_list_dialog.show(
    ctx,
    vm_state,
    &mut self.tab_debloat_state,
    self.google_play_enabled,
    self.fdroid_enabled,
    self.apkmirror_enabled,
    self.android_package_enabled,
);
```

Expected: Renderer flags correctly propagated from app state

- [ ] **Step 6: Test icon rendering**

Run: `RUST_LOG=info cargo run`
Test steps:
1. Ensure at least one metadata renderer is enabled in settings
2. Open mobile list dialog
3. Verify app icons load and display
4. Verify app titles show correctly
5. Check logs show "X textures loaded"

Expected: PASS - Icons and titles render correctly

- [ ] **Step 7: Commit**

```bash
git add mobile/src/dlg_mobile_list.rs mobile/src/uad_shizuku_app.rs
git commit -m "fix(mobile): correct renderer flags propagation to mobile list

- Add diagnostic logging for renderer flags
- Fix renderer flag propagation from app state to dialog
- Verify google_play_enabled/fdroid_enabled/apkmirror_enabled/android_package_enabled
  are passed correctly to view_mobile::render()
- Icons now render correctly in mobile list dialog

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Self-Review Checklist

After completing all tasks, verify:

**1. Spec Coverage:**
- ✅ Task 1: Mobile list window auto-close on resize >800px
- ✅ Task 2: Mobile list window fullsize with top-right close button
- ✅ Task 3: Info window (package details) fullsize
- ✅ Task 4: Filter logic extracted to common module
- ✅ Task 5: App icon rendering fixed

**2. Placeholder Scan:**
- ✅ No TBD, TODO, or "fill in details"
- ✅ All code blocks are complete
- ✅ All file paths are exact
- ✅ All test steps have expected outcomes

**3. Type Consistency:**
- ✅ Renderer flags: `bool` type consistently used
- ✅ Window sizing: `f32` for dimensions
- ✅ State updates: `table_version += 1` pattern consistent
- ✅ Function signatures match across desktop/mobile views

**4. Compilation:**
- All code compiles with `cargo check`
- No new clippy warnings introduced
- No new unsafe code blocks
- All imports correctly specified

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-08-15-mobile-ui-improvements.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?

# Mobile Debloat UI Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix three mobile debloat UI issues - filters not applying, app icons not rendering, and info button not working

**Architecture:** Unified filter system where filter controls call `viewmodel.filter_packages()` directly instead of using snapshot comparison. Remove unused state fields. Align info button callback with desktop pattern. Add diagnostic logging for icon rendering.

**Tech Stack:** Rust, egui, smol async runtime, MVVM architecture

## Global Constraints

- Follow Rust 2021 edition conventions
- Use `log::` macros for logging (info, debug, error)
- Maintain immutability patterns (no mutation of existing objects)
- Keep error handling explicit (no `.unwrap()` in production code)
- All filter changes must call `viewmodel.filter_packages()` immediately
- Text search keeps 300ms debounce (lines 88-113 in mod.rs)
- Desktop view must not regress (pass all existing tests)

---

## File Structure

**Modified Files:**
1. `mobile/src/tab_debloat/state.rs` - Remove `table_version` and `last_applied_filter` fields
2. `mobile/src/tab_debloat/filter_logic.rs` - Add `viewmodel` parameter, implement direct filter calls
3. `mobile/src/tab_debloat/mod.rs` - Remove snapshot comparison logic (lines 115-147)
4. `mobile/src/tab_debloat/view_desktop.rs` - Pass `viewmodel` to filter_logic functions
5. `mobile/src/tab_debloat/view_mobile.rs` - Add `viewmodel` parameter, fix info button, add diagnostic logging
6. `mobile/src/dlg_mobile_list.rs` - Update `view_mobile::render()` call with `viewmodel` parameter

**No new files created** - This is a refactoring and bug fix plan.

---

### Task 1: Remove Unused State Fields

**Files:**
- Modify: `mobile/src/tab_debloat/state.rs:109` (remove `table_version`)
- Modify: `mobile/src/tab_debloat/state.rs:121` (remove `last_applied_filter`)
- Modify: `mobile/src/tab_debloat/state.rs:181-210` (update `Default` impl)

**Interfaces:**
- Consumes: Existing `TabDebloatState` struct definition
- Produces: Cleaned `TabDebloatState` without `table_version` and `last_applied_filter` fields

- [ ] **Step 1: Remove table_version field**

Open `mobile/src/tab_debloat/state.rs` and locate line 109:

```rust
// DELETE this line:
pub table_version: u64,
```

Expected: Field removed from struct definition

- [ ] **Step 2: Remove last_applied_filter field**

In same file, locate line 121:

```rust
// DELETE this line:
pub last_applied_filter: DebloatFilter,
```

Expected: Field removed from struct definition

- [ ] **Step 3: Update Default implementation**

Locate the `Default` implementation (line 181) and remove the two field initializations:

```rust
impl Default for TabDebloatState {
    fn default() -> Self {
        Self {
            open: false,
            selected_packages: HashSet::new(),
            active_filter: DebloatFilter::default(),
            sort_column: None,
            sort_ascending: true,
            selected_device: None,
            // REMOVE: table_version: 0,
            last_filter_input: None,
            pending_filter_text: String::new(),
            applied_filter_text: String::new(),
            // REMOVE: last_applied_filter: DebloatFilter::default(),
            package_details_dialog: DlgPackageDetails::new(),
            uninstall_confirm_dialog: DlgUninstallConfirm::default(),
            cached_counts: CachedCategoryCounts::default(),
            unsafe_app_remove: false,
            expert_app_remove: false,
            batch_uninstall_state: BatchUninstallState::default(),
            batch_uninstall_progress: Arc::new(Mutex::new(None)),
            batch_uninstall_cancelled: Arc::new(Mutex::new(false)),
            batch_disable_state: BatchUninstallState::default(),
            batch_disable_progress: Arc::new(Mutex::new(None)),
            batch_disable_cancelled: Arc::new(Mutex::new(false)),
            batch_enable_state: BatchUninstallState::default(),
            batch_enable_progress: Arc::new(Mutex::new(None)),
            batch_enable_cancelled: Arc::new(Mutex::new(false)),
        }
    }
}
```

Expected: No compile errors, two lines removed from Default impl

- [ ] **Step 4: Compile to verify changes**

Run: `cargo check --package mobile`

Expected: Compilation errors showing `table_version` and `last_applied_filter` still referenced elsewhere (expected - we'll fix in next tasks)

- [ ] **Step 5: Commit state struct changes**

```bash
git add mobile/src/tab_debloat/state.rs
git commit -m "refactor(debloat): remove unused table_version and last_applied_filter

- Remove table_version field (no longer needed for cache invalidation)
- Remove last_applied_filter field (no longer needed for snapshot comparison)
- Update Default implementation

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Clean commit with state.rs changes only

---

### Task 2: Update filter_logic Module

**Files:**
- Modify: `mobile/src/tab_debloat/filter_logic.rs:12` (update `render_category_filters` signature)
- Modify: `mobile/src/tab_debloat/filter_logic.rs:~50+` (update filter button implementations)
- Modify: `mobile/src/tab_debloat/filter_logic.rs:~90+` (update `render_options_checkboxes` signature and implementation)

**Interfaces:**
- Consumes: `viewmodel: &crate::viewmodel::ViewModel` with `filter_packages()` method
- Produces: `render_category_filters(ui, local_state, viewmodel)` and `render_options_checkboxes(ui, local_state, viewmodel)` functions

- [ ] **Step 1: Update render_category_filters signature**

Open `mobile/src/tab_debloat/filter_logic.rs` and locate the function signature (around line 12):

```rust
// OLD:
pub fn render_category_filters(ui: &mut egui::Ui, local_state: &mut TabDebloatState)

// NEW:
pub fn render_category_filters(
    ui: &mut egui::Ui,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
)
```

Expected: Function signature updated with viewmodel parameter

- [ ] **Step 2: Remove table_version increments from category filters**

Find all lines with `local_state.table_version += 1;` in `render_category_filters()` (around lines 24, 35, 46, etc.) and delete them:

```rust
// DELETE these lines:
local_state.table_version += 1;
```

Expected: All `table_version` references removed from category filter buttons

- [ ] **Step 3: Add ViewModel call to "All" category button**

Locate the "All" category button click handler (around line 18-26) and replace with:

```rust
if ui
    .selectable_label(
        local_state.active_filter.category_filter.is_none(),
        format!("All ({}/{})", local_state.cached_counts.all_enabled, local_state.cached_counts.all),
    )
    .clicked()
{
    local_state.active_filter.category_filter = None;
    
    // Apply filter immediately via ViewModel
    let text_filter = if local_state.applied_filter_text.is_empty() {
        None
    } else {
        Some(local_state.applied_filter_text.clone())
    };
    
    if let Err(e) = viewmodel.filter_packages(
        text_filter,
        None,
        local_state.active_filter.show_only_enabled,
        local_state.active_filter.hide_system_apps,
    ) {
        log::error!("Failed to apply 'All' filter: {}", e);
    } else {
        log::debug!("Applied category filter: All");
    }
}
```

Expected: "All" button now calls viewmodel.filter_packages()

- [ ] **Step 4: Add ViewModel call to "Recommended" category button**

Locate the "Recommended" button click handler (around line 28-36) and replace with:

```rust
if ui
    .selectable_label(
        local_state.active_filter.category_filter.as_deref() == Some("recommended"),
        format!("Recommended ({}/{})", local_state.cached_counts.recommended_enabled, local_state.cached_counts.recommended),
    )
    .clicked()
{
    local_state.active_filter.category_filter = Some("recommended".to_string());
    
    // Apply filter immediately via ViewModel
    let text_filter = if local_state.applied_filter_text.is_empty() {
        None
    } else {
        Some(local_state.applied_filter_text.clone())
    };
    
    if let Err(e) = viewmodel.filter_packages(
        text_filter,
        Some("recommended".to_string()),
        local_state.active_filter.show_only_enabled,
        local_state.active_filter.hide_system_apps,
    ) {
        log::error!("Failed to apply 'Recommended' filter: {}", e);
    } else {
        log::debug!("Applied category filter: recommended");
    }
}
```

Expected: "Recommended" button now calls viewmodel.filter_packages()

- [ ] **Step 5: Add ViewModel call to "Advanced" category button**

Locate the "Advanced" button click handler (around line 38-46) and replace with:

```rust
if ui
    .selectable_label(
        local_state.active_filter.category_filter.as_deref() == Some("advanced"),
        format!("Advanced ({}/{})", local_state.cached_counts.advanced_enabled, local_state.cached_counts.advanced),
    )
    .clicked()
{
    local_state.active_filter.category_filter = Some("advanced".to_string());
    
    // Apply filter immediately via ViewModel
    let text_filter = if local_state.applied_filter_text.is_empty() {
        None
    } else {
        Some(local_state.applied_filter_text.clone())
    };
    
    if let Err(e) = viewmodel.filter_packages(
        text_filter,
        Some("advanced".to_string()),
        local_state.active_filter.show_only_enabled,
        local_state.active_filter.hide_system_apps,
    ) {
        log::error!("Failed to apply 'Advanced' filter: {}", e);
    } else {
        log::debug!("Applied category filter: advanced");
    }
}
```

Expected: "Advanced" button now calls viewmodel.filter_packages()

- [ ] **Step 6: Add ViewModel call to "Unsafe" category button**

Locate the "Unsafe" button click handler (around line 48-57) and replace with:

```rust
if ui
    .selectable_label(
        local_state.active_filter.category_filter.as_deref() == Some("unsafe"),
        format!("Unsafe ({}/{})", local_state.cached_counts.unsafe_apps_enabled, local_state.cached_counts.unsafe_apps),
    )
    .clicked()
{
    local_state.active_filter.category_filter = Some("unsafe".to_string());
    
    // Apply filter immediately via ViewModel
    let text_filter = if local_state.applied_filter_text.is_empty() {
        None
    } else {
        Some(local_state.applied_filter_text.clone())
    };
    
    if let Err(e) = viewmodel.filter_packages(
        text_filter,
        Some("unsafe".to_string()),
        local_state.active_filter.show_only_enabled,
        local_state.active_filter.hide_system_apps,
    ) {
        log::error!("Failed to apply 'Unsafe' filter: {}", e);
    } else {
        log::debug!("Applied category filter: unsafe");
    }
}
```

Expected: "Unsafe" button now calls viewmodel.filter_packages()

- [ ] **Step 7: Add ViewModel call to "Expert" category button**

Locate the "Expert" button click handler (around line 59-68) and replace with:

```rust
if ui
    .selectable_label(
        local_state.active_filter.category_filter.as_deref() == Some("expert"),
        format!("Expert ({}/{})", local_state.cached_counts.expert_enabled, local_state.cached_counts.expert),
    )
    .clicked()
{
    local_state.active_filter.category_filter = Some("expert".to_string());
    
    // Apply filter immediately via ViewModel
    let text_filter = if local_state.applied_filter_text.is_empty() {
        None
    } else {
        Some(local_state.applied_filter_text.clone())
    };
    
    if let Err(e) = viewmodel.filter_packages(
        text_filter,
        Some("expert".to_string()),
        local_state.active_filter.show_only_enabled,
        local_state.active_filter.hide_system_apps,
    ) {
        log::error!("Failed to apply 'Expert' filter: {}", e);
    } else {
        log::debug!("Applied category filter: expert");
    }
}
```

Expected: "Expert" button now calls viewmodel.filter_packages()

- [ ] **Step 8: Update render_options_checkboxes signature**

Locate `render_options_checkboxes` function signature (around line 75) and update:

```rust
// OLD:
pub fn render_options_checkboxes(ui: &mut egui::Ui, local_state: &mut TabDebloatState)

// NEW:
pub fn render_options_checkboxes(
    ui: &mut egui::Ui,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
)
```

Expected: Function signature updated with viewmodel parameter

- [ ] **Step 9: Update "Show only enabled" checkbox**

Locate the "Show only enabled" checkbox (around line 79-85) and replace with:

```rust
if ui
    .checkbox(
        &mut local_state.active_filter.show_only_enabled,
        "Show only enabled",
    )
    .changed()
{
    // Apply filter immediately via ViewModel
    let text_filter = if local_state.applied_filter_text.is_empty() {
        None
    } else {
        Some(local_state.applied_filter_text.clone())
    };
    
    if let Err(e) = viewmodel.filter_packages(
        text_filter,
        local_state.active_filter.category_filter.clone(),
        local_state.active_filter.show_only_enabled,
        local_state.active_filter.hide_system_apps,
    ) {
        log::error!("Failed to apply 'Show only enabled' filter: {}", e);
    } else {
        log::debug!("Applied 'Show only enabled' filter: {}", local_state.active_filter.show_only_enabled);
    }
}
```

Expected: Checkbox now calls viewmodel.filter_packages() on change

- [ ] **Step 10: Update "Hide system apps" checkbox**

Locate the "Hide system apps" checkbox (around line 87-93) and replace with:

```rust
if ui
    .checkbox(
        &mut local_state.active_filter.hide_system_apps,
        "Hide system apps",
    )
    .changed()
{
    // Apply filter immediately via ViewModel
    let text_filter = if local_state.applied_filter_text.is_empty() {
        None
    } else {
        Some(local_state.applied_filter_text.clone())
    };
    
    if let Err(e) = viewmodel.filter_packages(
        text_filter,
        local_state.active_filter.category_filter.clone(),
        local_state.active_filter.show_only_enabled,
        local_state.active_filter.hide_system_apps,
    ) {
        log::error!("Failed to apply 'Hide system apps' filter: {}", e);
    } else {
        log::debug!("Applied 'Hide system apps' filter: {}", local_state.active_filter.hide_system_apps);
    }
}
```

Expected: Checkbox now calls viewmodel.filter_packages() on change

- [ ] **Step 11: Compile to verify filter_logic changes**

Run: `cargo check --package mobile`

Expected: Compilation errors about missing viewmodel parameter in callers (expected - we'll fix in next tasks)

- [ ] **Step 12: Commit filter_logic changes**

```bash
git add mobile/src/tab_debloat/filter_logic.rs
git commit -m "refactor(debloat): filter controls call ViewModel directly

- Add viewmodel parameter to render_category_filters()
- Add viewmodel parameter to render_options_checkboxes()
- Remove all table_version increments
- Call viewmodel.filter_packages() immediately on filter changes
- Add debug logging for each filter application
- Add error handling with log::error!() for failed filter commands

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Clean commit with filter_logic.rs changes only

---

### Task 3: Remove Snapshot Comparison from TabDebloat

**Files:**
- Modify: `mobile/src/tab_debloat/mod.rs:115-147` (remove snapshot comparison block)

**Interfaces:**
- Consumes: Existing `TabDebloat::render()` method
- Produces: Simplified `render()` without snapshot comparison logic

- [ ] **Step 1: Locate and remove snapshot comparison code**

Open `mobile/src/tab_debloat/mod.rs` and locate lines 115-147 (the snapshot comparison block):

```rust
// DELETE THIS ENTIRE BLOCK (lines 115-147):
        // Check if non-text filters changed (category, checkboxes)
        let current_filter_snapshot = DebloatFilter {
            text_filter: self.state.applied_filter_text.clone(),
            category_filter: self.state.active_filter.category_filter.clone(),
            show_only_enabled: self.state.active_filter.show_only_enabled,
            hide_system_apps: self.state.active_filter.hide_system_apps,
        };

        if current_filter_snapshot != self.state.last_applied_filter {
            // Filter changed, apply immediately
            let text_filter = if current_filter_snapshot.text_filter.is_empty() {
                None
            } else {
                Some(current_filter_snapshot.text_filter.clone())
            };

            if let Err(e) = viewmodel.filter_packages(
                text_filter,
                current_filter_snapshot.category_filter.clone(),
                current_filter_snapshot.show_only_enabled,
                current_filter_snapshot.hide_system_apps,
            ) {
                log::error!("Failed to send filter command: {}", e);
            } else {
                log::debug!(
                    "Applied filter update: category={:?}, enabled={}, hide_system={}",
                    self.state.active_filter.category_filter,
                    self.state.active_filter.show_only_enabled,
                    self.state.active_filter.hide_system_apps
                );
                self.state.last_applied_filter = current_filter_snapshot;
            }
        }
```

Expected: Lines 115-147 deleted, only text search debounce remains (lines 88-113)

- [ ] **Step 2: Verify text search debounce is preserved**

Check that lines 88-113 remain untouched:

```rust
// KEEP THIS BLOCK UNCHANGED (lines 88-113):
        // Check if filter debounce has elapsed and we need to apply the filter
        if let Some(last_input_time) = self.state.last_filter_input {
            let elapsed = last_input_time.elapsed();
            if elapsed.as_millis() >= FILTER_DEBOUNCE_MS as u128
                && self.state.pending_filter_text != self.state.applied_filter_text
            {
                // Debounce elapsed, apply the pending filter
                let text_filter = if self.state.pending_filter_text.is_empty() {
                    None
                } else {
                    Some(self.state.pending_filter_text.clone())
                };

                if let Err(e) = viewmodel.filter_packages(
                    text_filter,
                    self.state.active_filter.category_filter.clone(),
                    self.state.active_filter.show_only_enabled,
                    self.state.active_filter.hide_system_apps,
                ) {
                    log::error!("Failed to send filter command: {}", e);
                } else {
                    // Mark filter as applied
                    self.state.applied_filter_text = self.state.pending_filter_text.clone();
                    self.state.last_filter_input = None;
                }
            }
        }
```

Expected: Text search debounce logic unchanged

- [ ] **Step 3: Compile to verify removal**

Run: `cargo check --package mobile`

Expected: Still compilation errors about viewmodel parameter in view callers (expected - we'll fix in next tasks)

- [ ] **Step 4: Commit TabDebloat changes**

```bash
git add mobile/src/tab_debloat/mod.rs
git commit -m "refactor(debloat): remove snapshot comparison logic

- Remove lines 115-147 (snapshot comparison block)
- Filter changes now handled directly in filter_logic module
- Keep text search debounce logic (lines 88-113) unchanged
- Simplify render() method

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Clean commit with mod.rs changes only

---

### Task 4: Update Desktop View to Pass ViewModel

**Files:**
- Modify: `mobile/src/tab_debloat/view_desktop.rs:74` (pass viewmodel to render_category_filters)
- Modify: `mobile/src/tab_debloat/view_desktop.rs:81` (pass viewmodel to render_options_checkboxes)

**Interfaces:**
- Consumes: `viewmodel: &crate::viewmodel::ViewModel` from `render()` function parameter
- Produces: Updated filter_logic calls in `render_sidebar()`

- [ ] **Step 1: Update render_category_filters call**

Open `mobile/src/tab_debloat/view_desktop.rs` and locate line 74 in `render_sidebar()`:

```rust
// OLD:
filter_logic::render_category_filters(ui, local_state);

// NEW:
filter_logic::render_category_filters(ui, local_state, viewmodel);
```

Expected: Added viewmodel parameter to function call

- [ ] **Step 2: Update render_options_checkboxes call**

Locate line 81 in same function:

```rust
// OLD:
filter_logic::render_options_checkboxes(ui, local_state);

// NEW:
filter_logic::render_options_checkboxes(ui, local_state, viewmodel);
```

Expected: Added viewmodel parameter to function call

- [ ] **Step 3: Verify render_sidebar has viewmodel access**

Check that `render_sidebar()` function signature already has `viewmodel` in scope through the `render()` function. The desktop view already receives viewmodel (line 40).

Expected: viewmodel is available in render_sidebar's parent scope

- [ ] **Step 4: Compile to verify desktop view changes**

Run: `cargo check --package mobile`

Expected: Desktop view compiles, but mobile view still has errors (expected - fixing in next task)

- [ ] **Step 5: Commit desktop view changes**

```bash
git add mobile/src/tab_debloat/view_desktop.rs
git commit -m "refactor(debloat): pass viewmodel to desktop filter controls

- Pass viewmodel to filter_logic::render_category_filters()
- Pass viewmodel to filter_logic::render_options_checkboxes()
- No functional changes, just parameter passing

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Clean commit with view_desktop.rs changes only

---

### Task 5: Update Mobile View - Add ViewModel, Fix Info Button, Add Logging

**Files:**
- Modify: `mobile/src/tab_debloat/view_mobile.rs:29` (add viewmodel parameter to render())
- Modify: `mobile/src/tab_debloat/view_mobile.rs:86` (add viewmodel parameter to render_filter_section())
- Modify: `mobile/src/tab_debloat/view_mobile.rs:45` (pass viewmodel to render_filter_section())
- Modify: `mobile/src/tab_debloat/view_mobile.rs:95,101` (pass viewmodel to filter_logic calls)
- Modify: `mobile/src/tab_debloat/view_mobile.rs:172-176` (fix info button callback)
- Modify: `mobile/src/tab_debloat/view_mobile.rs:116` (add diagnostic logging)

**Interfaces:**
- Consumes: `viewmodel: &crate::viewmodel::ViewModel` from callers
- Produces: Updated `render()` and `render_filter_section()` signatures, fixed info button callback, added diagnostic logging

- [ ] **Step 1: Add viewmodel parameter to render() signature**

Open `mobile/src/tab_debloat/view_mobile.rs` and locate the `render()` function signature (line 29):

```rust
// OLD:
pub fn render(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
)

// NEW:
pub fn render(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
)
```

Expected: viewmodel parameter added after local_state

- [ ] **Step 2: Update render_filter_section() signature**

Locate `render_filter_section()` function signature (line 86):

```rust
// OLD:
fn render_filter_section(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
)

// NEW:
fn render_filter_section(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
)
```

Expected: viewmodel parameter added after local_state

- [ ] **Step 3: Update render_filter_section call in render()**

Locate the call to `render_filter_section()` (line 45):

```rust
// OLD:
render_filter_section(ui, vm_state, local_state);

// NEW:
render_filter_section(ui, vm_state, local_state, viewmodel);
```

Expected: viewmodel passed to render_filter_section()

- [ ] **Step 4: Update filter_logic calls in render_filter_section()**

Locate the filter_logic calls (lines 95 and 101):

```rust
// OLD (line 95):
filter_logic::render_category_filters(ui, local_state);

// NEW:
filter_logic::render_category_filters(ui, local_state, viewmodel);

// OLD (line 101):
filter_logic::render_options_checkboxes(ui, local_state);

// NEW:
filter_logic::render_options_checkboxes(ui, local_state, viewmodel);
```

Expected: viewmodel passed to both filter_logic functions

- [ ] **Step 5: Fix info button callback**

Locate the info button callback in `render_package_list()` (lines 172-176):

```rust
// OLD:
&mut |pkg_id| {
    if let Some(idx) = vm_state.filtered_packages.iter().position(|p| p.pkg == pkg_id) {
        local_state.package_details_dialog.open(idx);
    }
},

// NEW (match desktop pattern):
&mut |pkg_id| {
    if let Some(idx) = vm_state.filtered_packages.iter().position(|p| p.pkg == pkg_id) {
        local_state.package_details_dialog.selected_package_index = Some(idx);
        local_state.package_details_dialog.open = true;
    }
},
```

Expected: Callback now sets fields directly instead of calling `.open()` method

- [ ] **Step 6: Add diagnostic logging to render_package_list()**

Locate the `render_package_list()` function (line 116) and add logging after the existing renderer flags log (line 126-127):

```rust
fn render_package_list(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) {
    // Existing log at line 126-127
    log::info!("[DEBLOAT] Renderer flags - GP: {}, FD: {}, APK: {}, AP: {}",
        google_play_enabled, fdroid_enabled, apkmirror_enabled, android_package_enabled);

    // ADD THIS LOG to confirm function is called:
    log::info!("[DEBLOAT] render_package_list called with {} filtered packages",
        vm_state.filtered_packages.len());

    // ... rest of function
}
```

Expected: Two log statements at the start of render_package_list()

- [ ] **Step 7: Compile to verify mobile view changes**

Run: `cargo check --package mobile`

Expected: Compilation errors in mod.rs and dlg_mobile_list.rs about missing viewmodel parameter (expected - fixing in next task)

- [ ] **Step 8: Commit mobile view changes**

```bash
git add mobile/src/tab_debloat/view_mobile.rs
git commit -m "fix(debloat): update mobile view with viewmodel, fix info button, add logging

- Add viewmodel parameter to render() and render_filter_section()
- Pass viewmodel to filter_logic::render_category_filters()
- Pass viewmodel to filter_logic::render_options_checkboxes()
- Fix info button callback to match desktop pattern (set fields directly)
- Add diagnostic logging to render_package_list()

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Clean commit with view_mobile.rs changes only

---

### Task 6: Update Callers of view_mobile::render()

**Files:**
- Modify: `mobile/src/tab_debloat/mod.rs:227` (pass viewmodel to view_mobile::render())
- Modify: `mobile/src/dlg_mobile_list.rs:110` (pass viewmodel to view_mobile::render())

**Interfaces:**
- Consumes: Updated `view_mobile::render()` signature with viewmodel parameter
- Produces: Fixed caller sites passing viewmodel

- [ ] **Step 1: Update view_mobile::render() call in TabDebloat::render_mobile()**

Open `mobile/src/tab_debloat/mod.rs` and locate `render_mobile()` method (line 227):

```rust
// OLD:
view_mobile::render(
    ui,
    vm_state,
    &mut self.state,
    google_play_enabled,
    fdroid_enabled,
    apkmirror_enabled,
    android_package_enabled,
);

// NEW:
view_mobile::render(
    ui,
    vm_state,
    &mut self.state,
    viewmodel,
    google_play_enabled,
    fdroid_enabled,
    apkmirror_enabled,
    android_package_enabled,
);
```

Expected: viewmodel parameter added to call

- [ ] **Step 2: Update view_mobile::render() call in dlg_mobile_list**

Open `mobile/src/dlg_mobile_list.rs` and locate the `view_mobile::render()` call (line 110):

```rust
// OLD:
crate::tab_debloat::view_mobile::render(
    ui,
    vm_state,
    tab_debloat_state,
    google_play_enabled,
    fdroid_enabled,
    apkmirror_enabled,
    android_package_enabled,
);

// NEW:
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
```

Expected: viewmodel parameter added to call

- [ ] **Step 3: Compile full project to verify all changes**

Run: `cargo check --package mobile`

Expected: Clean compilation with no errors

- [ ] **Step 4: Run tests to verify no regressions**

Run: `cargo test --package mobile`

Expected: All tests pass (if tests exist)

- [ ] **Step 5: Commit caller updates**

```bash
git add mobile/src/tab_debloat/mod.rs mobile/src/dlg_mobile_list.rs
git commit -m "fix(debloat): update view_mobile::render() callers with viewmodel

- Pass viewmodel to view_mobile::render() in TabDebloat::render_mobile()
- Pass viewmodel to view_mobile::render() in dlg_mobile_list
- All compilation errors resolved

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Clean commit with mod.rs and dlg_mobile_list.rs changes

---

### Task 7: Manual Testing and Verification

**Files:**
- No file changes (manual testing only)

**Interfaces:**
- Consumes: Fully compiled application with all changes applied
- Produces: Verified functionality per success criteria

- [ ] **Step 1: Build and run the application**

Run: `cargo run --package mobile`

Expected: Application starts without crashes

- [ ] **Step 2: Test mobile filter functionality**

Manual test steps:
1. Resize window to <800px width to trigger mobile view
2. Open Debloat tab
3. Expand "Filters" collapsible section
4. Click "Recommended" button
5. Verify packages filter immediately
6. Check console logs for: `Applied category filter: recommended`
7. Click "Advanced" button
8. Verify packages filter immediately
9. Check console logs for: `Applied category filter: advanced`
10. Toggle "Show only enabled" checkbox
11. Verify packages filter immediately
12. Check console logs for: `Applied 'Show only enabled' filter: true`
13. Toggle "Hide system apps" checkbox
14. Verify packages filter immediately
15. Check console logs for: `Applied 'Hide system apps' filter: true`

Expected: All filters work immediately with log messages confirming

- [ ] **Step 3: Test text search debounce**

Manual test steps:
1. Type "com.google" in search box rapidly
2. Verify filter doesn't apply immediately (300ms debounce)
3. Wait 300ms
4. Verify filter applies
5. Check console logs for filter application

Expected: Text search still has 300ms debounce working correctly

- [ ] **Step 4: Test icon rendering**

Manual test steps:
1. Open Settings and verify renderer flags are enabled (Google Play, F-Droid, APKMirror, Android Package)
2. Go to Debloat tab in mobile view
3. Check console logs for:
   - `[DEBLOAT] Renderer flags - GP: true, FD: true, APK: true, AP: true`
   - `[DEBLOAT] render_package_list called with X filtered packages`
   - `[DEBLOAT] Got metadata for X packages`
4. Verify icons appear next to package names
5. If icons don't appear, check logs for clues

Expected: Icons render if renderer flags are enabled; diagnostic logs help debug if not

- [ ] **Step 5: Test info button**

Manual test steps:
1. In mobile view package table
2. Click info icon (ⓘ) on any package
3. Verify package details dialog opens
4. Verify correct package is shown in dialog
5. Close dialog
6. Try with 2-3 different packages

Expected: Info button opens dialog with correct package every time

- [ ] **Step 6: Test desktop view regression**

Manual test steps:
1. Resize window to >800px width to trigger desktop view
2. Test all filters work (category buttons, checkboxes)
3. Test icons render
4. Test info button works
5. Test text search debounce works

Expected: Desktop view still works perfectly (no regressions)

- [ ] **Step 7: Test mobile list dialog**

Manual test steps:
1. If mobile list dialog is accessible separately from responsive mobile view:
2. Open mobile list dialog
3. Test filters work
4. Test icons render
5. Test info button works

Expected: Mobile list dialog works with all fixes applied

- [ ] **Step 8: Document test results**

Create a test summary:
```markdown
## Manual Test Results - Mobile Debloat UI Fixes

**Date:** 2026-08-15

### Filters (Mobile View)
- [ ] Category "Recommended" applies immediately
- [ ] Category "Advanced" applies immediately
- [ ] Category "Unsafe" applies immediately
- [ ] Category "Expert" applies immediately
- [ ] Category "All" applies immediately
- [ ] "Show only enabled" checkbox applies immediately
- [ ] "Hide system apps" checkbox applies immediately
- [ ] Logs show "Applied category filter: ..." messages
- [ ] Text search debounce still works (300ms)

### Icons (Mobile View)
- [ ] Icons render when renderer flags enabled
- [ ] Logs show "[DEBLOAT] render_package_list called..."
- [ ] Logs show "[DEBLOAT] Got metadata for X packages"
- [ ] Fallback to package ID when icon unavailable

### Info Button (Mobile View)
- [ ] Info icon opens package details dialog
- [ ] Correct package shown in dialog
- [ ] Works for multiple packages

### Regression Testing (Desktop View)
- [ ] Filters still work
- [ ] Icons still render
- [ ] Info button still works
- [ ] Text search debounce still works

**Status:** PASS / FAIL
**Notes:** [Any issues or observations]
```

Expected: Test summary document created with results

- [ ] **Step 9: Final commit with test results**

```bash
# If all tests passed:
git add docs/test-results/2026-08-15-mobile-debloat-fixes.md
git commit -m "test: verify mobile debloat UI fixes

All manual tests passed:
- Filters apply immediately in mobile view
- Icons render with diagnostic logging
- Info button opens dialog correctly
- Desktop view has no regressions

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

Expected: Test results committed

---

## Self-Review Checklist

**1. Spec coverage:**
- ✅ Issue 1 (Filters not working): Tasks 1-6 implement unified filter system
- ✅ Issue 2 (Icons not rendering): Task 5 step 6 adds diagnostic logging
- ✅ Issue 3 (Info button not working): Task 5 step 5 fixes callback pattern
- ✅ Remove unused state fields: Task 1
- ✅ Filter controls call ViewModel directly: Task 2
- ✅ Remove snapshot comparison: Task 3
- ✅ Update desktop view: Task 4
- ✅ Update mobile view: Task 5
- ✅ Update callers: Task 6
- ✅ Manual testing: Task 7

**2. Placeholder scan:**
- ✅ No TBD, TODO, or "fill in details"
- ✅ All code blocks are complete
- ✅ All file paths are exact with line numbers
- ✅ All commands have expected output

**3. Type consistency:**
- ✅ `viewmodel: &crate::viewmodel::ViewModel` used consistently
- ✅ Function signatures match across all tasks
- ✅ Field names consistent (`selected_package_index`, `open`)
- ✅ Log messages use consistent format

**All checks passed - plan is complete and ready for execution.**

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-08-15-mobile-debloat-fixes.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?

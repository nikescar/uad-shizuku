# Mobile Debloat UI Fixes Design

**Date:** 2026-08-15  
**Author:** Claude Sonnet 4.5  
**Status:** Design Approved

## Overview

This spec addresses three mobile debloat UI issues:
1. **Filters not working** - Controls render but don't apply changes
2. **App icon rendering broken** - Icons worked at 6517e7f, broke after fa520e6
3. **Info button not working** - Mobile callback uses different pattern than desktop

The primary fix is a unified filter system where filter controls call `viewmodel.filter_packages()` directly, eliminating fragile snapshot comparison logic.

## Problem Statement

### Issue 1: Filters Not Working in Mobile View

**Symptoms:**
- Filter controls render correctly (category buttons, checkboxes)
- User can click/toggle controls
- No filter changes apply to package list
- No logs appear when changing filters

**Root Cause:**
Filter change detection in `TabDebloat::render()` (lines 115-147) compares current filter snapshot against `last_applied_filter`. This comparison should trigger when filters change, but no logs appear, indicating the detection logic never runs or the state isn't persisted between frames.

**Current Flow:**
```
User clicks "Recommended"
  ↓
filter_logic updates local_state.active_filter.category_filter = Some("recommended")
  ↓
Increments local_state.table_version
  ↓
Next frame: TabDebloat::render() should detect change
  ↓
❌ No logs = comparison never triggers
```

### Issue 2: App Icon Rendering Broken

**Symptoms:**
- Icons worked at commit 6517e7f
- Broke sometime after fa520e6 refactor
- Only one commit (fa520e6) touched tab_debloat between working and broken versions

**Investigation:**
- `view_mobile.rs` calls `app_metadata_renderer::prepare_app_info_for_display()` correctly
- Returns 4-tuple `(texture, title, developer, version)`
- Converted to 2-tuple `(texture, title)` at lines 149-156
- Code looks correct, but icons don't render

**Hypothesis:**
Either renderer flags aren't enabled in settings, or mobile view isn't being called from the expected location (responsive width detection vs mobile list dialog).

### Issue 3: Info Button Not Working

**Symptoms:**
- Desktop callback sets `selected_package_index` and `open` fields directly
- Mobile callback calls `package_details_dialog.open(idx)` method
- Method might not exist or not work correctly

**Location:**
- Desktop: `view_desktop.rs:348-357`
- Mobile: `view_mobile.rs:172-176`

## Design Decisions

### Decision 1: Direct ViewModel Calls vs Snapshot Comparison

**Options:**
1. Debug existing snapshot comparison logic
2. Filter controls call ViewModel directly
3. Add event system for filter changes

**Choice:** Option 2 - Direct ViewModel calls

**Rationale:**
- Eliminates fragile state comparison
- Simpler code path (easier to debug)
- Matches React/egui reactive model (action → immediate effect)
- Text search still uses debounce (performance)
- Category/checkbox filters apply immediately (no debounce needed)

### Decision 2: Keep or Remove table_version

**Choice:** Remove `table_version`

**Rationale:**
- No longer needed for cache invalidation
- Filter changes trigger ViewModel updates directly
- Reduces state complexity

### Decision 3: Icon Fix Strategy

**Choice:** Add diagnostic logging + verify renderer flags

**Rationale:**
- Code looks correct, likely configuration issue
- Low-risk investigation before code changes
- Diagnostic commit (b4a7ced) suggests this was already being debugged

## Architecture

### Unified Filter System

**New Flow:**
```
User clicks filter button
  ↓
filter_logic::render_category_filters() detects click
  ↓
Calls viewmodel.filter_packages() immediately
  ↓
ViewModel updates vm_state.filtered_packages
  ↓
Next frame: UI renders with new filtered list
```

**Key Principles:**
1. **Immediate feedback** - Filters apply on click (no delay)
2. **Text search debounce** - Still 300ms for search input (prevents excessive filtering while typing)
3. **Single source of truth** - ViewModel owns filtered_packages
4. **Stateless UI** - Filter controls read from ViewModel state

### Component Changes

**Modified:**
1. `filter_logic.rs` - Add `viewmodel` parameter to all filter functions
2. `TabDebloat::render()` - Remove snapshot comparison (lines 115-147)
3. `view_desktop.rs` - Pass `viewmodel` to filter_logic calls
4. `view_mobile.rs` - Pass `viewmodel` to filter_logic calls

**Removed:**
1. `local_state.last_applied_filter` - No longer needed
2. `local_state.table_version` - No longer needed

**Kept:**
1. `local_state.active_filter` - Still tracks current filter for UI display
2. `local_state.pending_filter_text` - Still used for text search debounce
3. `local_state.applied_filter_text` - Still used for text search debounce

## Detailed Design

### 1. Filter Logic Module Changes

**File:** `mobile/src/tab_debloat/filter_logic.rs`

**Old Signatures:**
```rust
pub fn render_category_filters(ui: &mut egui::Ui, local_state: &mut TabDebloatState)
pub fn render_options_checkboxes(ui: &mut egui::Ui, local_state: &mut TabDebloatState)
pub fn render_advanced_settings(ui: &mut egui::Ui, local_state: &mut TabDebloatState)
```

**New Signatures:**
```rust
pub fn render_category_filters(
    ui: &mut egui::Ui,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
)

pub fn render_options_checkboxes(
    ui: &mut egui::Ui,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
)

// render_advanced_settings() - No change (doesn't affect filters)
pub fn render_advanced_settings(ui: &mut egui::Ui, local_state: &mut TabDebloatState)
```

**Implementation Pattern:**
```rust
// Example: Category filter button
if ui.selectable_label(
    local_state.active_filter.category_filter.as_deref() == Some("recommended"),
    format!("Recommended ({}/{})", counts.recommended_enabled, counts.recommended),
).clicked() {
    // Update local state for UI display
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
        log::error!("Failed to apply category filter: {}", e);
    } else {
        log::debug!("Applied category filter: recommended");
    }
}
```

### 2. TabDebloat Module Changes

**File:** `mobile/src/tab_debloat/mod.rs`

**Remove Lines 115-147 (Snapshot Comparison):**
```rust
// DELETE THIS ENTIRE BLOCK:
let current_filter_snapshot = DebloatFilter {
    text_filter: self.state.applied_filter_text.clone(),
    category_filter: self.state.active_filter.category_filter.clone(),
    show_only_enabled: self.state.active_filter.show_only_enabled,
    hide_system_apps: self.state.active_filter.hide_system_apps,
};

if current_filter_snapshot != self.state.last_applied_filter {
    // ... comparison logic ...
}
```

**Keep Lines 88-113 (Text Search Debounce):**
```rust
// KEEP THIS BLOCK UNCHANGED:
if let Some(last_input_time) = self.state.last_filter_input {
    let elapsed = last_input_time.elapsed();
    if elapsed.as_millis() >= FILTER_DEBOUNCE_MS as u128
        && self.state.pending_filter_text != self.state.applied_filter_text
    {
        // Apply text filter with debounce
        // ...
    }
}
```

### 3. View Changes

**File:** `mobile/src/tab_debloat/view_desktop.rs`

**Change filter_logic calls in `render_sidebar()` (lines 74, 81, 88):**
```rust
// OLD:
filter_logic::render_category_filters(ui, local_state);
filter_logic::render_options_checkboxes(ui, local_state);

// NEW:
filter_logic::render_category_filters(ui, local_state, viewmodel);
filter_logic::render_options_checkboxes(ui, local_state, viewmodel);
```

**File:** `mobile/src/tab_debloat/view_mobile.rs`

**Change filter_logic calls in `render_filter_section()` (lines 95, 101):**
```rust
// OLD:
filter_logic::render_category_filters(ui, local_state);
filter_logic::render_options_checkboxes(ui, local_state);

// NEW:
filter_logic::render_category_filters(ui, local_state, viewmodel);
filter_logic::render_options_checkboxes(ui, local_state, viewmodel);
```

**Add `viewmodel` parameter to `render_filter_section()`:**
```rust
// OLD signature:
fn render_filter_section(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
)

// NEW signature:
fn render_filter_section(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    viewmodel: &crate::viewmodel::ViewModel,
)
```

**Update call site in `render()` (line 45):**
```rust
// OLD:
render_filter_section(ui, vm_state, local_state);

// NEW:
render_filter_section(ui, vm_state, local_state, viewmodel);
```

**But wait - `view_mobile::render()` doesn't receive `viewmodel` parameter!**

**Add `viewmodel` parameter to `view_mobile::render()`:**
```rust
// OLD signature (line 29):
pub fn render(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
)

// NEW signature:
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

**Update call site in `TabDebloat::render_mobile()` (line 227):**
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

**Update call site in `dlg_mobile_list.rs` (line 110):**
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

### 4. State Struct Changes

**File:** `mobile/src/tab_debloat/state.rs`

**Remove fields from `TabDebloatState`:**
```rust
// DELETE these fields:
pub last_applied_filter: DebloatFilter,  // Line 121 - no longer needed
pub table_version: u64,                   // Line 109 - no longer needed
```

**Update `Default` implementation (line 181):**
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

**Remove `table_version` references:**
- Search codebase for `table_version` and remove all references
- Typically used in filter_logic when incrementing (e.g., `local_state.table_version += 1;`)

### 5. Icon Rendering Fix

**File:** `mobile/src/tab_debloat/view_mobile.rs`

**Add diagnostic logging to verify function is called (line 116):**
```rust
fn render_package_list(...) {
    // Existing log at line 126-127
    log::info!("[DEBLOAT] Renderer flags - GP: {}, FD: {}, APK: {}, AP: {}",
        google_play_enabled, fdroid_enabled, apkmirror_enabled, android_package_enabled);
    
    // ADD THIS LOG to confirm function runs:
    log::info!("[DEBLOAT] render_package_list called with {} filtered packages", 
        vm_state.filtered_packages.len());
    
    // ... rest of function
}
```

**Verification Steps:**
1. Check app settings for renderer flags (Google Play, F-Droid, APKMirror, Android Package)
2. Verify logs show metadata preparation completing (line 158)
3. Check if `app_metadata` HashMap has entries (log its length)

**If icons still don't work after verification:**
- Compare desktop `prepare_app_display_data()` with mobile's use of `app_metadata_renderer`
- Desktop implementation (view_desktop.rs:102-220) might have additional caching or texture loading logic

### 6. Info Button Fix

**File:** `mobile/src/tab_debloat/view_mobile.rs`

**Change callback pattern to match desktop (lines 172-176):**
```rust
// OLD:
&mut |pkg_id| {
    if let Some(idx) = vm_state.filtered_packages.iter().position(|p| p.pkg == pkg_id) {
        local_state.package_details_dialog.open(idx);
    }
},

// NEW (match desktop pattern from view_desktop.rs:348-357):
&mut |pkg_id| {
    if let Some(idx) = vm_state.filtered_packages.iter().position(|p| p.pkg == pkg_id) {
        local_state.package_details_dialog.selected_package_index = Some(idx);
        local_state.package_details_dialog.open = true;
    }
},
```

## Error Handling

### Filter Command Failures

When `viewmodel.filter_packages()` fails:
```rust
if let Err(e) = viewmodel.filter_packages(...) {
    log::error!("Failed to apply filter: {}", e);
    // UI remains in previous state
    // No crash or panic
}
```

**Graceful Degradation:**
- User sees error in logs (if they're checking)
- Filter UI shows previous state (not the failed state)
- App remains functional

### Icon Rendering Failures

Already handled by `app_metadata_renderer`:
- Returns empty HashMap if no renderers enabled
- Logs diagnostic info (lines 126-127, 158)
- Missing icons fall back to package ID display

### Edge Cases

**Empty package list:**
- Filters should not crash
- Show "No packages" message

**All packages filtered out:**
- Show "No packages match filter" message
- Allow user to clear filters

**Renderer flags all disabled:**
- Show package IDs only (no icons)
- App remains functional

## Testing Strategy

### Manual Testing

**1. Filter Functionality (Mobile View):**
```
✓ Click "Recommended" → packages filter immediately
✓ Click "Advanced" → packages filter immediately
✓ Click "Unsafe" → packages filter immediately
✓ Click "Expert" → packages filter immediately
✓ Click "All" → all packages shown
✓ Toggle "Show only enabled" → list updates
✓ Toggle "Hide system apps" → list updates
✓ Type in search box → 300ms debounce works
✓ Check logs for "Applied category filter: ..." messages
```

**2. Icon Rendering (Mobile View):**
```
✓ Open Settings → verify renderer flags enabled
✓ Open mobile debloat view
✓ Check logs: "[DEBLOAT] render_package_list called..."
✓ Check logs: "[DEBLOAT] Got metadata for X packages"
✓ Verify icons display next to package names
✓ Verify fallback to package ID if no icon available
```

**3. Info Button (Mobile Table):**
```
✓ Click info icon on any package
✓ Verify package details dialog opens
✓ Verify correct package shown in dialog
✓ Close dialog
✓ Try different packages
```

### Regression Testing

**Desktop View:**
```
✓ Filters still work in desktop view
✓ Icons still render in desktop view
✓ Info button still works in desktop view
✓ Text search debounce still works
```

**Mobile List Dialog:**
```
✓ Open mobile list dialog (if separate from responsive mobile view)
✓ Verify filters work
✓ Verify icons render
✓ Verify info button works
```

### Edge Case Testing

```
✓ Empty package list → no crash
✓ All packages filtered out → clear message
✓ All renderer flags disabled → package IDs shown
✓ Rapid filter changes → no UI freezing
✓ Filter + search together → both apply correctly
```

## Success Criteria

**Filters:**
- ✅ Category buttons filter packages immediately in mobile view
- ✅ Checkboxes filter packages immediately in mobile view
- ✅ Logs show "Applied category filter: ..." when changed
- ✅ Desktop filters still work (no regression)

**Icons:**
- ✅ Icons render in mobile view (if renderer flags enabled)
- ✅ Logs show metadata preparation completing
- ✅ Fallback to package ID if no icon available

**Info Button:**
- ✅ Clicking info icon opens package details dialog
- ✅ Correct package shown in dialog
- ✅ Desktop info button still works (no regression)

## Migration Notes

**Breaking Changes:**
- `view_mobile::render()` signature changed (added `viewmodel` parameter)
- All `filter_logic::render_*()` signatures changed (added `viewmodel` parameter)
- `TabDebloatState` fields removed (`last_applied_filter`, `table_version`)

**Callers to Update:**
1. `tab_debloat/mod.rs` - `TabDebloat::render_mobile()`
2. `dlg_mobile_list.rs` - Mobile list dialog
3. Any other code calling `view_mobile::render()`

**Search for:**
- `table_version` - Remove all references
- `last_applied_filter` - Remove all references

## Future Improvements

**Out of Scope (Not in This Design):**
1. Unify desktop and mobile metadata preparation functions
2. Add filter presets ("My Filters" feature)
3. Persist filter state across app restarts
4. Add filter animation/transition effects

**Potential Follow-ups:**
1. Extract common metadata rendering to single shared function
2. Add filter state to ViewModel (instead of local_state)
3. Add "Clear All Filters" button
4. Add filter count badge (e.g., "Filters (3)")

## References

**Related Files:**
- `mobile/src/tab_debloat/mod.rs` - Main tab controller
- `mobile/src/tab_debloat/filter_logic.rs` - Shared filter UI
- `mobile/src/tab_debloat/view_desktop.rs` - Desktop view
- `mobile/src/tab_debloat/view_mobile.rs` - Mobile view
- `mobile/src/tab_debloat/state.rs` - Tab state struct
- `mobile/src/app_metadata_renderer.rs` - Icon rendering
- `mobile/src/dlg_mobile_list.rs` - Mobile list dialog

**Commits:**
- `6517e7f` - Last working version for icons
- `fa520e6` - Filter logic refactor (only commit between working and broken)
- `b4a7ced` - Diagnostic logging for renderer flags

**Related Issues:**
- Filter controls not applying in mobile view
- App icon rendering broken after fa520e6
- Info button not working in mobile table

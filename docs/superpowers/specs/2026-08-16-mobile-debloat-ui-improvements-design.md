# Mobile Debloat Table UI Improvements Design

**Date:** 2026-08-16  
**Author:** Claude Sonnet 4.5  
**Status:** Design Approved

## Overview

This design addresses four mobile UX issues in the debloat table:
1. Info button opens desktop dialog (broken on mobile) → Create mobile-specific info dialog
2. Close button is `✕` symbol → Change to "Close" text in title bar
3. Filter UI is collapsible and complex → Flatten to 6-line layout with all controls visible
4. Toggle and delete buttons are non-functional → Wire to actual ADB operations with confirmations

**Approach:** Incremental refactoring - modify existing mobile components in place while reusing proven desktop patterns (confirmation dialogs, ViewModel commands, ADB operations).

## Architecture Overview

### Component Structure

```
dlg_mobile_list.rs (existing - modified)
  ├── Close button moved to title bar with "Close" text
  └── Renders view_mobile.rs content

view_mobile.rs (existing - modified)  
  ├── New 6-line filter layout at top (replaces collapsible section)
  ├── Batch actions moved from bottom to line 6
  ├── Info button → opens dlg_package_info_mobile.rs (new)
  ├── Toggle button → triggers ADB enable/disable via ViewModel
  └── Delete button → shows dlg_uninstall_confirm.rs + triggers ADB uninstall

dlg_package_info_mobile.rs (new)
  ├── Single vertical-scrolling panel
  ├── Combines all data sources (Package, UAD, metadata, scans)
  └── Mobile-optimized layout (large text, touch targets)

package_table_mobile.rs (existing - modified)
  └── Wire callbacks to actual operations instead of log statements
```

### Key Principles

- Reuse existing ViewModel commands (no new business logic)
- Reuse existing dialogs (`dlg_uninstall_confirm`) where appropriate
- Keep mobile and desktop views separate (proven pattern in this codebase)
- All state management through ViewModel (no direct ADB calls from UI)

## Component Design

### 1. `dlg_mobile_list.rs` - Mobile List Dialog

**File:** `mobile/src/dlg_mobile_list.rs`

**Changes:**
- Move close button from content area (line 105) to window title bar
- Use egui's title bar controls instead of custom button
- Change button text from `✕` to `"Close"`

**Implementation:**
```rust
egui::Window::new(window_title)
    .title_bar(true)  // Keep title bar
    .show(ctx, |ui| {
        // Remove the old close button from here (line 104-108)
        // Add custom title bar with "Close" button in top-right
    });
```

**Lines affected:** 86-108

---

### 2. `view_mobile.rs` - Mobile View Layout

**File:** `mobile/src/tab_debloat/view_mobile.rs`

**Major refactoring of filter section:**

**Current:** Collapsible header with filters inside (lines 95-123)  
**New:** 6-line flattened layout at top

#### New 6-Line Layout

```
Line 1: Search: [                    ] [Clear]
Line 2: Category: Recommended(1/72)  [read-only display]
Line 3: Options [] Show only enabled [] Hide system apps 
Line 4: Advanced [] Unsafe removal [] Expert removal
Line 5: Total Packages 417 Filtered 417
Line 6: Selected: 5  [Uninstall] [Disable] [Enable] [Clear Selection] [Select All]
```

**Lines affected:** 73-123 (render_search_bar, render_filter_section), 358-396 (render_batch_actions removal), 240-264 (callback modifications)

---

### 3. `dlg_package_info_mobile.rs` - New Mobile Info Dialog

**File:** `mobile/src/dlg_package_info_mobile.rs` (NEW)

**Paired state file:** `mobile/src/dlg_package_info_mobile_stt.rs` (NEW)

**Structure:**
- Single vertical-scrolling panel
- Combines all data sources (Package Info, UAD-NG, Google Play, F-Droid, APKMirror, VirusTotal, HybridAnalysis)
- Mobile-optimized spacing and touch targets

**Files to create:**
- `mobile/src/dlg_package_info_mobile.rs`
- `mobile/src/dlg_package_info_mobile_stt.rs`

**Files to update:**
- `mobile/src/lib.rs` - Add module declarations
- `mobile/src/tab_debloat/state.rs` - Add `mobile_info_dialog` field

---

### 4. State Management

**File:** `mobile/src/tab_debloat/state.rs`

**Add new state fields:**

```rust
pub struct TabDebloatState {
    // ... existing fields ...
    
    /// Mobile package info dialog
    pub mobile_info_dialog: crate::dlg_package_info_mobile::DlgPackageInfoMobile,
    
    /// Batch toggle confirmation dialog
    pub batch_toggle_confirm: DlgBatchToggleConfirm,
}
```

**New helper struct:**

```rust
/// Simple confirmation dialog for batch toggle operations
pub struct DlgBatchToggleConfirm {
    pub open: bool,
    pub is_enabling: bool,
    pub package_ids: Vec<String>,
}
```

---

## Data Flow

### Flow 1: Opening Mobile Info Dialog

```
User taps Info button on package row
  ↓
package_table_mobile.rs: on_info_clicked callback fires
  ↓
view_mobile.rs: Find package index in filtered_packages
  ↓
local_state.mobile_info_dialog.open(index)
  ↓
dlg_package_info_mobile.rs: Reads data from ViewModelState
  ├── Package data from vm_state.filtered_packages[index]
  ├── UAD data from vm_state.uad_ng_lists
  ├── Metadata from vm_state.cached_metadata
  └── Scan results from vm_state.vt_scanner_state & ha_scanner_state
  ↓
Renders combined view with vertical scrolling
```

### Flow 2: Toggle Package (Single)

```
User taps toggle button (package currently enabled)
  ↓
package_table_mobile.rs: on_toggle_clicked(pkg_id, is_enabled=true)
  ↓
view_mobile.rs: Check if multiple packages selected
  ├── Single package → Execute immediately
  └── Multiple selected → Show confirmation dialog first
  ↓
viewmodel.send_command(DebloatCommand::DisablePackages(vec![pkg_id]))
  ↓
ViewModel → DebloatActor: Execute ADB disable command
  ↓
DebloatActor → ADB: pm disable-user --user 0 <pkg_id>
  ↓
DebloatActor emits DebloatEvent::PackageDisabled
  ↓
UI polls events and updates package state
```

### Flow 3: Delete Package

```
User taps delete button
  ↓
package_table_mobile.rs: on_delete_clicked(pkg_id)
  ↓
view_mobile.rs: Open confirmation dialog
local_state.uninstall_confirm_dialog.open_for_package(pkg_id)
  ↓
User confirms in dialog
  ↓
viewmodel.send_command(DebloatCommand::UninstallPackages(vec![pkg_id]))
  ↓
ViewModel → DebloatActor: Execute ADB uninstall
  ↓
DebloatActor → ADB: pm uninstall --user 0 <pkg_id>
  ↓
DebloatActor emits DebloatEvent::PackageUninstalled
  ↓
UI polls events and updates package list
```

### Flow 4: Batch Actions (Line 6 buttons)

```
User selects multiple packages via checkboxes
  ↓
User taps "Disable" button on line 6
  ↓
view_mobile.rs: Collect selected package IDs
  ↓
Show batch confirmation dialog (since count > 1)
  ↓
User confirms
  ↓
viewmodel.send_command(DebloatCommand::DisablePackages(selected_ids))
  ↓
ViewModel → DebloatActor: Execute batch disable
  ↓
Progress bar updates as each package is processed
  ↓
Events emitted for each completed operation
```

**Key Points:**
- All ADB operations go through ViewModel commands (no direct ADB in UI)
- Single package operations are immediate, batch operations show confirmation
- Existing progress tracking and cancellation logic is reused
- Events flow back to UI for state updates

## Error Handling

### UI Layer Errors

**Invalid package index:**
```rust
// In info button callback
if let Some(idx) = vm_state.filtered_packages.iter().position(|p| p.pkg == pkg_id) {
    local_state.mobile_info_dialog.open(idx);
} else {
    log::error!("Package {} not found in filtered list", pkg_id);
    // Graceful degradation: Do nothing, button click has no effect
}
```

**Missing metadata:**
```rust
// In dlg_package_info_mobile.rs
if let Some(uad_data) = uad_ng_lists.as_ref().and_then(|l| l.apps.get(pkg_id)) {
    render_uad_section(ui, uad_data);
} else {
    ui.label("UAD data not available");
}
```

### ViewModel Command Errors

**ADB operation failures:**
```rust
match viewmodel.send_command(DebloatCommand::DisablePackages(vec![pkg_id])) {
    Ok(()) => {
        log::info!("Disable command sent for {}", pkg_id);
    }
    Err(e) => {
        log::error!("Failed to send disable command: {}", e);
        local_state.batch_disable_state.status_message = format!("Error: {}", e);
        ui.ctx().request_repaint();
    }
}
```

### Actor Layer Errors

**ADB command failures** (handled in DebloatActor):
```rust
// Existing pattern in viewmodel/debloat.rs
match adb.disable_package(&pkg_id).await {
    Ok(()) => {
        event_tx.send(DebloatEvent::PackageDisabled(pkg_id)).await.ok();
    }
    Err(e) => {
        event_tx.send(DebloatEvent::Error(format!("Failed to disable {}: {}", pkg_id, e))).await.ok();
    }
}
```

### User-Facing Error Display

**Error banner** (existing pattern in view_mobile.rs lines 269-307):
- Shows at top of mobile view
- Displays batch operation errors
- Persists until user dismisses or starts new operation

**Error Recovery:**
- Cancel button on progress bars (existing)
- Retry mechanism for network-dependent operations (metadata fetch)
- Graceful degradation when metadata unavailable

## Testing Strategy

### Manual Testing Checklist

#### 1. Close Button
- [ ] Open mobile list dialog on mobile viewport (<1010px)
- [ ] Verify "Close" text button appears in title bar top-right
- [ ] Click close button → dialog closes
- [ ] Verify old `✕` button is removed from content area

#### 2. Filter Layout
- [ ] Open mobile list dialog
- [ ] Verify 6-line layout renders correctly
- [ ] Verify collapsible filter header is removed
- [ ] Test on small screens (360px width)
- [ ] Change category from main view → verify line 2 updates

#### 3. Info Button
- [ ] Tap info button on any package row
- [ ] Verify new mobile info dialog opens (not desktop dialog)
- [ ] Verify all available sections render
- [ ] Verify vertical scrolling works smoothly
- [ ] Test with packages that have different metadata combinations

#### 4. Toggle Button - Single Package
- [ ] Tap toggle on enabled package (not selected)
- [ ] Verify immediate disable (no confirmation dialog)
- [ ] Verify ADB command executes
- [ ] Verify package state updates in UI

#### 5. Toggle Button - Batch Operation
- [ ] Select 3+ packages via checkboxes
- [ ] Tap toggle button on any selected package
- [ ] Verify confirmation dialog appears
- [ ] Cancel → no changes
- [ ] Confirm → all selected packages toggle

#### 6. Delete Button
- [ ] Tap delete button on any package
- [ ] Verify existing `dlg_uninstall_confirm` dialog opens
- [ ] Cancel → no changes
- [ ] Confirm → package uninstalls

#### 7. Batch Actions (Line 6)
- [ ] Select multiple packages
- [ ] Verify "Selected: N" count updates
- [ ] Test all batch buttons (Uninstall, Disable, Enable, Clear, Select All)
- [ ] Test progress bar during batch operations

### Integration Testing

**State synchronization:**
- [ ] Change filter options → verify counts update
- [ ] Toggle package → verify counts update
- [ ] Select packages → verify selection count updates

**Dialog interactions:**
- [ ] Open info dialog → close → mobile list still functional
- [ ] Multiple dialogs open/close sequence → no state corruption

**Viewport responsiveness:**
- [ ] Test at 360px, 768px, 1010px widths
- [ ] Verify mobile list auto-closes when viewport exceeds 1010px

### Edge Cases

- [ ] Empty package list → UI doesn't crash
- [ ] All packages filtered out → shows "0 filtered" correctly
- [ ] Package disappears while info dialog open → graceful error
- [ ] ADB disconnects during batch operation → error displayed

### Regression Testing

- [ ] Desktop view unchanged → all features work
- [ ] Desktop info dialog (tabs) still works
- [ ] Desktop filter tree (collapsible) still works
- [ ] Existing ViewModel commands unchanged

## Files Changed

| File | Action | Why |
|---|---|---|
| `mobile/src/dlg_mobile_list.rs` | UPDATE | Move close button to title bar |
| `mobile/src/tab_debloat/view_mobile.rs` | UPDATE | Flatten filter layout, wire callbacks |
| `mobile/src/dlg_package_info_mobile.rs` | CREATE | New mobile info dialog |
| `mobile/src/dlg_package_info_mobile_stt.rs` | CREATE | State file for mobile info dialog |
| `mobile/src/tab_debloat/state.rs` | UPDATE | Add new dialog state fields |
| `mobile/src/lib.rs` | UPDATE | Register new modules |

## Success Criteria

- ✅ Info button opens mobile-optimized dialog with all metadata sections
- ✅ Close button shows "Close" text in title bar
- ✅ Filter layout is 6 lines, all controls visible
- ✅ Toggle button works immediately for single, shows confirmation for batch
- ✅ Delete button shows confirmation and executes ADB uninstall
- ✅ Batch actions moved to line 6, all functional
- ✅ Desktop view unchanged and still functional
- ✅ All tests pass

## Next Steps

1. Create implementation plan via `writing-plans` skill
2. Implement changes task-by-task
3. Test thoroughly against checklist
4. Commit changes

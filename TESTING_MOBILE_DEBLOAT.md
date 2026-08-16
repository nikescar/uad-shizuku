# Mobile Debloat Table UI - Testing Guide

## Implementation Summary

The following improvements have been implemented for the mobile debloat table view (<1010px viewport):

### 1. Info Button Fix ✅
- **Before**: Info button opened desktop view (showed nothing on mobile)
- **After**: Info button opens mobile-optimized dialog with single vertical scroll panel
- **Dialog includes**: Package info, UAD-NG data, Google Play, F-Droid, APKMirror, VirusTotal, HybridAnalysis

### 2. Close Button Text ✅
- **Before**: Close button showed "✕" symbol
- **After**: Close button shows "Close" text in custom title bar

### 3. Flattened Filter Layout ✅
- **Before**: Collapsible tree with toggle button
- **After**: 6-line flattened layout (no collapse)
  - Line 1: Search bar with Clear button
  - Line 2: Category display (read-only, shows enabled/total counts)
  - Line 3: Options checkboxes (Show only enabled, Hide system apps)
  - Line 4: Advanced checkboxes (Unsafe removal, Expert removal)
  - Line 5: Package counts (Total/Filtered)
  - Line 6: Batch action buttons (Uninstall, Disable, Enable, Clear Selection, Select All)

### 4. Functional Enable/Disable Toggle ✅
- **Single package**: Immediate toggle (no confirmation)
- **Batch operation** (2+ selected): Shows confirmation dialog
- Uses ADB commands via viewmodel.batch_enable() / batch_disable()

### 5. Functional Delete Button ✅
- **Single delete**: Shows confirmation dialog with package name
- **Batch uninstall**: Shows confirmation dialog with count
- Determines system app status from package.flags.contains("SYSTEM")
- Uses ADB commands via viewmodel.batch_uninstall()

## Manual Testing Checklist

### Prerequisites
- Android device connected via ADB or Shizuku
- Viewport width < 1010px (resize window or use mobile device)
- UAD-NG lists loaded
- Packages scanned and visible in debloat tab

### Test Cases

#### TC1: Info Button Opens Mobile Dialog
1. Navigate to Debloat tab on mobile view
2. Click info button (ⓘ) on any package card
3. **Expected**: Mobile info dialog opens with single scrollable panel
4. **Expected**: Dialog shows package info, UAD data (if available), metadata (if available)
5. Scroll through all sections
6. Click "Close" button
7. **Expected**: Dialog closes

#### TC2: Close Button Shows Text
1. Open mobile list dialog (click any category card)
2. **Expected**: Title bar shows category name and "Close" button (text, not ✕)
3. Click "Close" button
4. **Expected**: Dialog closes and returns to main view

#### TC3: Filter Layout is Flattened
1. Open mobile list dialog
2. **Expected**: No collapsible tree toggle
3. **Expected**: 6 lines visible:
   - Search bar
   - Category: {Name} ({enabled}/{total})
   - Options checkboxes
   - Advanced checkboxes
   - Total/Filtered counts
   - Batch action buttons
4. **Expected**: All filters always visible (no collapse/expand)

#### TC4: Single Package Toggle (Immediate)
1. Find an enabled package
2. Click the enable/disable toggle
3. **Expected**: Package immediately toggles (no confirmation)
4. **Expected**: Status updates in UI
5. Toggle back to original state
6. **Expected**: Package toggles immediately again

#### TC5: Batch Package Toggle (With Confirmation)
1. Select 2 or more packages using checkboxes
2. Click enable/disable toggle on one of the selected packages
3. **Expected**: Confirmation dialog appears: "Are you sure you want to enable/disable N packages?"
4. Click "Cancel"
5. **Expected**: Dialog closes, no changes
6. Click toggle again
7. Click "Confirm"
8. **Expected**: All selected packages toggle state

#### TC6: Single Package Delete (With Confirmation)
1. Find any package
2. Click the delete button (trash icon)
3. **Expected**: Confirmation dialog appears with package name
4. Click "Cancel"
5. **Expected**: Dialog closes, package still exists
6. Click delete again
7. Click "Uninstall"
8. **Expected**: Package uninstalled via ADB

#### TC7: Batch Uninstall (With Confirmation)
1. Select 3+ packages using checkboxes
2. Click "Uninstall" button on line 6 of filters
3. **Expected**: Confirmation dialog appears: "Are you sure you want to uninstall N packages?"
4. Click "Cancel"
5. **Expected**: Dialog closes, packages still exist
6. Click "Uninstall" again
7. Click "Uninstall" in dialog
8. **Expected**: All selected packages uninstalled via ADB

#### TC8: Search Filter Works
1. Type package name in search bar (line 1)
2. **Expected**: Package list filters after 300ms debounce
3. Click "Clear" button
4. **Expected**: Search text clears, all packages shown

#### TC9: Options Checkboxes Work
1. Toggle "Show only enabled" checkbox
2. **Expected**: Package list filters to enabled packages only
3. Toggle "Hide system apps" checkbox
4. **Expected**: System apps hidden from list
5. Uncheck both
6. **Expected**: All packages shown

#### TC10: Batch Action Buttons Work
1. Click "Select All"
2. **Expected**: All visible packages selected
3. **Expected**: Line 6 shows "Selected: N" with N = filtered count
4. Click "Clear Selection"
5. **Expected**: All packages deselected
6. **Expected**: Line 6 shows "Selected: 0"
7. Manually select 2 packages
8. Click "Disable" button
9. **Expected**: Confirmation dialog opens
10. Click "Enable" button (after cancel)
11. **Expected**: Confirmation dialog opens

### Regression Testing

#### R1: Desktop View Unaffected
1. Resize viewport to > 1010px
2. **Expected**: Desktop table view shows (not mobile cards)
3. **Expected**: Desktop filters show (tree with toggle)
4. **Expected**: Desktop dialogs open (not mobile dialogs)

#### R2: Package Selection State Persists
1. Select packages in mobile view
2. Toggle filters (show only enabled, hide system apps)
3. **Expected**: Selection state maintained across filter changes
4. Open mobile list dialog, close it
5. **Expected**: Selection state maintained

#### R3: Category Filter Sync
1. Click category card (e.g., "Recommended")
2. **Expected**: Mobile list dialog opens with category filter applied
3. **Expected**: Line 2 shows "Category: Recommended (X/Y)"
4. Close dialog
5. **Expected**: Category filter cleared

#### R4: Dialog Lifecycle
1. Open info dialog for package A
2. Click "Close"
3. **Expected**: Dialog closes completely
4. Open info dialog for package B
5. **Expected**: Shows package B data (not package A)
6. Open batch toggle confirmation
7. **Expected**: Info dialog closes (only one dialog at a time)

## Success Criteria

✅ All test cases (TC1-TC10) pass
✅ All regression tests (R1-R4) pass
✅ No build errors
✅ No runtime errors in console

## Files Modified

- `mobile/src/dlg_package_info_mobile.rs` - New mobile info dialog
- `mobile/src/dlg_package_info_mobile_stt.rs` - State export
- `mobile/src/dlg_mobile_list.rs` - Custom title bar with "Close" text
- `mobile/src/tab_debloat/state.rs` - DlgBatchToggleConfirm, mobile_info_dialog field
- `mobile/src/tab_debloat/view_mobile.rs` - Flattened filters, wired callbacks
- `mobile/src/lib.rs` - Module declarations

## Build Status

✅ Compiles successfully
⚠️  66 warnings (existing, unrelated)

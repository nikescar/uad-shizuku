# Manual Test Plan: Debloat Tab Refactor

**Document**: Manual Testing Guide for Tab Refactor Implementation  
**Version**: 1.0  
**Date**: 2026-08-13  
**Status**: Ready for Testing

---

## Overview

This document provides comprehensive manual testing procedures for the Debloat Tab refactor (Tasks 1-8). The refactor introduces virtual scrolling, mobile/desktop responsive views, MVVM state management, and async filtering. All manual tests should be performed on both desktop and mobile platforms.

## Prerequisites

- Build system: `cargo build` (debug mode recommended for faster iteration)
- Test device: Desktop window, Android device or emulator (if testing mobile)
- Connected device: ADB-connected Android device or Shizuku-enabled device for testing
- Sample data: At least 50 packages for testing (to see virtual scrolling effects)

## Test Environment Setup

### Desktop Testing

```bash
# Build debug binary
cd /home/wj/work/uad-shizuku
cargo build

# Run with debug logging
RUST_LOG=debug cargo run
```

### Mobile/Emulator Testing

1. Build and deploy APK to device/emulator
2. Ensure Shizuku is installed and running (Android 12+)
3. Grant Shizuku permissions to UAD-Shizuku app
4. Test on screen size <= 800px width

### Performance Profiling

```bash
# For frame rate monitoring, use egui profiler:
# Press Ctrl+P in app to toggle profiler panel

# For detailed logging:
RUST_LOG=trace cargo run 2>&1 | grep -E "filter|scroll|render"
```

---

## Test Cases

### 1. Desktop Rendering Test

**Goal**: Verify desktop layout renders correctly with sidebar and table

**Preconditions**:
- App running on desktop screen
- Window width >= 800px
- Device connected with 50+ packages

**Steps**:
1. Launch UAD-Shizuku app
2. Navigate to Debloat tab
3. Verify UI layout:
   - [ ] Left sidebar visible (approx 200px wide)
   - [ ] Right panel contains filter search bar
   - [ ] Table with columns: checkbox, name, category, status, actions
   - [ ] Table rows visible (should see multiple packages)
   - [ ] No overlapping UI elements
   - [ ] All text readable with proper font sizing
4. Scroll down in package list
   - [ ] New rows load smoothly
   - [ ] No lag or jank during scroll
   - [ ] Row height consistent at 24px
5. Verify sidebar interactivity:
   - [ ] Category filter toggles clickable
   - [ ] Options checkboxes clickable
   - [ ] Advanced options expand/collapse

**Expected Results**:
- Desktop layout displays with sidebar on left, main content on right
- No rendering errors or visual glitches
- Sidebar filters are functional
- Table content scrolls smoothly

**Failure Criteria**:
- Layout broken or misaligned
- Sidebar not visible or clickable
- Table has visual artifacts
- Frame rate drops below 30 FPS during scroll

---

### 2. Mobile Rendering Test

**Goal**: Verify mobile layout renders correctly with stacked card view

**Preconditions**:
- App running on mobile device or emulator
- Screen width <= 800px (e.g., 400px or 600px)
- Device connected with 50+ packages

**Steps**:
1. Launch UAD-Shizuku app
2. Navigate to Debloat tab
3. Verify UI layout:
   - [ ] No sidebar visible (stacked layout)
   - [ ] Filter section at top (collapsible)
   - [ ] Package list as vertical cards
   - [ ] Each card height >= 48px (touch-friendly)
   - [ ] Action buttons large enough to tap
   - [ ] Batch actions at bottom (sticky or sticky-when-scrolled)
4. Expand filter section
   - [ ] Filters expand below search bar
   - [ ] Collapse works smoothly
   - [ ] Layout reflows without jank
5. Scroll through packages
   - [ ] Cards render smoothly
   - [ ] No lag during scroll
   - [ ] Touch scroll behavior natural (not too fast/slow)
6. Tap actions on cards
   - [ ] Action buttons respond immediately
   - [ ] No visual feedback delay

**Expected Results**:
- Mobile layout displays as stacked cards
- Filter section collapses/expands smoothly
- Cards render with appropriate height for touch
- Batch actions accessible at bottom

**Failure Criteria**:
- Sidebar visible on mobile
- Cards too small to tap (< 48px height)
- Layout breaks when filter section expands
- Action buttons unresponsive

---

### 3. Responsive Transition Test (Resize Across 800px Threshold)

**Goal**: Verify UI switches smoothly between desktop and mobile layouts at 800px threshold

**Preconditions**:
- App running on desktop with resizable window
- Window initially wider than 800px (desktop layout visible)
- Device connected with 50+ packages

**Steps**:
1. Launch app with desktop layout (width > 800px)
2. Verify initial layout is desktop (sidebar visible)
3. Slowly resize window to width < 800px
   - [ ] Transition starts at width = 800px (or very close)
   - [ ] Sidebar collapses smoothly
   - [ ] Layout switches to stacked view
   - [ ] No visual artifacts or UI duplication
   - [ ] Package cards expand to fill width
4. Resize back to width > 800px
   - [ ] Sidebar reappears
   - [ ] Table layout restores
   - [ ] Package list adapts back to table view
5. Resize rapidly between 750px and 850px
   - [ ] No UI corruption during rapid resizing
   - [ ] State preserved (selected packages, filter text, scroll position)
   - [ ] Transitions remain smooth even with rapid changes

**Expected Results**:
- Smooth, jank-free layout transition at 800px threshold
- No visual artifacts or state loss during transition
- Both layouts functional after resize

**Failure Criteria**:
- Transition threshold incorrect (not at 800px)
- Temporary UI corruption during resize
- Selected packages lost after transition
- Layout doesn't respond to resize immediately

---

### 4. Virtual Scrolling Performance Test (500+ Packages)

**Goal**: Verify virtual scrolling performs well with large package lists

**Preconditions**:
- App configured to show 500+ packages (or manually create test data)
- Desktop layout active
- Profiler available (Ctrl+P for egui profiler)

**Steps**:
1. Load package list with 500+ items
2. Open egui profiler (Ctrl+P)
   - [ ] Note baseline FPS (should be 60)
   - [ ] Note memory usage baseline
3. Scroll rapidly through list (top to bottom, bottom to top)
   - [ ] Maintain 50+ FPS during scroll
   - [ ] Memory usage stable (no continuous growth)
   - [ ] No lag or frame stuttering
4. Scroll to middle, stop, scroll back
   - [ ] Scroll performance consistent
   - [ ] Previously rendered rows not re-fetched unnecessarily
5. Scroll while filtering simultaneously
   - [ ] Filter text input responsive (not blocked by scroll)
   - [ ] Scroll remains smooth during filter updates
6. Check row rendering efficiency
   - [ ] Only visible rows and buffer (±1 screen) rendered
   - [ ] Off-screen rows not consuming render time

**Performance Metrics** (target):
- **FPS**: >= 50 during active scrolling, >= 60 when idle
- **Memory**: < 100MB increase even with 1000+ packages
- **Render Time**: < 5ms per frame during scroll

**Expected Results**:
- Large lists scroll smoothly at 60 FPS
- Virtual scrolling renders only visible rows
- Memory usage remains stable
- No performance degradation with large datasets

**Failure Criteria**:
- FPS drops below 30 during scroll
- Memory grows unbounded (> 200MB increase)
- Visible lag or stutter during scroll
- Rows render even when off-screen

---

### 5. Filter Debouncing Test

**Goal**: Verify filter input debounces at 300ms and updates UI responsively

**Preconditions**:
- App running with package list (50+ packages)
- Debug logging enabled to observe filter events
- Desktop layout active

**Steps**:
1. Navigate to filter search input
2. Type slowly: "facebook" (one character per 100ms)
   - [ ] Filter debounce timer starts on first character
   - [ ] No search initiated before full word typed
   - [ ] Search runs exactly once after 300ms of inactivity
   - [ ] Results update immediately after search
3. Verify no excessive searches:
   - [ ] Log shows exactly 1 "FilterPackages" command for the word "facebook"
   - [ ] No duplicate filter events
4. Type quickly: "amazon alarm" (typing very fast)
   - [ ] Text appears in input immediately
   - [ ] Filter doesn't update mid-typing
   - [ ] After 300ms silence, filter updates once
5. Clear filter (select all, delete)
   - [ ] Debounce applies
   - [ ] List resets after 300ms silence
6. Test edge case: type, wait 200ms, type again
   - [ ] Debounce timer resets when new character typed
   - [ ] Filter only runs after full 300ms silence
7. Verify UI feedback:
   - [ ] Search input shows typed text immediately (client-side)
   - [ ] Result count updates after filter completes
   - [ ] No spinner/loading state for simple filters (< 100ms)

**Expected Results**:
- Filter debounces at 300ms between keystrokes
- Text input responsive (no input lag)
- Search runs exactly once per filter change
- Results update immediately after search

**Failure Criteria**:
- Multiple filter commands sent for single input change
- Input field lags (not showing typed text immediately)
- Filter updates before 300ms elapsed
- Search runs on every keystroke (no debounce)

---

### 6. Batch Selection Test

**Goal**: Verify batch selection works correctly with updated MVVM state

**Preconditions**:
- App running with 50+ packages
- Desktop or mobile layout

**Steps**:

#### Part A: Select/Deselect Individual Items
1. Click checkbox on first package
   - [ ] Checkbox marks as selected
   - [ ] Package row highlights (visual feedback)
   - [ ] Selection stored in ViewModelState
2. Click another checkbox
   - [ ] Previous selection remains
   - [ ] Second package also marked selected
3. Click selected checkbox again
   - [ ] Deselects successfully
   - [ ] Row highlighting removed
4. Verify scroll preserves selection:
   - [ ] Scroll down to new packages
   - [ ] Previous selections remain checked
   - [ ] Scroll back up, selections still present

#### Part B: Select All / Deselect All
1. Find "Select All" button/toggle
   - [ ] Exists and is clickable
2. Click "Select All"
   - [ ] All visible packages marked selected
   - [ ] Checkboxes show selected state
   - [ ] Count updates (e.g., "100 selected")
3. Scroll down
   - [ ] New packages also marked selected
4. Click "Deselect All"
   - [ ] All selections cleared
   - [ ] All checkboxes unmarked
   - [ ] Count resets to 0

#### Part C: Batch Actions
1. Select 5 packages
2. Click batch action button (Uninstall/Disable/Enable)
   - [ ] Modal or confirmation dialog appears
   - [ ] Dialog shows count of selected packages
   - [ ] Confirm button enabled
3. Confirm action
   - [ ] Progress bar or status appears
   - [ ] Operation starts (packages shown as processing)
   - [ ] Selection remains visible during operation
4. Complete operation
   - [ ] Results shown (succeeded/failed count)
   - [ ] Selections cleared
   - [ ] List updates to reflect changes

#### Part D: Selection with Filtering
1. Select packages: A, B, C
2. Type filter "test" (filters list)
   - [ ] Previously selected packages remain selected if they match filter
   - [ ] If selected packages don't match filter, they stay selected (checkbox still checked even if hidden)
3. Clear filter
   - [ ] Original selections still present
4. Re-apply filter
   - [ ] Filtered selections still present

**Expected Results**:
- Individual selections work correctly
- Select All/Deselect All works
- Selections persist across scroll and resize
- Batch actions operate on selected packages
- Selections preserved when filtering

**Failure Criteria**:
- Selections lost after scroll or resize
- Select All selects only visible items, not all
- Batch action fails or operates on wrong items
- Filtering loses selection state

---

### 7. Error Display Test

**Goal**: Verify error handling and display works correctly

**Preconditions**:
- App running
- Network available (for testing API errors)
- Device with some failing operations (simulate or use real failure scenarios)

**Steps**:

#### Part A: Operation Error Banner
1. Select a package and attempt uninstall on a protected system package
   - [ ] Error banner appears at top of tab
   - [ ] Error message readable and specific
   - [ ] Error includes package name or reason
2. Dismiss banner
   - [ ] Click X or wait for auto-dismiss
   - [ ] Banner removed smoothly
3. Attempt another operation
   - [ ] New error banner appears if operation fails

#### Part B: Network/API Errors
1. (If applicable) Disconnect network or mock API error
2. Attempt operation that requires network
   - [ ] Error message appears
   - [ ] Message indicates network issue (not generic)
   - [ ] Includes retry option if appropriate
3. Fix network issue
4. Retry operation
   - [ ] Operation proceeds successfully
   - [ ] Error banner clears

#### Part C: State Handling During Errors
1. Select packages (A, B, C)
2. Trigger error during batch operation
   - [ ] Partial results handled gracefully
   - [ ] Selections remain for retry
   - [ ] State machine shows error state (not running, not complete)
3. View error details if available
   - [ ] Toast or dialog shows which packages failed
   - [ ] Reason for failure provided
4. Retry failed packages
   - [ ] Can retry without re-selecting

#### Part D: Error Recovery
1. Trigger error by uninstalling package while operation in progress
   - [ ] Error caught and displayed
   - [ ] App doesn't crash or freeze
   - [ ] Tab remains usable
2. Continue using app
   - [ ] Can perform other operations
   - [ ] No lingering error state

**Expected Results**:
- Errors display clearly and timely
- Error messages are informative
- Error state doesn't block UI
- Recovery actions available (retry, dismiss)
- App remains stable after errors

**Failure Criteria**:
- Generic error messages (e.g., "Error")
- Error banner persists even after fix
- App crashes or freezes on error
- Error state prevents further operations
- No indication of which packages failed

---

## Integration Test Checklist

Use these tests to verify the full refactor works end-to-end:

- [ ] **Task 1 (ViewModel)**: Filter command sent and event received
- [ ] **Task 2 (State Module)**: TabDebloatState initializes correctly
- [ ] **Task 3 (Virtual Table)**: Table renders 500+ packages smoothly
- [ ] **Task 4 (Desktop View)**: Desktop layout with sidebar works
- [ ] **Task 5 (Mobile View)**: Mobile layout with cards works
- [ ] **Task 6 (Debouncing)**: Filter debounces at 300ms
- [ ] **Task 7 (Integration Test)**: `cargo test` passes all new tests
- [ ] **Task 8 (App Integration)**: App uses new TabDebloat module
- [ ] **Task 9 (This Document)**: Manual tests all pass

---

## Performance Baseline

Record these metrics before and after refactor:

| Metric | Before | After | Notes |
|--------|--------|-------|-------|
| Initial render time (50 packages) | TBD ms | TBD ms | Measure from app launch to first frame |
| Scroll FPS (500 packages) | TBD | TBD | Average FPS during fast scroll |
| Memory usage (500 packages) | TBD MB | TBD MB | Peak RSS during operation |
| Filter latency (typing "facebook") | TBD ms | TBD ms | Time from keystroke to filter update |
| Resize latency (800px threshold) | TBD ms | TBD ms | Time from resize to layout change |
| Batch operation (100 items) | TBD ms | TBD ms | Time to complete 100 package operations |

---

## Known Issues and Workarounds

### Issue 1: Android Lifecycle
- **Status**: Known issue
- **Symptom**: App freezes after device sleep
- **Workaround**: Restart app from task switcher
- **Fix**: Planned for future session

### Issue 2: Virtual Scrolling on Mobile
- **Status**: Improved in this refactor
- **Symptom**: Previous: severe lag with 100+ packages
- **Fix**: Now using egui_extras::TableBuilder with virtual rows
- **Expected**: 60 FPS maintained with 500+ packages

---

## Browser and Device Compatibility

### Desktop Browsers (WASM)
- Chrome/Edge: Supported
- Firefox: Supported
- Safari: Not fully supported (Diesel WASM limitations)

### Mobile Platforms
- Android 12+: Fully supported
- Android 11: Should work, limited Shizuku features
- iOS: Not applicable (Android-only app)

---

## Regression Tests

After manual testing, run automated regression suite:

```bash
# All tests (including manual test markers)
cargo test --test '*' --features integration_tests

# Specific test suite
cargo test --test debloat_tab_integration_test

# With coverage
cargo llvm-cov --html

# Performance benchmarks
cargo bench --bench tab_refactor_bench
```

---

## Sign-Off Checklist

**Tester Name**: ________________  
**Date**: ________________  
**Platform**: [ ] Desktop [ ] Mobile [ ] Both

- [ ] Desktop rendering test passed
- [ ] Mobile rendering test passed
- [ ] Responsive transition test (800px) passed
- [ ] Virtual scrolling (500+ packages) passed at 50+ FPS
- [ ] Filter debouncing (300ms) verified
- [ ] Batch selection (select all, actions) passed
- [ ] Error display (banner, recovery) passed
- [ ] Integration tests all pass (`cargo test`)
- [ ] No regressions from previous behavior
- [ ] Performance metrics acceptable

**Issues Found** (if any):
```
1. [Issue Description]
   - Steps to reproduce:
   - Expected behavior:
   - Actual behavior:

2. [Additional issues...]
```

**Comments**:
```
[Any additional observations or notes]
```

---

## Next Steps After Testing

1. **If all tests pass**:
   - Merge feature branch to main
   - Update release notes
   - Plan Phase 2 (Scan/Apps tabs refactor)

2. **If issues found**:
   - File issues in tracker
   - Create targeted fixes
   - Re-test specific areas
   - Document workarounds if needed

3. **For future phases**:
   - Apply same test plan to Scan tab refactor
   - Apply to Apps tab refactor
   - Consider additional performance benchmarks

---

## References

- **Tab Refactor Design**: `/docs/superpowers/specs/2026-08-13-tab-refactor-design.md`
- **Tab Refactor Plan**: `/docs/superpowers/plans/2026-08-13-tab-refactor.md`
- **MVVM Architecture**: `/docs/mvvm-actor-migration-complete.md`
- **egui Virtual Scrolling**: https://docs.rs/egui_extras/latest/egui_extras/struct.TableBuilder.html
- **Performance Profiling**: Ctrl+P in app to open egui profiler

---

**Document Version History**:
- v1.0 (2026-08-13): Initial comprehensive manual test plan for Tasks 1-8

# Tab Controls Refactor - MVVM with Mobile/Desktop Separation

**Date:** 2026-08-13  
**Author:** Claude Sonnet 4.5  
**Status:** Design Approved

## Executive Summary

Refactor all tab controls (debloat, scan, apps) to:
1. **Fix lag** - Virtual scrolling + async filtering eliminates stuttering and UI freezes
2. **Separate views** - Distinct mobile/desktop layouts based on screen width
3. **Strict MVVM** - Pure rendering views, all logic in ViewModel/Actors
4. **Migrate SharedStore** - Move business state to ViewModel, rename SharedStore → TextureCache

## Goals

- ✅ **Performance**: 60 FPS scrolling, non-blocking filters/sorts
- ✅ **Maintainability**: Smaller files (200-400 lines vs 2,500+), clear separation
- ✅ **Responsive**: Dynamic mobile/desktop layouts based on `DESKTOP_MIN_WIDTH` (800px)
- ✅ **Architecture**: Complete MVVM with command/event patterns

## Current State Analysis

### Problems

1. **Tab files are huge:**
   - `tab_debloat_control.rs`: 2,592 lines
   - `tab_scan_control.rs`: 2,911 lines
   - `tab_apps_control.rs`: 1,680 lines

2. **Debloat tab lag:**
   - **Scrolling stutters** - Rendering all ~500+ packages every frame
   - **Filtering freezes UI** - Synchronous filtering blocks rendering

3. **No mobile/desktop separation:**
   - Same layout regardless of screen size
   - No touch-optimized UI for mobile

4. **SharedStore still used:**
   - `installed_packages` accessed directly
   - `uad_ng_lists` mutated from tabs
   - Mixed with ViewModel patterns

### Current Architecture (After MVVM Migration)

MVVM migration completed June 2026:
- ViewModel with command/event pattern exists
- Actors: DebloatActor, ScanActor, AppsActor, MetadataActor
- State centralized in ViewModelState
- SharedStore contains legacy business state + textures

## Design Overview

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    UadShizukuApp (Main App)                      │
│                                                                   │
│  ┌────────────────────────────────────────────────────────┐     │
│  │            Global ViewModel (Existing)                  │     │
│  │  • State: ViewModelState (packages, uad_lists, etc.)   │     │
│  │  • Actors: DebloatActor, ScanActor, AppsActor, etc.    │     │
│  │  • Commands/Events: async background processing        │     │
│  └────────────────────────────────────────────────────────┘     │
│                           │                                       │
│                           ▼                                       │
│  ┌────────────────────────────────────────────────────────┐     │
│  │              Tab Controllers (Refactored)               │     │
│  │                                                         │     │
│  │  TabDebloat::render(ui, &viewmodel) {                 │     │
│  │    if ui.available_width() >= DESKTOP_MIN_WIDTH {     │     │
│  │      view_desktop::render(ui, &viewmodel.state)       │     │
│  │    } else {                                            │     │
│  │      view_mobile::render(ui, &viewmodel.state)        │     │
│  │    }                                                   │     │
│  │  }                                                     │     │
│  └────────────────────────────────────────────────────────┘     │
│         │                        │                                │
│         ▼                        ▼                                │
│  ┌─────────────┐          ┌─────────────┐                       │
│  │view_mobile  │          │view_desktop │                       │
│  │• Stacked    │          │• Multi-col  │                       │
│  │• Simplified │          │• Sidebar    │                       │
│  │• Touch UI   │          │• Full table │                       │
│  └─────────────┘          └─────────────┘                       │
│         │                        │                                │
│         └────────────────────────┘                                │
│                    │                                              │
│                    ▼                                              │
│  ┌────────────────────────────────────────────────────────┐     │
│  │         Shared Components (Performance-Optimized)       │     │
│  │  • VirtualPackageTable (row virtualization)            │     │
│  │  • AsyncFilterPanel (background filtering)             │     │
│  │  • BatchActionBar (progress tracking)                  │     │
│  └────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────┘
```

### File Structure (Per Tab)

Each tab (debloat, scan, apps) will be refactored into:

```
mobile/src/tab_debloat/
├── mod.rs                    # Entry point, width detection, routing
├── view_mobile.rs            # Mobile layout implementation
├── view_desktop.rs           # Desktop layout implementation
├── state.rs                  # UI-specific state (filters, selection, dialogs)
└── components/
    ├── package_table.rs      # Virtual scrolling table component
    ├── filter_panel.rs       # Filter controls (mobile/desktop variants)
    └── action_bar.rs         # Batch action toolbar

mobile/src/tab_scan/
├── mod.rs
├── view_mobile.rs
├── view_desktop.rs
├── state.rs
└── components/
    └── scan_results_table.rs

mobile/src/tab_apps/
├── mod.rs
├── view_mobile.rs
├── view_desktop.rs
├── state.rs
└── components/
    └── apps_browser.rs
```

### State Migration

**Move from SharedStore to ViewModel:**
- `installed_packages` → `ViewModel.state.packages`
- `uad_ng_lists` → `ViewModel.state.uad_ng_lists`

**Rename SharedStore:**
- `SharedStore` → `TextureCache` (only contains `egui::TextureHandle`)

**Tab-local state stays in tabs:**
- UI-specific state (selected rows, dialog visibility, sort column) remains in `tab_debloat/state.rs`
- Business state (packages, scan results) lives in global `ViewModel`

## Lag Fixes - Virtual Scrolling & Async Filtering

### Problem Analysis

Current debloat tab lag sources:
1. **Scrolling stutters** - Rendering all ~500+ packages every frame
2. **Filtering freezes UI** - Synchronous filtering on main thread blocks rendering

### Solution 1: Virtual Scrolling

Use `egui_extras::TableBuilder` with row virtualization - only render visible rows.

**Before (current):**
```rust
// Renders ALL packages every frame
for package in filtered_packages.iter() {
    ui.horizontal(|ui| {
        ui.label(&package.name);
        ui.label(&package.version);
        // ... 10+ columns
    });
}
```

**After (virtualized):**
```rust
use egui_extras::{TableBuilder, Column};

TableBuilder::new(ui)
    .striped(true)
    .column(Column::auto())  // Package name
    .column(Column::auto())  // Version
    // ... more columns
    .body(|mut body| {
        body.rows(ROW_HEIGHT, filtered_packages.len(), |mut row| {
            let idx = row.index();
            let package = &filtered_packages[idx];
            
            row.col(|ui| { ui.label(&package.name); });
            row.col(|ui| { ui.label(&package.version); });
            // ... render only visible row
        });
    });
```

**Performance improvement:** 500 packages → render only ~20 visible rows per frame.

### Solution 2: Async Filtering/Sorting

Move filtering and sorting to background actor, send results via events.

**Data flow:**
```
1. User types in filter box
   ↓
2. UI sends FilterCommand { text: "google", category: Recommended }
   ↓
3. DebloatActor receives command on background thread
   ↓
4. Actor filters packages (non-blocking)
   ↓
5. Actor emits FilteredPackagesUpdated(Vec<PackageFingerprint>)
   ↓
6. UI polls events, updates filtered_packages
   ↓
7. Table re-renders with new filtered list (virtualized)
```

**Implementation:**

```rust
// In viewmodel/debloat.rs
pub enum DebloatCommand {
    FilterPackages { text: String, category: Option<DebloatFilter> },
    SortPackages { column: SortColumn, ascending: bool },
    // ... existing commands
}

pub enum ViewModelEvent {
    FilteredPackagesReady(Vec<PackageFingerprint>),
    // ... existing events
}

// In DebloatActor
async fn handle_filter_command(&mut self, text: String, category: Option<DebloatFilter>) {
    // Background filtering - doesn't block UI
    let filtered = self.packages.iter()
        .filter(|p| {
            let matches_text = text.is_empty() || p.pkg.contains(&text);
            let matches_category = category.map_or(true, |c| self.check_category(p, c));
            matches_text && matches_category
        })
        .cloned()
        .collect();
    
    self.event_tx.send(ViewModelEvent::FilteredPackagesReady(filtered)).await.ok();
}
```

### Debouncing

Add debouncing to filter text input - wait 300ms after user stops typing before filtering.

```rust
// In tab state
last_filter_input: Instant,
pending_filter_text: String,

// In update loop
if ui.text_edit_singleline(&mut self.pending_filter_text).changed() {
    self.last_filter_input = Instant::now();
}

// Check if debounce period elapsed
if self.last_filter_input.elapsed() > Duration::from_millis(300) {
    if self.pending_filter_text != self.applied_filter_text {
        viewmodel.send_command(DebloatCommand::FilterPackages {
            text: self.pending_filter_text.clone(),
            category: self.active_filter,
        });
        self.applied_filter_text = self.pending_filter_text.clone();
    }
}
```

**Expected results:**
- ✅ Scrolling: 60 FPS (only ~20 rows rendered)
- ✅ Filtering: UI stays responsive (300ms debounce + background processing)
- ✅ Sorting: Instant UI feedback, results arrive async

## Mobile vs Desktop View Differences

### Width Detection

```rust
const DESKTOP_MIN_WIDTH: f32 = 800.0; // Already defined in codebase

// In tab's render method
pub fn render(&mut self, ui: &mut Ui, viewmodel: &ViewModel) {
    if ui.available_width() >= DESKTOP_MIN_WIDTH {
        view_desktop::render(ui, &viewmodel.state, &mut self.state);
    } else {
        view_mobile::render(ui, &viewmodel.state, &mut self.state);
    }
}
```

### Layout Differences

#### Debloat Tab - Desktop View

```
┌─────────────────────────────────────────────────────────────┐
│ [Device Selector ▼]  [Search: ___________]  [Batch Actions] │
├──────────────┬──────────────────────────────────────────────┤
│  Filters     │  Package Table (Virtual Scrolling)           │
│              │                                               │
│ ☑ All        │ ☑ │ Name      │ Category  │ Status │ Actions│
│ ☐ Recommended│───┼───────────┼───────────┼────────┼────────│
│ ☐ Advanced   │ ☐ │ com.goo.. │ Recommend │ ✓ En.. │ [ℹ][×]│
│ ☐ Unsafe     │ ☐ │ com.sam.. │ Advanced  │ ✗ Dis..│ [ℹ][×]│
│ ☐ Unlisted   │ ☐ │ com.fac.. │ Unsafe    │ ✓ En.. │ [ℹ][×]│
│              │   │ ... (virtualized, only visible rows)     │
│ Options:     │                                               │
│ ☐ System only│                                               │
│ ☐ Enabled only│                                              │
└──────────────┴──────────────────────────────────────────────┘
```

**Key features:**
- Sidebar filters (left, 200px fixed width)
- Full table with all columns visible
- Compact row height (~24px)
- Hover tooltips for truncated text
- Keyboard shortcuts (Ctrl+F for filter, Delete for uninstall)

#### Debloat Tab - Mobile View

```
┌───────────────────────────────┐
│ [Device Selector ▼]           │
│ [Search: ___________]         │
│ ┌─────────────────────────┐   │
│ │ ▼ Filters              │   │
│ │ ☑ All  ☐ Recommended   │   │
│ │ ☐ Advanced  ☐ Unsafe   │   │
│ └─────────────────────────┘   │
│                               │
│ ┌─────────────────────────┐   │
│ │ ☐ com.google.android... │   │
│ │   Recommended • Enabled │   │
│ │   [Info] [Uninstall]    │   │
│ ├─────────────────────────┤   │
│ │ ☐ com.samsung.knox...   │   │
│ │   Advanced • Disabled   │   │
│ │   [Info] [Uninstall]    │   │
│ └─────────────────────────┘   │
│                               │
│ [Batch Uninstall Selected]    │
└───────────────────────────────┘
```

**Key features:**
- Stacked layout (no sidebar)
- Collapsible filter section
- Card-based package list (one per row)
- Larger tap targets (48px min)
- Essential info only (name, category, status)

### Responsive Transitions

Views switch dynamically when window resizes:
- Desktop → Mobile: Collapse filters into dropdown, expand cards
- Mobile → Desktop: Expand filters to sidebar, compact into table

**State preserved during transition:**
- Selected packages
- Filter settings
- Scroll position (reset to top on transition)
- Open dialogs

### Shared Components (Both Views Use)

```rust
// components/package_table.rs
pub fn render_virtual_table(
    ui: &mut Ui,
    packages: &[PackageFingerprint],
    selected: &mut HashSet<String>,
    mode: TableMode, // Desktop vs Mobile
) {
    match mode {
        TableMode::Desktop => render_compact_table(ui, packages, selected),
        TableMode::Mobile => render_card_list(ui, packages, selected),
    }
}
```

## Data Flow & Command/Event Patterns

### Strict MVVM Boundaries

**Rule:** Tab views are PURE rendering - they only:
1. Read from `ViewModel.state` (immutable reference)
2. Send commands to `ViewModel`
3. Manage local UI state (scroll position, dialog visibility)

**No direct business logic in views.**

### Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         User Action                          │
│              (click uninstall, type filter, etc.)            │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Tab View (Pure UI)                        │
│  • Capture user input                                        │
│  • Send command to ViewModel                                 │
│  • Read state for rendering                                  │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ viewmodel.send_command(...)
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   ViewModel (Coordinator)                    │
│  • Receives command on command channel                       │
│  • Routes to appropriate actor                               │
│  • Polls events from actors                                  │
│  • Updates ViewModelState                                    │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ send command via channel
                           ▼
┌─────────────────────────────────────────────────────────────┐
│            Actor (Background Thread - smol runtime)          │
│  • Receives command                                          │
│  • Performs async work (filter, API call, DB query)         │
│  • Emits event with results                                  │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ emit event via event_tx
                           ▼
┌─────────────────────────────────────────────────────────────┐
│              ViewModel.poll_events()                         │
│  • Receives event                                            │
│  • Updates ViewModelState                                    │
│  • UI reads updated state on next frame                      │
└─────────────────────────────────────────────────────────────┘
                           │
                           │ state updated
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Tab View Re-renders                       │
│  • Reads updated state                                       │
│  • Renders new UI                                            │
└─────────────────────────────────────────────────────────────┘
```

### Command Examples

```rust
// In viewmodel/debloat.rs
pub enum DebloatCommand {
    LoadPackages { device_id: String },
    LoadUadLists,
    FilterPackages { text: String, category: Option<DebloatFilter> },
    SortPackages { column: SortColumn, ascending: bool },
    UninstallPackage { package_name: String },
    BatchUninstall { package_names: Vec<String> },
    DisablePackage { package_name: String },
    EnablePackage { package_name: String },
}
```

### Event Examples

```rust
// In viewmodel/common.rs (existing ViewModelEvent enum)
pub enum ViewModelEvent {
    // Existing events...
    PackagesLoaded(Vec<PackageFingerprint>),
    UadListsLoaded(UadNgLists),
    
    // New events for refactor
    FilteredPackagesReady(Vec<PackageFingerprint>),
    SortedPackagesReady(Vec<PackageFingerprint>),
    PackageUninstalled { package_name: String, success: bool },
    BatchUninstallProgress { completed: usize, total: usize },
    BatchUninstallComplete { succeeded: Vec<String>, failed: Vec<String> },
}
```

### State Updates (ViewModel Side)

```rust
// In viewmodel/mod.rs
impl ViewModel {
    pub fn poll_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                ViewModelEvent::PackagesLoaded(packages) => {
                    self.state.packages = packages;
                }
                ViewModelEvent::UadListsLoaded(lists) => {
                    self.state.uad_ng_lists = Some(lists);
                }
                ViewModelEvent::FilteredPackagesReady(filtered) => {
                    self.state.filtered_packages = filtered;
                }
                // ... handle all events
            }
        }
    }
}
```

### State Reading (View Side)

```rust
// In tab_debloat/view_desktop.rs
pub fn render(ui: &mut Ui, vm_state: &ViewModelState, local_state: &mut TabDebloatState) {
    // Read global state (immutable)
    let packages = &vm_state.filtered_packages; // or .packages if no filter
    let uad_lists = vm_state.uad_ng_lists.as_ref();
    
    // Read local UI state (mutable)
    let selected = &mut local_state.selected_packages;
    
    // Render table
    render_package_table(ui, packages, uad_lists, selected);
}
```

### SharedStore Migration

**Before:**
```rust
// tab_debloat_control.rs (current)
let store = get_shared_store();
store.set_installed_packages(packages); // ❌ Direct mutation
let packages = store.get_installed_packages(); // ❌ Direct access
```

**After:**
```rust
// tab_debloat/mod.rs (refactored)
viewmodel.send_command(DebloatCommand::LoadPackages { device_id }); // ✅ Command
let packages = &viewmodel.state.packages; // ✅ Read-only access
```

**SharedStore cleanup:**
```rust
// shared_store.rs → texture_cache.rs
pub struct TextureCache {
    // Only egui::TextureHandle remains
    pub icon_cache: HashMap<String, TextureHandle>,
    pub app_icon_cache: HashMap<String, TextureHandle>,
}

// Remove all business data fields
// ❌ installed_packages - moved to ViewModel
// ❌ uad_ng_lists - moved to ViewModel
// ❌ cached_metadata - moved to ViewModel (already done)
```

## Error Handling & User Feedback

### Error Types

```rust
// In viewmodel/common.rs
#[derive(Debug, Clone)]
pub enum AppError {
    // Device/ADB errors
    DeviceNotConnected,
    AdbCommandFailed { command: String, error: String },
    
    // Package operation errors
    PackageNotFound { package_name: String },
    UninstallFailed { package_name: String, reason: String },
    
    // Data loading errors
    UadListsLoadFailed { error: String },
    PackageListLoadFailed { error: String },
    
    // Filtering/sorting errors (should be rare)
    FilteringFailed { error: String },
}

// Events can carry errors
pub enum ViewModelEvent {
    // ... existing events
    Error(AppError),
}
```

### Error Display (UI)

```rust
// In tab state
pub struct TabDebloatState {
    // ... existing fields
    error_message: Option<String>,
    error_timestamp: Option<Instant>,
}

// In view rendering
pub fn render(ui: &mut Ui, vm_state: &ViewModelState, local_state: &mut TabDebloatState) {
    // Show error banner at top if present
    if let Some(error) = &local_state.error_message {
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::RED, "⚠");
            ui.label(error);
            if ui.button("Dismiss").clicked() {
                local_state.error_message = None;
            }
        });
    }
    
    // Auto-dismiss after 5 seconds
    if let Some(timestamp) = local_state.error_timestamp {
        if timestamp.elapsed() > Duration::from_secs(5) {
            local_state.error_message = None;
            local_state.error_timestamp = None;
        }
    }
    
    // ... rest of UI
}
```

### Progress Feedback

**For long operations:**

```rust
// In ViewModelState
pub struct ViewModelState {
    // ... existing fields
    pub operation_in_progress: Option<OperationProgress>,
}

#[derive(Clone)]
pub struct OperationProgress {
    pub operation_type: OperationType,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

pub enum OperationType {
    BatchUninstall,
    LoadingPackages,
    FilteringPackages,
    LoadingUadLists,
}
```

**Progress UI:**

```rust
// In view rendering
if let Some(progress) = &vm_state.operation_in_progress {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(&progress.message);
        ui.label(format!("{}/{}", progress.current, progress.total));
        
        // Progress bar
        let progress_fraction = progress.current as f32 / progress.total as f32;
        ui.add(egui::ProgressBar::new(progress_fraction));
    });
}
```

### Loading States

```rust
// In ViewModelState
pub enum DataState<T> {
    NotLoaded,
    Loading,
    Loaded(T),
    Error(String),
}

pub struct ViewModelState {
    pub packages: DataState<Vec<PackageFingerprint>>,
    pub uad_ng_lists: DataState<UadNgLists>,
    // ...
}
```

**UI handles each state:**

```rust
match &vm_state.packages {
    DataState::NotLoaded => {
        ui.label("No device connected");
    }
    DataState::Loading => {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Loading packages...");
        });
    }
    DataState::Loaded(packages) => {
        render_package_table(ui, packages, ...);
    }
    DataState::Error(err) => {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
        if ui.button("Retry").clicked() {
            // Send LoadPackages command
        }
    }
}
```

### Cancellation Support

```rust
// In DebloatActor
async fn handle_batch_uninstall(&mut self, package_names: Vec<String>) {
    let total = package_names.len();
    
    for (idx, package_name) in package_names.iter().enumerate() {
        // Check cancellation
        if self.cancelled.load(Ordering::Relaxed) {
            self.event_tx.send(ViewModelEvent::BatchUninstallCancelled).await.ok();
            self.cancelled.store(false, Ordering::Relaxed);
            return;
        }
        
        // Send progress update
        self.event_tx.send(ViewModelEvent::BatchUninstallProgress {
            completed: idx,
            total,
        }).await.ok();
        
        // Perform uninstall
        let result = self.uninstall_package(package_name).await;
        
        // ... handle result
    }
}
```

**UI cancel button:**

```rust
if let Some(progress) = &vm_state.operation_in_progress {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(&progress.message);
        
        if ui.button("Cancel").clicked() {
            viewmodel.send_command(DebloatCommand::CancelBatchOperation);
        }
    });
}
```

## Testing Strategy

### Test Coverage Goals

Following ECC common/testing.md:
- **Minimum 80% coverage** (enforced via `cargo llvm-cov --fail-under-lines 80`)
- **Unit tests** for components and utilities
- **Integration tests** for ViewModel/Actor flows
- **Manual UI testing** (egui limitations)

### Unit Tests

**Component tests (in same file):**

```rust
// In tab_debloat/components/package_table.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_packages_by_text() {
        // Arrange
        let packages = vec![
            create_test_package("com.google.android.gms"),
            create_test_package("com.samsung.knox"),
        ];
        
        // Act
        let filtered = filter_packages(&packages, "google", None);
        
        // Assert
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].pkg, "com.google.android.gms");
    }
    
    #[test]
    fn test_sort_packages_by_name() {
        // Arrange
        let mut packages = vec![
            create_test_package("zzz"),
            create_test_package("aaa"),
        ];
        
        // Act
        sort_packages(&mut packages, SortColumn::Name, true);
        
        // Assert
        assert_eq!(packages[0].pkg, "aaa");
        assert_eq!(packages[1].pkg, "zzz");
    }
}
```

### Integration Tests

**Actor command/event flows:**

```rust
// In mobile/tests/integration/debloat_refactor_test.rs
use mobile::viewmodel::{ViewModel, DebloatCommand, ViewModelEvent};

#[test]
fn test_filter_packages_command_updates_state() {
    smol::block_on(async {
        // Arrange
        let mut viewmodel = ViewModel::new().await;
        
        // Load initial packages
        viewmodel.send_command(DebloatCommand::LoadPackages {
            device_id: "test_device".to_string(),
        });
        
        // Wait for packages to load
        for _ in 0..10 {
            viewmodel.poll_events();
            smol::Timer::after(Duration::from_millis(100)).await;
            if !viewmodel.state.packages.is_empty() {
                break;
            }
        }
        
        assert!(!viewmodel.state.packages.is_empty());
        
        // Act - send filter command
        viewmodel.send_command(DebloatCommand::FilterPackages {
            text: "google".to_string(),
            category: None,
        });
        
        // Wait for filtered results
        for _ in 0..10 {
            viewmodel.poll_events();
            smol::Timer::after(Duration::from_millis(100)).await;
            if !viewmodel.state.filtered_packages.is_empty() {
                break;
            }
        }
        
        // Assert
        assert!(!viewmodel.state.filtered_packages.is_empty());
        assert!(viewmodel.state.filtered_packages.iter()
            .all(|p| p.pkg.contains("google")));
    });
}

#[test]
fn test_batch_uninstall_progress_events() {
    smol::block_on(async {
        // Arrange
        let mut viewmodel = ViewModel::new().await;
        let packages = vec![
            "com.test.app1".to_string(),
            "com.test.app2".to_string(),
        ];
        
        // Act
        viewmodel.send_command(DebloatCommand::BatchUninstall {
            package_names: packages.clone(),
        });
        
        let mut progress_events = Vec::new();
        
        // Collect progress events
        for _ in 0..50 {
            viewmodel.poll_events();
            
            if let Some(progress) = &viewmodel.state.operation_in_progress {
                progress_events.push((progress.current, progress.total));
            }
            
            smol::Timer::after(Duration::from_millis(100)).await;
            
            // Stop when complete
            if viewmodel.state.operation_in_progress.is_none() && !progress_events.is_empty() {
                break;
            }
        }
        
        // Assert
        assert!(!progress_events.is_empty());
        assert_eq!(progress_events.last().unwrap().0, 2); // All completed
    });
}
```

### Performance Tests

**Measure lag improvements:**

```rust
// In mobile/tests/performance/table_rendering_test.rs
#[test]
fn test_virtual_scrolling_performance() {
    use std::time::Instant;
    
    // Arrange - create 1000 packages
    let packages: Vec<_> = (0..1000)
        .map(|i| create_test_package(&format!("com.test.app{}", i)))
        .collect();
    
    // Act - measure render time with virtualization
    let start = Instant::now();
    
    // Simulate rendering only visible rows (20 out of 1000)
    let visible_rows = render_virtual_table(&packages, 0, 20);
    
    let duration = start.elapsed();
    
    // Assert - should render in < 16ms (60fps)
    assert!(duration.as_millis() < 16, 
        "Virtual scrolling too slow: {:?}", duration);
    assert_eq!(visible_rows.len(), 20);
}
```

### Manual Testing Checklist

**UI testing (manual, documented in test plan):**

- [ ] **Desktop view (width >= 800px):**
  - [ ] Filter sidebar visible on left
  - [ ] Full table with all columns
  - [ ] Scrolling is smooth (60 FPS)
  - [ ] Filtering updates without freezing
  - [ ] Batch uninstall shows progress

- [ ] **Mobile view (width < 800px):**
  - [ ] Filters collapse into dropdown
  - [ ] Cards display instead of table
  - [ ] Touch targets are 48px minimum
  - [ ] Scrolling is smooth

- [ ] **View transition:**
  - [ ] Resize window across 800px threshold
  - [ ] Layout switches smoothly
  - [ ] Selected packages preserved
  - [ ] Filter state preserved

- [ ] **Error scenarios:**
  - [ ] Disconnect device during operation
  - [ ] Filter with no results
  - [ ] Cancel batch uninstall mid-operation

### Continuous Integration

```bash
# In CI pipeline (.github/workflows/ci.yml)
- name: Run tests with coverage
  run: |
    cargo install cargo-llvm-cov
    cargo llvm-cov --fail-under-lines 80

- name: Run clippy
  run: cargo clippy -- -D warnings

- name: Check formatting
  run: cargo fmt --check
```

## Implementation Phases

### Phase 1: Debloat Tab Refactor (Priority)
1. Create directory structure: `tab_debloat/`
2. Extract state to `state.rs`
3. Implement virtual scrolling component
4. Create `view_desktop.rs` and `view_mobile.rs`
5. Add async filtering to DebloatActor
6. Migrate unit tests
7. Add integration tests

### Phase 2: SharedStore Migration
1. Add `packages` and `uad_ng_lists` to ViewModelState
2. Update DebloatActor to emit load events
3. Remove SharedStore accessors from tabs
4. Rename `SharedStore` → `TextureCache`
5. Update all references

### Phase 3: Scan Tab Refactor
1. Apply same pattern to scan tab
2. Virtual scrolling for scan results
3. Async scanning (already exists, verify)
4. Mobile/desktop views

### Phase 4: Apps Tab Refactor
1. Apply same pattern to apps tab
2. Virtual scrolling for app lists
3. Async app loading
4. Mobile/desktop views

### Phase 5: Shared Components (If Needed)
1. Extract common components if patterns emerge
2. Refactor to use shared components
3. Document component library

## Success Criteria

- ✅ **Performance**: 60 FPS scrolling with 500+ packages
- ✅ **Responsiveness**: Filtering/sorting doesn't freeze UI
- ✅ **File sizes**: All tab files < 500 lines
- ✅ **Test coverage**: 80%+ coverage
- ✅ **Architecture**: Zero direct SharedStore access from tabs
- ✅ **Mobile/Desktop**: Distinct layouts at 800px threshold
- ✅ **Build**: All builds pass (debug + release)
- ✅ **No regressions**: All existing features work

## Open Questions

None - all design decisions validated with user.

## Next Steps

1. ✅ Design document written and committed
2. Invoke `writing-plans` skill to create detailed implementation plan
3. Begin Phase 1 implementation (debloat tab)

---

**Generated**: 2026-08-13  
**Author**: Claude Sonnet 4.5  
**Status**: Design Approved - Ready for Planning

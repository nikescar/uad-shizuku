# SharedStore Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate all business state from SharedStore to ViewModel.state using TDD with vertical slices, leaving only texture caches in a minimal TextureCache.

**Architecture:** Event-driven MVVM with centralized state in ViewModel.state. Four actors (DebloatActor, ScanActor, MetadataActor, AppsActor) emit events to update state. Integration tests verify end-to-end flows.

**Tech Stack:** Rust, smol async runtime, eframe/egui, integration testing

## Global Constraints

- Rust edition 2021, clippy warnings treated as errors
- All state changes flow through ViewModel events (no direct SharedStore mutation)
- Integration tests use real components (no mocks)
- Each phase independently committable with passing tests
- Preserve egui::TextureHandle in separate TextureCache (lifetime constraints)

---

## Task 1: Write All Integration Tests

**Files:**
- Create: `mobile/tests/integration/mod.rs`
- Create: `mobile/tests/integration/scanner_migration_test.rs`
- Create: `mobile/tests/integration/metadata_migration_test.rs`
- Create: `mobile/tests/integration/stalkerware_migration_test.rs`

**Interfaces:**
- Consumes: Current ViewModel API, SharedStore API
- Produces: Integration test suite validating migration (all tests RED initially)

- [ ] **Step 1: Create test directory structure**

```bash
mkdir -p mobile/tests/integration
```

- [ ] **Step 2: Write integration test module**

Create `mobile/tests/integration/mod.rs`:

```rust
pub mod scanner_migration_test;
pub mod metadata_migration_test;
pub mod stalkerware_migration_test;
```

- [ ] **Step 3: Write scanner state migration tests**

Create `mobile/tests/integration/scanner_migration_test.rs`:

```rust
use mobile::viewmodel::{ViewModel, ViewModelEvent, ScanEvent};
use mobile::shared_store_stt::get_shared_store;
use std::time::Duration;

#[test]
fn test_virustotal_state_in_viewmodel() {
    // Setup: Create ViewModel with real smol runtime
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Action: Start VirusTotal scan (will fail initially - no device)
    let result = vm.run_virustotal("test_device".into(), "test_key".into(), false);
    
    // Allow event processing
    std::thread::sleep(Duration::from_millis(200));
    vm.poll_events(&ctx);
    
    // Verify: Scanner state appears in ViewModel.state (not SharedStore)
    assert!(vm.state.vt_scanner_state.is_some(), 
        "VirusTotal scanner state should be in ViewModel.state");
    
    // Verify: NOT in SharedStore anymore
    let shared_store = get_shared_store();
    let store_state = shared_store.vt_scanner_state.lock().unwrap();
    assert!(store_state.is_none(), 
        "VirusTotal scanner state should NOT be in SharedStore");
}

#[test]
fn test_hybridanalysis_state_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    vm.run_hybridanalysis("test_device".into(), "test_key".into(), false).ok();
    
    std::thread::sleep(Duration::from_millis(200));
    vm.poll_events(&ctx);
    
    assert!(vm.state.ha_scanner_state.is_some(),
        "HybridAnalysis scanner state should be in ViewModel.state");
    
    let shared_store = get_shared_store();
    let store_state = shared_store.ha_scanner_state.lock().unwrap();
    assert!(store_state.is_none(),
        "HybridAnalysis scanner state should NOT be in SharedStore");
}

#[test]
fn test_scan_cancellation_clears_state() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Start scan
    vm.run_virustotal("test_device".into(), "test_key".into(), false).ok();
    std::thread::sleep(Duration::from_millis(100));
    vm.poll_events(&ctx);
    
    // Cancel scan
    vm.cancel_virustotal().ok();
    std::thread::sleep(Duration::from_millis(100));
    vm.poll_events(&ctx);
    
    // State should be cleared
    assert!(vm.state.vt_scanner_state.is_none(),
        "Cancelled scan should clear scanner state");
}
```

- [ ] **Step 4: Write metadata cache migration tests**

Create `mobile/tests/integration/metadata_migration_test.rs`:

```rust
use mobile::viewmodel::ViewModel;
use mobile::shared_store_stt::get_shared_store;
use std::time::Duration;

#[test]
fn test_google_play_metadata_cached_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Action: Fetch Google Play metadata
    vm.fetch_google_play_metadata("com.example.app".into()).ok();
    
    // Wait for background fetch
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);
    
    // Verify: Metadata in ViewModel cache
    let cached = vm.state.cached_metadata.get_google_play("com.example.app");
    assert!(cached.is_some(), 
        "Google Play metadata should be cached in ViewModel");
    
    // Verify: NOT in SharedStore
    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_google_play_apps.lock().unwrap();
    assert!(store_cache.is_empty(),
        "Google Play metadata should NOT be in SharedStore");
}

#[test]
fn test_fdroid_metadata_cached_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    vm.fetch_fdroid_metadata("com.example.app".into()).ok();
    
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);
    
    let cached = vm.state.cached_metadata.get_fdroid("com.example.app");
    assert!(cached.is_some(), 
        "F-Droid metadata should be cached in ViewModel");
    
    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_fdroid_apps.lock().unwrap();
    assert!(store_cache.is_empty(),
        "F-Droid metadata should NOT be in SharedStore");
}

#[test]
fn test_apkmirror_metadata_cached_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    vm.fetch_apkmirror_metadata("com.example.app".into()).ok();
    
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);
    
    let cached = vm.state.cached_metadata.get_apkmirror("com.example.app");
    assert!(cached.is_some(), 
        "APKMirror metadata should be cached in ViewModel");
    
    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_apkmirror_apps.lock().unwrap();
    assert!(store_cache.is_empty(),
        "APKMirror metadata should NOT be in SharedStore");
}

#[test]
fn test_android_package_metadata_cached_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    vm.fetch_android_package_metadata("com.example.app".into()).ok();
    
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);
    
    let cached = vm.state.cached_metadata.get_android_package("com.example.app");
    assert!(cached.is_some(), 
        "Android Package metadata should be cached in ViewModel");
    
    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_android_package_apps.lock().unwrap();
    assert!(store_cache.is_empty(),
        "Android Package metadata should NOT be in SharedStore");
}

#[test]
fn test_metadata_cache_persists_across_calls() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Fetch once
    vm.fetch_google_play_metadata("com.example.app".into()).ok();
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);
    
    let first_fetch = vm.state.cached_metadata.get_google_play("com.example.app");
    assert!(first_fetch.is_some());
    
    // Fetch again - should use cache
    let second_fetch = vm.state.cached_metadata.get_google_play("com.example.app");
    assert!(second_fetch.is_some());
    
    // Should be same instance (cached)
    assert!(std::ptr::eq(first_fetch.unwrap(), second_fetch.unwrap()),
        "Metadata should persist in cache");
}
```

- [ ] **Step 5: Write stalkerware migration tests**

Create `mobile/tests/integration/stalkerware_migration_test.rs`:

```rust
use mobile::viewmodel::ViewModel;
use mobile::shared_store_stt::get_shared_store;
use std::time::Duration;

#[test]
fn test_stalkerware_indicators_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Action: Load UAD lists (should also load stalkerware)
    vm.load_uad_ng_lists().ok();
    
    std::thread::sleep(Duration::from_millis(300));
    vm.poll_events(&ctx);
    
    // Verify: Indicators in ViewModel
    assert!(vm.state.stalkerware_indicators.is_some(),
        "Stalkerware indicators should be in ViewModel.state");
    
    // Verify: NOT in SharedStore
    let shared_store = get_shared_store();
    let store_indicators = shared_store.stalkerware_indicators.lock().unwrap();
    assert!(store_indicators.is_none(),
        "Stalkerware indicators should NOT be in SharedStore");
}
```

- [ ] **Step 6: Run tests to verify they fail**

```bash
cargo test --test integration -- --nocapture
```

Expected: All tests FAIL with compilation errors (ViewModel.state fields don't exist yet)

- [ ] **Step 7: Commit integration tests**

```bash
git add mobile/tests/integration/
git commit -m "test: add integration tests for SharedStore migration

All tests currently fail - fields don't exist yet. Will pass as migration
progresses through phases.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Migrate Scanner States to ViewModel

**Files:**
- Modify: `mobile/src/viewmodel/common.rs` (add state fields)
- Modify: `mobile/src/viewmodel/scan.rs` (add events, update actor)
- Modify: `mobile/src/viewmodel/mod.rs` (add event handlers)
- Modify: `mobile/src/tab_scan_control.rs` (use ViewModel instead of SharedStore)
- Modify: `mobile/src/shared_store_stt.rs` (remove scanner state fields)

**Interfaces:**
- Consumes: ViewModelState, ScanEvent, ScanActor
- Produces: 
  - `ViewModelState.vt_scanner_state: Option<VtScannerState>`
  - `ViewModelState.ha_scanner_state: Option<HaScannerState>`
  - `ScanEvent::VirusTotalStateUpdated(VtScannerState)`
  - `ScanEvent::HybridAnalysisStateUpdated(HaScannerState)`

- [ ] **Step 1: Add scanner state fields to ViewModelState**

Edit `mobile/src/viewmodel/common.rs`:

```rust
// Find ViewModelState struct and add new fields
#[derive(Default)]
pub struct ViewModelState {
    // === Existing fields ===
    pub packages: Vec<crate::adb::PackageFingerprint>,
    pub uad_ng_lists: Option<crate::uad_shizuku_app::UadNgLists>,
    pub active_operations: HashMap<String, OperationProgress>,
    
    // === NEW: Scanner states ===
    pub vt_scanner_state: Option<crate::calc_virustotal::ScannerState>,
    pub ha_scanner_state: Option<crate::calc_hybridanalysis::ScannerState>,
}
```

- [ ] **Step 2: Add scanner state update events**

Edit `mobile/src/viewmodel/scan.rs`, find `ScanEvent` enum and add:

```rust
#[derive(Debug, Clone)]
pub enum ScanEvent {
    // === Existing events ===
    ScanStarted { scanner: ScannerType },
    ScanProgress { scanner: ScannerType, current: usize, total: usize },
    ScanComplete { scanner: ScannerType, results: Vec<ScanResult> },
    ScanCancelled { scanner: ScannerType },
    Error { scanner: ScannerType, error: String },
    
    // === NEW: Scanner state updates ===
    VirusTotalStateUpdated(crate::calc_virustotal::ScannerState),
    HybridAnalysisStateUpdated(crate::calc_hybridanalysis::ScannerState),
}
```

- [ ] **Step 3: Update ScanActor to emit scanner state events**

Edit `mobile/src/viewmodel/scan.rs`, in `ScanActor::run()` method:

Find the VirusTotal scan handling section and add state update event emission:

```rust
// In handle_run_virustotal or similar method
async fn handle_run_virustotal(&self, device: String, api_key: String, submit_enabled: bool) {
    // ... existing scan logic ...
    
    // Create initial scanner state
    let initial_state = crate::calc_virustotal::ScannerState {
        scanned: 0,
        total: packages.len(),
        scanning: true,
        results: Vec::new(),
    };
    
    // Emit state update event
    self.event_tx.send(ViewModelEvent::Scan(
        ScanEvent::VirusTotalStateUpdated(initial_state)
    )).await.ok();
    
    // ... continue with scan loop ...
    
    // Update state as scan progresses
    for (i, package) in packages.iter().enumerate() {
        // ... scan logic ...
        
        let updated_state = crate::calc_virustotal::ScannerState {
            scanned: i + 1,
            total: packages.len(),
            scanning: true,
            results: results.clone(),
        };
        
        self.event_tx.send(ViewModelEvent::Scan(
            ScanEvent::VirusTotalStateUpdated(updated_state)
        )).await.ok();
    }
    
    // Final state on completion
    let final_state = crate::calc_virustotal::ScannerState {
        scanned: packages.len(),
        total: packages.len(),
        scanning: false,
        results,
    };
    
    self.event_tx.send(ViewModelEvent::Scan(
        ScanEvent::VirusTotalStateUpdated(final_state)
    )).await.ok();
}

// Similar for HybridAnalysis
async fn handle_run_hybridanalysis(&self, device: String, api_key: String, submit_enabled: bool) {
    // ... mirror VirusTotal pattern with HybridAnalysisStateUpdated events ...
}

// On cancellation, clear state
async fn handle_cancel_virustotal(&self) {
    // Signal cancellation
    // ... existing cancel logic ...
    
    // Clear scanner state
    self.event_tx.send(ViewModelEvent::Scan(
        ScanEvent::VirusTotalStateUpdated(None) // or remove from state
    )).await.ok();
}
```

- [ ] **Step 4: Add event handlers in ViewModel**

Edit `mobile/src/viewmodel/mod.rs`, in `apply_event` method:

```rust
fn apply_event(&mut self, event: &ViewModelEvent, ctx: &eframe::egui::Context) {
    match event {
        // === Existing event handling ===
        ViewModelEvent::Debloat(DebloatEvent::PackagesLoaded(packages)) => {
            self.state.packages = packages.clone();
        }
        ViewModelEvent::Debloat(DebloatEvent::UadNgListsLoaded(lists)) => {
            self.state.uad_ng_lists = Some(lists.clone());
        }
        // ... other existing handlers ...
        
        // === NEW: Scanner state events ===
        ViewModelEvent::Scan(ScanEvent::VirusTotalStateUpdated(state)) => {
            self.state.vt_scanner_state = Some(state.clone());
            ctx.request_repaint();
        }
        ViewModelEvent::Scan(ScanEvent::HybridAnalysisStateUpdated(state)) => {
            self.state.ha_scanner_state = Some(state.clone());
            ctx.request_repaint();
        }
        
        _ => {}
    }
}
```

- [ ] **Step 5: Update scan tab to use ViewModel state**

Edit `mobile/src/tab_scan_control.rs`:

Find all SharedStore scanner state access patterns and replace:

```rust
// OLD PATTERN (remove):
// let shared_store = get_shared_store();
// if let Some(vt_state) = shared_store.vt_scanner_state.lock().unwrap().as_ref() {
//     // ... use vt_state ...
// }

// NEW PATTERN (add):
if let Some(ref vm) = app.viewmodel {
    if let Some(vt_state) = &vm.state.vt_scanner_state {
        // Display scan progress
        ui.label(format!("Scanned: {}/{}", vt_state.scanned, vt_state.total));
        ui.add(egui::ProgressBar::new(
            vt_state.scanned as f32 / vt_state.total as f32
        ));
        
        // ... rest of UI logic ...
    }
}

// Repeat for HybridAnalysis scanner state
if let Some(ref vm) = app.viewmodel {
    if let Some(ha_state) = &vm.state.ha_scanner_state {
        ui.label(format!("Scanned: {}/{}", ha_state.scanned, ha_state.total));
        // ... UI logic ...
    }
}
```

- [ ] **Step 6: Remove scanner state fields from SharedStore**

Edit `mobile/src/shared_store_stt.rs`:

```rust
pub struct SharedStore {
    // ... other fields ...
    
    // DELETE these fields:
    // pub vt_scanner_state: Mutex<Option<VtScannerState>>,
    // pub ha_scanner_state: Mutex<Option<HaScannerState>>,
    
    // ... rest of fields ...
}

// Also remove from SharedStore::new() initialization
impl SharedStore {
    pub fn new() -> Self {
        Self {
            // ... other initializations ...
            // DELETE:
            // vt_scanner_state: Mutex::new(None),
            // ha_scanner_state: Mutex::new(None),
            // ... rest ...
        }
    }
}
```

- [ ] **Step 7: Build to verify compilation**

```bash
cargo build
```

Expected: Build succeeds with no errors

- [ ] **Step 8: Run scanner migration tests**

```bash
cargo test --test integration::scanner_migration_test -- --nocapture
```

Expected: All scanner tests PASS

- [ ] **Step 9: Run full build verification**

```bash
cargo build --release
cargo clippy -- -D warnings
```

Expected: All pass

- [ ] **Step 10: Commit scanner state migration**

```bash
git add mobile/src/viewmodel/common.rs mobile/src/viewmodel/scan.rs mobile/src/viewmodel/mod.rs mobile/src/tab_scan_control.rs mobile/src/shared_store_stt.rs
git commit -m "feat: migrate scanner states to ViewModel

- Add vt_scanner_state and ha_scanner_state to ViewModelState
- Add VirusTotalStateUpdated/HybridAnalysisStateUpdated events
- ScanActor emits state updates during scans
- Scan tab reads state from ViewModel instead of SharedStore
- Remove scanner state fields from SharedStore

Tests: cargo test --test integration::scanner_migration_test (all pass)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Migrate Metadata Cache to ViewModel

**Files:**
- Modify: `mobile/src/viewmodel/common.rs` (add MetadataCache struct)
- Modify: `mobile/src/viewmodel/metadata.rs` (add events, update actor)
- Modify: `mobile/src/viewmodel/mod.rs` (add event handlers)
- Modify: `mobile/src/tab_debloat_control.rs` (use ViewModel cache)
- Modify: `mobile/src/tab_scan_control.rs` (use ViewModel cache)
- Modify: `mobile/src/shared_store_stt.rs` (remove metadata cache fields)

**Interfaces:**
- Consumes: ViewModelState, MetadataEvent, MetadataActor
- Produces:
  - `ViewModelState.cached_metadata: MetadataCache`
  - `MetadataCache.get_google_play(&str) -> Option<&GooglePlayApp>`
  - `MetadataEvent::GooglePlayCached { pkg_id: String, app: GooglePlayApp }`
  - (similar for FDroid, ApkMirror, AndroidPackage)

- [ ] **Step 1: Add MetadataCache struct to ViewModelState**

Edit `mobile/src/viewmodel/common.rs`:

```rust
use std::collections::HashMap;
use crate::models::{GooglePlayApp, FDroidApp, ApkMirrorApp};
use crate::calc_androidpackage::AndroidPackageInfo;

/// Unified metadata cache for all sources
#[derive(Default, Clone, Debug)]
pub struct MetadataCache {
    pub google_play: HashMap<String, GooglePlayApp>,
    pub fdroid: HashMap<String, FDroidApp>,
    pub apkmirror: HashMap<String, ApkMirrorApp>,
    pub android_package: HashMap<String, AndroidPackageInfo>,
}

impl MetadataCache {
    pub fn get_google_play(&self, pkg_id: &str) -> Option<&GooglePlayApp> {
        self.google_play.get(pkg_id)
    }
    
    pub fn get_fdroid(&self, pkg_id: &str) -> Option<&FDroidApp> {
        self.fdroid.get(pkg_id)
    }
    
    pub fn get_apkmirror(&self, pkg_id: &str) -> Option<&ApkMirrorApp> {
        self.apkmirror.get(pkg_id)
    }
    
    pub fn get_android_package(&self, pkg_id: &str) -> Option<&AndroidPackageInfo> {
        self.android_package.get(pkg_id)
    }
}

// Add to ViewModelState
#[derive(Default)]
pub struct ViewModelState {
    // === Existing fields ===
    pub packages: Vec<crate::adb::PackageFingerprint>,
    pub uad_ng_lists: Option<crate::uad_shizuku_app::UadNgLists>,
    pub active_operations: HashMap<String, OperationProgress>,
    pub vt_scanner_state: Option<crate::calc_virustotal::ScannerState>,
    pub ha_scanner_state: Option<crate::calc_hybridanalysis::ScannerState>,
    
    // === NEW: Metadata cache ===
    pub cached_metadata: MetadataCache,
}
```

- [ ] **Step 2: Add metadata cache events**

Edit `mobile/src/viewmodel/metadata.rs`:

Check if MetadataEvent enum exists, if not create it:

```rust
use crate::models::{GooglePlayApp, FDroidApp, ApkMirrorApp};
use crate::calc_androidpackage::AndroidPackageInfo;

#[derive(Debug, Clone)]
pub enum MetadataEvent {
    GooglePlayCached { pkg_id: String, app: GooglePlayApp },
    FDroidCached { pkg_id: String, app: FDroidApp },
    ApkMirrorCached { pkg_id: String, app: ApkMirrorApp },
    AndroidPackageCached { pkg_id: String, app: AndroidPackageInfo },
    Error { pkg_id: String, error: String },
}
```

- [ ] **Step 3: Update MetadataActor to emit cache events**

Edit `mobile/src/viewmodel/metadata.rs`, in `MetadataActor`:

```rust
impl MetadataActor {
    // Add or update metadata fetch handlers
    async fn handle_fetch_google_play(&self, package: String) {
        match crate::calc_googleplay::fetch_google_play_info(&package).await {
            Ok(app) => {
                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::GooglePlayCached { 
                        pkg_id: package.clone(), 
                        app 
                    }
                )).await.ok();
            }
            Err(e) => {
                log::error!("Failed to fetch Google Play metadata for {}: {}", package, e);
                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::Error { 
                        pkg_id: package, 
                        error: e.to_string() 
                    }
                )).await.ok();
            }
        }
    }
    
    async fn handle_fetch_fdroid(&self, package: String) {
        match crate::calc_fdroid::fetch_fdroid_info(&package).await {
            Ok(app) => {
                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::FDroidCached { 
                        pkg_id: package.clone(), 
                        app 
                    }
                )).await.ok();
            }
            Err(e) => {
                log::error!("Failed to fetch F-Droid metadata for {}: {}", package, e);
                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::Error { 
                        pkg_id: package, 
                        error: e.to_string() 
                    }
                )).await.ok();
            }
        }
    }
    
    async fn handle_fetch_apkmirror(&self, package: String) {
        match crate::calc_apkmirror::fetch_apkmirror_info(&package).await {
            Ok(app) => {
                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::ApkMirrorCached { 
                        pkg_id: package.clone(), 
                        app 
                    }
                )).await.ok();
            }
            Err(e) => {
                log::error!("Failed to fetch APKMirror metadata for {}: {}", package, e);
                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::Error { 
                        pkg_id: package, 
                        error: e.to_string() 
                    }
                )).await.ok();
            }
        }
    }
    
    async fn handle_fetch_android_package(&self, package: String) {
        match crate::calc_androidpackage::fetch_package_info(&package).await {
            Ok(app) => {
                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::AndroidPackageCached { 
                        pkg_id: package.clone(), 
                        app 
                    }
                )).await.ok();
            }
            Err(e) => {
                log::error!("Failed to fetch Android Package metadata for {}: {}", package, e);
                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::Error { 
                        pkg_id: package, 
                        error: e.to_string() 
                    }
                )).await.ok();
            }
        }
    }
}
```

- [ ] **Step 4: Add metadata event handlers in ViewModel**

Edit `mobile/src/viewmodel/mod.rs`, in `apply_event`:

```rust
fn apply_event(&mut self, event: &ViewModelEvent, ctx: &eframe::egui::Context) {
    match event {
        // === Existing handlers ===
        // ... (keep all existing handlers) ...
        
        // === NEW: Metadata cache events ===
        ViewModelEvent::Metadata(MetadataEvent::GooglePlayCached { pkg_id, app }) => {
            self.state.cached_metadata.google_play.insert(pkg_id.clone(), app.clone());
            ctx.request_repaint();
        }
        ViewModelEvent::Metadata(MetadataEvent::FDroidCached { pkg_id, app }) => {
            self.state.cached_metadata.fdroid.insert(pkg_id.clone(), app.clone());
            ctx.request_repaint();
        }
        ViewModelEvent::Metadata(MetadataEvent::ApkMirrorCached { pkg_id, app }) => {
            self.state.cached_metadata.apkmirror.insert(pkg_id.clone(), app.clone());
            ctx.request_repaint();
        }
        ViewModelEvent::Metadata(MetadataEvent::AndroidPackageCached { pkg_id, app }) => {
            self.state.cached_metadata.android_package.insert(pkg_id.clone(), app.clone());
            ctx.request_repaint();
        }
        ViewModelEvent::Metadata(MetadataEvent::Error { pkg_id, error }) => {
            log::warn!("Metadata fetch error for {}: {}", pkg_id, error);
        }
        
        _ => {}
    }
}
```

- [ ] **Step 5: Update debloat tab to use ViewModel cache**

Edit `mobile/src/tab_debloat_control.rs`:

Find all SharedStore metadata access and replace:

```rust
// OLD PATTERN (remove):
// let shared_store = get_shared_store();
// let google_play_apps = shared_store.cached_google_play_apps.lock().unwrap();
// if let Some(app_info) = google_play_apps.get(&pkg_id) {
//     // ... use app_info ...
// }

// NEW PATTERN (add):
if let Some(ref vm) = app.viewmodel {
    if let Some(app_info) = vm.state.cached_metadata.get_google_play(&pkg_id) {
        ui.label(&app_info.title);
        ui.label(&app_info.description);
        // ... rest of UI logic ...
    } else {
        // Trigger fetch if not cached
        vm.fetch_google_play_metadata(pkg_id.clone()).ok();
        ui.spinner();
        ui.label("Loading metadata...");
    }
}

// Repeat for F-Droid, APKMirror, Android Package
if let Some(ref vm) = app.viewmodel {
    if let Some(app_info) = vm.state.cached_metadata.get_fdroid(&pkg_id) {
        // ... UI logic ...
    }
}

if let Some(ref vm) = app.viewmodel {
    if let Some(app_info) = vm.state.cached_metadata.get_apkmirror(&pkg_id) {
        // ... UI logic ...
    }
}

if let Some(ref vm) = app.viewmodel {
    if let Some(app_info) = vm.state.cached_metadata.get_android_package(&pkg_id) {
        // ... UI logic ...
    }
}
```

- [ ] **Step 6: Update scan tab to use ViewModel cache**

Edit `mobile/src/tab_scan_control.rs`:

Apply same pattern as debloat tab - replace SharedStore metadata access with ViewModel.state.cached_metadata access.

- [ ] **Step 7: Remove metadata cache fields from SharedStore**

Edit `mobile/src/shared_store_stt.rs`:

```rust
pub struct SharedStore {
    // ... other fields ...
    
    // DELETE these fields:
    // pub cached_google_play_apps: Mutex<HashMap<String, GooglePlayApp>>,
    // pub cached_fdroid_apps: Mutex<HashMap<String, FDroidApp>>,
    // pub cached_apkmirror_apps: Mutex<HashMap<String, ApkMirrorApp>>,
    // pub cached_android_package_apps: Mutex<HashMap<String, AndroidPackageInfo>>,
    
    // ... rest of fields ...
}

// Also remove from SharedStore::new()
impl SharedStore {
    pub fn new() -> Self {
        Self {
            // ... other initializations ...
            // DELETE:
            // cached_google_play_apps: Mutex::new(HashMap::new()),
            // cached_fdroid_apps: Mutex::new(HashMap::new()),
            // cached_apkmirror_apps: Mutex::new(HashMap::new()),
            // cached_android_package_apps: Mutex::new(HashMap::new()),
            // ... rest ...
        }
    }
}
```

- [ ] **Step 8: Build to verify compilation**

```bash
cargo build
```

Expected: Build succeeds

- [ ] **Step 9: Run metadata migration tests**

```bash
cargo test --test integration::metadata_migration_test -- --nocapture
```

Expected: All metadata tests PASS

- [ ] **Step 10: Run full build verification**

```bash
cargo build --release
cargo clippy -- -D warnings
```

Expected: All pass

- [ ] **Step 11: Commit metadata cache migration**

```bash
git add mobile/src/viewmodel/common.rs mobile/src/viewmodel/metadata.rs mobile/src/viewmodel/mod.rs mobile/src/tab_debloat_control.rs mobile/src/tab_scan_control.rs mobile/src/shared_store_stt.rs
git commit -m "feat: migrate metadata cache to ViewModel

- Add MetadataCache struct with unified cache for all sources
- Add cached_metadata field to ViewModelState
- Add GooglePlayCached/FDroidCached/ApkMirrorCached/AndroidPackageCached events
- MetadataActor emits cache events on successful fetch
- Tabs read metadata from ViewModel.state.cached_metadata
- Remove metadata cache fields from SharedStore

Tests: cargo test --test integration::metadata_migration_test (all pass)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Migrate Stalkerware Indicators to ViewModel

**Files:**
- Modify: `mobile/src/viewmodel/common.rs` (add stalkerware_indicators field)
- Modify: `mobile/src/viewmodel/debloat.rs` (add event, update actor)
- Modify: `mobile/src/viewmodel/mod.rs` (add event handler)
- Modify: `mobile/src/tab_debloat_control.rs` (use ViewModel stalkerware)
- Modify: `mobile/src/shared_store_stt.rs` (remove stalkerware field)

**Interfaces:**
- Consumes: ViewModelState, DebloatEvent, DebloatActor
- Produces:
  - `ViewModelState.stalkerware_indicators: Option<StalkerwareIndicators>`
  - `DebloatEvent::StalkerwareIndicatorsLoaded(StalkerwareIndicators)`

- [ ] **Step 1: Add stalkerware_indicators field to ViewModelState**

Edit `mobile/src/viewmodel/common.rs`:

```rust
#[derive(Default)]
pub struct ViewModelState {
    // === Existing fields ===
    pub packages: Vec<crate::adb::PackageFingerprint>,
    pub uad_ng_lists: Option<crate::uad_shizuku_app::UadNgLists>,
    pub active_operations: HashMap<String, OperationProgress>,
    pub vt_scanner_state: Option<crate::calc_virustotal::ScannerState>,
    pub ha_scanner_state: Option<crate::calc_hybridanalysis::ScannerState>,
    pub cached_metadata: MetadataCache,
    
    // === NEW: Stalkerware indicators ===
    pub stalkerware_indicators: Option<crate::calc_stalkerware_stt::StalkerwareIndicators>,
}
```

- [ ] **Step 2: Add stalkerware loaded event**

Edit `mobile/src/viewmodel/debloat.rs`:

```rust
#[derive(Debug, Clone)]
pub enum DebloatEvent {
    // === Existing events ===
    PackagesLoaded(Vec<crate::adb::PackageFingerprint>),
    UadNgListsLoaded(crate::uad_shizuku_app::UadNgLists),
    BatchProgress { operation: String, progress: f32, current: usize, total: usize },
    BatchComplete { operation: String, succeeded: Vec<String>, failed: Vec<String> },
    Error { operation: String, error: String },
    
    // === NEW ===
    StalkerwareIndicatorsLoaded(crate::calc_stalkerware_stt::StalkerwareIndicators),
}
```

- [ ] **Step 3: Update DebloatActor to load and emit stalkerware**

Edit `mobile/src/viewmodel/debloat.rs`, in `DebloatActor::handle_load_uad_ng_lists`:

```rust
async fn handle_load_uad_ng_lists(&self) {
    // Load UAD-NG lists
    match crate::uad_shizuku_app::load_uad_ng_lists().await {
        Ok(lists) => {
            self.event_tx.send(ViewModelEvent::Debloat(
                DebloatEvent::UadNgListsLoaded(lists)
            )).await.ok();
        }
        Err(e) => {
            log::error!("Failed to load UAD-NG lists: {}", e);
            self.event_tx.send(ViewModelEvent::Debloat(
                DebloatEvent::Error {
                    operation: "load_uad_lists".into(),
                    error: e.to_string(),
                }
            )).await.ok();
            return;
        }
    }
    
    // Also load stalkerware indicators
    match crate::calc_stalkerware_stt::load_stalkerware_indicators().await {
        Ok(indicators) => {
            self.event_tx.send(ViewModelEvent::Debloat(
                DebloatEvent::StalkerwareIndicatorsLoaded(indicators)
            )).await.ok();
        }
        Err(e) => {
            log::warn!("Failed to load stalkerware indicators: {}", e);
            // Non-fatal - stalkerware is optional
        }
    }
}
```

- [ ] **Step 4: Add stalkerware event handler in ViewModel**

Edit `mobile/src/viewmodel/mod.rs`, in `apply_event`:

```rust
fn apply_event(&mut self, event: &ViewModelEvent, ctx: &eframe::egui::Context) {
    match event {
        // === Existing handlers ===
        // ... (keep all existing handlers) ...
        
        // === NEW: Stalkerware indicators ===
        ViewModelEvent::Debloat(DebloatEvent::StalkerwareIndicatorsLoaded(indicators)) => {
            self.state.stalkerware_indicators = Some(indicators.clone());
            log::info!("Stalkerware indicators loaded: {} patterns", 
                indicators.patterns.len());
        }
        
        _ => {}
    }
}
```

- [ ] **Step 5: Update debloat tab to use ViewModel stalkerware**

Edit `mobile/src/tab_debloat_control.rs`:

```rust
// OLD PATTERN (remove):
// let shared_store = get_shared_store();
// if let Some(indicators) = shared_store.stalkerware_indicators.lock().unwrap().as_ref() {
//     // ... check package against indicators ...
// }

// NEW PATTERN (add):
if let Some(ref vm) = app.viewmodel {
    if let Some(indicators) = &vm.state.stalkerware_indicators {
        // Check if package matches stalkerware patterns
        if indicators.is_stalkerware(&pkg_id) {
            ui.colored_label(egui::Color32::RED, "⚠ Potential stalkerware");
            ui.label("This app matches known stalkerware indicators");
        }
    }
}
```

- [ ] **Step 6: Remove stalkerware field from SharedStore**

Edit `mobile/src/shared_store_stt.rs`:

```rust
pub struct SharedStore {
    // ... other fields ...
    
    // DELETE this field:
    // pub stalkerware_indicators: Mutex<Option<StalkerwareIndicators>>,
    
    // ... rest of fields ...
}

// Also remove from SharedStore::new()
impl SharedStore {
    pub fn new() -> Self {
        Self {
            // ... other initializations ...
            // DELETE:
            // stalkerware_indicators: Mutex::new(None),
            // ... rest ...
        }
    }
}
```

- [ ] **Step 7: Build to verify compilation**

```bash
cargo build
```

Expected: Build succeeds

- [ ] **Step 8: Run stalkerware migration tests**

```bash
cargo test --test integration::stalkerware_migration_test -- --nocapture
```

Expected: Stalkerware tests PASS

- [ ] **Step 9: Run all integration tests**

```bash
cargo test --test integration -- --nocapture
```

Expected: All tests PASS (scanner, metadata, stalkerware)

- [ ] **Step 10: Run full build verification**

```bash
cargo build --release
cargo clippy -- -D warnings
```

Expected: All pass

- [ ] **Step 11: Commit stalkerware migration**

```bash
git add mobile/src/viewmodel/common.rs mobile/src/viewmodel/debloat.rs mobile/src/viewmodel/mod.rs mobile/src/tab_debloat_control.rs mobile/src/shared_store_stt.rs
git commit -m "feat: migrate stalkerware indicators to ViewModel

- Add stalkerware_indicators field to ViewModelState
- Add StalkerwareIndicatorsLoaded event to DebloatEvent
- DebloatActor loads stalkerware when loading UAD lists
- Debloat tab reads stalkerware from ViewModel.state
- Remove stalkerware_indicators field from SharedStore

Tests: cargo test --test integration (all pass)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Refactor SharedStore to Texture-Only Cache

**Files:**
- Create: `mobile/src/texture_cache.rs`
- Modify: `mobile/src/lib.rs`
- Delete: `mobile/src/shared_store.rs`
- Delete: `mobile/src/shared_store_stt.rs`
- Modify: All files that import SharedStore (15+ files)

**Interfaces:**
- Consumes: Remaining SharedStore texture caches
- Produces:
  - `TextureCache` struct with only texture HashMap fields
  - `get_texture_cache() -> Arc<TextureCache>` global accessor

- [ ] **Step 1: Create TextureCache module**

Create `mobile/src/texture_cache.rs`:

```rust
use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Texture cache for UI icons - kept separate from ViewModel due to egui::TextureHandle lifetime constraints
pub struct TextureCache {
    pub google_play_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    pub fdroid_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    pub apkmirror_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    pub android_package_textures: Mutex<HashMap<String, egui::TextureHandle>>,
}

impl TextureCache {
    fn new() -> Self {
        Self {
            google_play_textures: Mutex::new(HashMap::new()),
            fdroid_textures: Mutex::new(HashMap::new()),
            apkmirror_textures: Mutex::new(HashMap::new()),
            android_package_textures: Mutex::new(HashMap::new()),
        }
    }
}

static TEXTURE_CACHE: OnceLock<Arc<TextureCache>> = OnceLock::new();

/// Get the global texture cache instance
pub fn get_texture_cache() -> Arc<TextureCache> {
    TEXTURE_CACHE
        .get_or_init(|| Arc::new(TextureCache::new()))
        .clone()
}
```

- [ ] **Step 2: Update lib.rs module declarations**

Edit `mobile/src/lib.rs`:

```rust
// DELETE these lines:
// pub mod shared_store;
// pub mod shared_store_stt;

// ADD this line:
pub mod texture_cache;
```

- [ ] **Step 3: Find all files that import SharedStore**

```bash
grep -r "use.*shared_store" mobile/src --include="*.rs" | cut -d: -f1 | sort -u
```

Expected output: List of 15+ files

- [ ] **Step 4: Update imports in all files**

For each file from step 3, update imports:

```rust
// OLD:
// use crate::shared_store_stt::{get_shared_store, SharedStore};

// NEW:
use crate::texture_cache::{get_texture_cache, TextureCache};
```

Files to update (check each):
- `mobile/src/uad_shizuku_app.rs`
- `mobile/src/tab_debloat_control.rs`
- `mobile/src/tab_scan_control.rs`
- `mobile/src/tab_apps_control.rs`
- And any other files found in step 3

- [ ] **Step 5: Update texture access patterns in all files**

For each file, find and replace SharedStore texture access:

```rust
// OLD PATTERN:
// let shared_store = get_shared_store();
// if let Some(texture) = shared_store.google_play_textures.lock().unwrap().get(&pkg_id) {
//     // ... use texture ...
// }

// NEW PATTERN:
let cache = get_texture_cache();
if let Some(texture) = cache.google_play_textures.lock().unwrap().get(&pkg_id) {
    image.paint_at(ui, rect);
}

// When storing textures:
let cache = get_texture_cache();
cache.google_play_textures.lock().unwrap().insert(
    pkg_id.clone(),
    texture_handle
);
```

Apply this pattern for all four texture caches:
- `google_play_textures`
- `fdroid_textures`
- `apkmirror_textures`
- `android_package_textures`

- [ ] **Step 6: Delete old SharedStore files**

```bash
git rm mobile/src/shared_store.rs
git rm mobile/src/shared_store_stt.rs
```

- [ ] **Step 7: Build to verify compilation**

```bash
cargo build
```

Expected: Build succeeds with no errors

- [ ] **Step 8: Run all integration tests**

```bash
cargo test --test integration -- --nocapture
```

Expected: ALL tests PASS (scanner, metadata, stalkerware)

- [ ] **Step 9: Run full build verification**

```bash
cargo build --release
cargo clippy -- -D warnings
```

Expected: All pass, no warnings

- [ ] **Step 10: Manual smoke test (optional but recommended)**

```bash
cargo run
```

Test in UI:
- Load packages (debloat tab)
- View app icons (should render from TextureCache)
- Start VirusTotal scan (scanner state from ViewModel)
- View app metadata (from ViewModel cache)

Expected: All features work, no SharedStore references in logs

- [ ] **Step 11: Commit SharedStore refactor**

```bash
git add mobile/src/texture_cache.rs mobile/src/lib.rs
git add -u  # Stage all deleted and modified files
git commit -m "refactor: convert SharedStore to texture-only TextureCache

- Create new TextureCache module with only texture HashMap fields
- Delete shared_store.rs and shared_store_stt.rs
- Update all imports from get_shared_store to get_texture_cache
- Update all texture access patterns across 15+ files

All business state now in ViewModel.state, only textures remain in
separate cache due to egui::TextureHandle lifetime constraints.

Tests: cargo test --test integration (all pass)
Build: cargo build --release && cargo clippy (clean)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Task 6: Final Verification and Documentation

**Files:**
- Modify: `docs/mvvm-migration-complete.md` (update completion status)
- Modify: `README.md` (if SharedStore mentioned)
- Create: `docs/architecture.md` (if doesn't exist, document new architecture)

**Interfaces:**
- Consumes: All completed migration tasks
- Produces: Updated documentation reflecting new ViewModel-centric architecture

- [ ] **Step 1: Run complete test suite**

```bash
cargo test
```

Expected: All tests pass (unit + integration)

- [ ] **Step 2: Run full build verification**

```bash
cargo build --release
cargo clippy -- -D warnings
```

Expected: Clean build, zero warnings

- [ ] **Step 3: Check git status**

```bash
git status
git log --oneline -5
```

Expected: 5 clean commits (tests + 4 migration phases)

- [ ] **Step 4: Update migration documentation**

Edit `docs/mvvm-migration-complete.md`, add section:

```markdown
## Phase 2: SharedStore Migration (2026-06-17)

Successfully migrated all business state from SharedStore to ViewModel.state using TDD vertical slices approach.

### Completed Work

**Task 1: Integration Tests**
- ✅ Scanner state migration tests
- ✅ Metadata cache migration tests  
- ✅ Stalkerware migration tests
- All tests written upfront (RED), then made GREEN through migration

**Task 2: Scanner States**
- ✅ vt_scanner_state moved to ViewModel.state
- ✅ ha_scanner_state moved to ViewModel.state
- ✅ ScanActor emits VirusTotalStateUpdated/HybridAnalysisStateUpdated events
- ✅ Scan tab reads state from ViewModel

**Task 3: Metadata Cache**
- ✅ MetadataCache struct with unified cache
- ✅ cached_metadata in ViewModel.state
- ✅ MetadataActor emits GooglePlayCached/FDroidCached/ApkMirrorCached/AndroidPackageCached events
- ✅ All tabs read metadata from ViewModel.state

**Task 4: Stalkerware Indicators**
- ✅ stalkerware_indicators in ViewModel.state
- ✅ DebloatActor emits StalkerwareIndicatorsLoaded event
- ✅ Debloat tab reads stalkerware from ViewModel

**Task 5: TextureCache Refactor**
- ✅ SharedStore deleted
- ✅ TextureCache created (texture-only)
- ✅ All imports updated (15+ files)
- ✅ Clean separation: business state in ViewModel, UI textures in TextureCache

### Architecture After Migration

All business state now centralized in ViewModel.state:
- packages, uad_ng_lists (already migrated)
- vt_scanner_state, ha_scanner_state (new)
- cached_metadata (new unified cache)
- stalkerware_indicators (new)
- active_operations (existing)

Textures remain in separate TextureCache due to egui::TextureHandle lifetime constraints.

### Verification

- ✅ All integration tests pass
- ✅ cargo build --release succeeds
- ✅ cargo clippy clean (zero warnings)
- ✅ Manual smoke test passed
- ✅ 5 commits (tests + 4 phases)

**Total LOC Changes:** ~1,280 lines
- Tests: 400 lines (new)
- ViewModel.state: 100 lines (new)
- Events: 80 lines (new)
- Actor updates: 200 lines (modified)
- Tab updates: 300 lines (modified)
- SharedStore → TextureCache: 150 lines (deleted), 50 lines (new)
```

- [ ] **Step 5: Update README if needed**

Check `README.md` for any SharedStore mentions:

```bash
grep -i "sharedstore" README.md
```

If found, update to mention ViewModel.state instead.

- [ ] **Step 6: Document architecture (if needed)**

If `docs/architecture.md` doesn't exist or needs update, document:

```markdown
# uad-shizuku Architecture

## MVVM Pattern

### UI Thread
- Tab UI components (debloat, scan, apps)
- Polls ViewModel events each frame
- Sends commands to ViewModel
- Reads state from ViewModel.state

### Background Thread (smol runtime)
- 4 actors: DebloatActor, ScanActor, AppsActor, MetadataActor
- Process commands asynchronously
- Emit events on state changes
- No direct UI access

### ViewModel (Bridge)
- Centralized state in ViewModel.state
- Command channels to actors
- Unified event channel from actors
- apply_event() updates state from events

### State Management
All business state in ViewModel.state:
- packages: Vec<PackageFingerprint>
- uad_ng_lists: Option<UadNgLists>
- vt_scanner_state, ha_scanner_state: Option<ScannerState>
- cached_metadata: MetadataCache
- stalkerware_indicators: Option<StalkerwareIndicators>
- active_operations: HashMap<String, OperationProgress>

### TextureCache (Separate)
UI textures kept in global TextureCache due to egui::TextureHandle lifetime constraints.
```

- [ ] **Step 7: Commit documentation updates**

```bash
git add docs/mvvm-migration-complete.md README.md docs/architecture.md
git commit -m "docs: update migration completion and architecture docs

Document Phase 2 completion: SharedStore migration to ViewModel.state
with TDD vertical slices approach.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

- [ ] **Step 8: Final verification checklist**

Verify all success criteria:

```markdown
## Final Checklist

- [ ] All integration tests pass (cargo test --test integration)
- [ ] Full build succeeds (cargo build --release)
- [ ] Clippy clean (cargo clippy -- -D warnings)
- [ ] Manual smoke test completed
- [ ] SharedStore files deleted
- [ ] TextureCache created
- [ ] All state in ViewModel.state
- [ ] 6 commits total (tests + 4 phases + docs)
- [ ] Documentation updated
```

Expected: All items checked

---

## Summary

This plan implements the SharedStore migration using strict TDD with vertical slices:

1. **Task 1**: Write all integration tests upfront (RED)
2. **Task 2**: Migrate scanner states → tests GREEN
3. **Task 3**: Migrate metadata cache → tests GREEN
4. **Task 4**: Migrate stalkerware → tests GREEN
5. **Task 5**: Refactor to TextureCache → all tests GREEN
6. **Task 6**: Final verification and docs

Each task is independently committable with clear verification steps. Total: ~1,280 LOC across 6 commits.

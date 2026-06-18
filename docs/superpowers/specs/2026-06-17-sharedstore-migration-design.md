# SharedStore Migration Design - TDD Approach

**Date:** 2026-06-17  
**Branch:** `refactor/mvvm-actor-architecture`  
**Approach:** Test-First Vertical Slices with Integration Testing

## Executive Summary

Migrate all business state from global SharedStore singleton to centralized ViewModel.state, leaving only texture caches in a minimal SharedStore (renamed to TextureCache). Use TDD with integration tests written upfront, then migrate in vertical slices (scanner states → metadata cache → stalkerware → SharedStore refactor).

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Test Coverage** | Integration tests focused | End-to-end flows with real components, pragmatic for refactoring |
| **Cached Metadata Location** | ViewModel.state centralized | Consistent with existing packages/uad_ng_lists pattern |
| **Scanner States Location** | ViewModel.state centralized | All state in one place for easy debugging |
| **Texture Cache Handling** | Keep minimal SharedStore | Respects egui::TextureHandle lifetime constraints |
| **Migration Approach** | Feature-freeze, vertical slices | Aggressive but clean, migrate all then refactor |
| **TDD Cadence** | Write all tests upfront | Clear success criteria before implementation |

## 1. Overview & Architecture

### Migration Goal

Move all business state from SharedStore to ViewModel, leaving only texture caches in a minimal SharedStore.

### What's Moving

```rust
// FROM: SharedStore (global singleton)
pub struct SharedStore {
    // ✅ Already migrated
    installed_packages: Mutex<Vec<PackageFingerprint>>,
    uad_ng_lists: Mutex<Option<UadNgLists>>,
    
    // 🔄 TO MIGRATE
    stalkerware_indicators: Mutex<Option<StalkerwareIndicators>>,
    vt_scanner_state: Mutex<Option<VtScannerState>>,
    ha_scanner_state: Mutex<Option<HaScannerState>>,
    cached_google_play_apps: Mutex<HashMap<String, GooglePlayApp>>,
    cached_fdroid_apps: Mutex<HashMap<String, FDroidApp>>,
    cached_apkmirror_apps: Mutex<HashMap<String, ApkMirrorApp>>,
    cached_android_package_apps: Mutex<HashMap<String, AndroidPackageInfo>>,
    
    // ⚠️ KEEP (texture-only SharedStore)
    google_play_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    fdroid_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    apkmirror_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    android_package_textures: Mutex<HashMap<String, egui::TextureHandle>>,
}

// TO: ViewModel.state (centralized, event-driven)
pub struct ViewModelState {
    // Existing
    pub packages: Vec<PackageFingerprint>,
    pub uad_ng_lists: Option<UadNgLists>,
    pub active_operations: HashMap<String, OperationProgress>,
    
    // NEW FIELDS
    pub stalkerware_indicators: Option<StalkerwareIndicators>,
    pub vt_scanner_state: Option<VtScannerState>,
    pub ha_scanner_state: Option<HaScannerState>,
    pub cached_metadata: MetadataCache,  // New unified cache
}
```

### Architecture After Migration

```
UI Thread                          Background Thread (smol)
┌─────────────────┐               ┌──────────────────────┐
│ Tab UI          │               │ DebloatActor         │
│  - render()     │──commands──>  │  - emits events      │
│  - poll events  │<──events────  │  - updates via       │
│                 │               │    MetadataActor     │
├─────────────────┤               ├──────────────────────┤
│ ViewModel       │               │ ScanActor            │
│  .state         │               │  - emits scanner     │
│  .poll_events() │               │    state events      │
│  .send_cmd()    │               ├──────────────────────┤
└─────────────────┘               │ MetadataActor        │
                                  │  - caches metadata   │
┌─────────────────┐               │  - emits cache evts  │
│ TextureCache    │               ├──────────────────────┤
│ (fka SharedStore)│              │ AppsActor            │
│  - textures only│               └──────────────────────┘
└─────────────────┘
```

## 2. ViewModel.state Extensions

### New State Fields

```rust
// mobile/src/viewmodel/common.rs

pub struct ViewModelState {
    // === Existing fields ===
    pub packages: Vec<PackageFingerprint>,
    pub uad_ng_lists: Option<UadNgLists>,
    pub active_operations: HashMap<String, OperationProgress>,
    
    // === NEW: Scanner states ===
    pub vt_scanner_state: Option<VtScannerState>,
    pub ha_scanner_state: Option<HaScannerState>,
    
    // === NEW: Stalkerware ===
    pub stalkerware_indicators: Option<StalkerwareIndicators>,
    
    // === NEW: Metadata cache ===
    pub cached_metadata: MetadataCache,
}

/// Unified metadata cache for all sources
#[derive(Default)]
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
```

### State Access Patterns

```rust
// Tabs read state directly from ViewModel
if let Some(ref vm) = app.viewmodel {
    // Scanner state
    if let Some(vt_state) = &vm.state.vt_scanner_state {
        ui.label(format!("Scan progress: {}/{}", vt_state.scanned, vt_state.total));
    }
    
    // Metadata cache
    if let Some(app_info) = vm.state.cached_metadata.get_google_play(&pkg_id) {
        ui.label(&app_info.title);
    }
    
    // Stalkerware indicators
    if let Some(indicators) = &vm.state.stalkerware_indicators {
        // Check package against indicators
    }
}
```

## 3. Event System Updates

### New Events for Scanner States

```rust
// mobile/src/viewmodel/scan.rs

#[derive(Debug, Clone)]
pub enum ScanEvent {
    // === Existing ===
    ScanStarted { scanner: ScannerType },
    ScanProgress { scanner: ScannerType, current: usize, total: usize },
    ScanComplete { scanner: ScannerType, results: Vec<ScanResult> },
    ScanCancelled { scanner: ScannerType },
    Error { scanner: ScannerType, error: String },
    
    // === NEW: Scanner state updates ===
    VirusTotalStateUpdated(VtScannerState),
    HybridAnalysisStateUpdated(HaScannerState),
}
```

### New Events for Metadata

```rust
// mobile/src/viewmodel/metadata.rs

#[derive(Debug, Clone)]
pub enum MetadataEvent {
    // === NEW: Cache updates ===
    GooglePlayCached { pkg_id: String, app: GooglePlayApp },
    FDroidCached { pkg_id: String, app: FDroidApp },
    ApkMirrorCached { pkg_id: String, app: ApkMirrorApp },
    AndroidPackageCached { pkg_id: String, app: AndroidPackageInfo },
    
    Error { pkg_id: String, error: String },
}
```

### New Events for Stalkerware

```rust
// mobile/src/viewmodel/debloat.rs

#[derive(Debug, Clone)]
pub enum DebloatEvent {
    // === Existing ===
    PackagesLoaded(Vec<PackageFingerprint>),
    UadNgListsLoaded(UadNgLists),
    BatchProgress { operation: String, progress: f32, current: usize, total: usize },
    BatchComplete { operation: String, succeeded: Vec<String>, failed: Vec<String> },
    Error { operation: String, error: String },
    
    // === NEW ===
    StalkerwareIndicatorsLoaded(StalkerwareIndicators),
}
```

### Event Application in ViewModel

```rust
// mobile/src/viewmodel/mod.rs

fn apply_event(&mut self, event: &ViewModelEvent, ctx: &eframe::egui::Context) {
    match event {
        // === Existing event handling ===
        ViewModelEvent::Debloat(DebloatEvent::PackagesLoaded(packages)) => {
            self.state.packages = packages.clone();
        }
        
        // === NEW: Scanner state events ===
        ViewModelEvent::Scan(ScanEvent::VirusTotalStateUpdated(state)) => {
            self.state.vt_scanner_state = Some(state.clone());
            ctx.request_repaint();
        }
        ViewModelEvent::Scan(ScanEvent::HybridAnalysisStateUpdated(state)) => {
            self.state.ha_scanner_state = Some(state.clone());
            ctx.request_repaint();
        }
        
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
        
        // === NEW: Stalkerware ===
        ViewModelEvent::Debloat(DebloatEvent::StalkerwareIndicatorsLoaded(indicators)) => {
            self.state.stalkerware_indicators = Some(indicators.clone());
        }
        
        _ => {}
    }
}
```

## 4. Integration Test Structure

### Test File Organization

```
mobile/tests/
├── integration/
│   ├── mod.rs
│   ├── scanner_migration_test.rs      # Scanner state tests
│   ├── metadata_migration_test.rs     # Metadata cache tests
│   └── stalkerware_migration_test.rs  # Stalkerware tests
```

### Test Strategy

Each test file covers **end-to-end flows** with real components (no mocks).

#### Scanner Migration Tests

```rust
// mobile/tests/integration/scanner_migration_test.rs

#[test]
fn test_virustotal_state_in_viewmodel() {
    // Setup: Create ViewModel with real smol runtime
    let ctx = egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Action: Start VirusTotal scan
    vm.run_virustotal("device_id".into(), "api_key".into(), false).unwrap();
    
    // Verify: Scanner state appears in ViewModel.state
    std::thread::sleep(Duration::from_millis(100)); // Allow event processing
    vm.poll_events(&ctx);
    
    assert!(vm.state.vt_scanner_state.is_some());
    let state = vm.state.vt_scanner_state.as_ref().unwrap();
    assert_eq!(state.scanned, 0);
    
    // Verify: NOT in SharedStore anymore
    let shared_store = get_shared_store();
    assert!(shared_store.vt_scanner_state.lock().unwrap().is_none());
}

#[test]
fn test_hybridanalysis_state_in_viewmodel() {
    // Similar pattern for HybridAnalysis
}

#[test]
fn test_scan_cancellation_clears_state() {
    // Test that cancelling clears state properly
}
```

#### Metadata Migration Tests

```rust
// mobile/tests/integration/metadata_migration_test.rs

#[test]
fn test_google_play_metadata_cached_in_viewmodel() {
    let ctx = egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Action: Fetch Google Play metadata
    vm.fetch_google_play_metadata("com.example.app".into()).unwrap();
    
    // Wait for background fetch
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);
    
    // Verify: Metadata in ViewModel cache
    assert!(vm.state.cached_metadata.get_google_play("com.example.app").is_some());
    
    // Verify: NOT in SharedStore
    let shared_store = get_shared_store();
    assert!(shared_store.cached_google_play_apps.lock().unwrap().is_empty());
}

#[test]
fn test_fdroid_metadata_cached_in_viewmodel() { /* similar */ }

#[test]
fn test_apkmirror_metadata_cached_in_viewmodel() { /* similar */ }

#[test]
fn test_android_package_metadata_cached_in_viewmodel() { /* similar */ }

#[test]
fn test_metadata_cache_persists_across_tabs() {
    // Verify metadata cached once is available everywhere
}
```

#### Stalkerware Migration Tests

```rust
// mobile/tests/integration/stalkerware_migration_test.rs

#[test]
fn test_stalkerware_indicators_in_viewmodel() {
    let ctx = egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Action: Load stalkerware indicators (via debloat command)
    vm.load_uad_ng_lists().unwrap(); // Should also load stalkerware
    
    std::thread::sleep(Duration::from_millis(200));
    vm.poll_events(&ctx);
    
    // Verify: Indicators in ViewModel
    assert!(vm.state.stalkerware_indicators.is_some());
    
    // Verify: NOT in SharedStore
    let shared_store = get_shared_store();
    assert!(shared_store.stalkerware_indicators.lock().unwrap().is_none());
}
```

### Test Execution Order

1. **Write all tests first** (they all fail - RED)
2. **Migrate scanner states** → tests pass (GREEN)
3. **Migrate metadata cache** → tests pass (GREEN)
4. **Migrate stalkerware** → tests pass (GREEN)
5. **Refactor SharedStore** → all tests still pass (REFACTOR)

## 5. Migration Phases (Vertical Slices)

### Phase 0: Setup (Write All Tests)

**Goal:** Create complete integration test suite

**Steps:**
1. Create test directory structure
2. Write scanner migration tests (all failing)
3. Write metadata migration tests (all failing)
4. Write stalkerware migration tests (all failing)
5. Run: `cargo test --test integration` → expect all RED ❌

**Deliverable:** Complete test suite (all failing)

**Files Modified:**
- `mobile/tests/integration/scanner_migration_test.rs` (new)
- `mobile/tests/integration/metadata_migration_test.rs` (new)
- `mobile/tests/integration/stalkerware_migration_test.rs` (new)

---

### Phase 1: Migrate Scanner States

**Goal:** Move VT/HA scanner states to ViewModel.state

#### Step 1.1: Extend ViewModel.state

**File:** `mobile/src/viewmodel/common.rs`

```rust
pub struct ViewModelState {
    // ... existing fields
    pub vt_scanner_state: Option<VtScannerState>,
    pub ha_scanner_state: Option<HaScannerState>,
}
```

#### Step 1.2: Add Events

**File:** `mobile/src/viewmodel/scan.rs`

```rust
pub enum ScanEvent {
    // ... existing
    VirusTotalStateUpdated(VtScannerState),
    HybridAnalysisStateUpdated(HaScannerState),
}
```

#### Step 1.3: Update ScanActor

**File:** `mobile/src/viewmodel/scan.rs`

```rust
// In ScanActor::run()
async fn run(self) {
    // When scanner state changes, emit event
    self.event_tx.send(ViewModelEvent::Scan(
        ScanEvent::VirusTotalStateUpdated(new_state)
    )).await.ok();
}
```

#### Step 1.4: Update ViewModel Event Handler

**File:** `mobile/src/viewmodel/mod.rs`

```rust
fn apply_event(&mut self, event: &ViewModelEvent, ctx: &Context) {
    match event {
        ViewModelEvent::Scan(ScanEvent::VirusTotalStateUpdated(state)) => {
            self.state.vt_scanner_state = Some(state.clone());
            ctx.request_repaint();
        }
        ViewModelEvent::Scan(ScanEvent::HybridAnalysisStateUpdated(state)) => {
            self.state.ha_scanner_state = Some(state.clone());
            ctx.request_repaint();
        }
        // ... rest
    }
}
```

#### Step 1.5: Update Scan Tab

**File:** `mobile/src/tab_scan_control.rs`

Replace SharedStore access with ViewModel.state:

```rust
// OLD:
let shared_store = get_shared_store();
if let Some(vt_state) = shared_store.vt_scanner_state.lock().unwrap().as_ref() {
    // ...
}

// NEW:
if let Some(ref vm) = app.viewmodel {
    if let Some(vt_state) = &vm.state.vt_scanner_state {
        // ...
    }
}
```

#### Step 1.6: Remove from SharedStore

**File:** `mobile/src/shared_store_stt.rs`

```rust
pub struct SharedStore {
    // DELETE these fields:
    // pub vt_scanner_state: Mutex<Option<VtScannerState>>,
    // pub ha_scanner_state: Mutex<Option<HaScannerState>>,
}
```

**Verification:** `cargo test --test integration::scanner_migration_test` → PASS ✅

**Deliverable:** Scanner states migrated, tests green

**Commit:** `feat: migrate scanner states to ViewModel`

**Files Modified:**
- `mobile/src/viewmodel/common.rs`
- `mobile/src/viewmodel/scan.rs`
- `mobile/src/viewmodel/mod.rs`
- `mobile/src/tab_scan_control.rs`
- `mobile/src/shared_store_stt.rs`

---

### Phase 2: Migrate Metadata Cache

**Goal:** Move all cached app metadata to ViewModel.state

#### Step 2.1: Extend ViewModel.state

**File:** `mobile/src/viewmodel/common.rs`

```rust
pub struct ViewModelState {
    // ... existing
    pub cached_metadata: MetadataCache,
}

#[derive(Default)]
pub struct MetadataCache {
    pub google_play: HashMap<String, GooglePlayApp>,
    pub fdroid: HashMap<String, FDroidApp>,
    pub apkmirror: HashMap<String, ApkMirrorApp>,
    pub android_package: HashMap<String, AndroidPackageInfo>,
}
```

#### Step 2.2: Add Metadata Events

**File:** `mobile/src/viewmodel/metadata.rs`

```rust
pub enum MetadataEvent {
    GooglePlayCached { pkg_id: String, app: GooglePlayApp },
    FDroidCached { pkg_id: String, app: FDroidApp },
    ApkMirrorCached { pkg_id: String, app: ApkMirrorApp },
    AndroidPackageCached { pkg_id: String, app: AndroidPackageInfo },
    Error { pkg_id: String, error: String },
}
```

#### Step 2.3: Update MetadataActor

**File:** `mobile/src/viewmodel/metadata.rs`

```rust
async fn handle_fetch_google_play(&self, package: String) {
    match fetch_google_play_info(&package).await {
        Ok(app) => {
            self.event_tx.send(ViewModelEvent::Metadata(
                MetadataEvent::GooglePlayCached { pkg_id: package, app }
            )).await.ok();
        }
        Err(e) => { /* emit error event */ }
    }
}
// Similar for FDroid, ApkMirror, AndroidPackage
```

#### Step 2.4: Update ViewModel Event Handler

**File:** `mobile/src/viewmodel/mod.rs`

```rust
ViewModelEvent::Metadata(MetadataEvent::GooglePlayCached { pkg_id, app }) => {
    self.state.cached_metadata.google_play.insert(pkg_id.clone(), app.clone());
    ctx.request_repaint();
}
// Similar for other metadata types
```

#### Step 2.5: Update All Tabs

**Files:** `mobile/src/tab_debloat_control.rs`, `mobile/src/tab_scan_control.rs`, etc.

```rust
// OLD:
let shared_store = get_shared_store();
if let Some(app_info) = shared_store.cached_google_play_apps.lock().unwrap().get(&pkg_id) {
    // ...
}

// NEW:
if let Some(ref vm) = app.viewmodel {
    if let Some(app_info) = vm.state.cached_metadata.get_google_play(&pkg_id) {
        // ...
    }
}
```

#### Step 2.6: Remove from SharedStore

**File:** `mobile/src/shared_store_stt.rs`

```rust
// DELETE:
// pub cached_google_play_apps: Mutex<HashMap<String, GooglePlayApp>>,
// pub cached_fdroid_apps: Mutex<HashMap<String, FDroidApp>>,
// pub cached_apkmirror_apps: Mutex<HashMap<String, ApkMirrorApp>>,
// pub cached_android_package_apps: Mutex<HashMap<String, AndroidPackageInfo>>,
```

**Verification:** `cargo test --test integration::metadata_migration_test` → PASS ✅

**Deliverable:** Metadata cache migrated, tests green

**Commit:** `feat: migrate metadata cache to ViewModel`

**Files Modified:**
- `mobile/src/viewmodel/common.rs`
- `mobile/src/viewmodel/metadata.rs`
- `mobile/src/viewmodel/mod.rs`
- `mobile/src/tab_debloat_control.rs`
- `mobile/src/tab_scan_control.rs`
- `mobile/src/shared_store_stt.rs`

---

### Phase 3: Migrate Stalkerware Indicators

**Goal:** Move stalkerware indicators to ViewModel.state

#### Step 3.1: Extend ViewModel.state

**File:** `mobile/src/viewmodel/common.rs`

```rust
pub struct ViewModelState {
    // ... existing
    pub stalkerware_indicators: Option<StalkerwareIndicators>,
}
```

#### Step 3.2: Add Event

**File:** `mobile/src/viewmodel/debloat.rs`

```rust
pub enum DebloatEvent {
    // ... existing
    StalkerwareIndicatorsLoaded(StalkerwareIndicators),
}
```

#### Step 3.3: Update DebloatActor

**File:** `mobile/src/viewmodel/debloat.rs`

```rust
async fn handle_load_uad_ng_lists(&self) {
    // ... load UAD lists
    // Also load stalkerware
    if let Ok(indicators) = load_stalkerware_indicators().await {
        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::StalkerwareIndicatorsLoaded(indicators)
        )).await.ok();
    }
}
```

#### Step 3.4: Update ViewModel Event Handler

**File:** `mobile/src/viewmodel/mod.rs`

```rust
ViewModelEvent::Debloat(DebloatEvent::StalkerwareIndicatorsLoaded(indicators)) => {
    self.state.stalkerware_indicators = Some(indicators.clone());
}
```

#### Step 3.5: Update Debloat Tab

**File:** `mobile/src/tab_debloat_control.rs`

```rust
// OLD:
let shared_store = get_shared_store();
if let Some(indicators) = shared_store.stalkerware_indicators.lock().unwrap().as_ref() {
    // ...
}

// NEW:
if let Some(ref vm) = app.viewmodel {
    if let Some(indicators) = &vm.state.stalkerware_indicators {
        // ...
    }
}
```

#### Step 3.6: Remove from SharedStore

**File:** `mobile/src/shared_store_stt.rs`

```rust
// DELETE:
// pub stalkerware_indicators: Mutex<Option<StalkerwareIndicators>>,
```

**Verification:** `cargo test --test integration::stalkerware_migration_test` → PASS ✅

**Deliverable:** Stalkerware migrated, tests green

**Commit:** `feat: migrate stalkerware indicators to ViewModel`

**Files Modified:**
- `mobile/src/viewmodel/common.rs`
- `mobile/src/viewmodel/debloat.rs`
- `mobile/src/viewmodel/mod.rs`
- `mobile/src/tab_debloat_control.rs`
- `mobile/src/shared_store_stt.rs`

---

### Phase 4: Refactor SharedStore to Texture-Only

**Goal:** Delete SharedStore, create minimal TextureCache

#### Step 4.1: Create TextureCache

**File:** `mobile/src/texture_cache.rs` (new)

```rust
use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

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

pub fn get_texture_cache() -> Arc<TextureCache> {
    TEXTURE_CACHE.get_or_init(|| Arc::new(TextureCache::new())).clone()
}
```

#### Step 4.2: Update All Texture Access

**Files:** All files that access SharedStore for textures

```rust
// OLD:
let shared_store = get_shared_store();
if let Some(texture) = shared_store.google_play_textures.lock().unwrap().get(pkg_id) {
    // ...
}

// NEW:
let cache = get_texture_cache();
if let Some(texture) = cache.google_play_textures.lock().unwrap().get(pkg_id) {
    // ...
}
```

#### Step 4.3: Delete Old SharedStore Files

```bash
git rm mobile/src/shared_store.rs
git rm mobile/src/shared_store_stt.rs
```

#### Step 4.4: Update lib.rs

**File:** `mobile/src/lib.rs`

```rust
// OLD:
pub mod shared_store;
pub mod shared_store_stt;

// NEW:
pub mod texture_cache;
```

#### Step 4.5: Update All Imports

**Files:** 15+ files that imported SharedStore

```rust
// OLD:
use crate::shared_store_stt::{get_shared_store, SharedStore};

// NEW:
use crate::texture_cache::{get_texture_cache, TextureCache};
```

**Verification:** 
- `cargo test --test integration` → ALL PASS ✅
- `cargo build --release` → SUCCESS ✅

**Deliverable:** SharedStore deleted, only TextureCache remains

**Commit:** `refactor: convert SharedStore to texture-only TextureCache`

**Files Modified:**
- `mobile/src/texture_cache.rs` (new)
- `mobile/src/lib.rs`
- `mobile/src/shared_store.rs` (deleted)
- `mobile/src/shared_store_stt.rs` (deleted)
- 15+ files with import updates

---

## 6. Error Handling

### Error Propagation Pattern

All errors flow through the event system:

```rust
// In actors: Convert errors to events
async fn handle_fetch_metadata(&self, package: String) {
    match fetch_google_play_info(&package).await {
        Ok(app) => {
            // Success: emit cache event
            self.event_tx.send(ViewModelEvent::Metadata(
                MetadataEvent::GooglePlayCached { pkg_id: package, app }
            )).await.ok();
        }
        Err(e) => {
            // Error: emit error event
            log::error!("Failed to fetch metadata for {}: {}", package, e);
            self.event_tx.send(ViewModelEvent::Metadata(
                MetadataEvent::Error { 
                    pkg_id: package, 
                    error: e.to_string() 
                }
            )).await.ok();
        }
    }
}
```

### UI Error Handling

```rust
// Tabs handle error events gracefully
fn handle_metadata_event(&mut self, event: MetadataEvent) {
    match event {
        MetadataEvent::GooglePlayCached { pkg_id, app } => {
            // Update UI with metadata
        }
        MetadataEvent::Error { pkg_id, error } => {
            // Show error in UI (toast, status label, etc.)
            self.error_message = Some(format!("Failed to load {}: {}", pkg_id, error));
        }
    }
}
```

### Graceful Degradation

During migration, handle missing data gracefully:

```rust
if let Some(ref vm) = app.viewmodel {
    match vm.state.cached_metadata.get_google_play(&pkg_id) {
        Some(app_info) => {
            // Show full metadata
            ui.label(&app_info.title);
        }
        None => {
            // Not cached yet - show loading state
            ui.spinner();
            ui.label("Loading...");
            
            // Trigger fetch if not already in progress
            if !self.pending_fetches.contains(&pkg_id) {
                vm.fetch_google_play_metadata(pkg_id.clone()).ok();
                self.pending_fetches.insert(pkg_id.clone());
            }
        }
    }
}
```

### Rollback Strategy

If a phase fails:

1. **Revert the commit** for that phase
2. **Review test failures** to identify root cause
3. **Fix the issue** in isolation
4. **Re-run tests** before proceeding

```bash
# If Phase 2 (metadata) fails:
git reset --hard HEAD~1  # Revert Phase 2 commit
# Fix issue
cargo test --test integration::metadata_migration_test
# Re-commit when passing
```

### Validation Checkpoints

After each phase:

```bash
# 1. All integration tests pass
cargo test --test integration

# 2. Full build succeeds
cargo build --release

# 3. Manual smoke test (optional but recommended)
cargo run
```

## 7. Testing & Verification

### Test Execution Strategy

#### Continuous Testing During Migration

```bash
# After each phase, run phase-specific tests
cargo test --test integration::scanner_migration_test    # After Phase 1
cargo test --test integration::metadata_migration_test   # After Phase 2
cargo test --test integration::stalkerware_migration_test # After Phase 3

# Final verification: ALL tests
cargo test --test integration
```

#### Full Build Verification

```bash
# Debug build
cargo build

# Release build (catches optimization issues)
cargo build --release

# Run clippy
cargo clippy -- -D warnings
```

#### Manual Smoke Testing

After each phase, verify in running app:

**Phase 1 (Scanner States):**
- Start VirusTotal scan
- Verify progress updates in UI
- Cancel scan mid-way
- Verify state clears properly

**Phase 2 (Metadata):**
- View package list in debloat tab
- Verify Google Play metadata loads
- Verify F-Droid metadata loads
- Check metadata persists when switching tabs

**Phase 3 (Stalkerware):**
- Load UAD lists
- Verify stalkerware detection works
- Check indicators highlight suspicious apps

**Phase 4 (SharedStore Removal):**
- All above still work
- Textures still render correctly
- No SharedStore references in logs

### Success Criteria Checklist

**Phase 1: Scanner States**
- [ ] `scanner_migration_test.rs` all tests pass
- [ ] Scan tab UI shows scanner state from ViewModel
- [ ] SharedStore.vt_scanner_state removed
- [ ] SharedStore.ha_scanner_state removed
- [ ] No compilation errors
- [ ] Commit: "feat: migrate scanner states to ViewModel"

**Phase 2: Metadata Cache**
- [ ] `metadata_migration_test.rs` all tests pass
- [ ] All tabs access metadata from ViewModel.state.cached_metadata
- [ ] SharedStore.cached_*_apps removed (4 fields)
- [ ] No compilation errors
- [ ] Commit: "feat: migrate metadata cache to ViewModel"

**Phase 3: Stalkerware**
- [ ] `stalkerware_migration_test.rs` all tests pass
- [ ] Stalkerware indicators in ViewModel.state
- [ ] SharedStore.stalkerware_indicators removed
- [ ] No compilation errors
- [ ] Commit: "feat: migrate stalkerware indicators to ViewModel"

**Phase 4: SharedStore Refactor**
- [ ] All integration tests still pass
- [ ] SharedStore renamed to TextureCache
- [ ] Only texture fields remain
- [ ] All imports updated (15+ files)
- [ ] shared_store.rs deleted
- [ ] shared_store_stt.rs deleted
- [ ] No compilation errors
- [ ] Commit: "refactor: convert SharedStore to texture-only TextureCache"

**Final Verification**
- [ ] `cargo test --test integration` → 100% pass
- [ ] `cargo build --release` → success
- [ ] `cargo clippy` → no warnings
- [ ] Manual smoke test → all features work
- [ ] Git history clean (4 commits)
- [ ] Documentation updated

### Documentation Updates

After migration complete:

```bash
# Update these files:
docs/architecture.md              # SharedStore → ViewModel + TextureCache
docs/mvvm-migration-complete.md   # Add Phase 2 completion notes
README.md                         # Update if SharedStore mentioned
```

## Summary

This design migrates all business state from SharedStore to ViewModel using a disciplined TDD approach with vertical slices. Each phase is independently verifiable with clear success criteria. The result is a cleaner MVVM architecture with centralized state management and event-driven UI updates, while respecting egui's texture lifetime constraints by keeping a minimal TextureCache.

**Total Estimated LOC Changes:**
- Tests: ~400 lines (new)
- ViewModel.state: ~100 lines (new)
- Events: ~80 lines (new)
- Actor updates: ~200 lines (modified)
- Tab updates: ~300 lines (modified)
- SharedStore → TextureCache: ~150 lines (deleted), ~50 lines (new)

**Total: ~1,280 lines across 4 commits**

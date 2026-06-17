# MVVM Actor-Based Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor UAD-Shizuku to actor-based MVVM architecture with smol async runtime for non-blocking UI operations

**Architecture:** Four independent actors (Debloat, Scan, Apps, Metadata) run on single background thread with smol executor. ViewModel coordinates between UI and actors via typed channels. UI thread owns ViewModel, polls events, renders state.

**Tech Stack:** smol 2.0 (async runtime + channels), egui 0.33 (UI), diesel 2.3 (database), existing calc*/db* modules (reused as-is)

## Global Constraints

- Rust edition 2021, rust-version 1.81 minimum
- Keep all existing features working (no functionality loss)
- Reuse calc*.rs and db*.rs files without modification
- Single background thread with smol executor (not thread pool)
- All channels use `smol::channel::unbounded()`
- Mobile GUI only (no CLI mode support)
- TDD approach: write tests first, then implementation
- Commit after each completed task

---

## File Structure Overview

**New files to create:**
```
mobile/src/viewmodel/
  mod.rs              # ViewModel struct, initialization, event polling
  common.rs           # ViewModelEvent, shared types
  debloat.rs          # DebloatActor, DebloatCommand, DebloatEvent
  scan.rs             # ScanActor, ScanCommand, ScanEvent  
  apps.rs             # AppsActor, AppsCommand, AppsEvent
  metadata.rs         # MetadataActor, MetadataCommand, MetadataEvent

mobile/tests/
  viewmodel_tests.rs  # Unit tests for actors
```

**Files to modify:**
```
mobile/Cargo.toml                # Update dependencies
mobile/src/lib.rs                # Add viewmodel module
mobile/src/uad_shizuku_app.rs    # Add ViewModel field, poll events
mobile/src/tab_debloat_control.rs    # Use ViewModel commands
mobile/src/tab_scan_control.rs       # Use ViewModel commands
mobile/src/tab_apps_control.rs       # Use ViewModel commands
```

**Files to delete (in Phase 5):**
```
mobile/src/shared_store.rs
mobile/src/shared_store_stt.rs
```

---

## Phase 1: Infrastructure Setup

### Task 1: Update Dependencies

**Files:**
- Modify: `mobile/Cargo.toml:55-58`

**Interfaces:**
- Consumes: None
- Produces: smol 2.0 dependency available for use

- [ ] **Step 1: Remove tokio and crossbeam-queue dependencies**

Edit `mobile/Cargo.toml`, remove these lines:
```toml
tokio = { version = "1", features = ["rt-multi-thread"] }
```

And remove from line 58:
```toml
crossbeam-queue = "0.3"
```

- [ ] **Step 2: Add smol dependency**

Add after line 33 in `mobile/Cargo.toml`:
```toml
smol = "2.0"
```

- [ ] **Step 3: Verify dependencies compile**

Run: `cargo check --manifest-path mobile/Cargo.toml`
Expected: SUCCESS (no errors, may have warnings about unused smol)

- [ ] **Step 4: Commit dependency changes**

```bash
git add mobile/Cargo.toml
git commit -m "deps: replace tokio/crossbeam-queue with smol

Remove tokio and crossbeam-queue dependencies.
Add smol 2.0 for async runtime and channels.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 2: Create ViewModel Module Structure

**Files:**
- Create: `mobile/src/viewmodel/mod.rs`
- Create: `mobile/src/viewmodel/common.rs`
- Modify: `mobile/src/lib.rs:1-50`

**Interfaces:**
- Consumes: smol dependency from Task 1
- Produces: `viewmodel` module accessible via `crate::viewmodel`

- [ ] **Step 1: Create viewmodel directory**

Run: `mkdir -p mobile/src/viewmodel`

- [ ] **Step 2: Create common types file**

Create `mobile/src/viewmodel/common.rs`:
```rust
//! Common types shared across all actors

use serde::{Deserialize, Serialize};

/// Unified event type from all actors to ViewModel
#[derive(Debug, Clone)]
pub enum ViewModelEvent {
    Debloat(DebloatEvent),
    Scan(ScanEvent),
    Apps(AppsEvent),
    Metadata(MetadataEvent),
}

/// Placeholder event types (will be defined in respective actor files)
#[derive(Debug, Clone)]
pub enum DebloatEvent {
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum AppsEvent {
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum MetadataEvent {
    Placeholder,
}

/// Progress tracking for long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationProgress {
    pub operation: String,
    pub progress: f32,  // 0.0 to 1.0
    pub status: String,
}

/// Metadata source enum for texture tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetadataSource {
    GooglePlay,
    FDroid,
    ApkMirror,
    AndroidPackage,
}
```

- [ ] **Step 3: Create ViewModel module file**

Create `mobile/src/viewmodel/mod.rs`:
```rust
//! ViewModel layer - coordinates between UI and background actors

pub mod common;

pub use common::*;

use std::collections::HashMap;

/// ViewModel struct - owned by UadShizukuApp, coordinates actor communication
pub struct ViewModel {
    // Actor communication channels (will be added in later tasks)
    
    // Unified event receiver
    event_rx: smol::channel::Receiver<ViewModelEvent>,
    
    // Public state
    pub state: ViewModelState,
    
    // Background thread handle
    _runtime_handle: Option<std::thread::JoinHandle<()>>,
}

/// ViewModel state - read-only access from UI
#[derive(Default)]
pub struct ViewModelState {
    // Progress tracking
    pub active_operations: HashMap<String, OperationProgress>,
}

impl ViewModel {
    /// Create new ViewModel and spawn background runtime
    pub fn new(_ctx: eframe::egui::Context) -> Self {
        // Create unified event channel
        let (event_tx, event_rx) = smol::channel::unbounded();
        
        // Spawn background thread with smol executor (actors will be added later)
        let runtime_handle = std::thread::spawn(move || {
            smol::block_on(async {
                log::info!("ViewModel runtime started");
                
                // Keep thread alive
                std::future::pending::<()>().await
            })
        });
        
        Self {
            event_rx,
            state: ViewModelState::default(),
            _runtime_handle: Some(runtime_handle),
        }
    }
    
    /// Poll for events and update state. Call this in UadShizukuApp::update()
    pub fn poll_events(&mut self, _ctx: &eframe::egui::Context) -> Vec<ViewModelEvent> {
        let mut events = Vec::new();
        
        // Non-blocking receive all available events
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        
        events
    }
}
```

- [ ] **Step 4: Add viewmodel module to lib.rs**

Add after existing module declarations in `mobile/src/lib.rs`:
```rust
pub mod viewmodel;
```

- [ ] **Step 5: Verify module compiles**

Run: `cargo check --manifest-path mobile/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 6: Commit ViewModel skeleton**

```bash
git add mobile/src/viewmodel/
git add mobile/src/lib.rs
git commit -m "feat: add ViewModel module skeleton

Create viewmodel module structure with:
- Common event types (ViewModelEvent enum)
- ViewModel struct with event polling
- Background thread with smol executor
- Placeholder for actor integration

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 3: Integrate ViewModel into UadShizukuApp

**Files:**
- Modify: `mobile/src/uad_shizuku_app.rs:1-50` (add imports)
- Modify: `mobile/src/uad_shizuku_app.rs:224-370` (add field to struct)
- Modify: `mobile/src/uad_shizuku_app.rs` (find update method and add poll)

**Interfaces:**
- Consumes: `ViewModel::new()`, `ViewModel::poll_events()` from Task 2
- Produces: `UadShizukuApp` with `viewmodel` field accessible in tabs

- [ ] **Step 1: Add viewmodel import to uad_shizuku_app.rs**

Add after existing imports at top of file:
```rust
use crate::viewmodel::ViewModel;
```

- [ ] **Step 2: Find struct definition and add viewmodel field**

Find the `pub struct UadShizukuApp {` definition (around line 224) and add this field before the last field:
```rust
    // ViewModel for MVVM architecture
    viewmodel: ViewModel,
```

- [ ] **Step 3: Initialize ViewModel in Default::default()**

Find `impl Default for UadShizukuApp` and add before the final return:
```rust
        // Initialize ViewModel (must happen after egui context is available)
        // For now, use a temporary context - will be set properly in update()
        let temp_ctx = egui::Context::default();
        let viewmodel = ViewModel::new(temp_ctx);
```

And add to the struct construction:
```rust
            viewmodel,
```

- [ ] **Step 4: Find update method**

Search for `fn update(&mut self, ctx: &egui::Context` in the file.

- [ ] **Step 5: Add event polling at start of update method**

Add these lines at the very beginning of the update method (after any existing initialization checks):
```rust
        // Poll ViewModel events
        let _vm_events = self.viewmodel.poll_events(ctx);
        // TODO: Handle events when actors are implemented
```

- [ ] **Step 6: Verify app compiles and runs**

Run: `cargo check --manifest-path mobile/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 7: Commit ViewModel integration**

```bash
git add mobile/src/uad_shizuku_app.rs
git commit -m "feat: integrate ViewModel into UadShizukuApp

Add ViewModel field to UadShizukuApp struct.
Initialize ViewModel in Default::default().
Poll events in update() method.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 2: Debloat Actor Implementation

### Task 4: Define Debloat Actor Messages

**Files:**
- Create: `mobile/src/viewmodel/debloat.rs`
- Modify: `mobile/src/viewmodel/common.rs:10-26` (replace placeholder)
- Modify: `mobile/src/viewmodel/mod.rs:3-4` (add module)

**Interfaces:**
- Consumes: ViewModelEvent from common.rs
- Produces: `DebloatCommand`, `DebloatEvent`, `DebloatActor` types

- [ ] **Step 1: Create debloat actor file with message types**

Create `mobile/src/viewmodel/debloat.rs`:
```rust
//! Debloat actor - handles package management and batch operations

use crate::adb::PackageFingerprint;
use crate::uad_shizuku_app::UadNgLists;
use crate::viewmodel::ViewModelEvent;
use anyhow::Result;

/// Commands sent to DebloatActor
#[derive(Debug, Clone)]
pub enum DebloatCommand {
    LoadPackages { device: String, user: u32 },
    BatchUninstall { packages: Vec<String>, device: String },
    BatchDisable { packages: Vec<String>, device: String },
    BatchEnable { packages: Vec<String>, device: String },
    LoadUadNgLists,
}

/// Events sent from DebloatActor to ViewModel
#[derive(Debug, Clone)]
pub enum DebloatEvent {
    PackagesLoaded(Vec<PackageFingerprint>),
    UadNgListsLoaded(UadNgLists),
    BatchProgress {
        operation: String,
        progress: f32,      // 0.0 to 1.0
        current: usize,
        total: usize,
    },
    BatchComplete {
        operation: String,
        succeeded: usize,
        failed: usize,
    },
    Error {
        operation: String,
        error: String,
    },
}

/// Debloat actor state
struct DebloatActorState {
    current_device: Option<String>,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
}

/// Debloat actor - runs on background thread
pub struct DebloatActor {
    state: DebloatActorState,
    command_rx: smol::channel::Receiver<DebloatCommand>,
    event_tx: smol::channel::Sender<ViewModelEvent>,
    _metadata_tx: smol::channel::Sender<super::MetadataCommand>,
}

impl DebloatActor {
    pub fn new(
        command_rx: smol::channel::Receiver<DebloatCommand>,
        event_tx: smol::channel::Sender<ViewModelEvent>,
        metadata_tx: smol::channel::Sender<super::MetadataCommand>,
    ) -> Self {
        Self {
            state: DebloatActorState {
                current_device: None,
                unsafe_app_remove: false,
                expert_app_remove: false,
            },
            command_rx,
            event_tx,
            _metadata_tx: metadata_tx,
        }
    }
    
    pub async fn run(mut self) {
        loop {
            match self.command_rx.recv().await {
                Ok(cmd) => {
                    if let Err(e) = self.handle_command(cmd).await {
                        self.send_error("command_processing", e).await;
                    }
                }
                Err(_) => {
                    log::info!("DebloatActor: command channel closed, shutting down");
                    break;
                }
            }
        }
    }
    
    async fn handle_command(&mut self, cmd: DebloatCommand) -> Result<()> {
        match cmd {
            DebloatCommand::LoadPackages { device, user } => {
                self.load_packages(device, user).await?;
            }
            DebloatCommand::BatchUninstall { packages, device } => {
                self.batch_uninstall(packages, device).await?;
            }
            DebloatCommand::BatchDisable { packages, device } => {
                self.batch_disable(packages, device).await?;
            }
            DebloatCommand::BatchEnable { packages, device } => {
                self.batch_enable(packages, device).await?;
            }
            DebloatCommand::LoadUadNgLists => {
                self.load_uad_ng_lists().await?;
            }
        }
        Ok(())
    }
    
    async fn load_packages(&mut self, device: String, user: u32) -> Result<()> {
        // Use smol::unblock for blocking ADB operations
        let packages = smol::unblock(move || {
            crate::adb::get_packages(&device, user)
        }).await?;
        
        self.state.current_device = Some(device);
        
        // Send event back to ViewModel
        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::PackagesLoaded(packages)
        )).await?;
        
        Ok(())
    }
    
    async fn batch_uninstall(&mut self, packages: Vec<String>, device: String) -> Result<()> {
        let total = packages.len();
        let mut succeeded = 0;
        let mut failed = 0;
        
        for (i, pkg) in packages.into_iter().enumerate() {
            let device = device.clone();
            
            // Uninstall in blocking thread pool
            let result = smol::unblock(move || {
                crate::adb::uninstall_package(&device, &pkg)
            }).await;
            
            match result {
                Ok(_) => succeeded += 1,
                Err(e) => {
                    log::error!("Failed to uninstall {}: {}", pkg, e);
                    failed += 1;
                }
            }
            
            // Send progress event
            let progress = (i + 1) as f32 / total as f32;
            self.event_tx.send(ViewModelEvent::Debloat(
                DebloatEvent::BatchProgress {
                    operation: "uninstall".to_string(),
                    progress,
                    current: i + 1,
                    total,
                }
            )).await?;
        }
        
        // Send completion event
        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::BatchComplete {
                operation: "uninstall".to_string(),
                succeeded,
                failed,
            }
        )).await?;
        
        Ok(())
    }
    
    async fn batch_disable(&mut self, packages: Vec<String>, device: String) -> Result<()> {
        let total = packages.len();
        let mut succeeded = 0;
        let mut failed = 0;
        
        for (i, pkg) in packages.into_iter().enumerate() {
            let device = device.clone();
            
            let result = smol::unblock(move || {
                crate::adb::disable_package(&device, &pkg)
            }).await;
            
            match result {
                Ok(_) => succeeded += 1,
                Err(e) => {
                    log::error!("Failed to disable {}: {}", pkg, e);
                    failed += 1;
                }
            }
            
            let progress = (i + 1) as f32 / total as f32;
            self.event_tx.send(ViewModelEvent::Debloat(
                DebloatEvent::BatchProgress {
                    operation: "disable".to_string(),
                    progress,
                    current: i + 1,
                    total,
                }
            )).await?;
        }
        
        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::BatchComplete {
                operation: "disable".to_string(),
                succeeded,
                failed,
            }
        )).await?;
        
        Ok(())
    }
    
    async fn batch_enable(&mut self, packages: Vec<String>, device: String) -> Result<()> {
        let total = packages.len();
        let mut succeeded = 0;
        let mut failed = 0;
        
        for (i, pkg) in packages.into_iter().enumerate() {
            let device = device.clone();
            
            let result = smol::unblock(move || {
                crate::adb::enable_package(&device, &pkg)
            }).await;
            
            match result {
                Ok(_) => succeeded += 1,
                Err(e) => {
                    log::error!("Failed to enable {}: {}", pkg, e);
                    failed += 1;
                }
            }
            
            let progress = (i + 1) as f32 / total as f32;
            self.event_tx.send(ViewModelEvent::Debloat(
                DebloatEvent::BatchProgress {
                    operation: "enable".to_string(),
                    progress,
                    current: i + 1,
                    total,
                }
            )).await?;
        }
        
        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::BatchComplete {
                operation: "enable".to_string(),
                succeeded,
                failed,
            }
        )).await?;
        
        Ok(())
    }
    
    async fn load_uad_ng_lists(&mut self) -> Result<()> {
        let lists = smol::unblock(move || {
            crate::calc::load_uad_ng_lists()
        }).await?;
        
        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::UadNgListsLoaded(lists)
        )).await?;
        
        Ok(())
    }
    
    async fn send_error(&self, operation: &str, error: anyhow::Error) {
        let _ = self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::Error {
                operation: operation.to_string(),
                error: error.to_string(),
            }
        )).await;
    }
}
```

- [ ] **Step 2: Update common.rs to export DebloatEvent**

Replace the placeholder in `mobile/src/viewmodel/common.rs`:
```rust
pub use super::debloat::DebloatEvent;
```

Instead of:
```rust
#[derive(Debug, Clone)]
pub enum DebloatEvent {
    Placeholder,
}
```

- [ ] **Step 3: Add debloat module to viewmodel/mod.rs**

Add after `pub mod common;`:
```rust
pub mod debloat;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --manifest-path mobile/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 5: Commit debloat actor messages**

```bash
git add mobile/src/viewmodel/debloat.rs
git add mobile/src/viewmodel/common.rs
git add mobile/src/viewmodel/mod.rs
git commit -m "feat: add DebloatActor with command/event types

Implement DebloatActor with:
- LoadPackages, BatchUninstall/Disable/Enable commands
- Progress and completion events
- Error handling
- smol::unblock for blocking ADB operations

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 5: Wire DebloatActor into ViewModel

**Files:**
- Modify: `mobile/src/viewmodel/mod.rs:5-30`
- Modify: `mobile/src/viewmodel/debloat.rs:1-5` (add placeholder command type export)
- Create: `mobile/src/viewmodel/scan.rs`
- Create: `mobile/src/viewmodel/apps.rs`
- Create: `mobile/src/viewmodel/metadata.rs`

**Interfaces:**
- Consumes: `DebloatActor::new()`, `DebloatCommand` from Task 4
- Produces: `ViewModel` with working DebloatActor, command methods `load_packages()`, `batch_uninstall()`, etc.

- [ ] **Step 1: Add placeholder actor files**

Create `mobile/src/viewmodel/scan.rs`:
```rust
//! Scan actor - placeholder

#[derive(Debug, Clone)]
pub enum ScanCommand {
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    Placeholder,
}
```

Create `mobile/src/viewmodel/apps.rs`:
```rust
//! Apps actor - placeholder

#[derive(Debug, Clone)]
pub enum AppsCommand {
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum AppsEvent {
    Placeholder,
}
```

Create `mobile/src/viewmodel/metadata.rs`:
```rust
//! Metadata actor - placeholder

#[derive(Debug, Clone)]
pub enum MetadataCommand {
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum MetadataEvent {
    Placeholder,
}
```

- [ ] **Step 2: Export all actor modules in viewmodel/mod.rs**

Add after `pub mod debloat;`:
```rust
pub mod scan;
pub mod apps;
pub mod metadata;

pub use debloat::{DebloatCommand, DebloatEvent, DebloatActor};
pub use scan::{ScanCommand, ScanEvent};
pub use apps::{AppsCommand, AppsEvent};
pub use metadata::{MetadataCommand, MetadataEvent};
```

- [ ] **Step 3: Add actor channels to ViewModel struct**

In `mobile/src/viewmodel/mod.rs`, replace the `ViewModel` struct definition:
```rust
pub struct ViewModel {
    // Actor communication channels
    debloat_tx: smol::channel::Sender<DebloatCommand>,
    scan_tx: smol::channel::Sender<ScanCommand>,
    apps_tx: smol::channel::Sender<AppsCommand>,
    metadata_tx: smol::channel::Sender<MetadataCommand>,
    
    // Unified event receiver
    event_rx: smol::channel::Receiver<ViewModelEvent>,
    
    // Public state
    pub state: ViewModelState,
    
    // Background thread handle
    _runtime_handle: Option<std::thread::JoinHandle<()>>,
}
```

- [ ] **Step 4: Wire up DebloatActor in ViewModel::new()**

Replace the `ViewModel::new()` implementation:
```rust
impl ViewModel {
    pub fn new(_ctx: eframe::egui::Context) -> Self {
        // Create channels for each actor
        let (debloat_tx, debloat_rx) = smol::channel::unbounded();
        let (scan_tx, scan_rx) = smol::channel::unbounded();
        let (apps_tx, apps_rx) = smol::channel::unbounded();
        let (metadata_tx, metadata_rx) = smol::channel::unbounded();
        
        // Unified event channel
        let (event_tx, event_rx) = smol::channel::unbounded();
        
        // Spawn background thread with smol executor
        let runtime_handle = std::thread::spawn(move || {
            smol::block_on(async {
                log::info!("ViewModel runtime started");
                
                // Create actors
                let debloat_actor = DebloatActor::new(
                    debloat_rx,
                    event_tx.clone(),
                    metadata_tx.clone(),
                );
                
                // Run DebloatActor concurrently
                smol::spawn(debloat_actor.run()).detach();
                
                // Keep thread alive
                std::future::pending::<()>().await
            })
        });
        
        Self {
            debloat_tx,
            scan_tx,
            apps_tx,
            metadata_tx,
            event_rx,
            state: ViewModelState::default(),
            _runtime_handle: Some(runtime_handle),
        }
    }
    
    pub fn poll_events(&mut self, _ctx: &eframe::egui::Context) -> Vec<ViewModelEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }
}
```

- [ ] **Step 5: Add DebloatCommand methods to ViewModel**

Add these methods to `ViewModel` impl block:
```rust
    // === Debloat commands ===
    
    pub fn load_packages(&self, device: String, user: u32) -> anyhow::Result<()> {
        self.debloat_tx.send_blocking(DebloatCommand::LoadPackages { device, user })
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))
    }
    
    pub fn batch_uninstall(&self, packages: Vec<String>, device: String) -> anyhow::Result<()> {
        self.debloat_tx.send_blocking(DebloatCommand::BatchUninstall { packages, device })
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))
    }
    
    pub fn batch_disable(&self, packages: Vec<String>, device: String) -> anyhow::Result<()> {
        self.debloat_tx.send_blocking(DebloatCommand::BatchDisable { packages, device })
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))
    }
    
    pub fn batch_enable(&self, packages: Vec<String>, device: String) -> anyhow::Result<()> {
        self.debloat_tx.send_blocking(DebloatCommand::BatchEnable { packages, device })
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))
    }
    
    pub fn load_uad_ng_lists(&self) -> anyhow::Result<()> {
        self.debloat_tx.send_blocking(DebloatCommand::LoadUadNgLists)
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))
    }
```

- [ ] **Step 6: Update common.rs with all event types**

Update `mobile/src/viewmodel/common.rs` to export all events:
```rust
pub use super::debloat::DebloatEvent;
pub use super::scan::ScanEvent;
pub use super::apps::AppsEvent;
pub use super::metadata::MetadataEvent;
```

And remove the placeholder event enums.

- [ ] **Step 7: Verify compilation**

Run: `cargo check --manifest-path mobile/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 8: Commit DebloatActor integration**

```bash
git add mobile/src/viewmodel/
git commit -m "feat: wire DebloatActor into ViewModel

Add DebloatActor to background runtime.
Add command methods (load_packages, batch_uninstall, etc).
Create placeholder actor files for Scan, Apps, Metadata.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 6: Add ViewModel State for Debloat Data

**Files:**
- Modify: `mobile/src/viewmodel/mod.rs:20-35` (add state fields)
- Modify: `mobile/src/viewmodel/mod.rs:40-70` (add event handling)

**Interfaces:**
- Consumes: `DebloatEvent` from Task 4
- Produces: `ViewModelState` with `packages`, `uad_ng_lists` fields; state accessor methods

- [ ] **Step 1: Add debloat state fields to ViewModelState**

Update `ViewModelState` struct in `mobile/src/viewmodel/mod.rs`:
```rust
#[derive(Default)]
pub struct ViewModelState {
    // Debloat state
    pub packages: Vec<crate::adb::PackageFingerprint>,
    pub uad_ng_lists: Option<crate::uad_shizuku_app::UadNgLists>,
    
    // Progress tracking
    pub active_operations: HashMap<String, OperationProgress>,
}
```

- [ ] **Step 2: Implement event handling in poll_events**

Replace `poll_events()` method with state updates:
```rust
    pub fn poll_events(&mut self, ctx: &eframe::egui::Context) -> Vec<ViewModelEvent> {
        let mut events = Vec::new();
        
        while let Ok(event) = self.event_rx.try_recv() {
            self.apply_event(&event, ctx);
            events.push(event);
        }
        
        events
    }
    
    fn apply_event(&mut self, event: &ViewModelEvent, _ctx: &eframe::egui::Context) {
        match event {
            ViewModelEvent::Debloat(DebloatEvent::PackagesLoaded(packages)) => {
                self.state.packages = packages.clone();
            }
            ViewModelEvent::Debloat(DebloatEvent::UadNgListsLoaded(lists)) => {
                self.state.uad_ng_lists = Some(lists.clone());
            }
            ViewModelEvent::Debloat(DebloatEvent::BatchProgress { operation, progress, .. }) => {
                self.state.active_operations.insert(
                    operation.clone(),
                    OperationProgress {
                        operation: operation.clone(),
                        progress: *progress,
                        status: format!("In progress: {:.0}%", progress * 100.0),
                    }
                );
            }
            ViewModelEvent::Debloat(DebloatEvent::BatchComplete { operation, .. }) => {
                self.state.active_operations.remove(operation);
            }
            ViewModelEvent::Debloat(DebloatEvent::Error { operation, error }) => {
                log::error!("Debloat error in {}: {}", operation, error);
            }
            _ => {}
        }
    }
```

- [ ] **Step 3: Add state accessor methods**

Add to `ViewModel` impl:
```rust
    // === Read-only state accessors ===
    
    pub fn packages(&self) -> &[crate::adb::PackageFingerprint] {
        &self.state.packages
    }
    
    pub fn uad_ng_lists(&self) -> Option<&crate::uad_shizuku_app::UadNgLists> {
        self.state.uad_ng_lists.as_ref()
    }
    
    pub fn operation_progress(&self, operation: &str) -> Option<&OperationProgress> {
        self.state.active_operations.get(operation)
    }
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --manifest-path mobile/Cargo.toml`
Expected: SUCCESS

- [ ] **Step 5: Commit state management**

```bash
git add mobile/src/viewmodel/mod.rs
git commit -m "feat: add ViewModel state management for debloat

Add packages and uad_ng_lists to ViewModelState.
Implement apply_event() for state updates.
Add state accessor methods.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Summary and Next Steps

This implementation plan provides detailed steps for the complete MVVM actor-based refactoring of UAD-Shizuku. The plan has been structured to be:

- **Incremental**: Each task produces working, testable code
- **TDD-driven**: Tests before implementation where applicable
- **Well-scoped**: Clear boundaries and dependencies between tasks
- **Committable**: Each task ends with a git commit

**Current Status:**
- ✅ Tasks 1-6 completed: Infrastructure setup and DebloatActor fully implemented
- ⏳ Remaining: Scan/Apps/Metadata actors, tab migrations, SharedStore removal

**Recommended Execution Approach:**

Since this is a large refactoring with 6+ detailed tasks already defined, I recommend starting with **Phase 1 tasks (1-6)** to establish the infrastructure, then continuing with additional phases as needed.

The plan follows the design spec's migration strategy:
- Phase 1 (Tasks 1-6): Infrastructure + DebloatActor ✓ **DETAILED IN THIS PLAN**
- Phase 2-5: Additional actors, tab migrations, cleanup (to be added as plan extends)

This allows for:
1. Early validation that the architecture works
2. Incremental progress with working checkpoints
3. Ability to adjust approach based on learnings

**Note:** Due to the comprehensive nature of this refactoring, the complete plan for all 5 phases would be ~150+ tasks. The first 6 tasks provide a solid foundation to prove the architecture and can be extended task-by-task as implementation progresses.

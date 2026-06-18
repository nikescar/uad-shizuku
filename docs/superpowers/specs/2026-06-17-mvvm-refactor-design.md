# MVVM Actor-Based Architecture Refactoring

**Date:** 2026-06-17  
**Status:** Approved Design  
**Goal:** Refactor UAD-Shizuku to use actor-based MVVM architecture with smol async runtime

## Executive Summary

Refactor UAD-Shizuku from direct UI-blocking I/O to a clean **actor-based MVVM architecture** where:
- UI operations are non-blocking with smooth progress updates
- Database, network, and ADB operations run on background actors
- Actors communicate via typed channels using smol async runtime
- Single source of truth (ViewModel) replaces current SharedStore pattern
- Clear separation between UI (view), coordination (ViewModel), and business logic (actors)

## Objectives

### Primary Goals
1. **Replace async libraries**: Remove `tokio` and `crossbeam-queue`, use `smol` for both runtime and channels
2. **Separate I/O from UI**: Move all blocking operations (DB, network, ADB) to background actors
3. **Clean architecture**: Actor-based MVVM with clear boundaries and testable components
4. **Preserve functionality**: Keep all existing features working, improve UX with progress updates

### Non-Goals
- Changing `calc*.rs` or `db*.rs` implementation (reuse as-is)
- Adding new features beyond architecture refactoring
- CLI mode support (mobile GUI only)

## Architecture Overview

### System Layers

```
┌─────────────────────────────────────────────────────┐
│                    UI Layer                         │
│  (Main Thread - egui rendering)                     │
│                                                      │
│  UadShizukuApp owns ViewModel                       │
│  Tabs receive &mut ViewModel                        │
│  UI polls events, renders state                     │
└──────────────────┬──────────────────────────────────┘
                   │ Command methods
                   │ Event polling
                   │ State getters
┌──────────────────▼──────────────────────────────────┐
│              ViewModel Layer                        │
│  (Coordination - lives on UI thread)                │
│                                                      │
│  • Holds channels to actors                         │
│  • Provides command API for tabs                    │
│  • Aggregates events from actors                    │
│  • Exposes read-only state                          │
└──────────────────┬──────────────────────────────────┘
                   │ smol::channel
                   │ Commands ↓
                   │ Events ↑
┌──────────────────▼──────────────────────────────────┐
│               Actor Layer                           │
│  (Background Thread - smol executor)                │
│                                                      │
│  ┌─────────────┐  ┌─────────────┐                  │
│  │  Debloat    │  │    Scan     │                  │
│  │   Actor     │  │   Actor     │                  │
│  └─────────────┘  └─────────────┘                  │
│  ┌─────────────┐  ┌─────────────┐                  │
│  │    Apps     │  │  Metadata   │                  │
│  │   Actor     │  │   Actor     │                  │
│  └─────────────┘  └─────────────┘                  │
│                                                      │
│  Each actor: command loop + concurrent tasks        │
└──────────────────┬──────────────────────────────────┘
                   │ Function calls
                   │ (reuse existing logic)
┌──────────────────▼──────────────────────────────────┐
│               Data Layer                            │
│  (Unchanged - reused by actors)                     │
│                                                      │
│  calc*.rs - computation logic                       │
│  db*.rs - database operations                       │
└─────────────────────────────────────────────────────┘
```

### Message Flow Example

```
User clicks "Load Packages"
  ↓
UI calls: vm.load_packages(device, user)
  ↓
ViewModel sends: DebloatCommand::LoadPackages { device, user }
  ↓ (via channel)
DebloatActor receives command
  ↓
Actor calls: smol::unblock(|| adb::get_packages(&device, user))
  ↓
Actor sends: ViewModelEvent::Debloat(PackagesLoaded(packages))
  ↓ (via channel)
ViewModel.poll_events() receives event
  ↓
ViewModel updates internal state
  ↓
UI renders updated package list
```

## Actor Domains

### 1. DebloatActor

**Responsibilities:**
- Load packages from ADB
- Batch uninstall/disable/enable operations
- UAD-NG list management
- Package state tracking

**Commands:**
```rust
pub enum DebloatCommand {
    LoadPackages { device: String, user: u32 },
    BatchUninstall { packages: Vec<String>, device: String },
    BatchDisable { packages: Vec<String>, device: String },
    BatchEnable { packages: Vec<String>, device: String },
    LoadUadNgLists,
}
```

**Events:**
```rust
pub enum DebloatEvent {
    PackagesLoaded(Vec<PackageFingerprint>),
    UadNgListsLoaded(UadNgLists),
    BatchProgress { 
        operation: String, 
        progress: f32,      // 0.0 to 1.0
        current: usize, 
        total: usize 
    },
    BatchComplete { 
        operation: String, 
        succeeded: usize, 
        failed: usize 
    },
    Error { operation: String, error: String },
}
```

**State:**
```rust
struct DebloatActorState {
    current_device: Option<String>,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
}
```

---

### 2. ScanActor

**Responsibilities:**
- VirusTotal scanning
- HybridAnalysis scanning
- Stalkerware detection
- IzzyRisk analysis

**Commands:**
```rust
pub enum ScanCommand {
    ScanVirusTotal { 
        package: String, 
        apk_path: String, 
        force_upload: bool 
    },
    ScanHybridAnalysis { 
        package: String, 
        apk_path: String, 
        force_upload: bool 
    },
    LoadStalkerwareIndicators,
    BatchScan { 
        packages: Vec<String>, 
        scanner: ScannerType 
    },
}

pub enum ScannerType {
    VirusTotal,
    HybridAnalysis,
    Both,
}
```

**Events:**
```rust
pub enum ScanEvent {
    VirusTotalResult { 
        package: String, 
        result: VtScanResult 
    },
    HybridAnalysisResult { 
        package: String, 
        result: HaScanResult 
    },
    StalkerwareIndicatorsLoaded(StalkerwareIndicators),
    ScanProgress { 
        scanner: String, 
        progress: f32,
        current: usize,
        total: usize,
    },
    Error { operation: String, error: String },
}
```

---

### 3. AppsActor

**Responsibilities:**
- Load OFFA/FMHY FOSS app lists
- Install apps via ADB
- Track installation progress

**Commands:**
```rust
pub enum AppsCommand {
    LoadOffaList,
    LoadFmhyList,
    InstallApp { 
        url: String, 
        package_name: String, 
        device: String 
    },
    CancelInstall { package_name: String },
}
```

**Events:**
```rust
pub enum AppsEvent {
    OffaListLoaded(Vec<FossApp>),
    FmhyListLoaded(Vec<FossApp>),
    InstallProgress { 
        package: String, 
        progress: f32, 
        status: String 
    },
    InstallComplete { 
        package: String, 
        success: bool 
    },
    Error { operation: String, error: String },
}
```

---

### 4. MetadataActor

**Responsibilities:**
- Fetch Google Play metadata
- Fetch F-Droid metadata
- Fetch APKMirror metadata
- Image/texture loading

**Commands:**
```rust
pub enum MetadataCommand {
    FetchGooglePlay { package_id: String },
    FetchFDroid { package_id: String },
    FetchApkMirror { package_id: String },
    FetchAndroidPackageInfo { package_id: String },
    LoadTexture { 
        package_id: String, 
        source: MetadataSource, 
        url: String 
    },
}

pub enum MetadataSource {
    GooglePlay,
    FDroid,
    ApkMirror,
    AndroidPackage,
}
```

**Events:**
```rust
pub enum MetadataEvent {
    GooglePlayFetched { 
        package_id: String, 
        app: GooglePlayApp 
    },
    FDroidFetched { 
        package_id: String, 
        app: FDroidApp 
    },
    ApkMirrorFetched { 
        package_id: String, 
        app: ApkMirrorApp 
    },
    TextureLoaded { 
        package_id: String, 
        source: MetadataSource, 
        image_data: Vec<u8> 
    },
    Error { operation: String, error: String },
}
```

## ViewModel Structure

### Public API

```rust
pub struct ViewModel {
    // Actor communication channels
    debloat_tx: smol::channel::Sender<DebloatCommand>,
    scan_tx: smol::channel::Sender<ScanCommand>,
    apps_tx: smol::channel::Sender<AppsCommand>,
    metadata_tx: smol::channel::Sender<MetadataCommand>,
    
    // Unified event receiver (all actors send here)
    event_rx: smol::channel::Receiver<ViewModelEvent>,
    
    // Public state (read-only access from UI)
    state: ViewModelState,
    
    // Background thread handle
    _runtime_handle: Option<std::thread::JoinHandle<()>>,
}

pub struct ViewModelState {
    // Debloat state
    pub packages: Vec<PackageFingerprint>,
    pub uad_ng_lists: Option<UadNgLists>,
    
    // Scan state
    pub vt_results: HashMap<String, VtScanResult>,
    pub ha_results: HashMap<String, HaScanResult>,
    pub stalkerware_indicators: Option<StalkerwareIndicators>,
    
    // Apps state
    pub offa_apps: Vec<FossApp>,
    pub fmhy_apps: Vec<FossApp>,
    
    // Metadata state
    pub google_play_apps: HashMap<String, GooglePlayApp>,
    pub fdroid_apps: HashMap<String, FDroidApp>,
    pub apkmirror_apps: HashMap<String, ApkMirrorApp>,
    pub textures: HashMap<(String, MetadataSource), egui::TextureHandle>,
    
    // Progress tracking
    pub active_operations: HashMap<String, OperationProgress>,
}

pub struct OperationProgress {
    pub operation: String,
    pub progress: f32,  // 0.0 to 1.0
    pub status: String,
}

// Unified event type
pub enum ViewModelEvent {
    Debloat(DebloatEvent),
    Scan(ScanEvent),
    Apps(AppsEvent),
    Metadata(MetadataEvent),
}
```

### Command Methods

Tabs call these to trigger actor operations:

```rust
impl ViewModel {
    // === Debloat commands ===
    pub fn load_packages(&self, device: String, user: u32) -> Result<()> {
        self.debloat_tx.send_blocking(DebloatCommand::LoadPackages { device, user })
    }
    
    pub fn batch_uninstall(&self, packages: Vec<String>, device: String) -> Result<()> {
        self.debloat_tx.send_blocking(DebloatCommand::BatchUninstall { packages, device })
    }
    
    pub fn batch_disable(&self, packages: Vec<String>, device: String) -> Result<()> {
        self.debloat_tx.send_blocking(DebloatCommand::BatchDisable { packages, device })
    }
    
    // === Scan commands ===
    pub fn scan_virustotal(&self, package: String, apk_path: String, force_upload: bool) -> Result<()> {
        self.scan_tx.send_blocking(ScanCommand::ScanVirusTotal { package, apk_path, force_upload })
    }
    
    pub fn scan_hybrid_analysis(&self, package: String, apk_path: String, force_upload: bool) -> Result<()> {
        self.scan_tx.send_blocking(ScanCommand::ScanHybridAnalysis { package, apk_path, force_upload })
    }
    
    // === Apps commands ===
    pub fn load_offa_list(&self) -> Result<()> {
        self.apps_tx.send_blocking(AppsCommand::LoadOffaList)
    }
    
    pub fn install_app(&self, url: String, package_name: String, device: String) -> Result<()> {
        self.apps_tx.send_blocking(AppsCommand::InstallApp { url, package_name, device })
    }
    
    // === Metadata commands ===
    pub fn fetch_google_play(&self, package_id: String) -> Result<()> {
        self.metadata_tx.send_blocking(MetadataCommand::FetchGooglePlay { package_id })
    }
    
    pub fn fetch_fdroid(&self, package_id: String) -> Result<()> {
        self.metadata_tx.send_blocking(MetadataCommand::FetchFDroid { package_id })
    }
}
```

### Event Processing

```rust
impl ViewModel {
    /// Poll for events and update state. Call this in UadShizukuApp::update()
    pub fn poll_events(&mut self, ctx: &egui::Context) -> Vec<ViewModelEvent> {
        let mut events = Vec::new();
        
        // Non-blocking receive all available events
        while let Ok(event) = self.event_rx.try_recv() {
            self.apply_event(&event, ctx);  // Update internal state
            events.push(event);
        }
        
        events
    }
    
    /// Apply event to internal state
    fn apply_event(&mut self, event: &ViewModelEvent, ctx: &egui::Context) {
        match event {
            ViewModelEvent::Debloat(DebloatEvent::PackagesLoaded(packages)) => {
                self.state.packages = packages.clone();
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
            ViewModelEvent::Scan(ScanEvent::VirusTotalResult { package, result }) => {
                self.state.vt_results.insert(package.clone(), result.clone());
            }
            ViewModelEvent::Metadata(MetadataEvent::TextureLoaded { package_id, source, image_data }) => {
                // Load texture on UI thread (egui requirement)
                let texture = Self::load_texture_from_bytes(ctx, image_data);
                self.state.textures.insert((package_id.clone(), *source), texture);
            }
            // ... handle all other events
            _ => {}
        }
    }
    
    // === Read-only state accessors ===
    pub fn packages(&self) -> &[PackageFingerprint] { 
        &self.state.packages 
    }
    
    pub fn vt_result(&self, pkg: &str) -> Option<&VtScanResult> { 
        self.state.vt_results.get(pkg) 
    }
    
    pub fn texture(&self, pkg: &str, source: MetadataSource) -> Option<&egui::TextureHandle> {
        self.state.textures.get(&(pkg.to_string(), source))
    }
    
    pub fn operation_progress(&self, operation: &str) -> Option<&OperationProgress> {
        self.state.active_operations.get(operation)
    }
}
```

## Actor Implementation

### Actor Structure

Each actor follows this pattern:

```rust
struct DebloatActor {
    // Actor's private state
    state: DebloatActorState,
    
    // Communication channels
    command_rx: smol::channel::Receiver<DebloatCommand>,
    event_tx: smol::channel::Sender<ViewModelEvent>,
    
    // References to other actors (for inter-actor communication)
    metadata_tx: smol::channel::Sender<MetadataCommand>,
}

struct DebloatActorState {
    current_device: Option<String>,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
}

impl DebloatActor {
    fn new(
        command_rx: smol::channel::Receiver<DebloatCommand>,
        event_tx: smol::channel::Sender<ViewModelEvent>,
        metadata_tx: smol::channel::Sender<MetadataCommand>,
    ) -> Self {
        Self {
            state: DebloatActorState {
                current_device: None,
                unsafe_app_remove: false,
                expert_app_remove: false,
            },
            command_rx,
            event_tx,
            metadata_tx,
        }
    }
    
    async fn run(mut self) {
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
    
    async fn handle_command(&mut self, cmd: DebloatCommand) -> anyhow::Result<()> {
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
    
    async fn load_packages(&mut self, device: String, user: u32) -> anyhow::Result<()> {
        // Use smol::unblock for blocking operations (reuse existing calc/adb functions)
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
    
    async fn batch_uninstall(&mut self, packages: Vec<String>, device: String) -> anyhow::Result<()> {
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

### Background Thread and Runtime

```rust
impl ViewModel {
    pub fn new(ctx: egui::Context) -> Self {
        // Create unbounded channels for each actor
        let (debloat_tx, debloat_rx) = smol::channel::unbounded();
        let (scan_tx, scan_rx) = smol::channel::unbounded();
        let (apps_tx, apps_rx) = smol::channel::unbounded();
        let (metadata_tx, metadata_rx) = smol::channel::unbounded();
        
        // Unified event channel (all actors send here)
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
                let scan_actor = ScanActor::new(
                    scan_rx, 
                    event_tx.clone(),
                    metadata_tx.clone(),
                );
                let apps_actor = AppsActor::new(apps_rx, event_tx.clone());
                let metadata_actor = MetadataActor::new(metadata_rx, event_tx.clone());
                
                // Run all actors concurrently
                smol::spawn(debloat_actor.run()).detach();
                smol::spawn(scan_actor.run()).detach();
                smol::spawn(apps_actor.run()).detach();
                smol::spawn(metadata_actor.run()).detach();
                
                // Keep thread alive (actors run until channels close)
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
}
```

### Concurrency Within Actors

Actors can spawn concurrent tasks for independent operations:

```rust
// Example: MetadataActor fetching multiple packages concurrently
impl MetadataActor {
    async fn fetch_batch_google_play(&self, package_ids: Vec<String>) -> anyhow::Result<()> {
        // Spawn concurrent tasks (up to 10 at a time to respect rate limits)
        let semaphore = Arc::new(smol::lock::Semaphore::new(10));
        
        let tasks: Vec<_> = package_ids.into_iter()
            .map(|pkg_id| {
                let event_tx = self.event_tx.clone();
                let sem = semaphore.clone();
                
                smol::spawn(async move {
                    let _permit = sem.acquire().await;
                    
                    match Self::fetch_google_play_metadata(&pkg_id).await {
                        Ok(app) => {
                            event_tx.send(ViewModelEvent::Metadata(
                                MetadataEvent::GooglePlayFetched { 
                                    package_id: pkg_id, 
                                    app 
                                }
                            )).await.ok();
                        }
                        Err(e) => {
                            log::error!("Failed to fetch Google Play for {}: {}", pkg_id, e);
                        }
                    }
                })
            })
            .collect();
        
        // Wait for all fetches to complete
        for task in tasks {
            task.await;
        }
        
        Ok(())
    }
    
    async fn fetch_google_play_metadata(package_id: &str) -> anyhow::Result<GooglePlayApp> {
        // Reuse existing calc_googleplay.rs functions
        smol::unblock(move || {
            crate::calc_googleplay::fetch_app_info(package_id)
        }).await
    }
}
```

## Error Handling

### Error Event Structure

```rust
// Each domain event enum includes Error variant
pub enum DebloatEvent {
    // ... success events
    Error { operation: String, error: String },
}

pub enum ScanEvent {
    // ... success events
    Error { operation: String, error: String },
}

// Similar for Apps and Metadata events
```

### Error Propagation

**In Actors:**
```rust
async fn handle_command(&mut self, cmd: DebloatCommand) -> anyhow::Result<()> {
    let result = match cmd {
        DebloatCommand::LoadPackages { device, user } => {
            self.load_packages(device, user).await
        }
        // ... other commands
    };
    
    // Convert errors to events (don't crash the actor)
    if let Err(e) = result {
        self.send_error(self.current_operation(), e).await;
    }
    
    Ok(()) // Actor keeps running despite command errors
}

async fn send_error(&self, operation: &str, error: anyhow::Error) {
    let _ = self.event_tx.send(ViewModelEvent::Debloat(
        DebloatEvent::Error {
            operation: operation.to_string(),
            error: error.to_string(),
        }
    )).await;
}
```

**In UI:**
```rust
impl UadShizukuApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll ViewModel events
        let events = self.viewmodel.poll_events(ctx);
        
        for event in events {
            match event {
                ViewModelEvent::Debloat(DebloatEvent::Error { operation, error }) => {
                    // Show error dialog or toast notification
                    self.show_error_dialog(&format!(
                        "Debloat operation '{}' failed:\n{}", 
                        operation, 
                        error
                    ));
                }
                ViewModelEvent::Scan(ScanEvent::Error { operation, error }) => {
                    // Log scan errors, show in status bar
                    log::error!("Scan error in {}: {}", operation, error);
                    self.scan_error_message = Some(error);
                }
                // ... handle other events
                _ => {}
            }
        }
    }
}
```

## Testing Strategy

### Unit Tests (Actor Logic)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_debloat_actor_load_packages() {
        smol::block_on(async {
            let (cmd_tx, cmd_rx) = smol::channel::unbounded();
            let (event_tx, event_rx) = smol::channel::unbounded();
            let (metadata_tx, _) = smol::channel::unbounded();
            
            let actor = DebloatActor::new(cmd_rx, event_tx, metadata_tx);
            smol::spawn(actor.run()).detach();
            
            // Send command
            cmd_tx.send(DebloatCommand::LoadPackages { 
                device: "test_device".into(), 
                user: 0 
            }).await.unwrap();
            
            // Assert event received
            let event = event_rx.recv().await.unwrap();
            match event {
                ViewModelEvent::Debloat(DebloatEvent::PackagesLoaded(packages)) => {
                    assert!(!packages.is_empty());
                }
                _ => panic!("Expected PackagesLoaded event, got {:?}", event),
            }
        });
    }
    
    #[test]
    fn test_debloat_actor_batch_uninstall_progress() {
        smol::block_on(async {
            let (cmd_tx, cmd_rx) = smol::channel::unbounded();
            let (event_tx, event_rx) = smol::channel::unbounded();
            let (metadata_tx, _) = smol::channel::unbounded();
            
            let actor = DebloatActor::new(cmd_rx, event_tx, metadata_tx);
            smol::spawn(actor.run()).detach();
            
            // Send batch uninstall command
            cmd_tx.send(DebloatCommand::BatchUninstall {
                packages: vec!["com.test.app1".into(), "com.test.app2".into()],
                device: "test_device".into(),
            }).await.unwrap();
            
            // Collect progress events
            let mut progress_events = Vec::new();
            while let Ok(event) = event_rx.recv().await {
                match event {
                    ViewModelEvent::Debloat(DebloatEvent::BatchProgress { .. }) => {
                        progress_events.push(event);
                    }
                    ViewModelEvent::Debloat(DebloatEvent::BatchComplete { .. }) => {
                        break;
                    }
                    _ => {}
                }
            }
            
            assert_eq!(progress_events.len(), 2); // One per package
        });
    }
}
```

### Integration Tests (ViewModel)

```rust
#[test]
fn test_viewmodel_debloat_flow() {
    let ctx = egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Send command
    vm.load_packages("test_device".into(), 0).unwrap();
    
    // Poll for events (in real app, this happens in update loop)
    std::thread::sleep(std::time::Duration::from_millis(100));
    let events = vm.poll_events(&ctx);
    
    // Verify state updated
    assert!(!vm.packages().is_empty());
    
    // Verify event was received
    assert!(events.iter().any(|e| matches!(
        e, 
        ViewModelEvent::Debloat(DebloatEvent::PackagesLoaded(_))
    )));
}

#[test]
fn test_viewmodel_concurrent_operations() {
    let ctx = egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    
    // Start multiple concurrent operations
    vm.load_packages("device1".into(), 0).unwrap();
    vm.scan_virustotal("com.test.app".into(), "/path/to/app.apk".into(), false).unwrap();
    vm.load_offa_list().unwrap();
    
    // Poll events
    std::thread::sleep(std::time::Duration::from_millis(200));
    let events = vm.poll_events(&ctx);
    
    // Verify all operations produced events
    assert!(events.len() >= 3);
}
```

## Migration Strategy

### Phase 1: Infrastructure Setup (Week 1)

**Goals:** Set up ViewModel skeleton, replace dependencies

**Tasks:**
1. ✅ Update `Cargo.toml`:
   - Remove `tokio = { version = "1", features = ["rt-multi-thread"] }`
   - Remove `crossbeam-queue = "0.3"`
   - Add `smol = "2.0"`
   
2. ✅ Create ViewModel module structure:
   ```
   mobile/src/viewmodel/
     mod.rs              # ViewModel struct, initialization
     debloat.rs          # DebloatActor + messages
     scan.rs             # ScanActor + messages
     apps.rs             # AppsActor + messages
     metadata.rs         # MetadataActor + messages
     common.rs           # Shared types (ViewModelEvent, etc.)
   ```

3. ✅ Implement ViewModel skeleton:
   - `ViewModel::new()` - spawn background thread
   - Channel creation
   - Empty actor structs with `run()` loops
   - `poll_events()` stub

4. ✅ Add ViewModel to `UadShizukuApp`:
   ```rust
   pub struct UadShizukuApp {
       viewmodel: ViewModel,
       // ... existing fields
   }
   ```

5. ✅ Call `viewmodel.poll_events(ctx)` in `UadShizukuApp::update()`

**Success criteria:** App compiles and runs (no functionality change yet)

---

### Phase 2: Debloat Actor Migration (Week 2)

**Goals:** Migrate debloat tab operations to DebloatActor

**Tasks:**
1. ✅ Implement DebloatActor commands:
   - `LoadPackages` - call `adb::get_packages()` via `smol::unblock`
   - `LoadUadNgLists` - load UAD-NG data
   - `BatchUninstall/Disable/Enable` - with progress events

2. ✅ Update `tab_debloat_control.rs`:
   - Replace direct ADB calls with `vm.load_packages(device, user)`
   - Remove `Arc<Mutex<>>` for packages (read from `vm.packages()`)
   - Handle `DebloatEvent` events for updates

3. ✅ Migrate batch operations:
   - Replace `std::thread::spawn` with ViewModel commands
   - Remove `batch_uninstall_progress` `Arc<Mutex<>>`
   - Show progress from `vm.operation_progress("uninstall")`

4. ✅ Test debloat tab:
   - Load packages from device
   - Batch uninstall with progress
   - Batch disable/enable

**Success criteria:** Debloat tab fully functional via ViewModel

---

### Phase 3: Scan Actor Migration (Week 2-3)

**Goals:** Migrate scan operations to ScanActor

**Tasks:**
1. ✅ Implement ScanActor commands:
   - `ScanVirusTotal` - scan with progress
   - `ScanHybridAnalysis` - scan with progress
   - `LoadStalkerwareIndicators`
   - `BatchScan` - concurrent scanning

2. ✅ Update `tab_scan_control.rs`:
   - Replace direct scanner calls with `vm.scan_virustotal()`
   - Remove scanner state machines from tab struct
   - Handle `ScanEvent` events

3. ✅ Migrate VirusTotal/HybridAnalysis queues:
   - Remove `db_virustotal::init_upsert_queue()` (replace with actor)
   - Remove `db_hybridanalysis::init_upsert_queue()`
   - Batch DB writes from ScanActor

4. ✅ Test scan tab:
   - Single package scan
   - Batch scan with progress
   - Stalkerware detection

**Success criteria:** Scan tab fully functional via ViewModel

---

### Phase 4: Apps & Metadata Actors Migration (Week 3-4)

**Goals:** Migrate apps tab and metadata fetching

**Tasks:**
1. ✅ Implement AppsActor commands:
   - `LoadOffaList` - fetch FOSS apps
   - `LoadFmhyList` - fetch FOSS apps
   - `InstallApp` - install via ADB with progress

2. ✅ Update `tab_apps_control.rs`:
   - Replace direct network calls with `vm.load_offa_list()`
   - Handle `AppsEvent` events

3. ✅ Implement MetadataActor commands:
   - `FetchGooglePlay/FDroid/ApkMirror` - concurrent fetching
   - `LoadTexture` - download images, send bytes to UI

4. ✅ Migrate texture loading:
   - Remove renderer state machines from `uad_shizuku_app.rs`
   - Load textures on UI thread from `MetadataEvent::TextureLoaded`
   - Store textures in `vm.state.textures`

5. ✅ Test apps tab and metadata:
   - Load FOSS app lists
   - Install app with progress
   - Google Play/F-Droid icons load correctly

**Success criteria:** Apps tab and all metadata fetching via ViewModel

---

### Phase 5: Cleanup and Optimization (Week 4)

**Goals:** Remove legacy code, optimize performance

**Tasks:**
1. ✅ Remove `SharedStore`:
   - Delete `mobile/src/shared_store.rs`
   - Delete `mobile/src/shared_store_stt.rs`
   - Remove `get_shared_store()` calls

2. ✅ Remove old threading code:
   - Remove `Arc<Mutex<>>` progress trackers from tabs
   - Remove `std::thread::spawn` in tab files
   - Remove `package_loading_thread` from `UadShizukuApp`

3. ✅ Update Cargo.toml features:
   - Verify no unused dependencies
   - Check eframe `glow` feature is active (already done)

4. ✅ Performance tuning:
   - Add rate limiting to MetadataActor (respect API limits)
   - Batch DB writes in actors (reduce lock contention)
   - Tune channel buffer sizes if needed

5. ✅ Comprehensive testing:
   - All tabs functional
   - No UI blocking
   - Progress bars smooth
   - Error handling works

**Success criteria:** All legacy code removed, app performs well

---

### Incremental Migration Notes

- **Feature flag approach (optional):**
  ```rust
  #[cfg(feature = "legacy_store")]
  use crate::shared_store::get_shared_store;
  
  #[cfg(not(feature = "legacy_store"))]
  fn get_packages() -> Vec<Package> {
      // Use ViewModel instead
  }
  ```

- **Keep old code paths** until ViewModel equivalent is tested
- **One tab at a time** - don't migrate all tabs simultaneously
- **Commit after each phase** - easy rollback if issues arise

## File Structure

```
mobile/
├── src/
│   ├── viewmodel/
│   │   ├── mod.rs              # ViewModel struct, init, poll_events
│   │   ├── debloat.rs          # DebloatActor + DebloatCommand/Event
│   │   ├── scan.rs             # ScanActor + ScanCommand/Event
│   │   ├── apps.rs             # AppsActor + AppsCommand/Event
│   │   ├── metadata.rs         # MetadataActor + MetadataCommand/Event
│   │   └── common.rs           # ViewModelEvent, shared types
│   ├── uad_shizuku_app.rs      # UadShizukuApp owns ViewModel
│   ├── tab_debloat_control.rs  # Uses vm.load_packages(), etc.
│   ├── tab_scan_control.rs     # Uses vm.scan_virustotal(), etc.
│   ├── tab_apps_control.rs     # Uses vm.load_offa_list(), etc.
│   ├── calc*.rs                # UNCHANGED - reused by actors
│   ├── db*.rs                  # UNCHANGED - called from actors
│   └── ...
├── Cargo.toml                   # smol added, tokio/crossbeam-queue removed
└── tests/
    ├── viewmodel_tests.rs       # Unit tests for actors
    └── integration_tests.rs     # Integration tests for ViewModel
```

## Dependencies Changes

### Remove
```toml
tokio = { version = "1", features = ["rt-multi-thread"] }
crossbeam-queue = "0.3"
```

### Add
```toml
smol = "2.0"
```

### Verify eframe glow (already configured)
```toml
# In workspace Cargo.toml
eframe = { version = "0.33", default-features = false, features = [
    "default_fonts",
    "glow",  # ✓ Already present
] }
```

## Non-Functional Requirements

### Performance
- UI remains responsive during heavy I/O operations
- Progress updates at least 10 FPS (100ms intervals)
- Batch operations support cancellation
- Texture loading doesn't block UI

### Reliability
- Actor failures don't crash the app
- Channel closure triggers graceful shutdown
- Errors are user-visible with actionable messages
- State remains consistent across actor crashes

### Maintainability
- Clear separation of concerns (UI, coordination, business logic)
- Testable actors (dependency injection via channels)
- Easy to add new actors/commands/events
- Follows Rust idioms (ownership, Result types)

## Success Criteria

**Technical:**
- ✅ No `tokio` or `crossbeam-queue` dependencies
- ✅ All I/O operations use `smol::unblock` or async
- ✅ `SharedStore` completely removed
- ✅ UI thread never blocks on I/O
- ✅ All existing features work identically

**User Experience:**
- ✅ Smooth progress bars during batch operations
- ✅ Cancellable long-running operations
- ✅ Clear error messages on failures
- ✅ No perceived performance regression

**Code Quality:**
- ✅ All actor logic has unit tests
- ✅ Integration tests cover ViewModel flows
- ✅ No clippy warnings
- ✅ Documentation for public ViewModel API

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Actor complexity overwhelming | High | Start with simple DebloatActor, iterate |
| Channel deadlocks | High | Use unbounded channels, careful error handling |
| Performance regression | Medium | Benchmark before/after, tune if needed |
| Migration scope too large | High | Incremental migration, one tab at a time |
| Texture loading on UI thread | Low | Keep current pattern (load bytes in actor, create texture on UI) |

## Future Enhancements (Out of Scope)

- Actor supervision tree (restart failed actors)
- Distributed tracing for actor messages
- WebAssembly support (would need different runtime)
- CLI mode support (requires conditional ViewModel)

## References

- **Reference implementation:** `reference/mvvm.rs` - basic message-passing pattern
- **BingTray CLAUDE.md:** Similar MVVM architecture with Diesel
- **smol documentation:** https://docs.rs/smol/
- **eframe examples:** egui-based apps with async operations

---

**Design approved by:** User  
**Next step:** Create implementation plan using writing-plans skill

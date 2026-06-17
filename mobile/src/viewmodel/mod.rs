//! ViewModel layer - coordinates between UI and background actors

pub mod common;
pub mod debloat;
pub mod scan;
pub mod apps;
pub mod metadata;

pub use common::*;
pub use debloat::{DebloatCommand, DebloatEvent, DebloatActor};
pub use scan::{ScanCommand, ScanEvent};
pub use apps::{AppsCommand, AppsEvent};
pub use metadata::{MetadataCommand, MetadataEvent};

use std::collections::HashMap;

/// ViewModel struct - owned by UadShizukuApp, coordinates actor communication
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

/// ViewModel state - read-only access from UI
#[derive(Default)]
pub struct ViewModelState {
    // Progress tracking
    pub active_operations: HashMap<String, OperationProgress>,
}

impl ViewModel {
    /// Create new ViewModel and spawn background runtime
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

    /// Poll for events and update state. Call this in UadShizukuApp::update()
    pub fn poll_events(&mut self, _ctx: &eframe::egui::Context) -> Vec<ViewModelEvent> {
        let mut events = Vec::new();

        // Non-blocking receive all available events
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }

        events
    }

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
}

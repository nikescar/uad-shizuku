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

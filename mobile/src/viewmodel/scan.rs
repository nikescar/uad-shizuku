//! Scan actor - handles virus scanning operations

use crate::viewmodel::ViewModelEvent;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum ScanCommand {
    RunVirusTotal {
        device: String,
        api_key: String,
        submit_enabled: bool,
    },
    RunHybridAnalysis {
        device: String,
        api_key: String,
        submit_enabled: bool,
    },
    CancelVirusTotal,
    CancelHybridAnalysis,
}

#[derive(Debug, Clone)]
pub enum ScannerType {
    VirusTotal,
    HybridAnalysis,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    VirusTotalStarted,
    HybridAnalysisStarted,
    ScanProgress { scanner: ScannerType, progress: f32 },
    VirusTotalComplete,
    HybridAnalysisComplete,
    VirusTotalCancelled,
    HybridAnalysisCancelled,
    Error { operation: String, error: String },

    // === NEW: Scanner state updates ===
    VirusTotalStateUpdated(crate::calc_virustotal_stt::ScannerState),
    HybridAnalysisStateUpdated(crate::calc_hybridanalysis_stt::ScannerState),
}

pub struct ScanActor {
    command_rx: smol::channel::Receiver<ScanCommand>,
    event_tx: smol::channel::Sender<ViewModelEvent>,
    vt_cancel: std::sync::Arc<std::sync::Mutex<bool>>,
    ha_cancel: std::sync::Arc<std::sync::Mutex<bool>>,
}

impl ScanActor {
    pub fn new(
        command_rx: smol::channel::Receiver<ScanCommand>,
        event_tx: smol::channel::Sender<ViewModelEvent>,
    ) -> Self {
        Self {
            command_rx,
            event_tx,
            vt_cancel: std::sync::Arc::new(std::sync::Mutex::new(false)),
            ha_cancel: std::sync::Arc::new(std::sync::Mutex::new(false)),
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.command_rx.recv().await {
                Ok(cmd) => {
                    if let Err(e) = self.handle_command(cmd).await {
                        self.send_error("scan", e).await;
                    }
                }
                Err(_) => {
                    log::info!("ScanActor: shutting down");
                    break;
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: ScanCommand) -> Result<()> {
        match cmd {
            ScanCommand::RunVirusTotal {
                device,
                api_key,
                submit_enabled,
            } => {
                // Reset cancellation flag
                if let Ok(mut cancel) = self.vt_cancel.lock() {
                    *cancel = false;
                }

                self.event_tx
                    .send(ViewModelEvent::Scan(ScanEvent::VirusTotalStarted))
                    .await?;

                // Send initial empty state to indicate scan in progress
                use std::sync::{Arc, Mutex};
                let initial_state = Arc::new(Mutex::new(HashMap::new()));
                // Mark with a placeholder to indicate scan is running (not cancelled/empty)
                {
                    let mut state = initial_state.lock().unwrap();
                    state.insert(
                        "__scanning__".to_string(),
                        crate::calc_virustotal_stt::ScanStatus::Scanning {
                            scanned: 0,
                            total: 0,
                            operation: "Initializing...".to_string(),
                        },
                    );
                }
                self.event_tx
                    .send(ViewModelEvent::Scan(ScanEvent::VirusTotalStateUpdated(
                        initial_state,
                    )))
                    .await?;

                let device_clone = device.clone();
                let api_key_clone = api_key.clone();
                let cancel_clone = self.vt_cancel.clone();
                let event_tx = self.event_tx.clone();

                // Spawn task to run VirusTotal scan in background
                smol::spawn(async move {
                    let store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = store.get_installed_packages();
                    let package_risk_scores: HashMap<String, i32> = HashMap::new();
                    let progress = std::sync::Arc::new(std::sync::Mutex::new(Some(0.0)));
                    let progress_clone = progress.clone();

                    // Run scan using existing calc_virustotal function
                    let (scanner_state, _rate_limiter) = smol::unblock(move || {
                        crate::calc_virustotal::run_virustotal(
                            installed_packages,
                            device_clone,
                            api_key_clone,
                            submit_enabled,
                            package_risk_scores,
                            progress,
                            cancel_clone,
                        )
                    })
                    .await;

                    // Send scanner state update event to ViewModel
                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::VirusTotalStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

                    // Send completion event
                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::VirusTotalComplete))
                        .await;
                })
                .detach();

                Ok(())
            }
            ScanCommand::RunHybridAnalysis {
                device,
                api_key,
                submit_enabled,
            } => {
                // Reset cancellation flag
                if let Ok(mut cancel) = self.ha_cancel.lock() {
                    *cancel = false;
                }

                self.event_tx
                    .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisStarted))
                    .await?;

                // Send initial empty state to indicate scan in progress
                use std::sync::{Arc, Mutex};
                let initial_state = Arc::new(Mutex::new(HashMap::new()));
                // Mark with a placeholder to indicate scan is running (not cancelled/empty)
                {
                    let mut state = initial_state.lock().unwrap();
                    state.insert(
                        "__scanning__".to_string(),
                        crate::calc_hybridanalysis_stt::ScanStatus::Scanning {
                            scanned: 0,
                            total: 0,
                            operation: "Initializing...".to_string(),
                        },
                    );
                }
                self.event_tx
                    .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisStateUpdated(
                        initial_state,
                    )))
                    .await?;

                let device_clone = device.clone();
                let api_key_clone = api_key.clone();
                let cancel_clone = self.ha_cancel.clone();
                let event_tx = self.event_tx.clone();

                // Spawn task to run HybridAnalysis scan in background
                smol::spawn(async move {
                    let store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = store.get_installed_packages();
                    let package_risk_scores: HashMap<String, i32> = HashMap::new();
                    let progress = std::sync::Arc::new(std::sync::Mutex::new(Some(0.0)));
                    let progress_clone = progress.clone();

                    // Run scan using existing calc_hybridanalysis function
                    let (scanner_state, _rate_limiter) = smol::unblock(move || {
                        crate::calc_hybridanalysis::run_hybridanalysis(
                            installed_packages,
                            device_clone,
                            api_key_clone,
                            submit_enabled,
                            package_risk_scores,
                            progress,
                            cancel_clone,
                        )
                    })
                    .await;

                    // Send scanner state update event to ViewModel
                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

                    // Send completion event
                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisComplete))
                        .await;
                })
                .detach();

                Ok(())
            }
            ScanCommand::CancelVirusTotal => {
                if let Ok(mut cancel) = self.vt_cancel.lock() {
                    *cancel = true;
                }

                // Send cancellation event (state will be cleared by ViewModel)
                self.event_tx
                    .send(ViewModelEvent::Scan(ScanEvent::VirusTotalCancelled))
                    .await?;
                Ok(())
            }
            ScanCommand::CancelHybridAnalysis => {
                if let Ok(mut cancel) = self.ha_cancel.lock() {
                    *cancel = true;
                }

                // Send cancellation event (state will be cleared by ViewModel)
                self.event_tx
                    .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisCancelled))
                    .await?;
                Ok(())
            }
        }
    }

    async fn send_error(&self, operation: &str, error: anyhow::Error) {
        let _ = self
            .event_tx
            .send(ViewModelEvent::Scan(ScanEvent::Error {
                operation: operation.to_string(),
                error: error.to_string(),
            }))
            .await;
    }
}

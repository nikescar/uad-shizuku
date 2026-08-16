//! Scan actor - handles virus scanning operations

use crate::viewmodel::ViewModelEvent;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How often to sample the shared progress value and forward it as a ScanEvent::ScanProgress.
const SCAN_PROGRESS_POLL_INTERVAL: Duration = Duration::from_millis(300);

/// Periodically samples `progress` and forwards it as `ScanEvent::ScanProgress` until `done`
/// is set. Runs concurrently with the actual scan so the UI gets live progress updates instead
/// of only a start/complete pair.
async fn poll_scan_progress(
    scanner: ScannerType,
    progress: Arc<Mutex<Option<f32>>>,
    done: Arc<AtomicBool>,
    event_tx: smol::channel::Sender<ViewModelEvent>,
) {
    loop {
        if done.load(Ordering::Relaxed) {
            break;
        }
        let current = progress.lock().ok().and_then(|guard| *guard);
        if let Some(p) = current {
            let _ = event_tx
                .send(ViewModelEvent::Scan(ScanEvent::ScanProgress {
                    scanner: scanner.clone(),
                    progress: p,
                }))
                .await;
        }
        smol::Timer::after(SCAN_PROGRESS_POLL_INTERVAL).await;
    }
}

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
    /// Same as `RunVirusTotal` but scoped to a single package, e.g. to refresh
    /// results right after that package was uninstalled/changed.
    RunVirusTotalForPackage {
        device: String,
        api_key: String,
        submit_enabled: bool,
        package: crate::adb_stt::PackageFingerprint,
    },
    /// Same as `RunHybridAnalysis` but scoped to a single package.
    RunHybridAnalysisForPackage {
        device: String,
        api_key: String,
        submit_enabled: bool,
        package: crate::adb_stt::PackageFingerprint,
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

                let device_clone = device.clone();
                let api_key_clone = api_key.clone();
                let cancel_clone = self.vt_cancel.clone();
                let event_tx = self.event_tx.clone();

                // Spawn task to run VirusTotal scan in background
                smol::spawn(async move {
                    let store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = store.get_installed_packages();
                    let package_risk_scores: HashMap<String, i32> = HashMap::new();
                    let progress = Arc::new(Mutex::new(Some(0.0)));
                    let progress_for_scan = progress.clone();
                    let progress_done = Arc::new(AtomicBool::new(false));

                    // run_virustotal() spawns its own background std::thread and returns
                    // scanner_state (an Arc<Mutex<HashMap<..>>>) immediately, well before the
                    // scan itself finishes: the thread mutates that same HashMap in place as
                    // each package completes. Hand this Arc to the UI right away, rather than
                    // waiting for the whole (possibly many-minutes-long, VirusTotal free tier
                    // is 4 req/min) scan to finish first, so results appear per-package as the
                    // scan runs instead of only once at the very end.
                    let (scanner_state, _rate_limiter, handle) = crate::calc_virustotal::run_virustotal(
                        installed_packages,
                        device_clone,
                        api_key_clone,
                        submit_enabled,
                        package_risk_scores,
                        progress_for_scan,
                        cancel_clone,
                    );

                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::VirusTotalStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

                    // Forward live progress to the UI while the scan runs.
                    let poll_task = smol::spawn(poll_scan_progress(
                        ScannerType::VirusTotal,
                        progress,
                        progress_done.clone(),
                        event_tx.clone(),
                    ));

                    // Wait for the background thread to actually finish before declaring
                    // completion (join happens on the blocking thread pool used by
                    // smol::unblock, not on the async executor).
                    smol::unblock(move || {
                        let _ = handle.join();
                    })
                    .await;

                    progress_done.store(true, Ordering::Relaxed);
                    poll_task.await;

                    // Send a final scanner state update (same Arc, but this also flips
                    // vt_scan_state to "complete" on the UI side alongside it) and completion.
                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::VirusTotalStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

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

                let device_clone = device.clone();
                let api_key_clone = api_key.clone();
                let cancel_clone = self.ha_cancel.clone();
                let event_tx = self.event_tx.clone();

                // Spawn task to run HybridAnalysis scan in background
                smol::spawn(async move {
                    let store = crate::shared_store_stt::get_shared_store();
                    let installed_packages = store.get_installed_packages();
                    let package_risk_scores: HashMap<String, i32> = HashMap::new();
                    let progress = Arc::new(Mutex::new(Some(0.0)));
                    let progress_for_scan = progress.clone();
                    let progress_done = Arc::new(AtomicBool::new(false));

                    // run_hybridanalysis() spawns its own background std::thread and returns
                    // scanner_state (an Arc<Mutex<HashMap<..>>>) immediately, well before the
                    // scan itself finishes: the thread mutates that same HashMap in place as
                    // each package completes. Hand this Arc to the UI right away, rather than
                    // waiting for the whole scan to finish first, so results appear
                    // per-package as the scan runs instead of only once at the very end.
                    let (scanner_state, _rate_limiter, handle) =
                        crate::calc_hybridanalysis::run_hybridanalysis(
                            installed_packages,
                            device_clone,
                            api_key_clone,
                            submit_enabled,
                            package_risk_scores,
                            progress_for_scan,
                            cancel_clone,
                        );

                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

                    // Forward live progress to the UI while the scan runs.
                    let poll_task = smol::spawn(poll_scan_progress(
                        ScannerType::HybridAnalysis,
                        progress,
                        progress_done.clone(),
                        event_tx.clone(),
                    ));

                    // Wait for the background thread to actually finish before declaring
                    // completion (join happens on the blocking thread pool used by
                    // smol::unblock, not on the async executor).
                    smol::unblock(move || {
                        let _ = handle.join();
                    })
                    .await;

                    progress_done.store(true, Ordering::Relaxed);
                    poll_task.await;

                    // Send a final scanner state update (same Arc, but this also flips
                    // ha_scan_state to "complete" on the UI side alongside it) and completion.
                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisComplete))
                        .await;
                })
                .detach();

                Ok(())
            }
            ScanCommand::RunVirusTotalForPackage {
                device,
                api_key,
                submit_enabled,
                package,
            } => {
                // Reset cancellation flag
                if let Ok(mut cancel) = self.vt_cancel.lock() {
                    *cancel = false;
                }

                self.event_tx
                    .send(ViewModelEvent::Scan(ScanEvent::VirusTotalStarted))
                    .await?;

                let device_clone = device.clone();
                let api_key_clone = api_key.clone();
                let cancel_clone = self.vt_cancel.clone();
                let event_tx = self.event_tx.clone();

                // Spawn task to run a single-package VirusTotal scan in background
                smol::spawn(async move {
                    let package_risk_scores: HashMap<String, i32> = HashMap::new();
                    let progress = Arc::new(Mutex::new(Some(0.0)));
                    let progress_for_scan = progress.clone();
                    let progress_done = Arc::new(AtomicBool::new(false));

                    let (scanner_state, _rate_limiter, handle) = crate::calc_virustotal::run_virustotal(
                        vec![package],
                        device_clone,
                        api_key_clone,
                        submit_enabled,
                        package_risk_scores,
                        progress_for_scan,
                        cancel_clone,
                    );

                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::VirusTotalStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

                    let poll_task = smol::spawn(poll_scan_progress(
                        ScannerType::VirusTotal,
                        progress,
                        progress_done.clone(),
                        event_tx.clone(),
                    ));

                    smol::unblock(move || {
                        let _ = handle.join();
                    })
                    .await;

                    progress_done.store(true, Ordering::Relaxed);
                    poll_task.await;

                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::VirusTotalStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::VirusTotalComplete))
                        .await;
                })
                .detach();

                Ok(())
            }
            ScanCommand::RunHybridAnalysisForPackage {
                device,
                api_key,
                submit_enabled,
                package,
            } => {
                // Reset cancellation flag
                if let Ok(mut cancel) = self.ha_cancel.lock() {
                    *cancel = false;
                }

                self.event_tx
                    .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisStarted))
                    .await?;

                let device_clone = device.clone();
                let api_key_clone = api_key.clone();
                let cancel_clone = self.ha_cancel.clone();
                let event_tx = self.event_tx.clone();

                // Spawn task to run a single-package HybridAnalysis scan in background
                smol::spawn(async move {
                    let package_risk_scores: HashMap<String, i32> = HashMap::new();
                    let progress = Arc::new(Mutex::new(Some(0.0)));
                    let progress_for_scan = progress.clone();
                    let progress_done = Arc::new(AtomicBool::new(false));

                    let (scanner_state, _rate_limiter, handle) =
                        crate::calc_hybridanalysis::run_hybridanalysis(
                            vec![package],
                            device_clone,
                            api_key_clone,
                            submit_enabled,
                            package_risk_scores,
                            progress_for_scan,
                            cancel_clone,
                        );

                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

                    let poll_task = smol::spawn(poll_scan_progress(
                        ScannerType::HybridAnalysis,
                        progress,
                        progress_done.clone(),
                        event_tx.clone(),
                    ));

                    smol::unblock(move || {
                        let _ = handle.join();
                    })
                    .await;

                    progress_done.store(true, Ordering::Relaxed);
                    poll_task.await;

                    let _ = event_tx
                        .send(ViewModelEvent::Scan(ScanEvent::HybridAnalysisStateUpdated(
                            scanner_state.clone(),
                        )))
                        .await;

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

#[cfg(test)]
mod poll_scan_progress_tests {
    use super::*;

    #[test]
    fn emits_progress_events_until_done_then_stops() {
        smol::block_on(async {
            let (tx, rx) = smol::channel::unbounded();
            let progress = Arc::new(Mutex::new(Some(0.42_f32)));
            let done = Arc::new(AtomicBool::new(false));

            let poll_task = smol::spawn(poll_scan_progress(
                ScannerType::VirusTotal,
                progress.clone(),
                done.clone(),
                tx,
            ));

            // Let the poller sample a few times, then signal completion.
            smol::Timer::after(Duration::from_millis(50)).await;
            done.store(true, Ordering::Relaxed);
            poll_task.await;

            let mut saw_progress = false;
            while let Ok(event) = rx.try_recv() {
                match event {
                    ViewModelEvent::Scan(ScanEvent::ScanProgress { scanner, progress }) => {
                        assert!(matches!(scanner, ScannerType::VirusTotal));
                        assert_eq!(progress, 0.42);
                        saw_progress = true;
                    }
                    other => panic!("unexpected event: {:?}", other),
                }
            }
            assert!(
                saw_progress,
                "expected at least one ScanProgress event before done"
            );
        });
    }
}

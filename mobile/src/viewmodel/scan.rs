//! Scan actor - handles virus scanning operations

use crate::viewmodel::ViewModelEvent;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum ScanCommand {
    ScanVirusTotal { package: String, apk_path: String, force_upload: bool },
    ScanHybridAnalysis { package: String, apk_path: String, force_upload: bool },
    LoadStalkerwareIndicators,
    BatchScan { packages: Vec<String>, scanner: ScannerType },
}

#[derive(Debug, Clone)]
pub enum ScannerType {
    VirusTotal,
    HybridAnalysis,
    Both,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    VirusTotalResult { package: String, result: String },  // Simplified for now
    HybridAnalysisResult { package: String, result: String },
    StalkerwareIndicatorsLoaded,
    ScanProgress { scanner: String, progress: f32, current: usize, total: usize },
    Error { operation: String, error: String },
}

pub struct ScanActor {
    command_rx: smol::channel::Receiver<ScanCommand>,
    event_tx: smol::channel::Sender<ViewModelEvent>,
}

impl ScanActor {
    pub fn new(
        command_rx: smol::channel::Receiver<ScanCommand>,
        event_tx: smol::channel::Sender<ViewModelEvent>,
    ) -> Self {
        Self { command_rx, event_tx }
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
            ScanCommand::ScanVirusTotal { package, apk_path, force_upload } => {
                let package_clone = package.clone();
                let result = smol::unblock(move || {
                    // Use existing calc_virustotal functions
                    format!("VT scan result for {}", package_clone)  // Placeholder
                }).await;

                self.event_tx.send(ViewModelEvent::Scan(
                    ScanEvent::VirusTotalResult { package, result }
                )).await?;
            }
            ScanCommand::LoadStalkerwareIndicators => {
                smol::unblock(|| {
                    // Use existing calc_stalkerware functions
                }).await;

                self.event_tx.send(ViewModelEvent::Scan(
                    ScanEvent::StalkerwareIndicatorsLoaded
                )).await?;
            }
            _ => {} // Other commands similar pattern
        }
        Ok(())
    }

    async fn send_error(&self, operation: &str, error: anyhow::Error) {
        let _ = self.event_tx.send(ViewModelEvent::Scan(
            ScanEvent::Error {
                operation: operation.to_string(),
                error: error.to_string(),
            }
        )).await;
    }
}

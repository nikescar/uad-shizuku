//! Metadata actor - handles metadata fetching from various sources

use crate::viewmodel::ViewModelEvent;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum MetadataCommand {
    FetchGooglePlay { package: String },
    FetchFDroid { package: String },
    FetchApkMirror { package: String },
    BatchFetch { packages: Vec<String>, sources: Vec<MetadataSource> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataSource {
    GooglePlay,
    FDroid,
    ApkMirror,
}

#[derive(Debug, Clone)]
pub enum MetadataEvent {
    MetadataFetched { package: String, source: MetadataSource },
    FetchProgress { progress: f32, current: usize, total: usize },
    Error { operation: String, error: String },
}

pub struct MetadataActor {
    command_rx: smol::channel::Receiver<MetadataCommand>,
    event_tx: smol::channel::Sender<ViewModelEvent>,
}

impl MetadataActor {
    pub fn new(
        command_rx: smol::channel::Receiver<MetadataCommand>,
        event_tx: smol::channel::Sender<ViewModelEvent>,
    ) -> Self {
        Self { command_rx, event_tx }
    }

    pub async fn run(mut self) {
        loop {
            match self.command_rx.recv().await {
                Ok(cmd) => {
                    if let Err(e) = self.handle_command(cmd).await {
                        self.send_error("metadata", e).await;
                    }
                }
                Err(_) => {
                    log::info!("MetadataActor: shutting down");
                    break;
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: MetadataCommand) -> Result<()> {
        match cmd {
            MetadataCommand::FetchGooglePlay { package } => {
                smol::unblock(move || {
                    // Use existing metadata fetching logic
                    log::info!("Fetching Google Play metadata for {}", package);
                }).await?;

                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::MetadataFetched {
                        package,
                        source: MetadataSource::GooglePlay,
                    }
                )).await?;
            }
            MetadataCommand::FetchFDroid { package } => {
                smol::unblock(move || {
                    log::info!("Fetching F-Droid metadata for {}", package);
                }).await?;

                self.event_tx.send(ViewModelEvent::Metadata(
                    MetadataEvent::MetadataFetched {
                        package,
                        source: MetadataSource::FDroid,
                    }
                )).await?;
            }
            _ => {} // Other commands similar pattern
        }
        Ok(())
    }

    async fn send_error(&self, operation: &str, error: anyhow::Error) {
        let _ = self.event_tx.send(ViewModelEvent::Metadata(
            MetadataEvent::Error {
                operation: operation.to_string(),
                error: error.to_string(),
            }
        )).await;
    }
}

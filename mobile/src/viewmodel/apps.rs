//! Apps actor - handles FOSS app management

use crate::viewmodel::ViewModelEvent;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum AppsCommand {
    LoadFossAppList,
    InstallApp { package: String, apk_url: String },
    BatchInstall { apps: Vec<String> },
}

#[derive(Debug, Clone)]
pub enum AppsEvent {
    FossAppListLoaded { count: usize },
    AppInstalled { package: String },
    InstallProgress { package: String, progress: f32 },
    Error { operation: String, error: String },
}

pub struct AppsActor {
    command_rx: smol::channel::Receiver<AppsCommand>,
    event_tx: smol::channel::Sender<ViewModelEvent>,
}

impl AppsActor {
    pub fn new(
        command_rx: smol::channel::Receiver<AppsCommand>,
        event_tx: smol::channel::Sender<ViewModelEvent>,
    ) -> Self {
        Self { command_rx, event_tx }
    }

    pub async fn run(mut self) {
        loop {
            match self.command_rx.recv().await {
                Ok(cmd) => {
                    if let Err(e) = self.handle_command(cmd).await {
                        self.send_error("apps", e).await;
                    }
                }
                Err(_) => {
                    log::info!("AppsActor: shutting down");
                    break;
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: AppsCommand) -> Result<()> {
        match cmd {
            AppsCommand::LoadFossAppList => {
                let count = smol::unblock(|| {
                    // Use existing calc_foss functions
                    0  // Placeholder
                }).await;

                self.event_tx.send(ViewModelEvent::Apps(
                    AppsEvent::FossAppListLoaded { count }
                )).await?;
            }
            AppsCommand::InstallApp { package, apk_url } => {
                let package_clone = package.clone();
                let apk_url_clone = apk_url.clone();
                smol::unblock(move || {
                    // Use existing installation logic
                    log::info!("Installing {} from {}", package_clone, apk_url_clone);
                }).await;

                self.event_tx.send(ViewModelEvent::Apps(
                    AppsEvent::AppInstalled { package }
                )).await?;
            }
            _ => {} // Other commands similar pattern
        }
        Ok(())
    }

    async fn send_error(&self, operation: &str, error: anyhow::Error) {
        let _ = self.event_tx.send(ViewModelEvent::Apps(
            AppsEvent::Error {
                operation: operation.to_string(),
                error: error.to_string(),
            }
        )).await;
    }
}

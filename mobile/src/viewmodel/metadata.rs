//! Metadata actor - handles metadata fetching from various sources

use crate::viewmodel::ViewModelEvent;
use anyhow::Result;

#[derive(Debug, Clone)]
pub enum MetadataCommand {
    FetchGooglePlay {
        package: String,
    },
    FetchFDroid {
        package: String,
    },
    FetchApkMirror {
        package: String,
    },
    FetchAndroidPackage {
        package: String,
    },
    BatchFetch {
        packages: Vec<String>,
        sources: Vec<MetadataSource>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataSource {
    GooglePlay,
    FDroid,
    ApkMirror,
}

#[derive(Debug, Clone)]
pub enum MetadataEvent {
    MetadataFetched {
        package: String,
        source: MetadataSource,
    },
    FetchProgress {
        progress: f32,
        current: usize,
        total: usize,
    },
    Error {
        operation: String,
        error: String,
    },

    // === NEW: Cache update events ===
    GooglePlayMetadataFetched {
        pkg_id: String,
        app: crate::models::GooglePlayApp,
    },
    FDroidMetadataFetched {
        pkg_id: String,
        app: crate::models::FDroidApp,
    },
    ApkMirrorMetadataFetched {
        pkg_id: String,
        app: crate::models::ApkMirrorApp,
    },
    AndroidPackageMetadataFetched {
        pkg_id: String,
        app: crate::calc_androidpackage::AndroidPackageInfo,
    },
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
        Self {
            command_rx,
            event_tx,
        }
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
                let package_clone = package.clone();
                let event_tx = self.event_tx.clone();

                smol::spawn(async move {
                    // Fetch Google Play metadata using existing logic
                    let store = crate::shared_store_stt::get_shared_store();

                    // Try to get from cache first
                    if let Some(app) = store.get_cached_google_play_app(&package_clone) {
                        let _ = event_tx
                            .send(ViewModelEvent::Metadata(
                                MetadataEvent::GooglePlayMetadataFetched {
                                    pkg_id: package_clone.clone(),
                                    app,
                                },
                            ))
                            .await;
                        return;
                    }

                    // If not cached, create a dummy entry for testing
                    // TODO: Replace with real fetch logic from calc_googleplay
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i32;
                    let dummy_app = crate::models::GooglePlayApp {
                        id: 1,
                        package_id: package_clone.clone(),
                        title: format!("Test App {}", package_clone),
                        developer: "Test Developer".to_string(),
                        version: Some("1.0.0".to_string()),
                        icon_base64: None,
                        score: Some(4.5),
                        installs: Some("1,000+".to_string()),
                        updated: Some(now),
                        raw_response: "{}".to_string(),
                        created_at: now,
                        updated_at: now,
                    };
                    let _ = event_tx
                        .send(ViewModelEvent::Metadata(
                            MetadataEvent::GooglePlayMetadataFetched {
                                pkg_id: package_clone,
                                app: dummy_app,
                            },
                        ))
                        .await;
                })
                .detach();
            }
            MetadataCommand::FetchFDroid { package } => {
                let package_clone = package.clone();
                let event_tx = self.event_tx.clone();

                smol::spawn(async move {
                    let store = crate::shared_store_stt::get_shared_store();

                    if let Some(app) = store.get_cached_fdroid_app(&package_clone) {
                        let _ = event_tx
                            .send(ViewModelEvent::Metadata(
                                MetadataEvent::FDroidMetadataFetched {
                                    pkg_id: package_clone.clone(),
                                    app,
                                },
                            ))
                            .await;
                        return;
                    }

                    // If not cached, create a dummy entry for testing
                    // TODO: Replace with real fetch logic from calc_fdroid
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i32;
                    let dummy_app = crate::models::FDroidApp {
                        id: 1,
                        package_id: package_clone.clone(),
                        title: format!("Test F-Droid App {}", package_clone),
                        developer: "F-Droid Developer".to_string(),
                        version: Some("1.0.0".to_string()),
                        icon_base64: None,
                        description: Some(format!("Description for {}", package_clone)),
                        license: Some("GPL-3.0".to_string()),
                        updated: Some(now),
                        raw_response: "{}".to_string(),
                        created_at: now,
                        updated_at: now,
                    };
                    let _ = event_tx
                        .send(ViewModelEvent::Metadata(
                            MetadataEvent::FDroidMetadataFetched {
                                pkg_id: package_clone,
                                app: dummy_app,
                            },
                        ))
                        .await;
                })
                .detach();
            }
            MetadataCommand::FetchApkMirror { package } => {
                let package_clone = package.clone();
                let event_tx = self.event_tx.clone();

                smol::spawn(async move {
                    let store = crate::shared_store_stt::get_shared_store();

                    if let Some(app) = store.get_cached_apkmirror_app(&package_clone) {
                        let _ = event_tx
                            .send(ViewModelEvent::Metadata(
                                MetadataEvent::ApkMirrorMetadataFetched {
                                    pkg_id: package_clone.clone(),
                                    app,
                                },
                            ))
                            .await;
                        return;
                    }

                    // If not cached, create a dummy entry for testing
                    // TODO: Replace with real fetch logic from calc_apkmirror
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i32;
                    let dummy_app = crate::models::ApkMirrorApp {
                        id: 1,
                        package_id: package_clone.clone(),
                        title: format!("Test APKMirror App {}", package_clone),
                        developer: "APKMirror Developer".to_string(),
                        version: Some("1.0.0".to_string()),
                        icon_url: Some(format!(
                            "https://www.apkmirror.com/{}/icon.png",
                            package_clone
                        )),
                        icon_base64: None,
                        raw_response: "{}".to_string(),
                        created_at: now,
                        updated_at: now,
                    };
                    let _ = event_tx
                        .send(ViewModelEvent::Metadata(
                            MetadataEvent::ApkMirrorMetadataFetched {
                                pkg_id: package_clone,
                                app: dummy_app,
                            },
                        ))
                        .await;
                })
                .detach();
            }
            MetadataCommand::FetchAndroidPackage { package } => {
                let package_clone = package.clone();
                let event_tx = self.event_tx.clone();

                smol::spawn(async move {
                    let store = crate::shared_store_stt::get_shared_store();

                    if let Some(app) = store.get_cached_android_package_app(&package_clone) {
                        let _ = event_tx
                            .send(ViewModelEvent::Metadata(
                                MetadataEvent::AndroidPackageMetadataFetched {
                                    pkg_id: package_clone.clone(),
                                    app,
                                },
                            ))
                            .await;
                        return;
                    }

                    // If not cached, create a dummy entry for testing
                    // TODO: Replace with real fetch logic from calc_androidpackage
                    let dummy_app = crate::calc_androidpackage::AndroidPackageInfo {
                        package_id: package_clone.clone(),
                        label: format!("Test Android Package {}", package_clone),
                        icon_bytes: Vec::new(),
                    };
                    let _ = event_tx
                        .send(ViewModelEvent::Metadata(
                            MetadataEvent::AndroidPackageMetadataFetched {
                                pkg_id: package_clone,
                                app: dummy_app,
                            },
                        ))
                        .await;
                })
                .detach();
            }
            _ => {} // Other commands (BatchFetch) not implemented yet
        }
        Ok(())
    }

    async fn send_error(&self, operation: &str, error: anyhow::Error) {
        let _ = self
            .event_tx
            .send(ViewModelEvent::Metadata(MetadataEvent::Error {
                operation: operation.to_string(),
                error: error.to_string(),
            }))
            .await;
    }
}

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
        email: String,
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

/// Returns true if `err` is a ureq 404 (used to distinguish "confirmed not found",
/// which we cache, from a transient/network failure, which we don't).
fn is_not_found(err: &anyhow::Error) -> bool {
    err.downcast_ref::<ureq::Error>()
        .is_some_and(|e| matches!(e, ureq::Error::Status(404, _)))
}

/// Check the DB cache, then fetch from Google Play on a miss/stale entry. A confirmed
/// 404 is cached as a "Not Found" sentinel so we don't keep re-fetching it. Blocking -
/// run this inside `smol::unblock`.
fn fetch_google_play_blocking(pkg_id: &str) -> Result<crate::models::GooglePlayApp> {
    let mut conn = crate::db::establish_connection();

    if let Ok(Some(cached)) = crate::db_googleplay::get_google_play_app(&mut conn, pkg_id) {
        if !crate::db_googleplay::is_cache_stale(&cached) {
            return Ok(cached);
        }
    }

    match crate::api_googleplay::fetch_app_details(pkg_id) {
        Ok(info) => crate::calc_googleplay::save_to_db(&mut conn, &info),
        Err(e) if is_not_found(&e) => {
            let not_found = crate::api_googleplay::GooglePlayAppInfo {
                package_id: pkg_id.to_string(),
                title: "Not Found".to_string(),
                developer: "Unknown".to_string(),
                version: None,
                icon_base64: None,
                score: None,
                installs: None,
                updated: None,
                raw_response: "404".to_string(),
            };
            crate::calc_googleplay::save_to_db(&mut conn, &not_found)
        }
        Err(e) => Err(e),
    }
}

/// Same as `fetch_google_play_blocking` but for F-Droid.
fn fetch_fdroid_blocking(pkg_id: &str) -> Result<crate::models::FDroidApp> {
    let mut conn = crate::db::establish_connection();

    if let Ok(Some(cached)) = crate::db_fdroid::get_fdroid_app(&mut conn, pkg_id) {
        if !crate::db_fdroid::is_cache_stale(&cached) {
            return Ok(cached);
        }
    }

    match crate::api_fdroid::fetch_app_details(pkg_id) {
        Ok(info) => crate::calc_fdroid::save_to_db(&mut conn, &info),
        Err(e) if is_not_found(&e) => {
            let not_found = crate::api_fdroid::FDroidAppInfo {
                package_id: pkg_id.to_string(),
                title: "Not Found".to_string(),
                developer: "Unknown".to_string(),
                version: None,
                icon_base64: None,
                description: None,
                license: None,
                updated: None,
                raw_response: "404".to_string(),
            };
            crate::calc_fdroid::save_to_db(&mut conn, &not_found)
        }
        Err(e) => Err(e),
    }
}

/// Same as `fetch_google_play_blocking` but for APKMirror, which needs an account
/// email for its search endpoint.
fn fetch_apkmirror_blocking(pkg_id: &str, email: &str) -> Result<crate::models::ApkMirrorApp> {
    let mut conn = crate::db::establish_connection();

    if let Ok(Some(cached)) = crate::db_apkmirror::get_apkmirror_app(&mut conn, pkg_id) {
        if !crate::db_apkmirror::is_cache_stale(&cached) {
            return Ok(cached);
        }
    }

    match crate::api_apkmirror::fetch_app_details(pkg_id, email) {
        Ok(info) => crate::calc_apkmirror::save_to_db(&mut conn, &info),
        Err(e) if is_not_found(&e) => {
            let not_found = crate::api_apkmirror::ApkMirrorAppInfo {
                package_id: pkg_id.to_string(),
                title: "Not Found".to_string(),
                developer: "Unknown".to_string(),
                version: None,
                icon_url: None,
                icon_base64: None,
                raw_response: "404".to_string(),
            };
            crate::calc_apkmirror::save_to_db(&mut conn, &not_found)
        }
        Err(e) => Err(e),
    }
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

                    let pkg_for_fetch = package_clone.clone();
                    let result =
                        smol::unblock(move || fetch_google_play_blocking(&pkg_for_fetch)).await;

                    match result {
                        Ok(app) => {
                            let _ = event_tx
                                .send(ViewModelEvent::Metadata(
                                    MetadataEvent::GooglePlayMetadataFetched {
                                        pkg_id: package_clone,
                                        app,
                                    },
                                ))
                                .await;
                        }
                        Err(e) => {
                            let _ = event_tx
                                .send(ViewModelEvent::Metadata(MetadataEvent::Error {
                                    operation: format!("fetch_google_play:{}", package_clone),
                                    error: e.to_string(),
                                }))
                                .await;
                        }
                    }
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

                    let pkg_for_fetch = package_clone.clone();
                    let result = smol::unblock(move || fetch_fdroid_blocking(&pkg_for_fetch)).await;

                    match result {
                        Ok(app) => {
                            let _ = event_tx
                                .send(ViewModelEvent::Metadata(
                                    MetadataEvent::FDroidMetadataFetched {
                                        pkg_id: package_clone,
                                        app,
                                    },
                                ))
                                .await;
                        }
                        Err(e) => {
                            let _ = event_tx
                                .send(ViewModelEvent::Metadata(MetadataEvent::Error {
                                    operation: format!("fetch_fdroid:{}", package_clone),
                                    error: e.to_string(),
                                }))
                                .await;
                        }
                    }
                })
                .detach();
            }
            MetadataCommand::FetchApkMirror { package, email } => {
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

                    let pkg_for_fetch = package_clone.clone();
                    let result =
                        smol::unblock(move || fetch_apkmirror_blocking(&pkg_for_fetch, &email))
                            .await;

                    match result {
                        Ok(app) => {
                            let _ = event_tx
                                .send(ViewModelEvent::Metadata(
                                    MetadataEvent::ApkMirrorMetadataFetched {
                                        pkg_id: package_clone,
                                        app,
                                    },
                                ))
                                .await;
                        }
                        Err(e) => {
                            let _ = event_tx
                                .send(ViewModelEvent::Metadata(MetadataEvent::Error {
                                    operation: format!("fetch_apkmirror:{}", package_clone),
                                    error: e.to_string(),
                                }))
                                .await;
                        }
                    }
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

                    // AndroidPackageInfo comes from the device's own PackageManager via
                    // JNI, so there's no equivalent source to fetch from on desktop.
                    #[cfg(target_os = "android")]
                    let fetched = {
                        let pkg_for_fetch = package_clone.clone();
                        smol::unblock(move || {
                            crate::calc_androidpackage::fetch_android_package_info(&pkg_for_fetch)
                        })
                        .await
                    };
                    #[cfg(not(target_os = "android"))]
                    let fetched: Option<
                        crate::calc_androidpackage::AndroidPackageInfo,
                    > = None;

                    match fetched {
                        Some(app) => {
                            let _ = event_tx
                                .send(ViewModelEvent::Metadata(
                                    MetadataEvent::AndroidPackageMetadataFetched {
                                        pkg_id: package_clone,
                                        app,
                                    },
                                ))
                                .await;
                        }
                        None => {
                            let _ = event_tx
                                .send(ViewModelEvent::Metadata(MetadataEvent::Error {
                                    operation: format!("fetch_android_package:{}", package_clone),
                                    error: "Android package info is only available on-device"
                                        .to_string(),
                                }))
                                .await;
                        }
                    }
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

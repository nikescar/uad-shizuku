//! Debloat actor - handles package management and batch operations

use crate::adb::PackageFingerprint;
use crate::uad_shizuku_app::UadNgLists;
use crate::viewmodel::ViewModelEvent;
use anyhow::Result;

/// Filter criteria for packages
#[derive(Debug, Clone)]
pub struct PackageFilterCriteria {
    pub text_filter: Option<String>,
    pub category_filter: Option<String>,
    pub show_only_enabled: bool,
    pub hide_system_apps: bool,
}

/// Sort criteria for packages
#[derive(Debug, Clone)]
pub struct PackageSortCriteria {
    pub column: SortColumn,
    pub ascending: bool,
}

/// Sortable columns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    PackageName,
    LastUpdate,
    VersionCode,
}

/// Commands sent to DebloatActor
#[derive(Debug, Clone)]
pub enum DebloatCommand {
    LoadPackages { device: String, user: u32 },
    BatchUninstall { packages: Vec<String>, device: String },
    BatchDisable { packages: Vec<String>, device: String },
    BatchEnable { packages: Vec<String>, device: String },
    LoadUadNgLists,
    SetOptions { unsafe_app_remove: bool, expert_app_remove: bool },
    FilterPackages {
        packages: Vec<PackageFingerprint>,
        criteria: PackageFilterCriteria,
    },
    SortPackages { criteria: PackageSortCriteria },
}

/// Events sent from DebloatActor to ViewModel
#[derive(Debug, Clone)]
pub enum DebloatEvent {
    PackagesLoaded(Vec<PackageFingerprint>),
    UadNgListsLoaded(UadNgLists),
    StalkerwareIndicatorsLoaded(crate::calc_stalkerware_stt::StalkerwareIndicators),
    FilteredPackagesReady(Vec<PackageFingerprint>),
    BatchProgress {
        operation: String,
        progress: f32,      // 0.0 to 1.0
        current: usize,
        total: usize,
    },
    BatchComplete {
        operation: String,
        succeeded: usize,
        failed: usize,
    },
    Error {
        operation: String,
        error: String,
    },
}

/// Debloat actor state
struct DebloatActorState {
    current_device: Option<String>,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
}

/// Debloat actor - runs on background thread
pub struct DebloatActor {
    state: DebloatActorState,
    command_rx: smol::channel::Receiver<DebloatCommand>,
    event_tx: smol::channel::Sender<ViewModelEvent>,
    _metadata_tx: smol::channel::Sender<super::MetadataCommand>,
}

impl DebloatActor {
    pub fn new(
        command_rx: smol::channel::Receiver<DebloatCommand>,
        event_tx: smol::channel::Sender<ViewModelEvent>,
        metadata_tx: smol::channel::Sender<super::MetadataCommand>,
    ) -> Self {
        Self {
            state: DebloatActorState {
                current_device: None,
                unsafe_app_remove: false,
                expert_app_remove: false,
            },
            command_rx,
            event_tx,
            _metadata_tx: metadata_tx,
        }
    }

    pub async fn run(mut self) {
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

    async fn handle_command(&mut self, cmd: DebloatCommand) -> Result<()> {
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
            DebloatCommand::SetOptions { unsafe_app_remove, expert_app_remove } => {
                self.state.unsafe_app_remove = unsafe_app_remove;
                self.state.expert_app_remove = expert_app_remove;
            }
            DebloatCommand::FilterPackages { packages, criteria } => {
                self.filter_packages(packages, criteria).await?;
            }
            DebloatCommand::SortPackages { criteria } => {
                self.sort_packages(criteria).await?;
            }
        }
        Ok(())
    }

    async fn load_packages(&mut self, device: String, _user: u32) -> Result<()> {
        // Use smol::unblock for blocking ADB operations
        let device_clone = device.clone();
        let packages = smol::unblock(move || {
            crate::adb::get_all_packages_fingerprints(&device_clone)
        }).await?;

        self.state.current_device = Some(device);

        // Send event back to ViewModel
        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::PackagesLoaded(packages)
        )).await?;

        Ok(())
    }

    async fn batch_uninstall(&mut self, packages: Vec<String>, device: String) -> Result<()> {
        let total = packages.len();
        let mut succeeded = 0;
        let mut failed = 0;

        for (i, pkg) in packages.into_iter().enumerate() {
            let device_clone = device.clone();
            let pkg_clone = pkg.clone();

            // Uninstall in blocking thread pool
            let result = smol::unblock(move || {
                crate::adb::uninstall_app(&pkg_clone, &device_clone)
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

    async fn batch_disable(&mut self, packages: Vec<String>, device: String) -> Result<()> {
        let total = packages.len();
        let mut succeeded = 0;
        let mut failed = 0;

        for (i, pkg) in packages.into_iter().enumerate() {
            let device_clone = device.clone();
            let pkg_clone = pkg.clone();

            let result = smol::unblock(move || {
                crate::adb::disable_app_current_user(&pkg_clone, &device_clone, None)
            }).await;

            match result {
                Ok(_) => succeeded += 1,
                Err(e) => {
                    log::error!("Failed to disable {}: {}", pkg, e);
                    failed += 1;
                }
            }

            let progress = (i + 1) as f32 / total as f32;
            self.event_tx.send(ViewModelEvent::Debloat(
                DebloatEvent::BatchProgress {
                    operation: "disable".to_string(),
                    progress,
                    current: i + 1,
                    total,
                }
            )).await?;
        }

        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::BatchComplete {
                operation: "disable".to_string(),
                succeeded,
                failed,
            }
        )).await?;

        Ok(())
    }

    async fn batch_enable(&mut self, packages: Vec<String>, device: String) -> Result<()> {
        let total = packages.len();
        let mut succeeded = 0;
        let mut failed = 0;

        for (i, pkg) in packages.into_iter().enumerate() {
            let device_clone = device.clone();
            let pkg_clone = pkg.clone();

            let result = smol::unblock(move || {
                crate::adb::enable_app(&pkg_clone, &device_clone)
            }).await;

            match result {
                Ok(_) => succeeded += 1,
                Err(e) => {
                    log::error!("Failed to enable {}: {}", pkg, e);
                    failed += 1;
                }
            }

            let progress = (i + 1) as f32 / total as f32;
            self.event_tx.send(ViewModelEvent::Debloat(
                DebloatEvent::BatchProgress {
                    operation: "enable".to_string(),
                    progress,
                    current: i + 1,
                    total,
                }
            )).await?;
        }

        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::BatchComplete {
                operation: "enable".to_string(),
                succeeded,
                failed,
            }
        )).await?;

        Ok(())
    }

    async fn load_uad_ng_lists(&mut self) -> Result<()> {
        // Load UAD lists from embedded resources
        let lists = smol::unblock(move || {
            // For now, create empty lists - will be implemented properly later
            use std::collections::HashMap;
            UadNgLists {
                apps: HashMap::new(),
            }
        }).await;

        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::UadNgListsLoaded(lists)
        )).await?;

        // Also load stalkerware indicators
        let event_tx = self.event_tx.clone();
        smol::spawn(async move {
            let indicators = smol::unblock(move || {
                // Load stalkerware indicators from embedded resources
                const STALKERWARE_YAML: &str = include_str!("../../resources/stalkerware_ioc.yaml");
                crate::calc_stalkerware::parse_stalkerware_yaml(STALKERWARE_YAML)
            }).await;

            match indicators {
                Ok(indicators) => {
                    let _ = event_tx.send(ViewModelEvent::Debloat(
                        DebloatEvent::StalkerwareIndicatorsLoaded(indicators)
                    )).await;
                }
                Err(e) => {
                    log::error!("Failed to load stalkerware indicators: {}", e);
                }
            }
        }).detach();

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

    async fn filter_packages(&mut self, packages: Vec<PackageFingerprint>, criteria: PackageFilterCriteria) -> Result<()> {
        // Run filtering in background thread to avoid blocking
        let filtered = smol::unblock(move || {
            Self::apply_filters(packages, criteria)
        }).await;

        // Send filtered result back to ViewModel
        self.event_tx.send(ViewModelEvent::Debloat(
            DebloatEvent::FilteredPackagesReady(filtered)
        )).await?;

        Ok(())
    }

    async fn sort_packages(&mut self, criteria: PackageSortCriteria) -> Result<()> {
        // Clone current filtered packages (or all if not filtered)
        // Note: sorting operates on the result of filtering
        // For now, we'll just return success - actual sort will be implemented when needed
        // This maintains the interface defined in the spec

        log::info!("Sort packages requested: {:?}, ascending: {}", criteria.column, criteria.ascending);
        Ok(())
    }

    /// Apply filter criteria to packages (sync, runs in thread pool)
    fn apply_filters(packages: Vec<PackageFingerprint>, criteria: PackageFilterCriteria) -> Vec<PackageFingerprint> {
        packages.into_iter().filter(|pkg| {
            // Text filter: search in package name
            if let Some(ref text) = criteria.text_filter {
                if !text.is_empty() && !pkg.pkg.to_lowercase().contains(&text.to_lowercase()) {
                    return false;
                }
            }

            // Show only enabled filter
            if criteria.show_only_enabled {
                let is_enabled = pkg.users.iter().any(|u| u.enabled == 1);
                if !is_enabled {
                    return false;
                }
            }

            // Hide system apps filter
            if criteria.hide_system_apps {
                let is_system = pkg.flags.contains("SYSTEM");
                if is_system {
                    return false;
                }
            }

            // Category filter (if provided)
            // Note: this would need UAD lists integration, which is not in current state
            // For now, we skip this - will be implemented when UAD integration is added
            if let Some(ref _category) = criteria.category_filter {
                // TODO: integrate with UAD lists when available in actor state
            }

            true
        }).collect()
    }
}

use crate::adb::PackageFingerprint;
use crate::calc_androidpackage::AndroidPackageInfo;
use crate::calc_stalkerware_stt::StalkerwareIndicators;
use crate::models::{ApkMirrorApp, FDroidApp, GooglePlayApp};
use crate::shared_store_stt::{SharedStore, SharedStoreUpdate};
use crate::uad_shizuku_app::UadNgLists;
use eframe::egui;
use std::collections::HashMap;

impl Default for SharedStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedStore {
    pub fn new() -> Self {
        use crossbeam_queue::SegQueue;
        use std::sync::Mutex;

        Self {
            installed_packages: Mutex::new(Vec::new()),
            uad_ng_lists: Mutex::new(None),
            stalkerware_indicators: Mutex::new(None),
            google_play_textures: Mutex::new(HashMap::new()),
            fdroid_textures: Mutex::new(HashMap::new()),
            apkmirror_textures: Mutex::new(HashMap::new()),
            android_package_textures: Mutex::new(HashMap::new()),
            cached_google_play_apps: Mutex::new(HashMap::new()),
            cached_fdroid_apps: Mutex::new(HashMap::new()),
            cached_apkmirror_apps: Mutex::new(HashMap::new()),
            cached_android_package_apps: Mutex::new(HashMap::new()),
            update_queue: SegQueue::new(),
            ui_context: Mutex::new(None),
        }
    }

    /// Set the UI context for requesting repaints from background threads
    pub fn set_ui_context(&self, ctx: egui::Context) {
        if let Ok(mut ui_ctx) = self.ui_context.lock() {
            *ui_ctx = Some(ctx);
        }
    }

    /// Request a UI repaint from a background thread
    pub fn request_repaint(&self) {
        if let Ok(ui_ctx) = self.ui_context.lock() {
            if let Some(ctx) = ui_ctx.as_ref() {
                ctx.request_repaint();
            }
        }
    }

    /// Process all pending updates from the queue
    pub fn process_updates(&self) {
        while let Some(update) = self.update_queue.pop() {
            match update {
                SharedStoreUpdate::InstalledPackages(packages) => {
                    if let Ok(mut installed) = self.installed_packages.lock() {
                        *installed = packages;
                    }
                }
                SharedStoreUpdate::UadNgLists(lists) => {
                    if let Ok(mut uad) = self.uad_ng_lists.lock() {
                        *uad = lists;
                    }
                }
                SharedStoreUpdate::StalkerwareIndicators(indicators) => {
                    if let Ok(mut stalkerware) = self.stalkerware_indicators.lock() {
                        *stalkerware = indicators;
                    }
                }
                SharedStoreUpdate::CachedGooglePlayApp { pkg_id, app } => {
                    if let Ok(mut cache) = self.cached_google_play_apps.lock() {
                        cache.insert(pkg_id, app);
                    }
                }
                SharedStoreUpdate::CachedFDroidApp { pkg_id, app } => {
                    if let Ok(mut cache) = self.cached_fdroid_apps.lock() {
                        cache.insert(pkg_id, app);
                    }
                }
                SharedStoreUpdate::CachedApkMirrorApp { pkg_id, app } => {
                    if let Ok(mut cache) = self.cached_apkmirror_apps.lock() {
                        cache.insert(pkg_id, app);
                    }
                }
                SharedStoreUpdate::CachedAndroidPackageApp { pkg_id, app } => {
                    if let Ok(mut cache) = self.cached_android_package_apps.lock() {
                        cache.insert(pkg_id, app);
                    }
                }
            }
        }
    }

    // === Installed packages ===

    pub fn get_installed_packages(&self) -> Vec<PackageFingerprint> {
        self.installed_packages
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn set_installed_packages(&self, packages: Vec<PackageFingerprint>) {
        if let Ok(mut installed) = self.installed_packages.lock() {
            *installed = packages;
        }
    }

    pub fn queue_installed_packages(&self, packages: Vec<PackageFingerprint>) {
        self.update_queue
            .push(SharedStoreUpdate::InstalledPackages(packages));
    }

    // === UAD-NG lists ===

    pub fn get_uad_ng_lists(&self) -> Option<UadNgLists> {
        self.uad_ng_lists.lock().ok().and_then(|g| g.clone())
    }

    pub fn set_uad_ng_lists(&self, lists: Option<UadNgLists>) {
        if let Ok(mut uad) = self.uad_ng_lists.lock() {
            *uad = lists;
        }
    }

    pub fn queue_uad_ng_lists(&self, lists: Option<UadNgLists>) {
        self.update_queue.push(SharedStoreUpdate::UadNgLists(lists));
    }

    // === Stalkerware indicators ===

    pub fn get_stalkerware_indicators(&self) -> Option<StalkerwareIndicators> {
        self.stalkerware_indicators
            .lock()
            .ok()
            .and_then(|g| g.clone())
    }

    pub fn set_stalkerware_indicators(&self, indicators: Option<StalkerwareIndicators>) {
        if let Ok(mut stalkerware) = self.stalkerware_indicators.lock() {
            *stalkerware = indicators;
        }
    }

    pub fn queue_stalkerware_indicators(&self, indicators: Option<StalkerwareIndicators>) {
        self.update_queue
            .push(SharedStoreUpdate::StalkerwareIndicators(indicators));
    }

    // === Texture caches ===

    pub fn get_google_play_texture(&self, pkg_id: &str) -> Option<egui::TextureHandle> {
        self.google_play_textures
            .lock()
            .ok()
            .and_then(|g| g.get(pkg_id).cloned())
    }

    pub fn set_google_play_texture(&self, pkg_id: String, texture: egui::TextureHandle) {
        if let Ok(mut cache) = self.google_play_textures.lock() {
            cache.insert(pkg_id, texture);
        }
    }

    pub fn get_fdroid_texture(&self, pkg_id: &str) -> Option<egui::TextureHandle> {
        self.fdroid_textures
            .lock()
            .ok()
            .and_then(|g| g.get(pkg_id).cloned())
    }

    pub fn set_fdroid_texture(&self, pkg_id: String, texture: egui::TextureHandle) {
        if let Ok(mut cache) = self.fdroid_textures.lock() {
            cache.insert(pkg_id, texture);
        }
    }

    pub fn get_apkmirror_texture(&self, pkg_id: &str) -> Option<egui::TextureHandle> {
        self.apkmirror_textures
            .lock()
            .ok()
            .and_then(|g| g.get(pkg_id).cloned())
    }

    pub fn set_apkmirror_texture(&self, pkg_id: String, texture: egui::TextureHandle) {
        if let Ok(mut cache) = self.apkmirror_textures.lock() {
            cache.insert(pkg_id, texture);
        }
    }

    pub fn get_android_package_texture(&self, pkg_id: &str) -> Option<egui::TextureHandle> {
        self.android_package_textures
            .lock()
            .ok()
            .and_then(|g| g.get(pkg_id).cloned())
    }

    pub fn set_android_package_texture(&self, pkg_id: String, texture: egui::TextureHandle) {
        if let Ok(mut cache) = self.android_package_textures.lock() {
            cache.insert(pkg_id, texture);
        }
    }

    pub fn clear_all_textures(&self) {
        if let Ok(mut cache) = self.google_play_textures.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.fdroid_textures.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.apkmirror_textures.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.android_package_textures.lock() {
            cache.clear();
        }
    }

    // === Cached app info ===

    pub fn get_cached_google_play_apps(&self) -> HashMap<String, GooglePlayApp> {
        self.cached_google_play_apps
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn get_cached_google_play_app(&self, pkg_id: &str) -> Option<GooglePlayApp> {
        self.cached_google_play_apps
            .lock()
            .ok()
            .and_then(|g| g.get(pkg_id).cloned())
    }

    pub fn set_cached_google_play_app(&self, pkg_id: String, app: GooglePlayApp) {
        if let Ok(mut cache) = self.cached_google_play_apps.lock() {
            cache.insert(pkg_id, app);
        }
    }

    pub fn queue_cached_google_play_app(&self, pkg_id: String, app: GooglePlayApp) {
        self.update_queue
            .push(SharedStoreUpdate::CachedGooglePlayApp { pkg_id, app });
    }

    pub fn get_cached_fdroid_apps(&self) -> HashMap<String, FDroidApp> {
        self.cached_fdroid_apps
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn get_cached_fdroid_app(&self, pkg_id: &str) -> Option<FDroidApp> {
        self.cached_fdroid_apps
            .lock()
            .ok()
            .and_then(|g| g.get(pkg_id).cloned())
    }

    pub fn set_cached_fdroid_app(&self, pkg_id: String, app: FDroidApp) {
        if let Ok(mut cache) = self.cached_fdroid_apps.lock() {
            cache.insert(pkg_id, app);
        }
    }

    pub fn queue_cached_fdroid_app(&self, pkg_id: String, app: FDroidApp) {
        self.update_queue
            .push(SharedStoreUpdate::CachedFDroidApp { pkg_id, app });
    }

    pub fn get_cached_apkmirror_apps(&self) -> HashMap<String, ApkMirrorApp> {
        self.cached_apkmirror_apps
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn get_cached_apkmirror_app(&self, pkg_id: &str) -> Option<ApkMirrorApp> {
        self.cached_apkmirror_apps
            .lock()
            .ok()
            .and_then(|g| g.get(pkg_id).cloned())
    }

    pub fn set_cached_apkmirror_app(&self, pkg_id: String, app: ApkMirrorApp) {
        if let Ok(mut cache) = self.cached_apkmirror_apps.lock() {
            cache.insert(pkg_id, app);
        }
    }

    pub fn queue_cached_apkmirror_app(&self, pkg_id: String, app: ApkMirrorApp) {
        self.update_queue
            .push(SharedStoreUpdate::CachedApkMirrorApp { pkg_id, app });
    }

    pub fn get_cached_android_package_apps(&self) -> HashMap<String, AndroidPackageInfo> {
        self.cached_android_package_apps
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn get_cached_android_package_app(&self, pkg_id: &str) -> Option<AndroidPackageInfo> {
        self.cached_android_package_apps
            .lock()
            .ok()
            .and_then(|g| g.get(pkg_id).cloned())
    }

    pub fn set_cached_android_package_app(&self, pkg_id: String, app: AndroidPackageInfo) {
        if let Ok(mut cache) = self.cached_android_package_apps.lock() {
            cache.insert(pkg_id, app);
        }
    }

    pub fn clear_all_cached_apps(&self) {
        if let Ok(mut cache) = self.cached_google_play_apps.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.cached_fdroid_apps.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.cached_apkmirror_apps.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.cached_android_package_apps.lock() {
            cache.clear();
        }
    }

    // === Scanner states (DEPRECATED - now in ViewModel) ===
    // Stub methods for backward compatibility during migration

    pub fn get_vt_scanner_state(&self) -> Option<crate::calc_virustotal_stt::ScannerState> {
        // Scanner state migrated to ViewModel.state.vt_scanner_state
        None
    }

    pub fn set_vt_scanner_state(&self, _state: Option<crate::calc_virustotal_stt::ScannerState>) {
        // Scanner state migrated to ViewModel.state.vt_scanner_state
        // This method is now a no-op
    }

    pub fn get_ha_scanner_state(&self) -> Option<crate::calc_hybridanalysis_stt::ScannerState> {
        // Scanner state migrated to ViewModel.state.ha_scanner_state
        None
    }

    pub fn set_ha_scanner_state(
        &self,
        _state: Option<crate::calc_hybridanalysis_stt::ScannerState>,
    ) {
        // Scanner state migrated to ViewModel.state.ha_scanner_state
        // This method is now a no-op
    }
}

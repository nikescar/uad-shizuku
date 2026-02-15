use crate::adb::PackageFingerprint;
use crate::calc_androidpackage::AndroidPackageInfo;
use crate::calc_hybridanalysis::ScannerState as HaScannerState;
use crate::calc_virustotal::ScannerState as VtScannerState;
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
            google_play_textures: Mutex::new(HashMap::new()),
            fdroid_textures: Mutex::new(HashMap::new()),
            apkmirror_textures: Mutex::new(HashMap::new()),
            android_package_textures: Mutex::new(HashMap::new()),
            cached_google_play_apps: Mutex::new(HashMap::new()),
            cached_fdroid_apps: Mutex::new(HashMap::new()),
            cached_apkmirror_apps: Mutex::new(HashMap::new()),
            cached_android_package_apps: Mutex::new(HashMap::new()),
            vt_scanner_state: Mutex::new(None),
            ha_scanner_state: Mutex::new(None),
            update_queue: SegQueue::new(),
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
        self.update_queue
            .push(SharedStoreUpdate::UadNgLists(lists));
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

    // === Scanner states (scan tab only) ===

    pub fn get_vt_scanner_state(&self) -> Option<VtScannerState> {
        self.vt_scanner_state.lock().ok().and_then(|g| g.clone())
    }

    pub fn set_vt_scanner_state(&self, state: Option<VtScannerState>) {
        if let Ok(mut s) = self.vt_scanner_state.lock() {
            *s = state;
        }
    }

    pub fn get_ha_scanner_state(&self) -> Option<HaScannerState> {
        self.ha_scanner_state.lock().ok().and_then(|g| g.clone())
    }

    pub fn set_ha_scanner_state(&self, state: Option<HaScannerState>) {
        if let Ok(mut s) = self.ha_scanner_state.lock() {
            *s = state;
        }
    }
}

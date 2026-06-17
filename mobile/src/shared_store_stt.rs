use crate::adb::PackageFingerprint;
use crate::calc_androidpackage::AndroidPackageInfo;
use crate::calc_stalkerware_stt::StalkerwareIndicators;
use crate::models::{ApkMirrorApp, FDroidApp, GooglePlayApp};
use crate::uad_shizuku_app::UadNgLists;
use crossbeam_queue::SegQueue;
use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Update types for the shared store queue
pub enum SharedStoreUpdate {
    /// Update installed packages
    InstalledPackages(Vec<PackageFingerprint>),
    /// Update UAD-NG lists
    UadNgLists(Option<UadNgLists>),
    /// Update stalkerware indicators
    StalkerwareIndicators(Option<StalkerwareIndicators>),
    /// Update cached Google Play app
    CachedGooglePlayApp { pkg_id: String, app: GooglePlayApp },
    /// Update cached F-Droid app
    CachedFDroidApp { pkg_id: String, app: FDroidApp },
    /// Update cached APKMirror app
    CachedApkMirrorApp { pkg_id: String, app: ApkMirrorApp },
    /// Update cached Android Package app
    CachedAndroidPackageApp {
        pkg_id: String,
        app: AndroidPackageInfo,
    },
}

/// Shared store for UI resources and legacy state
///
/// # Migration Status
/// This store is transitioning to texture-only storage. Business state has been
/// migrated to ViewModel following MVVM pattern:
/// - Scanner states → ViewModel.state.vt_scanner_state / ha_scanner_state
/// - Metadata cache → ViewModel.state.cached_metadata
/// - Stalkerware → ViewModel.state.stalkerware_indicators
///
/// ## Current State
/// - Texture caches: ACTIVE (egui::TextureHandle lifetime requires special handling)
/// - Metadata caches: LEGACY (checked for backward compat, ViewModel is source of truth)
/// - Business state: LEGACY (ViewModel is authoritative, SharedStore has stubs)
pub struct SharedStore {
    // === LEGACY: Migrated to ViewModel.state ===
    /// LEGACY: Use ViewModel.state.packages instead
    pub installed_packages: Mutex<Vec<PackageFingerprint>>,
    /// LEGACY: Use ViewModel.state.uad_ng_lists instead
    pub uad_ng_lists: Mutex<Option<UadNgLists>>,
    /// LEGACY: Use ViewModel.state.stalkerware_indicators instead
    pub stalkerware_indicators: Mutex<Option<StalkerwareIndicators>>,

    // === ACTIVE: Texture caches (egui lifetime constraints) ===
    /// Texture cache for Google Play icons
    pub google_play_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    /// Texture cache for F-Droid icons
    pub fdroid_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    /// Texture cache for APKMirror icons
    pub apkmirror_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    /// Texture cache for Android Package icons
    pub android_package_textures: Mutex<HashMap<String, egui::TextureHandle>>,

    // === LEGACY: Metadata caches (ViewModel is source of truth) ===
    /// LEGACY: Use ViewModel.state.cached_metadata.google_play instead
    pub cached_google_play_apps: Mutex<HashMap<String, GooglePlayApp>>,
    /// LEGACY: Use ViewModel.state.cached_metadata.fdroid instead
    pub cached_fdroid_apps: Mutex<HashMap<String, FDroidApp>>,
    /// LEGACY: Use ViewModel.state.cached_metadata.apkmirror instead
    pub cached_apkmirror_apps: Mutex<HashMap<String, ApkMirrorApp>>,
    /// LEGACY: Use ViewModel.state.cached_metadata.android_package instead
    pub cached_android_package_apps: Mutex<HashMap<String, AndroidPackageInfo>>,

    // === Infrastructure ===
    /// Update queue for thread-safe updates from background threads
    pub update_queue: SegQueue<SharedStoreUpdate>,
    /// UI context for requesting repaints from background threads
    pub ui_context: Mutex<Option<egui::Context>>,
}

/// Global shared store instance
static SHARED_STORE: OnceLock<Arc<SharedStore>> = OnceLock::new();

/// Get the global shared store instance
pub fn get_shared_store() -> Arc<SharedStore> {
    SHARED_STORE
        .get_or_init(|| Arc::new(SharedStore::new()))
        .clone()
}

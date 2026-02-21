#[cfg(not(target_family = "wasm"))]
use crate::adb::PackageFingerprint;
#[cfg(not(target_family = "wasm"))]
use crate::calc_androidpackage::AndroidPackageInfo;
#[cfg(not(target_family = "wasm"))]
use crate::calc_hybridanalysis::ScannerState as HaScannerState;
#[cfg(not(target_family = "wasm"))]
use crate::calc_virustotal::ScannerState as VtScannerState;
#[cfg(not(target_family = "wasm"))]
use crate::models::{ApkMirrorApp, FDroidApp, GooglePlayApp};
use crate::uad_shizuku_app::UadNgLists;
#[cfg(not(target_family = "wasm"))]
use crossbeam_queue::SegQueue;
use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

// Wasm placeholder types
#[cfg(target_family = "wasm")]
type PackageFingerprint = String;
#[cfg(target_family = "wasm")]
type AndroidPackageInfo = String;
#[cfg(target_family = "wasm")]
type HaScannerState = String;
#[cfg(target_family = "wasm")]
type VtScannerState = String;
#[cfg(target_family = "wasm")]
type ApkMirrorApp = String;
#[cfg(target_family = "wasm")]
type FDroidApp = String;
#[cfg(target_family = "wasm")]
type GooglePlayApp = String;

// Wasm placeholder for SegQueue using a simple Vec in Mutex
#[cfg(target_family = "wasm")]
pub struct SegQueue<T>(Mutex<Vec<T>>);
#[cfg(target_family = "wasm")]
impl<T> SegQueue<T> {
    pub fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }
    pub fn push(&self, item: T) {
        if let Ok(mut queue) = self.0.lock() {
            queue.push(item);
        }
    }
    pub fn pop(&self) -> Option<T> {
        self.0.lock().ok().and_then(|mut queue| {
            if queue.is_empty() {
                None
            } else {
                Some(queue.remove(0))
            }
        })
    }
}

/// Update types for the shared store queue
pub enum SharedStoreUpdate {
    /// Update installed packages
    InstalledPackages(Vec<PackageFingerprint>),
    /// Update UAD-NG lists
    UadNgLists(Option<UadNgLists>),
    /// Update cached Google Play app
    CachedGooglePlayApp { pkg_id: String, app: GooglePlayApp },
    /// Update cached F-Droid app
    CachedFDroidApp { pkg_id: String, app: FDroidApp },
    /// Update cached APKMirror app
    CachedApkMirrorApp { pkg_id: String, app: ApkMirrorApp },
    /// Update cached Android Package app
    CachedAndroidPackageApp { pkg_id: String, app: AndroidPackageInfo },
}

/// Shared store for data that is accessed by both debloat and scan tabs
pub struct SharedStore {
    /// Installed packages list
    pub installed_packages: Mutex<Vec<PackageFingerprint>>,
    /// UAD-NG bloat lists
    pub uad_ng_lists: Mutex<Option<UadNgLists>>,
    /// Texture cache for Google Play icons
    pub google_play_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    /// Texture cache for F-Droid icons
    pub fdroid_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    /// Texture cache for APKMirror icons
    pub apkmirror_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    /// Texture cache for Android Package icons
    pub android_package_textures: Mutex<HashMap<String, egui::TextureHandle>>,
    /// Cached Google Play app info
    pub cached_google_play_apps: Mutex<HashMap<String, GooglePlayApp>>,
    /// Cached F-Droid app info
    pub cached_fdroid_apps: Mutex<HashMap<String, FDroidApp>>,
    /// Cached APKMirror app info
    pub cached_apkmirror_apps: Mutex<HashMap<String, ApkMirrorApp>>,
    /// Cached Android Package app info
    pub cached_android_package_apps: Mutex<HashMap<String, AndroidPackageInfo>>,
    /// VirusTotal scanner state (scan tab only)
    pub vt_scanner_state: Mutex<Option<VtScannerState>>,
    /// Hybrid Analysis scanner state (scan tab only)
    pub ha_scanner_state: Mutex<Option<HaScannerState>>,
    /// Update queue for thread-safe updates from background threads
    pub update_queue: SegQueue<SharedStoreUpdate>,
}

/// Global shared store instance
static SHARED_STORE: OnceLock<Arc<SharedStore>> = OnceLock::new();

/// Get the global shared store instance
pub fn get_shared_store() -> Arc<SharedStore> {
    SHARED_STORE
        .get_or_init(|| Arc::new(SharedStore::new()))
        .clone()
}

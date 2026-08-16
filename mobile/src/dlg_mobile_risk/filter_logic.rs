//! Filter predicates for the mobile IzzyRisk/Stalkerware risk table.
//!
//! Ported from `DlgDashCounterDetails::should_show_package` /
//! `matches_text_filter` / `get_display_name` (dlg_dashcounter_details.rs:2229-2665), with
//! `get_display_name` rebased onto the ViewModel's `cached_metadata` instead of the legacy
//! `shared_store`, per the design spec's decoupling goal.

use crate::adb_stt::PackageFingerprint;
use crate::dlg_mobile_risk::RiskCategory;
use crate::viewmodel::ViewModelState;

pub(crate) use crate::tab_debloat::is_package_enabled;

/// Applies the `show_only_enabled` / `hide_system_app` toggles.
pub fn should_show_package(
    package: &PackageFingerprint,
    show_only_enabled: bool,
    hide_system_app: bool,
) -> bool {
    let is_system = package.flags.contains("SYSTEM");
    let is_enabled = is_package_enabled(package);

    if show_only_enabled && !is_enabled {
        return false;
    }
    if hide_system_app && is_system {
        return false;
    }
    true
}

/// Display name for text-filter matching, sourced from the ViewModel's metadata cache.
/// Same priority order as the original `shared_store`-based version: Android package label,
/// then FDroid/GooglePlay/APKMirror title, then the package id itself.
pub fn get_display_name(pkg_id: &str, vm_state: &ViewModelState) -> String {
    if let Some(android_app) = vm_state.cached_metadata.get_android_package(pkg_id) {
        if !android_app.label.is_empty() {
            return android_app.label.to_lowercase();
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        if let Some(fdroid_app) = vm_state.cached_metadata.get_fdroid(pkg_id) {
            if !fdroid_app.title.is_empty() {
                return fdroid_app.title.to_lowercase();
            }
        }
        if let Some(gp_app) = vm_state.cached_metadata.get_google_play(pkg_id) {
            if !gp_app.title.is_empty() {
                return gp_app.title.to_lowercase();
            }
        }
        if let Some(am_app) = vm_state.cached_metadata.get_apkmirror(pkg_id) {
            if !am_app.title.is_empty() {
                return am_app.title.to_lowercase();
            }
        }
    }

    pkg_id.to_lowercase()
}

/// Case-insensitive substring match against package id, display name, and version.
pub fn matches_text_filter(
    text_filter: &str,
    package: &PackageFingerprint,
    vm_state: &ViewModelState,
) -> bool {
    if text_filter.is_empty() {
        return true;
    }
    let filter_lower = text_filter.to_lowercase();

    if package.pkg.to_lowercase().contains(&filter_lower) {
        return true;
    }
    if get_display_name(&package.pkg, vm_state).contains(&filter_lower) {
        return true;
    }
    if package.versionName.to_lowercase().contains(&filter_lower) {
        return true;
    }
    false
}

/// IzzyRisk risk-score bucket predicate. Mirrors the filter closure in
/// `render_izzyrisk_table` (dlg_dashcounter_details.rs:1555-1565).
pub fn matches_izzyrisk_category(category: &RiskCategory, score: i32) -> bool {
    match category {
        RiskCategory::IzzyRiskSafe => score == 0,
        RiskCategory::IzzyRiskNormal => (1..=10).contains(&score),
        RiskCategory::IzzyRiskModerate => (11..=20).contains(&score),
        RiskCategory::IzzyRiskHigh => score > 20,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adb_stt::{AdbPackageInfoUser, PackageFingerprint};

    fn make_package(pkg: &str, flags: &str, enabled: i32, installed: bool) -> PackageFingerprint {
        PackageFingerprint {
            pkg: pkg.to_string(),
            codePath: String::new(),
            versionCode: 1,
            versionName: "1.0".to_string(),
            flags: flags.to_string(),
            privateFlags: String::new(),
            installPermissions: Vec::new(),
            users: vec![AdbPackageInfoUser {
                userId: 0,
                ceDataInode: 0,
                deDataInode: 0,
                installed,
                hidden: false,
                suspended: false,
                distractionFlags: 0,
                stopped: false,
                notLaunched: false,
                enabled,
                instant: false,
                virtualField: false,
                quarantined: false,
                installReason: 0,
                dataDir: String::new(),
                firstInstallTime: String::new(),
                uninstallReason: 0,
                lastDisabledCaller: String::new(),
                gids: Vec::new(),
                runtimePermissions: Vec::new(),
            }],
            lastUpdateTime: String::new(),
            pkgChecksum: String::new(),
            dumpText: String::new(),
        }
    }

    #[test]
    fn test_should_show_package_no_filters() {
        let pkg = make_package("com.example.app", "", 1, true);
        assert!(should_show_package(&pkg, false, false));
    }

    #[test]
    fn test_should_show_package_hides_disabled_when_show_only_enabled() {
        let pkg = make_package("com.example.app", "", 2, true); // enabled=2 => disabled
        assert!(!should_show_package(&pkg, true, false));
        assert!(should_show_package(&pkg, false, false));
    }

    #[test]
    fn test_should_show_package_hides_system_when_hide_system_app() {
        let pkg = make_package("com.example.app", "SYSTEM", 1, true);
        assert!(!should_show_package(&pkg, false, true));
        assert!(should_show_package(&pkg, false, false));
    }

    #[test]
    fn test_matches_text_filter_empty_filter_matches_everything() {
        let pkg = make_package("com.example.app", "", 1, true);
        let vm_state = ViewModelState::default();
        assert!(matches_text_filter("", &pkg, &vm_state));
    }

    #[test]
    fn test_matches_text_filter_matches_package_id() {
        let pkg = make_package("com.example.app", "", 1, true);
        let vm_state = ViewModelState::default();
        assert!(matches_text_filter("example", &pkg, &vm_state));
        assert!(matches_text_filter("EXAMPLE", &pkg, &vm_state));
        assert!(!matches_text_filter("nomatch", &pkg, &vm_state));
    }

    #[test]
    fn test_matches_text_filter_matches_version() {
        let pkg = make_package("com.example.app", "", 1, true);
        let vm_state = ViewModelState::default();
        assert!(matches_text_filter("1.0", &pkg, &vm_state));
    }

    #[test]
    fn test_get_display_name_falls_back_to_pkg_id_when_no_metadata() {
        let vm_state = ViewModelState::default();
        assert_eq!(
            get_display_name("com.example.App", &vm_state),
            "com.example.app"
        );
    }

    #[test]
    fn test_matches_izzyrisk_category_buckets() {
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskSafe, 0));
        assert!(!matches_izzyrisk_category(&RiskCategory::IzzyRiskSafe, 1));
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskNormal, 1));
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskNormal, 10));
        assert!(!matches_izzyrisk_category(
            &RiskCategory::IzzyRiskNormal,
            11
        ));
        assert!(matches_izzyrisk_category(
            &RiskCategory::IzzyRiskModerate,
            11
        ));
        assert!(matches_izzyrisk_category(
            &RiskCategory::IzzyRiskModerate,
            20
        ));
        assert!(matches_izzyrisk_category(&RiskCategory::IzzyRiskHigh, 21));
        assert!(!matches_izzyrisk_category(&RiskCategory::IzzyRiskHigh, 20));
        assert!(!matches_izzyrisk_category(
            &RiskCategory::StalkerwareDetected,
            5
        ));
    }
}

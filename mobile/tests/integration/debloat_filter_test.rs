//! Integration test for debloat filtering functionality

use uad_shizuku::adb_stt::{AdbPackageInfoUser, PackageFingerprint};
use uad_shizuku::viewmodel::{DebloatEvent, ViewModel, ViewModelEvent};

/// Helper to create test package
fn create_test_package(name: &str, enabled: i32, is_system: bool) -> PackageFingerprint {
    let flags = if is_system { "SYSTEM" } else { "" }.to_string();

    PackageFingerprint {
        pkg: name.to_string(),
        codePath: "/system/app".to_string(),
        versionCode: 1,
        versionName: "1.0".to_string(),
        flags,
        privateFlags: String::new(),
        installPermissions: vec![],
        users: vec![AdbPackageInfoUser {
            userId: 0,
            ceDataInode: 0,
            deDataInode: 0,
            installed: true,
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
            gids: vec![],
            runtimePermissions: vec![],
        }],
        lastUpdateTime: String::new(),
        pkgChecksum: String::new(),
        dumpText: String::new(),
    }
}

#[test]
fn test_filter_packages_command() {
    // Arrange: Create ViewModel
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    // Create test packages
    let packages = vec![
        create_test_package("com.example.app1", 1, false),
        create_test_package("com.example.app2", 0, false),
        create_test_package("com.android.system1", 1, true),
        create_test_package("com.android.system2", 2, true),
    ];

    // Load packages into actor
    vm.load_packages_from_memory(packages.clone())
        .expect("Failed to load test packages");

    // Wait for PackagesLoaded event
    let timeout = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    while vm.state.packages.is_empty() && start.elapsed() < timeout {
        let _events = vm.poll_events(&ctx);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Act: Send FilterPackages command
    // For now, we'll test basic text filter
    let filter_result = vm.filter_packages(
        Some("example".to_string()), // text_filter
        None,                        // category_filter
        false,                       // show_only_enabled
        false,                       // hide_system_apps
    );

    // Assert: Command sent successfully
    assert!(filter_result.is_ok(), "Filter command should succeed");

    // Poll with timeout until filter event arrives
    let mut found_filter_event = false;
    let timeout = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();

    while !found_filter_event && start.elapsed() < timeout {
        let events = vm.poll_events(&ctx);
        for event in events {
            if let ViewModelEvent::Debloat(DebloatEvent::FilteredPackagesReady(_)) = event {
                found_filter_event = true;
                break;
            }
        }
        if !found_filter_event {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    assert!(
        found_filter_event,
        "Should receive FilteredPackagesReady event within timeout"
    );

    // Check that filtered_packages contains only matching packages
    assert_eq!(
        vm.state.filtered_packages.len(),
        2,
        "Should have 2 packages matching 'example'"
    );
    assert!(
        vm.state
            .filtered_packages
            .iter()
            .all(|p| p.pkg.contains("example")),
        "All filtered packages should contain 'example'"
    );
}

#[test]
fn test_filter_by_enabled_state() {
    // Arrange
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    let packages = vec![
        create_test_package("com.example.enabled", 1, false), // enabled
        create_test_package("com.example.disabled", 2, false), // disabled
        create_test_package("com.example.default", 0, false), // default
    ];

    vm.load_packages_from_memory(packages.clone())
        .expect("Failed to load test packages");

    // Wait for PackagesLoaded event
    let timeout = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    while vm.state.packages.is_empty() && start.elapsed() < timeout {
        let _events = vm.poll_events(&ctx);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Act: Filter to show only enabled
    let filter_result = vm.filter_packages(
        None, None, true, // show_only_enabled
        false,
    );

    assert!(filter_result.is_ok());

    // Poll with timeout until filter event arrives
    let mut found_filter_event = false;
    let timeout = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    while !found_filter_event && start.elapsed() < timeout {
        let events = vm.poll_events(&ctx);
        for event in events {
            if let ViewModelEvent::Debloat(DebloatEvent::FilteredPackagesReady(_)) = event {
                found_filter_event = true;
                break;
            }
        }
        if !found_filter_event {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    assert!(
        found_filter_event,
        "Should receive FilteredPackagesReady event within timeout"
    );

    // Assert: Should only show enabled packages (enabled == 1)
    assert_eq!(
        vm.state.filtered_packages.len(),
        1,
        "Should have 1 enabled package"
    );
    assert!(
        !vm.state.filtered_packages.is_empty(),
        "filtered_packages should not be empty"
    );
    assert_eq!(vm.state.filtered_packages[0].pkg, "com.example.enabled");
}

#[test]
fn test_filter_hide_system_apps() {
    // Arrange
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    let packages = vec![
        create_test_package("com.example.user", 1, false),
        create_test_package("com.android.system", 1, true),
    ];

    vm.load_packages_from_memory(packages.clone())
        .expect("Failed to load test packages");

    // Wait for PackagesLoaded event
    let timeout = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    while vm.state.packages.is_empty() && start.elapsed() < timeout {
        let _events = vm.poll_events(&ctx);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Act: Filter to hide system apps
    let filter_result = vm.filter_packages(
        None, None, false, true, // hide_system_apps
    );

    assert!(filter_result.is_ok());

    // Poll with timeout until filter event arrives
    let mut found_filter_event = false;
    let timeout = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    while !found_filter_event && start.elapsed() < timeout {
        let events = vm.poll_events(&ctx);
        for event in events {
            if let ViewModelEvent::Debloat(DebloatEvent::FilteredPackagesReady(_)) = event {
                found_filter_event = true;
                break;
            }
        }
        if !found_filter_event {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    assert!(
        found_filter_event,
        "Should receive FilteredPackagesReady event within timeout"
    );

    // Assert: Should only show user apps
    assert_eq!(
        vm.state.filtered_packages.len(),
        1,
        "Should have 1 user app"
    );
    assert!(
        !vm.state.filtered_packages.is_empty(),
        "filtered_packages should not be empty"
    );
    assert_eq!(vm.state.filtered_packages[0].pkg, "com.example.user");
}

#[test]
fn test_filter_no_filters_returns_all() {
    // Arrange
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    let packages = vec![
        create_test_package("com.example.app1", 1, false),
        create_test_package("com.example.app2", 2, false),
        create_test_package("com.android.system", 1, true),
    ];

    vm.load_packages_from_memory(packages.clone())
        .expect("Failed to load test packages");

    // Wait for PackagesLoaded event
    let timeout_load = std::time::Duration::from_secs(2);
    let start_load = std::time::Instant::now();
    while vm.state.packages.is_empty() && start_load.elapsed() < timeout_load {
        let _events = vm.poll_events(&ctx);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Act: Filter with no filters (should return all)
    let filter_result = vm.filter_packages(None, None, false, false);

    assert!(filter_result.is_ok());

    // Poll with timeout until filter completes
    let timeout = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    while vm.state.filtered_packages.len() < packages.len() && start.elapsed() < timeout {
        let _events = vm.poll_events(&ctx);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Assert: Should return all packages
    assert_eq!(
        vm.state.filtered_packages.len(),
        packages.len(),
        "Should return all packages when no filters applied"
    );
}

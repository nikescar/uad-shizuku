//! Integration test for TabDebloat + ViewModel filter flow
//!
//! This test verifies the end-to-end filter flow:
//! 1. TabDebloat UI state updated (simulated)
//! 2. Filter command sent to ViewModel
//! 3. DebloatActor processes filter
//! 4. FilteredPackagesReady event emitted
//! 5. UI state updated with filtered results

use uad_shizuku::adb_stt::{AdbPackageInfoUser, PackageFingerprint};
use uad_shizuku::tab_debloat::{DebloatFilter, TabDebloat};
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
fn test_full_filter_flow() {
    // Arrange: Create ViewModel and TabDebloat
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    let _tab = TabDebloat::new();

    // Create test packages
    let packages = vec![
        create_test_package("com.example.app1", 1, false),
        create_test_package("com.example.app2", 0, false),
        create_test_package("com.android.system1", 1, true),
        create_test_package("com.android.system2", 2, true),
    ];

    // Load packages into ViewModel
    vm.state.packages = packages.clone();

    // Act: Send filter command (simulating what TabDebloat would do)
    // In real usage, TabDebloat.render() would call this after debounce
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

    // Assert: Event received
    assert!(
        found_filter_event,
        "Should receive FilteredPackagesReady event within timeout"
    );

    // Assert: Filtered packages updated
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

    // Verify the complete flow worked:
    // Command → DebloatActor → Event → State Update
    // This validates the full MVVM architecture for filtering
}

#[test]
fn test_filter_with_category() {
    // Arrange: Create ViewModel and TabDebloat
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());
    let mut tab = TabDebloat::new();

    // Create test packages
    let packages = vec![
        create_test_package("com.example.app1", 1, false),
        create_test_package("com.example.app2", 0, false),
    ];

    vm.state.packages = packages.clone();

    // Act: Set category filter in TabDebloat state
    tab.state.active_filter = DebloatFilter {
        text_filter: String::new(),
        category_filter: Some("Recommended".to_string()), // Placeholder for future UAD category filtering
        show_only_enabled: false,
        hide_system_apps: false,
    };

    // Send filter command with category (simulating what TabDebloat would do)
    let filter_result = vm.filter_packages(None, Some("Recommended".to_string()), false, false);

    assert!(filter_result.is_ok(), "Filter command should succeed");

    // Poll with timeout until filter completes
    let timeout = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    let mut received_event = false;

    while !received_event && start.elapsed() < timeout {
        let events = vm.poll_events(&ctx);
        for event in events {
            if let ViewModelEvent::Debloat(DebloatEvent::FilteredPackagesReady(_)) = event {
                received_event = true;
                break;
            }
        }
        if !received_event {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    assert!(received_event, "Should receive FilteredPackagesReady event");

    // NOTE: This is a placeholder test for future category filtering functionality.
    // Category filtering is not yet implemented in DebloatActor, so we just verify
    // the command/event flow works. Once UAD category filtering is added:
    // 1. Update DebloatActor to filter by UAD category (Recommended, Advanced, etc.)
    // 2. Update this test to verify only packages in the selected category are returned
    // 3. Load uad_ng_lists into ViewModel before running this test

    // For now, just verify the filter command was accepted
    // Future: assert filtered_packages only contains packages with matching UAD category
}

#[test]
fn test_filter_flow_verifies_actor_event_state_chain() {
    // Arrange
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    let packages = vec![
        create_test_package("com.android.system", 1, true),
        create_test_package("com.example.user", 1, false),
    ];

    vm.state.packages = packages;

    // Act: Send filter command
    let filter_result = vm.filter_packages(
        None, None, false, true, // hide_system_apps
    );

    assert!(filter_result.is_ok());

    // Poll with timeout
    let timeout = std::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    let mut event_received = false;

    while !event_received && start.elapsed() < timeout {
        let events = vm.poll_events(&ctx);
        for event in events {
            if let ViewModelEvent::Debloat(DebloatEvent::FilteredPackagesReady(_)) = event {
                event_received = true;
                break;
            }
        }
        if !event_received {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    // Assert: Verify the complete chain
    assert!(event_received, "Command → Actor → Event chain verified");
    assert_eq!(
        vm.state.filtered_packages.len(),
        1,
        "Event → State Update verified"
    );
    assert_eq!(
        vm.state.filtered_packages[0].pkg, "com.example.user",
        "State contains correct filtered data"
    );

    // This test explicitly verifies the MVVM actor architecture:
    // 1. Command sent via channel (filter_packages)
    // 2. DebloatActor processes command in background thread
    // 3. Actor emits FilteredPackagesReady event
    // 4. UI polls events and updates state
    // Complete flow validated ✓
}

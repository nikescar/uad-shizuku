use uad_shizuku::viewmodel::{ViewModel, ViewModelEvent, ScanEvent};
use uad_shizuku::shared_store_stt::get_shared_store;
use std::time::Duration;

#[test]
fn test_virustotal_state_in_viewmodel() {
    // Setup: Create ViewModel with real smol runtime
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    // Action: Start VirusTotal scan (will fail initially - no device)
    let result = vm.run_virustotal("test_device".into(), "test_key".into(), false);
    assert!(result.is_ok(), "Failed to start VT scan: {:?}", result);

    // Allow event processing - poll multiple times with delays
    let mut found_state = false;
    let mut event_count = 0;
    for i in 0..10 {
        std::thread::sleep(Duration::from_millis(50));
        let events = vm.poll_events(&ctx);
        event_count += events.len();
        for event in &events {
            eprintln!("  Event: {:?}", event);
        }
        eprintln!("Poll {}: received {} events, state is {:?}", i, events.len(),
            vm.state.vt_scanner_state.as_ref().map(|s| {
                let state_map = s.lock().unwrap();
                format!("{} entries", state_map.len())
            }));
        if vm.state.vt_scanner_state.is_some() {
            found_state = true;
            break;
        }
    }

    // Verify: Scanner state appears in ViewModel.state (not SharedStore)
    assert!(found_state,
        "VirusTotal scanner state should be in ViewModel.state (received {} events total)", event_count);

    // Verify: NOT in SharedStore anymore
    let shared_store = get_shared_store();
    let store_state = shared_store.get_vt_scanner_state();
    assert!(store_state.is_none(),
        "VirusTotal scanner state should NOT be in SharedStore");
}

#[test]
fn test_hybridanalysis_state_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    vm.run_hybridanalysis("test_device".into(), "test_key".into(), false).ok();

    // Allow event processing - poll multiple times with delays
    let mut found_state = false;
    for _ in 0..10 {
        std::thread::sleep(Duration::from_millis(50));
        vm.poll_events(&ctx);
        if vm.state.ha_scanner_state.is_some() {
            found_state = true;
            break;
        }
    }

    assert!(found_state,
        "HybridAnalysis scanner state should be in ViewModel.state");

    let shared_store = get_shared_store();
    let store_state = shared_store.get_ha_scanner_state();
    assert!(store_state.is_none(),
        "HybridAnalysis scanner state should NOT be in SharedStore");
}

#[test]
fn test_scan_cancellation_clears_state() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    // Start scan
    vm.run_virustotal("test_device".into(), "test_key".into(), false).ok();
    std::thread::sleep(Duration::from_millis(100));
    vm.poll_events(&ctx);

    // Cancel scan
    vm.cancel_virustotal().ok();
    std::thread::sleep(Duration::from_millis(100));
    vm.poll_events(&ctx);

    // State should be cleared
    assert!(vm.state.vt_scanner_state.is_none(),
        "Cancelled scan should clear scanner state");
}

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

    // Allow event processing
    std::thread::sleep(Duration::from_millis(200));
    vm.poll_events(&ctx);

    // Verify: Scanner state appears in ViewModel.state (not SharedStore)
    assert!(vm.state.vt_scanner_state.is_some(),
        "VirusTotal scanner state should be in ViewModel.state");

    // Verify: NOT in SharedStore anymore
    let shared_store = get_shared_store();
    let store_state = shared_store.vt_scanner_state.lock().unwrap();
    assert!(store_state.is_none(),
        "VirusTotal scanner state should NOT be in SharedStore");
}

#[test]
fn test_hybridanalysis_state_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    vm.run_hybridanalysis("test_device".into(), "test_key".into(), false).ok();

    std::thread::sleep(Duration::from_millis(200));
    vm.poll_events(&ctx);

    assert!(vm.state.ha_scanner_state.is_some(),
        "HybridAnalysis scanner state should be in ViewModel.state");

    let shared_store = get_shared_store();
    let store_state = shared_store.ha_scanner_state.lock().unwrap();
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

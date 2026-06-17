use uad_shizuku::viewmodel::ViewModel;
use uad_shizuku::shared_store_stt::get_shared_store;
use std::time::Duration;

#[test]
fn test_stalkerware_indicators_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    // Action: Load UAD lists (should also load stalkerware)
    vm.load_uad_ng_lists().ok();

    std::thread::sleep(Duration::from_millis(300));
    vm.poll_events(&ctx);

    // Verify: Indicators in ViewModel
    assert!(vm.state.stalkerware_indicators.is_some(),
        "Stalkerware indicators should be in ViewModel.state");

    // Verify: NOT in SharedStore
    let shared_store = get_shared_store();
    let store_indicators = shared_store.stalkerware_indicators.lock().unwrap();
    assert!(store_indicators.is_none(),
        "Stalkerware indicators should NOT be in SharedStore");
}

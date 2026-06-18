use uad_shizuku::viewmodel::ViewModel;
use uad_shizuku::shared_store_stt::get_shared_store;
use std::time::Duration;

#[test]
fn test_google_play_metadata_cached_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    // Action: Fetch Google Play metadata
    vm.fetch_google_play_metadata("com.example.app".into()).ok();

    // Wait for background fetch
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);

    // Verify: Metadata in ViewModel cache
    let cached = vm.state.cached_metadata.get_google_play("com.example.app");
    assert!(cached.is_some(),
        "Google Play metadata should be cached in ViewModel");

    // Verify: NOT in SharedStore
    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_google_play_apps.lock().unwrap();
    assert!(store_cache.is_empty(),
        "Google Play metadata should NOT be in SharedStore");
}

#[test]
fn test_fdroid_metadata_cached_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    vm.fetch_fdroid_metadata("com.example.app".into()).ok();

    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);

    let cached = vm.state.cached_metadata.get_fdroid("com.example.app");
    assert!(cached.is_some(),
        "F-Droid metadata should be cached in ViewModel");

    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_fdroid_apps.lock().unwrap();
    assert!(store_cache.is_empty(),
        "F-Droid metadata should NOT be in SharedStore");
}

#[test]
fn test_apkmirror_metadata_cached_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    vm.fetch_apkmirror_metadata("com.example.app".into()).ok();

    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);

    let cached = vm.state.cached_metadata.get_apkmirror("com.example.app");
    assert!(cached.is_some(),
        "APKMirror metadata should be cached in ViewModel");

    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_apkmirror_apps.lock().unwrap();
    assert!(store_cache.is_empty(),
        "APKMirror metadata should NOT be in SharedStore");
}

#[test]
fn test_android_package_metadata_cached_in_viewmodel() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    vm.fetch_android_package_metadata("com.example.app".into()).ok();

    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);

    let cached = vm.state.cached_metadata.get_android_package("com.example.app");
    assert!(cached.is_some(),
        "Android Package metadata should be cached in ViewModel");

    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_android_package_apps.lock().unwrap();
    assert!(store_cache.is_empty(),
        "Android Package metadata should NOT be in SharedStore");
}

#[test]
fn test_metadata_cache_persists_across_calls() {
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    // Fetch once
    vm.fetch_google_play_metadata("com.example.app".into()).ok();
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);

    let first_fetch = vm.state.cached_metadata.get_google_play("com.example.app");
    assert!(first_fetch.is_some());

    // Fetch again - should use cache
    let second_fetch = vm.state.cached_metadata.get_google_play("com.example.app");
    assert!(second_fetch.is_some());

    // Should be same instance (cached)
    assert!(std::ptr::eq(first_fetch.unwrap(), second_fetch.unwrap()),
        "Metadata should persist in cache");
}

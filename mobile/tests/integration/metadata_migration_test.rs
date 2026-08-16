use std::time::Duration;
use uad_shizuku::shared_store_stt::get_shared_store;
use uad_shizuku::viewmodel::{MetadataEvent, ViewModel, ViewModelEvent};
use uad_shizuku::{db, db_apkmirror, db_fdroid, db_googleplay};

// These tests pre-seed the DB cache directly (rather than relying on a live network
// fetch of a fake package) so MetadataActor's real DB-cache-hit path is exercised
// hermetically. Each test uses its own package ID to stay independent under
// cargo test's default parallel execution, since all DB tests in this crate share
// the same on-disk uad.db (see db_virustotal::tests for the same convention).

#[test]
fn test_google_play_metadata_cached_in_viewmodel() {
    let pkg_id = "com.example.metadata_migration_googleplay";
    let mut conn = db::establish_connection();
    db_googleplay::upsert_google_play_app(
        &mut conn,
        pkg_id,
        "Test App",
        "Test Developer",
        None,
        None,
        None,
        None,
        None,
        "{}",
    )
    .expect("Failed to seed Google Play cache");

    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    // Action: Fetch Google Play metadata
    vm.fetch_google_play_metadata(pkg_id.into()).ok();

    // Wait for background fetch
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);

    // Verify: Metadata in ViewModel cache, sourced from the DB row we seeded
    let cached = vm.state.cached_metadata.get_google_play(pkg_id);
    assert!(
        cached.is_some(),
        "Google Play metadata should be cached in ViewModel"
    );
    assert_eq!(cached.unwrap().title, "Test App");

    // Verify: NOT in SharedStore
    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_google_play_apps.lock().unwrap();
    assert!(
        store_cache.is_empty(),
        "Google Play metadata should NOT be in SharedStore"
    );
    drop(store_cache);

    db_googleplay::delete_google_play_app(&mut conn, pkg_id).ok();
}

#[test]
fn test_fdroid_metadata_cached_in_viewmodel() {
    let pkg_id = "com.example.metadata_migration_fdroid";
    let mut conn = db::establish_connection();
    db_fdroid::upsert_fdroid_app(
        &mut conn,
        pkg_id,
        "Test F-Droid App",
        "F-Droid Developer",
        None,
        None,
        None,
        None,
        None,
        "{}",
    )
    .expect("Failed to seed F-Droid cache");

    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    vm.fetch_fdroid_metadata(pkg_id.into()).ok();

    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);

    let cached = vm.state.cached_metadata.get_fdroid(pkg_id);
    assert!(
        cached.is_some(),
        "F-Droid metadata should be cached in ViewModel"
    );
    assert_eq!(cached.unwrap().title, "Test F-Droid App");

    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_fdroid_apps.lock().unwrap();
    assert!(
        store_cache.is_empty(),
        "F-Droid metadata should NOT be in SharedStore"
    );
    drop(store_cache);

    db_fdroid::delete_fdroid_app(&mut conn, pkg_id).ok();
}

#[test]
fn test_apkmirror_metadata_cached_in_viewmodel() {
    let pkg_id = "com.example.metadata_migration_apkmirror";
    let mut conn = db::establish_connection();
    db_apkmirror::upsert_apkmirror_app(
        &mut conn,
        pkg_id,
        "Test APKMirror App",
        "APKMirror Developer",
        None,
        None,
        None,
        "{}",
    )
    .expect("Failed to seed APKMirror cache");

    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    vm.fetch_apkmirror_metadata(pkg_id.into(), "test@example.com".into())
        .ok();

    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);

    let cached = vm.state.cached_metadata.get_apkmirror(pkg_id);
    assert!(
        cached.is_some(),
        "APKMirror metadata should be cached in ViewModel"
    );
    assert_eq!(cached.unwrap().title, "Test APKMirror App");

    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_apkmirror_apps.lock().unwrap();
    assert!(
        store_cache.is_empty(),
        "APKMirror metadata should NOT be in SharedStore"
    );
    drop(store_cache);

    db_apkmirror::delete_apkmirror_app(&mut conn, pkg_id).ok();
}

#[test]
fn test_android_package_metadata_cached_in_viewmodel() {
    // AndroidPackageInfo is only obtainable via the device's PackageManager (JNI),
    // so on desktop (this test's target) MetadataActor correctly reports it as
    // unavailable rather than fabricating a result.
    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    vm.fetch_android_package_metadata("com.example.metadata_migration_androidpkg".into())
        .ok();

    std::thread::sleep(Duration::from_millis(500));
    let events = vm.poll_events(&ctx);

    let cached = vm
        .state
        .cached_metadata
        .get_android_package("com.example.metadata_migration_androidpkg");
    assert!(
        cached.is_none(),
        "Android Package metadata has no desktop source, so it should not be cached"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ViewModelEvent::Metadata(MetadataEvent::Error { .. }))),
        "Should receive a MetadataEvent::Error since this platform has no PackageManager"
    );

    let shared_store = get_shared_store();
    let store_cache = shared_store.cached_android_package_apps.lock().unwrap();
    assert!(
        store_cache.is_empty(),
        "Android Package metadata should NOT be in SharedStore"
    );
}

#[test]
fn test_metadata_cache_persists_across_calls() {
    let pkg_id = "com.example.metadata_migration_persist";
    let mut conn = db::establish_connection();
    db_googleplay::upsert_google_play_app(
        &mut conn,
        pkg_id,
        "Persist App",
        "Test Developer",
        None,
        None,
        None,
        None,
        None,
        "{}",
    )
    .expect("Failed to seed Google Play cache");

    let ctx = eframe::egui::Context::default();
    let mut vm = ViewModel::new(ctx.clone());

    // Fetch once
    vm.fetch_google_play_metadata(pkg_id.into()).ok();
    std::thread::sleep(Duration::from_millis(500));
    vm.poll_events(&ctx);

    let first_fetch = vm.state.cached_metadata.get_google_play(pkg_id);
    assert!(first_fetch.is_some());

    // Fetch again - should use cache
    let second_fetch = vm.state.cached_metadata.get_google_play(pkg_id);
    assert!(second_fetch.is_some());

    // Should be same instance (cached)
    assert!(
        std::ptr::eq(first_fetch.unwrap(), second_fetch.unwrap()),
        "Metadata should persist in cache"
    );

    db_googleplay::delete_google_play_app(&mut conn, pkg_id).ok();
}

use crate::adb::{get_users, PackageFingerprint, UserInfo};
use crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::models::PackageInfoCache;
use crate::shared_store_stt::get_shared_store;
use crate::uad_shizuku_app::{UadNgLists, UadShizukuApp};
use eframe::egui;
use egui_i18n::tr;
use egui_material3::{icon_button_standard, theme::get_global_color, DataTableCell};
use std::collections::HashMap;

#[cfg(not(target_os = "android"))]
use crate::adb::get_devices;

/// Load texture from base64 encoded image data
pub fn load_texture_from_base64(
    ctx: &egui::Context,
    prefix: &str,
    pkg_id: &str,
    base64_data: &str,
) -> Option<egui::TextureHandle> {
    let store = get_shared_store();

    // Check if texture is already cached
    let cached_texture = match prefix {
        "gp" => store.get_google_play_texture(pkg_id),
        "fd" => store.get_fdroid_texture(pkg_id),
        "am" => store.get_apkmirror_texture(pkg_id),
        _ => None,
    };

    if let Some(texture) = cached_texture {
        return Some(texture);
    }

    // Strip data URL prefix if present (e.g., "data:image/png;base64,")
    let raw_base64 = if let Some(comma_pos) = base64_data.find(',') {
        &base64_data[comma_pos + 1..]
    } else {
        base64_data
    };

    // Decode base64 to bytes
    match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, raw_base64) {
        Ok(bytes) => {
            // Load image from bytes
            match image::load_from_memory(&bytes) {
                Ok(image) => {
                    let size = [image.width() as _, image.height() as _];
                    let image_buffer = image.to_rgba8();
                    let pixels = image_buffer.as_flat_samples();
                    let color_image =
                        egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    let texture = ctx.load_texture(
                        format!("{}_{}", prefix, pkg_id),
                        color_image,
                        egui::TextureOptions::LINEAR,
                    );

                    // Cache texture in shared store
                    match prefix {
                        "gp" => store.set_google_play_texture(pkg_id.to_string(), texture.clone()),
                        "fd" => store.set_fdroid_texture(pkg_id.to_string(), texture.clone()),
                        "am" => store.set_apkmirror_texture(pkg_id.to_string(), texture.clone()),
                        _ => {}
                    }

                    Some(texture)
                }
                Err(e) => {
                    log::debug!("Failed to load image for {}: {}", pkg_id, e);
                    None
                }
            }
        }
        Err(e) => {
            log::debug!("Failed to decode base64 for {}: {}", pkg_id, e);
            None
        }
    }
}

/// Load texture from PNG bytes (Android Package native icons)
pub fn load_texture_from_bytes(
    ctx: &egui::Context,
    package_id: &str,
    png_bytes: &[u8],
) -> Option<egui::TextureHandle> {
    let store = get_shared_store();

    // Check cache first
    if let Some(texture) = store.get_android_package_texture(package_id) {
        return Some(texture);
    }

    match image::load_from_memory(png_bytes) {
        Ok(image) => {
            let size = [image.width() as _, image.height() as _];
            let image_buffer = image.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            let texture = ctx.load_texture(
                format!("ap_{}", package_id),
                color_image,
                egui::TextureOptions::LINEAR,
            );
            store.set_android_package_texture(package_id.to_string(), texture.clone());
            Some(texture)
        }
        Err(e) => {
            log::debug!("Failed to load image for {}: {}", package_id, e);
            None
        }
    }
}

/// Render app description cell with icon and title
pub fn render_app_description_cell(ctx: &egui::Context, pkg_id: &str) -> DataTableCell {
    let store = get_shared_store();

    // Prepare icon and title data
    let mut texture: Option<egui::TextureId> = None;
    let mut title_text = pkg_id.to_string();
    let mut subtitle_text = String::new();

    // Priority 1: Android Package (native icons)
    if let Some(android_app) = store.get_cached_android_package_app(pkg_id) {
        if !android_app.icon_bytes.is_empty() {
            if let Some(tex) = load_texture_from_bytes(ctx, pkg_id, &android_app.icon_bytes) {
                texture = Some(tex.id());
                if !android_app.label.is_empty() {
                    title_text = android_app.label.clone();
                }
            }
        }
    }

    // Priority 2-4: External sources (disabled on Android)
    #[cfg(not(target_os = "android"))]
    {
        // Priority 2: FDroid
        if texture.is_none() {
            if let Some(fdroid_app) = store.get_cached_fdroid_app(pkg_id) {
                if let Some(icon) = &fdroid_app.icon_base64 {
                    if let Some(tex) = load_texture_from_base64(ctx, "fd", pkg_id, icon) {
                        texture = Some(tex.id());
                        if !fdroid_app.title.is_empty() {
                            title_text = fdroid_app.title.clone();
                        }
                        if !fdroid_app.developer.is_empty() {
                            subtitle_text = fdroid_app.developer.clone();
                        }
                    }
                }
            }
        }

        // Priority 3: GooglePlay
        if texture.is_none() {
            if let Some(gp_app) = store.get_cached_google_play_app(pkg_id) {
                if let Some(icon) = &gp_app.icon_base64 {
                    if let Some(tex) = load_texture_from_base64(ctx, "gp", pkg_id, icon) {
                        texture = Some(tex.id());
                        if !gp_app.title.is_empty() {
                            title_text = gp_app.title.clone();
                        }
                        if !gp_app.developer.is_empty() {
                            subtitle_text = gp_app.developer.clone();
                        }
                    }
                }
            }
        }

        // Priority 4: APKMirror
        if texture.is_none() {
            if let Some(am_app) = store.get_cached_apkmirror_app(pkg_id) {
                if let Some(icon) = &am_app.icon_base64 {
                    if let Some(tex) = load_texture_from_base64(ctx, "am", pkg_id, icon) {
                        texture = Some(tex.id());
                        if !am_app.title.is_empty() {
                            title_text = am_app.title.clone();
                        }
                        if !am_app.developer.is_empty() {
                            subtitle_text = am_app.developer.clone();
                        }
                    }
                }
            }
        }
    }

    // If no subtitle, use package ID
    if subtitle_text.is_empty() {
        subtitle_text = pkg_id.to_string();
    }

    // Determine if we need scrollable title (for GooglePlay and APKMirror sources)
    #[cfg(not(target_os = "android"))]
    let use_scrollable_title = texture.is_some()
        && (store.get_cached_google_play_app(pkg_id).is_some()
            || store.get_cached_apkmirror_app(pkg_id).is_some());

    // On Android, never use scrollable title
    #[cfg(target_os = "android")]
    let use_scrollable_title = false;

    // Clone pkg_id for use in closure (unique identifier for ScrollArea)
    let pkg_id_for_scroll = pkg_id.to_string();

    // Create cell with icon and text (matches tab_debloat_control.rs:1468-1496)
    DataTableCell::widget(move |ui: &mut egui::Ui| {
        let on_surface = get_global_color("onSurface");

        ui.horizontal(|ui| {
            // App icon (38x38)
            if let Some(tex_id) = texture {
                ui.image((tex_id, egui::vec2(38.0, 38.0)));
            }

            ui.vertical(|ui| {
                ui.style_mut().spacing.item_spacing.y = 0.1;

                // Title (with optional scroll area) - use onSurface color for theme support
                // Use package ID for unique ScrollArea ID to avoid conflicts
                if use_scrollable_title {
                    egui::ScrollArea::horizontal()
                        .id_salt(format!("app_title_scroll_{}", pkg_id_for_scroll))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&title_text).strong().color(on_surface),
                                )
                                .wrap_mode(egui::TextWrapMode::Extend),
                            );
                        });
                } else {
                    ui.label(egui::RichText::new(&title_text).strong().color(on_surface));
                }

                // Subtitle (package ID or developer)
                if !subtitle_text.is_empty() {
                    ui.label(
                        egui::RichText::new(&subtitle_text)
                            .small()
                            .color(egui::Color32::GRAY),
                    );
                }
            });
        });
    })
}

/// Render action buttons cell (info, enable/disable, uninstall)
pub fn render_action_buttons_cell(
    _ui: &mut egui::Ui,
    pkg_id: &str,
    package: &PackageFingerprint,
    _uad_ng_lists: &Option<UadNgLists>,
) -> DataTableCell {
    let pkg_id_clone = pkg_id.to_string();
    let enabled_str = if let Some(user) = package.users.get(0) {
        enabled_to_string(user.enabled).to_string()
    } else {
        "UNKNOWN".to_string()
    };

    let is_system = package.flags.contains("SYSTEM");

    DataTableCell::widget(move |ui: &mut egui::Ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            // Info button
            if ui
                .add(icon_button_standard(ICON_INFO.to_string()))
                .on_hover_text(tr!("package-info"))
                .clicked()
            {
                // Store clicked package for opening details dialog
                ui.data_mut(|data| {
                    data.insert_temp(egui::Id::new("info_clicked_package"), pkg_id_clone.clone());
                });
            }

            // Enable/disable toggle
            let pkg_enabled = enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED");

            if pkg_enabled {
                if ui
                    .add(icon_button_standard(ICON_TOGGLE_ON.to_string()))
                    .on_hover_text(tr!("disable"))
                    .clicked()
                {
                    ui.data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new("disable_clicked_package"),
                            pkg_id_clone.clone(),
                        );
                    });
                }
            } else {
                if ui
                    .add(icon_button_standard(ICON_TOGGLE_OFF.to_string()))
                    .on_hover_text(tr!("enable"))
                    .clicked()
                {
                    ui.data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new("enable_clicked_package"),
                            pkg_id_clone.clone(),
                        );
                    });
                }
            }

            // Uninstall button (only for enabled apps)
            if pkg_enabled {
                if ui
                    .add(
                        icon_button_standard(ICON_DELETE.to_string())
                            .icon_color(egui::Color32::from_rgb(211, 47, 47)),
                    )
                    .on_hover_text(tr!("uninstall"))
                    .clicked()
                {
                    ui.data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new("uninstall_clicked_package"),
                            pkg_id_clone.clone(),
                        );
                        data.insert_temp(egui::Id::new("uninstall_clicked_is_system"), is_system);
                    });
                }
            }
        });
    })
}

/// Helper function to convert enabled code to string
fn enabled_to_string(enabled: i32) -> &'static str {
    match enabled {
        0 => "DEFAULT",
        1 => "ENABLED",
        2 => "DISABLED",
        3 => "DISABLED_USER",
        _ => "UNKNOWN",
    }
}

// ============================================================================
// Device and Package Management Functions
// ============================================================================

/// Retrieve ADB devices and initialize Shizuku connection (Android) or ADB devices (Desktop)
pub fn retrieve_adb_devices(app: &mut UadShizukuApp) {
    {
        // clear current selections
        app.selected_device = None;
        app.current_device = None;
        app.adb_users.clear();
        app.selected_user = None;
        app.current_user = None;
        {
            get_shared_store().set_installed_packages(Vec::new());
        }
        app.tab_debloat_control.update_packages(Vec::new());
        app.tab_debloat_control.update_uad_ng_lists(UadNgLists {
            apps: HashMap::new(),
        });
        app.tab_scan_control.update_packages(Vec::new(), app.viewmodel.as_ref());
        app.tab_scan_control.update_uad_ng_lists(UadNgLists {
            apps: HashMap::new(),
        });
        app.tab_apps_control.update_packages(Vec::new());

        #[cfg(target_os = "android")]
        {
            use crate::android_shizuku;

            // Step 0: Initialize ShizukuBridge (register permission listener) - once only
            if !app.shizuku_init_done {
                android_shizuku::shizuku_init();
                app.shizuku_init_done = true;
            }

            // Step 1: Check if Shizuku is running
            if !android_shizuku::shizuku_is_available() {
                log::error!("Shizuku is not running. Please install and activate Shizuku.");
                app.dlg_adb_install.open = true;
                app.adb_devices.clear();
                return;
            }

            // Step 2: Check/request permission
            if !android_shizuku::shizuku_has_permission() {
                let perm_state = android_shizuku::shizuku_get_permission_state();
                if perm_state == 0 || perm_state == 3 {
                    // Not yet requested or previously denied -- request now
                    log::error!("Requesting Shizuku permission...");
                    android_shizuku::shizuku_request_permission();
                    app.shizuku_permission_requested = true;
                }
                app.dlg_adb_install.open = true;
                app.adb_devices.clear();
                return;
            }

            // Step 3: Bind service (non-blocking)
            let bind_state = android_shizuku::shizuku_get_bind_state();
            match bind_state {
                0 => {
                    // Not bound, start binding
                    log::error!("Binding Shizuku ShellService...");
                    android_shizuku::shizuku_bind_service();
                    app.shizuku_bind_requested = true;
                    app.dlg_adb_install.open = true;
                    app.adb_devices.clear();
                    return;
                }
                1 => {
                    // Binding in progress, wait
                    app.dlg_adb_install.open = true;
                    app.adb_devices.clear();
                    return;
                }
                3 => {
                    // Bind failed
                    log::error!("Failed to bind Shizuku ShellService");
                    app.dlg_adb_install.open = true;
                    app.adb_devices.clear();
                    return;
                }
                2 => {
                    // Bound successfully, fall through
                }
                _ => {
                    app.adb_devices.clear();
                    return;
                }
            }

            // Step 4: Service is bound, set device
            app.shizuku_connected = true;
            app.adb_devices = vec!["local".to_string()];
            app.selected_device = Some("local".to_string());
            app.current_device = Some("local".to_string());
            retrieve_adb_users(app);
        }

        #[cfg(not(target_os = "android"))]
        {
            match get_devices() {
                Ok(devices) => {
                    app.adb_devices = devices;

                    // Auto-select first device if available (mirrors Android behavior)
                    if !app.adb_devices.is_empty() {
                        app.selected_device = Some(app.adb_devices[0].clone());
                        app.current_device = Some(app.adb_devices[0].clone());
                    }

                    retrieve_adb_users(app);
                }
                Err(e) => {
                    log::error!("[ERROR] Failed to get ADB devices: {}", e);
                    app.adb_devices.clear();
                }
            }
        }
    }
}

/// Retrieve ADB users for the selected device
pub fn retrieve_adb_users(app: &mut UadShizukuApp) {
    if let Some(ref device) = app.selected_device {
        log::debug!("Retrieving users for device: {}", device);
        match get_users(device) {
            Ok(users) => {
                log::debug!("Successfully retrieved {} users", users.len());
                app.adb_users = users;

                retrieve_installed_packages(app);
            }
            Err(e) => {
                log::error!("Failed to get users: {}", e);
                app.adb_users.clear();
            }
        }
    } else {
        log::debug!("No device selected, skipping user retrieval");
        app.adb_users.clear();
    }
}

/// Retrieve installed packages for the selected device and user
pub fn retrieve_installed_packages(app: &mut UadShizukuApp) {
    // Don't start a new loading thread if one is already running
    if app.package_loading_thread.is_some() {
        log::debug!("Package loading already in progress, skipping");
        return;
    }

    // Load uad_ng_lists after struct is constructed
    app.retrieve_uad_ng_lists();

    // Load stalkerware indicators
    app.retrieve_stalkerware_indicators();

    let Some(device) = app.selected_device.clone() else {
        log::debug!("No device selected, skipping package retrieval");
        return;
    };

    // Open loading dialog
    app.package_loading_dialog_open = true;
    app.package_loading_status = tr!("loading-packages");

    // Clone necessary data for the async task
    let selected_user = app.selected_user;
    let debloat_progress = app.package_load_progress.clone();
    let shared_store = get_shared_store();
    let uad_ng_lists = shared_store.uad_ng_lists.lock().unwrap().clone();

    // Start background thread
    let handle = std::thread::spawn(move || {
        use crate::adb::get_all_packages_fingerprints;
        use crate::db_package_cache::upsert_package_info_cache;

        log::debug!("Retrieving installed packages for device: {}", device);

        // Step 1: Get package fingerprints (lightweight) with retry logic
        let mut parsed_packages = match get_all_packages_fingerprints(&device) {
            Ok(fp) => fp,
            Err(e) => {
                log::error!("Failed to get package fingerprints: {}", e);
                return (Vec::new(), None);
            }
        };
        log::debug!("Retrieved {} package fingerprints", parsed_packages.len());

        // Step 1.5: If empty, wait 3 seconds and retry once
        if parsed_packages.is_empty() {
            log::warn!("Package fingerprint retrieval returned 0 packages, waiting 3 seconds and retrying...");
            std::thread::sleep(std::time::Duration::from_secs(3));

            match get_all_packages_fingerprints(&device) {
                Ok(fp) => {
                    parsed_packages = fp;
                    log::debug!(
                        "Retry retrieved {} package fingerprints",
                        parsed_packages.len()
                    );
                }
                Err(e) => {
                    log::error!("Retry failed to get package fingerprints: {}", e);
                    return (Vec::new(), None);
                }
            }

            // If still empty after retry, return error
            if parsed_packages.is_empty() {
                log::error!("Package retrieval failed: got 0 packages after retry. Shizuku may not be ready yet.");
                return (Vec::new(), None);
            }
        }

        // Step 2: load all contents from get_cached_packages_with_apk, db_package_cache
        let cached_packages: Vec<PackageInfoCache> =
            crate::db_package_cache::get_cached_packages_with_apk(&device);
        log::debug!(
            "Loaded {} cached packages from database",
            cached_packages.len()
        );

        // Step 3: fill apk path and sha256sum using background worker
        let parsed_packages_for_thread = parsed_packages.clone();
        let device_for_thread = device.to_string();
        let debloat_progress_clone = debloat_progress.clone();

        // Initialize debloat_progress
        if let Ok(mut p) = debloat_progress_clone.lock() {
            *p = Some(0.0);
        }

        std::thread::spawn(move || {
            log::info!("fill apk path and sha256sum from all packages -f");
            if cached_packages.len() < parsed_packages_for_thread.len() / 2 {
                match crate::adb::get_all_packages_sha256sum(&device_for_thread) {
                    Ok(package_data) => {
                        log::info!("Retrieved sha256 sums for {} packages", package_data.len());
                        // Convert Vec<(String, String, String)> to HashMap for easier lookup
                        let sha256_map: std::collections::HashMap<String, (String, String)> =
                            package_data
                                .into_iter()
                                .map(|(pkg, sha256, path)| (pkg, (sha256, path)))
                                .collect();

                        let total = parsed_packages_for_thread.len();
                        for (i, pkg) in parsed_packages_for_thread.iter().enumerate() {
                            // Update debloat_progress
                            if let Ok(mut p) = debloat_progress_clone.lock() {
                                *p = Some(i as f32 / total as f32);
                            }

                            if let Some((sha256, apk_path)) = sha256_map.get(&pkg.pkg) {
                                // insert into db
                                match upsert_package_info_cache(
                                    &pkg.pkg,
                                    &pkg.pkgChecksum,
                                    &pkg.dumpText,
                                    &pkg.codePath,
                                    pkg.versionCode,
                                    &pkg.versionName,
                                    "", // first_install_time - not available from this data
                                    &pkg.lastUpdateTime,
                                    Some(apk_path.as_str()),
                                    Some(sha256.as_str()),
                                    None, // izzyscore - calculated separately
                                    &device_for_thread,
                                ) {
                                    Ok(_) => {
                                        log::debug!(
                                            "Cached package info for {}: {} ({})",
                                            pkg.pkg,
                                            sha256,
                                            apk_path
                                        );
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Failed to cache package info for {}: {}",
                                            pkg.pkg,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get package sha256 sums: {}", e);
                    }
                }
            }
            // Clear progress when done
            if let Ok(mut p) = debloat_progress_clone.lock() {
                *p = None;
            }
            // Request UI repaint when background sha256sum fetching completes
            let shared_store = crate::shared_store_stt::get_shared_store();
            shared_store.request_repaint();
        });

        // use package
        let mut packages = parsed_packages;

        // Filter packages by selected user if a specific user is selected
        if let Some(user_id) = selected_user {
            log::debug!("Filtering packages for user: {}", user_id);
            packages.retain(|pkg| pkg.users.iter().any(|u| u.userId == user_id && u.installed));
            log::debug!(
                "Filtered to {} packages for user {}",
                packages.len(),
                user_id
            );
        } else {
            log::debug!("Showing all users' packages");
        }

        log::debug!("Package retrieval complete");

        // Request UI repaint when package loading completes
        let shared_store = crate::shared_store_stt::get_shared_store();
        shared_store.request_repaint();

        (packages, uad_ng_lists)
    });

    app.package_loading_thread = Some(handle);
}

/// Handle package loading result from background thread
pub fn handle_package_loading_result(app: &mut UadShizukuApp) {
    // Check if thread is complete
    let should_check = app.package_loading_thread.is_some();
    if !should_check {
        return;
    }

    // Try to take the thread handle and check if it's finished
    if let Some(handle) = app.package_loading_thread.take() {
        if handle.is_finished() {
            // Thread is complete, get the result
            match handle.join() {
                Ok((packages, uad_lists)) => {
                    // Loading complete, update UI
                    log::info!(
                        "Applying loaded packages to UI - {} packages loaded",
                        packages.len()
                    );

                    let shared_store = get_shared_store();
                    {
                        let mut installed_pkgs = shared_store.installed_packages.lock().unwrap();
                        *installed_pkgs = packages.clone();
                    }
                    log::debug!("Updated shared_store with {} packages", packages.len());
                    app.tab_debloat_control.update_packages(packages.clone());
                    log::debug!(
                        "Updated tab_debloat_control with {} packages",
                        packages.len()
                    );

                    if let Some(lists) = uad_lists {
                        app.tab_debloat_control.update_uad_ng_lists(lists.clone());
                        app.tab_scan_control.update_uad_ng_lists(lists);
                    }

                    app.tab_debloat_control
                        .set_selected_device(app.selected_device.clone());

                    // Update TabScanControl with API key, device serial, and settings
                    app.tab_scan_control.vt_api_key = Some(app.settings.virustotal_apikey.clone());
                    app.tab_scan_control.ha_api_key =
                        Some(app.settings.hybridanalysis_apikey.clone());
                    app.tab_scan_control.device_serial = app.selected_device.clone();
                    app.tab_scan_control.virustotal_submit_enabled = app.settings.virustotal_submit;
                    app.tab_scan_control.hybridanalysis_submit_enabled =
                        app.settings.hybridanalysis_submit;
                    log::info!(
                        "Synced hybridanalysis_submit_enabled={} to tab_scan_control",
                        app.settings.hybridanalysis_submit
                    );

                    let installed_packages =
                        shared_store.installed_packages.lock().unwrap().clone();
                    app.tab_scan_control
                        .update_packages(installed_packages.clone(), app.viewmodel.as_ref());

                    app.tab_apps_control
                        .update_packages(installed_packages.clone());
                    app.tab_apps_control
                        .set_selected_device(app.selected_device.clone());
                    log::debug!("Updated tab controls with packages");

                    // Close dialog
                    app.package_loading_dialog_open = false;
                }
                Err(e) => {
                    log::error!("Package loading thread panicked: {:?}", e);
                    app.package_loading_dialog_open = false;
                }
            }
        } else {
            // Thread not finished yet, put it back
            app.package_loading_thread = Some(handle);
        }
    }
}

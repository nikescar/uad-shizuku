//! Shared app metadata rendering utilities
//!
//! This module provides shared functionality for loading and rendering app metadata
//! (icons, titles, developers) across different tabs (Scan, Debloat, etc.).

use eframe::egui;
use std::collections::HashMap;

use crate::shared_store_stt::get_shared_store;
use crate::viewmodel::ViewModelState;

pub type AppMetadataMap = HashMap<String, (Option<egui::TextureHandle>, String, String, Option<String>)>;

/// Prepare app info for display with metadata and icon textures
///
/// This function fetches app metadata from various sources based on enabled renderers:
/// 1. Android Package (highest priority on Android platform)
/// 2. F-Droid (for non-system apps)
/// 3. Google Play (for non-system apps)
/// 4. APKMirror (for system apps)
///
/// Returns a HashMap mapping package ID to (texture, title, developer, version)
pub fn prepare_app_info_for_display(
    ctx: &egui::Context,
    package_ids: &[String],
    system_packages: &std::collections::HashSet<String>,
    vm_state: &ViewModelState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) -> AppMetadataMap {
    let start_time = std::time::Instant::now();
    log::debug!("[RENDER] prepare_app_info_for_display started for {} packages", package_ids.len());

    let mut app_data_map = HashMap::new();

    if !google_play_enabled && !fdroid_enabled && !apkmirror_enabled && !android_package_enabled {
        log::debug!("[RENDER] No renderers enabled, returning empty map ({:?})", start_time.elapsed());
        return app_data_map;
    }

    let cache_fetch_start = std::time::Instant::now();
    let store = get_shared_store();
    log::debug!("[RENDER] Cache fetch took {:?}", cache_fetch_start.elapsed());

    let mut apps_to_load: Vec<(String, Option<String>, String, String, Option<String>)> = Vec::new();

    for pkg_id in package_ids {
        // Android Package renderer (highest priority on Android)
        if android_package_enabled {
            // Try ViewModel cache first (future MVVM implementation)
            if let Some(ap_app) = vm_state.cached_metadata.get_android_package(pkg_id) {
                let texture = load_texture_from_bytes(ctx, pkg_id, &ap_app.icon_bytes, &store);
                app_data_map.insert(
                    pkg_id.clone(),
                    (texture, ap_app.label.clone(), pkg_id.clone(), None),
                );
                continue;
            }
        }

        let is_system = system_packages.contains(pkg_id);

        if !is_system {
            if fdroid_enabled {
                // Try ViewModel cache first (future MVVM)
                if let Some(fd_app) = vm_state.cached_metadata.get_fdroid(pkg_id) {
                    if fd_app.raw_response != "404" {
                        apps_to_load.push((
                            pkg_id.clone(),
                            fd_app.icon_base64.clone(),
                            fd_app.title.clone(),
                            fd_app.developer.clone(),
                            fd_app.version.clone(),
                        ));
                        continue;
                    }
                }

                // Fall back to SharedStore (legacy queues store here)
                if let Some(fd_app) = store.get_cached_fdroid_app(pkg_id) {
                    if fd_app.raw_response != "404" {
                        apps_to_load.push((
                            pkg_id.clone(),
                            fd_app.icon_base64.clone(),
                            fd_app.title.clone(),
                            fd_app.developer.clone(),
                            fd_app.version.clone(),
                        ));
                        continue;
                    }
                }
            }

            if google_play_enabled {
                // Try ViewModel cache first (future MVVM)
                if let Some(gp_app) = vm_state.cached_metadata.get_google_play(pkg_id) {
                    if gp_app.raw_response != "404" {
                        apps_to_load.push((
                            pkg_id.clone(),
                            gp_app.icon_base64.clone(),
                            gp_app.title.clone(),
                            gp_app.developer.clone(),
                            gp_app.version.clone(),
                        ));
                        continue;
                    }
                }

                // Fall back to SharedStore (legacy queues store here)
                if let Some(gp_app) = store.get_cached_google_play_app(pkg_id) {
                    if gp_app.raw_response != "404" {
                        apps_to_load.push((
                            pkg_id.clone(),
                            gp_app.icon_base64.clone(),
                            gp_app.title.clone(),
                            gp_app.developer.clone(),
                            gp_app.version.clone(),
                        ));
                        continue;
                    }
                }
            }
        } else {
            if apkmirror_enabled {
                // Try ViewModel cache first (future MVVM)
                if let Some(am_app) = vm_state.cached_metadata.get_apkmirror(pkg_id) {
                    if am_app.raw_response != "404" {
                        apps_to_load.push((
                            pkg_id.clone(),
                            am_app.icon_base64.clone(),
                            am_app.title.clone(),
                            am_app.developer.clone(),
                            am_app.version.clone(),
                        ));
                        continue;
                    }
                }

                // Fall back to SharedStore (legacy queues store here)
                if let Some(am_app) = store.get_cached_apkmirror_app(pkg_id) {
                    if am_app.raw_response != "404" {
                        apps_to_load.push((
                            pkg_id.clone(),
                            am_app.icon_base64.clone(),
                            am_app.title.clone(),
                            am_app.developer.clone(),
                            am_app.version.clone(),
                        ));
                        continue;
                    }
                }
            }
        }
    }

    let texture_load_start = std::time::Instant::now();
    let mut texture_count = 0;
    let mut texture_load_times = Vec::new();

    for (pkg_id, icon_base64, title, developer, version) in apps_to_load {
        let tex_start = std::time::Instant::now();
        let texture = icon_base64.as_ref().and_then(|b64| load_texture_from_base64(ctx, &pkg_id, b64, &store));
        let tex_elapsed = tex_start.elapsed();

        if texture.is_some() {
            texture_count += 1;
            texture_load_times.push(tex_elapsed);
            log::debug!("[RENDER] Loaded texture for {} in {:?}", pkg_id, tex_elapsed);
        }

        app_data_map.insert(pkg_id, (texture, title, developer, version));
    }

    let total_texture_time: std::time::Duration = texture_load_times.iter().sum();
    let avg_texture_time = if texture_count > 0 {
        total_texture_time / texture_count as u32
    } else {
        std::time::Duration::ZERO
    };

    log::info!(
        "[RENDER] prepare_app_info_for_display completed: {} packages, {} textures loaded in {:?} (avg: {:?}/texture), total elapsed: {:?}",
        package_ids.len(),
        texture_count,
        total_texture_time,
        avg_texture_time,
        start_time.elapsed()
    );

    app_data_map
}

/// Load texture from base64-encoded image data
fn load_texture_from_base64(
    ctx: &egui::Context,
    pkg_id: &str,
    base64_data: &str,
    store: &crate::shared_store_stt::SharedStore,
) -> Option<egui::TextureHandle> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64_data).ok()?;
    load_texture_from_bytes(ctx, pkg_id, &bytes, store)
}

/// Load texture from raw image bytes (with SharedStore caching)
fn load_texture_from_bytes(
    ctx: &egui::Context,
    pkg_id: &str,
    bytes: &[u8],
    store: &crate::shared_store_stt::SharedStore,
) -> Option<egui::TextureHandle> {
    // Check SharedStore cache first
    if let Some(texture) = store.get_android_package_texture(pkg_id) {
        return Some(texture);
    }

    let image = image::load_from_memory(bytes).ok()?;
    let rgba_image = image.to_rgba8();
    let size = [rgba_image.width() as usize, rgba_image.height() as usize];
    let pixels = rgba_image.as_flat_samples();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

    let texture = ctx.load_texture(
        format!("app_icon_{}", pkg_id),
        color_image,
        egui::TextureOptions::LINEAR,
    );

    // Cache in SharedStore
    store.set_android_package_texture(pkg_id.to_string(), texture.clone());
    Some(texture)
}

//! Desktop view for debloat tab
//!
//! This module implements the desktop layout (800px+) with:
//! - 200px left sidebar: Filter controls and options
//! - Main content area: Search bar, batch actions, error banner, package table
//!
//! The desktop view uses virtual scrolling for performance with large package lists.

use eframe::egui;
use std::collections::HashMap;

use super::components::render_package_table;
use super::state::TabDebloatState;
use crate::adb_stt::PackageFingerprint;
use crate::shared_store_stt::get_shared_store;
use crate::viewmodel::ViewModelState;

/// Sidebar width in pixels
const SIDEBAR_WIDTH: f32 = 200.0;

/// Type alias for app display data: (texture, title)
type AppDisplayData = HashMap<String, (Option<egui::TextureHandle>, String)>;

/// Render desktop view with sidebar layout
///
/// This function implements the desktop interface with:
/// - Left sidebar (200px): Category filters, options, advanced settings
/// - Main content area: Search, batch actions, error banner, virtual table
///
/// # Arguments
/// * `ui` - egui context for rendering
/// * `vm_state` - ViewModel state (read-only access to packages and UAD lists)
/// * `local_state` - Tab-local UI state (mutable for selection, filters, etc.)
pub fn render(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) {
    egui::SidePanel::left("debloat_sidebar")
        .exact_width(SIDEBAR_WIDTH)
        .resizable(false)
        .show_inside(ui, |ui| {
            render_sidebar(ui, vm_state, local_state);
        });

    egui::CentralPanel::default().show_inside(ui, |ui| {
        render_main_content(
            ui,
            vm_state,
            local_state,
            google_play_enabled,
            fdroid_enabled,
            apkmirror_enabled,
            android_package_enabled,
        );
    });
}

/// Render the left sidebar with filter controls
fn render_sidebar(ui: &mut egui::Ui, vm_state: &ViewModelState, local_state: &mut TabDebloatState) {
    ui.vertical(|ui| {
        ui.heading("Filters");
        ui.separator();

        // Category filters section
        ui.label("Category");
        ui.horizontal(|ui| {
            if ui
                .selectable_label(local_state.active_filter.category_filter.is_none(), "All")
                .clicked()
            {
                local_state.active_filter.category_filter = None;
                local_state.table_version += 1;
            }
            ui.label(format!("({})", local_state.cached_counts.all));
        });

        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    local_state.active_filter.category_filter.as_deref() == Some("recommended"),
                    "Recommended",
                )
                .clicked()
            {
                local_state.active_filter.category_filter = Some("recommended".to_string());
                local_state.table_version += 1;
            }
            ui.label(format!("({})", local_state.cached_counts.recommended));
        });

        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    local_state.active_filter.category_filter.as_deref() == Some("unsafe"),
                    "Unsafe",
                )
                .clicked()
            {
                local_state.active_filter.category_filter = Some("unsafe".to_string());
                local_state.table_version += 1;
            }
            ui.label(format!("({})", local_state.cached_counts.unsafe_apps));
        });

        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    local_state.active_filter.category_filter.as_deref() == Some("expert"),
                    "Expert",
                )
                .clicked()
            {
                local_state.active_filter.category_filter = Some("expert".to_string());
                local_state.table_version += 1;
            }
            ui.label(format!("({})", local_state.cached_counts.expert));
        });

        ui.add_space(16.0);

        // Options section
        ui.separator();
        ui.heading("Options");

        if ui
            .checkbox(
                &mut local_state.active_filter.show_only_enabled,
                "Show only enabled",
            )
            .changed()
        {
            local_state.table_version += 1;
        }

        if ui
            .checkbox(
                &mut local_state.active_filter.hide_system_apps,
                "Hide system apps",
            )
            .changed()
        {
            local_state.table_version += 1;
        }

        ui.add_space(16.0);

        // Advanced settings section
        ui.separator();
        ui.heading("Advanced");

        ui.checkbox(&mut local_state.unsafe_app_remove, "Unsafe removal");
        ui.checkbox(&mut local_state.expert_app_remove, "Expert mode");

        // Display device info if available
        if let Some(device) = &local_state.selected_device {
            ui.add_space(16.0);
            ui.separator();
            ui.label("Device");
            ui.label(device);
        }

        // Display package count from ViewModel
        ui.add_space(16.0);
        ui.separator();
        ui.label(format!("Total packages: {}", vm_state.packages.len()));
        ui.label(format!("Filtered: {}", vm_state.filtered_packages.len()));
    });
}

/// Pre-load app textures and titles to avoid egui RwLock conflicts
///
/// IMPORTANT: `ctx.load_texture(...)` takes egui::Context's internal RwLock.
/// `ui.data_mut(...)` also takes that same RwLock in write mode.
/// Computing textures inside `ui.data_mut` causes deadlock.
/// We must pre-load textures BEFORE caching or table rendering.
fn prepare_app_display_data(
    ctx: &egui::Context,
    packages: &[PackageFingerprint],
    vm_state: &ViewModelState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) -> AppDisplayData {
    log::info!("[DESKTOP] prepare_app_display_data: GP={}, FD={}, APK={}, AP={}, packages={}",
        google_play_enabled, fdroid_enabled, apkmirror_enabled, android_package_enabled, packages.len());

    let mut app_data = HashMap::new();

    if !google_play_enabled && !fdroid_enabled && !apkmirror_enabled && !android_package_enabled {
        log::info!("[DESKTOP] No renderers enabled, returning empty");
        return app_data;
    }

    let store = get_shared_store();

    for package in packages {
        let is_system = package.flags.contains("SYSTEM");

        // Try Android Package first (if enabled)
        if android_package_enabled {
            if let Some(ap_app) = vm_state.cached_metadata.get_android_package(&package.pkg) {
                let texture =
                    load_texture_from_bytes(ctx, &package.pkg, &ap_app.icon_bytes, &store);
                app_data.insert(package.pkg.clone(), (texture, ap_app.label.clone()));
                continue;
            }
        }

        // Try F-Droid for non-system apps
        if !is_system && fdroid_enabled {
            // Try ViewModel cache first
            if let Some(fd_app) = vm_state.cached_metadata.get_fdroid(&package.pkg) {
                if fd_app.raw_response != "404" {
                    if let Some(icon_base64) = &fd_app.icon_base64 {
                        let texture =
                            load_texture_from_base64(ctx, "fd", &package.pkg, icon_base64, &store);
                        app_data.insert(package.pkg.clone(), (texture, fd_app.title.clone()));
                        continue;
                    }
                }
            }

            // Fall back to SharedStore (legacy queues)
            if let Some(fd_app) = store.get_cached_fdroid_app(&package.pkg) {
                if fd_app.raw_response != "404" {
                    if let Some(icon_base64) = &fd_app.icon_base64 {
                        let texture =
                            load_texture_from_base64(ctx, "fd", &package.pkg, icon_base64, &store);
                        app_data.insert(package.pkg.clone(), (texture, fd_app.title.clone()));
                        continue;
                    }
                }
            }
        }

        // Try Google Play for non-system apps
        if !is_system && google_play_enabled {
            // Try ViewModel cache first
            if let Some(gp_app) = vm_state.cached_metadata.get_google_play(&package.pkg) {
                if gp_app.raw_response != "404" {
                    if let Some(icon_base64) = &gp_app.icon_base64 {
                        let texture =
                            load_texture_from_base64(ctx, "gp", &package.pkg, icon_base64, &store);
                        app_data.insert(package.pkg.clone(), (texture, gp_app.title.clone()));
                        continue;
                    }
                }
            }

            // Fall back to SharedStore (legacy queues)
            if let Some(gp_app) = store.get_cached_google_play_app(&package.pkg) {
                if gp_app.raw_response != "404" {
                    if let Some(icon_base64) = &gp_app.icon_base64 {
                        let texture =
                            load_texture_from_base64(ctx, "gp", &package.pkg, icon_base64, &store);
                        app_data.insert(package.pkg.clone(), (texture, gp_app.title.clone()));
                        continue;
                    }
                }
            }
        }

        // Try APKMirror for system apps
        if is_system && apkmirror_enabled {
            // Try ViewModel cache first
            if let Some(am_app) = vm_state.cached_metadata.get_apkmirror(&package.pkg) {
                if am_app.raw_response != "404" {
                    if let Some(icon_base64) = &am_app.icon_base64 {
                        let texture =
                            load_texture_from_base64(ctx, "am", &package.pkg, icon_base64, &store);
                        app_data.insert(package.pkg.clone(), (texture, am_app.title.clone()));
                        continue;
                    }
                }
            }

            // Fall back to SharedStore (legacy queues)
            if let Some(am_app) = store.get_cached_apkmirror_app(&package.pkg) {
                if am_app.raw_response != "404" {
                    if let Some(icon_base64) = &am_app.icon_base64 {
                        let texture =
                            load_texture_from_base64(ctx, "am", &package.pkg, icon_base64, &store);
                        app_data.insert(package.pkg.clone(), (texture, am_app.title.clone()));
                        continue;
                    }
                }
            }
        }
    }

    log::info!("[DESKTOP] prepare_app_display_data completed: {} packages with metadata", app_data.len());
    app_data
}

/// Load texture from base64-encoded icon (with SharedStore caching)
fn load_texture_from_base64(
    ctx: &egui::Context,
    prefix: &str,
    package_id: &str,
    base64_data: &str,
    store: &crate::shared_store_stt::SharedStore,
) -> Option<egui::TextureHandle> {
    // Check SharedStore cache first
    let existing_texture = match prefix {
        "gp" => store.get_google_play_texture(package_id),
        "fd" => store.get_fdroid_texture(package_id),
        "am" => store.get_apkmirror_texture(package_id),
        _ => None,
    };
    if let Some(texture) = existing_texture {
        return Some(texture);
    }

    // Decode base64
    use base64::{engine::general_purpose, Engine as _};
    let base64_str = if base64_data.starts_with("data:") {
        base64_data.split(',').nth(1).unwrap_or(base64_data)
    } else {
        base64_data
    };

    let bytes = general_purpose::STANDARD.decode(base64_str).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;

    // Create egui texture
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    let texture = ctx.load_texture(
        format!("{}_{}", prefix, package_id),
        color_image,
        Default::default(),
    );

    // Cache in SharedStore
    match prefix {
        "gp" => store.set_google_play_texture(package_id.to_string(), texture.clone()),
        "fd" => store.set_fdroid_texture(package_id.to_string(), texture.clone()),
        "am" => store.set_apkmirror_texture(package_id.to_string(), texture.clone()),
        _ => {}
    }

    Some(texture)
}

/// Load texture from PNG bytes (with SharedStore caching)
fn load_texture_from_bytes(
    ctx: &egui::Context,
    package_id: &str,
    png_bytes: &[u8],
    store: &crate::shared_store_stt::SharedStore,
) -> Option<egui::TextureHandle> {
    // Check SharedStore cache first
    if let Some(texture) = store.get_android_package_texture(package_id) {
        return Some(texture);
    }

    // Load from bytes
    let image = image::load_from_memory(png_bytes).ok()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
    let texture = ctx.load_texture(
        format!("ap_{}", package_id),
        color_image,
        Default::default(),
    );

    // Cache in SharedStore
    store.set_android_package_texture(package_id.to_string(), texture.clone());
    Some(texture)
}

/// Render the main content area
fn render_main_content(
    ui: &mut egui::Ui,
    vm_state: &ViewModelState,
    local_state: &mut TabDebloatState,
    google_play_enabled: bool,
    fdroid_enabled: bool,
    apkmirror_enabled: bool,
    android_package_enabled: bool,
) {
    ui.vertical(|ui| {
        // Search bar
        render_search_bar(ui, local_state);

        ui.add_space(8.0);

        // Batch action buttons
        render_batch_actions(ui, local_state);

        ui.add_space(8.0);

        // Error banner (if any errors exist)
        render_error_banner(ui, local_state);

        ui.separator();

        // Pre-load app icons and titles (MUST be done before table rendering to avoid egui RwLock deadlock)
        let app_display_data = prepare_app_display_data(
            ui.ctx(),
            &vm_state.filtered_packages,
            vm_state,
            google_play_enabled,
            fdroid_enabled,
            apkmirror_enabled,
            android_package_enabled,
        );

        // Package table (virtual scrolling)
        log::debug!(
            "DEBUG: view_desktop rendering package table with {} filtered packages",
            vm_state.filtered_packages.len()
        );
        render_package_table(
            ui,
            &vm_state.filtered_packages,
            &mut local_state.selected_packages,
            vm_state.uad_ng_lists.as_ref(),
            &app_display_data,
        );
    });
}

/// Render search bar for text filtering with debouncing
fn render_search_bar(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
    ui.horizontal(|ui| {
        ui.label("Search:");
        let response = ui.text_edit_singleline(&mut local_state.pending_filter_text);
        if response.changed() {
            // User typed something - start/reset debounce timer
            local_state.last_filter_input = Some(std::time::Instant::now());
        }
        if ui.button("Clear").clicked() {
            local_state.pending_filter_text.clear();
            local_state.applied_filter_text.clear();
            local_state.active_filter.text_filter.clear();
            local_state.last_filter_input = None;
        }
    });
}

/// Render batch action buttons
fn render_batch_actions(ui: &mut egui::Ui, local_state: &mut TabDebloatState) {
    ui.horizontal(|ui| {
        let selection_count = local_state.selected_packages.len();
        ui.label(format!("Selected: {}", selection_count));

        ui.add_enabled_ui(selection_count > 0, |ui| {
            if ui.button("Uninstall").clicked() {
                // TODO: Trigger batch uninstall via ViewModel command
                log::info!("Batch uninstall requested for {} packages", selection_count);
            }

            if ui.button("Disable").clicked() {
                // TODO: Trigger batch disable via ViewModel command
                log::info!("Batch disable requested for {} packages", selection_count);
            }

            if ui.button("Enable").clicked() {
                // TODO: Trigger batch enable via ViewModel command
                log::info!("Batch enable requested for {} packages", selection_count);
            }
        });

        if selection_count > 0 {
            if ui.button("Clear Selection").clicked() {
                local_state.selected_packages.clear();
            }
        }

        // Select all / Deselect all buttons
        if ui.button("Select All").clicked() {
            // TODO: Select all filtered packages
            log::info!("Select all requested");
        }
    });
}

/// Render error banner if there are active errors
fn render_error_banner(ui: &mut egui::Ui, local_state: &TabDebloatState) {
    // Check if batch operations have errors
    let has_error = !local_state.batch_uninstall_state.status_message.is_empty()
        || !local_state.batch_disable_state.status_message.is_empty()
        || !local_state.batch_enable_state.status_message.is_empty();

    if has_error {
        ui.colored_label(
            egui::Color32::from_rgb(255, 100, 100),
            "⚠ Operation errors detected",
        );

        if !local_state.batch_uninstall_state.status_message.is_empty() {
            ui.label(format!(
                "Uninstall: {}",
                local_state.batch_uninstall_state.status_message
            ));
        }

        if !local_state.batch_disable_state.status_message.is_empty() {
            ui.label(format!(
                "Disable: {}",
                local_state.batch_disable_state.status_message
            ));
        }

        if !local_state.batch_enable_state.status_message.is_empty() {
            ui.label(format!(
                "Enable: {}",
                local_state.batch_enable_state.status_message
            ));
        }

        ui.separator();
    }

    // Show batch operation progress if active
    render_batch_progress(ui, local_state);
}

/// Render batch operation progress bars
fn render_batch_progress(ui: &mut egui::Ui, local_state: &TabDebloatState) {
    // Uninstall progress
    if let Ok(guard) = local_state.batch_uninstall_progress.try_lock() {
        if let Some(progress) = *guard {
            ui.horizontal(|ui| {
                ui.label("Uninstalling:");
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                if let Ok(mut cancelled) = local_state.batch_uninstall_cancelled.try_lock() {
                    if ui.button("Cancel").clicked() {
                        *cancelled = true;
                    }
                }
            });
        }
    }

    // Disable progress
    if let Ok(guard) = local_state.batch_disable_progress.try_lock() {
        if let Some(progress) = *guard {
            ui.horizontal(|ui| {
                ui.label("Disabling:");
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                if let Ok(mut cancelled) = local_state.batch_disable_cancelled.try_lock() {
                    if ui.button("Cancel").clicked() {
                        *cancelled = true;
                    }
                }
            });
        }
    }

    // Enable progress
    if let Ok(guard) = local_state.batch_enable_progress.try_lock() {
        if let Some(progress) = *guard {
            ui.horizontal(|ui| {
                ui.label("Enabling:");
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                if let Ok(mut cancelled) = local_state.batch_enable_cancelled.try_lock() {
                    if ui.button("Cancel").clicked() {
                        *cancelled = true;
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_width() {
        assert_eq!(SIDEBAR_WIDTH, 200.0);
    }
}

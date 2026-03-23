use crate::adb::PackageFingerprint;
use crate::shared_store_stt::get_shared_store;
use crate::uad_shizuku_app::UadNgLists;
use crate::material_symbol_icons::{ICON_INFO, ICON_DELETE, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use eframe::egui;
use egui_material3::{icon_button_standard, DataTableCell, theme::get_global_color};
use egui_i18n::tr;

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
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
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

    // If no subtitle, use package ID
    if subtitle_text.is_empty() {
        subtitle_text = pkg_id.to_string();
    }

    // Determine if we need scrollable title (for GooglePlay and APKMirror sources)
    let use_scrollable_title = texture.is_some() && (
        store.get_cached_google_play_app(pkg_id).is_some() ||
        store.get_cached_apkmirror_app(pkg_id).is_some()
    );

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
                            ui.add(egui::Label::new(egui::RichText::new(&title_text).strong().color(on_surface)).wrap_mode(egui::TextWrapMode::Extend));
                        });
                } else {
                    ui.label(egui::RichText::new(&title_text).strong().color(on_surface));
                }

                // Subtitle (package ID or developer)
                if !subtitle_text.is_empty() {
                    ui.label(egui::RichText::new(&subtitle_text).small().color(egui::Color32::GRAY));
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
            if ui.add(icon_button_standard(ICON_INFO.to_string()))
                .on_hover_text(tr!("package-info")).clicked() {
                // Store clicked package for opening details dialog
                ui.data_mut(|data| {
                    data.insert_temp(egui::Id::new("info_clicked_package"), pkg_id_clone.clone());
                });
            }

            // Enable/disable toggle
            let pkg_enabled = enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED");

            if pkg_enabled {
                if ui.add(icon_button_standard(ICON_TOGGLE_ON.to_string()))
                    .on_hover_text(tr!("disable")).clicked() {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("disable_clicked_package"), pkg_id_clone.clone());
                    });
                }
            } else {
                if ui.add(icon_button_standard(ICON_TOGGLE_OFF.to_string()))
                    .on_hover_text(tr!("enable")).clicked() {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("enable_clicked_package"), pkg_id_clone.clone());
                    });
                }
            }

            // Uninstall button (only for enabled apps)
            if pkg_enabled {
                if ui.add(icon_button_standard(ICON_DELETE.to_string())
                    .icon_color(egui::Color32::from_rgb(211, 47, 47)))
                    .on_hover_text(tr!("uninstall")).clicked() {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("uninstall_clicked_package"), pkg_id_clone.clone());
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

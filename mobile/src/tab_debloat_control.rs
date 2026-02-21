use crate::adb::PackageFingerprint;
use crate::models::{ApkMirrorApp, FDroidApp, GooglePlayApp};
use crate::shared_store_stt::get_shared_store;
use crate::uad_shizuku_app::UadNgLists;
pub use crate::tab_debloat_control_stt::*;
use crate::dlg_package_details::DlgPackageDetails;
use crate::dlg_uninstall_confirm::DlgUninstallConfirm;
use eframe::egui;
use egui_i18n::tr;
use egui_material3::{data_table, icon_button_standard, theme::get_global_color, MaterialButton};

use crate::material_symbol_icons::{ICON_INFO, ICON_DELETE, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::{DESKTOP_MIN_WIDTH, BASE_TABLE_WIDTH};

impl Default for TabDebloatControl {
    fn default() -> Self {
        Self {
            open: false,
            // NOTE: installed_packages, uad_ng_lists, textures, and cached apps are now in shared_store_stt::SharedStore
            selected_packages: std::collections::HashSet::new(),
            package_details_dialog: DlgPackageDetails::new(),
            active_filter: DebloatFilter::All,
            sort_column: None,
            sort_ascending: true,
            selected_device: None,
            table_version: 0,
            show_only_enabled: false,
            hide_system_app: false,
            cached_counts: CachedCategoryCounts::default(),
            text_filter: String::new(),
            unsafe_app_remove: false,
            uninstall_confirm_dialog: DlgUninstallConfirm::default(),
        }
    }
}

impl TabDebloatControl {
    fn enabled_to_display_string(enabled: i32, installed: bool, is_system: bool) -> &'static str {
        match enabled {
            0 => {
                if !installed && is_system {
                    "REMOVED_USER"
                } else {
                    "DEFAULT"
                }
            }
            1 => "ENABLED",
            2 => "DISABLED",
            3 => "DISABLED_USER",
            _ => "UNKNOWN",
        }
    }

    fn install_reason_to_string(install_reason: i32) -> &'static str {
        match install_reason {
            0 => "UNKNOWN",
            1 => "POLICY",
            2 => "DEVICE_RESTORE",
            3 => "DEVICE_SETUP",
            4 => "USER_REQUESTED",
            _ => "UNKNOWN",
        }
    }

    pub fn update_packages(&mut self, packages: Vec<PackageFingerprint>) {
        let store = get_shared_store();
        let package_names: std::collections::HashSet<String> =
            packages.iter().map(|p| p.pkg.clone()).collect();
        self.selected_packages
            .retain(|pkg| package_names.contains(pkg));

        store.set_installed_packages(packages);
        self.table_version = self.table_version.wrapping_add(1);
        self.sort_column = None;
        self.sort_ascending = true;
    }

    pub fn set_selected_device(&mut self, device: Option<String>) {
        self.selected_device = device;
    }

    pub fn update_uad_ng_lists(&mut self, lists: UadNgLists) {
        let store = get_shared_store();
        store.set_uad_ng_lists(Some(lists));
    }

    /// Update cached app info from fetched results
    pub fn update_cached_google_play(&mut self, pkg_id: String, app: GooglePlayApp) {
        let store = get_shared_store();
        store.set_cached_google_play_app(pkg_id, app);
    }

    pub fn update_cached_fdroid(&mut self, pkg_id: String, app: FDroidApp) {
        let store = get_shared_store();
        store.set_cached_fdroid_app(pkg_id, app);
    }

    pub fn update_cached_apkmirror(&mut self, pkg_id: String, app: ApkMirrorApp) {
        let store = get_shared_store();
        store.set_cached_apkmirror_app(pkg_id, app);
    }

    /// Update cached category counts if version has changed
    fn update_cached_counts(&mut self, installed_packages: &[PackageFingerprint], uad_ng_lists: Option<&UadNgLists>) {
        if self.cached_counts.version == self.table_version {
            return; // Cache is still valid
        }

        if let Some(lists) = uad_ng_lists {
            // Compute all counts in a single pass
            let mut recommended = (0usize, 0usize);
            let mut advanced = (0usize, 0usize);
            let mut expert = (0usize, 0usize);
            let mut unsafe_count = (0usize, 0usize);
            let mut unknown = (0usize, 0usize);

            for package in installed_packages {
                let is_enabled = self.is_package_enabled(package);

                if let Some(app_entry) = lists.apps.get(&package.pkg) {
                    match app_entry.removal.as_str() {
                        "Recommended" => {
                            recommended.1 += 1;
                            if is_enabled { recommended.0 += 1; }
                        }
                        "Advanced" => {
                            advanced.1 += 1;
                            if is_enabled { advanced.0 += 1; }
                        }
                        "Expert" => {
                            expert.1 += 1;
                            if is_enabled { expert.0 += 1; }
                        }
                        "Unsafe" => {
                            unsafe_count.1 += 1;
                            if is_enabled { unsafe_count.0 += 1; }
                        }
                        _ => {}
                    }
                } else {
                    // Unknown category
                    unknown.1 += 1;
                    if is_enabled { unknown.0 += 1; }
                }
            }

            self.cached_counts.recommended = recommended;
            self.cached_counts.advanced = advanced;
            self.cached_counts.expert = expert;
            self.cached_counts.unsafe_count = unsafe_count;
            self.cached_counts.unknown = unknown;
        } else {
            // No UAD lists, reset counts
            self.cached_counts.recommended = (0, 0);
            self.cached_counts.advanced = (0, 0);
            self.cached_counts.expert = (0, 0);
            self.cached_counts.unsafe_count = (0, 0);
            self.cached_counts.unknown = (0, 0);
        }

        self.cached_counts.version = self.table_version;
    }

    fn get_recommended_count(&self) -> (usize, usize) {
        self.cached_counts.recommended
    }

    fn get_advanced_count(&self) -> (usize, usize) {
        self.cached_counts.advanced
    }

    fn get_expert_count(&self) -> (usize, usize) {
        self.cached_counts.expert
    }

    fn get_unsafe_count(&self) -> (usize, usize) {
        self.cached_counts.unsafe_count
    }

    fn get_unknown_count(&self) -> (usize, usize) {
        self.cached_counts.unknown
    }

    fn is_package_enabled(&self, package: &PackageFingerprint) -> bool {
        let is_system = package.flags.contains("SYSTEM");
        package
            .users
            .first()
            .map(|u| {
                let display_str = Self::enabled_to_display_string(u.enabled, u.installed, is_system);
                display_str == "ENABLED" || display_str == "DEFAULT" || display_str == "UNKNOWN"
            })
            .unwrap_or(false)
    }

    fn should_show_package(&self, package: &PackageFingerprint, uad_ng_lists: Option<&UadNgLists>) -> bool {
        if self.hide_system_app && package.flags.contains("SYSTEM") {
            return false;
        }

        if self.show_only_enabled {
            let is_system = package.flags.contains("SYSTEM");
            let should_show = package
                .users
                .first()
                .map(|user| {
                    let enabled = user.enabled;
                    let installed = user.installed;
                    let is_removed_user = enabled == 0 && !installed && is_system;
                    let is_disabled = enabled == 2;
                    let is_disabled_user = enabled == 3;
                    !(is_removed_user || is_disabled || is_disabled_user)
                })
                .unwrap_or(false);

            if !should_show {
                return false;
            }
        }

        match &self.active_filter {
            DebloatFilter::All => true,
            DebloatFilter::Recommended => Self::matches_category_static(package, "Recommended", uad_ng_lists),
            DebloatFilter::Advanced => Self::matches_category_static(package, "Advanced", uad_ng_lists),
            DebloatFilter::Expert => Self::matches_category_static(package, "Expert", uad_ng_lists),
            DebloatFilter::Unsafe => Self::matches_category_static(package, "Unsafe", uad_ng_lists),
            DebloatFilter::Unknown => {
                if let Some(lists) = uad_ng_lists {
                    lists.apps.get(&package.pkg).is_none()
                } else {
                    true
                }
            }
        }
    }

    fn matches_category_static(package: &PackageFingerprint, category: &str, uad_ng_lists: Option<&UadNgLists>) -> bool {
        if let Some(lists) = uad_ng_lists {
            lists
                .apps
                .get(&package.pkg)
                .map(|app| app.removal == category)
                .unwrap_or(false)
        } else {
            false
        }
    }

    fn matches_text_filter(
        &self,
        package: &PackageFingerprint,
        uad_ng_lists: Option<&UadNgLists>,
        cached_fdroid_apps: &std::collections::HashMap<String, FDroidApp>,
        cached_google_play_apps: &std::collections::HashMap<String, GooglePlayApp>,
        cached_apkmirror_apps: &std::collections::HashMap<String, ApkMirrorApp>,
    ) -> bool {
        if self.text_filter.is_empty() {
            return true;
        }

        let filter_lower = self.text_filter.to_lowercase();
        let is_system = package.flags.contains("SYSTEM");

        // Check package name and version
        let package_name = format!("{} ({})", package.pkg, package.versionName).to_lowercase();
        if package_name.contains(&filter_lower) {
            return true;
        }

        // Check debloat category
        let debloat_category = if let Some(lists) = uad_ng_lists {
            lists
                .apps
                .get(&package.pkg)
                .map(|app| app.removal.clone())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        };
        if debloat_category.to_lowercase().contains(&filter_lower) {
            return true;
        }

        // Check enabled status
        let enabled = package
            .users
            .first()
            .map(|u| Self::enabled_to_display_string(u.enabled, u.installed, is_system))
            .unwrap_or("DEFAULT");
        if enabled.to_lowercase().contains(&filter_lower) {
            return true;
        }

        // Check install reason
        let install_reason_value = package.users.first().map(|u| u.installReason).unwrap_or(0);
        let install_reason = if is_system {
            if install_reason_value == 0 {
                "SYSTEM".to_string()
            } else {
                format!("{} (SYSTEM)", Self::install_reason_to_string(install_reason_value))
            }
        } else {
            Self::install_reason_to_string(install_reason_value).to_string()
        };
        if install_reason.to_lowercase().contains(&filter_lower) {
            return true;
        }

        // Check cached app info (title and developer)
        if let Some(fd_app) = cached_fdroid_apps.get(&package.pkg) {
            if fd_app.raw_response != "404" {
                if fd_app.title.to_lowercase().contains(&filter_lower) {
                    return true;
                }
                if fd_app.developer.to_lowercase().contains(&filter_lower) {
                    return true;
                }
            }
        }

        if let Some(gp_app) = cached_google_play_apps.get(&package.pkg) {
            if gp_app.raw_response != "404" {
                if gp_app.title.to_lowercase().contains(&filter_lower) {
                    return true;
                }
                if gp_app.developer.to_lowercase().contains(&filter_lower) {
                    return true;
                }
            }
        }

        if let Some(am_app) = cached_apkmirror_apps.get(&package.pkg) {
            if am_app.raw_response != "404" {
                if am_app.title.to_lowercase().contains(&filter_lower) {
                    return true;
                }
                if am_app.developer.to_lowercase().contains(&filter_lower) {
                    return true;
                }
            }
        }

        false
    }

    fn sort_packages(&mut self) {
        if let Some(col_idx) = self.sort_column {
            let store = get_shared_store();
            let uad_ng_lists = store.get_uad_ng_lists();
            let mut installed_packages = store.get_installed_packages();

            installed_packages.sort_by(|a, b| {
                let ordering = match col_idx {
                    0 => {
                        let name_a = format!("{} ({})", a.pkg, a.versionName);
                        let name_b = format!("{} ({})", b.pkg, b.versionName);
                        name_a.cmp(&name_b)
                    }
                    1 => {
                        let cat_a = if let Some(ref lists) = uad_ng_lists {
                            lists
                                .apps
                                .get(&a.pkg)
                                .map(|app| app.removal.clone())
                                .unwrap_or_else(|| "Unknown".to_string())
                        } else {
                            "Unknown".to_string()
                        };
                        let cat_b = if let Some(ref lists) = uad_ng_lists {
                            lists
                                .apps
                                .get(&b.pkg)
                                .map(|app| app.removal.clone())
                                .unwrap_or_else(|| "Unknown".to_string())
                        } else {
                            "Unknown".to_string()
                        };
                        cat_a.cmp(&cat_b)
                    }
                    2 => {
                        let perms_a = a.users.first().map(|u| u.runtimePermissions.len()).unwrap_or(0);
                        let perms_b = b.users.first().map(|u| u.runtimePermissions.len()).unwrap_or(0);
                        perms_a.cmp(&perms_b)
                    }
                    3 => {
                        let is_system_a = a.flags.contains("SYSTEM");
                        let is_system_b = b.flags.contains("SYSTEM");
                        let enabled_a = a
                            .users
                            .first()
                            .map(|u| Self::enabled_to_display_string(u.enabled, u.installed, is_system_a))
                            .unwrap_or("DEFAULT");
                        let enabled_b = b
                            .users
                            .first()
                            .map(|u| Self::enabled_to_display_string(u.enabled, u.installed, is_system_b))
                            .unwrap_or("DEFAULT");
                        enabled_a.cmp(enabled_b)
                    }
                    4 => {
                        let is_system_a = a.flags.contains("SYSTEM");
                        let is_system_b = b.flags.contains("SYSTEM");
                        let reason_a = a.users.first().map(|u| u.installReason).unwrap_or(0);
                        let reason_b = b.users.first().map(|u| u.installReason).unwrap_or(0);

                        let reason_str_a = if is_system_a {
                            if reason_a == 0 {
                                "SYSTEM".to_string()
                            } else {
                                format!("{} (SYSTEM)", Self::install_reason_to_string(reason_a))
                            }
                        } else {
                            Self::install_reason_to_string(reason_a).to_string()
                        };

                        let reason_str_b = if is_system_b {
                            if reason_b == 0 {
                                "SYSTEM".to_string()
                            } else {
                                format!("{} (SYSTEM)", Self::install_reason_to_string(reason_b))
                            }
                        } else {
                            Self::install_reason_to_string(reason_b).to_string()
                        };

                        reason_str_a.cmp(&reason_str_b)
                    }
                    _ => std::cmp::Ordering::Equal,
                };

                if self.sort_ascending {
                    ordering
                } else {
                    ordering.reverse()
                }
            });

            // Save sorted packages back to shared store
            store.set_installed_packages(installed_packages);
        }
    }

    fn load_texture_from_base64(
        ctx: &egui::Context,
        prefix: &str,
        package_id: &str,
        base64_data: &str,
    ) -> Option<egui::TextureHandle> {
        let store = get_shared_store();

        // Check shared store for existing texture
        let existing_texture = match prefix {
            "gp" => store.get_google_play_texture(package_id),
            "fd" => store.get_fdroid_texture(package_id),
            "am" => store.get_apkmirror_texture(package_id),
            _ => None,
        };
        if let Some(texture) = existing_texture {
            return Some(texture);
        }

        use base64::{engine::general_purpose, Engine as _};

        let base64_str = if base64_data.starts_with("data:") {
            base64_data.split(',').nth(1).unwrap_or(base64_data)
        } else {
            base64_data
        };

        let bytes = match general_purpose::STANDARD.decode(base64_str) {
            Ok(b) => b,
            Err(e) => {
                log::warn!("Failed to decode base64 image for {}: {}", package_id, e);
                return None;
            }
        };

        let image = match image::load_from_memory(&bytes) {
            Ok(img) => img,
            Err(e) => {
                log::warn!("Failed to load image for {}: {}", package_id, e);
                return None;
            }
        };

        let size = [image.width() as _, image.height() as _];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        let texture = ctx.load_texture(
            format!("{}_{}", prefix, package_id),
            color_image,
            Default::default(),
        );

        // Store texture in shared store
        match prefix {
            "gp" => store.set_google_play_texture(package_id.to_string(), texture.clone()),
            "fd" => store.set_fdroid_texture(package_id.to_string(), texture.clone()),
            "am" => store.set_apkmirror_texture(package_id.to_string(), texture.clone()),
            _ => {}
        }
        Some(texture)
    }

    fn load_texture_from_bytes(
        ctx: &egui::Context,
        package_id: &str,
        png_bytes: &[u8],
    ) -> Option<egui::TextureHandle> {
        let store = get_shared_store();

        if let Some(texture) = store.get_android_package_texture(package_id) {
            return Some(texture);
        }

        let image = match image::load_from_memory(png_bytes) {
            Ok(img) => img,
            Err(e) => {
                log::warn!("Failed to load image for {}: {}", package_id, e);
                return None;
            }
        };

        let size = [image.width() as _, image.height() as _];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
        let texture = ctx.load_texture(
            format!("ap_{}", package_id),
            color_image,
            Default::default(),
        );

        store.set_android_package_texture(package_id.to_string(), texture.clone());
        Some(texture)
    }


    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        google_play_enabled: bool,
        fdroid_enabled: bool,
        apkmirror_enabled: bool,
        android_package_enabled: bool,
    ) -> Option<AdbResult> {

        // Get viewport width for responsive design
        let available_width = ui.ctx().content_rect().width();
        let is_desktop = available_width >= DESKTOP_MIN_WIDTH;
        
        let mut result = None;
        let store = get_shared_store();

        // Pre-fetch data once at the start to avoid repeated clones
        let installed_packages = store.get_installed_packages();
        let uad_ng_lists = store.get_uad_ng_lists();
        let uad_ng_lists_ref = uad_ng_lists.as_ref();

        // Pre-fetch cached app data maps for efficient lookups
        let cached_fdroid_apps = store.get_cached_fdroid_apps();
        let cached_google_play_apps = store.get_cached_google_play_apps();
        let cached_apkmirror_apps = store.get_cached_apkmirror_apps();
        let cached_android_package_apps = if android_package_enabled {
            store.get_cached_android_package_apps()
        } else {
            std::collections::HashMap::new()
        };

        // Update cached counts if needed (only recomputes when table_version changes)
        self.update_cached_counts(&installed_packages, uad_ng_lists_ref);

        // Check if mobile view for filter button style
        let filter_is_mobile = ui.available_width() < DESKTOP_MIN_WIDTH;

        // Filter Buttons
        if !installed_packages.is_empty() {
            ui.horizontal_wrapped(|ui| { 
                let all_total = installed_packages.len();
                let all_enabled = installed_packages.iter().filter(|p| self.is_package_enabled(p)).count();
                let all_text = tr!("all", { enabled: all_enabled, total: all_total });

                let (rec_enabled, rec_total) = self.get_recommended_count();
                let rec_text = tr!("recommended", { enabled: rec_enabled, total: rec_total });

                let (adv_enabled, adv_total) = self.get_advanced_count();
                let adv_text = tr!("advanced", { enabled: adv_enabled, total: adv_total });

                let (exp_enabled, exp_total) = self.get_expert_count();
                let exp_text = tr!("expert", { enabled: exp_enabled, total: exp_total });

                let (unsafe_enabled, unsafe_total) = self.get_unsafe_count();
                let unsafe_text = tr!("unsafe", { enabled: unsafe_enabled, total: unsafe_total });

                let (unknown_enabled, unknown_total) = self.get_unknown_count();
                let unknown_text = tr!("unknown", { enabled: unknown_enabled, total: unknown_total });

                if filter_is_mobile {
                    // Mobile: use small MaterialButton with custom colors (same as desktop)
                    let show_all_colors = self.active_filter == DebloatFilter::All;

                    let button = if self.active_filter == DebloatFilter::All {
                        MaterialButton::filled(&all_text).small().fill(egui::Color32::from_rgb(158, 158, 158))
                    } else {
                        MaterialButton::outlined(&all_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::All;
                    }

                    let button = if self.active_filter == DebloatFilter::Recommended || show_all_colors {
                        MaterialButton::filled(&rec_text).small().fill(egui::Color32::from_rgb(56, 142, 60))
                    } else {
                        MaterialButton::outlined(&rec_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Recommended;
                    }

                    let button = if self.active_filter == DebloatFilter::Advanced || show_all_colors {
                        MaterialButton::filled(&adv_text).small().fill(egui::Color32::from_rgb(33, 150, 243))
                    } else {
                        MaterialButton::outlined(&adv_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Advanced;
                    }

                    let button = if self.active_filter == DebloatFilter::Expert || show_all_colors {
                        MaterialButton::filled(&exp_text).small().fill(egui::Color32::from_rgb(255, 152, 0))
                    } else {
                        MaterialButton::outlined(&exp_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Expert;
                    }

                    let button = if self.active_filter == DebloatFilter::Unsafe || show_all_colors {
                        MaterialButton::filled(&unsafe_text).small().fill(egui::Color32::from_rgb(255, 235, 59))
                    } else {
                        MaterialButton::outlined(&unsafe_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Unsafe;
                    }

                    let button = if self.active_filter == DebloatFilter::Unknown || show_all_colors {
                        MaterialButton::filled(&unknown_text).small().fill(egui::Color32::from_rgb(255, 255, 255))
                    } else {
                        MaterialButton::outlined(&unknown_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Unknown;
                    }
                } else {
                    // Desktop: use small MaterialButton with custom colors
                    let show_all_colors = self.active_filter == DebloatFilter::All;

                    let button = if self.active_filter == DebloatFilter::All {
                        MaterialButton::filled(&all_text).small().fill(egui::Color32::from_rgb(158, 158, 158))
                    } else {
                        MaterialButton::outlined(&all_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::All;
                    }

                    let button = if self.active_filter == DebloatFilter::Recommended || show_all_colors {
                        MaterialButton::filled(&rec_text).small().fill(egui::Color32::from_rgb(56, 142, 60))
                    } else {
                        MaterialButton::outlined(&rec_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Recommended;
                    }

                    let button = if self.active_filter == DebloatFilter::Advanced || show_all_colors {
                        MaterialButton::filled(&adv_text).small().fill(egui::Color32::from_rgb(33, 150, 243))
                    } else {
                        MaterialButton::outlined(&adv_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Advanced;
                    }

                    let button = if self.active_filter == DebloatFilter::Expert || show_all_colors {
                        MaterialButton::filled(&exp_text).small().fill(egui::Color32::from_rgb(255, 152, 0))
                    } else {
                        MaterialButton::outlined(&exp_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Expert;
                    }

                    let button = if self.active_filter == DebloatFilter::Unsafe || show_all_colors {
                        MaterialButton::filled(&unsafe_text).small().fill(egui::Color32::from_rgb(255, 235, 59))
                    } else {
                        MaterialButton::outlined(&unsafe_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Unsafe;
                    }

                    let button = if self.active_filter == DebloatFilter::Unknown || show_all_colors {
                        MaterialButton::filled(&unknown_text).small().fill(egui::Color32::from_rgb(255, 255, 255))
                    } else {
                        MaterialButton::outlined(&unknown_text).small()
                    };
                    if ui.add(button).clicked() {
                        self.active_filter = DebloatFilter::Unknown;
                    }
                }
            }); 
        }

        if installed_packages.is_empty() {
            ui.label(tr!("no-packages-loaded"));
            return None;
        }

        // Batch action buttons
        ui.horizontal(|ui| { 
            let selected_count = self.selected_packages.len();

            if filter_is_mobile {
                // Mobile: use compact plain buttons
                if ui.button(tr!("deselect-all")).clicked() {
                    self.selected_packages.clear();
                }

                ui.separator();

                ui.label(tr!("selected-count", { count: selected_count }));

                if selected_count > 0 {
                    if ui.button(&tr!("uninstall-selected", { count: selected_count })).clicked() {
                        ui.data_mut(|data| {
                            data.insert_temp(egui::Id::new("batch_uninstall_clicked"), true);
                        });
                    }

                    if ui.button(&tr!("disable-selected", { count: selected_count })).clicked() {
                        ui.data_mut(|data| {
                            data.insert_temp(egui::Id::new("batch_disable_clicked"), true);
                        });
                    }

                    if ui.button(&tr!("enable-selected", { count: selected_count })).clicked() {
                        ui.data_mut(|data| {
                            data.insert_temp(egui::Id::new("batch_enable_clicked"), true);
                        });
                    }
                }
            } else {
                // Desktop: use small MaterialButton
                if ui.add(MaterialButton::outlined(tr!("deselect-all")).small()).clicked() {
                    self.selected_packages.clear();
                }

                ui.separator();

                ui.label(tr!("selected-count", { count: selected_count }));

                if selected_count > 0 {
                    if ui
                        .add(MaterialButton::filled(&tr!("uninstall-selected", { count: selected_count })).small())
                        .clicked()
                    {
                        ui.data_mut(|data| {
                            data.insert_temp(egui::Id::new("batch_uninstall_clicked"), true);
                        });
                    }

                    if ui
                        .add(MaterialButton::filled(&tr!("disable-selected", { count: selected_count })).small())
                        .clicked()
                    {
                        ui.data_mut(|data| {
                            data.insert_temp(egui::Id::new("batch_disable_clicked"), true);
                        });
                    }

                    if ui
                        .add(MaterialButton::filled(&tr!("enable-selected", { count: selected_count })).small())
                        .clicked()
                    {
                        ui.data_mut(|data| {
                            data.insert_temp(egui::Id::new("batch_enable_clicked"), true);
                        });
                    }
                }
            }
        }); 
        ui.add_space(10.0);

        // Show only enabled toggle
        ui.horizontal_wrapped(|ui| { 
            ui.label(tr!("show-only-enabled"));
            toggle_ui(ui, &mut self.show_only_enabled);
            ui.add_space(10.0);
            ui.label(tr!("hide-system-app"));
            toggle_ui(ui, &mut self.hide_system_app);
            ui.add_space(10.0);
            ui.label(tr!("filter"));
            let response = ui.add(egui::TextEdit::singleline(&mut self.text_filter)
                .hint_text(tr!("filter-hint"))
                .desired_width(200.0));
            #[cfg(target_os = "android")]
            {
                if response.gained_focus() {
                    let _ = crate::android_inputmethod::show_soft_input();
                }
                if response.lost_focus() {
                    let _ = crate::android_inputmethod::hide_soft_input();
                }
            }
            crate::clipboard_popup::show_clipboard_popup(ui, &response, &mut self.text_filter);
            if !self.text_filter.is_empty() && ui.button("X").clicked() {
                self.text_filter.clear();
            }

            if is_desktop {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("* RP: {}", tr!("col-runtime-permissions")));
                });
            }
        }); 

        ui.horizontal(|ui| { 
            // Sort buttons for hidden columns in mobile view
            if !filter_is_mobile {
                return;
            }
            
            ui.label(tr!("sort-by"));
            
            // Debloat Category sort button
            let category_selected = self.sort_column == Some(1);
            let category_label = if category_selected {
                format!("{} {}", tr!("col-debloat-category"), if self.sort_ascending { "▲" } else { "▼" })
            } else {
                format!("{} {}", tr!("col-debloat-category"), "▲") // Default ascending
            };
            if ui.selectable_label(category_selected, category_label).clicked() {
                if self.sort_column == Some(1) {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(1);
                    self.sort_ascending = true;
                }
                self.sort_packages();
            }
            
            // Runtime Permissions sort button
            let rp_selected = self.sort_column == Some(2);
            let rp_label = if rp_selected {
                format!("RP {}", if self.sort_ascending { "▲" } else { "▼" })
            } else {
                "RP ▼".to_string() // Default descending
            };
            if ui.selectable_label(rp_selected, rp_label).clicked() {
                if self.sort_column == Some(2) {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(2);
                    self.sort_ascending = false;
                }
                self.sort_packages();
            }
            
            // Enabled sort button
            let enabled_selected = self.sort_column == Some(3);
            let enabled_label = if enabled_selected {
                format!("{} {}", tr!("col-enabled"), if self.sort_ascending { "▲" } else { "▼" })
            } else {
                format!("{} {}", tr!("col-enabled"), "▲") // Default ascending
            };
            if ui.selectable_label(enabled_selected, enabled_label).clicked() {
                if self.sort_column == Some(3) {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(3);
                    self.sort_ascending = true;
                }
                self.sort_packages();
            }
            
            // Install Reason sort button
            let reason_selected = self.sort_column == Some(4);
            let reason_label = if reason_selected {
                format!("{} {}", tr!("col-install-reason"), if self.sort_ascending { "▲" } else { "▼" })
            } else {
                format!("{} {}", tr!("col-install-reason"), "▲") // Default ascending
            };
            if ui.selectable_label(reason_selected, reason_label).clicked() {
                if self.sort_column == Some(4) {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_column = Some(4);
                    self.sort_ascending = true;
                }
                self.sort_packages();
            }
        }); 

        let clicked_package_idx = std::sync::Arc::new(std::sync::Mutex::new(None::<usize>));

        // Build table with proportional column widths for desktop
        let width_ratio = available_width / BASE_TABLE_WIDTH;
        let mut debloat_table = data_table()
            .id(egui::Id::new(format!(
                "debloat_data_table_v{}",
                self.table_version
            )))
            .default_row_height(if is_desktop { 56.0 } else { 80.0 })
            // .auto_row_height(true)
            .sortable_column(tr!("col-package-name"), if is_desktop { 350.0 * width_ratio } else { available_width * 0.52 }, false);
        if is_desktop {
            debloat_table = debloat_table
                .sortable_column(tr!("col-debloat-category"), 130.0 * width_ratio, false)
                .sortable_column("RP", 80.0 * width_ratio, true)
                .sortable_column(tr!("col-enabled"), 120.0 * width_ratio, false)
                .sortable_column(tr!("col-install-reason"), 110.0 * width_ratio, false);
        }
        debloat_table = debloat_table
            .sortable_column(tr!("col-tasks"), if is_desktop { 160.0 * width_ratio } else { available_width * 0.3  }, false)
            .allow_selection(true);

        // Sort column index mapping: self.sort_column uses logical (desktop) indices
        // Desktop: [0=PackageName, 1=DebloatCategory, 2=RP, 3=Enabled, 4=InstallReason, 5=Tasks]
        // Mobile:  [0=PackageName, 1=Tasks]
        let to_physical = |logical: usize| -> usize {
            if is_desktop { logical } else { match logical { 0 => 0, _ => 1 } }
        };
        let to_logical = |physical: usize| -> usize {
            if is_desktop { physical } else { match physical { 0 => 0, _ => 5 } }
        };

        if let Some(sort_col) = self.sort_column {
            use egui_material3::SortDirection;
            let direction = if self.sort_ascending {
                SortDirection::Ascending
            } else {
                SortDirection::Descending
            };
            if is_desktop || sort_col == 0 || sort_col == 5 {
                debloat_table = debloat_table.sort_by(to_physical(sort_col), direction);
            }
        }

        let mut filtered_package_names = Vec::new();

        // Collect filtered packages info first to avoid borrow issues
        // Note: uad_ng_lists_ref is pre-fetched at the start of ui()
        let filtered_packages: Vec<(usize, String, String, bool, String, String, String, String, bool)> = installed_packages
            .iter()
            .enumerate()
            .filter(|(_, p)| self.should_show_package(p, uad_ng_lists_ref))
            .filter(|(_, p)| self.matches_text_filter(p, uad_ng_lists_ref, &cached_fdroid_apps, &cached_google_play_apps, &cached_apkmirror_apps))
            .map(|(idx, package)| {
                let is_system = package.flags.contains("SYSTEM");
                let package_name = format!("{} ({})", package.pkg, package.versionName);
                let debloat_category = if let Some(ref lists) = uad_ng_lists_ref {
                    lists
                        .apps
                        .get(&package.pkg)
                        .map(|app| app.removal.clone())
                        .unwrap_or_else(|| "Unknown".to_string())
                } else {
                    "Unknown".to_string()
                };
                let runtime_perms = package
                    .users
                    .first()
                    .map(|u| u.runtimePermissions.len())
                    .unwrap_or(0)
                    .to_string();
                let enabled = package
                    .users
                    .first()
                    .map(|u| Self::enabled_to_display_string(u.enabled, u.installed, is_system))
                    .unwrap_or("DEFAULT")
                    .to_string();
                let install_reason_value = package.users.first().map(|u| u.installReason).unwrap_or(0);
                let install_reason = if is_system {
                    if install_reason_value == 0 {
                        "SYSTEM".to_string()
                    } else {
                        format!("{} (SYSTEM)", Self::install_reason_to_string(install_reason_value))
                    }
                } else {
                    Self::install_reason_to_string(install_reason_value).to_string()
                };
                let is_selected = self.selected_packages.contains(&package.pkg);
                (idx, package.pkg.clone(), package_name, is_system, debloat_category, runtime_perms, enabled, install_reason, is_selected)
            })
            .collect();

        log::debug!(
            "TabDebloatControl: Displaying {} of {} packages (filter: {:?}, hide_system: {}, show_only_enabled: {})",
            filtered_packages.len(),
            installed_packages.len(),
            self.active_filter,
            self.hide_system_app,
            self.show_only_enabled
        );

        // Collect filtered package names for both views
        for (_, pkg_id, _, _, _, _, _, _, _) in &filtered_packages {
            filtered_package_names.push(pkg_id.clone());
        }

        for (idx, pkg_id, package_name, is_system, debloat_category, runtime_perms, enabled_text, install_reason, is_selected) in filtered_packages {
            let clicked_idx_clone = clicked_package_idx.clone();
            let pkg_id_clone = pkg_id.clone();
            let package_name_clone = package_name.clone();
            let enabled_str = enabled_text.clone();
            let debloat_category_clone = debloat_category.clone();

            // Get cached app info from pre-fetched maps (avoids repeated mutex locks)
            let fd_cached = cached_fdroid_apps.get(&pkg_id);
            let gp_cached = cached_google_play_apps.get(&pkg_id);
            let am_cached = cached_apkmirror_apps.get(&pkg_id);

            // Prepare Android Package texture data (highest priority on Android)
            let (ap_texture, ap_title) = if android_package_enabled {
                if let Some(ap_app) = cached_android_package_apps.get(&pkg_id) {
                    let tex = Self::load_texture_from_bytes(ui.ctx(), &pkg_id, &ap_app.icon_bytes);
                    (tex.map(|t| t.id()), Some(ap_app.label.clone()))
                } else {
                    #[cfg(target_os = "android")]
                    {
                        if let Some(info) = crate::calc_androidpackage::fetch_android_package_info(&pkg_id) {
                            store.set_cached_android_package_app(pkg_id.clone(), info.clone());
                            let tex = Self::load_texture_from_bytes(ui.ctx(), &pkg_id, &info.icon_bytes);
                            (tex.map(|t| t.id()), Some(info.label.clone()))
                        } else {
                            (None, None)
                        }
                    }
                    #[cfg(not(target_os = "android"))]
                    { (None, None) }
                }
            } else {
                (None, None)
            };

            // Prepare texture data
            let (fd_texture, fd_title, fd_developer) = if !is_system && fdroid_enabled {
                if let Some(fd_app) = fd_cached {
                    if fd_app.raw_response != "404" {
                        let tex = fd_app.icon_base64.as_ref().and_then(|icon| {
                            Self::load_texture_from_base64(ui.ctx(), "fd", &pkg_id, icon)
                        });
                        (tex.map(|t| t.id()), Some(fd_app.title.clone()), Some(fd_app.developer.clone()))
                    } else {
                        (None, None, None)
                    }
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            };

            let (gp_texture, gp_title, gp_developer) = if !is_system && google_play_enabled && fd_title.is_none() {
                if let Some(gp_app) = gp_cached {
                    if gp_app.raw_response != "404" {
                        let tex = gp_app.icon_base64.as_ref().and_then(|icon| {
                            Self::load_texture_from_base64(ui.ctx(), "gp", &pkg_id, icon)
                        });
                        (tex.map(|t| t.id()), Some(gp_app.title.clone()), Some(gp_app.developer.clone()))
                    } else {
                        (None, None, None)
                    }
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            };

            let (am_texture, am_title, am_developer) = if is_system && apkmirror_enabled {
                if let Some(am_app) = am_cached {
                    if am_app.raw_response != "404" {
                        let tex = am_app.icon_base64.as_ref().and_then(|icon| {
                            Self::load_texture_from_base64(ui.ctx(), "am", &pkg_id, icon)
                        });
                        (tex.map(|t| t.id()), Some(am_app.title.clone()), Some(am_app.developer.clone()))
                    } else {
                        (None, None, None)
                    }
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            };

            debloat_table = debloat_table.row(|table_row| {
                // Package name column - show app info if available, otherwise show plain package name
                let debloat_category_clone2 = debloat_category.clone();
                let enabled_text_clone2 = enabled_text.clone();
                let install_reason_clone = install_reason.clone();
                let runtime_perms_clone = runtime_perms.clone();
                
                let ap_title_clone = ap_title.clone();
                let ap_pkg_id_display = pkg_id.clone();
                let mut row_builder = if let Some(title) = ap_title_clone {
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        ui.horizontal(|ui| {
                            if let Some(tex_id) = ap_texture {
                                ui.image((tex_id, egui::vec2(38.0, 38.0)));
                            }
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.1;
                                ui.label(egui::RichText::new(&title).strong());
                                ui.label(egui::RichText::new(&ap_pkg_id_display).small().color(egui::Color32::GRAY));

                                if !is_desktop {
                                    ui.add_space(4.0);
                                    egui::ScrollArea::horizontal()
                                        .id_salt(format!("debloat_ap_badge_scroll_{}", idx))
                                        .auto_shrink([false, true])
                                        .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            egui::Frame::new()
                                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 158, 158)))
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(format!("RP:{}", &runtime_perms_clone)).size(10.0));
                                                });

                                            let bg_color = match debloat_category_clone2.as_str() {
                                                "Recommended" => egui::Color32::from_rgb(56, 142, 60),
                                                "Advanced" => egui::Color32::from_rgb(33, 150, 243),
                                                "Expert" => egui::Color32::from_rgb(255, 152, 0),
                                                "Unsafe" => egui::Color32::from_rgb(255, 235, 59),
                                                "Unknown" => egui::Color32::from_rgb(255, 255, 255),
                                                _ => egui::Color32::from_rgb(158, 158, 158),
                                            };
                                            let text_color = match debloat_category_clone2.as_str() {
                                                "Unknown" | "Unsafe" => egui::Color32::from_rgb(0, 0, 0),
                                                _ => egui::Color32::WHITE,
                                            };
                                            let label_text = match debloat_category_clone2.as_str() {
                                                "Recommended" => tr!("label-recommended"),
                                                "Advanced" => tr!("label-advanced"),
                                                "Expert" => tr!("label-expert"),
                                                "Unsafe" => tr!("label-unsafe"),
                                                "Unknown" => tr!("label-unknown"),
                                                _ => debloat_category_clone2.clone(),
                                            };
                                            egui::Frame::new()
                                                .fill(bg_color)
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(&label_text).color(text_color).size(10.0));
                                                });

                                            let enabled_bg_color = match enabled_text_clone2.as_str() {
                                                "REMOVED_USER" | "DISABLED" | "DISABLED_USER" => egui::Color32::from_rgb(211, 47, 47),
                                                "DEFAULT" | "ENABLED" | "UNKNOWN" => egui::Color32::from_rgb(56, 142, 60),
                                                _ => egui::Color32::from_rgb(158, 158, 158),
                                            };
                                            egui::Frame::new()
                                                .fill(enabled_bg_color)
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(&enabled_text_clone2).color(egui::Color32::WHITE).size(10.0));
                                                });

                                            ui.label(egui::RichText::new(&install_reason_clone).small().color(egui::Color32::GRAY).size(10.0));
                                        });
                                    });
                                }
                            });
                        });
                    })
                } else if let (Some(title), Some(developer)) = (fd_title.clone(), fd_developer.clone()) {
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        ui.horizontal(|ui| {
                            if let Some(tex_id) = fd_texture {
                                ui.image((tex_id, egui::vec2(38.0, 38.0)));
                            }
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.1;
                                ui.label(egui::RichText::new(&title).strong());
                                ui.label(egui::RichText::new(&developer).small().color(egui::Color32::GRAY));

                                if !is_desktop {
                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        // Runtime permissions badge
                                        egui::Frame::new()
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 158, 158)))
                                            .corner_radius(6.0)
                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(format!("RP:{}", &runtime_perms_clone)).size(10.0));
                                            });
                                        
                                        // Debloat category badge
                                        let bg_color = match debloat_category_clone2.as_str() {
                                            "Recommended" => egui::Color32::from_rgb(56, 142, 60),
                                            "Advanced" => egui::Color32::from_rgb(33, 150, 243),
                                            "Expert" => egui::Color32::from_rgb(255, 152, 0),
                                            "Unsafe" => egui::Color32::from_rgb(255, 235, 59),
                                            "Unknown" => egui::Color32::from_rgb(255, 255, 255),
                                            _ => egui::Color32::from_rgb(158, 158, 158),
                                        };
                                        let text_color = match debloat_category_clone2.as_str() {
                                            "Unknown" | "Unsafe" => egui::Color32::from_rgb(0, 0, 0),
                                            _ => egui::Color32::WHITE,
                                        };
                                        let label_text = match debloat_category_clone2.as_str() {
                                            "Recommended" => tr!("label-recommended"),
                                            "Advanced" => tr!("label-advanced"),
                                            "Expert" => tr!("label-expert"),
                                            "Unsafe" => tr!("label-unsafe"),
                                            "Unknown" => tr!("label-unknown"),
                                            _ => debloat_category_clone2.clone(),
                                        };
                                        egui::Frame::new()
                                            .fill(bg_color)
                                            .corner_radius(6.0)
                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(&label_text).color(text_color).size(10.0));
                                            });
                                        
                                        // Enabled status badge
                                        let enabled_bg_color = match enabled_text_clone2.as_str() {
                                            "REMOVED_USER" | "DISABLED" | "DISABLED_USER" => egui::Color32::from_rgb(211, 47, 47),
                                            "DEFAULT" | "ENABLED" | "UNKNOWN" => egui::Color32::from_rgb(56, 142, 60),
                                            _ => egui::Color32::from_rgb(158, 158, 158),
                                        };
                                        egui::Frame::new()
                                            .fill(enabled_bg_color)
                                            .corner_radius(6.0)
                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(&enabled_text_clone2).color(egui::Color32::WHITE).size(10.0));
                                            });
                                        
                                        // Install reason badge
                                        ui.label(egui::RichText::new(&install_reason_clone).small().color(egui::Color32::GRAY).size(10.0));
                                    });
                                }
                            });
                        });
                    })
                } else if let (Some(title), Some(developer)) = (gp_title.clone(), gp_developer.clone()) {
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        ui.horizontal(|ui| {
                            if let Some(tex_id) = gp_texture {
                                ui.image((tex_id, egui::vec2(38.0, 38.0)));
                            }
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.1;
                                egui::ScrollArea::horizontal()
                                    .id_salt(format!("debloat_title_scroll_{}", idx))
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        ui.add(egui::Label::new(egui::RichText::new(&title).strong()).wrap_mode(egui::TextWrapMode::Extend));
                                    });
                                ui.label(egui::RichText::new(&developer).small().color(egui::Color32::GRAY));

                                if !is_desktop {
                                    ui.add_space(4.0);
                                    egui::ScrollArea::horizontal()
                                        .id_salt(format!("debloat_badge_scroll_{}", idx))
                                        .auto_shrink([false, true])
                                        .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // Runtime permissions badge
                                            egui::Frame::new()
                                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 158, 158)))
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(format!("RP:{}", &runtime_perms_clone)).size(10.0));
                                                });

                                            // Debloat category badge
                                            let bg_color = match debloat_category_clone2.as_str() {
                                                "Recommended" => egui::Color32::from_rgb(56, 142, 60),
                                                "Advanced" => egui::Color32::from_rgb(33, 150, 243),
                                                "Expert" => egui::Color32::from_rgb(255, 152, 0),
                                                "Unsafe" => egui::Color32::from_rgb(255, 235, 59),
                                                "Unknown" => egui::Color32::from_rgb(255, 255, 255),
                                                _ => egui::Color32::from_rgb(158, 158, 158),
                                            };
                                            let text_color = match debloat_category_clone2.as_str() {
                                                "Unknown" | "Unsafe" => egui::Color32::from_rgb(0, 0, 0),
                                                _ => egui::Color32::WHITE,
                                            };
                                            let label_text = match debloat_category_clone2.as_str() {
                                                "Recommended" => tr!("label-recommended"),
                                                "Advanced" => tr!("label-advanced"),
                                                "Expert" => tr!("label-expert"),
                                                "Unsafe" => tr!("label-unsafe"),
                                                "Unknown" => tr!("label-unknown"),
                                                _ => debloat_category_clone2.clone(),
                                            };
                                            egui::Frame::new()
                                                .fill(bg_color)
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(&label_text).color(text_color).size(10.0));
                                                });

                                            // Enabled status badge
                                            let enabled_bg_color = match enabled_text_clone2.as_str() {
                                                "REMOVED_USER" | "DISABLED" | "DISABLED_USER" => egui::Color32::from_rgb(211, 47, 47),
                                                "DEFAULT" | "ENABLED" | "UNKNOWN" => egui::Color32::from_rgb(56, 142, 60),
                                                _ => egui::Color32::from_rgb(158, 158, 158),
                                            };
                                            egui::Frame::new()
                                                .fill(enabled_bg_color)
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(&enabled_text_clone2).color(egui::Color32::WHITE).size(10.0));
                                                });

                                            // Install reason badge
                                            ui.label(egui::RichText::new(&install_reason_clone).small().color(egui::Color32::GRAY).size(10.0));
                                        });
                                    });
                                }
                            });
                        });
                    })
                } else if let (Some(title), Some(developer)) = (am_title.clone(), am_developer.clone()) {
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        ui.horizontal(|ui| {
                            if let Some(tex_id) = am_texture {
                                ui.image((tex_id, egui::vec2(38.0, 38.0)));
                            }
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.1;
                                egui::ScrollArea::horizontal()
                                    .id_salt(format!("debloat_am_title_scroll_{}", idx))
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                        ui.add(egui::Label::new(egui::RichText::new(&title).strong()).wrap_mode(egui::TextWrapMode::Extend));
                                    });
                                ui.label(egui::RichText::new(&developer).small().color(egui::Color32::GRAY));

                                if !is_desktop {
                                    ui.add_space(4.0);
                                    egui::ScrollArea::horizontal()
                                        .id_salt(format!("debloat_am_badge_scroll_{}", idx))
                                        .auto_shrink([false, true])
                                        .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            // Runtime permissions badge
                                            egui::Frame::new()
                                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 158, 158)))
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(format!("RP:{}", &runtime_perms_clone)).size(10.0));
                                                });

                                            // Debloat category badge
                                            let bg_color = match debloat_category_clone2.as_str() {
                                                "Recommended" => egui::Color32::from_rgb(56, 142, 60),
                                                "Advanced" => egui::Color32::from_rgb(33, 150, 243),
                                                "Expert" => egui::Color32::from_rgb(255, 152, 0),
                                                "Unsafe" => egui::Color32::from_rgb(255, 235, 59),
                                                "Unknown" => egui::Color32::from_rgb(255, 255, 255),
                                                _ => egui::Color32::from_rgb(158, 158, 158),
                                            };
                                            let text_color = match debloat_category_clone2.as_str() {
                                                "Unknown" | "Unsafe" => egui::Color32::from_rgb(0, 0, 0),
                                                _ => egui::Color32::WHITE,
                                            };
                                            let label_text = match debloat_category_clone2.as_str() {
                                                "Recommended" => tr!("label-recommended"),
                                                "Advanced" => tr!("label-advanced"),
                                                "Expert" => tr!("label-expert"),
                                                "Unsafe" => tr!("label-unsafe"),
                                                "Unknown" => tr!("label-unknown"),
                                                _ => debloat_category_clone2.clone(),
                                            };
                                            egui::Frame::new()
                                                .fill(bg_color)
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(&label_text).color(text_color).size(10.0));
                                                });

                                            // Enabled status badge
                                            let enabled_bg_color = match enabled_text_clone2.as_str() {
                                                "REMOVED_USER" | "DISABLED" | "DISABLED_USER" => egui::Color32::from_rgb(211, 47, 47),
                                                "DEFAULT" | "ENABLED" | "UNKNOWN" => egui::Color32::from_rgb(56, 142, 60),
                                                _ => egui::Color32::from_rgb(158, 158, 158),
                                            };
                                            egui::Frame::new()
                                                .fill(enabled_bg_color)
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(8, 3))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new(&enabled_text_clone2).color(egui::Color32::WHITE).size(10.0));
                                                });

                                            // Install reason badge
                                            ui.label(egui::RichText::new(&install_reason_clone).small().color(egui::Color32::GRAY).size(10.0));
                                        });
                                    });
                                }
                            });
                        });
                    })
                } else {
                    // No app info available, show plain package name (no spinner)
                    let pkg_name = package_name_clone.clone();
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        ui.vertical(|ui| {
                            egui::ScrollArea::horizontal()
                                .id_salt(format!("debloat_pkg_scroll_{}", idx))
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    ui.add(egui::Label::new(&pkg_name).wrap_mode(egui::TextWrapMode::Extend));
                                });
                            
                            if !is_desktop {
                                ui.add_space(4.0);
                                egui::ScrollArea::horizontal()
                                    .id_salt(format!("debloat_pkg_badge_scroll_{}", idx))
                                    .auto_shrink([false, true])
                                    .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        // Runtime permissions badge
                                        egui::Frame::new()
                                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(158, 158, 158)))
                                            .corner_radius(6.0)
                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(format!("RP:{}", &runtime_perms_clone)).size(10.0));
                                            });

                                        // Debloat category badge
                                        let bg_color = match debloat_category_clone2.as_str() {
                                            "Recommended" => egui::Color32::from_rgb(56, 142, 60),
                                            "Advanced" => egui::Color32::from_rgb(33, 150, 243),
                                            "Expert" => egui::Color32::from_rgb(255, 152, 0),
                                            "Unsafe" => egui::Color32::from_rgb(255, 235, 59),
                                            "Unknown" => egui::Color32::from_rgb(255, 255, 255),
                                            _ => egui::Color32::from_rgb(158, 158, 158),
                                        };
                                        let text_color = match debloat_category_clone2.as_str() {
                                            "Unknown" | "Unsafe" => egui::Color32::from_rgb(0, 0, 0),
                                            _ => egui::Color32::WHITE,
                                        };
                                        let label_text = match debloat_category_clone2.as_str() {
                                            "Recommended" => tr!("label-recommended"),
                                            "Advanced" => tr!("label-advanced"),
                                            "Expert" => tr!("label-expert"),
                                            "Unsafe" => tr!("label-unsafe"),
                                            "Unknown" => tr!("label-unknown"),
                                            _ => debloat_category_clone2.clone(),
                                        };
                                        egui::Frame::new()
                                            .fill(bg_color)
                                            .corner_radius(6.0)
                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(&label_text).color(text_color).size(10.0));
                                            });

                                        // Enabled status badge
                                        let enabled_bg_color = match enabled_text_clone2.as_str() {
                                            "REMOVED_USER" | "DISABLED" | "DISABLED_USER" => egui::Color32::from_rgb(211, 47, 47),
                                            "DEFAULT" | "ENABLED" | "UNKNOWN" => egui::Color32::from_rgb(56, 142, 60),
                                            _ => egui::Color32::from_rgb(158, 158, 158),
                                        };
                                        egui::Frame::new()
                                            .fill(enabled_bg_color)
                                            .corner_radius(6.0)
                                            .inner_margin(egui::Margin::symmetric(8, 3))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(&enabled_text_clone2).color(egui::Color32::WHITE).size(10.0));
                                            });

                                        // Install reason badge
                                        ui.label(egui::RichText::new(&install_reason_clone).small().color(egui::Color32::GRAY).size(10.0));
                                    });
                                });
                            }
                        });
                    })
                };
                
                // Desktop-only columns
                if is_desktop {
                    // Debloat category column
                    row_builder = row_builder.widget_cell(move |ui: &mut egui::Ui| {
                        let bg_color = match debloat_category_clone.as_str() {
                            "Recommended" => egui::Color32::from_rgb(56, 142, 60),
                            "Advanced" => egui::Color32::from_rgb(33, 150, 243),
                            "Expert" => egui::Color32::from_rgb(255, 152, 0),
                            "Unsafe" => egui::Color32::from_rgb(255, 235, 59),
                            "Unknown" => egui::Color32::from_rgb(255, 255, 255),
                            _ => egui::Color32::from_rgb(158, 158, 158),
                        };
                        let text_color = match debloat_category_clone.as_str() {
                            "Unknown" | "Unsafe" => egui::Color32::from_rgb(0, 0, 0),
                            _ => egui::Color32::WHITE,
                        };
                        let label_text = match debloat_category_clone.as_str() {
                            "Recommended" => tr!("label-recommended"),
                            "Advanced" => tr!("label-advanced"),
                            "Expert" => tr!("label-expert"),
                            "Unsafe" => tr!("label-unsafe"),
                            "Unknown" => tr!("label-unknown"),
                            _ => debloat_category_clone.clone(),
                        };
                        ui.horizontal(|ui| {
                            egui::Frame::new()
                                .fill(bg_color)
                                .corner_radius(8.0)
                                .inner_margin(egui::Margin::symmetric(12, 6))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(&label_text).color(text_color).size(12.0));
                                });
                        });
                    });

                    // Runtime permissions column
                    row_builder = row_builder.cell(&runtime_perms);

                    // Enabled column
                    let enabled_text_clone = enabled_text.clone();
                    row_builder = row_builder.widget_cell(move |ui: &mut egui::Ui| {
                        let bg_color = match enabled_text_clone.as_str() {
                            "REMOVED_USER" | "DISABLED" | "DISABLED_USER" => egui::Color32::from_rgb(211, 47, 47),
                            "DEFAULT" | "ENABLED" | "UNKNOWN" => egui::Color32::from_rgb(56, 142, 60),
                            _ => egui::Color32::from_rgb(158, 158, 158),
                        };
                        ui.horizontal(|ui| {
                            egui::Frame::new()
                                .fill(bg_color)
                                .corner_radius(8.0)
                                .inner_margin(egui::Margin::symmetric(12, 6))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(&enabled_text_clone).color(egui::Color32::WHITE).size(12.0));
                                });
                        });
                    });

                    // Install reason column
                    row_builder = row_builder.cell(install_reason);
                }

                // Tasks column
                let pkg_id_for_buttons = pkg_id_clone.clone();
                let is_unsafe_blocked = debloat_category == "Unsafe" && !self.unsafe_app_remove;
                row_builder = row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt(format!("debloat_task_scroll_{}", idx))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;

                            if ui.add(icon_button_standard(ICON_INFO.to_string())).on_hover_text(tr!("package-info")).clicked() {
                                if let Ok(mut clicked) = clicked_idx_clone.lock() {
                                    *clicked = Some(idx);
                                }
                            }
                            
                            if (enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED")) && !is_unsafe_blocked {
                                if ui.add(icon_button_standard(ICON_TOGGLE_OFF.to_string()).icon_color(egui::Color32::from_rgb(56, 142, 60))).on_hover_text(tr!("disable")).clicked() {
                                    ui.data_mut(|data| {
                                        data.insert_temp(egui::Id::new("disable_clicked_package"), pkg_id_for_buttons.clone());
                                    });
                                }
                            }

                            if enabled_str.contains("REMOVED_USER") || enabled_str.contains("DISABLED_USER") || enabled_str.contains("DISABLED") {
                                if ui.add(icon_button_standard(ICON_TOGGLE_ON.to_string()).icon_color(egui::Color32::from_rgb(211, 47, 47))).on_hover_text(tr!("enable")).clicked() {
                                    ui.data_mut(|data| {
                                        data.insert_temp(egui::Id::new("enable_clicked_package"), pkg_id_for_buttons.clone());
                                    });
                                }
                            }

                            if (enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED")) && !is_unsafe_blocked {
                                if ui.add(icon_button_standard(ICON_DELETE.to_string()).icon_color(egui::Color32::from_rgb(211, 47, 47))).on_hover_text(tr!("uninstall")).clicked() {
                                    ui.data_mut(|data| {
                                        data.insert_temp(egui::Id::new("uninstall_clicked_package"), pkg_id_for_buttons.clone());
                                        data.insert_temp(egui::Id::new("uninstall_clicked_is_system"), is_system);
                                    });
                                }
                            }

                        });
                    });
                });

                row_builder = row_builder.id(format!("debloat_table_row_{}", idx));

                if is_selected {
                    row_builder = row_builder.selected(true);
                }

                row_builder
            });
        }

        let table_response = debloat_table.show(ui);

        // Sync sort state from widget, but only when sorting by a column the widget knows about.
        // On mobile, hidden columns (1-4) are managed by the mobile sort buttons, not the table widget.
        let mobile_hidden_sort = !is_desktop && matches!(self.sort_column, Some(1..=4));
        if !mobile_hidden_sort {
            let (widget_sort_col, widget_sort_dir) = table_response.sort_state;
            let logical_sort_col = widget_sort_col.map(|c| to_logical(c));
            let widget_sort_ascending = matches!(widget_sort_dir, egui_material3::SortDirection::Ascending);

            if logical_sort_col != self.sort_column
                || (logical_sort_col.is_some() && widget_sort_ascending != self.sort_ascending)
            {
                self.sort_column = logical_sort_col;
                self.sort_ascending = widget_sort_ascending;
                if self.sort_column.is_some() {
                    self.sort_packages();
                }
            }
        }

        if let Some(clicked_col) = table_response.column_clicked {
            let logical_clicked = to_logical(clicked_col);
            if self.sort_column == Some(logical_clicked) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(logical_clicked);
                self.sort_ascending = true;
            }
            self.sort_packages();
        }

        // Sync selection state
        if table_response.selected_rows.len() == filtered_package_names.len() {
            for (filtered_idx, package_name) in filtered_package_names.iter().enumerate() {
                if let Some(&selected) = table_response.selected_rows.get(filtered_idx) {
                    if selected {
                        self.selected_packages.insert(package_name.clone());
                    } else {
                        self.selected_packages.remove(package_name);
                    }
                }
            }
        }

        // Handle package details dialog
        if let Ok(clicked) = clicked_package_idx.lock() {
            if let Some(idx) = *clicked {
                self.package_details_dialog.open(idx);
            }
        }

        // Handle button clicks
        let mut uninstall_package: Option<String> = None;
        let mut uninstall_is_system: bool = false;
        let mut enable_package: Option<String> = None;
        let mut disable_package: Option<String> = None;
        let mut batch_uninstall: bool = false;
        let mut batch_disable: bool = false;
        let mut batch_enable: bool = false;

        ui.data_mut(|data| {
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("uninstall_clicked_package")) {
                uninstall_package = Some(pkg);
                uninstall_is_system = data.get_temp::<bool>(egui::Id::new("uninstall_clicked_is_system")).unwrap_or(false);
                data.remove::<String>(egui::Id::new("uninstall_clicked_package"));
                data.remove::<bool>(egui::Id::new("uninstall_clicked_is_system"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("enable_clicked_package")) {
                enable_package = Some(pkg);
                data.remove::<String>(egui::Id::new("enable_clicked_package"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("disable_clicked_package")) {
                disable_package = Some(pkg);
                data.remove::<String>(egui::Id::new("disable_clicked_package"));
            }
            if data.get_temp::<bool>(egui::Id::new("batch_uninstall_clicked")).unwrap_or(false) {
                batch_uninstall = true;
                data.remove::<bool>(egui::Id::new("batch_uninstall_clicked"));
            }
            if data.get_temp::<bool>(egui::Id::new("batch_disable_clicked")).unwrap_or(false) {
                batch_disable = true;
                data.remove::<bool>(egui::Id::new("batch_disable_clicked"));
            }
            if data.get_temp::<bool>(egui::Id::new("batch_enable_clicked")).unwrap_or(false) {
                batch_enable = true;
                data.remove::<bool>(egui::Id::new("batch_enable_clicked"));
            }
        });

        // Open confirm dialog for single uninstall
        if let Some(pkg_name) = uninstall_package {
            self.uninstall_confirm_dialog.open_single(pkg_name, uninstall_is_system);
        }

        // Perform enable
        if let Some(pkg_name) = enable_package {
            if let Some(ref device) = self.selected_device {
                match crate::adb::enable_app(&pkg_name, device) {
                    Ok(output) => {
                        log::info!("App enabled successfully: {}", output);
                        let mut packages = store.get_installed_packages();
                        if let Some(pkg) = packages.iter_mut().find(|p| p.pkg == pkg_name) {
                            for user in pkg.users.iter_mut() {
                                user.enabled = 1;
                                user.installed = true;
                            }
                        }
                        store.set_installed_packages(packages);
                        result = Some(AdbResult::Success(pkg_name.clone()));
                    }
                    Err(e) => {
                        log::error!("Failed to enable app: {}", e);
                        result = Some(AdbResult::Failure);
                    }
                }
            } else {
                log::error!("No device selected for enable");
                result = Some(AdbResult::Failure);
            }
        }

        // Perform disable
        if let Some(pkg_name) = disable_package {
            if let Some(ref device) = self.selected_device {
                match crate::adb::disable_app_current_user(&pkg_name, device, None) {
                    Ok(output) => {
                        log::info!("App disabled successfully: {}", output);
                        let mut packages = store.get_installed_packages();
                        if let Some(pkg) = packages.iter_mut().find(|p| p.pkg == pkg_name) {
                            for user in pkg.users.iter_mut() {
                                user.enabled = 3;
                            }
                        }
                        store.set_installed_packages(packages);
                        result = Some(AdbResult::Success(pkg_name.clone()));
                    }
                    Err(e) => {
                        log::error!("Failed to disable app: {}", e);
                        result = Some(AdbResult::Failure);
                    }
                }
            } else {
                log::error!("No device selected for disable");
                result = Some(AdbResult::Failure);
            }
        }

        // Open confirm dialog for batch uninstall
        if batch_uninstall {
            let packages_to_uninstall: Vec<String> = self.selected_packages.iter().cloned().collect();
            let installed = store.get_installed_packages();
            let is_system_flags: Vec<bool> = packages_to_uninstall.iter().map(|pkg| {
                installed.iter().find(|p| p.pkg == *pkg)
                    .map(|p| p.flags.contains("SYSTEM")).unwrap_or(false)
            }).collect();
            self.uninstall_confirm_dialog.open_batch(packages_to_uninstall, is_system_flags);
        }

        // Handle batch disable
        if batch_disable {
            if let Some(ref device) = self.selected_device {
                let packages_to_disable: Vec<String> = self.selected_packages.iter().cloned().collect();
                let mut packages = store.get_installed_packages();
                for pkg_name in packages_to_disable {
                    match crate::adb::disable_app_current_user(&pkg_name, device, None) {
                        Ok(output) => {
                            log::info!("App disabled successfully: {}", output);
                            if let Some(pkg) = packages.iter_mut().find(|p| p.pkg == pkg_name) {
                                for user in pkg.users.iter_mut() {
                                    user.enabled = 3;
                                }
                            }
                            result = Some(AdbResult::Success(pkg_name.clone()));
                        }
                        Err(e) => {
                            log::error!("Failed to disable app {}: {}", pkg_name, e);
                            result = Some(AdbResult::Failure);
                        }
                    }
                }
                store.set_installed_packages(packages);
            } else {
                log::error!("No device selected for batch disable");
                result = Some(AdbResult::Failure);
            }
        }

        // Handle batch enable
        if batch_enable {
            if let Some(ref device) = self.selected_device {
                let packages_to_enable: Vec<String> = self.selected_packages.iter().cloned().collect();
                let mut packages = store.get_installed_packages();
                for pkg_name in packages_to_enable {
                    match crate::adb::enable_app(&pkg_name, device) {
                        Ok(output) => {
                            log::info!("App enabled successfully: {}", output);
                            if let Some(pkg) = packages.iter_mut().find(|p| p.pkg == pkg_name) {
                                for user in pkg.users.iter_mut() {
                                    user.enabled = 1;
                                    user.installed = true;
                                }
                            }
                            result = Some(AdbResult::Success(pkg_name.clone()));
                        }
                        Err(e) => {
                            log::error!("Failed to enable app {}: {}", pkg_name, e);
                            result = Some(AdbResult::Failure);
                        }
                    }
                }
                store.set_installed_packages(packages);
            } else {
                log::error!("No device selected for batch enable");
                result = Some(AdbResult::Failure);
            }
        }

        // Show uninstall confirm dialog and execute on confirmation
        if self.uninstall_confirm_dialog.show(ui.ctx()) {
            let pkgs = std::mem::take(&mut self.uninstall_confirm_dialog.packages);
            let sys_flags = std::mem::take(&mut self.uninstall_confirm_dialog.is_system);
            self.uninstall_confirm_dialog.reset();

            if let Some(ref device) = self.selected_device {
                let mut packages = store.get_installed_packages();
                for (pkg_name, is_system) in pkgs.into_iter().zip(sys_flags.into_iter()) {
                    // Skip unsafe apps when unsafe_app_remove is disabled
                    let is_unsafe = uad_ng_lists_ref
                        .and_then(|lists| lists.apps.get(&pkg_name))
                        .map(|app| app.removal == "Unsafe")
                        .unwrap_or(false);
                    if is_unsafe && !self.unsafe_app_remove {
                        log::warn!("Skipping uninstall of unsafe app: {}", pkg_name);
                        continue;
                    }
                    let uninstall_result = if is_system {
                        crate::adb::uninstall_app_user(&pkg_name, device, None)
                    } else {
                        crate::adb::uninstall_app(&pkg_name, device)
                    };

                    match uninstall_result {
                        Ok(output) => {
                            log::info!("App uninstalled successfully: {}", output);
                            if is_system {
                                if let Some(pkg) = packages.iter_mut().find(|p| p.pkg == pkg_name) {
                                    for user in pkg.users.iter_mut() {
                                        user.installed = false;
                                        user.enabled = 0;
                                    }
                                }
                            } else {
                                packages.retain(|pkg| pkg.pkg != pkg_name);
                                self.selected_packages.remove(&pkg_name);
                            }
                            result = Some(AdbResult::Success(pkg_name.clone()));
                        }
                        Err(e) => {
                            log::error!("Failed to uninstall app({}): {}", pkg_name, e);
                            result = Some(AdbResult::Failure);
                        }
                    }
                }
                store.set_installed_packages(packages);
            } else {
                log::error!("No device selected for uninstall");
                result = Some(AdbResult::Failure);
            }
        }

        // Show package details dialog
        let packages_for_dialog = store.get_installed_packages();
        let uad_lists_for_dialog = store.get_uad_ng_lists();
        self.package_details_dialog.show(ui.ctx(), &packages_for_dialog, &uad_lists_for_dialog);

        result
    }
}

/// iOS-style toggle switch
fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter().circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

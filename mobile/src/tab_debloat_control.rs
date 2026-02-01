use crate::adb::PackageFingerprint;
use crate::uad_shizuku_app::UadNgLists;
use crate::models::{ApkMirrorApp, FDroidApp, GooglePlayApp};
pub use crate::tab_debloat_control_stt::*;
use crate::win_package_details_dialog::PackageDetailsDialog;
use eframe::egui;
use egui_i18n::tr;
use egui_material3::{assist_chip, data_table, theme::get_global_color, MaterialButton};
use std::collections::HashMap;

use crate::svg_stt::*;

impl Default for TabDebloatControl {
    fn default() -> Self {
        Self {
            open: false,
            installed_packages: Vec::new(),
            uad_ng_lists: None,
            selected_packages: std::collections::HashSet::new(),
            package_details_dialog: PackageDetailsDialog::new(),
            active_filter: DebloatFilter::All,
            sort_column: None,
            sort_ascending: true,
            selected_device: None,
            table_version: 0,
            google_play_textures: HashMap::new(),
            fdroid_textures: HashMap::new(),
            apkmirror_textures: HashMap::new(),
            cached_google_play_apps: HashMap::new(),
            cached_fdroid_apps: HashMap::new(),
            cached_apkmirror_apps: HashMap::new(),
            show_only_enabled: false,
            hide_system_app: false,
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
        let package_names: std::collections::HashSet<String> =
            packages.iter().map(|p| p.pkg.clone()).collect();
        self.selected_packages
            .retain(|pkg| package_names.contains(pkg));

        self.installed_packages = packages;
        self.table_version = self.table_version.wrapping_add(1);
        self.sort_column = None;
        self.sort_ascending = true;
    }

    pub fn set_selected_device(&mut self, device: Option<String>) {
        self.selected_device = device;
    }

    pub fn update_uad_ng_lists(&mut self, lists: UadNgLists) {
        self.uad_ng_lists = Some(lists);
    }

    /// Update cached app info from fetched results
    pub fn update_cached_google_play(&mut self, pkg_id: String, app: GooglePlayApp) {
        self.cached_google_play_apps.insert(pkg_id, app);
    }

    pub fn update_cached_fdroid(&mut self, pkg_id: String, app: FDroidApp) {
        self.cached_fdroid_apps.insert(pkg_id, app);
    }

    pub fn update_cached_apkmirror(&mut self, pkg_id: String, app: ApkMirrorApp) {
        self.cached_apkmirror_apps.insert(pkg_id, app);
    }

    fn get_recommended_count(&self) -> (usize, usize) {
        self.get_category_count("Recommended")
    }

    fn get_advanced_count(&self) -> (usize, usize) {
        self.get_category_count("Advanced")
    }

    fn get_expert_count(&self) -> (usize, usize) {
        self.get_category_count("Expert")
    }

    fn get_unsafe_count(&self) -> (usize, usize) {
        self.get_category_count("Unsafe")
    }

    fn get_unknown_count(&self) -> (usize, usize) {
        if let Some(uad_ng_lists) = &self.uad_ng_lists {
            let unknown_packages: Vec<_> = self
                .installed_packages
                .iter()
                .filter(|package| uad_ng_lists.apps.get(&package.pkg).is_none())
                .collect();

            let total_count = unknown_packages.len();
            let enabled_count = unknown_packages
                .iter()
                .filter(|package| self.is_package_enabled(package))
                .count();

            (enabled_count, total_count)
        } else {
            (0, 0)
        }
    }

    fn get_category_count(&self, category: &str) -> (usize, usize) {
        if let Some(uad_ng_lists) = &self.uad_ng_lists {
            let category_packages: Vec<_> = self
                .installed_packages
                .iter()
                .filter(|package| {
                    uad_ng_lists
                        .apps
                        .get(&package.pkg)
                        .map(|app_entry| app_entry.removal == category)
                        .unwrap_or(false)
                })
                .collect();

            let total_count = category_packages.len();
            let enabled_count = category_packages
                .iter()
                .filter(|package| self.is_package_enabled(package))
                .count();

            (enabled_count, total_count)
        } else {
            (0, 0)
        }
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

    fn should_show_package(&self, package: &PackageFingerprint) -> bool {
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
            DebloatFilter::Recommended => self.matches_category(package, "Recommended"),
            DebloatFilter::Advanced => self.matches_category(package, "Advanced"),
            DebloatFilter::Expert => self.matches_category(package, "Expert"),
            DebloatFilter::Unsafe => self.matches_category(package, "Unsafe"),
            DebloatFilter::Unknown => {
                if let Some(uad_ng_lists) = &self.uad_ng_lists {
                    uad_ng_lists.apps.get(&package.pkg).is_none()
                } else {
                    true
                }
            }
        }
    }

    fn matches_category(&self, package: &PackageFingerprint, category: &str) -> bool {
        if let Some(uad_ng_lists) = &self.uad_ng_lists {
            uad_ng_lists
                .apps
                .get(&package.pkg)
                .map(|app| app.removal == category)
                .unwrap_or(false)
        } else {
            false
        }
    }

    fn sort_packages(&mut self) {
        if let Some(col_idx) = self.sort_column {
            let uad_ng_lists = self.uad_ng_lists.clone();

            self.installed_packages.sort_by(|a, b| {
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
        }
    }

    fn load_texture_from_base64(
        ctx: &egui::Context,
        textures: &mut HashMap<String, egui::TextureHandle>,
        prefix: &str,
        package_id: &str,
        base64_data: &str,
    ) -> Option<egui::TextureHandle> {
        if let Some(texture) = textures.get(package_id) {
            return Some(texture.clone());
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
                tracing::warn!("Failed to decode base64 image for {}: {}", package_id, e);
                return None;
            }
        };

        let image = match image::load_from_memory(&bytes) {
            Ok(img) => img,
            Err(e) => {
                tracing::warn!("Failed to load image for {}: {}", package_id, e);
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

        textures.insert(package_id.to_string(), texture.clone());
        Some(texture)
    }


    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        google_play_enabled: bool,
        fdroid_enabled: bool,
        apkmirror_enabled: bool,
    ) -> Option<AdbResult> {
        let mut result = None;

        // Filter Buttons
        if !self.installed_packages.is_empty() {
            ui.horizontal(|ui| {
                let show_all_colors = self.active_filter == DebloatFilter::All;

                let all_count = self.installed_packages.len();
                let all_text = tr!("all", { count: all_count });
                let button = if self.active_filter == DebloatFilter::All {
                    MaterialButton::filled(&all_text)
                        .fill(egui::Color32::from_rgb(158, 158, 158))
                } else {
                    MaterialButton::outlined(&all_text)
                };
                if ui.add(button).clicked() {
                    self.active_filter = DebloatFilter::All;
                }

                let (enabled, total) = self.get_recommended_count();
                let rec_text = tr!("recommended", { enabled: enabled, total: total });
                let button = if self.active_filter == DebloatFilter::Recommended || show_all_colors {
                    MaterialButton::filled(&rec_text)
                        .fill(egui::Color32::from_rgb(56, 142, 60))
                } else {
                    MaterialButton::outlined(&rec_text)
                };
                if ui.add(button).clicked() {
                    self.active_filter = DebloatFilter::Recommended;
                }

                let (enabled, total) = self.get_advanced_count();
                let adv_text = tr!("advanced", { enabled: enabled, total: total });
                let button = if self.active_filter == DebloatFilter::Advanced || show_all_colors {
                    MaterialButton::filled(&adv_text)
                        .fill(egui::Color32::from_rgb(33, 150, 243))
                } else {
                    MaterialButton::outlined(&adv_text)
                };
                if ui.add(button).clicked() {
                    self.active_filter = DebloatFilter::Advanced;
                }

                let (enabled, total) = self.get_expert_count();
                let exp_text = tr!("expert", { enabled: enabled, total: total });
                let button = if self.active_filter == DebloatFilter::Expert || show_all_colors {
                    MaterialButton::filled(&exp_text)
                        .fill(egui::Color32::from_rgb(255, 152, 0))
                } else {
                    MaterialButton::outlined(&exp_text)
                };
                if ui.add(button).clicked() {
                    self.active_filter = DebloatFilter::Expert;
                }

                let (enabled, total) = self.get_unsafe_count();
                let unsafe_text = tr!("unsafe", { enabled: enabled, total: total });
                let button = if self.active_filter == DebloatFilter::Unsafe || show_all_colors {
                    MaterialButton::filled(&unsafe_text)
                        .fill(egui::Color32::from_rgb(255, 235, 59))
                } else {
                    MaterialButton::outlined(&unsafe_text)
                };
                if ui.add(button).clicked() {
                    self.active_filter = DebloatFilter::Unsafe;
                }

                let (enabled, total) = self.get_unknown_count();
                let unknown_text = tr!("unknown", { enabled: enabled, total: total });
                let button = if self.active_filter == DebloatFilter::Unknown || show_all_colors {
                    MaterialButton::filled(&unknown_text)
                        .fill(egui::Color32::from_rgb(255, 255, 255))
                } else {
                    MaterialButton::outlined(&unknown_text)
                };
                if ui.add(button).clicked() {
                    self.active_filter = DebloatFilter::Unknown;
                }
            });
        }

        if self.installed_packages.is_empty() {
            ui.label(tr!("no-packages-loaded"));
            return None;
        }

        // Batch action buttons
        ui.horizontal(|ui| {
            let selected_count = self.selected_packages.len();

            if ui.add(MaterialButton::outlined(tr!("deselect-all"))).clicked() {
                self.selected_packages.clear();
            }

            ui.separator();

            ui.label(tr!("selected-count", { count: selected_count }));

            if selected_count > 0 {
                if ui
                    .add(MaterialButton::filled(&tr!("uninstall-selected", { count: selected_count })))
                    .clicked()
                {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("batch_uninstall_clicked"), true);
                    });
                }

                if ui
                    .add(MaterialButton::filled(&tr!("disable-selected", { count: selected_count })))
                    .clicked()
                {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("batch_disable_clicked"), true);
                    });
                }

                if ui
                    .add(MaterialButton::filled(&tr!("enable-selected", { count: selected_count })))
                    .clicked()
                {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("batch_enable_clicked"), true);
                    });
                }
            }
        });
        ui.add_space(10.0);

        // Show only enabled toggle
        ui.horizontal(|ui| {
            ui.label(tr!("show-only-enabled"));
            toggle_ui(ui, &mut self.show_only_enabled);
            ui.add_space(10.0);
            ui.label(tr!("hide-system-app"));
            toggle_ui(ui, &mut self.hide_system_app);
        });
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                ui.label(format!("* RP: {}", tr!("col-runtime-permissions")));
            });
        });

        let clicked_package_idx = std::sync::Arc::new(std::sync::Mutex::new(None::<usize>));

        // Apply Material theme styling
        let surface = get_global_color("surface");
        let on_surface = get_global_color("onSurface");
        let primary = get_global_color("primary");

        let mut style = (*ui.ctx().style()).clone();
        style.visuals.widgets.noninteractive.bg_fill = surface;
        style.visuals.widgets.inactive.bg_fill = surface;
        style.visuals.widgets.hovered.bg_fill =
            egui::Color32::from_rgba_premultiplied(primary.r(), primary.g(), primary.b(), 20);
        style.visuals.widgets.active.bg_fill =
            egui::Color32::from_rgba_premultiplied(primary.r(), primary.g(), primary.b(), 40);
        style.visuals.selection.bg_fill = primary;
        style.visuals.widgets.noninteractive.fg_stroke.color = on_surface;
        style.visuals.widgets.inactive.fg_stroke.color = on_surface;
        style.visuals.widgets.hovered.fg_stroke.color = on_surface;
        style.visuals.widgets.active.fg_stroke.color = on_surface;
        style.visuals.striped = true;
        style.visuals.faint_bg_color = egui::Color32::from_rgba_premultiplied(
            on_surface.r(),
            on_surface.g(),
            on_surface.b(),
            10,
        );
        ui.ctx().set_style(style);

        // Build table
        let mut debloat_table = data_table()
            .id(egui::Id::new(format!(
                "debloat_data_table_v{}",
                self.table_version
            )))
            .sortable_column(tr!("col-package-name"), 350.0, false)
            .sortable_column(tr!("col-debloat-category"), 130.0, false)
            .sortable_column("RP", 80.0, true)
            .sortable_column(tr!("col-enabled"), 120.0, false)
            .sortable_column(tr!("col-install-reason"), 110.0, false)
            .sortable_column(tr!("col-tasks"), 160.0, false)
            .allow_selection(true);

        if let Some(sort_col) = self.sort_column {
            use egui_material3::SortDirection;
            let direction = if self.sort_ascending {
                SortDirection::Ascending
            } else {
                SortDirection::Descending
            };
            debloat_table = debloat_table.sort_by(sort_col, direction);
        }

        let mut filtered_package_names = Vec::new();

        // Collect filtered packages info first to avoid borrow issues
        let filtered_packages: Vec<(usize, String, String, bool, String, String, String, String, bool)> = self
            .installed_packages
            .iter()
            .enumerate()
            .filter(|(_, p)| self.should_show_package(p))
            .map(|(idx, package)| {
                let is_system = package.flags.contains("SYSTEM");
                let package_name = format!("{} ({})", package.pkg, package.versionName);
                let debloat_category = if let Some(uad_ng_lists) = &self.uad_ng_lists {
                    uad_ng_lists
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

        for (idx, pkg_id, package_name, is_system, debloat_category, runtime_perms, enabled_text, install_reason, is_selected) in filtered_packages {
            filtered_package_names.push(pkg_id.clone());

            let clicked_idx_clone = clicked_package_idx.clone();
            let pkg_id_clone = pkg_id.clone();
            let package_name_clone = package_name.clone();
            let enabled_str = enabled_text.clone();
            let debloat_category_clone = debloat_category.clone();

            // Get cached app info for this package
            let fd_cached = self.cached_fdroid_apps.get(&pkg_id).cloned();
            let gp_cached = self.cached_google_play_apps.get(&pkg_id).cloned();
            let am_cached = self.cached_apkmirror_apps.get(&pkg_id).cloned();

            // Prepare texture data
            let (fd_texture, fd_title, fd_developer) = if !is_system && fdroid_enabled {
                if let Some(ref fd_app) = fd_cached {
                    if fd_app.raw_response != "404" {
                        let tex = fd_app.icon_base64.as_ref().and_then(|icon| {
                            Self::load_texture_from_base64(ui.ctx(), &mut self.fdroid_textures, "fdroid", &pkg_id, icon)
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
                if let Some(ref gp_app) = gp_cached {
                    if gp_app.raw_response != "404" {
                        let tex = gp_app.icon_base64.as_ref().and_then(|icon| {
                            Self::load_texture_from_base64(ui.ctx(), &mut self.google_play_textures, "googleplay", &pkg_id, icon)
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
                if let Some(ref am_app) = am_cached {
                    if am_app.raw_response != "404" {
                        let tex = am_app.icon_base64.as_ref().and_then(|icon| {
                            Self::load_texture_from_base64(ui.ctx(), &mut self.apkmirror_textures, "apkmirror", &pkg_id, icon)
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
                let mut row_builder = if let (Some(title), Some(developer)) = (fd_title.clone(), fd_developer.clone()) {
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        ui.horizontal(|ui| {
                            if let Some(tex_id) = fd_texture {
                                ui.image((tex_id, egui::vec2(38.0, 38.0)));
                            }
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 0.1;
                                ui.label(egui::RichText::new(&title).strong());
                                ui.label(egui::RichText::new(&developer).small().color(egui::Color32::GRAY));
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
                                ui.label(egui::RichText::new(&title).strong());
                                ui.label(egui::RichText::new(&developer).small().color(egui::Color32::GRAY));
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
                                ui.label(egui::RichText::new(&title).strong());
                                ui.label(egui::RichText::new(&developer).small().color(egui::Color32::GRAY));
                            });
                        });
                    })
                } else {
                    // No app info available, show plain package name (no spinner)
                    let pkg_name = package_name_clone.clone();
                    table_row.widget_cell(move |ui: &mut egui::Ui| {
                        ui.add(egui::Label::new(&pkg_name).wrap());
                    })
                };

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

                // Tasks column
                let pkg_id_for_buttons = pkg_id_clone.clone();
                row_builder = row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    ui.horizontal(|ui| {
                        let chip = assist_chip("").leading_icon_svg(INFO_SVG).elevated(true);
                        if ui.add(chip.on_click(|| {})).clicked() {
                            if let Ok(mut clicked) = clicked_idx_clone.lock() {
                                *clicked = Some(idx);
                            }
                        }

                        if enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED") {
                            let uninstall_chip = assist_chip("").leading_icon_svg(TRASH_RED_SVG).elevated(true);
                            if ui.add(uninstall_chip.on_click(|| {})).clicked() {
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("uninstall_clicked_package"), pkg_id_for_buttons.clone());
                                    data.insert_temp(egui::Id::new("uninstall_clicked_is_system"), is_system);
                                });
                            }
                        }

                        if enabled_str.contains("REMOVED_USER") || enabled_str.contains("DISABLED_USER") || enabled_str.contains("DISABLED") {
                            let enable_chip = assist_chip("").leading_icon_svg(ENABLE_GREEN_SVG).elevated(true);
                            if ui.add(enable_chip.on_click(|| {})).clicked() {
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("enable_clicked_package"), pkg_id_for_buttons.clone());
                                });
                            }
                        }

                        if enabled_str.contains("DEFAULT") || enabled_str.contains("ENABLED") {
                            let disable_chip = assist_chip("").leading_icon_svg(DISABLE_RED_SVG).elevated(true);
                            if ui.add(disable_chip.on_click(|| {})).clicked() {
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("disable_clicked_package"), pkg_id_for_buttons.clone());
                                });
                            }
                        }
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

        // Sync sort state
        let (widget_sort_col, widget_sort_dir) = table_response.sort_state;
        let widget_sort_ascending = matches!(widget_sort_dir, egui_material3::SortDirection::Ascending);

        if widget_sort_col != self.sort_column
            || (widget_sort_col.is_some() && widget_sort_ascending != self.sort_ascending)
        {
            self.sort_column = widget_sort_col;
            self.sort_ascending = widget_sort_ascending;
            if self.sort_column.is_some() {
                self.sort_packages();
            }
        }

        if let Some(clicked_col) = table_response.column_clicked {
            if self.sort_column == Some(clicked_col) {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = Some(clicked_col);
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

        // Perform uninstall
        if let Some(pkg_name) = uninstall_package {
            if let Some(ref device) = self.selected_device {
                let uninstall_result = if uninstall_is_system {
                    crate::adb::uninstall_app_user(&pkg_name, device, None)
                } else {
                    crate::adb::uninstall_app(&pkg_name, device)
                };

                match uninstall_result {
                    Ok(output) => {
                        tracing::info!("App uninstalled successfully: {}", output);
                        let is_system = self.installed_packages.iter().find(|p| p.pkg == pkg_name).map(|p| p.flags.contains("SYSTEM")).unwrap_or(false);

                        if is_system {
                            if let Some(pkg) = self.installed_packages.iter_mut().find(|p| p.pkg == pkg_name) {
                                for user in pkg.users.iter_mut() {
                                    user.installed = false;
                                    user.enabled = 0;
                                }
                            }
                        } else {
                            self.installed_packages.retain(|pkg| pkg.pkg != pkg_name);
                            self.selected_packages.remove(&pkg_name);
                        }
                        result = Some(AdbResult::Success(pkg_name.clone()));
                    }
                    Err(e) => {
                        tracing::error!("Failed to uninstall app({}): {}", pkg_name, e);
                        result = Some(AdbResult::Failure);
                    }
                }
            } else {
                tracing::error!("No device selected for uninstall");
                result = Some(AdbResult::Failure);
            }
        }

        // Perform enable
        if let Some(pkg_name) = enable_package {
            if let Some(ref device) = self.selected_device {
                match crate::adb::enable_app(&pkg_name, device) {
                    Ok(output) => {
                        tracing::info!("App enabled successfully: {}", output);
                        if let Some(pkg) = self.installed_packages.iter_mut().find(|p| p.pkg == pkg_name) {
                            for user in pkg.users.iter_mut() {
                                user.enabled = 1;
                                user.installed = true;
                            }
                        }
                        result = Some(AdbResult::Success(pkg_name.clone()));
                    }
                    Err(e) => {
                        tracing::error!("Failed to enable app: {}", e);
                        result = Some(AdbResult::Failure);
                    }
                }
            } else {
                tracing::error!("No device selected for enable");
                result = Some(AdbResult::Failure);
            }
        }

        // Perform disable
        if let Some(pkg_name) = disable_package {
            if let Some(ref device) = self.selected_device {
                match crate::adb::disable_app_current_user(&pkg_name, device, None) {
                    Ok(output) => {
                        tracing::info!("App disabled successfully: {}", output);
                        if let Some(pkg) = self.installed_packages.iter_mut().find(|p| p.pkg == pkg_name) {
                            for user in pkg.users.iter_mut() {
                                user.enabled = 3;
                            }
                        }
                        result = Some(AdbResult::Success(pkg_name.clone()));
                    }
                    Err(e) => {
                        tracing::error!("Failed to disable app: {}", e);
                        result = Some(AdbResult::Failure);
                    }
                }
            } else {
                tracing::error!("No device selected for disable");
                result = Some(AdbResult::Failure);
            }
        }

        // Handle batch uninstall
        if batch_uninstall {
            if let Some(ref device) = self.selected_device {
                let packages_to_uninstall: Vec<String> = self.selected_packages.iter().cloned().collect();
                for pkg_name in packages_to_uninstall {
                    let is_system = self.installed_packages.iter().find(|p| p.pkg == pkg_name).map(|p| p.flags.contains("SYSTEM")).unwrap_or(false);
                    let uninstall_result = if is_system {
                        crate::adb::uninstall_app_user(&pkg_name, device, None)
                    } else {
                        crate::adb::uninstall_app(&pkg_name, device)
                    };

                    match uninstall_result {
                        Ok(output) => {
                            tracing::info!("App uninstalled successfully: {}", output);
                            if is_system {
                                if let Some(pkg) = self.installed_packages.iter_mut().find(|p| p.pkg == pkg_name) {
                                    for user in pkg.users.iter_mut() {
                                        user.installed = false;
                                        user.enabled = 0;
                                    }
                                }
                            } else {
                                self.installed_packages.retain(|pkg| pkg.pkg != pkg_name);
                                self.selected_packages.remove(&pkg_name);
                            }
                            result = Some(AdbResult::Success(pkg_name.clone()));
                        }
                        Err(e) => {
                            tracing::error!("Failed to uninstall app {}: {}", pkg_name, e);
                            result = Some(AdbResult::Failure);
                        }
                    }
                }
            } else {
                tracing::error!("No device selected for batch uninstall");
                result = Some(AdbResult::Failure);
            }
        }

        // Handle batch disable
        if batch_disable {
            if let Some(ref device) = self.selected_device {
                let packages_to_disable: Vec<String> = self.selected_packages.iter().cloned().collect();
                for pkg_name in packages_to_disable {
                    match crate::adb::disable_app_current_user(&pkg_name, device, None) {
                        Ok(output) => {
                            tracing::info!("App disabled successfully: {}", output);
                            if let Some(pkg) = self.installed_packages.iter_mut().find(|p| p.pkg == pkg_name) {
                                for user in pkg.users.iter_mut() {
                                    user.enabled = 3;
                                }
                            }
                            result = Some(AdbResult::Success(pkg_name.clone()));
                        }
                        Err(e) => {
                            tracing::error!("Failed to disable app {}: {}", pkg_name, e);
                            result = Some(AdbResult::Failure);
                        }
                    }
                }
            } else {
                tracing::error!("No device selected for batch disable");
                result = Some(AdbResult::Failure);
            }
        }

        // Handle batch enable
        if batch_enable {
            if let Some(ref device) = self.selected_device {
                let packages_to_enable: Vec<String> = self.selected_packages.iter().cloned().collect();
                for pkg_name in packages_to_enable {
                    match crate::adb::enable_app(&pkg_name, device) {
                        Ok(output) => {
                            tracing::info!("App enabled successfully: {}", output);
                            if let Some(pkg) = self.installed_packages.iter_mut().find(|p| p.pkg == pkg_name) {
                                for user in pkg.users.iter_mut() {
                                    user.enabled = 1;
                                    user.installed = true;
                                }
                            }
                            result = Some(AdbResult::Success(pkg_name.clone()));
                        }
                        Err(e) => {
                            tracing::error!("Failed to enable app {}: {}", pkg_name, e);
                            result = Some(AdbResult::Failure);
                        }
                    }
                }
            } else {
                tracing::error!("No device selected for batch enable");
                result = Some(AdbResult::Failure);
            }
        }

        // Show package details dialog
        self.package_details_dialog.show(ui.ctx(), &self.installed_packages, &self.uad_ng_lists);

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

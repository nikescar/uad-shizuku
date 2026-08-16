//! Mobile-optimized scan table component (VirusTotal / HybridAnalysis).
//!
//! 3 columns: Name/Status + Scan Result (chips) + Tasks. Modeled on
//! `dlg_mobile_risk::components::package_table_mobile`, with an extra Scan Result column since
//! VT/HA results are interactive colored chips (click-to-open-report, hover tooltip, multiple
//! per row for multi-file packages) that don't compress into a text line.

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use egui_i18n::tr;

use crate::adb_stt::PackageFingerprint;
use crate::calc_hybridanalysis_stt::ScannerState as HaScannerState;
use crate::calc_virustotal_stt::ScannerState as VtScannerState;
use crate::dlg_mobile_scan::{is_virustotal, ScanCategory};
use crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::tab_debloat::components::package_table_mobile::AppDisplayData;
use crate::uad_shizuku_app::UadNgLists;
use egui_material3::icon_button_standard;

const ROW_HEIGHT: f32 = 56.0;
const SCAN_RESULT_COLUMN_WIDTH: f32 = 260.0;
const TASKS_COLUMN_WIDTH: f32 = 200.0;
const MOBILE_BUTTON_SPACING: f32 = 16.0;
const MOBILE_TOUCH_TARGET: f32 = 40.0;

fn is_row_enabled(package: &PackageFingerprint) -> bool {
    package.users.first().map_or(false, |user| {
        let enabled = user.enabled;
        let installed = user.installed;
        let is_system = package.flags.contains("SYSTEM");
        !(enabled == 0 && !installed && is_system || enabled == 2 || enabled == 3)
    })
}

/// Whether the delete/uninstall button should show for this package, gated by the
/// Unsafe/Expert removal toggles — mirrors `render_action_buttons_static`'s gating
/// (dlg_dashcounter_details.rs:806-808).
fn show_delete_button(
    pkg_id: &str,
    uad_ng_lists: Option<&UadNgLists>,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
) -> bool {
    let category = uad_ng_lists
        .and_then(|lists| lists.apps.get(pkg_id))
        .map(|app| app.removal.as_str());

    match category {
        Some("Unsafe") => unsafe_app_remove,
        Some("Expert") => expert_app_remove,
        _ => true,
    }
}

/// Get HybridAnalysis display text (ported from `DlgDashCounterDetails::get_ha_display_text`,
/// dlg_dashcounter_details.rs:376-497).
fn get_ha_display_text(file_result: &crate::calc_hybridanalysis_stt::FileScanResult) -> String {
    if file_result.verdict == "upload_error" || file_result.verdict == "analysis_error" {
        if let Some(ref error_msg) = file_result.error_message {
            if error_msg.contains("File too large") {
                if let Some(mb_pos) = error_msg.find(" MB ") {
                    if let Some(start) =
                        error_msg[..mb_pos].rfind(|c: char| !c.is_numeric() && c != '.')
                    {
                        let size = &error_msg[start + 1..mb_pos + 3];
                        return tr!("ha-file-too-large", { size: size.to_string() });
                    } else {
                        return tr!("ha-file-too-large-default");
                    }
                } else {
                    return tr!("ha-file-too-large-default");
                }
            } else if error_msg.contains("No such file or directory") {
                return tr!("ha-pull-failed");
            } else if error_msg.contains("Failed to create tmp directory") {
                return tr!("ha-temp-dir-error");
            } else if file_result.verdict == "upload_error" {
                return tr!("ha-upload-error");
            } else {
                return tr!("ha-analysis-error");
            }
        } else if file_result.verdict == "upload_error" {
            return tr!("ha-upload-error");
        } else {
            return tr!("ha-analysis-error");
        }
    }

    let has_tags = !file_result.classification_tags.is_empty();
    let base_text = if has_tags {
        let tags_str = file_result.classification_tags.join(", ");
        match file_result.verdict.as_str() {
            "malicious" => tr!("ha-malicious-tags", { tags: tags_str }),
            "suspicious" => tr!("ha-suspicious-tags", { tags: tags_str }),
            "whitelisted" => tr!("ha-whitelisted-tags", { tags: tags_str }),
            "no specific threat" => tr!("ha-no-specific-threat-tags", { tags: tags_str }),
            _ => match file_result.verdict.as_str() {
                "no-result" => tr!("ha-no-result"),
                "rate_limited" => tr!("ha-rate-limited"),
                "submitted" => tr!("ha-submitted"),
                "pending_analysis" => tr!("ha-pending-analysis"),
                "404 Not Found" => tr!("ha-404"),
                "" => tr!("ha-skipped"),
                _ => file_result.verdict.clone(),
            },
        }
    } else if let Some(score) = file_result.threat_score {
        match file_result.verdict.as_str() {
            "malicious" => tr!("ha-malicious-score", { score: score }),
            "suspicious" => tr!("ha-suspicious-score", { score: score }),
            "whitelisted" => tr!("ha-whitelisted-score", { score: score }),
            "no specific threat" => tr!("ha-no-specific-threat-score", { score: score }),
            _ => match file_result.verdict.as_str() {
                "no-result" => tr!("ha-no-result"),
                "rate_limited" => tr!("ha-rate-limited"),
                "submitted" => tr!("ha-submitted"),
                "pending_analysis" => tr!("ha-pending-analysis"),
                "404 Not Found" => tr!("ha-404"),
                "" => tr!("ha-skipped"),
                _ => file_result.verdict.clone(),
            },
        }
    } else {
        match file_result.verdict.as_str() {
            "malicious" => tr!("ha-malicious"),
            "suspicious" => tr!("ha-suspicious"),
            "whitelisted" => tr!("ha-whitelisted"),
            "no specific threat" => tr!("ha-no-specific-threat"),
            "no-result" => tr!("ha-no-result"),
            "rate_limited" => tr!("ha-rate-limited"),
            "submitted" => tr!("ha-submitted"),
            "pending_analysis" => {
                if let Some(ref job_id) = file_result.job_id {
                    let short_id = if job_id.len() > 8 { &job_id[..8] } else { job_id };
                    tr!("ha-pending", { jobid: short_id.to_string() })
                } else {
                    tr!("ha-pending-analysis")
                }
            }
            "404 Not Found" => tr!("ha-404"),
            "" => tr!("ha-skipped"),
            _ => file_result.verdict.clone(),
        }
    };

    if let Some(wait_until) = file_result.wait_until {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if wait_until > now {
            let remaining_secs = wait_until - now;
            let hours = remaining_secs / 3600;
            let mins = (remaining_secs % 3600) / 60;
            if hours > 0 {
                tr!("ha-wait-hours", { text: base_text, hours: hours, mins: mins })
            } else if mins > 0 {
                tr!("ha-wait-mins", { text: base_text, mins: mins })
            } else {
                tr!("ha-wait-less-than-min", { text: base_text })
            }
        } else {
            base_text
        }
    } else {
        base_text
    }
}

/// Renders the VirusTotal chip row for one package. Ported from
/// `DlgDashCounterDetails::render_vt_cell` (dlg_dashcounter_details.rs:190-265), converted from
/// a `DataTableCell::widget` closure to a direct `ui` call (TableBuilder's `row.col` already
/// hands the cell its own `ui`).
fn render_vt_chips(
    ui: &mut egui::Ui,
    vt_result: Option<&crate::calc_virustotal_stt::ScanStatus>,
    idx: usize,
) {
    egui::ScrollArea::horizontal()
        .id_salt(format!("vt_scroll_mobile_{}", idx))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                match vt_result {
                    None => {
                        ui.label(tr!("scan-not-initialized"));
                    }
                    Some(crate::calc_virustotal_stt::ScanStatus::Pending) => {
                        ui.label(tr!("scan-not-scanned"));
                    }
                    Some(crate::calc_virustotal_stt::ScanStatus::Scanning { scanned, total, .. }) => {
                        ui.label(tr!("scan-scanning", { scanned: scanned, total: total }));
                    }
                    Some(crate::calc_virustotal_stt::ScanStatus::Completed(result)) => {
                        for (i, file_result) in result.file_results.iter().enumerate() {
                            let (text, bg_color) = if file_result.error.is_some() {
                                (tr!("scan-error"), egui::Color32::from_rgb(211, 47, 47))
                            } else if file_result.skipped {
                                (tr!("scan-skip"), egui::Color32::from_rgb(128, 128, 128))
                            } else if file_result.not_found {
                                (tr!("scan-404"), egui::Color32::from_rgb(128, 128, 128))
                            } else if file_result.malicious > 0 {
                                (
                                    tr!("scan-malicious", { count: file_result.malicious + file_result.suspicious, total: file_result.total() }),
                                    egui::Color32::from_rgb(211, 47, 47),
                                )
                            } else if file_result.suspicious > 0 {
                                (
                                    tr!("scan-suspicious", { count: file_result.suspicious, total: file_result.total() }),
                                    egui::Color32::from_rgb(255, 152, 0),
                                )
                            } else {
                                (
                                    tr!("scan-clean", { count: file_result.total(), total: file_result.total() }),
                                    egui::Color32::from_rgb(56, 142, 60),
                                )
                            };

                            let inner_response = egui::Frame::new()
                                .fill(bg_color)
                                .corner_radius(8.0)
                                .inner_margin(egui::Margin::symmetric(12, 6))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(12.0))
                                });

                            let response = ui.interact(
                                inner_response.response.rect,
                                ui.id().with(format!("vt_chip_mobile_{}_{}", idx, i)),
                                egui::Sense::click(),
                            );

                            if let Some(ref err) = file_result.error {
                                response.on_hover_text(format!("{}\n{}", file_result.file_path, err));
                            } else {
                                if response.clicked() {
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        if let Err(err) = webbrowser::open(&file_result.vt_link) {
                                            log::error!("Failed to open VirusTotal link: {}", err);
                                        }
                                    }
                                }
                                response.on_hover_text(&file_result.file_path);
                            }
                        }
                    }
                    Some(crate::calc_virustotal_stt::ScanStatus::Error(e)) => {
                        ui.label(tr!("scan-error-msg", { message: e.clone() }));
                    }
                }
            });
        });
}

/// Renders the HybridAnalysis chip row for one package. Ported from
/// `DlgDashCounterDetails::render_ha_cell` (dlg_dashcounter_details.rs:268-373).
fn render_ha_chips(
    ui: &mut egui::Ui,
    ha_result: Option<&crate::calc_hybridanalysis_stt::ScanStatus>,
    idx: usize,
    ha_tag_ignorelist: &str,
) {
    egui::ScrollArea::horizontal()
        .id_salt(format!("ha_scroll_mobile_{}", idx))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                match ha_result {
                    None => {
                        ui.label(tr!("scan-not-initialized"));
                    }
                    Some(crate::calc_hybridanalysis_stt::ScanStatus::Pending) => {
                        ui.label(tr!("scan-not-scanned"));
                    }
                    Some(crate::calc_hybridanalysis_stt::ScanStatus::Scanning { scanned, total, .. }) => {
                        ui.label(tr!("scan-scanning", { scanned: scanned, total: total }));
                    }
                    Some(crate::calc_hybridanalysis_stt::ScanStatus::Completed(result)) => {
                        if result.file_results.is_empty() {
                            ui.label(tr!("scan-no-results"));
                        }
                        for (i, file_result) in result.file_results.iter().enumerate() {
                            let text = get_ha_display_text(file_result);

                            let ignorelist_tags: Vec<String> = ha_tag_ignorelist
                                .split(',')
                                .map(|s| s.trim().to_lowercase())
                                .filter(|s| !s.is_empty())
                                .collect();

                            let all_tags_ignored = if file_result.classification_tags.is_empty() {
                                true
                            } else {
                                file_result
                                    .classification_tags
                                    .iter()
                                    .all(|tag| ignorelist_tags.contains(&tag.to_lowercase()))
                            };

                            let bg_color = match file_result.verdict.as_str() {
                                "malicious" => {
                                    if all_tags_ignored {
                                        egui::Color32::from_rgb(128, 128, 128)
                                    } else {
                                        egui::Color32::from_rgb(211, 47, 47)
                                    }
                                }
                                "suspicious" => egui::Color32::from_rgb(255, 152, 0),
                                "whitelisted" => egui::Color32::from_rgb(56, 142, 60),
                                "no specific threat" => egui::Color32::from_rgb(0, 150, 136),
                                "no-result" => egui::Color32::from_rgb(158, 158, 158),
                                "rate_limited" => egui::Color32::from_rgb(156, 39, 176),
                                "submitted" => egui::Color32::from_rgb(33, 150, 243),
                                "pending_analysis" => egui::Color32::from_rgb(33, 150, 243),
                                "upload_error" | "analysis_error" => egui::Color32::from_rgb(211, 47, 47),
                                "404 Not Found" => egui::Color32::from_rgb(158, 158, 158),
                                "" => egui::Color32::from_rgb(158, 158, 158),
                                _ => egui::Color32::from_rgb(158, 158, 158),
                            };

                            let inner_response = egui::Frame::new()
                                .fill(bg_color)
                                .corner_radius(8.0)
                                .inner_margin(egui::Margin::symmetric(12, 6))
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(&text).color(egui::Color32::WHITE).size(12.0))
                                });

                            let response = ui.interact(
                                inner_response.response.rect,
                                ui.id().with(format!("ha_chip_mobile_{}_{}", idx, i)),
                                egui::Sense::click(),
                            );

                            if let Some(ref error_msg) = file_result.error_message {
                                response.on_hover_text(format!("{}\n{}", file_result.file_path, error_msg));
                            } else {
                                if response.clicked() {
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        if !file_result.ha_link.is_empty() {
                                            if let Err(err) = webbrowser::open(&file_result.ha_link) {
                                                log::error!("Failed to open HybridAnalysis link: {}", err);
                                            }
                                        }
                                    }
                                }
                                response.on_hover_text(&file_result.file_path);
                            }
                        }
                    }
                    Some(crate::calc_hybridanalysis_stt::ScanStatus::Error(e)) => {
                        ui.label(tr!("scan-error-msg", { message: e.clone() }));
                    }
                }
            });
        });
}

#[allow(clippy::too_many_arguments)]
pub fn render_scan_table_mobile(
    ui: &mut egui::Ui,
    packages: &[&PackageFingerprint],
    category: &ScanCategory,
    vt_scanner_state: Option<&VtScannerState>,
    ha_scanner_state: Option<&HaScannerState>,
    hybridanalysis_tag_ignorelist: &str,
    uad_ng_lists: Option<&UadNgLists>,
    app_display_data: &AppDisplayData,
    unsafe_app_remove: bool,
    expert_app_remove: bool,
    on_info_clicked: &mut dyn FnMut(&str),
    on_toggle_clicked: &mut dyn FnMut(&str, bool),
    on_delete_clicked: &mut dyn FnMut(&str),
) {
    let show_vt = is_virustotal(category);
    let header_label = if show_vt {
        tr!("col-virustotal")
    } else {
        tr!("col-hybrid-analysis")
    };

    TableBuilder::new(ui)
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::remainder())
        .column(Column::exact(SCAN_RESULT_COLUMN_WIDTH))
        .column(Column::exact(TASKS_COLUMN_WIDTH))
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.label("Name");
            });
            header.col(|ui| {
                ui.label(header_label);
            });
            header.col(|ui| {
                ui.label("Tasks");
            });
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, packages.len(), |mut row| {
                let row_idx = row.index();
                let package = packages[row_idx];

                // Column 1: Name/Status
                row.col(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let (texture_handle, app_title) = app_display_data
                            .get(&package.pkg)
                            .map(|(tex, title)| (tex.as_ref(), Some(title.as_str())))
                            .unwrap_or((None, None));

                        if let Some(tex) = texture_handle {
                            ui.image((tex.id(), egui::vec2(38.0, 38.0)));
                        }

                        ui.vertical(|ui| {
                            ui.style_mut().spacing.item_spacing.y = 2.0;

                            if let Some(title) = app_title {
                                let text_color = ui.style().visuals.text_color();
                                ui.label(egui::RichText::new(title).strong().color(text_color));
                                ui.label(egui::RichText::new(&package.pkg).small().weak());
                            } else {
                                ui.label(&package.pkg);
                            }

                            let (status_text, status_color) = if package.users.is_empty() {
                                ("Uninstalled", egui::Color32::from_rgb(128, 128, 128))
                            } else {
                                let user = &package.users[0];
                                let is_system = package.flags.contains("SYSTEM");
                                if user.enabled == 0 && !user.installed && is_system {
                                    ("Removed", egui::Color32::from_rgb(158, 158, 158))
                                } else if user.enabled == 2 {
                                    ("Disabled", egui::Color32::from_rgb(211, 47, 47))
                                } else if user.enabled == 3 {
                                    ("Disabled-User", egui::Color32::from_rgb(244, 67, 54))
                                } else {
                                    ("Enabled", egui::Color32::from_rgb(56, 142, 60))
                                }
                            };
                            ui.label(egui::RichText::new(status_text).color(status_color));
                        });
                    });
                });

                // Column 2: Scan Result (VT or HA chips)
                row.col(|ui| {
                    if show_vt {
                        let vt_status = vt_scanner_state
                            .and_then(|state| state.lock().ok())
                            .and_then(|locked| locked.get(&package.pkg).cloned());
                        render_vt_chips(ui, vt_status.as_ref(), row_idx);
                    } else {
                        let ha_status = ha_scanner_state
                            .and_then(|state| state.lock().ok())
                            .and_then(|locked| locked.get(&package.pkg).cloned());
                        render_ha_chips(ui, ha_status.as_ref(), row_idx, hybridanalysis_tag_ignorelist);
                    }
                });

                // Column 3: Tasks (info / toggle / uninstall)
                row.col(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = MOBILE_BUTTON_SPACING;
                        ui.style_mut().spacing.interact_size =
                            egui::vec2(MOBILE_TOUCH_TARGET, MOBILE_TOUCH_TARGET);

                        if ui
                            .add(icon_button_standard(ICON_INFO.to_string()))
                            .on_hover_text("Package details")
                            .clicked()
                        {
                            on_info_clicked(&package.pkg);
                        }

                        let is_enabled = is_row_enabled(package);
                        let toggle_icon = if is_enabled { ICON_TOGGLE_ON } else { ICON_TOGGLE_OFF };
                        let toggle_text = if is_enabled { "Disable" } else { "Enable" };

                        if ui
                            .add(icon_button_standard(toggle_icon.to_string()))
                            .on_hover_text(toggle_text)
                            .clicked()
                        {
                            on_toggle_clicked(&package.pkg, is_enabled);
                        }

                        if show_delete_button(&package.pkg, uad_ng_lists, unsafe_app_remove, expert_app_remove) {
                            if ui
                                .add(icon_button_standard(ICON_DELETE.to_string()))
                                .on_hover_text("Uninstall package")
                                .clicked()
                            {
                                on_delete_clicked(&package.pkg);
                            }
                        }
                    });
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(ROW_HEIGHT, 56.0);
        assert_eq!(SCAN_RESULT_COLUMN_WIDTH, 260.0);
        assert_eq!(TASKS_COLUMN_WIDTH, 200.0);
        assert_eq!(MOBILE_BUTTON_SPACING, 16.0);
        assert_eq!(MOBILE_TOUCH_TARGET, 40.0);
    }

    #[test]
    fn test_show_delete_button_defaults_true_with_no_uad_lists() {
        assert!(show_delete_button("com.example.app", None, false, false));
        assert!(show_delete_button("com.example.app", None, true, true));
    }
}

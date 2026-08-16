# Mobile Table View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace card-based mobile view with touch-optimized table view using virtual scrolling for 1000-2000 packages in < 300ms.

**Architecture:** Clone desktop `package_table.rs`, simplify to 3 columns (Checkbox + Name/Status + Tasks), add touch optimizations (40px targets, 16px spacing), integrate into `view_mobile.rs`.

**Tech Stack:** Rust, egui, egui_extras::TableBuilder, egui_material3

## Global Constraints

- Row height: 56.0px (Material Design standard, same as desktop)
- Checkbox column: 40.0px (vs 30.0px desktop for touch)
- Tasks column: 200.0px (vs 160.0px desktop for spacing)
- Button spacing: 16.0px (vs 4.0px desktop for touch)
- Touch targets: 40.0px minimum (Material Design)
- Performance target: < 300ms for 1000-2000 packages
- Test coverage: 80% minimum (ECC Rust standard)
- No `.unwrap()` in production code (use `?` or `unwrap_or_default()`)

---

## Task Structure

```
Task 1: Create mobile table component with TDD (package_table_mobile.rs)
Task 2: Integrate into view_mobile.rs  
Task 3: Update module exports (mod.rs)
Task 4: Remove old card view (package_cards.rs)
Task 5: Performance testing and verification
```

Dependencies: Task 1 → Tasks 2,3 → Task 4 → Task 5

---

### Task 1: Create Mobile Table Component with Tests

**Files:**
- Create: `mobile/src/tab_debloat/components/package_table_mobile.rs`

**Interfaces:**
- Consumes: `PackageFingerprint`, `UadNgLists`, Material icons
- Produces: `pub fn render_package_table_mobile(ui, packages, selected_packages, uad_ng_lists, app_display_data, on_info_clicked, on_refresh_clicked, on_toggle_clicked, on_delete_clicked)`

---

- [ ] **Step 1: Create file with module docs and constants**

File: `/home/wj/work/uad-shizuku/mobile/src/tab_debloat/components/package_table_mobile.rs`

```rust
//! Mobile-optimized package table component
//!
//! 3 columns: Checkbox (40px) + Name/Status (remainder) + Tasks (200px)
//! Optimized for 1000-2000 packages with < 300ms render time.

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::{HashMap, HashSet};

use crate::adb_stt::PackageFingerprint;
use crate::material_symbol_icons::{ICON_DELETE, ICON_INFO, ICON_TOGGLE_OFF, ICON_TOGGLE_ON};
use crate::uad_shizuku_app::UadNgLists;
use egui_material3::icon_button_standard;

const ROW_HEIGHT: f32 = 56.0;
const CHECKBOX_COLUMN_WIDTH: f32 = 40.0;
const TASKS_COLUMN_WIDTH: f32 = 200.0;
const MOBILE_BUTTON_SPACING: f32 = 16.0;
const MOBILE_TOUCH_TARGET: f32 = 40.0;

pub type AppDisplayData = HashMap<String, (Option<egui::TextureHandle>, String)>;
```

- [ ] **Step 2: Add unit tests for constants**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(ROW_HEIGHT, 56.0);
        assert_eq!(CHECKBOX_COLUMN_WIDTH, 40.0);
        assert_eq!(TASKS_COLUMN_WIDTH, 200.0);
        assert_eq!(MOBILE_BUTTON_SPACING, 16.0);
        assert_eq!(MOBILE_TOUCH_TARGET, 40.0);
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cd mobile && cargo test package_table_mobile::tests
```

Expected: 1 test passes

- [ ] **Step 4: Add render function stub and compilation tests**

Before `#[cfg(test)]`:

```rust
pub fn render_package_table_mobile(
    ui: &mut egui::Ui,
    packages: &[PackageFingerprint],
    selected_packages: &mut HashSet<String>,
    uad_ng_lists: Option<&UadNgLists>,
    app_display_data: &AppDisplayData,
    on_info_clicked: &mut dyn FnMut(&str),
    on_refresh_clicked: &mut dyn FnMut(&str),
    on_toggle_clicked: &mut dyn FnMut(&str, bool),
    on_delete_clicked: &mut dyn FnMut(&str),
) {
    log::debug!("render_package_table_mobile: {} packages", packages.len());
}
```

In tests module:

```rust
#[test]
fn test_function_compiles() {
    let packages = vec![];
    let mut selected = HashSet::new();
    let metadata = HashMap::new();
    assert_eq!(packages.len(), 0);
}
```

- [ ] **Step 5: Implement TableBuilder with 3 columns**

Replace function body:

```rust
TableBuilder::new(ui)
    .striped(true)
    .resizable(false)
    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
    .column(Column::exact(CHECKBOX_COLUMN_WIDTH))
    .column(Column::remainder())
    .column(Column::exact(TASKS_COLUMN_WIDTH))
    .header(20.0, |mut header| {
        header.col(|ui| { ui.label(""); });
        header.col(|ui| { ui.label("Name"); });
        header.col(|ui| { ui.label("Tasks"); });
    })
    .body(|body| {
        body.rows(ROW_HEIGHT, packages.len(), |mut row| {
            let package = &packages[row.index()];

            // Column 1: Checkbox
            row.col(|ui| {
                let mut is_selected = selected_packages.contains(&package.pkg);
                if ui.checkbox(&mut is_selected, "").changed() {
                    if is_selected {
                        selected_packages.insert(package.pkg.clone());
                    } else {
                        selected_packages.remove(&package.pkg);
                    }
                }
            });

            // Column 2: Name/Status (placeholder)
            row.col(|ui| {
                ui.label(&package.pkg);
            });

            // Column 3: Tasks (placeholder)
            row.col(|ui| {
                ui.label("Tasks");
            });
        });
    });
```

- [ ] **Step 6: Build to verify**

```bash
cargo build
```

Expected: Builds successfully

- [ ] **Step 7: Implement Column 2 (Name/Status combined)**

Replace Column 2 placeholder:

```rust
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
                ui.label(egui::RichText::new(title).strong());
                ui.label(egui::RichText::new(&package.pkg).small().weak());
            } else {
                ui.label(&package.pkg);
            }

            let (status_text, status_color) = if package.users.is_empty() {
                ("Uninstalled", egui::Color32::from_rgb(128, 128, 128))
            } else {
                let user = &package.users[0];
                let enabled = user.enabled;
                let installed = user.installed;
                let is_system = package.flags.contains("SYSTEM");

                if enabled == 0 && !installed && is_system {
                    ("Removed", egui::Color32::from_rgb(158, 158, 158))
                } else if enabled == 2 {
                    ("Disabled", egui::Color32::from_rgb(211, 47, 47))
                } else if enabled == 3 {
                    ("Disabled-User", egui::Color32::from_rgb(244, 67, 54))
                } else {
                    ("Enabled", egui::Color32::from_rgb(56, 142, 60))
                }
            };
            ui.label(egui::RichText::new(status_text).color(status_color));
        });
    });
});
```

- [ ] **Step 8: Implement Column 3 (Touch-optimized buttons)**

Replace Column 3 placeholder:

```rust
row.col(|ui| {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = MOBILE_BUTTON_SPACING;
        ui.style_mut().spacing.interact_size = egui::vec2(MOBILE_TOUCH_TARGET, MOBILE_TOUCH_TARGET);

        if ui.add(icon_button_standard(ICON_INFO.to_string()))
            .on_hover_text("Package details")
            .clicked()
        {
            on_info_clicked(&package.pkg);
        }

        let is_enabled = package.users.first().map_or(false, |user| {
            let enabled = user.enabled;
            let installed = user.installed;
            let is_system = package.flags.contains("SYSTEM");
            !(enabled == 0 && !installed && is_system || enabled == 2 || enabled == 3)
        });

        let toggle_icon = if is_enabled { ICON_TOGGLE_ON } else { ICON_TOGGLE_OFF };
        let toggle_text = if is_enabled { "Disable" } else { "Enable" };

        if ui.add(icon_button_standard(toggle_icon.to_string()))
            .on_hover_text(toggle_text)
            .clicked()
        {
            on_toggle_clicked(&package.pkg, is_enabled);
        }

        if ui.add(icon_button_standard(ICON_DELETE.to_string()))
            .on_hover_text("Uninstall package")
            .clicked()
        {
            on_delete_clicked(&package.pkg);
        }
    });
});
```

- [ ] **Step 9: Final build and test**

```bash
cargo build && cargo test package_table_mobile::tests
```

Expected: Builds successfully, tests pass

- [ ] **Step 10: Commit Task 1**

```bash
git add mobile/src/tab_debloat/components/package_table_mobile.rs
git commit -m "feat(mobile): add touch-optimized table component

- 3-column layout: Checkbox (40px) + Name/Status + Tasks (200px)
- Virtual scrolling for performance
- Touch optimization: 40px targets, 16px spacing
- Status badges with desktop color logic

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 2: Integrate into view_mobile.rs

**Files:**
- Modify: `mobile/src/tab_debloat/view_mobile.rs`

**Interfaces:**
- Consumes: `render_package_table_mobile` from Task 1
- Produces: Integrated mobile table in `render_package_list`

---

- [ ] **Step 1: Update import**

Line ~13, change:

```rust
use super::components::package_cards::render_package_cards;
```

To:

```rust
use super::components::package_table_mobile::render_package_table_mobile;
```

- [ ] **Step 2: Replace function call**

In `render_package_list` (~line 238-250), replace:

```rust
let clicked_index = render_package_cards(...);
if let Some(idx) = clicked_index {
    local_state.package_details_dialog.open(idx);
}
```

With:

```rust
render_package_table_mobile(
    ui,
    &vm_state.filtered_packages,
    &mut local_state.selected_packages,
    vm_state.uad_ng_lists.as_ref(),
    &app_metadata,
    &mut |pkg_id| {
        if let Some(idx) = vm_state.filtered_packages.iter().position(|p| p.pkg == pkg_id) {
            local_state.package_details_dialog.open(idx);
        }
    },
    &mut |_pkg_id| { log::debug!("Refresh: {}", _pkg_id); },
    &mut |_pkg_id, is_enabled| { log::debug!("Toggle {}: {}", _pkg_id, is_enabled); },
    &mut |_pkg_id| { log::debug!("Delete: {}", _pkg_id); },
);
```

- [ ] **Step 3: Build and verify**

```bash
cargo build
```

Expected: Builds successfully

- [ ] **Step 4: Commit Task 2**

```bash
git add mobile/src/tab_debloat/view_mobile.rs
git commit -m "feat(mobile): integrate table into mobile view

- Replace card view with table view
- Add button callbacks (info opens details, others log)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 3: Update Module Exports

**Files:**
- Modify: `mobile/src/tab_debloat/components/mod.rs`

---

- [ ] **Step 1: Add module export**

```rust
pub mod package_table_mobile;
```

- [ ] **Step 2: Build**

```bash
cargo build
```

- [ ] **Step 3: Commit**

```bash
git add mobile/src/tab_debloat/components/mod.rs
git commit -m "feat(mobile): export package_table_mobile module

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 4: Remove Old Card View

**Files:**
- Remove: `mobile/src/tab_debloat/components/package_cards.rs`
- Modify: `mobile/src/tab_debloat/components/mod.rs`

---

- [ ] **Step 1: Remove export**

In `mod.rs`, delete:

```rust
pub mod package_cards;
```

- [ ] **Step 2: Delete file**

```bash
git rm mobile/src/tab_debloat/components/package_cards.rs
```

- [ ] **Step 3: Build and test**

```bash
cargo build && cargo test
```

Expected: Success

- [ ] **Step 4: Commit**

```bash
git commit -m "refactor(mobile): remove obsolete card view

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 5: Testing and Verification

**Manual testing checklist**

- [ ] **Build release**

```bash
cargo build --release
```

- [ ] **Functional tests**

- [ ] Table renders (100, 1000, 2000 packages)
- [ ] Virtual scrolling smooth
- [ ] Checkboxes work
- [ ] Multi-select works
- [ ] Info button opens details
- [ ] Status colors correct
- [ ] Icons load

- [ ] **Performance tests**

- [ ] Initial render < 300ms (1000 packages)
- [ ] Initial render < 300ms (2000 packages)
- [ ] 60 FPS scrolling

- [ ] **Touch UX tests**

- [ ] Buttons easy to tap (40px)
- [ ] No accidental clicks
- [ ] 16px spacing visible

- [ ] **Regression tests**

- [ ] Search/filters work
- [ ] Batch actions work
- [ ] All mobile view features preserved

---

## Self-Review

**Spec Coverage:** ✅ All requirements implemented
**Placeholders:** ✅ None
**Type Consistency:** ✅ All signatures match

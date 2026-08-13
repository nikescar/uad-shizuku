# Debloat Tab Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor debloat tab with virtual scrolling, mobile/desktop views, and strict MVVM to fix lag and improve maintainability

**Architecture:** Split monolithic 2,592-line tab into focused modules: separate mobile/desktop views (pure rendering), centralized state, virtual scrolling component for performance, async filtering via DebloatActor

**Tech Stack:** Rust, egui, egui_extras::TableBuilder (virtual scrolling), smol async runtime, async-channel

## Global Constraints

- Minimum Rust 2021 edition
- Use smol async runtime (NOT tokio)
- egui for UI framework
- Target line count: < 500 lines per file
- Test coverage: 80% minimum (cargo llvm-cov)
- Desktop width threshold: 800px (`DESKTOP_MIN_WIDTH`)
- Row height: 24px for desktop, 48px for mobile
- Filter debounce: 300ms

---

**Note:** This plan focuses on Phase 1 (Debloat Tab) from the design spec. Subsequent phases (Scan, Apps tabs) will follow the same pattern.

## File Structure

```
mobile/src/
├── tab_debloat/                     # NEW: Module directory
│   ├── mod.rs                       # Entry point, width-based routing
│   ├── state.rs                     # UI state (selection, filters, dialogs)
│   ├── view_desktop.rs              # Desktop layout (sidebar + table)
│   ├── view_mobile.rs               # Mobile layout (stacked + cards)
│   └── components/
│       ├── mod.rs                   
│       ├── package_table.rs         # Virtual scrolling table
│       └── package_cards.rs         # Card list (mobile)
├── viewmodel/
│   ├── debloat.rs                   # Add FilterPackages command
│   └── mod.rs                       # Add filtered_packages state
```

---

### Task 1: Extend ViewModel with Filtering Support

**Files:**
- Modify: `mobile/src/viewmodel/debloat.rs`
- Modify: `mobile/src/viewmodel/mod.rs`
- Test: `mobile/tests/integration/debloat_filter_test.rs` (create)

**Interfaces:**
- Produces: `FilterPackages` command, `FilteredPackagesReady` event, `ViewModelState.filtered_packages` field

- [ ] Add `FilterPackages` and `SortPackages` to `DebloatCommand` enum
- [ ] Add `FilteredPackagesReady` event to `DebloatEvent` enum
- [ ] Add `filtered_packages: Vec<PackageFingerprint>` to `ViewModelState`
- [ ] Implement async filter handler in `DebloatActor`
- [ ] Handle `FilteredPackagesReady` in `ViewModel::poll_events`
- [ ] Write integration test verifying filter command flow
- [ ] Commit with message: "feat(viewmodel): add filter/sort commands and events"

---

### Task 2: Create tab_debloat Directory and State

**Files:**
- Create: `mobile/src/tab_debloat/mod.rs`
- Create: `mobile/src/tab_debloat/state.rs`
- Create: `mobile/src/tab_debloat/components/mod.rs`
- Modify: `mobile/src/lib.rs`

**Interfaces:**
- Produces: `TabDebloatState` struct, `TabDebloat` controller with width-based routing stub

- [ ] Create `tab_debloat/` directory structure
- [ ] Write `state.rs` with `TabDebloatState` (selection, filters, dialogs, error handling)
- [ ] Write `mod.rs` with `TabDebloat::render` doing width-based routing (800px threshold)
- [ ] Add `pub mod tab_debloat;` to `lib.rs`
- [ ] Build to verify compilation
- [ ] Commit with message: "feat(tab): create tab_debloat module structure"

---

### Task 3: Implement Virtual Scrolling Table

**Files:**
- Create: `mobile/src/tab_debloat/components/package_table.rs`
- Modify: `mobile/src/tab_debloat/components/mod.rs`

**Interfaces:**
- Consumes: `&[PackageFingerprint]`, `&mut HashSet<String>` (selected)
- Produces: `render_package_table(ui, packages, selected)` function using `egui_extras::TableBuilder`

- [ ] Implement `render_package_table` with virtual scrolling
- [ ] Use `TableBuilder::new(ui).body(|body| body.rows(24.0, len, |row| ...))` 
- [ ] Columns: checkbox (30px), name (remainder), category (100px), status (80px), actions (80px)
- [ ] Export from `components/mod.rs`
- [ ] Build to verify
- [ ] Commit with message: "feat(tab): add virtual scrolling package table"

---

### Task 4: Implement Desktop View

**Files:**
- Create: `mobile/src/tab_debloat/view_desktop.rs`
- Modify: `mobile/src/tab_debloat/mod.rs`

**Interfaces:**
- Consumes: `TabDebloatState`, `ViewModelState`, `render_package_table`
- Produces: `view_desktop::render(ui, vm_state, local_state)` function

- [ ] Create `view_desktop.rs` with sidebar layout (200px left panel)
- [ ] Render filter sidebar: category filters, options, advanced checkboxes
- [ ] Render main content: search bar, batch actions, error banner, virtual table
- [ ] Update `mod.rs` to call `view_desktop::render` when width >= 800px
- [ ] Build to verify
- [ ] Commit with message: "feat(tab): add desktop view with sidebar layout"

---

### Task 5: Implement Mobile View

**Files:**
- Create: `mobile/src/tab_debloat/view_mobile.rs`
- Create: `mobile/src/tab_debloat/components/package_cards.rs`
- Modify: `mobile/src/tab_debloat/components/mod.rs`
- Modify: `mobile/src/tab_debloat/mod.rs`

**Interfaces:**
- Consumes: `TabDebloatState`, `ViewModelState`
- Produces: `view_mobile::render(...)`, `render_package_cards(...)` with 48px card height

- [ ] Create `package_cards.rs` with card-based list (48px min height per card)
- [ ] Create `view_mobile.rs` with stacked layout
- [ ] Add collapsible filter section
- [ ] Batch actions at bottom
- [ ] Update `mod.rs` to call `view_mobile::render` when width < 800px
- [ ] Build to verify
- [ ] Commit with message: "feat(tab): add mobile view with card layout"

---

### Task 6: Add Filter Debouncing

**Files:**
- Modify: `mobile/src/tab_debloat/mod.rs`
- Modify: `mobile/src/tab_debloat/state.rs`
- Modify: `mobile/src/viewmodel/mod.rs`

**Interfaces:**
- Consumes: `ViewModel::send_command`, debounce timer state
- Produces: 300ms debounced filter dispatch

- [ ] Add debounce logic to `TabDebloat::render` (300ms check)
- [ ] Add `last_filter_input`, `pending_filter_text`, `applied_filter_text` to state
- [ ] Add `filter_packages` method to `ViewModel`
- [ ] Dispatch `FilterPackages` command when debounce elapsed
- [ ] Build and test
- [ ] Commit with message: "feat(tab): add 300ms filter debouncing"

---

### Task 7: Integration Test

**Files:**
- Create: `mobile/tests/integration/debloat_tab_integration_test.rs`

**Interfaces:**
- Consumes: Full `TabDebloat` + `ViewModel` integration
- Produces: End-to-end filter flow test

- [ ] Write `test_full_filter_flow` verifying text filtering works
- [ ] Write `test_filter_with_category` (placeholder for future)
- [ ] Verify command → actor → event → state update flow
- [ ] Run with `cargo nextest run`
- [ ] Commit with message: "test(tab): add integration tests for filter flow"

---

### Task 8: Update Main App

**Files:**
- Modify: `mobile/src/uad_shizuku_app.rs`

**Interfaces:**
- Consumes: New `TabDebloat` module
- Produces: App using refactored tab

- [ ] Replace `TabDebloatControl` with `TabDebloat` in app struct
- [ ] Update tab render call to `self.tab_debloat.render(ui, &mut self.viewmodel)`
- [ ] Comment out old tab import (keep for reference)
- [ ] Build and run manual smoke test
- [ ] Commit with message: "feat(app): switch to refactored debloat tab"

---

### Task 9: Manual Testing Documentation

**Files:**
- Create: `docs/testing/tab-refactor-manual-test-plan.md`

**Interfaces:**
- Produces: Manual test plan document

- [ ] Create test plan with desktop/mobile rendering tests
- [ ] Add responsive transition test (resize across 800px)
- [ ] Add virtual scrolling performance test (500+ packages)
- [ ] Add filter debouncing test
- [ ] Add batch selection test
- [ ] Add error display test
- [ ] Commit with message: "docs: add manual test plan for tab refactor"

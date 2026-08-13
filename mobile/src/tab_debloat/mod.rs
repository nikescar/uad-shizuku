//! Debloat tab module
//!
//! This module implements the refactored debloat tab with MVVM architecture:
//! - `state.rs` - UI state (selection, filters, sorting, dialogs)
//! - `components/mod.rs` - Reusable UI components (desktop, mobile)
//! - `view_desktop.rs` - Desktop layout (800px+) with sidebar
//! - `view_mobile.rs` - Mobile layout (<800px) with card layout
//!
//! The main `TabDebloat` struct implements width-based responsive routing:
//! - Desktop view (800px+): Full-featured data table with all controls
//! - Mobile view (<800px): Card-based list view for small screens

pub mod state;
pub mod components;
pub mod view_desktop;
pub mod view_mobile;

pub use state::{TabDebloatState, SortColumn, DebloatFilter, BatchUninstallState, CachedCategoryCounts};

use eframe::egui;

/// Width threshold (pixels) for switching between desktop and mobile views
const RESPONSIVE_WIDTH_THRESHOLD: f32 = 800.0;

/// Debloat tab controller - coordinates UI rendering and state management
///
/// This struct is responsible for:
/// 1. Rendering the appropriate view based on screen width
/// 2. Handling user interactions (filtering, sorting, selection)
/// 3. Coordinating with ViewModel for data access
/// 4. Managing dialog states (package details, uninstall confirmation)
///
/// The controller uses width-based routing to switch between:
/// - Desktop view (800px+): Full data table with virtual scrolling
/// - Mobile view (<800px): Simplified list interface
#[derive(Debug)]
pub struct TabDebloat {
    /// Tab UI state
    pub state: TabDebloatState,
}

impl Default for TabDebloat {
    fn default() -> Self {
        Self {
            state: TabDebloatState::default(),
        }
    }
}

impl TabDebloat {
    /// Create a new debloat tab controller
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the debloat tab with responsive width-based routing
    ///
    /// This method determines which view to render based on available width:
    /// - If width >= 800px: render desktop view with full data table
    /// - If width < 800px: render mobile view with simplified interface
    ///
    /// # Arguments
    /// * `ui` - egui context for rendering
    /// * `vm_state` - ViewModel state (read-only access to packages and UAD lists)
    /// * `available_width` - available width in pixels for layout
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        vm_state: &crate::viewmodel::ViewModelState,
        available_width: f32,
    ) {
        if available_width >= RESPONSIVE_WIDTH_THRESHOLD {
            self.render_desktop(ui, vm_state);
        } else {
            self.render_mobile(ui, vm_state);
        }
    }

    /// Render desktop view (800px+)
    ///
    /// Full-featured interface with:
    /// - Left sidebar: Category filters, options, advanced settings
    /// - Main content: Search bar, batch actions, error banner, virtual table
    /// - Virtual scrolling data table for performance
    fn render_desktop(&mut self, ui: &mut egui::Ui, vm_state: &crate::viewmodel::ViewModelState) {
        view_desktop::render(ui, vm_state, &mut self.state);
    }

    /// Render mobile view (<800px)
    ///
    /// Simplified interface with:
    /// - Stacked vertical layout
    /// - Collapsible filter section
    /// - Card-based list (48px minimum per card)
    /// - Batch actions at bottom
    fn render_mobile(&mut self, ui: &mut egui::Ui, vm_state: &crate::viewmodel::ViewModelState) {
        view_mobile::render(ui, vm_state, &mut self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tab_debloat() {
        let tab = TabDebloat::new();
        assert!(!tab.state.open);
    }

    #[test]
    fn test_responsive_width_threshold() {
        assert_eq!(RESPONSIVE_WIDTH_THRESHOLD, 800.0);
    }

    #[test]
    fn test_default_tab_debloat() {
        let tab = TabDebloat::default();
        assert_eq!(tab.state.table_version, 0);
        assert!(tab.state.selected_packages.is_empty());
    }
}

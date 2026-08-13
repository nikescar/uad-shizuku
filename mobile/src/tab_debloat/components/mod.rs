//! Debloat tab UI components
//!
//! This module contains reusable UI components for the debloat tab,
//! including desktop and mobile views.
//!
//! Components are organized by layout:
//! - `desktop.rs` - Optimized layout for wide screens (800px+)
//! - `mobile.rs` - Optimized layout for narrow screens (<800px)
//! - `package_table.rs` - Virtual scrolling table component

pub mod package_table;

pub use package_table::render_package_table;

// Placeholder for future component implementations
// pub mod desktop;
// pub mod mobile;

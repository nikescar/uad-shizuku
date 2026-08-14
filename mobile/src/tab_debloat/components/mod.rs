//! Debloat tab UI components
//!
//! This module contains reusable UI components for the debloat tab,
//! including desktop and mobile views.
//!
//! Components are organized by layout:
//! - `package_table.rs` - Virtual scrolling table component (desktop)
//! - `package_cards.rs` - Card-based list component (mobile)

pub mod package_cards;
pub mod package_table;

pub use package_cards::render_package_cards;
pub use package_table::render_package_table;

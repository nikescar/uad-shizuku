//! Common types shared across all actors

use serde::{Deserialize, Serialize};

/// Unified event type from all actors to ViewModel
#[derive(Debug, Clone)]
pub enum ViewModelEvent {
    Debloat(DebloatEvent),
    Scan(ScanEvent),
    Apps(AppsEvent),
    Metadata(MetadataEvent),
}

/// Events from each actor module
pub use super::debloat::DebloatEvent;
pub use super::scan::ScanEvent;
pub use super::apps::AppsEvent;
pub use super::metadata::MetadataEvent;

/// Progress tracking for long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationProgress {
    pub operation: String,
    pub progress: f32,  // 0.0 to 1.0
    pub status: String,
}

/// Metadata source enum for texture tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetadataSource {
    GooglePlay,
    FDroid,
    ApkMirror,
    AndroidPackage,
}

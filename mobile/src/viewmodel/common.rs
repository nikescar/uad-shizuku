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

/// Placeholder event types (will be defined in respective actor files)
#[derive(Debug, Clone)]
pub enum DebloatEvent {
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum AppsEvent {
    Placeholder,
}

#[derive(Debug, Clone)]
pub enum MetadataEvent {
    Placeholder,
}

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

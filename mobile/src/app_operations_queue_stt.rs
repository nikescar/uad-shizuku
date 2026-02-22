use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum OperationType {
    Install { app_name: String, download_url: String, link_type: String },
    Uninstall { package_name: String, is_system: bool },
}

#[derive(Debug, Clone)]
pub enum OperationStatus {
    Pending,
    Processing,
    Success(String), // Success message
    Error(String),
}

#[derive(Debug, Clone)]
pub struct OperationItem {
    pub operation: OperationType,
    pub status: OperationStatus,
}

pub struct AppOperationsQueue {
    pub queue: Arc<Mutex<VecDeque<OperationItem>>>,
    pub results: Arc<Mutex<HashMap<String, OperationStatus>>>,
    pub is_running: Arc<Mutex<bool>>,
    pub progress: Arc<Mutex<Option<f32>>>,
    pub cancelled: Arc<Mutex<bool>>,
}

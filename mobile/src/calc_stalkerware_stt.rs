use std::collections::{HashMap, HashSet};

/// Stalkerware indicators data structure for package detection
#[derive(Debug, Clone)]
pub struct StalkerwareIndicators {
    /// Set of known stalkerware package names for O(1) lookup
    pub package_names: HashSet<String>,
    /// Map package name to stalkerware family name
    pub package_to_family: HashMap<String, String>,
    /// When the indicators were last updated (Unix timestamp)
    pub last_updated: i64,
}

impl StalkerwareIndicators {
    pub fn new() -> Self {
        Self {
            package_names: HashSet::new(),
            package_to_family: HashMap::new(),
            last_updated: 0,
        }
    }

    /// Check if a package is identified as stalkerware
    pub fn is_stalkerware(&self, package_id: &str) -> bool {
        self.package_names.contains(package_id)
    }

    /// Get stalkerware family name if package is identified as stalkerware
    pub fn get_family_name(&self, package_id: &str) -> Option<&String> {
        self.package_to_family.get(package_id)
    }
}

impl Default for StalkerwareIndicators {
    fn default() -> Self {
        Self::new()
    }
}

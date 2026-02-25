pub use crate::calc_stalkerware_stt::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Structure matching the stalkerware IoC YAML format
#[derive(Debug, Deserialize, Serialize)]
pub struct StalkerwareEntry {
    /// Name of the stalkerware family
    pub name: String,
    /// Optional: Alternative names
    #[serde(default)]
    pub names: Vec<String>,
    /// Optional: Type (e.g., "stalkerware")
    #[serde(default)]
    pub r#type: String,
    /// Package names associated with this stalkerware
    #[serde(default)]
    pub packages: Vec<String>,
    /// Optional: Certificate fingerprints (for future use)
    #[serde(default)]
    pub certificates: Vec<String>,
    /// Optional: Associated websites (for future use)
    #[serde(default)]
    pub websites: Vec<String>,
}

/// Parse stalkerware IoC YAML content into StalkerwareIndicators
pub fn parse_stalkerware_yaml(yaml_content: &str) -> Result<StalkerwareIndicators, String> {
    let entries: Vec<StalkerwareEntry> = serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;

    let mut package_names = HashSet::new();
    let mut package_to_family = HashMap::new();

    for entry in entries {
        for package in entry.packages {
            package_names.insert(package.clone());
            package_to_family.insert(package, entry.name.clone());
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    log::info!(
        "Parsed stalkerware IoC with {} package indicators",
        package_names.len()
    );

    Ok(StalkerwareIndicators {
        package_names,
        package_to_family,
        last_updated: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stalkerware_yaml() {
        let yaml = r#"
- name: TestStalkerware
  type: stalkerware
  packages:
    - com.test.stalker
    - com.bad.app
- name: AnotherStalker
  packages:
    - com.evil.tracker
"#;

        let indicators = parse_stalkerware_yaml(yaml).unwrap();
        assert_eq!(indicators.package_names.len(), 3);
        assert!(indicators.package_names.contains("com.test.stalker"));
        assert!(indicators.package_names.contains("com.bad.app"));
        assert!(indicators.package_names.contains("com.evil.tracker"));

        assert_eq!(
            indicators.package_to_family.get("com.test.stalker"),
            Some(&"TestStalkerware".to_string())
        );
        assert_eq!(
            indicators.package_to_family.get("com.evil.tracker"),
            Some(&"AnotherStalker".to_string())
        );
    }

    #[test]
    fn test_is_stalkerware() {
        let mut indicators = StalkerwareIndicators::new();
        indicators.package_names.insert("com.evil.app".to_string());

        assert!(indicators.is_stalkerware("com.evil.app"));
        assert!(!indicators.is_stalkerware("com.good.app"));
    }

    #[test]
    fn test_get_family_name() {
        let mut indicators = StalkerwareIndicators::new();
        indicators.package_to_family.insert(
            "com.evil.app".to_string(),
            "EvilFamily".to_string(),
        );

        assert_eq!(
            indicators.get_family_name("com.evil.app"),
            Some(&"EvilFamily".to_string())
        );
        assert_eq!(indicators.get_family_name("com.good.app"), None);
    }
}

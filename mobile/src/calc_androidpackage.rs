#[derive(Clone, Debug)]
pub struct AndroidPackageInfo {
    pub package_id: String,
    pub label: String,
    pub icon_bytes: Vec<u8>,
}

#[cfg(target_os = "android")]
pub fn fetch_android_package_info(package_id: &str) -> Option<AndroidPackageInfo> {
    let label = match crate::android_packagemanager::get_application_label(package_id) {
        Ok(l) => l,
        Err(e) => {
            log::debug!("Failed to get label for {}: {}", package_id, e);
            return None;
        }
    };

    let icon_bytes = match crate::android_packagemanager::get_application_icon(package_id) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::debug!("Failed to get icon for {}: {}", package_id, e);
            return None;
        }
    };

    Some(AndroidPackageInfo {
        package_id: package_id.to_string(),
        label,
        icon_bytes,
    })
}

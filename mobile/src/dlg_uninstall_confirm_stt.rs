pub struct DlgUninstallConfirm {
    pub open: bool,
    pub packages: Vec<String>,
    pub is_system: Vec<bool>,
    pub confirmed: bool,
    /// Optional human-readable app names (used by tab_apps_control)
    pub app_names: Vec<Option<String>>,
}

impl Default for DlgUninstallConfirm {
    fn default() -> Self {
        Self {
            open: false,
            packages: Vec::new(),
            is_system: Vec::new(),
            confirmed: false,
            app_names: Vec::new(),
        }
    }
}

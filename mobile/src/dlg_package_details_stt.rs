#[derive(Debug, Clone)]
pub struct DlgPackageDetails {
    pub open: bool,
    pub selected_package_index: Option<usize>,
}

impl Default for DlgPackageDetails {
    fn default() -> Self {
        Self {
            open: false,
            selected_package_index: None,
        }
    }
}

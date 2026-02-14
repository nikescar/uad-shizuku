pub struct DlgAdbInstall {
    pub open: bool,
    pub retry_requested: bool,
}

impl Default for DlgAdbInstall {
    fn default() -> Self {
        Self {
            open: false,
            retry_requested: false,
        }
    }
}

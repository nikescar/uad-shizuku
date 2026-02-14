pub struct DlgUpdate {
    pub open: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: String,
    pub download_url: String,
    pub do_update: bool,
}

impl Default for DlgUpdate {
    fn default() -> Self {
        Self {
            open: false,
            current_version: String::new(),
            latest_version: String::new(),
            release_notes: String::new(),
            download_url: String::new(),
            do_update: false,
        }
    }
}

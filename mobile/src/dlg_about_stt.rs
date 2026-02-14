pub struct DlgAbout {
    pub open: bool,
    pub do_check_update: bool,
    pub do_perform_update: bool,
}

impl Default for DlgAbout {
    fn default() -> Self {
        Self {
            open: false,
            do_check_update: false,
            do_perform_update: false,
        }
    }
}

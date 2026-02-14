pub struct DlgSettings {
    pub open: bool,
    // Temporary settings for dialog (applied only on Save)
    pub virustotal_apikey: String,
    pub hybridanalysis_apikey: String,
    pub invalidate_cache: bool,
    pub flush_virustotal: bool,
    pub flush_hybridanalysis: bool,
    pub flush_googleplay: bool,
    pub flush_fdroid: bool,
    pub flush_apkmirror: bool,
    pub google_play_renderer: bool,
    pub fdroid_renderer: bool,
    pub apkmirror_renderer: bool,
    pub virustotal_submit: bool,
    pub hybridanalysis_submit: bool,
    pub hybridanalysis_tag_ignorelist: String,
    pub unsafe_app_remove: bool,
    pub autoupdate: bool,
    // Font selector state
    pub selected_font_display: String,
    pub system_fonts: Vec<(String, String)>,
    pub system_fonts_loaded: bool,
    // Action results
    pub save_clicked: bool,
    pub theme_to_apply: Option<String>,
}

impl Default for DlgSettings {
    fn default() -> Self {
        Self {
            open: false,
            virustotal_apikey: String::new(),
            hybridanalysis_apikey: String::new(),
            invalidate_cache: false,
            flush_virustotal: false,
            flush_hybridanalysis: false,
            flush_googleplay: false,
            flush_fdroid: false,
            flush_apkmirror: false,
            google_play_renderer: false,
            fdroid_renderer: false,
            apkmirror_renderer: false,
            virustotal_submit: false,
            hybridanalysis_submit: false,
            hybridanalysis_tag_ignorelist: String::new(),
            unsafe_app_remove: false,
            autoupdate: false,
            selected_font_display: "Default (NotoSansKr)".to_string(),
            system_fonts: Vec::new(),
            system_fonts_loaded: false,
            save_clicked: false,
            theme_to_apply: None,
        }
    }
}

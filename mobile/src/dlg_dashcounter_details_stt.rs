#[derive(Debug, Clone, PartialEq)]
pub enum DashCounterCategory {
    /// Debloat categories
    DebloatRecommend,
    DebloatAdvanced,
    DebloatExpert,
    DebloatUnsafe,
    DebloatUnknown,
    /// Stalkerware categories
    StalkerwareDetected,
    StalkerwareUndetected,
    /// IzzyRisk categories
    IzzyRiskSafe, // 0
    IzzyRiskNormal,   // 1-10
    IzzyRiskModerate, // 11-20
    IzzyRiskHigh,     // 20+
    /// VirusTotal categories
    VirusTotalMalicious,
    VirusTotalSuspicious,
    VirusTotalSafe,
    VirusTotalNotScanned,
    /// HybridAnalysis categories
    HybridAnalysisMalicious,
    HybridAnalysisMaliciousIgnored,
    HybridAnalysisSuspicious,
    HybridAnalysisSafe,
    HybridAnalysisNotScanned,
    /// Offa FOSS Apps category (dynamic based on app list categories)
    OffaCategory(String),
    /// FMHY FOSS Apps category (dynamic based on app list categories)
    FmhyCategory(String),
}

#[derive(Debug, Clone)]
pub struct DlgDashCounterDetails {
    pub open: bool,
    pub category: Option<DashCounterCategory>,
    pub count_enabled: usize,
    pub count_total: usize,
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
    pub show_only_enabled: bool,
    pub hide_system_app: bool,
    pub text_filter: String,
    pub offa_apps: Vec<crate::tab_apps_control_stt::AppEntry>,
    // Cache fields for performance
    pub cache_key: String,
    pub cached_rows: Vec<CachedRowData>,
    pub last_refresh_time: f64,
    pub refresh_interval: f64, // in seconds, default 0.05 (50ms)
}

#[derive(Debug, Clone)]
pub struct CachedRowData {
    pub package_id: String,
    pub title: String,
    pub subtitle: String,
    pub has_texture: bool, // Flag to indicate if texture should exist in store
}

impl Default for DlgDashCounterDetails {
    fn default() -> Self {
        Self {
            open: false,
            category: None,
            count_enabled: 0,
            count_total: 0,
            sort_column: None,
            sort_ascending: true,
            show_only_enabled: false,
            hide_system_app: false,
            text_filter: String::new(),
            offa_apps: Vec::new(),
            cache_key: String::new(),
            cached_rows: Vec::new(),
            last_refresh_time: 0.0,
            refresh_interval: 0.05, // 50ms throttle
        }
    }
}

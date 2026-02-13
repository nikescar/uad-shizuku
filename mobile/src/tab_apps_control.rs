use crate::adb::PackageFingerprint;
pub use crate::tab_apps_control_stt::*;
use eframe::egui;
use egui_i18n::tr;
use egui_material3::{data_table, icon_button_standard, theme::get_global_color};

// SVG icons as constants (moved to svg_stt.rs)
use crate::svg_stt::*;
use crate::material_symbol_icons::{ICON_CANCEL, ICON_CHECK_CIRCLE, ICON_DELETE, ICON_DOWNLOAD, ICON_INFO, ICON_CHECK_BOX, ICON_REFRESH};
use crate::{DESKTOP_MIN_WIDTH, BASE_TABLE_WIDTH};

// Pre-compiled regex patterns for performance (avoid recompiling on every call)
lazy_static::lazy_static! {
    static ref GITHUB_DOWNLOADABLE_RE: regex::Regex =
        regex::Regex::new(r"github\.com/[^/]+/[^/]+/?$").unwrap();
    static ref FDROID_DOWNLOADABLE_RE: regex::Regex =
        regex::Regex::new(r"f-droid\.org/(?:[^/]+/)?packages/[^/]+").unwrap();
    static ref IZZY_DOWNLOADABLE_RE: regex::Regex =
        regex::Regex::new(r"izzysoft\.de/(?:[^/]+/)?apk/[^/]+").unwrap();
    static ref GITLAB_DOWNLOADABLE_RE: regex::Regex =
        regex::Regex::new(r"gitlab\.com/[^/]+/[^/]+/?$").unwrap();
}

impl Default for TabAppsControl {
    fn default() -> Self {
        Self {
            open: false,
            installed_packages: Vec::new(),
            app_lists: Vec::new(),
            selected_app_list: None,
            app_entries: Vec::new(),
            refresh_pending: false,
            cache_dir: std::path::PathBuf::new(),
            tmp_dir: std::path::PathBuf::new(),
            installing_apps: std::collections::HashMap::new(),
            selected_device: None,
            previous_app_list: None,
            recently_installed_apps: std::collections::HashSet::new(),
            show_only_installable: true, // Default to showing only installable apps
            disable_github_install: true, // Default to allowing GitHub installs
            text_filter: String::new(),
            sort_column: None,
            sort_ascending: true,
        }
    }
}

impl TabAppsControl {
    pub fn new(cache_dir: std::path::PathBuf, tmp_dir: std::path::PathBuf) -> Self {
        let mut instance = Self::default();
        instance.cache_dir = cache_dir;
        instance.tmp_dir = tmp_dir;
        instance.load_app_lists();
        instance
    }

    pub fn update_packages(&mut self, packages: Vec<PackageFingerprint>) {
        self.installed_packages = packages;
    }

    pub fn set_selected_device(&mut self, device: Option<String>) {
        self.selected_device = device;
    }

    /// Sort app entries based on current sort column and direction
    pub fn sort_apps(&mut self) {
        if let Some(col) = self.sort_column {
            let ascending = self.sort_ascending;
            match col {
                0 => {
                    // Sort by category
                    self.app_entries.sort_by(|a, b| {
                        let cmp = a.category.to_lowercase().cmp(&b.category.to_lowercase());
                        if ascending { cmp } else { cmp.reverse() }
                    });
                }
                1 => {
                    // Sort by app name
                    self.app_entries.sort_by(|a, b| {
                        let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                        if ascending { cmp } else { cmp.reverse() }
                    });
                }
                2 => {
                    // Sort by number of links (more links = higher priority)
                    self.app_entries.sort_by(|a, b| {
                        let cmp = a.links.len().cmp(&b.links.len());
                        if ascending { cmp } else { cmp.reverse() }
                    });
                }
                _ => {}
            }
        }
    }

    pub fn load_app_lists(&mut self) {
        let apps_list_content = include_str!("../resources/apps-list.txt");
        self.app_lists = apps_list_content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() == 3 {
                    Some(AppListSource {
                        name: parts[0].trim().to_string(),
                        info_url: parts[1].trim().to_string(),
                        contents_url: parts[2].trim().to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();
    }

    fn parse_markdown_content(&mut self, content: &str) {
        self.app_entries.clear();
        let mut current_category = String::from("Uncategorized");
        // let content = html2md::parse_html(content);

        log::debug!(
            "Parsing markdown content, {} lines total",
            content.lines().count()
        );

        for line in content.lines() {
            // Check if line is a markdown header (category)
            if line.starts_with('#') {
                let header_text = line.trim_start_matches('#').trim();
                if !header_text.is_empty() {
                    current_category = self.sanitize_string(header_text);
                    log::debug!("Found category: {}", current_category);
                }
            } else if line.contains("f-droid.org")
                || line.contains("izzysoft.de")
                || line.contains("github.com")
                || line.contains("gitlab.com")
            {
                // Skip if current category contains "iOS"
                if current_category.contains("iOS") {
                    log::debug!(
                        "Skipping line in iOS category: {}",
                        line.chars().take(100).collect::<String>()
                    );
                    continue;
                }

                // Parse app line (works with or without leading dash)
                let name = self.extract_app_name(line);
                let name = self.sanitize_string(&name);
                let links = self.extract_links(line);

                if !name.is_empty() && !links.is_empty() {
                    log::debug!(
                        "Found app: {} with {} links in category {}",
                        name,
                        links.len(),
                        current_category
                    );
                    self.app_entries.push(AppEntry {
                        category: current_category.clone(),
                        name,
                        links,
                        package_name: None,
                    });
                } else {
                    log::debug!(
                        "Skipped line (empty name or no links): {}",
                        line.chars().take(100).collect::<String>()
                    );
                }
            }
        }

        log::info!("Parsing complete: {} apps found", self.app_entries.len());
    }

    fn get_fdroid_download_url(&self, package_name: &str) -> Option<String> {
        // Access F-Droid package page
        let url = format!("https://f-droid.org/packages/{}/", package_name);

        log::info!("Fetching F-Droid page: {}", url);

        // Fetch HTML content
        let html = match self.fetch_url(&url) {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to fetch F-Droid page: {}", e);
                return None;
            }
        };

        // Parse HTML content directly for href attributes
        let mut hrefs: Vec<String> = Vec::new();
        for line in html.lines() {
            if line.contains("href=") && (line.contains(".apk") || line.contains("download")) {
                // Extract href value
                if let Some(start) = line.find("href=\"") {
                    let start_pos = start + 6;
                    if let Some(end) = line[start_pos..].find('"') {
                        let href = &line[start_pos..start_pos + end];
                        hrefs.push(href.to_string());
                    }
                } else if let Some(start) = line.find("href='") {
                    let start_pos = start + 6;
                    if let Some(end) = line[start_pos..].find('\'') {
                        let href = &line[start_pos..start_pos + end];
                        hrefs.push(href.to_string());
                    }
                }
            }
        }

        // Find the first link that contains the package_name and ends with .apk
        for href in &hrefs {
            if href.contains(package_name) && href.ends_with(".apk") {
                // Convert relative URL to absolute if needed
                let download_url = if href.starts_with("http") {
                    href.clone()
                } else if href.starts_with("/") {
                    format!("https://f-droid.org{}", href)
                } else {
                    format!("https://f-droid.org/{}", href)
                };

                log::info!("Found F-Droid APK download URL: {}", download_url);

                return Some(download_url);

                // // Download APK to tmp_dir
                // match self.download_apk_to_tmp(&download_url, package_name) {
                //     Ok(path) => {
                //         log::info!("APK downloaded to tmp_dir: {:?}", path);
                //         return Some(download_url);
                //     }
                //     Err(e) => {
                //         log::error!("Failed to download APK to tmp_dir: {}", e);
                //         return None;
                //     }
                // }
            }
        }

        log::warn!(
            "No matching APK download link found for package: {}",
            package_name
        );
        None
    }

    fn download_apk(
        &self,
        url: &str,
        package_name: &str,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        log::info!("Downloading APK to tmp_dir from: {}", url);

        let response = ureq::get(url).call()?;

        // Create tmp directory if it doesn't exist
        std::fs::create_dir_all(&self.tmp_dir)?;

        // Save APK file to tmp_dir
        let apk_path = self.tmp_dir.join(format!("{}.apk", package_name));
        let mut file = std::fs::File::create(&apk_path)?;
        std::io::copy(&mut response.into_reader(), &mut file)?;

        log::info!("APK downloaded to tmp_dir: {:?}", apk_path);
        Ok(apk_path)
    }

    // fn get_izzy_download_url(&self, package_name: &str) -> Option<String> {
    //     // IzzyOnDroid APK download URL format
    //     Some(format!("https://apt.izzysoft.de/fdroid/repo/{}_latest.apk", package_name))
    // }

    fn get_github_download_url(&self, repo_url: &str) -> Option<String> {
        // Extract owner/repo from GitHub URL
        // Supports: https://github.com/owner/repo or github.com/owner/repo
        let repo_path = repo_url
            .trim_end_matches('/')
            .replace("https://", "")
            .replace("http://", "")
            .replace("github.com/", "");

        let parts: Vec<&str> = repo_path.split('/').collect();
        if parts.len() < 2 {
            log::error!("Invalid GitHub URL format: {}", repo_url);
            return None;
        }

        let owner = parts[0];
        let repo = parts[1];
        let api_url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        );

        log::info!("Fetching GitHub releases from: {}", api_url);

        // Fetch the releases JSON
        let json_str = match self.fetch_url(&api_url) {
            Ok(content) => content,
            Err(e) => {
                log::error!("Failed to fetch GitHub releases: {}", e);
                return None;
            }
        };

        // Parse JSON
        let json: serde_json::Value = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to parse GitHub releases JSON: {}", e);
                return None;
            }
        };

        // Extract assets
        let assets = match json.get("assets").and_then(|a| a.as_array()) {
            Some(a) => a,
            None => {
                log::warn!("No assets found in GitHub release");
                return None;
            }
        };

        // Collect all APK download URLs
        let mut apk_urls: Vec<(String, String)> = Vec::new();
        for asset in assets {
            let name = asset
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_lowercase();
            let download_url = asset
                .get("browser_download_url")
                .and_then(|u| u.as_str())
                .unwrap_or("");

            if name.ends_with(".apk") && !download_url.is_empty() {
                apk_urls.push((name, download_url.to_string()));
            }
        }

        if apk_urls.is_empty() {
            log::warn!("No APK files found in GitHub release for {}/{}", owner, repo);
            return None;
        }

        log::debug!("Found {} APK files in release", apk_urls.len());

        // Get device CPU ABI list for architecture-specific matching
        let device_abis: Vec<String> = if let Some(ref device) = self.selected_device {
            #[cfg(not(target_os = "android"))]
            {
                crate::adb::get_cpu_abi_list(device).unwrap_or_default()
            }
            #[cfg(target_os = "android")]
            {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        log::debug!("Device ABIs: {:?}", device_abis);

        // Priority order for APK selection:
        // 1. Universal APK (always preferred)
        // 2. Device-specific ABI in priority order from ro.product.cpu.abilist
        // 3. APK without architecture suffix (likely universal)
        // 4. Fallback to first APK

        // Type 1: Look for universal APK
        for (name, url) in &apk_urls {
            if name.contains("universal") {
                log::info!("Found universal APK: {}", url);
                return Some(url.clone());
            }
        }

        // Type 2: Look for APK matching device ABIs in priority order
        for abi in &device_abis {
            // Create variants of the ABI name for matching
            let abi_lower = abi.to_lowercase();
            let abi_underscore = abi_lower.replace("-", "_");

            for (name, url) in &apk_urls {
                if name.contains(&abi_lower) || name.contains(&abi_underscore) {
                    log::info!("Found APK matching device ABI '{}': {}", abi, url);
                    return Some(url.clone());
                }
            }
        }

        // Type 3: If only one APK exists, or APK without architecture (likely universal)
        for (name, url) in &apk_urls {
            // Check if APK name doesn't contain common architecture strings
            let has_arch = name.contains("arm64")
                || name.contains("arm")
                || name.contains("x86")
                || name.contains("mips");
            if !has_arch {
                log::info!("Found APK without architecture suffix (likely universal): {}", url);
                return Some(url.clone());
            }
        }

        // Type 4: Fallback - return the first APK if nothing else matched
        if let Some((name, url)) = apk_urls.first() {
            log::info!("Falling back to first APK: {} -> {}", name, url);
            return Some(url.clone());
        }

        log::warn!("No suitable APK found for {}/{}", owner, repo);
        None
    }

    /// Install an app by downloading its APK and installing via adb
    /// Returns Ok(()) on success, Err with error message on failure
    #[cfg(not(target_os = "android"))]
    pub fn install_app(&mut self, app: &AppEntry) -> Result<(), String> {
        let device = match &self.selected_device {
            Some(d) => d.clone(),
            None => return Err("No device selected".to_string()),
        };

        // Get the downloadable link
        let (url, link_type) = match self.get_downloadable_link(app) {
            Some(link) => link,
            None => return Err("No downloadable link found".to_string()),
        };

        log::info!(
            "Installing app '{}' from {} ({})",
            app.name,
            url,
            link_type
        );

        // Check if GitHub installs are disabled
        if link_type == "github-downloadable" && self.disable_github_install {
            return Err("GitHub installations are disabled".to_string());
        }

        // Update status
        self.installing_apps
            .insert(app.name.clone(), "Fetching download URL...".to_string());

        // Get the actual APK download URL based on link type
        let download_url = match link_type.as_str() {
            "github-downloadable" => {
                match self.get_github_download_url(&url) {
                    Some(u) => u,
                    None => return Err("Failed to get GitHub download URL".to_string()),
                }
            }
            "fdroid-downloadable" => {
                // Extract package name from fdroid URL
                let package_name = match self.extract_package_from_url(&url) {
                    Some(p) => p,
                    None => return Err("Failed to extract package name from F-Droid URL".to_string()),
                };
                match self.get_fdroid_download_url(&package_name) {
                    Some(u) => u,
                    None => return Err("Failed to get F-Droid download URL".to_string()),
                }
            }
            _ => return Err(format!("Unsupported link type: {}", link_type)),
        };

        log::info!("Download URL: {}", download_url);

        // Update status
        self.installing_apps
            .insert(app.name.clone(), "Downloading APK...".to_string());

        // Generate a safe filename from app name
        let safe_name: String = app
            .name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        let safe_name = if safe_name.is_empty() {
            "app".to_string()
        } else {
            safe_name
        };

        // Download the APK
        let apk_path = match self.download_apk(&download_url, &safe_name) {
            Ok(path) => path,
            Err(e) => {
                self.installing_apps.remove(&app.name);
                return Err(format!("Failed to download APK: {}", e));
            }
        };

        log::info!("APK downloaded to: {:?}", apk_path);

        // Update status
        self.installing_apps
            .insert(app.name.clone(), "Installing APK...".to_string());

        // Install the APK using adb
        let apk_path_str = apk_path.to_string_lossy().to_string();
        match crate::adb::install_apk(&apk_path_str, &device) {
            Ok(result) => {
                log::info!("APK installed successfully: {}", result);
                self.installing_apps.remove(&app.name);

                // Clean up the downloaded APK
                if let Err(e) = std::fs::remove_file(&apk_path) {
                    log::warn!("Failed to clean up APK file: {}", e);
                }

                // Track this app as recently installed (for GitHub apps where package name isn't in URL)
                self.recently_installed_apps.insert(app.name.clone());

                // Mark that packages need to be refreshed to show updated install status
                self.refresh_pending = true;

                Ok(())
            }
            Err(e) => {
                self.installing_apps.remove(&app.name);

                // Clean up the downloaded APK even on failure
                if let Err(e2) = std::fs::remove_file(&apk_path) {
                    log::warn!("Failed to clean up APK file: {}", e2);
                }

                Err(format!("Failed to install APK: {}", e))
            }
        }
    }

    #[cfg(target_os = "android")]
    pub fn install_app(&mut self, _app: &AppEntry) -> Result<(), String> {
        Err("Direct install not supported on Android".to_string())
    }

    pub fn reload_applist_and_parse_apps(&mut self) {
        let selected_idx = match self.selected_app_list {
            Some(idx) => idx,
            None => return,
        };

        if selected_idx >= self.app_lists.len() {
            return;
        }

        let app_list = &self.app_lists[selected_idx];
        let cache_file = self
            .cache_dir
            .join(format!("{}.md", app_list.name.replace(" ", "_")));

        // Try to download the content
        match self.fetch_url(&app_list.contents_url) {
            Ok(content) => {
                // Save to cache
                if let Err(e) = std::fs::write(&cache_file, &content) {
                    log::error!("Failed to save cache file: {}", e);
                }
                self.parse_markdown_content(&content);
            }
            Err(e) => {
                log::error!("Failed to download app list: {}", e);
                // Try to load from cache
                if let Ok(content) = std::fs::read_to_string(&cache_file) {
                    self.parse_markdown_content(&content);
                }
            }
        }
    }

    fn fetch_url(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = ureq::get(url).call()?;
        let content = response.into_string()?;
        Ok(content)
    }

    fn sanitize_string(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .trim()
            .to_string()
    }

    fn extract_app_name(&self, line: &str) -> String {
        // Remove leading markers (-, *, numbers, etc)
        let mut text = line.trim();

        // Remove markdown list markers
        if text.starts_with('-') || text.starts_with('*') || text.starts_with('+') {
            text = text[1..].trim();
        }

        // Remove numbered list markers (e.g., "1. ", "12. ")
        if let Some(pos) = text.find(|c: char| c == '.' || c == ')') {
            if text[..pos].chars().all(|c| c.is_ascii_digit()) {
                text = text[pos + 1..].trim();
            }
        }

        // Try to extract from markdown link format [Name](url)
        if let Some(start) = text.find('[') {
            if let Some(end) = text.find(']') {
                if end > start {
                    let name = text[start + 1..end].trim();
                    if !name.is_empty() {
                        return name.to_string();
                    }
                }
            }
        }

        // Fallback: take text before first http link
        if let Some(link_start) = text.find("http") {
            let name = text[..link_start].trim();
            // Remove trailing dashes, colons, etc
            let name = name.trim_end_matches(&['-', ':', '—', '–'][..]).trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }

        // Last resort: return the whole cleaned line
        text.to_string()
    }

    fn extract_links(&self, line: &str) -> Vec<(String, String)> {
        let mut links = Vec::new();
        let mut search_pos = 0;

        while let Some(link_start) = line[search_pos..].find("http") {
            let actual_start = search_pos + link_start;
            let remaining = &line[actual_start..];

            // Find end of URL (space, ), ], or end of line)
            let end_pos = remaining
                .find(|c: char| c.is_whitespace() || c == ')' || c == ']')
                .unwrap_or(remaining.len());

            let url = remaining[..end_pos].to_string();

            // Determine link type based on URL using pre-compiled regexes
            let link_type = if FDROID_DOWNLOADABLE_RE.is_match(&url) {
                "fdroid-downloadable"
            } else if url.contains("f-droid.org") {
                "fdroid"
            } else if IZZY_DOWNLOADABLE_RE.is_match(&url) {
                "izzy-downloadable"
            } else if url.contains("izzysoft.de") || url.contains("apt.izzysoft.de") {
                "izzy"
            } else if GITHUB_DOWNLOADABLE_RE.is_match(&url) {
                "github-downloadable"
            } else if url.contains("github.com") {
                "github"
            } else if GITLAB_DOWNLOADABLE_RE.is_match(&url) {
                "gitlab-downloadable"
            } else if url.contains("gitlab.com") {
                "gitlab"
            } else if url.contains("play.google.com") {
                "googleplay"
            } else if url.contains("reddit.com") {
                "reddit"
            } else if url.contains("discord.") {
                "discord"
            } else if url.contains("matrix.org") {
                "matrix"
            } else if url.contains("telegram.") {
                "telegram"
            } else if url.contains("t.me") {
                "telegram"
            } else if url.contains("youtube.com") || url.contains("youtu.be") {
                "youtube"
            } else {
                "home"
            };

            links.push((url, link_type.to_string()));
            search_pos = actual_start + end_pos;
        }

        links
    }

    fn is_app_installed(&self, app: &AppEntry) -> bool {
        // First check if this app was recently installed (for GitHub apps where package name isn't in URL)
        if self.recently_installed_apps.contains(&app.name) {
            return true;
        }

        // Check if any package name matches
        // For now, we'll use a simple heuristic: extract package name from fdroid/izzy links
        for (url, link_type) in &app.links {
            if link_type.contains("downloadable") {
                // || link_type == "gitlab"  || link_type == "izzy"
                if let Some(package_name) = self.extract_package_from_url(url) {
                    for installed in &self.installed_packages {
                        if installed.pkg == package_name {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Get the installed package info for an app
    /// Returns Some((package_name, is_system, enabled_state)) if found
    fn get_installed_package_info(&self, app: &AppEntry) -> Option<(String, bool, String)> {
        for (url, link_type) in &app.links {
            if link_type.contains("downloadable") {
                if let Some(package_name) = self.extract_package_from_url(url) {
                    for installed in &self.installed_packages {
                        if installed.pkg == package_name {
                            let is_system = installed.flags.contains("SYSTEM");
                            // Get enabled state from first user
                            let enabled_state = installed
                                .users
                                .first()
                                .map(|u| {
                                    match u.enabled {
                                        0 => "DEFAULT",
                                        1 => "ENABLED",
                                        2 => "DISABLED",
                                        3 => "DISABLED_USER",
                                        4 => "DISABLED_UNTIL_USED",
                                        _ => "UNKNOWN",
                                    }
                                    .to_string()
                                })
                                .unwrap_or_else(|| "DEFAULT".to_string());
                            return Some((package_name, is_system, enabled_state));
                        }
                    }
                }
            }
        }
        None
    }

    // run get_fdroid_download_url without error, return the url if successful
    #[allow(dead_code)]
    fn auto_downloadable_link(&self, app: &AppEntry) -> Option<String> {
        for (url, link_type) in &app.links {
            if link_type == "fdroid-downloadable" {
                if let Some(package_name) = self.extract_package_from_url(url) {
                    if let Some(download_url) = self.get_fdroid_download_url(&package_name) {
                        return Some(download_url);
                    }
                }
            } else if link_type == "github-downloadable" {
                if let Some(download_url) = self.get_github_download_url(url) {
                    return Some(download_url);
                }
            }
        }
        None
    }

    fn extract_package_from_url(&self, url: &str) -> Option<String> {
        // Use pre-compiled static regexes for performance
        // Extract package name from F-Droid or IzzyOnDroid URL
        if FDROID_DOWNLOADABLE_RE.is_match(url) {
            url.split("packages/")
                .nth(1)
                .map(|s| s.trim_end_matches('/').to_string())
        } else if IZZY_DOWNLOADABLE_RE.is_match(url) {
            url.split("apk/")
                .nth(1)
                .map(|s| s.trim_end_matches('/').to_string())
        } else if url.contains("apt.izzysoft.de/packages/") {
            url.split("packages/")
                .nth(1)
                .map(|s| s.trim_end_matches('/').to_string())
        } else {
            None
        }
    }

    /// Check if an app entry matches the text filter
    fn matches_text_filter(&self, app: &AppEntry) -> bool {
        if self.text_filter.is_empty() {
            return true;
        }

        let filter_lower = self.text_filter.to_lowercase();

        // Check category
        if app.category.to_lowercase().contains(&filter_lower) {
            return true;
        }

        // Check app name
        if app.name.to_lowercase().contains(&filter_lower) {
            return true;
        }

        // Check package name if available
        if let Some(ref pkg_name) = app.package_name {
            if pkg_name.to_lowercase().contains(&filter_lower) {
                return true;
            }
        }

        // Check link URLs and types
        for (url, link_type) in &app.links {
            if url.to_lowercase().contains(&filter_lower) {
                return true;
            }
            if link_type.to_lowercase().contains(&filter_lower) {
                return true;
            }
        }

        false
    }

    /// Get the first downloadable link type and URL for an app
    /// Returns Some((url, link_type)) for github-downloadable or fdroid-downloadable
    /// Returns None for non-downloadable or not-yet-supported types (gitlab, izzy)
    fn get_downloadable_link(&self, app: &AppEntry) -> Option<(String, String)> {
        for (url, link_type) in &app.links {
            match link_type.as_str() {
                "github-downloadable" | "fdroid-downloadable" => {
                    return Some((url.clone(), link_type.clone()));
                }
                // gitlab-downloadable and izzy-downloadable are not implemented yet
                "gitlab-downloadable" | "izzy-downloadable" => {
                    continue;
                }
                _ => continue,
            }
        }
        None
    }

    /// Returns true if an error occurred during any operation
    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut has_error = false;
        // Top controls: combo box and refresh chip
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(tr!("app-list"));

                // Get selected text for combo box
                let selected_text = if let Some(idx) = self.selected_app_list {
                    if idx < self.app_lists.len() {
                        self.app_lists[idx].name.clone()
                    } else {
                        tr!("select-app-list")
                    }
                } else {
                    tr!("select-app-list")
                };

                // Create combo box for app lists
                egui::ComboBox::from_label("")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        for (idx, app_list) in self.app_lists.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.selected_app_list,
                                Some(idx),
                                &app_list.name,
                            );
                        }
                    });

                // Manually detect selection change
                let selection_changed = self.selected_app_list != self.previous_app_list;

                if selection_changed {
                    log::info!(
                        "App list selection changed from {:?} to {:?}",
                        self.previous_app_list,
                        self.selected_app_list
                    );
                    self.previous_app_list = self.selected_app_list;

                    if let Some(idx) = self.selected_app_list {
                        if idx < self.app_lists.len() {
                            let app_list = &self.app_lists[idx];
                            let cache_file = self
                                .cache_dir
                                .join(format!("{}.md", app_list.name.replace(" ", "_")));

                            log::info!(
                                "Loading app list '{}' from index {}",
                                app_list.name,
                                idx
                            );

                            // Check if cache file exists
                            if cache_file.exists() {
                                log::info!("Cache file exists at {:?}, loading...", cache_file);
                                // Load from cache
                                if let Ok(content) = std::fs::read_to_string(&cache_file) {
                                    log::info!(
                                        "Successfully read cache file, {} bytes",
                                        content.len()
                                    );
                                    self.parse_markdown_content(&content);
                                } else {
                                    log::error!(
                                        "Failed to read cache file at {:?}",
                                        cache_file
                                    );
                                }
                            } else {
                                // Download if cache doesn't exist
                                log::info!(
                                    "Cache file not found at {:?}, downloading from: {}",
                                    cache_file,
                                    app_list.contents_url
                                );
                                match self.fetch_url(&app_list.contents_url) {
                                    Ok(content) => {
                                        log::info!(
                                            "Successfully downloaded {} bytes",
                                            content.len()
                                        );
                                        // Save to cache
                                        if let Err(e) = std::fs::write(&cache_file, &content) {
                                            log::error!("Failed to save cache file: {}", e);
                                        } else {
                                            log::info!("Successfully saved to cache");
                                        }
                                        self.parse_markdown_content(&content);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to download app list: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            });
            if ui.add(icon_button_standard(ICON_INFO.to_string())).on_hover_text(tr!("app-list-info")).clicked() {
                // open selected app list info URL
                if let Some(idx) = self.selected_app_list {
                    if idx < self.app_lists.len() {
                        let app_list = &self.app_lists[idx];
                        let info_url = &app_list.info_url;
                        log::info!("Opening info URL: {}", info_url);
                        #[cfg(not(target_os = "android"))]
                        {
                            if let Err(e) = webbrowser::open(info_url) {
                                log::error!("Failed to open info URL: {}", e);
                            }
                        }
                    }
                }
            }
            if ui.add(icon_button_standard(ICON_REFRESH.to_string())).on_hover_text(tr!("refresh-list")).clicked() {
                self.reload_applist_and_parse_apps();
            }

            ui.add_space(10.0);

            // Show number of apps found
            if !self.app_entries.is_empty() {
                ui.label(tr!("apps-found", { count: self.app_entries.len() }));
            }
        });

        ui.add_space(10.0);

        // Show only installable toggle
        ui.horizontal(|ui| {
            ui.label(tr!("show-only-installable"));
            toggle_ui(ui, &mut self.show_only_installable);
            ui.add_space(10.0);
            ui.label(tr!("disable-github-install"));
            toggle_ui(ui, &mut self.disable_github_install);
            ui.add_space(10.0);
            ui.label(tr!("filter"));
            let response = ui.add(egui::TextEdit::singleline(&mut self.text_filter)
                .hint_text(tr!("filter-hint"))
                .desired_width(200.0));
            #[cfg(target_os = "android")]
            {
                if response.gained_focus() {
                    let _ = crate::android_inputmethod::show_soft_input();
                }
                if response.lost_focus() {
                    let _ = crate::android_inputmethod::hide_soft_input();
                }
            }
            crate::clipboard_popup::show_clipboard_popup(ui, &response, &mut self.text_filter);
            if !self.text_filter.is_empty() && ui.button("✕").clicked() {
                self.text_filter.clear();
            }
        });

        ui.add_space(10.0);

        // Apply Material theme styling to the table area
        let surface = get_global_color("surface");
        let on_surface = get_global_color("onSurface");
        let primary = get_global_color("primary");

        // Override table styling with Material theme
        let mut style = (*ui.ctx().style()).clone();
        style.visuals.widgets.noninteractive.bg_fill = surface;
        style.visuals.widgets.inactive.bg_fill = surface;
        style.visuals.widgets.hovered.bg_fill =
            egui::Color32::from_rgba_premultiplied(primary.r(), primary.g(), primary.b(), 20);
        style.visuals.widgets.active.bg_fill =
            egui::Color32::from_rgba_premultiplied(primary.r(), primary.g(), primary.b(), 40);
        style.visuals.selection.bg_fill = primary;
        style.visuals.widgets.noninteractive.fg_stroke.color = on_surface;
        style.visuals.widgets.inactive.fg_stroke.color = on_surface;
        style.visuals.widgets.hovered.fg_stroke.color = on_surface;
        style.visuals.widgets.active.fg_stroke.color = on_surface;
        style.visuals.striped = true;
        style.visuals.faint_bg_color = egui::Color32::from_rgba_premultiplied(
            on_surface.r(),
            on_surface.g(),
            on_surface.b(),
            10,
        );
        ui.ctx().set_style(style);

        // Get viewport width for responsive design
        let available_width = ui.ctx().screen_rect().width();
        let is_desktop = available_width >= DESKTOP_MIN_WIDTH;
        let width_ratio = if is_desktop { available_width / BASE_TABLE_WIDTH } else { 1.0 };
        // log::debug!(
        //     "Viewport width: {}, is_desktop: {}, width_ratio: {}",
        //     available_width,
        //     is_desktop,
        //     width_ratio
        // );

        // Use the data_table widget with proportional column widths
        let mut interactive_table = data_table()
            .id(egui::Id::new("apps_control_data_table"));
        if is_desktop {
            interactive_table = interactive_table
                .column(tr!("category"), 200.0 * width_ratio, false)
                .column(tr!("app-name"), 200.0 * width_ratio, false)
                .column(tr!("links"), 298.0 * width_ratio, false)
                .column(tr!("install"), 300.0 * width_ratio, false);
        } else {
            interactive_table = interactive_table
                .column(tr!("app-name"), available_width * 0.45, false)
                .column(tr!("install"), available_width * 0.55, false);
        }
        interactive_table = interactive_table.allow_selection(false);

        // Track which app's install button was clicked
        let mut install_clicked_app: Option<AppEntry> = None;

        // Add rows to the table
        for (idx, app) in self.app_entries.clone().iter().enumerate() {
            // Filter based on show_only_installable setting
            let downloadable_link = self.get_downloadable_link(&app);
            if self.show_only_installable && downloadable_link.is_none() {
                continue; // Skip apps without downloadable links
            }

            // Filter based on text filter
            if !self.matches_text_filter(&app) {
                continue;
            }

            let app_for_links = app.clone();
            let app_for_install = app.clone();
            let app_name = app.name.clone();
            let is_installed = self.is_app_installed(&app);

            // Get installed package info for action buttons
            let installed_pkg_info = self.get_installed_package_info(&app);

            // Check if this app is currently being installed
            let install_status = self.installing_apps.get(&app_name).cloned();

            interactive_table = interactive_table.row(|row| {
                // Category and App Name columns (desktop: both, mobile: name only)
                let row_builder = if is_desktop {
                    row.cell(&app.category).cell(&app.name)
                } else {
                    row.cell(&app.name)
                };

                // Links column (desktop only)
                let row_builder = if is_desktop {
                    row_builder.widget_cell(move |ui: &mut egui::Ui| {
                        egui::ScrollArea::horizontal()
                            .id_salt(format!("links_scroll_{}_{}", idx, app_for_links.category))
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    let num_links = app_for_links.links.len();
                                    let estimated_width = (num_links as f32) * 44.0;
                                    ui.set_min_width(estimated_width);

                                    for (url, link_type) in &app_for_links.links {
                                        let svg = match link_type.as_str() {
                                            "fdroid" => FDROID_SVG,
                                            "izzy" => IZZYONDROID_SVG,
                                            "github" => GITHUB_SVG,
                                            "gitlab" => GITLAB_SVG,
                                            "fdroid-downloadable" => FDROID_SVG,
                                            "izzy-downloadable" => IZZYONDROID_SVG,
                                            "github-downloadable" => GITHUB_SVG,
                                            "gitlab-downloadable" => GITLAB_SVG,
                                            "googleplay" => GOOGLEPLAY_SVG,
                                            "reddit" => REDDIT_SVG,
                                            "discord" => DISCORD_SVG,
                                            "matrix" => MATRIX_SVG,
                                            "telegram" => TELEGRAM_SVG,
                                            "youtube" => YOUTUBE_SVG,
                                            "source" => SOURCE_SVG,
                                            _ => HOME_SVG,
                                        };

                                        let response = ui
                                            .add(icon_button_standard("")
                                                .svg_data(svg))
                                            .on_hover_text(url.as_str());

                                        if response.clicked()
                                        {
                                            #[cfg(not(target_os = "android"))]
                                            {
                                                if let Err(e) = webbrowser::open(url) {
                                                    log::error!("Failed to open URL: {}", e);
                                                }
                                            }
                                        }
                                    }
                                });
                            });
                    })
                } else {
                    row_builder
                };

                // Install/Actions column (always)
                row_builder.widget_cell(move |ui: &mut egui::Ui| {
                    ui.horizontal(|ui| {
                        if let Some(ref status) = install_status {
                            ui.label(status);
                        } else if is_installed {
                            ui.add(icon_button_standard(ICON_CHECK_BOX.to_string())).on_hover_text(tr!("installed"));

                            if let Some((ref pkg_name, is_system, ref enabled_state)) = installed_pkg_info {
                                if enabled_state == "DEFAULT" || enabled_state == "ENABLED" {
                                    if ui.add(icon_button_standard(ICON_DELETE.to_string())).on_hover_text(tr!("uninstall")).clicked() {
                                        ui.data_mut(|data| {
                                            data.insert_temp(
                                                egui::Id::new("apps_uninstall_clicked_package"),
                                                pkg_name.clone(),
                                            );
                                            data.insert_temp(
                                                egui::Id::new("apps_uninstall_clicked_is_system"),
                                                is_system,
                                            );
                                            data.insert_temp(
                                                egui::Id::new("apps_uninstall_clicked_app_name"),
                                                app_for_install.name.clone(),
                                            );
                                        });
                                    }
                                }

                                if enabled_state == "DISABLED" || enabled_state == "DISABLED_USER" {
                                    if ui.add(icon_button_standard(ICON_CHECK_CIRCLE.to_string())).on_hover_text(tr!("enable")).clicked() {
                                        ui.data_mut(|data| {
                                            data.insert_temp(
                                                egui::Id::new("apps_enable_clicked_package"),
                                                pkg_name.clone(),
                                            );
                                        });
                                    }
                                }

                                if enabled_state == "DEFAULT" || enabled_state == "ENABLED" {
                                    if ui.add(icon_button_standard(ICON_CANCEL.to_string())).on_hover_text(tr!("disable")).clicked() {
                                        ui.data_mut(|data| {
                                            data.insert_temp(
                                                egui::Id::new("apps_disable_clicked_package"),
                                                pkg_name.clone(),
                                            );
                                        });
                                    }
                                }
                            }
                        } else if let Some((ref url, ref link_type)) = downloadable_link {
                            let hover_text = format!("[{}]\n{}", link_type, url);

                            if ui.add(icon_button_standard(ICON_DOWNLOAD.to_string())).on_hover_text(&hover_text).clicked() {
                                ui.data_mut(|data| {
                                    data.insert_temp(egui::Id::new("install_clicked_app"), app_for_install.clone());
                                });
                            }
                        }
                    });
                })
            });
        }

        interactive_table.show(ui);

        // Track action button clicks
        let mut uninstall_package: Option<String> = None;
        let mut uninstall_is_system = false;
        let mut uninstall_app_name: Option<String> = None;
        let mut enable_package: Option<String> = None;
        let mut disable_package: Option<String> = None;

        // Check if install was clicked and trigger installation
        ui.data_mut(|data| {
            if let Some(app) = data.get_temp::<AppEntry>(egui::Id::new("install_clicked_app")) {
                install_clicked_app = Some(app);
                data.remove::<AppEntry>(egui::Id::new("install_clicked_app"));
            }

            // Check for action button clicks
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("apps_uninstall_clicked_package")) {
                uninstall_package = Some(pkg);
                uninstall_is_system = data
                    .get_temp::<bool>(egui::Id::new("apps_uninstall_clicked_is_system"))
                    .unwrap_or(false);
                uninstall_app_name = data.get_temp::<String>(egui::Id::new("apps_uninstall_clicked_app_name"));
                data.remove::<String>(egui::Id::new("apps_uninstall_clicked_package"));
                data.remove::<bool>(egui::Id::new("apps_uninstall_clicked_is_system"));
                data.remove::<String>(egui::Id::new("apps_uninstall_clicked_app_name"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("apps_enable_clicked_package")) {
                enable_package = Some(pkg);
                data.remove::<String>(egui::Id::new("apps_enable_clicked_package"));
            }
            if let Some(pkg) = data.get_temp::<String>(egui::Id::new("apps_disable_clicked_package")) {
                disable_package = Some(pkg);
                data.remove::<String>(egui::Id::new("apps_disable_clicked_package"));
            }
        });

        // Perform installation if an app was clicked
        if let Some(app) = install_clicked_app {
            {
                match self.install_app(&app) {
                    Ok(()) => {
                        log::info!("Successfully installed: {}", app.name);
                    }
                    Err(e) => {
                        log::error!("Failed to install {}: {}", app.name, e);
                        has_error = true;
                    }
                }
            }
        }

        // Perform uninstall if clicked
        if let Some(pkg_name) = uninstall_package {
            if let Some(ref device) = self.selected_device {
                {
                    let uninstall_result = if uninstall_is_system {
                        crate::adb::uninstall_app_user(&pkg_name, device, None)
                    } else {
                        crate::adb::uninstall_app(&pkg_name, device)
                    };

                    match uninstall_result {
                        Ok(output) => {
                            log::info!("App uninstalled successfully: {}", output);
                            // Remove from recently_installed_apps if present
                            if let Some(app_name) = uninstall_app_name {
                                self.recently_installed_apps.remove(&app_name);
                            }
                            // Trigger refresh to update UI
                            self.refresh_pending = true;
                        }
                        Err(e) => {
                            log::error!("Failed to uninstall app({}): {}", pkg_name, e);
                            has_error = true;
                        }
                    }
                }
            } else {
                log::error!("No device selected for uninstall");
                has_error = true;
            }
        }

        // Perform enable if clicked
        if let Some(pkg_name) = enable_package {
            if let Some(ref device) = self.selected_device {
                {
                    match crate::adb::enable_app(&pkg_name, device) {
                        Ok(output) => {
                            log::info!("App enabled successfully: {}", output);
                            // Trigger refresh to update UI
                            self.refresh_pending = true;
                        }
                        Err(e) => {
                            log::error!("Failed to enable app: {}", e);
                            has_error = true;
                        }
                    }
                }
            } else {
                log::error!("No device selected for enable");
                has_error = true;
            }
        }

        // Perform disable if clicked
        if let Some(pkg_name) = disable_package {
            if let Some(ref device) = self.selected_device {
                {
                    match crate::adb::disable_app_current_user(&pkg_name, device, None) {
                        Ok(output) => {
                            log::info!("App disabled successfully: {}", output);
                            // Trigger refresh to update UI
                            self.refresh_pending = true;
                        }
                        Err(e) => {
                            log::error!("Failed to disable app: {}", e);
                            has_error = true;
                        }
                    }
                }
            } else {
                log::error!("No device selected for disable");
                has_error = true;
            }
        }
        has_error
    }
}

fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *on, "")
    });

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

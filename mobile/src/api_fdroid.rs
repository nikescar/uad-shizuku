pub use crate::api_fdroid_stt::*;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::NaiveDate;
use regex::Regex;
use std::io::Read;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Fetch F-Droid app details by package ID
pub fn fetch_app_details(package_id: &str) -> Result<FDroidAppInfo> {
    // Default to English for consistent parsing
    let url = format!("https://f-droid.org/en/packages/{}/", package_id);

    log::info!("Fetching F-Droid data for package: {}", package_id);

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .call()
        .context("Failed to fetch F-Droid page")?;

    let html = response
        .into_string()
        .context("Failed to read response body")?;

    parse_app_details(package_id, &html)
}

/// Parse HTML to extract app information
pub fn parse_app_details(package_id: &str, html: &str) -> Result<FDroidAppInfo> {
    log::debug!("Parsing F-Droid data for package: {}", package_id);

    // Title
    let title_re = Regex::new(r#"<h3 class="package-name">\s*(.*?)\s*</h3>"#).unwrap();
    let title = title_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Developer (Author)
    let dev_re =
        Regex::new(r#"<li class="package-link" id="author">[^<]*<a href="[^"]*">\s*(.*?)\s*</a>"#)
            .unwrap();
    let developer = dev_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Version
    // Look for <b>Version X.Y.Z</b> or <b>버전 X.Y.Z</b> or similar generic pattern
    let version_re = Regex::new(r#"<b>(?:Version|버전)\s+([^<]+)</b>"#).unwrap();
    let version = version_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string());

    // Icon
    let icon_re = Regex::new(r#"<img class="package-icon" src="([^"]+)"#).unwrap();
    let icon_base64 = if let Some(cap) = icon_re.captures(html) {
        if let Some(src) = cap.get(1) {
            download_image_as_base64(src.as_str()).ok()
        } else {
            None
        }
    } else {
        None
    };

    // Description
    let desc_re =
        Regex::new(r#"<div class="package-description" dir="auto">([\s\S]*?)</div>"#).unwrap();
    let description = desc_re.captures(html).and_then(|c| c.get(1)).map(|m| {
        // Simple cleanup of <br>
        m.as_str().replace("<br>", "\n").trim().to_string()
    });

    // License
    let license_re =
        Regex::new(r#"<li class="package-link" id="license">[^<]*<a href="[^"]*">([\s\S]*?)</a>"#)
            .unwrap();
    let license = license_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string());

    // Updated
    // Find date in format YYYY-MM-DD or "Added on MMM DD, YYYY"
    let date_iso_re = Regex::new(r#"(\d{4}-\d{2}-\d{2})"#).unwrap();
    // English format: Added on Dec 18, 2025. Regex needs to match "Dec 18, 2025"
    let date_en_re = Regex::new(r#"Added on ([A-Z][a-z]{2} \d{1,2}, \d{4})"#).unwrap();

    let updated = if let Some(cap) = date_iso_re.captures(html) {
        cap.get(1)
            .and_then(|m| parse_date_to_timestamp(m.as_str()))
            .unwrap_or(0)
    } else if let Some(cap) = date_en_re.captures(html) {
        cap.get(1)
            .and_then(|m| parse_en_date_to_timestamp(m.as_str()))
            .unwrap_or(0)
    } else {
        0
    };

    // Convert to Option<i32> for struct
    let updated = if updated > 0 { Some(updated) } else { None };

    Ok(FDroidAppInfo {
        package_id: package_id.to_string(),
        title,
        developer,
        version,
        icon_base64,
        description,
        license,
        updated,
        raw_response: html.to_string(),
    })
}

/// Download image and convert to base64
fn download_image_as_base64(url: &str) -> Result<String> {
    log::debug!("Downloading image from: {}", url);

    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .context("Failed to download image")?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .context("Failed to read image bytes")?;

    let base64_str = general_purpose::STANDARD.encode(&bytes);

    // Determine MIME type from URL or content
    let mime_type = if url.contains(".png") {
        "image/png"
    } else if url.contains(".jpg") || url.contains(".jpeg") {
        "image/jpeg"
    } else if url.contains(".webp") {
        "image/webp"
    } else {
        "image/png" // default
    };

    Ok(format!("data:{};base64,{}", mime_type, base64_str))
}

fn parse_date_to_timestamp(date_str: &str) -> Option<i32> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp() as i32)
}

fn parse_en_date_to_timestamp(date_str: &str) -> Option<i32> {
    NaiveDate::parse_from_str(date_str, "%b %d, %Y")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fdroid_html() {
        let html = r#"
        <h3 class="package-name">
            Fossify Gallery
        </h3>
        <li class="package-link" id="author">
            Author:
            <a href="mailto:hello@fossify.org">
                Fossify
            </a>
        </li>
        <div class="package-version-header">
            <b>Version 1.10.0</b> (24)
        </div>
        <img class="package-icon" src="https://example.com/icon.png" alt="icon" />
        <div class="package-description" dir="auto">
            Description here<br>New line
        </div>
        <li class="package-link" id="license">
            License:
            <a href="...">GNU General Public License v3.0 only</a>
        </li>
        Added on 2025-12-18
        "#;

        let result = parse_app_details("org.fossify.gallery", html).unwrap();

        assert_eq!(result.title, "Fossify Gallery");
        assert_eq!(result.developer, "Fossify");
        assert_eq!(result.version, Some("1.10.0".to_string()));
        assert_eq!(
            result.license,
            Some("GNU General Public License v3.0 only".to_string())
        );
        assert!(result.updated.is_some());
    }
}

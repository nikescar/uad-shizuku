pub use crate::api_googleplay_stt::*;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use jsonpath_lib::select;
use regex::Regex;
use serde_json::Value;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Fetch Google Play app details by package ID
pub fn fetch_app_details(package_id: &str) -> Result<GooglePlayAppInfo> {
    let url = format!(
        "https://play.google.com/store/apps/details?id={}",
        package_id
    );

    log::info!("Fetching Google Play data for package: {}", package_id);

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .call()
        .context("Failed to fetch Google Play page")?;

    let html = response
        .into_string()
        .context("Failed to read response body")?;

    parse_app_details(package_id, &html)
}

/// Parse HTML to extract app information
pub fn parse_app_details(package_id: &str, html: &str) -> Result<GooglePlayAppInfo> {
    log::debug!("Parsing Google Play data for package: {}", package_id);

    // Extract JSON data from the script tag
    let json_data = extract_json_from_html(html)?;

    // Parse title
    let title = extract_title(&json_data)?;

    // Parse developer
    let developer = extract_developer(&json_data)?;

    // Parse version
    let version = extract_version(&json_data);

    // Parse icon URL and convert to base64
    let icon_base64 = extract_icon(&json_data)?;

    // Parse score
    let score = extract_score(&json_data);

    // Parse installs
    let installs = extract_installs(&json_data);

    // Parse updated timestamp
    let updated = extract_updated(&json_data);

    Ok(GooglePlayAppInfo {
        package_id: package_id.to_string(),
        title,
        developer,
        version,
        icon_base64,
        score,
        installs,
        updated,
        raw_response: json_data.to_string(),
    })
}

/// Extract JSON data from HTML script tags
fn extract_json_from_html(html: &str) -> Result<Value> {
    // Find the AF_initDataCallback script containing ds:5
    // Format: AF_initDataCallback({key: 'ds:5', hash: '12', data:[...], sideChannel: {}});
    let re = Regex::new(r#"AF_initDataCallback\(\{key:\s*'ds:5',\s*hash:\s*'[^']*',\s*data:\s*(\[.+?\]),\s*sideChannel:"#)
        .context("Failed to create regex")?;

    if let Some(captures) = re.captures(html) {
        if let Some(json_str) = captures.get(1) {
            let json_str = json_str.as_str();
            log::debug!("Found ds:5 JSON data, length: {}", json_str.len());

            let json: Value =
                serde_json::from_str(json_str).context("Failed to parse JSON from HTML")?;

            return Ok(json);
        }
    }

    anyhow::bail!("Could not find ds:5 data in HTML")
}

/// Extract title using JSONPath: $['ds:5'][1][2][0][0]
fn extract_title(json: &Value) -> Result<String> {
    let path = "$[1][2][0][0]";
    let result = select(json, path)?;

    if let Some(title_value) = result.first() {
        if let Some(title) = title_value.as_str() {
            return Ok(title.to_string());
        }
    }

    anyhow::bail!("Could not extract title")
}

/// Extract developer using JSONPath: $['ds:5'][1][2][68][0]
fn extract_developer(json: &Value) -> Result<String> {
    let path = "$[1][2][68][0]";
    let result = select(json, path)?;

    if let Some(dev_value) = result.first() {
        if let Some(developer) = dev_value.as_str() {
            return Ok(developer.to_string());
        }
    }

    anyhow::bail!("Could not extract developer")
}

/// Extract version using JSONPath: $['ds:5'][1][2][140][0][0][0]
fn extract_version(json: &Value) -> Option<String> {
    let paths = vec!["$[1][2][140][0][0][0]", "$[1][2][-1]['141'][0][0][0]"];

    for path in paths {
        if let Ok(result) = select(json, path) {
            if let Some(version_value) = result.first() {
                if let Some(version) = version_value.as_str() {
                    if !version.is_empty() && version != "VARY" {
                        return Some(version.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Extract icon URL and convert to base64
fn extract_icon(json: &Value) -> Result<Option<String>> {
    let path = "$[1][2][95][0][3][2]";
    let result = select(json, path)?;

    if let Some(icon_value) = result.first() {
        if let Some(icon_url) = icon_value.as_str() {
            log::debug!("Found icon URL: {}", icon_url);

            // Download the icon and convert to base64
            match download_image_as_base64(icon_url) {
                Ok(base64_str) => return Ok(Some(base64_str)),
                Err(e) => {
                    log::warn!("Failed to download icon: {}", e);
                    return Ok(None);
                }
            }
        }
    }

    Ok(None)
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

/// Extract score using JSONPath: $['ds:5'][1][2][51][0][1]
fn extract_score(json: &Value) -> Option<f32> {
    let path = "$[1][2][51][0][1]";

    if let Ok(result) = select(json, path) {
        if let Some(score_value) = result.first() {
            if let Some(score) = score_value.as_f64() {
                return Some(score as f32);
            }
        }
    }

    None
}

/// Extract installs using JSONPath: $['ds:5'][1][2][13][0]
fn extract_installs(json: &Value) -> Option<String> {
    let path = "$[1][2][13][0]";

    if let Ok(result) = select(json, path) {
        if let Some(installs_value) = result.first() {
            if let Some(installs) = installs_value.as_str() {
                return Some(installs.to_string());
            }
        }
    }

    None
}

/// Extract updated timestamp using JSONPath: $['ds:5'][1][2][145][0][1][0]
fn extract_updated(json: &Value) -> Option<i32> {
    let paths = vec!["$[1][2][145][0][1][0]", "$[1][2][-1]['146'][0][1][0]"];

    for path in paths {
        if let Ok(result) = select(json, path) {
            if let Some(updated_value) = result.first() {
                if let Some(timestamp) = updated_value.as_i64() {
                    // Convert from milliseconds to seconds if needed
                    if timestamp > 10000000000 {
                        return Some((timestamp / 1000) as i32);
                    }
                    return Some(timestamp as i32);
                }
            }
        }
    }

    None
}

/// Parse HTML file for testing
#[allow(dead_code)]
pub fn parse_html_file(package_id: &str, file_path: &str) -> Result<GooglePlayAppInfo> {
    use std::fs;

    log::info!("Parsing HTML file: {}", file_path);
    let html = fs::read_to_string(file_path).context("Failed to read HTML file")?;

    parse_app_details(package_id, &html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html_file() {
        // This test requires the reference file to be present
        let result = parse_html_file("org.fossify.gallery", "../reference/googleplay_detail.html");

        match result {
            Ok(app_info) => {
                println!("Package ID: {}", app_info.package_id);
                println!("Title: {}", app_info.title);
                println!("Developer: {}", app_info.developer);
                println!("Version: {:?}", app_info.version);
                println!("Score: {:?}", app_info.score);
                println!("Installs: {:?}", app_info.installs);
                println!("Updated: {:?}", app_info.updated);
                println!(
                    "Icon Base64 length: {}",
                    app_info.icon_base64.as_ref().map(|s| s.len()).unwrap_or(0)
                );

                assert!(!app_info.title.is_empty());
                assert!(!app_info.developer.is_empty());
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}

pub use crate::api_apkmirror_stt::*;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use std::io::Read;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Fetch APKMirror app details by package ID (search and get first result)
pub fn fetch_app_details(package_id: &str, email: &str) -> Result<ApkMirrorAppInfo> {
    let url = format!(
        "https://www.apkmirror.com/?post_type=app_release&searchtype=app&sortby=date&sort=desc&s={}",
        package_id
    );

    tracing::info!("Fetching APKMirror data for package: {}", package_id);

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .set("Accept-Language", "en-US,en;q=0.9")
        .set(
            "Cookie",
            &format!("usprivacy=1---; apkmirror_email={}", email),
        )
        .call()
        .context("Failed to fetch APKMirror search page")?;

    let html = response
        .into_string()
        .context("Failed to read response body")?;

    parse_app_details(package_id, &html)
}

/// Parse HTML to extract first search result app information
pub fn parse_app_details(package_id: &str, html: &str) -> Result<ApkMirrorAppInfo> {
    tracing::debug!("Parsing APKMirror data for package: {}", package_id);

    // Check for "no results" message - treat as 404
    if html.contains("No results found matching your query") {
        tracing::info!(
            "APKMirror returned 'No results found' for package: {}",
            package_id
        );
        return Ok(ApkMirrorAppInfo {
            package_id: package_id.to_string(),
            title: "Unknown".to_string(),
            developer: "Unknown".to_string(),
            version: None,
            icon_url: None,
            icon_base64: None,
            raw_response: html.to_string(),
        });
    }

    // Find first search result widget - look for appRowTitle class
    // <h5 title="App Name" class="appRowTitle ...">...</h5>
    // We match the text content inside the <a> tag as it's more robust against attribute ordering
    let title_re =
        Regex::new(r#"<h5[^>]*class="[^"]*appRowTitle[^"]*"[^>]*>[\s\S]*?<a[^>]*>(.*?)</a>"#)
            .unwrap();

    let title = title_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Find developer name from "by DeveloperName"
    // <a class="byDeveloper ..." ...>by Developer Name</a>
    let dev_re = Regex::new(r#"class="[^"]*byDeveloper[^"]*"[^>]*>(?:by\s+)?(.*?)</a"#).unwrap();

    let developer = dev_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // Find app icon from the search result
    // <img class="ellipsisText" ... src="...">
    let icon_re =
        Regex::new(r#"<img[^>]*class="[^"]*ellipsisText[^"]*"[^>]*src="([^"]+)"[^>]*>"#).unwrap();

    let icon_url = icon_re.captures(html).and_then(|c| c.get(1)).map(|m| {
        let url = m.as_str().trim().to_string();

        // Handle APKMirror resize script URL
        if url.contains("ap_resize.php") {
            if let Some(src_start) = url.find("src=") {
                let rest = &url[src_start + 4..];
                let end = rest.find('&').unwrap_or(rest.len());
                let encoded_url = &rest[..end];

                // Simple URL decoding
                let decoded_url = encoded_url
                    .replace("%3A", ":")
                    .replace("%2F", "/")
                    .replace("%2B", "+")
                    .replace("%3F", "?")
                    .replace("%3D", "=")
                    .replace("%26", "&");

                return decoded_url;
            }
        }

        // Make sure URL is absolute
        if url.starts_with("//") {
            format!("https:{}", url)
        } else if url.starts_with("/") {
            format!("https://www.apkmirror.com{}", url)
        } else {
            url
        }
    });

    // Find version from infoSlide
    // <span class="infoSlide-name">Version:</span><span class="infoSlide-value">2.5.8                    </span>
    let version_re = Regex::new(
        r#"<span[^>]*class="[^"]*infoSlide-name[^"]*"[^>]*>Version:</span>\s*<span[^>]*class="[^"]*infoSlide-value[^"]*"[^>]*>([^<]+)</span>"#,
    )
    .unwrap();

    let version = version_re
        .captures(html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string());

    // Download icon if URL found
    let icon_base64 = icon_url
        .as_ref()
        .and_then(|url| download_image_as_base64(url).ok());

    Ok(ApkMirrorAppInfo {
        package_id: package_id.to_string(),
        title,
        developer,
        version,
        icon_url,
        icon_base64,
        raw_response: html.to_string(),
    })
}

/// Check if APK file already exists on APKMirror
pub fn check_apk_uploadable(md5_hash: &str, email: &str) -> Result<bool> {
    let url = format!(
        "https://www.apkmirror.com/wp-json/apkm/v1/apk_uploadable/{}",
        md5_hash
    );

    tracing::info!("Checking if APK is uploadable: {}", md5_hash);

    let response = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "*/*")
        .set("Accept-Language", "en-US,en;q=0.9")
        .set("Referer", "https://www.apkmirror.com/")
        .set(
            "Cookie",
            &format!("usprivacy=1---; apkmirror_email={}", email),
        )
        .call()
        .context("Failed to check APK uploadability")?;

    let status = response.status();

    // If status is 200, the APK can be uploaded (doesn't exist yet)
    // If it returns an error or specific response, it already exists
    Ok(status == 200)
}

/// Upload APK file to APKMirror
pub fn upload_apk(apk_path: &str, name: &str, email: &str) -> Result<ApkMirrorUploadResult> {
    use std::fs::File;
    use std::time::{SystemTime, UNIX_EPOCH};

    let url = "https://www.apkmirror.com/wp-json/apkm/v1/upload/";

    tracing::info!("Uploading APK to APKMirror: {}", apk_path);

    // Read APK file
    let mut file = File::open(apk_path).context("Failed to open APK file")?;

    let mut apk_bytes = Vec::new();
    file.read_to_end(&mut apk_bytes)
        .context("Failed to read APK file")?;

    // Get filename from path
    let filename = std::path::Path::new(apk_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload.apk");

    // Build multipart form data using timestamp for unique boundary
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let boundary = format!("----geckoformboundary{:x}", timestamp);

    let mut body = Vec::new();

    // Add fullname field (required by APKMirror API)
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"fullname\"\r\n\r\n");
    body.extend_from_slice(name.as_bytes());
    body.extend_from_slice(b"\r\n");

    // Add email field (required by APKMirror API)
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"email\"\r\n\r\n");
    body.extend_from_slice(email.as_bytes());
    body.extend_from_slice(b"\r\n");

    // Add file field
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
            filename
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/vnd.android.package-archive\r\n\r\n");
    body.extend_from_slice(&apk_bytes);
    body.extend_from_slice(b"\r\n");

    // End boundary
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let result = ureq::post(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "*/*")
        .set("Accept-Language", "en-US,en;q=0.9")
        .set("X-Requested-With", "XMLHttpRequest")
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={}", boundary),
        )
        .set("Origin", "https://www.apkmirror.com")
        .set("Referer", "https://www.apkmirror.com/")
        .set(
            "Cookie",
            &format!(
                "usprivacy=1---; apkmirror_name={}; apkmirror_email={}",
                name, email
            ),
        )
        .send_bytes(&body);

    match result {
        Ok(response) => {
            let status = response.status();
            let response_text = response.into_string().unwrap_or_default();
            tracing::info!("Upload response ({}): {}", status, response_text);

            if status == 200 {
                // Parse JSON response to check actual success status
                // Response format: {"success":true/false,"data":"message"}
                let parsed = parse_upload_response(&response_text);

                Ok(ApkMirrorUploadResult {
                    success: parsed.success,
                    already_exists: parsed.already_exists,
                    rate_limited: parsed.rate_limited,
                    message: response_text,
                })
            } else {
                Ok(ApkMirrorUploadResult {
                    success: false,
                    already_exists: status == 409,
                    rate_limited: false,
                    message: response_text,
                })
            }
        }
        Err(ureq::Error::Status(status, response)) => {
            let response_text = response.into_string().unwrap_or_default();
            tracing::error!("Upload failed with status {}: {}", status, response_text);

            Ok(ApkMirrorUploadResult {
                success: false,
                already_exists: status == 409,
                rate_limited: status == 429,
                message: format!("HTTP {}: {}", status, response_text),
            })
        }
        Err(e) => Err(anyhow::anyhow!("Failed to upload APK: {}", e)),
    }
}

/// Parsed upload response info
struct ParsedUploadResponse {
    success: bool,
    rate_limited: bool,
    already_exists: bool,
}

/// Parse upload API response to extract success status, rate limit, and already exists info
/// Response format: {"success":true/false,"data":"message"}
fn parse_upload_response(response_text: &str) -> ParsedUploadResponse {
    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response_text) {
        let success = json
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let data = json.get("data").and_then(|v| v.as_str()).unwrap_or("");

        // Check for rate limit message
        let rate_limited = data.contains("Too many APKs") && data.contains("24 hours");

        // Check for already exists message
        let already_exists = data.contains("we already have a similar APK");

        ParsedUploadResponse {
            success,
            rate_limited,
            already_exists,
        }
    } else {
        // If not JSON, assume success if we got a 200 response
        ParsedUploadResponse {
            success: true,
            rate_limited: false,
            already_exists: false,
        }
    }
}

/// Download image and convert to base64
fn download_image_as_base64(url: &str) -> Result<String> {
    tracing::debug!("Downloading image from: {}", url);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_apkmirror_html() {
        let html = r#"
        <div class="appRow">
            <div class="table-row">
                <div style="width: 56px;" class="table-cell">
                    <div class="bubble-wrap p-relative">
                        <img class="ellipsisText" style="width:32px; height:32px;" alt="Google Play services 25.47.63"  src="/wp-content/themes/APKMirror/ap_resize/ap_resize.php?src=https%3A%2F%2Fdownloadr2.apkmirror.com%2Fwp-content%2Fuploads%2F2021%2F07%2F80%2F60ec9b7cad6dc.png&w=32&h=32&q=100" />
                    </div>
                </div>
                <div class="table-cell">
                    <h5 title="Google Play services 25.47.63" class="appRowTitle wrapText marginZero block-on-mobile">
                        <a class="fontBlack" href="/apk/google-inc/google-play-services/google-play-services-25-47-63-release/">Google Play services 25.47.63</a>
                    </h5>
                </div>
                <div class="table-cell">
                    <a href="/apk/google-inc/" class="byDeveloper block-on-mobile wrapText">by Google LLC</a>
                </div>
            </div>
        </div>
        "#;

        let result = parse_app_details("com.google.android.gms", html).unwrap();

        assert_eq!(result.title, "Google Play services 25.47.63");
        assert_eq!(result.developer, "Google LLC");
        assert_eq!(result.version, None); // No version in this HTML snippet
        assert_eq!(
            result.icon_url,
            Some(
                "https://downloadr2.apkmirror.com/wp-content/uploads/2021/07/80/60ec9b7cad6dc.png"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_parse_version() {
        let html = r#"
        <div class="appRow">
            <h5 class="appRowTitle"><a>Test App</a></h5>
            <a class="byDeveloper">by Test Developer</a>
            <span class="infoSlide-name">Version:</span><span class="infoSlide-value">2.5.8                    </span>
        </div>
        "#;

        let result = parse_app_details("com.test.app", html).unwrap();

        assert_eq!(result.title, "Test App");
        assert_eq!(result.developer, "Test Developer");
        assert_eq!(result.version, Some("2.5.8".to_string()));
    }

    #[test]
    fn test_parse_no_results_found() {
        let html = r#"
        <div class="searchContent">
            <p>No results found matching your query</p>
        </div>
        "#;

        let result = parse_app_details("com.nonexistent.app", html).unwrap();

        // Should be treated as not found (Unknown/Unknown triggers 404 handling in caller)
        assert_eq!(result.title, "Unknown");
        assert_eq!(result.developer, "Unknown");
        assert_eq!(result.version, None);
        assert_eq!(result.icon_url, None);
    }
}

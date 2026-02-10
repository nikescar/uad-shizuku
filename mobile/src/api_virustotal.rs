pub use crate::api_virustotal_stt::*;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Error types for VirusTotal API
#[derive(Debug)]
pub enum VtError {
    NotFound,
    RateLimit { retry_after: u64 },
    HttpError(Box<dyn Error>),
    IoError(std::io::Error),
    ParseError(Box<dyn Error>),
}

impl std::fmt::Display for VtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VtError::NotFound => write!(f, "File not found in VirusTotal"),
            VtError::RateLimit { retry_after } => write!(
                f,
                "Rate limit exceeded, retry after {} seconds",
                retry_after
            ),
            VtError::HttpError(e) => write!(f, "HTTP error: {}", e),
            VtError::IoError(e) => write!(f, "IO error: {}", e),
            VtError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl Error for VtError {}

impl From<std::io::Error> for VtError {
    fn from(err: std::io::Error) -> Self {
        VtError::IoError(err)
    }
}

/// Fetch file report from VirusTotal API (blocking)
pub fn get_file_report(sha256: &str, api_key: &str) -> Result<VirusTotalResponse, VtError> {
    let url = format!("https://www.virustotal.com/api/v3/files/{}", sha256);

    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .set("accept", "application/json")
        .set("x-apikey", api_key)
        .call();

    match response {
        Ok(resp) => {
            log::trace!(
                "VirusTotal file report HTTP response status: {}",
                resp.status()
            );
            let response_text = resp
                .into_string()
                .map_err(|e| VtError::HttpError(Box::new(e)))?;
            log::trace!(
                "VirusTotal file report HTTP response body: {}",
                response_text
            );
            let vt_response: VirusTotalResponse = serde_json::from_str(&response_text)
                .map_err(|e| VtError::ParseError(Box::new(e)))?;
            Ok(vt_response)
        }
        Err(ureq::Error::Status(code, resp)) => {
            log::trace!("VirusTotal file report HTTP error status: {}", code);
            if code == 404 {
                Err(VtError::NotFound)
            } else if code == 429 {
                // Parse retry-after header or response body for wait time
                let retry_after = if let Some(header) = resp.header("Retry-After") {
                    header.parse::<u64>().unwrap_or(60)
                } else {
                    // Try to parse from response body
                    match resp.into_string() {
                        Ok(body) => {
                            // Try to extract seconds from error message
                            // VirusTotal typically returns something like "Quota exceeded. Please wait 60 seconds"
                            extract_wait_seconds(&body).unwrap_or(60)
                        }
                        Err(_) => 60,
                    }
                };
                Err(VtError::RateLimit { retry_after })
            } else {
                let err_msg = format!("HTTP error {}", code);
                Err(VtError::HttpError(err_msg.into()))
            }
        }
        Err(e) => Err(VtError::HttpError(Box::new(e))),
    }
}

/// Upload a file to VirusTotal for analysis
pub fn upload_file(file_path: &Path, api_key: &str) -> Result<VirusTotalUploadResponse, VtError> {
    let url = "https://www.virustotal.com/api/v3/files";

    // Read the file
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let _filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file.apk");

    // Create multipart form using ureq_multipart
    let (boundary, body) = ureq_multipart::MultipartBuilder::new()
        .add_file("file", file_path)
        .map_err(|e| VtError::HttpError(Box::new(e)))?
        .finish()
        .map_err(|e| VtError::HttpError(Box::new(e)))?;

    let response = ureq::post(url)
        .timeout(std::time::Duration::from_secs(60))
        .set("accept", "application/json")
        .set("x-apikey", api_key)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={}", boundary),
        )
        .send_bytes(&body);

    match response {
        Ok(resp) => {
            log::trace!("VirusTotal upload HTTP response status: {}", resp.status());
            let response_text = resp
                .into_string()
                .map_err(|e| VtError::HttpError(Box::new(e)))?;
            log::trace!("VirusTotal upload HTTP response body: {}", response_text);
            let vt_response: VirusTotalUploadResponse = serde_json::from_str(&response_text)
                .map_err(|e| VtError::ParseError(Box::new(e)))?;
            Ok(vt_response)
        }
        Err(ureq::Error::Status(code, resp)) => {
            log::trace!("VirusTotal upload HTTP error status: {}", code);
            if code == 429 {
                let retry_after = if let Some(header) = resp.header("Retry-After") {
                    header.parse::<u64>().unwrap_or(60)
                } else {
                    match resp.into_string() {
                        Ok(body) => extract_wait_seconds(&body).unwrap_or(60),
                        Err(_) => 60,
                    }
                };
                Err(VtError::RateLimit { retry_after })
            } else {
                let err_msg = format!("HTTP error {}", code);
                Err(VtError::HttpError(err_msg.into()))
            }
        }
        Err(e) => Err(VtError::HttpError(Box::new(e))),
    }
}

/// Maximum file size for standard upload (32MB)
const MAX_STANDARD_UPLOAD_SIZE: u64 = 32 * 1024 * 1024;

/// Get upload URL for large files (>32MB)
pub fn get_upload_url(api_key: &str) -> Result<String, VtError> {
    let url = "https://www.virustotal.com/api/v3/files/upload_url";

    let response = ureq::get(url)
        .timeout(std::time::Duration::from_secs(60))
        .set("accept", "application/json")
        .set("x-apikey", api_key)
        .call();

    match response {
        Ok(resp) => {
            log::trace!(
                "VirusTotal upload URL HTTP response status: {}",
                resp.status()
            );
            let response_text = resp
                .into_string()
                .map_err(|e| VtError::HttpError(Box::new(e)))?;
            log::trace!(
                "VirusTotal upload URL HTTP response body: {}",
                response_text
            );
            let url_response: VirusTotalUploadUrlResponse = serde_json::from_str(&response_text)
                .map_err(|e| VtError::ParseError(Box::new(e)))?;
            Ok(url_response.data)
        }
        Err(ureq::Error::Status(code, resp)) => {
            log::trace!("VirusTotal upload URL HTTP error status: {}", code);
            if code == 429 {
                let retry_after = if let Some(header) = resp.header("Retry-After") {
                    header.parse::<u64>().unwrap_or(60)
                } else {
                    match resp.into_string() {
                        Ok(body) => extract_wait_seconds(&body).unwrap_or(60),
                        Err(_) => 60,
                    }
                };
                Err(VtError::RateLimit { retry_after })
            } else {
                let err_msg = format!("HTTP error {}", code);
                Err(VtError::HttpError(err_msg.into()))
            }
        }
        Err(e) => Err(VtError::HttpError(Box::new(e))),
    }
}

/// Upload a file to a specific URL (used for large file uploads)
fn upload_file_to_url(
    file_path: &Path,
    upload_url: &str,
    api_key: &str,
) -> Result<VirusTotalUploadResponse, VtError> {
    // Create multipart form using ureq_multipart
    let (boundary, body) = ureq_multipart::MultipartBuilder::new()
        .add_file("file", file_path)
        .map_err(|e| VtError::HttpError(Box::new(e)))?
        .finish()
        .map_err(|e| VtError::HttpError(Box::new(e)))?;

    // Use a longer timeout for large files (10 minutes)
    let response = ureq::post(upload_url)
        .timeout(std::time::Duration::from_secs(600))
        .set("accept", "application/json")
        .set("x-apikey", api_key)
        .set(
            "Content-Type",
            &format!("multipart/form-data; boundary={}", boundary),
        )
        .send_bytes(&body);

    match response {
        Ok(resp) => {
            log::trace!(
                "VirusTotal large upload HTTP response status: {}",
                resp.status()
            );
            let response_text = resp
                .into_string()
                .map_err(|e| VtError::HttpError(Box::new(e)))?;
            log::trace!(
                "VirusTotal large upload HTTP response body: {}",
                response_text
            );
            let vt_response: VirusTotalUploadResponse = serde_json::from_str(&response_text)
                .map_err(|e| VtError::ParseError(Box::new(e)))?;
            Ok(vt_response)
        }
        Err(ureq::Error::Status(code, resp)) => {
            log::trace!("VirusTotal large upload HTTP error status: {}", code);
            if code == 429 {
                let retry_after = if let Some(header) = resp.header("Retry-After") {
                    header.parse::<u64>().unwrap_or(60)
                } else {
                    match resp.into_string() {
                        Ok(body) => extract_wait_seconds(&body).unwrap_or(60),
                        Err(_) => 60,
                    }
                };
                Err(VtError::RateLimit { retry_after })
            } else {
                let err_msg = format!("HTTP error {}", code);
                Err(VtError::HttpError(err_msg.into()))
            }
        }
        Err(e) => Err(VtError::HttpError(Box::new(e))),
    }
}

/// Upload a large file to VirusTotal (>32MB) using the special upload URL
pub fn upload_large_file(
    file_path: &Path,
    api_key: &str,
) -> Result<VirusTotalUploadResponse, VtError> {
    // Get upload URL
    log::info!("Getting upload URL for large file");
    let upload_url = get_upload_url(api_key)?;
    log::info!("Got upload URL: {}", upload_url);

    // Upload to the special URL
    upload_file_to_url(file_path, &upload_url, api_key)
}

/// Smart upload function that chooses the right endpoint based on file size
pub fn upload_file_smart(
    file_path: &Path,
    api_key: &str,
) -> Result<VirusTotalUploadResponse, VtError> {
    let file_size = std::fs::metadata(file_path)?.len();
    let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
    log::info!("File size: {:.2} MB", file_size_mb);

    if file_size > MAX_STANDARD_UPLOAD_SIZE {
        log::info!("File is larger than 32MB, using large file upload endpoint");
        upload_large_file(file_path, api_key)
    } else {
        log::info!("File is smaller than 32MB, using standard upload endpoint");
        upload_file(file_path, api_key)
    }
}

/// Get analysis result by analysis ID
pub fn get_analysis(analysis_id: &str, api_key: &str) -> Result<VirusTotalResponse, VtError> {
    let url = format!("https://www.virustotal.com/api/v3/analyses/{}", analysis_id);

    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .set("accept", "application/json")
        .set("x-apikey", api_key)
        .call();

    match response {
        Ok(resp) => {
            log::trace!(
                "VirusTotal analysis HTTP response status: {}",
                resp.status()
            );
            let response_text = resp
                .into_string()
                .map_err(|e| VtError::HttpError(Box::new(e)))?;
            log::trace!("VirusTotal analysis HTTP response body: {}", response_text);

            // The analysis endpoint returns slightly different structure
            // Try to parse as file report first, if it has the right structure
            let vt_response: VirusTotalResponse = serde_json::from_str(&response_text)
                .map_err(|e| VtError::ParseError(Box::new(e)))?;
            Ok(vt_response)
        }
        Err(ureq::Error::Status(code, resp)) => {
            log::trace!("VirusTotal analysis HTTP error status: {}", code);
            if code == 429 {
                let retry_after = if let Some(header) = resp.header("Retry-After") {
                    header.parse::<u64>().unwrap_or(60)
                } else {
                    match resp.into_string() {
                        Ok(body) => extract_wait_seconds(&body).unwrap_or(60),
                        Err(_) => 60,
                    }
                };
                Err(VtError::RateLimit { retry_after })
            } else {
                let err_msg = format!("HTTP error {}", code);
                Err(VtError::HttpError(err_msg.into()))
            }
        }
        Err(e) => Err(VtError::HttpError(Box::new(e))),
    }
}

/// Extract wait seconds from error message
fn extract_wait_seconds(message: &str) -> Option<u64> {
    // Try to find a number followed by "second" or "seconds"
    let re = regex::Regex::new(r"(\d+)\s*second").ok()?;
    let caps = re.captures(message)?;
    caps.get(1)?.as_str().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_virustotal_response() {
        let json = include_str!("../../reference/virustotal_filereport_response.json");
        let response: VirusTotalResponse = serde_json::from_str(json).unwrap();

        assert_eq!(
            response.data.id,
            "6f2ca352440a0027b9f8ed014d40a1557df2b4b3d3e3fc06e574cc02ead982aa"
        );
        let stats = response
            .data
            .attributes
            .last_analysis_stats
            .as_ref()
            .unwrap();
        assert_eq!(stats.malicious, 0);
        assert_eq!(stats.suspicious, 0);
        assert_eq!(stats.undetected, 62);

        let dex_count = response
            .data
            .attributes
            .androguard
            .as_ref()
            .and_then(|a| a.risk_indicator.as_ref())
            .and_then(|r| r.apk.as_ref())
            .and_then(|a| a.dex);
        assert_eq!(dex_count, Some(8));
    }

    #[test]
    fn test_parse_upload_response() {
        let json = include_str!("../../reference/virustotal_uploadafile_response.json");
        let response: VirusTotalUploadResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.data.data_type, "analysis");
        assert!(response.data.id.contains("=="));
    }

    #[test]
    fn test_extract_wait_seconds() {
        assert_eq!(extract_wait_seconds("Please wait 60 seconds"), Some(60));
        assert_eq!(extract_wait_seconds("Retry after 120 second"), Some(120));
        assert_eq!(extract_wait_seconds("No number here"), None);
    }

    /// Integration test for VirusTotal upload functionality.
    /// Run with: VT_API_KEY=your_key cargo test test_virustotal_upload_integration -- --ignored --nocapture
    #[test]
    #[ignore]
    fn test_virustotal_upload_integration() {
        let api_key = std::env::var("VT_API_KEY").expect("VT_API_KEY environment variable not set");

        // Create a small test file with unique content
        let test_content = format!(
            "Test file for VirusTotal upload integration test - {}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let temp_dir = std::env::temp_dir();
        let test_file_path = temp_dir.join("vt_upload_test.txt");
        std::fs::write(&test_file_path, &test_content).expect("Failed to write test file");

        println!("Testing file upload to VirusTotal...");
        println!("Test file: {:?}", test_file_path);

        // Test upload
        let result = upload_file(&test_file_path, &api_key);

        // Clean up test file
        let _ = std::fs::remove_file(&test_file_path);

        match result {
            Ok(response) => {
                println!("Upload successful!");
                println!("Analysis ID: {}", response.data.id);
                println!("Type: {}", response.data.data_type);
                assert_eq!(response.data.data_type, "analysis");
                assert!(!response.data.id.is_empty());
            }
            Err(VtError::RateLimit { retry_after }) => {
                println!(
                    "Rate limited (expected if running multiple tests). Retry after: {} seconds",
                    retry_after
                );
                // This is acceptable - the API is working, just rate limited
            }
            Err(e) => {
                panic!("Upload failed with error: {:?}", e);
            }
        }
    }

    /// Integration test for VirusTotal file lookup.
    /// Run with: VT_API_KEY=your_key cargo test test_virustotal_lookup_integration -- --ignored --nocapture
    #[test]
    #[ignore]
    fn test_virustotal_lookup_integration() {
        let api_key = std::env::var("VT_API_KEY").expect("VT_API_KEY environment variable not set");

        // Use a well-known SHA256 (Google Chrome APK or similar common file)
        // This hash is for a common benign file that should exist in VT
        let known_sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"; // SHA256 of empty file

        println!("Testing file lookup on VirusTotal...");
        println!("SHA256: {}", known_sha256);

        let result = get_file_report(known_sha256, &api_key);

        match result {
            Ok(response) => {
                println!("Lookup successful!");
                println!("File ID: {}", response.data.id);
                if let Some(stats) = &response.data.attributes.last_analysis_stats {
                    println!("Malicious: {}", stats.malicious);
                    println!("Suspicious: {}", stats.suspicious);
                    println!("Undetected: {}", stats.undetected);
                } else {
                    println!("File not yet analyzed");
                }
            }
            Err(VtError::NotFound) => {
                println!("File not found in VirusTotal (this is OK for the test)");
            }
            Err(VtError::RateLimit { retry_after }) => {
                println!("Rate limited. Retry after: {} seconds", retry_after);
            }
            Err(e) => {
                panic!("Lookup failed with error: {:?}", e);
            }
        }
    }
}

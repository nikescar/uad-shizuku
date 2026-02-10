pub use crate::api_hybridanalysis_stt::*;
use std::error::Error;
use std::path::Path;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Error types for Hybrid Analysis API
#[derive(Debug)]
pub enum HaError {
    NotFound,
    RateLimit { retry_after: u64 },
    HttpError(Box<dyn Error>),
    IoError(std::io::Error),
    ParseError(Box<dyn Error>),
}

impl std::fmt::Display for HaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HaError::NotFound => write!(f, "File not found in Hybrid Analysis"),
            HaError::RateLimit { retry_after } => write!(
                f,
                "Rate limit exceeded, retry after {} seconds",
                retry_after
            ),
            HaError::HttpError(e) => write!(f, "HTTP error: {}", e),
            HaError::IoError(e) => write!(f, "IO error: {}", e),
            HaError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl Error for HaError {}

impl From<std::io::Error> for HaError {
    fn from(err: std::io::Error) -> Self {
        HaError::IoError(err)
    }
}

/// Search for file hash in Hybrid Analysis API (blocking)
pub fn search_hash(sha256: &str, api_key: &str) -> Result<HybridAnalysisHashResponse, HaError> {
    let url = format!(
        "https://hybrid-analysis.com/api/v2/search/hash?hash={}",
        sha256
    );

    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .set("accept", "application/json")
        .set("api-key", api_key)
        .set("User-Agent", USER_AGENT)
        .call();

    match response {
        Ok(resp) => {
            log::trace!(
                "Hybrid Analysis search hash HTTP response status: {}",
                resp.status()
            );
            let response_text = resp
                .into_string()
                .map_err(|e| HaError::HttpError(Box::new(e)))?;
            log::trace!(
                "Hybrid Analysis search hash HTTP response body: {}",
                response_text
            );
            let ha_response: HybridAnalysisHashResponse = serde_json::from_str(&response_text)
                .map_err(|e| HaError::ParseError(Box::new(e)))?;
            Ok(ha_response)
        }
        Err(ureq::Error::Status(code, _resp)) => {
            log::trace!("Hybrid Analysis search hash HTTP error status: {}", code);
            if code == 404 {
                Err(HaError::NotFound)
            } else if code == 429 {
                // Hybrid Analysis uses 3 second minimum interval
                // Use fixed 3 second retry after for rate limiting
                Err(HaError::RateLimit { retry_after: 3 })
            } else {
                let err_msg = format!("HTTP error {}", code);
                Err(HaError::HttpError(err_msg.into()))
            }
        }
        Err(e) => Err(HaError::HttpError(Box::new(e))),
    }
}

/// Get report summary by report ID
pub fn get_report_summary(
    report_id: &str,
    api_key: &str,
) -> Result<HybridAnalysisReportResponse, HaError> {
    let url = format!(
        "https://hybrid-analysis.com/api/v2/report/{}/summary",
        report_id
    );

    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .set("accept", "application/json")
        .set("api-key", api_key)
        .set("User-Agent", USER_AGENT)
        .call();

    match response {
        Ok(resp) => {
            log::trace!(
                "Hybrid Analysis report summary HTTP response status: {}",
                resp.status()
            );
            let response_text = resp
                .into_string()
                .map_err(|e| HaError::HttpError(Box::new(e)))?;
            log::trace!(
                "Hybrid Analysis report summary HTTP response body: {}",
                response_text
            );
            let ha_response: HybridAnalysisReportResponse = serde_json::from_str(&response_text)
                .map_err(|e| HaError::ParseError(Box::new(e)))?;
            Ok(ha_response)
        }
        Err(ureq::Error::Status(code, _resp)) => {
            log::trace!("Hybrid Analysis report summary HTTP error status: {}", code);
            if code == 404 {
                Err(HaError::NotFound)
            } else if code == 429 {
                Err(HaError::RateLimit { retry_after: 3 })
            } else {
                let err_msg = format!("HTTP error {}", code);
                Err(HaError::HttpError(err_msg.into()))
            }
        }
        Err(e) => Err(HaError::HttpError(Box::new(e))),
    }
}

/// Check submission quota
pub fn check_quota(api_key: &str) -> Result<HybridAnalysisQuotaResponse, HaError> {
    let url = "https://hybrid-analysis.com/api/v2/key/submission-quota";

    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .set("accept", "application/json")
        .set("api-key", api_key)
        .set("User-Agent", USER_AGENT)
        .call();

    match response {
        Ok(resp) => {
            log::trace!(
                "Hybrid Analysis check quota HTTP response status: {}",
                resp.status()
            );
            let response_text = resp
                .into_string()
                .map_err(|e| HaError::HttpError(Box::new(e)))?;
            log::trace!(
                "Hybrid Analysis check quota HTTP response body: {}",
                response_text
            );
            let ha_response: HybridAnalysisQuotaResponse = serde_json::from_str(&response_text)
                .map_err(|e| HaError::ParseError(Box::new(e)))?;
            Ok(ha_response)
        }
        Err(ureq::Error::Status(code, _resp)) => {
            log::trace!("Hybrid Analysis check quota HTTP error status: {}", code);
            if code == 429 {
                Err(HaError::RateLimit { retry_after: 3 })
            } else {
                let err_msg = format!("HTTP error {}", code);
                Err(HaError::HttpError(err_msg.into()))
            }
        }
        Err(e) => Err(HaError::HttpError(Box::new(e))),
    }
}

/// Get the state of a submitted job by job_id
pub fn get_job_state(
    job_id: &str,
    api_key: &str,
) -> Result<HybridAnalysisJobStateResponse, HaError> {
    let url = format!("https://hybrid-analysis.com/api/v2/report/{}/state", job_id);

    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .set("accept", "application/json")
        .set("api-key", api_key)
        .set("User-Agent", USER_AGENT)
        .call();

    match response {
        Ok(resp) => {
            log::trace!(
                "Hybrid Analysis job state HTTP response status: {}",
                resp.status()
            );
            let response_text = resp
                .into_string()
                .map_err(|e| HaError::HttpError(Box::new(e)))?;
            log::trace!(
                "Hybrid Analysis job state HTTP response body: {}",
                response_text
            );
            let ha_response: HybridAnalysisJobStateResponse = serde_json::from_str(&response_text)
                .map_err(|e| HaError::ParseError(Box::new(e)))?;
            Ok(ha_response)
        }
        Err(ureq::Error::Status(code, _resp)) => {
            log::trace!("Hybrid Analysis job state HTTP error status: {}", code);
            if code == 404 {
                Err(HaError::NotFound)
            } else if code == 429 {
                Err(HaError::RateLimit { retry_after: 3 })
            } else {
                let err_msg = format!("HTTP error {}", code);
                Err(HaError::HttpError(err_msg.into()))
            }
        }
        Err(e) => Err(HaError::HttpError(Box::new(e))),
    }
}

/// Maximum file size for Hybrid Analysis uploads (200 MB)
/// Note: While the API documentation says 250 MB, free tier users may have lower limits
const MAX_UPLOAD_SIZE_MB: f64 = 200.0;

/// Upload a file to Hybrid Analysis for scanning
pub fn ha_submit_file(
    file_path: &Path,
    api_key: &str,
) -> Result<HybridAnalysisQuickScanResponse, HaError> {
    let url = "https://hybrid-analysis.com/api/v2/submit/file";

    // Get file size for timeout calculation
    let file_metadata = std::fs::metadata(file_path).map_err(|e| {
        log::error!("Cannot access file for upload: {:?} - {}", file_path, e);
        HaError::IoError(std::io::Error::new(
            e.kind(),
            format!("Cannot access file {:?}: {}", file_path, e)
        ))
    })?;
    let file_size_mb = file_metadata.len() as f64 / 1024.0 / 1024.0;
    log::info!(
        "File {:?} size: {:.2} MB ({} bytes)",
        file_path,
        file_size_mb,
        file_metadata.len()
    );

    // Check file size limit
    if file_size_mb > MAX_UPLOAD_SIZE_MB {
        log::warn!(
            "File too large for Hybrid Analysis upload: {:.2} MB (max: {} MB)",
            file_size_mb,
            MAX_UPLOAD_SIZE_MB
        );
        return Err(HaError::HttpError(
            format!(
                "File too large: {:.2} MB exceeds {} MB limit",
                file_size_mb, MAX_UPLOAD_SIZE_MB
            )
            .into(),
        ));
    }

    // Calculate timeout based on file size
    // Assume minimum 1 MB/s upload speed, with 60s base timeout
    // For a 200MB file: 200 + 60 = 260s, for 500MB: 500 + 60 = 560s
    let timeout_secs = (file_size_mb as u64) + 60;
    let timeout_secs = timeout_secs.max(60).min(1800); // Clamp between 60s and 30min
    log::info!("Using upload timeout: {} seconds", timeout_secs);

    // Create multipart form using ureq_multipart
    log::debug!("Building multipart form for file: {:?}", file_path);
    let (content_type, body) = ureq_multipart::MultipartBuilder::new()
        .add_text("environment_id", "200")? // 200 = Android Static Analysis
        .add_file("file", file_path)?
        .finish()?;

    log::info!(
        "Multipart form created: content_type={}, body_size={} bytes ({:.2} MB)",
        content_type,
        body.len(),
        body.len() as f64 / 1024.0 / 1024.0
    );

    log::info!("Sending HTTP POST request to {}", url);
    log::debug!("Request headers: accept=application/json, api-key=<redacted>, User-Agent={}, Content-Type={}", USER_AGENT, content_type);

    let response = ureq::post(url)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .set("accept", "application/json")
        .set("api-key", api_key)
        .set("User-Agent", USER_AGENT)
        .set("Content-Type", &content_type)
        .send_bytes(&body);

    log::info!("HTTP POST request completed (response received or error)");

    match response {
        Ok(resp) => {
            log::trace!(
                "Hybrid Analysis submit file HTTP response status: {}",
                resp.status()
            );
            log::info!("Got successful response, parsing JSON");
            let response_text = resp.into_string().map_err(|e| {
                log::error!("Failed to read response body: {}", e);
                HaError::HttpError(Box::new(e))
            })?;
            log::trace!(
                "Hybrid Analysis submit file HTTP response body: {}",
                response_text
            );
            log::debug!("Response body: {}", response_text);
            let ha_response: HybridAnalysisQuickScanResponse = serde_json::from_str(&response_text)
                .map_err(|e| {
                    log::error!("Failed to parse JSON response: {}", e);
                    log::error!("Response was: {}", response_text);
                    HaError::ParseError(Box::new(e))
                })?;
            log::info!("Successfully parsed response");
            Ok(ha_response)
        }
        Err(ureq::Error::Status(code, resp)) => {
            log::trace!("Hybrid Analysis submit file HTTP error status: {}", code);
            log::error!("HTTP request failed with status code: {}", code);
            if code == 429 {
                Err(HaError::RateLimit { retry_after: 3 })
            } else {
                // Try to read the error response body for debugging
                let error_body = resp
                    .into_string()
                    .unwrap_or_else(|_| String::from("(no body)"));
                log::error!("Error response body: {}", error_body);
                let err_msg = format!("HTTP error {}: {}", code, error_body);
                Err(HaError::HttpError(err_msg.into()))
            }
        }
        Err(e) => {
            log::error!(
                "HTTP request failed with network error: {} (file: {:?}, body_size: {} bytes)",
                e,
                file_path,
                body.len()
            );
            // Log additional context for broken pipe errors
            if e.to_string().contains("Broken pipe") {
                log::error!(
                    "Broken pipe error - this usually means the server closed the connection. \
                     Possible causes: file too large, timeout, or server rejection. \
                     File size: {:.2} MB, Timeout: {} seconds",
                    file_metadata.len() as f64 / 1024.0 / 1024.0,
                    timeout_secs
                );
            }
            Err(HaError::HttpError(Box::new(e)))
        }
    }
}

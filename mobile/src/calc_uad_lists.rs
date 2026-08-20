//! UAD-NG debloat list loading logic, shared by the legacy SharedStore path
//! (`uad_shizuku_app::retrieve_uad_ng_lists`) and the MVVM `DebloatActor`.

use crate::uad_shizuku_app::UadNgLists;
use std::path::Path;

const UAD_LISTS_URL: &str = "https://fastly.jsdelivr.net/gh/Universal-Debloater-Alliance/universal-android-debloater-next-generation@main/resources/assets/uad_lists.json";
const UAD_LISTS_FILENAME: &str = "uad_lists.json";
// Embedded fallback file (pre-downloaded and compressed at build time with zstd)
// Compression reduces binary size and makes embedded data less obvious to AV static analysis
const UAD_LISTS_FALLBACK_COMPRESSED: &[u8] = include_bytes!("../resources/uad_lists.json.zst");

/// Load the UAD-NG debloat lists, refreshing the on-disk cache if it is
/// missing or older than 7 days. Blocking - call from a background thread.
pub fn load_uad_ng_lists_blocking(cache_dir: &Path) -> Option<UadNgLists> {
    let cache_file_path = cache_dir.join(UAD_LISTS_FILENAME);

    let should_download = !cache_file_path.exists() || {
        cache_file_path
            .metadata()
            .and_then(|m| m.modified())
            .map(|modified| {
                modified
                    .elapsed()
                    .map(|elapsed| elapsed.as_secs() > 7 * 24 * 60 * 60)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    };

    if should_download {
        log::info!(
            "UAD lists not found in cache or older than 7 days, downloading from {}",
            UAD_LISTS_URL
        );
        download_or_fallback(&cache_file_path);
    } else {
        log::info!("UAD lists found in cache at {:?}", cache_file_path);
    }

    match std::fs::read_to_string(&cache_file_path) {
        Ok(json_content) => match serde_json::from_str::<UadNgLists>(&json_content) {
            Ok(uad_lists) => {
                log::info!(
                    "Successfully parsed UAD lists with {} apps",
                    uad_lists.apps.len()
                );
                Some(uad_lists)
            }
            Err(e) => {
                log::error!("Failed to parse UAD lists JSON from file: {}", e);
                parse_embedded_fallback()
            }
        },
        Err(e) => {
            log::error!("Failed to read UAD lists from cache: {}", e);
            parse_embedded_fallback()
        }
    }
}

fn download_or_fallback(cache_file_path: &Path) {
    let mut request = ehttp::Request::get(UAD_LISTS_URL);
    request.headers.insert(
        "User-Agent".to_string(),
        format!("uad-shizuku/{}", env!("CARGO_PKG_VERSION")),
    );
    let (sender, receiver) = std::sync::mpsc::channel();

    ehttp::fetch(request, move |result| {
        sender.send(result).ok();
    });

    match receiver.recv() {
        Ok(Ok(response)) if response.ok => {
            // jsdelivr CDN returns raw JSON, not GitHub HTML
            match std::fs::write(cache_file_path, &response.bytes) {
                Ok(_) => {
                    log::info!(
                        "Successfully downloaded and cached UAD lists to {:?}",
                        cache_file_path
                    );
                }
                Err(e) => {
                    log::error!("Failed to write UAD lists to cache: {}", e);
                    write_fallback(cache_file_path);
                }
            }
        }
        Ok(Ok(response)) => {
            log::error!(
                "Failed to download UAD lists: HTTP {}, using fallback",
                response.status
            );
            write_fallback(cache_file_path);
        }
        Ok(Err(e)) => {
            log::error!("Failed to download UAD lists: {}, using fallback", e);
            write_fallback(cache_file_path);
        }
        Err(e) => {
            log::error!("Failed to receive download response: {}, using fallback", e);
            write_fallback(cache_file_path);
        }
    }
}

fn write_fallback(cache_file_path: &Path) {
    // Decompress the embedded zstd-compressed fallback data
    match zstd::decode_all(UAD_LISTS_FALLBACK_COMPRESSED) {
        Ok(decompressed_json) => {
            match std::fs::write(cache_file_path, decompressed_json) {
                Ok(_) => log::info!("Successfully wrote decompressed UAD lists fallback to cache"),
                Err(e) => log::error!("Failed to write UAD lists fallback to cache: {}", e),
            }
        }
        Err(e) => log::error!("Failed to decompress UAD lists fallback: {}", e),
    }
}

fn parse_embedded_fallback() -> Option<UadNgLists> {
    log::info!("Attempting to parse from embedded fallback data");

    // First decompress the zstd-compressed embedded data
    let decompressed_json = match zstd::decode_all(UAD_LISTS_FALLBACK_COMPRESSED) {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to decompress embedded fallback: {}", e);
            return None;
        }
    };

    // Then parse the decompressed JSON
    match serde_json::from_slice::<UadNgLists>(&decompressed_json) {
        Ok(uad_lists) => {
            log::info!(
                "Successfully parsed UAD lists from embedded fallback with {} apps",
                uad_lists.apps.len()
            );
            Some(uad_lists)
        }
        Err(e) => {
            log::error!("Failed to parse embedded fallback: {}", e);
            None
        }
    }
}

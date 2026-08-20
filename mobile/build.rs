// build.rs

extern crate winres;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::SystemTime;

fn main() {
    // Download fallback resources for offline usage
    download_fallback_resources();

    // Windows-specific resource compilation
    // Add rich PE metadata to help AVs understand this is legitimate software
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/logo.ico");

        // PE metadata visible in Windows Explorer and to AV scanners
        res.set("FileDescription", "UAD-Shizuku Android Debloater");
        res.set("ProductName", "UAD-Shizuku");
        res.set("CompanyName", "Universal Android Debloater");
        res.set(
            "LegalCopyright",
            "Copyright (C) 2024 - Licensed under GPL-3.0 OR MIT OR Apache-2.0",
        );
        res.set(
            "Comments",
            "Legitimate Android debloating tool. Connects to Android devices via ADB. \
             Includes malware detection databases to SCAN for threats (not distribute them). \
             If flagged by antivirus, it is a false positive - please submit: \
             https://www.microsoft.com/en-us/wdsi/filesubmission",
        );

        res.compile().unwrap();
    }
}

fn download_fallback_resources() {
    const UAD_LISTS_URL: &str = "https://cdn.jsdelivr.net/gh/0x192/universal-android-debloater@latest/resources/assets/uad_lists.json";
    const STALKERWARE_IOC_URL: &str =
        "https://raw.githubusercontent.com/AssoEchap/stalkerware-indicators/master/ioc.yaml";

    let resources_dir = Path::new("resources");

    // Ensure resources directory exists
    if let Err(e) = fs::create_dir_all(resources_dir) {
        eprintln!("Warning: Failed to create resources directory: {}", e);
        return;
    }

    // Download UAD lists and compress with zstd to reduce binary size
    // and make embedded data less obvious to AV static analysis
    let uad_json_path = resources_dir.join("uad_lists.json");
    let uad_zst_path = resources_dir.join("uad_lists.json.zst");

    download_if_needed(
        UAD_LISTS_URL,
        &uad_json_path,
        "UAD lists",
    );

    // Compress UAD lists with zstd (level 3 = good balance of speed/size)
    if uad_json_path.exists() {
        match compress_uad_lists(&uad_json_path, &uad_zst_path) {
            Ok(_) => println!("cargo:warning=Compressed UAD lists to {:?}", uad_zst_path),
            Err(e) => eprintln!("Warning: Failed to compress UAD lists: {}", e),
        }
    }

    // Download Stalkerware IoC
    download_if_needed(
        STALKERWARE_IOC_URL,
        &resources_dir.join("stalkerware_ioc.yaml"),
        "Stalkerware IoC",
    );

    // Tell Cargo to rerun this build script if the files are deleted
    println!("cargo:rerun-if-changed=resources/uad_lists.json.zst");
    println!("cargo:rerun-if-changed=resources/stalkerware_ioc.yaml");
}

fn download_if_needed(url: &str, file_path: &Path, description: &str) {
    // Check if file exists and is recent (less than 7 days old)
    let should_download = if file_path.exists() {
        match fs::metadata(file_path) {
            Ok(metadata) => {
                match metadata.modified() {
                    Ok(modified) => {
                        match SystemTime::now().duration_since(modified) {
                            Ok(age) => {
                                // Download if older than 7 days
                                age.as_secs() > 7 * 24 * 60 * 60
                            }
                            Err(_) => false, // Can't determine age, keep existing
                        }
                    }
                    Err(_) => false, // Can't get modification time, keep existing
                }
            }
            Err(_) => true, // Can't read metadata, try to download
        }
    } else {
        true // File doesn't exist, download
    };

    if !should_download {
        println!(
            "cargo:warning={} is up-to-date at {:?}",
            description, file_path
        );
        return;
    }

    println!("cargo:warning=Downloading {} from {}", description, url);

    match ureq::get(url).set("User-Agent", "uad-shizuku/1.0").call() {
        Ok(response) => {
            let mut buffer = Vec::new();
            if let Err(e) = response.into_reader().read_to_end(&mut buffer) {
                eprintln!("Warning: Failed to read {} response: {}", description, e);
                if !file_path.exists() {
                    eprintln!("ERROR: {} not available and download failed!", description);
                }
                return;
            }

            match fs::write(file_path, &buffer) {
                Ok(_) => println!(
                    "cargo:warning=Successfully downloaded {} to {:?}",
                    description, file_path
                ),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to write {} to {:?}: {}",
                        description, file_path, e
                    );
                    if !file_path.exists() {
                        eprintln!(
                            "ERROR: {} not available and cannot write file!",
                            description
                        );
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to download {}: {}", description, e);
            if !file_path.exists() {
                eprintln!("ERROR: {} not available and download failed!", description);
                eprintln!("       Please ensure network connectivity or manually download:");
                eprintln!("       curl -o {:?} \"{}\"", file_path, url);
            } else {
                eprintln!("       Using existing file at {:?}", file_path);
            }
        }
    }
}

fn compress_uad_lists(json_path: &Path, zst_path: &Path) -> std::io::Result<()> {
    // Read the JSON file
    let json_data = fs::read(json_path)?;

    // Compress with zstd (level 3 = good balance of speed and compression)
    let compressed = zstd::encode_all(&json_data[..], 3)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Write compressed data
    fs::write(zst_path, compressed)?;

    let original_size = json_data.len();
    let compressed_size = fs::metadata(zst_path)?.len();
    let ratio = (compressed_size as f64 / original_size as f64) * 100.0;

    println!(
        "cargo:warning=Compressed UAD lists: {} bytes -> {} bytes ({:.1}% of original)",
        original_size, compressed_size, ratio
    );

    Ok(())
}

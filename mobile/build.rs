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
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/logo.ico");
        res.compile().unwrap();
    }
}

fn download_fallback_resources() {
    const UAD_LISTS_URL: &str = "https://cdn.jsdelivr.net/gh/0x192/universal-android-debloater@latest/resources/assets/uad_lists.json";
    const STALKERWARE_IOC_URL: &str = "https://raw.githubusercontent.com/AssoEchap/stalkerware-indicators/master/ioc.yaml";

    let resources_dir = Path::new("resources");

    // Ensure resources directory exists
    if let Err(e) = fs::create_dir_all(resources_dir) {
        eprintln!("Warning: Failed to create resources directory: {}", e);
        return;
    }

    // Download UAD lists
    download_if_needed(
        UAD_LISTS_URL,
        &resources_dir.join("uad_lists.json"),
        "UAD lists"
    );

    // Download Stalkerware IoC
    download_if_needed(
        STALKERWARE_IOC_URL,
        &resources_dir.join("stalkerware_ioc.yaml"),
        "Stalkerware IoC"
    );

    // Tell Cargo to rerun this build script if the files are deleted
    println!("cargo:rerun-if-changed=resources/uad_lists.json");
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
        println!("cargo:warning={} is up-to-date at {:?}", description, file_path);
        return;
    }

    println!("cargo:warning=Downloading {} from {}", description, url);

    match ureq::get(url).call() {
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
                Ok(_) => println!("cargo:warning=Successfully downloaded {} to {:?}", description, file_path),
                Err(e) => {
                    eprintln!("Warning: Failed to write {} to {:?}: {}", description, file_path, e);
                    if !file_path.exists() {
                        eprintln!("ERROR: {} not available and cannot write file!", description);
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

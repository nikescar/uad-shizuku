// Standalone fake `adb` binary for testing uad-shizuku without a real device.
//
// It mimics the exact subset of `adb` invocations that `src/adb.rs` makes on
// desktop (devices/kill-server/root/shell/install/uninstall/pull), replying
// with responses captured from a real Pixel 8a (see
// tests/fixtures/adb/README.md) or, for inputs not covered by a captured
// fixture, a deterministic synthesized response shaped like the real thing.
//
// Build it, then put it on PATH ahead of the real `adb` (renamed to `adb` /
// `adb.exe`) so `Command::new("adb")` calls in adb.rs resolve to this binary.

use std::path::Path;
use std::process::ExitCode;

const DEVICES_L: &str = include_str!("../../tests/fixtures/adb/devices_l.txt");
const PM_LIST_USERS: &str = include_str!("../../tests/fixtures/adb/pm_list_users.txt");
const PM_LIST_PACKAGES_F: &str = include_str!("../../tests/fixtures/adb/pm_list_packages_f.txt");
const DUMPSYS_PACKAGE_PACKAGES: &str =
    include_str!("../../tests/fixtures/adb/dumpsys_package_packages.txt");
const GETPROP_ABILIST: &str = include_str!("../../tests/fixtures/adb/getprop_abilist.txt");
const GETPROP_VERSION: &str = include_str!("../../tests/fixtures/adb/getprop_version.txt");
const DUMPSYS_PACKAGE_NOT_FOUND: &str =
    include_str!("../../tests/fixtures/adb/dumpsys_package_not_found.txt");

const NTFY_PKG: &str = "io.heckel.ntfy";
const NTFY_DUMPSYS: &str =
    include_str!("../../tests/fixtures/adb/dumpsys_package_io.heckel.ntfy.txt");
const NTFY_CODEPATH: &str =
    "/data/app/~~ny1aVKTNxKUDyWLDzM5bPw==/io.heckel.ntfy-K2BhGdAlPTfLZs8XlI-moA==";
const NTFY_APK_PATH: &str =
    "/data/app/~~ny1aVKTNxKUDyWLDzM5bPw==/io.heckel.ntfy-K2BhGdAlPTfLZs8XlI-moA==/base.apk";
const NTFY_APK_SHA256: &str = "609f2c93b9bbdac5c17218b32d1e0de1b0022e71c96665edf4eefe16ff4a196d";

const OVERLAY_PKG: &str = "com.google.android.systemui.overlay.pixelbatteryhealthconfig";
const OVERLAY_DUMPSYS: &str = include_str!(
    "../../tests/fixtures/adb/dumpsys_package_com.google.android.systemui.overlay.pixelbatteryhealthconfig.txt"
);

/// Explicit package name a test can use to exercise the "package not
/// installed" path deterministically.
const NOT_FOUND_PKG: &str = "com.mockadb.not.installed";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();

    match argv.as_slice() {
        ["devices", "-l"] => {
            print!("{}", DEVICES_L);
            ExitCode::SUCCESS
        }
        ["kill-server"] => ExitCode::SUCCESS,
        ["root"] => {
            println!("restarting adbd as root");
            ExitCode::SUCCESS
        }
        ["-s", _serial, "shell", command] => run_shell(command),
        ["-s", _serial, "install", apk_path] => {
            eprintln!("Performing Streamed Install for {}", apk_path);
            println!("Success");
            ExitCode::SUCCESS
        }
        ["-s", _serial, "uninstall", package] => {
            println!("Success");
            let _ = package;
            ExitCode::SUCCESS
        }
        ["-s", _serial, "pull", remote, local] => run_pull(remote, local),
        other => {
            eprintln!("mock_adb: unrecognized invocation: {:?}", other);
            ExitCode::FAILURE
        }
    }
}

fn run_pull(remote: &str, local: &str) -> ExitCode {
    if let Some(parent) = Path::new(local).parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("adb: failed to create {}: {}", parent.display(), e);
                return ExitCode::FAILURE;
            }
        }
    }
    let dummy_contents = format!("mock-adb-pulled-file source={}\n", remote);
    if let Err(e) = std::fs::write(local, dummy_contents.as_bytes()) {
        eprintln!(
            "adb: error: failed to copy '{}' to '{}': {}",
            remote, local, e
        );
        return ExitCode::FAILURE;
    }
    println!(
        "{}: 1 file pulled, 0 skipped. 1.0 MB/s ({} bytes in 0.010s)",
        remote,
        dummy_contents.len()
    );
    ExitCode::SUCCESS
}

fn run_shell(command: &str) -> ExitCode {
    let cmd = command.trim();

    if cmd == "pm list users" {
        print!("{}", PM_LIST_USERS);
        return ExitCode::SUCCESS;
    }
    if cmd == "pm list packages -f" {
        print!("{}", PM_LIST_PACKAGES_F);
        return ExitCode::SUCCESS;
    }
    if cmd == "dumpsys package packages" {
        print!("{}", DUMPSYS_PACKAGE_PACKAGES);
        return ExitCode::SUCCESS;
    }
    if let Some(pkg) = cmd.strip_prefix("dumpsys package ") {
        print!("{}", dumpsys_package(pkg.trim()));
        return ExitCode::SUCCESS;
    }
    if cmd == "getprop ro.product.cpu.abilist" {
        print!("{}", GETPROP_ABILIST);
        return ExitCode::SUCCESS;
    }
    if cmd == "getprop ro.build.version.release" {
        print!("{}", GETPROP_VERSION);
        return ExitCode::SUCCESS;
    }
    if cmd == "dumpsys usagestats -history" {
        // Not captured from a real device (tab_usage is a stub feature) —
        // synthesized minimal-but-well-formed placeholder.
        println!("mock-adb: synthesized empty usagestats history");
        return ExitCode::SUCCESS;
    }
    if let Some(rest) = cmd.strip_prefix("sha256sum ") {
        print!("{}", sha256sum(rest));
        return ExitCode::SUCCESS;
    }
    if let Some(rest) = cmd.strip_prefix("find ") {
        if let Some(dir) = rest.strip_suffix(" -type f") {
            print!("{}", find_type_f(dir.trim()));
            return ExitCode::SUCCESS;
        }
    }
    if let Some(rest) = cmd.strip_prefix("ls ") {
        if let Some(dir) = rest.strip_suffix("/*.apk") {
            println!("{}/base.apk", dir.trim());
            return ExitCode::SUCCESS;
        }
    }
    if let Some(rest) = cmd.strip_prefix("pm uninstall --user ") {
        let _ = rest;
        println!("Success");
        return ExitCode::SUCCESS;
    }
    if let Some(rest) = cmd.strip_prefix("pm disable-user --user ") {
        if let Some(pkg) = rest.split_whitespace().nth(1) {
            println!("Package {} new state: disabled-user", pkg);
        }
        return ExitCode::SUCCESS;
    }
    if let Some(pkg) = cmd.strip_prefix("pm enable ") {
        println!("Package {} new state: enabled", pkg.trim());
        return ExitCode::SUCCESS;
    }
    if let Some(pkg) = cmd.strip_prefix("cmd package install-existing ") {
        println!("Package {} installed for user: 0", pkg.trim());
        return ExitCode::SUCCESS;
    }
    if cmd.starts_with("pm path --user ") {
        // extract_apk() is #[allow(dead_code)] and unused today; no fixture captured.
        return ExitCode::SUCCESS;
    }

    let program = cmd.split_whitespace().next().unwrap_or(cmd);
    eprintln!("/system/bin/sh: {}: not found", program);
    ExitCode::from(127)
}

fn dumpsys_package(pkg: &str) -> String {
    if pkg == NTFY_PKG {
        return NTFY_DUMPSYS.to_string();
    }
    if pkg == OVERLAY_PKG {
        return OVERLAY_DUMPSYS.to_string();
    }
    if pkg == NOT_FOUND_PKG {
        return DUMPSYS_PACKAGE_NOT_FOUND.to_string();
    }
    // Synthesize a well-formed dump for any other package name so the mock
    // stays useful for arbitrary test packages, not just the captured ones.
    format!(
        "  Package [{pkg}] (deadbeef):\n    appId=10999\n    pkg=Package{{deadbeef {pkg}}}\n    codePath=/data/app/~~mock==/{pkg}-mock==\n    resourcePath=/data/app/~~mock==/{pkg}-mock==\n    legacyNativeLibraryDir=/data/app/~~mock==/{pkg}-mock==/lib\n    primaryCpuAbi=arm64-v8a\n    secondaryCpuAbi=null\n    versionCode=1 minSdk=21 targetSdk=34\n    versionName=1.0.0\n    flags=[ HAS_CODE ALLOW_CLEAR_USER_DATA ALLOW_BACKUP ]\n    privateFlags=[ ]\n    User 0: ceDataInode=0 installed=true hidden=false suspended=false stopped=false enabled=0\n",
        pkg = pkg
    )
}

fn find_type_f(dir: &str) -> String {
    if dir == NTFY_CODEPATH {
        format!("{}\n", NTFY_APK_PATH)
    } else {
        format!("{}/base.apk\n", dir)
    }
}

fn sha256sum(args: &str) -> String {
    let args = args.trim().trim_end_matches("2>/dev/null").trim();
    let mut out = String::new();
    for path in extract_quoted(args) {
        let hash = if path == NTFY_APK_PATH {
            NTFY_APK_SHA256.to_string()
        } else {
            fake_sha256(&path)
        };
        out.push_str(&hash);
        out.push_str("  ");
        out.push_str(&path);
        out.push('\n');
    }
    out
}

fn extract_quoted(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut token = String::new();
            for c2 in chars.by_ref() {
                if c2 == '"' {
                    break;
                }
                token.push(c2);
            }
            if !token.is_empty() {
                result.push(token);
            }
        }
    }
    result
}

/// Deterministic, non-cryptographic 64-hex-char stand-in for a real sha256
/// hash, derived from the input path. Good enough to be stable and
/// unique-looking across test runs; not a real hash.
fn fake_sha256(input: &str) -> String {
    let mut out = String::with_capacity(64);
    for seed in [
        0xcbf29ce484222325u64,
        0x9e3779b97f4a7c15,
        0x100000001b3,
        0xc6a4a7935bd1e995,
    ] {
        let mut hash = seed;
        for b in input.bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        out.push_str(&format!("{:016x}", hash));
    }
    out
}

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 1. Overview

UAD-Shizuku is a cross-platform Android device management tool for debloating, malware scanning, and FOSS app installation. It connects to Android devices via ADB/Shizuku and provides a desktop GUI (Windows, macOS, Linux) and native Android app.

## 2. Features

- **Debloat**: Uninstall/disable bloatware using UAD-NG lists with safety ratings
- **Scan**: Malware detection via VirusTotal and HybridAnalysis APIs
- **Install**: Browse and install FOSS apps from curated lists (GitHub, F-Droid)
- **Metadata**: Rich app information from Google Play, F-Droid, and APKMirror

## 3. Repository Structure

```
uad-shizuku/
├── mobile/                          # Main application workspace
│   ├── src/
│   │   ├── viewmodel/              # MVVM ViewModel layer (NEW - Jun 2026)
│   │   │   ├── mod.rs              # ViewModel struct, channels, state
│   │   │   ├── debloat.rs          # DebloatActor (packages, UAD lists)
│   │   │   ├── scan.rs             # ScanActor (VirusTotal, HybridAnalysis)
│   │   │   ├── apps.rs             # AppsActor (FOSS app lists)
│   │   │   ├── metadata.rs         # MetadataActor (GooglePlay, F-Droid, etc.)
│   │   │   └── common.rs           # Shared types (commands, events, state)
│   │   ├── uad_shizuku_app.rs      # Main app struct (egui)
│   │   ├── main.rs                 # Desktop entry point
│   │   ├── main_android.rs         # Android entry point
│   │   ├── lib.rs                  # Library exports, Config, Settings
│   │   ├── adb.rs                  # ADB client implementation
│   │   ├── android_shizuku.rs      # Shizuku JNI integration (Android)
│   │   ├── android_*.rs            # Android platform integration modules
│   │   ├── tab_debloat_control.rs  # Debloat UI tab
│   │   ├── tab_scan_control.rs     # Scan UI tab
│   │   ├── tab_apps_control.rs     # Apps UI tab
│   │   ├── tab_usage_control.rs    # Usage tracking tab (stub)
│   │   ├── dlg_*.rs                # Dialog windows
│   │   ├── api_virustotal.rs       # VirusTotal API client
│   │   ├── api_hybridanalysis.rs   # HybridAnalysis API client
│   │   ├── api_googleplay.rs       # Google Play scraper
│   │   ├── api_fdroid.rs           # F-Droid API client
│   │   ├── api_apkmirror.rs        # APKMirror scraper
│   │   ├── api_*.rs                # Other API clients
│   │   ├── db.rs                   # Database connection & migrations
│   │   ├── db_virustotal.rs        # VirusTotal cache DB operations
│   │   ├── db_hybridanalysis.rs    # HybridAnalysis cache DB operations
│   │   ├── db_package_cache.rs     # Package metadata DB operations
│   │   ├── db_googleplay.rs        # Google Play metadata DB operations
│   │   ├── db_fdroid.rs            # F-Droid metadata DB operations
│   │   ├── db_apkmirror.rs         # APKMirror metadata DB operations
│   │   ├── calc.rs                 # Core calculation/processing logic
│   │   ├── calc_*.rs               # Domain-specific business logic
│   │   ├── app_operations_queue.rs # App install/uninstall queue
│   │   ├── shared_store.rs         # LEGACY: Global state (being phased out)
│   │   ├── models.rs               # Data models
│   │   ├── schema.rs               # Diesel schema (auto-generated)
│   │   ├── material_symbol_icons.rs # Material Design icon definitions
│   │   └── *_stt.rs                # State struct modules (paired with main modules)
│   ├── migrations/                 # Diesel SQL migrations (timestamped)
│   ├── tests/                      # Integration tests
│   │   ├── integration/            # Integration test modules
│   │   ├── test_fdroid.rs
│   │   ├── test_hybridanalysis.rs
│   │   └── test_virustotal_db.rs.disabled
│   ├── assets/                     # Embedded resources
│   │   └── languages/fluent/       # i18n translations (en-US, ko-KR)
│   ├── resources/                  # Downloaded at build time
│   │   ├── uad_lists.json          # UAD-NG debloat lists
│   │   └── stalkerware_ioc.yaml    # Stalkerware indicators
│   ├── app/                        # Android app configuration
│   │   └── src/main/               # Android manifest, resources
│   ├── build.rs                    # Build script (downloads resources)
│   ├── Cargo.toml
│   └── diesel.toml                 # Diesel configuration
├── reference/                      # Reference implementations
│   └── bingtray/                   # Similar MVVM project for reference
├── docs/                           # Documentation
│   ├── mvvm-actor-migration-complete.md  # Architecture migration notes
│   ├── next-session-handoff.md     # Session context for Claude
│   └── superpowers/                # Claude Code plans and specs
├── deploy/                         # Build and deployment scripts
├── scripts/                        # Utility scripts
├── fastlane/                       # App store deployment automation
└── Cargo.toml                      # Workspace root
```

## 4. Architecture

### MVVM with Actor-Based Concurrency (June 2026 Migration)

UAD-Shizuku uses MVVM pattern with actors for background processing:

```
┌─────────────────────────────────────────────────────────────┐
│                    UadShizukuApp (egui UI)                   │
│  ┌────────────────────────────────────────────────────┐     │
│  │              ViewModel (Command/Event)              │     │
│  │                                                     │     │
│  │  ┌──────────────────────────────────────────┐     │     │
│  │  │       ViewModelState (Read-Only)          │     │     │
│  │  │  • packages: Vec<PackageFingerprint>     │     │     │
│  │  │  • uad_ng_lists: UAD debloat ratings     │     │     │
│  │  │  • vt_scanner_state: VirusTotal state    │     │     │
│  │  │  • ha_scanner_state: HybridAnalysis state│     │     │
│  │  │  • cached_metadata: MetadataCache        │     │     │
│  │  │  • stalkerware_indicators: IoC data      │     │     │
│  │  └──────────────────────────────────────────┘     │     │
│  │                                                     │     │
│  │  Command Channels (UI → Actors):                   │     │
│  │  • debloat_tx  → DebloatActor                     │     │
│  │  • scan_tx     → ScanActor                        │     │
│  │  • apps_tx     → AppsActor                        │     │
│  │  • metadata_tx → MetadataActor                    │     │
│  │                                                     │     │
│  │  Event Channel (Actors → UI):                      │     │
│  │  • event_rx ← All Actors                          │     │
│  └────────────────────────────────────────────────────┘     │
│                                                               │
│  Actors (Background Thread - smol runtime):                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ DebloatActor │  │  ScanActor   │  │MetadataActor │      │
│  │ • LoadPkgs   │  │ • VT Scan    │  │ • GooglePlay │      │
│  │ • LoadUAD    │  │ • HA Scan    │  │ • FDroid     │      │
│  │ • Uninstall  │  │ • State Mgmt │  │ • APKMirror  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
  ┌──────────────────────────────────────────────────┐
  │          Database Layer (Diesel + SQLite)         │
  │  • virustotal_results                            │
  │  • hybridanalysis_results                        │
  │  • package_info_cache                            │
  │  • google_play_apps, fdroid_apps, apkmirror_apps │
  └──────────────────────────────────────────────────┘
```

**Key Patterns**:
- **Commands**: UI sends commands to actors (e.g., `ScanCommand::ScanPackage`)
- **Events**: Actors emit events consumed by UI (e.g., `ScanEvent::ScanComplete`)
- **State**: `ViewModelState` is the single source of truth, read-only from UI
- **Polling**: UI calls `viewmodel.poll_events()` in `update()` to process events

### Database Layer (Diesel + SQLite)

- **Location**: `mobile/src/db*.rs`
- **Migrations**: Embedded in binary, run automatically on first connection
- **Schema**: Auto-generated `schema.rs` (do not edit manually)
- **Storage Paths**:
  - Desktop: `~/.config/uad_shizuku/dbs/` (Linux/macOS), `%APPDATA%\uad_shizuku\dbs\` (Windows)
  - Android: `/data/data/pe.nikescar.uad_shizuku/dbs/`

**Tables**:
- `virustotal_results`: Cached malware scan results
- `hybridanalysis_results`: Cached sandbox analysis results
- `package_info_cache`: Android package metadata
- `google_play_apps`, `fdroid_apps`, `apkmirror_apps`: App metadata

### External Dependencies

- **UAD-NG Lists**: Downloaded at build time from https://github.com/0x192/universal-android-debloater
- **Stalkerware IoC**: Downloaded at build time from https://github.com/AssoEchap/stalkerware-indicators

## 5. Build and Test

### Desktop Build

```bash
# Debug build (fast iteration)
cargo build

# Release build (optimized, stripped)
cargo build --release

# Run on desktop
cargo run
```

### Android Build

Android builds are managed via gradle in `deploy/` directory. Requires Android Studio and NDK.

### Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test --test viewmodel_tests

# Run with logs visible
RUST_LOG=debug cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy

# Check licenses and dependencies
cargo deny check
```

## 6. Common Development Tasks

### Adding a New Migration

```bash
cd mobile
diesel migration generate add_new_column
# Edit migrations/<timestamp>_add_new_column/up.sql
# Edit migrations/<timestamp>_add_new_column/down.sql
cargo build  # Schema auto-updates
```

### Adding a New Actor Command/Event

1. Add command variant to `viewmodel/<domain>.rs`
2. Add event variant to `viewmodel/common.rs` (`ViewModelEvent` enum)
3. Handle command in actor's `async fn run()` loop
4. Emit event via `event_tx.send()`
5. Handle event in `ViewModel::poll_events()`

### Updating UAD-NG Lists

Lists are downloaded during build. To force update:
```bash
rm mobile/resources/uad_lists.json
cargo clean
cargo build
```

### Adding New Translations

Edit `mobile/assets/languages/fluent/{en-US,ko-KR}.ftl` using Fluent syntax.

## 7. Platform-Specific Notes

### Android

- **Entry Point**: `mobile/src/main_android.rs`
- **ADB Alternative**: Uses Shizuku JNI for native Android execution
- **Permissions**: Requires Shizuku permission grant from user

### Desktop

- **Entry Point**: `mobile/src/main.rs`
- **ADB Requirement**: Must have `adb` in PATH or bundled
- **Window Manager**: Uses native OS windowing (GTK on Linux, Win32 on Windows, Cocoa on macOS)

### WASM (Experimental)

- **Target**: `wasm32-unknown-unknown`
- **Limitations**: No ADB support, Diesel uses WASM SQLite VFS
- **Status**: Partially implemented, not production-ready

## 8. Recent Changes

### MVVM Actor Architecture Migration (June 2026)

**Completed**:
- ✅ Migrated from global `SharedStore` singleton to MVVM pattern
- ✅ Implemented 4 actors: Debloat, Scan, Apps, Metadata
- ✅ Centralized state in `ViewModelState`
- ✅ Added comprehensive integration tests
- ✅ Scanner states now in ViewModel (VirusTotal, HybridAnalysis)
- ✅ Metadata cache now in ViewModel (GooglePlay, F-Droid, APKMirror)
- ✅ Stalkerware indicators loaded into ViewModel

**Legacy Code**:
- `shared_store.rs` still exists for texture caches (egui constraint)
- New code should use ViewModel, not SharedStore

**Documentation**: See `docs/mvvm-actor-migration-complete.md`

## 9. API Keys and Configuration

User settings stored in:
- Desktop: `~/.config/uad_shizuku/settings.txt`
- Android: `/data/data/pe.nikescar.uad_shizuku/files/settings.txt`

**Required for Full Functionality**:
- VirusTotal API Key (4 requests/min free tier)
- HybridAnalysis API Key (200 requests/min free tier)

## 10. Code Style Guidelines

- Use `tracing::` macros for logging (`info!`, `warn!`, `error!`)
- Prefix Android JNI modules with `android_*`
- Prefix API clients with `api_*`
- Prefix business logic with `calc_*`
- Prefix database modules with `db_*`
- Prefix dialogs with `dlg_*`
- Prefix tabs with `tab_*`
- Suffix state structs with `_stt` module (e.g., `scan_stt.rs` for `ScanState`)
- All new async operations should use actors in `viewmodel/`
- Do not add new dependencies to `SharedStore` - use ViewModel instead

## 11. Testing Guidelines

- Integration tests for actors in `mobile/tests/`
- Unit tests in same file as implementation (inline `#[cfg(test)]`)
- Mock ADB responses using in-memory fixtures
- Database tests use temporary SQLite files
- UI tests are manual (egui integration testing is limited)

## 12. Known Limitations

- Android lifecycle handling needs improvement (app freezes after sleep)
- APKMirror renderer app ID search is not accurate
- Web version blocked by Diesel/ureq WASM compatibility
- Some metadata fetchers may fail due to website structure changes

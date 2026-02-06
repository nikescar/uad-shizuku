# Usage

This guide explains how to use UAD-Shizuku to manage Android applications on your device.

## Getting Started

### Prerequisites

1. **ADB (Android Debug Bridge)**: Required for connecting to your Android device
   - On first launch, if ADB is not found, a dialog will guide you through installation
   - For desktop platforms (Windows/Linux/macOS), the app can automatically download platform-tools

2. **USB Debugging**: Enable USB debugging on your Android device
   - Go to Settings > Developer Options > USB Debugging
   - Connect your device via USB or configure wireless ADB

3. **Shizuku (Optional)**: For Android APK mode, Shizuku provides root-like access without root

### Initial Setup

1. Launch UAD-Shizuku
2. Accept the disclaimer dialog
3. Connect your Android device with USB debugging enabled
4. Select your device from the **Devices** dropdown
5. Select a user from the **Users** dropdown (or leave as "All Users")
6. The app will load the list of installed packages

---

## Main Menu

Click the **☰** hamburger button in the top-left corner to access:

| Menu Item | Description |
|-----------|-------------|
| **Settings** | Open application settings dialog |
| **Install/Uninstall** | Install or uninstall UAD-Shizuku on your system (desktop only) |
| **About** | View application information and version |
| **Exit** | Close the application |

---

## Tabs

UAD-Shizuku has four main tabs:

### 1. Debloat Tab

Manage and remove bloatware from your Android device.

#### Filter Categories

Packages are categorized based on the [UAD-NG bloat list](https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation):

| Category | Color | Description |
|----------|-------|-------------|
| **Recommended** | Green | Safe to remove, typically bloatware |
| **Advanced** | Blue | Requires some knowledge, may affect minor features |
| **Expert** | Orange | May break functionality, use with caution |
| **Unsafe** | Yellow | High risk, may cause system instability |
| **Unknown** | White | Not in the UAD-NG database |

#### Package Actions

For each package, you can:

- **Info** : View detailed package information
- **Uninstall** : Remove the package (user apps) or uninstall for current user (system apps)
- **Disable** : Disable the package without uninstalling
- **Enable** : Re-enable a disabled or removed package

#### Batch Operations

1. Select multiple packages using checkboxes
2. Use batch action buttons:
   - **Uninstall Selected**: Remove all selected packages
   - **Disable Selected**: Disable all selected packages
   - **Enable Selected**: Enable all selected packages
3. Click **Deselect All** to clear selection

#### Filters and Options

- **Show Only Enabled**: Hide disabled/removed packages
- **Hide System App**: Hide system applications
- **Filter**: Text search across package names, categories, and app info

---

### 2. Scan Tab

Scan installed applications for security threats using online services.

#### Scan Services

| Service | Description |
|---------|-------------|
| **VirusTotal** | Multi-antivirus scanner (requires API key) |
| **Hybrid Analysis** | Sandbox-based malware analysis (requires API key) |
| **IzzyRisk** | Risk scoring based on app permissions and behavior |

#### Scan Filters

Filter packages by scan results:
- **All**: Show all packages
- **Malicious**: Packages flagged as malware
- **Suspicious**: Packages with suspicious indicators
- **Safe**: Packages with clean scan results
- **Not Scanned**: Packages pending scan

#### How to Scan

1. Configure API keys in Settings (for VirusTotal and Hybrid Analysis)
2. Navigate to the Scan tab
3. Packages are automatically scanned when loaded
4. View scan progress in the notification area
5. Click **Stop** to cancel an ongoing scan

---

### 3. Apps Tab

Browse and install FOSS (Free and Open Source Software) applications.

#### App Sources

- **OFFA**: Apps from [android-foss](https://github.com/offa/android-foss) curated list
- **FMHY**: Apps from [FMHY mobile guide](https://fmhy.pages.dev/mobile)

#### Installing Apps

1. Select an app source from the dropdown
2. Browse or search for apps
3. Click the install button on an app card
4. The app downloads and installs via ADB

#### Options

- **Show Only Installable**: Filter to apps with downloadable links (F-Droid, GitHub, etc.)
- **Disable GitHub Install**: Skip GitHub releases as download source

---

### 4. Usage Tab

View usage statistics for your Android device (placeholder for future features).

---

## Settings

Access settings via Menu > Settings.

### Appearance

| Setting | Options | Description |
|---------|---------|-------------|
| **Language** | English, Korean | Application interface language |
| **Font** | System fonts | Custom font for the interface |
| **Theme** | Light, Dark, Auto | Color theme mode |
| **Contrast** | Normal, Medium, High | UI contrast level |
| **Text Style** | Small, Body, Button, Heading, Monospace | Override text size |

### API Keys

| Setting | Description |
|---------|-------------|
| **VirusTotal API Key** | API key for VirusTotal scanning ([Get key](https://www.virustotal.com/gui/my-apikey)) |
| **Hybrid Analysis API Key** | API key for Hybrid Analysis ([Get key](https://www.hybrid-analysis.com/my-account?tab=%23api-key-tab)) |

### Renderers

Enable app info fetching from external sources:

| Renderer | Description |
|----------|-------------|
| **Google Play** | Fetch app info from Google Play Store |
| **F-Droid** | Fetch app info from F-Droid repository |
| **APKMirror** | Fetch app info from APKMirror (for system apps) |

### Scan Options

| Setting | Description |
|---------|-------------|
| **VirusTotal Submit** | Automatically submit APKs not in VirusTotal database |
| **Hybrid Analysis Submit** | Automatically submit APKs not in Hybrid Analysis database |
| **HA Tag Blacklist** | Comma-separated tags to ignore in Hybrid Analysis results |

### Logging

| Setting | Description |
|---------|-------------|
| **Show Logs** | Display log panel at bottom of window |
| **Log Level** | Error, Warn, Info, Debug, Trace |

### Cache Management

| Action | Description |
|--------|-------------|
| **Invalidate Cache** | Clear local package cache |
| **Flush VirusTotal** | Clear VirusTotal scan results |
| **Flush Hybrid Analysis** | Clear Hybrid Analysis scan results |
| **Flush Google Play** | Clear Google Play app info cache |
| **Flush F-Droid** | Clear F-Droid app info cache |
| **Flush APKMirror** | Clear APKMirror app info cache |

---

## Keyboard Shortcuts

The application uses standard UI keyboard navigation. Specific shortcuts may vary by platform.

---

## Troubleshooting

### ADB Not Found

1. Click **Retry Detection** in the ADB installation dialog
2. If ADB is still not found, follow the platform-specific installation instructions in the dialog
3. For manual installation, ensure ADB is in your system PATH

### Device Not Detected

1. Ensure USB debugging is enabled on your device
2. Check that your device is properly connected
3. Click the **Refresh** button to rescan for devices
4. Accept the USB debugging authorization prompt on your device

### Package List Empty

1. Verify device connection
2. Check selected user (try "All Users")
3. Look for error messages in the log panel (enable in Settings)

### Scan Errors

1. Verify API keys are correct
2. Check internet connection
3. Review rate limits for API services
4. Check log panel for detailed error messages

---

## Platform Notes

### Desktop (Windows/Linux/macOS)

- Supports system installation via Menu > Install
- Auto-update checking for new releases
- ADB platform-tools auto-download

### Android (APK)

- Requires Shizuku for package management
- Wireless ADB connection to other devices
- Native Android UI experience

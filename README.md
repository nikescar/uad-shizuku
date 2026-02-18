<img src="./imgs/logo110.png" alt="drawing" width="120"/>

# UAD-Shizuku

debloat, scan virus, install foss apps on android.

UAD-Shizuku is using debloat apps info from [UAD-NG](https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation). <br/>
[UAD-NG](https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation) has better implementation on debloat app per manufacturer.

<img src="./imgs/screenshot.png" alt="drawing" width="1024"/>

* UAD-Shizuku does not afilliated with virustotal or hybrid-analysis or apkmirror. their service through this program may have ended without notification.
## Features

* debloat : app list with debloater using uad-ng and shizuku(adb-wireless).
* scan : scan app with virustotal and hybrid-analysis
* install : open source app list using [offa list](https://github.com/offa/android-foss?tab=readme-ov-file#-dialer), [fmhy list](https://fmhy.net/mobile#modded-apks).

## Download

| Arch          | Windows        | MacOS         | Linux        | Android        | IOS         |
|:--------------|:--------------:|:-------------:|:------------:|:--------------:|--------------:|
| X86_64(AMD64) | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku.exe) | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-x86_64-apple-darwin.tar.gz) | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-x86_64-unknown-linux-musl.tar.gz) | [APK](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-all-signed.apk)  | - |
| AARCH64(ARM64)| - | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-aarch64-apple-darwin.tar.gz) | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-aarch64-linux-android.tar.gz) | [APK](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-all-signed.apk) | - |

[Latest Release](https://github.com/nikescar/UAD-Shizuku/releases)<br/>
<br/>
<br/>

## Usage

* install android platform-tools(adb)
* run uad-shizuku application
* debloat, scan, install apps

## Settings

* Language : Korean, English
* Font : Default (NotoSansKr) or system fonts
* Text Style : Customize text rendering style
* Display Size : Desktop (1024x768), 1080p (1920x1080)
* Color Mode : Light, Auto, Dark
* Contrast : High, Medium, Normal
* VirusTotal API Key : API key for VirusTotal malware scanning service (4/min rate limit)
* Allow VirusTotal file upload : Upload APK files to VirusTotal for analysis if not found in database
* HybridAnalysis API Key : API key for Hybrid Analysis malware scanning service (200/min rate limit)
* Allow Hybrid Analysis file upload : Upload APK files to Hybrid Analysis for analysis if not found
* Google Play Renderer : Fetch and display app metadata from Google Play Store
* F-Droid Renderer : Fetch and display app metadata from F-Droid repository for non-system apps
* APKMirror Renderer : Fetch and display app metadata from APKMirror (app id search on apkmirror is not accurate. app info from apkmirror might be wrong.)
* APKMirror Auto-Upload : Auto-upload APKs when device version is newer than APKMirror
* APKMirror Email : Email address for APKMirror contributions
* APKMirror Name : Display name for APKMirror contributions
* Invalidate Cache : Clear all cached data (app info, scan results)
* Show Logs : Display application logs with selectable verbosity (Error/Warn/Info/Debug/Trace) 

## Debloat Tab

- Able to filter apps by debloat categories(Recommended/Advanced/Expert/Unsafe/Unknown)
- Select multiple apps and apply uninstall/disable/enable apps
- Uninstall operation removes userdata while disable operation keeps userdata

### Application State and Actions

* DEFAULT : installed app from user or pre-installed
* ENABLED : enabled app from user or pre-installed
* DISABLED : disabled system app
* DISABLED_USER : disabled user app

### Installation Reason Codes

These codes help identify why an app was installed on the device, which is useful when deciding whether to debloat or keep specific applications.

* UNKNOWN : user installer
* SYSTEM : pre-installed
* POLICY : installed by mdm policy 
* DEVICE_RESTORE : *
* DEVICE_SETUP : *
* USER_REQUESTED : *

## Scan Tab

Scan apps with virustotal and hybridanalysis

## App Tab

Get list of FOSS apps, and install directly. github and fdroid auto-install supports.

## Star History

<a href="https://www.star-history.com/#nikescar/uad-shizuku&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=nikescar/uad-shizuku&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=nikescar/uad-shizuku&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=nikescar/uad-shizuku&type=date&legend=top-left" />
 </picture>
</a>

<details markdown>
<summary> Todos </summary>

## Todos

* separate malicious filter into non-ignored app and ignored app
* match datatable color to theme.
* windows app signing & attestation.
* web version through webusb on browser
* usage history & netstats history
* detach app installer from app table
* logcat ?
* android : add obtainium list https://apps.obtainium.imranr.dev/data.json?rand=6958.845712989482
* izzy auto install
* github install changes to selectable. installation source selectable.
* fix download & install operation.
  - izzy downloader - 

</details>

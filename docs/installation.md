# Installation

## Windows

### Prerequisites

* OpenGL/WebGL
* VC Redistributable
* ADB Platform-tools

### Downloads

#### VC Redistributable

| Architecture | Download Link |
|-------------|---------------|
| ARM64 | https://aka.ms/vc14/vc_redist.arm64.exe |
| X86 | https://aka.ms/vc14/vc_redist.x86.exe |
| X64 | https://aka.ms/vc14/vc_redist.x64.exe |

#### OpenGL/WebGL (Mesa)

Download from: https://github.com/pal1000/mesa-dist-win/releases/latest

#### ADB Platform-tools

**Download:** https://dl.google.com/android/repository/platform-tools-latest-windows.zip

**Installation (PowerShell - Run as Administrator):**

```powershell
# Download and extract platform-tools
$sdkPath = "$env:LOCALAPPDATA\Android\Sdk"
$ptPath = "$sdkPath\platform-tools"
New-Item -ItemType Directory -Force -Path $sdkPath | Out-Null
Invoke-WebRequest -Uri "https://dl.google.com/android/repository/platform-tools-latest-windows.zip" -OutFile "$env:TEMP\platform-tools.zip"
Expand-Archive -Path "$env:TEMP\platform-tools.zip" -DestinationPath $sdkPath -Force
Remove-Item "$env:TEMP\platform-tools.zip"

# Add to PATH permanently via registry
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -notlike "*$ptPath*") {
    [Environment]::SetEnvironmentVariable("Path", "$currentPath;$ptPath", "User")
    Write-Host "Added $ptPath to user PATH. Restart your terminal."
} else {
    Write-Host "Path already contains platform-tools."
}
Write-Host "Installation complete. Run 'adb version' in a new terminal to verify."
```

## macOS

### Prerequisites

* ADB Platform-tools

### ADB Platform-tools

**Download:** https://dl.google.com/android/repository/platform-tools-latest-darwin.zip

**Installation (Terminal - Zsh):**

```bash
# Download and extract platform-tools
SDK_PATH="$HOME/Library/Android/sdk"
PT_PATH="$SDK_PATH/platform-tools"
mkdir -p "$SDK_PATH"
curl -o /tmp/platform-tools.zip https://dl.google.com/android/repository/platform-tools-latest-darwin.zip
unzip -o /tmp/platform-tools.zip -d "$SDK_PATH"
rm /tmp/platform-tools.zip

# Add to PATH in shell config
SHELL_RC="$HOME/.zshrc"
if ! grep -q "platform-tools" "$SHELL_RC" 2>/dev/null; then
    echo "export PATH=\"\$PATH:$PT_PATH\"" >> "$SHELL_RC"
    echo "Added to $SHELL_RC. Run: source $SHELL_RC"
else
    echo "PATH already configured."
fi
echo "Run 'adb version' to verify installation."
```

## Linux

### Prerequisites

* ADB Platform-tools

### ADB Platform-tools

**Download:** https://dl.google.com/android/repository/platform-tools-latest-linux.zip

**Installation (Terminal - Bash/Zsh):**

```bash
# Download and extract platform-tools
SDK_PATH="$HOME/Android/Sdk"
PT_PATH="$SDK_PATH/platform-tools"
mkdir -p "$SDK_PATH"
curl -o /tmp/platform-tools.zip https://dl.google.com/android/repository/platform-tools-latest-linux.zip
unzip -o /tmp/platform-tools.zip -d "$SDK_PATH"
rm /tmp/platform-tools.zip

# Add to PATH in shell config
SHELL_RC="$HOME/.bashrc"
[ -f "$HOME/.zshrc" ] && SHELL_RC="$HOME/.zshrc"
if ! grep -q "platform-tools" "$SHELL_RC" 2>/dev/null; then
    echo "export PATH=\"\$PATH:$PT_PATH\"" >> "$SHELL_RC"
    echo "Added to $SHELL_RC. Run: source $SHELL_RC"
else
    echo "PATH already configured."
fi
echo "Run 'adb version' to verify installation."
```


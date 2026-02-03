# 설치 가이드

## Windows

### 사전 요구 사항

* OpenGL/WebGL
* VC 재배포 가능 패키지
* ADB Platform-tools

### 다운로드

#### VC 재배포 가능 패키지

| 아키텍처 | 다운로드 링크 |
|---------|---------------|
| ARM64 | https://aka.ms/vc14/vc_redist.arm64.exe |
| X86 | https://aka.ms/vc14/vc_redist.x86.exe |
| X64 | https://aka.ms/vc14/vc_redist.x64.exe |

#### OpenGL/WebGL (Mesa)

다운로드: https://github.com/pal1000/mesa-dist-win/releases/latest

#### ADB Platform-tools

**다운로드:** https://dl.google.com/android/repository/platform-tools-latest-windows.zip

**설치 방법 (PowerShell - 관리자 권한으로 실행):**

```powershell
# platform-tools 다운로드 및 압축 해제
$sdkPath = "$env:LOCALAPPDATA\Android\Sdk"
$ptPath = "$sdkPath\platform-tools"
New-Item -ItemType Directory -Force -Path $sdkPath | Out-Null
Invoke-WebRequest -Uri "https://dl.google.com/android/repository/platform-tools-latest-windows.zip" -OutFile "$env:TEMP\platform-tools.zip"
Expand-Archive -Path "$env:TEMP\platform-tools.zip" -DestinationPath $sdkPath -Force
Remove-Item "$env:TEMP\platform-tools.zip"

# 레지스트리를 통해 PATH에 영구적으로 추가
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -notlike "*$ptPath*") {
    [Environment]::SetEnvironmentVariable("Path", "$currentPath;$ptPath", "User")
    Write-Host "$ptPath 를 사용자 PATH에 추가했습니다. 터미널을 다시 시작하세요."
} else {
    Write-Host "PATH에 이미 platform-tools가 포함되어 있습니다."
}
Write-Host "설치 완료. 새 터미널에서 'adb version'을 실행하여 확인하세요."
```

## macOS

### 사전 요구 사항

* ADB Platform-tools

### ADB Platform-tools

**다운로드:** https://dl.google.com/android/repository/platform-tools-latest-darwin.zip

**설치 방법 (터미널 - Zsh):**

```bash
# platform-tools 다운로드 및 압축 해제
SDK_PATH="$HOME/Library/Android/sdk"
PT_PATH="$SDK_PATH/platform-tools"
mkdir -p "$SDK_PATH"
curl -o /tmp/platform-tools.zip https://dl.google.com/android/repository/platform-tools-latest-darwin.zip
unzip -o /tmp/platform-tools.zip -d "$SDK_PATH"
rm /tmp/platform-tools.zip

# 셸 설정에 PATH 추가
SHELL_RC="$HOME/.zshrc"
if ! grep -q "platform-tools" "$SHELL_RC" 2>/dev/null; then
    echo "export PATH=\"\$PATH:$PT_PATH\"" >> "$SHELL_RC"
    echo "$SHELL_RC에 추가했습니다. 실행: source $SHELL_RC"
else
    echo "PATH가 이미 설정되어 있습니다."
fi
echo "'adb version'을 실행하여 설치를 확인하세요."
```

## Linux

### 사전 요구 사항

* ADB Platform-tools

### ADB Platform-tools

**다운로드:** https://dl.google.com/android/repository/platform-tools-latest-linux.zip

**설치 방법 (터미널 - Bash/Zsh):**

```bash
# platform-tools 다운로드 및 압축 해제
SDK_PATH="$HOME/Android/Sdk"
PT_PATH="$SDK_PATH/platform-tools"
mkdir -p "$SDK_PATH"
curl -o /tmp/platform-tools.zip https://dl.google.com/android/repository/platform-tools-latest-linux.zip
unzip -o /tmp/platform-tools.zip -d "$SDK_PATH"
rm /tmp/platform-tools.zip

# 셸 설정에 PATH 추가
SHELL_RC="$HOME/.bashrc"
[ -f "$HOME/.zshrc" ] && SHELL_RC="$HOME/.zshrc"
if ! grep -q "platform-tools" "$SHELL_RC" 2>/dev/null; then
    echo "export PATH=\"\$PATH:$PT_PATH\"" >> "$SHELL_RC"
    echo "$SHELL_RC에 추가했습니다. 실행: source $SHELL_RC"
else
    echo "PATH가 이미 설정되어 있습니다."
fi
echo "'adb version'을 실행하여 설치를 확인하세요."
```


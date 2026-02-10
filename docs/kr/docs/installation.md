# 설치 가이드

이 가이드는 Windows, macOS, Linux에서 UAD-Shizuku와 필수 구성 요소를 설치하는 방법을 안내합니다. UAD-Shizuku는 Android 기기와 통신하여 블로트웨어 제거, 앱 검사, 앱 설치를 수행하기 위해 ADB(Android Debug Bridge)가 필요합니다.

## 개요

UAD-Shizuku를 사용하기 전에 다음이 필요합니다:

1. **ADB Platform-tools** - 모든 플랫폼에서 Android 기기와 통신하는 데 필요
2. **VC 재배포 가능 패키지** (Windows 전용) - Microsoft Visual C++ 런타임 라이브러리
3. **OpenGL/WebGL 드라이버** (Windows 전용) - GUI용 그래픽 라이브러리

---

## Windows

### 사전 요구 사항

| 구성 요소 | 용도 | 필수 여부 |
|-----------|------|----------|
| ADB Platform-tools | USB/Wi-Fi를 통해 Android 기기와 통신 | 예 |
| VC 재배포 가능 패키지 | 애플리케이션용 Microsoft C++ 런타임 | 예 |
| OpenGL/WebGL (Mesa) | GUI 인터페이스 그래픽 렌더링 | 예 (GPU 드라이버가 없는 경우) |

### 1단계: VC 재배포 가능 패키지 설치

Visual C++ 재배포 가능 패키지는 Visual Studio로 빌드된 C++ 애플리케이션을 실행하는 데 필요한 런타임 구성 요소를 설치합니다.

시스템 아키텍처에 맞는 버전을 다운로드하세요:

| 아키텍처 | 다운로드 링크 |
|---------|---------------|
| ARM64 | https://aka.ms/vc14/vc_redist.arm64.exe |
| X86 (32비트) | https://aka.ms/vc14/vc_redist.x86.exe |
| X64 (64비트) | https://aka.ms/vc14/vc_redist.x64.exe |

다운로드한 설치 프로그램을 실행하고 안내에 따라 설치를 완료하세요.

### 2단계: OpenGL/WebGL (Mesa) 설치 - 필요한 경우

Mesa는 소프트웨어 기반 OpenGL 렌더링을 제공합니다. 다음의 경우에**만 필요**합니다:
- GPU 드라이버가 OpenGL 3.3 이상을 지원하지 않는 경우
- 가상 머신에서 실행하는 경우
- UAD-Shizuku 실행 시 그래픽 관련 오류가 발생하는 경우

**다운로드:** https://github.com/pal1000/mesa-dist-win/releases/latest

압축 파일을 해제하고 포함된 배포 스크립트를 실행하세요.

### 3단계: ADB Platform-tools 설치

ADB (Android Debug Bridge)는 Android 기기와 통신할 수 있는 명령줄 도구입니다.

**다운로드:** https://dl.google.com/android/repository/platform-tools-latest-windows.zip

#### 자동 설치 (권장)

**PowerShell을 관리자 권한으로** 열고 다음을 실행하세요:

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

#### 수동 설치

1. 위 링크에서 ZIP 파일을 다운로드합니다
2. `C:\Users\<사용자이름>\AppData\Local\Android\Sdk\platform-tools`에 압축을 해제합니다
3. 시스템 PATH에 폴더를 추가합니다:
   - `Win + X`를 누르고 "시스템"을 선택합니다
   - "고급 시스템 설정"을 클릭합니다
   - "환경 변수"를 클릭합니다
   - "사용자 변수"에서 "Path"를 선택하고 "편집"을 클릭합니다
   - "새로 만들기"를 클릭하고 platform-tools 경로를 추가합니다
   - 확인을 클릭하여 저장합니다

### 설치 확인

**새** 터미널 (명령 프롬프트 또는 PowerShell)을 열고 다음을 실행하세요:

```
adb version
```

다음과 같은 출력이 표시되어야 합니다:
```
Android Debug Bridge version 1.0.41
Version 35.0.0-11411520
```

---

## macOS

### 사전 요구 사항

| 구성 요소 | 용도 | 필수 여부 |
|-----------|------|----------|
| ADB Platform-tools | USB/Wi-Fi를 통해 Android 기기와 통신 | 예 |

macOS는 다른 모든 필수 구성 요소 (OpenGL 지원, C++ 런타임)를 포함하고 있습니다.

### ADB Platform-tools 설치

**다운로드:** https://dl.google.com/android/repository/platform-tools-latest-darwin.zip

#### 자동 설치 (권장)

**터미널**을 열고 다음을 실행하세요:

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

#### Homebrew 사용 (대안)

Homebrew가 설치되어 있다면:

```bash
brew install android-platform-tools
```

### 설치 확인

**새** 터미널 창을 열거나 `source ~/.zshrc`를 실행한 후 다음을 실행하세요:

```bash
adb version
```

---

## Linux

### 사전 요구 사항

| 구성 요소 | 용도 | 필수 여부 |
|-----------|------|----------|
| ADB Platform-tools | USB/Wi-Fi를 통해 Android 기기와 통신 | 예 |

대부분의 Linux 배포판은 기본적으로 OpenGL 및 C++ 런타임 라이브러리를 포함합니다.

### ADB Platform-tools 설치

**다운로드:** https://dl.google.com/android/repository/platform-tools-latest-linux.zip

#### 방법 1: 패키지 관리자 (권장)

대부분의 배포판은 패키지 관리자를 통해 ADB를 제공합니다:

**Debian/Ubuntu:**
```bash
sudo apt update && sudo apt install android-tools-adb
```

**Fedora:**
```bash
sudo dnf install android-tools
```

**Arch Linux:**
```bash
sudo pacman -S android-tools
```

#### 방법 2: 수동 설치

**터미널**을 열고 다음을 실행하세요:

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

### USB 권한 설정 (Linux 전용)

Linux에서는 루트 권한 없이 Android 기기에 접근하기 위해 udev 규칙을 구성해야 할 수 있습니다:

```bash
# Android 기기용 udev 규칙 생성
sudo tee /etc/udev/rules.d/51-android.rules << 'EOF'
SUBSYSTEM=="usb", ATTR{idVendor}=="*", MODE="0666", GROUP="plugdev"
EOF

# udev 규칙 다시 로드
sudo udevadm control --reload-rules
sudo udevadm trigger

# 사용자를 plugdev 그룹에 추가
sudo usermod -aG plugdev $USER
```

그룹 변경 사항을 적용하려면 로그아웃 후 다시 로그인하세요.

### 설치 확인

**새** 터미널 창을 열거나 `source ~/.bashrc`를 실행한 후 다음을 실행하세요:

```bash
adb version
```

---

## Android (Shizuku)

Android 기기에서 UAD-Shizuku를 실행할 때는 로컬에서 ADB와 유사한 기능을 제공하는 **Shizuku**가 필요합니다.

### 사전 요구 사항

| 구성 요소 | 용도 | 필수 여부 |
|-----------|------|----------|
| Shizuku 앱 | 기기에서 ADB 기능 제공 | 예 |
| 개발자 옵션 | USB/무선 디버깅 활성화 | 예 |

### 1단계: Shizuku 앱 설치

Google Play 스토어에서 Shizuku를 설치하세요:

**다운로드:** https://play.google.com/store/apps/details?id=moe.shizuku.privileged.api

또는 대체 소스를 선호하는 경우 F-Droid 또는 GitHub 릴리스에서 설치할 수 있습니다.

### 2단계: 개발자 모드 활성화

1. Android 기기에서 **설정**을 엽니다
2. **휴대전화 정보** (또는 **태블릿 정보**)로 이동합니다
3. **빌드 번호**를 찾습니다 ("소프트웨어 정보" 아래에 있을 수 있음)
4. **빌드 번호**를 7번 탭합니다
5. "개발자가 되었습니다!" 메시지가 표시되어야 합니다

### 3단계: 무선 디버깅 활성화

1. **설정** > **개발자 옵션**으로 이동합니다
2. **무선 디버깅**을 활성화합니다
3. **무선 디버깅**을 탭하여 설정을 엽니다
4. **페어링 코드로 기기 페어링**을 탭합니다
5. IP 주소, 포트, 페어링 코드를 확인합니다

**중요:** 기기는 이 화면에 머물러야 합니다. 아직 닫지 마세요.

### 4단계: Shizuku 서비스 시작

1. **Shizuku** 앱을 엽니다
2. **페어링** 또는 **시작** 버튼을 탭합니다
3. 메시지가 표시되면 3단계의 페어링 코드를 사용합니다
4. Shizuku가 연결될 때까지 기다립니다 (상태가 "실행 중"으로 표시되어야 함)
5. Shizuku의 권한 요청을 승인합니다

**참고:** Shizuku 시작에 실패하는 경우:
- 무선 디버깅이 여전히 활성화되어 있는지 확인하세요
- 기기를 재부팅하고 3단계부터 다시 시도하세요
- 기기가 절전 모드가 아닌지 확인하세요

### 5단계: UAD-Shizuku로 돌아가기

1. **UAD-Shizuku** 앱을 엽니다
2. **새로고침 버튼** (☰) 메뉴를 탭합니다
3. **새로고침**을 선택하여 기기를 감지합니다
4. UAD-Shizuku가 Shizuku 접근을 요청하면 권한을 부여합니다

이제 기기 목록에서 기기가 "local"로 표시되어야 합니다.

### 설치 확인

- Shizuku 앱에서 Shizuku 상태가 "실행 중"으로 표시되어야 합니다
- UAD-Shizuku의 기기 드롭다운에 "local"이 표시되어야 합니다
- 사용자를 선택하고 설치된 패키지를 볼 수 있어야 합니다

### Shizuku 문제 해결

**Shizuku가 시작되지 않음:**
- 무선 디버깅이 활성화되어 있는지 확인하세요
- 기기를 재시작하세요
- Shizuku를 제거한 후 다시 설치해 보세요
- 다른 앱이 포트 5555 (ADB 포트)를 사용하고 있지 않은지 확인하세요

**UAD-Shizuku가 기기를 감지하지 못함:**
- Shizuku가 "실행 중" 상태를 표시하는지 확인하세요
- UAD-Shizuku가 요청할 때 Shizuku 권한을 부여하세요
- UAD-Shizuku에서 새로고침 버튼을 클릭하세요
- Shizuku와 UAD-Shizuku 앱을 모두 재시작하세요

**권한 거부 오류:**
- Shizuku 앱을 열고 모든 권한을 부여하세요
- 개발자 옵션이 여전히 활성화되어 있는지 확인하세요
- 무선 디버깅이 활성화되어 있는지 확인하세요

---

## 문제 해결

### 설치 후 ADB를 찾을 수 없음

- **Windows:** 설치 후 새 터미널을 여세요. PATH 변경 사항은 새 세션이 필요합니다.
- **macOS/Linux:** `source ~/.zshrc` 또는 `source ~/.bashrc`를 실행하여 셸 구성을 다시 로드하세요.

### 기기가 감지되지 않음

1. Android 기기에서 **USB 디버깅**을 활성화하세요:
   - 설정 > 휴대전화 정보로 이동합니다
   - "빌드 번호"를 7번 탭하여 개발자 옵션을 활성화합니다
   - 설정 > 개발자 옵션으로 이동합니다
   - "USB 디버깅"을 활성화합니다

2. `adb devices`를 실행하여 기기가 목록에 표시되는지 확인합니다

3. 기기가 "unauthorized"로 표시되면 휴대폰에서 인증 프롬프트를 확인하세요

### Windows에서 그래픽 문제

UAD-Shizuku에서 그래픽 오류나 빈 창이 표시되는 경우:
1. GPU 드라이버를 업데이트합니다
2. 그래도 해결되지 않으면 위 링크에서 Mesa를 설치합니다

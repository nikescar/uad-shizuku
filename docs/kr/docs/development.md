---
title: 개발 가이드
---

# 개발 가이드

이 가이드는 데스크톱 애플리케이션, Android APK, 문서 빌드를 포함한 UAD-Shizuku 개발 환경 설정에 대해 다룹니다.

## 사전 요구 사항

시작하기 전에 다음이 설치되어 있는지 확인하세요:

| 도구 | 버전 | 용도 |
|------|------|------|
| Rust | 최신 stable | 애플리케이션의 핵심 언어 |
| Git | 최신 버전 | 소스 코드 관리 |
| ADB | 최신 버전 | Android 기기 통신 |

### Rust 설치

UAD-Shizuku를 빌드하려면 Rust가 필요합니다. rustup을 사용하여 설치하세요:

**Windows:**
https://rustup.rs/ 에서 설치 프로그램을 다운로드하고 실행하세요

**macOS/Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

설치 확인:
```bash
rustc --version
cargo --version
```

### 권장 IDE 설정

최상의 개발 경험을 위해 다음을 권장합니다:

- **Visual Studio Code** 및 다음 확장 프로그램:
  - rust-analyzer (Rust 언어 지원)
  - Even Better TOML (Cargo.toml 편집)
  - CodeLLDB (디버깅)

- **RustRover** (JetBrains Rust IDE)

---

## 프로젝트 구조

```
uad-shizuku/
├── src/              # 데스크톱 애플리케이션 소스 코드
├── mobile/           # Android 빌드 파일 및 스크립트
│   ├── app/          # Android 앱 모듈
│   ├── build.sh      # Android 빌드 스크립트
│   └── qemu.sh       # QEMU 컨테이너 관리
├── docs/             # 문서 (이 사이트)
├── resources/        # 정적 자산 및 리소스
└── Cargo.toml        # Rust 프로젝트 설정
```

---

## 데스크톱 애플리케이션 빌드

데스크톱 애플리케이션은 Windows, macOS, Linux용으로 빌드할 수 있습니다.

### 저장소 클론

```bash
git clone https://github.com/nikescar/uad-shizuku
cd uad-shizuku
```

### 개발 빌드

개발 중 빠른 반복 작업을 위해:

```bash
cargo build
```

디버그 바이너리는 `target/debug/uad-shizuku` (Windows에서는 `uad-shizuku.exe`)에 있습니다.

### 릴리스 빌드

최적화된 프로덕션 빌드를 위해:

```bash
cargo build --release
```

릴리스 바이너리는 `target/release/uad-shizuku`에 있습니다.

### 애플리케이션 실행

```bash
# 개발 모드
cargo run

# 릴리스 모드
cargo run --release
```

### 크로스 컴파일 타겟

UAD-Shizuku는 여러 타겟에 대한 빌드를 지원합니다:

| 타겟 | Triple |
|------|--------|
| Windows x64 | `x86_64-pc-windows-msvc` |
| macOS x64 | `x86_64-apple-darwin` |
| macOS ARM64 | `aarch64-apple-darwin` |
| Linux x64 | `x86_64-unknown-linux-musl` |
| Linux ARM64 | `aarch64-linux-android` |

특정 타겟용으로 빌드하려면:
```bash
rustup target add <target-triple>
cargo build --release --target <target-triple>
```

---

## Android APK 빌드

Android 버전은 다양한 호스트 시스템에서 일관된 빌드를 보장하기 위해 QEMU 기반 Alpine Linux 컨테이너를 사용하여 빌드됩니다.

### Android 빌드 사전 요구 사항

- 시스템에 QEMU 설치
- SSH 클라이언트
- rsync

### 빌드 단계

```bash
cd mobile

# 1단계: QEMU 게스트 컨테이너 초기화 (처음 한 번만)
./qemu.sh init

# 2단계: QEMU 게스트 시작
./qemu.sh run

# 3단계: 프로젝트 파일을 컨테이너로 동기화
./qemu.sh syncto

# 4단계: Alpine Linux 컨테이너에 SSH 접속
./qemu.sh ssh

# 5단계: 컨테이너 내부에서 빌드
cd /opt/uad-shizuku/mobile && ./build.sh

# 6단계: SSH 종료 후 빌드된 파일을 다시 동기화
exit
./qemu.sh syncfrom

# 7단계: 빌드된 APK 찾기
ls app/build/outputs/apk/debug/
ls app/build/outputs/apk/release/
```

### 디버그 빌드 설치

```bash
adb install app/build/outputs/apk/debug/app-arm64-v8a-debug.apk
```

### QEMU 스크립트 명령어

| 명령어 | 설명 |
|--------|------|
| `./qemu.sh init` | 빌드 컨테이너 초기화 및 설정 |
| `./qemu.sh run` | QEMU 가상 머신 시작 |
| `./qemu.sh stop` | 실행 중인 QEMU 인스턴스 중지 |
| `./qemu.sh ssh` | 실행 중인 컨테이너에 SSH 접속 |
| `./qemu.sh syncto` | 프로젝트 파일을 컨테이너로 복사 |
| `./qemu.sh syncfrom` | 빌드된 파일을 컨테이너에서 복사 |

---

## 문서 빌드

문서 사이트는 mdx-sitegen-solidbase를 사용하여 빌드됩니다.

### 사전 요구 사항

- Node.js (npx용)
- darkhttpd (로컬 미리보기용, 선택 사항)

### 문서 빌드

```bash
# 프로젝트 루트에서 문서 빌드
npx github:nikescar/mdx-sitegen-solidbase

# 로컬에서 미리보기
darkhttpd .solidbase/.output/public
```

빌드된 사이트는 `.solidbase/.output/public/`에 있습니다.

### 문서 구조

```
docs/
├── index.md           # 홈 페이지
├── installation.md    # 설치 가이드
├── development.md     # 이 파일
├── usage.md           # 사용 가이드
└── kr/                # 한국어 번역
    └── docs/
        ├── index.md
        ├── installation.md
        ├── development.md
        └── usage.md
```

---

## 개발 워크플로우

### 변경 사항 적용

1. 기능이나 수정을 위한 새 브랜치 생성:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. 변경 사항을 적용하고 로컬에서 테스트:
   ```bash
   cargo build
   cargo run
   ```

3. 설명적인 메시지와 함께 변경 사항 커밋:
   ```bash
   git add .
   git commit -m "feat: 기능 설명 추가"
   ```

4. 푸시하고 풀 리퀘스트 생성:
   ```bash
   git push origin feature/your-feature-name
   ```

### 코드 포맷팅

코드가 Rust 포맷팅 표준을 따르도록 합니다:

```bash
cargo fmt
```

### 린팅

일반적인 문제 확인:

```bash
cargo clippy
```

---

## 문제 해결

### 빌드 오류

**누락된 의존성:**
```bash
# Debian/Ubuntu
sudo apt install build-essential pkg-config libssl-dev

# Fedora
sudo dnf install gcc pkg-config openssl-devel

# macOS (Homebrew 사용)
brew install openssl pkg-config
```

### QEMU 문제

**QEMU가 시작되지 않음:**
- QEMU가 설치되어 있는지 확인: `qemu-system-x86_64 --version`
- 사용 가능한 디스크 공간 확인
- 특정 오류에 대한 QEMU 로그 검토

**동기화 실패:**
- QEMU 게스트가 실행 중인지 확인
- 호스트와 게스트 간 네트워크 연결 확인

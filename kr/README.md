# UAD-Shizuku

안드로이드에서 블로트웨어 제거, 바이러스 검사, FOSS 앱 설치를 지원합니다.

UAD-Shizuku는 [UAD-NG](https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation)의 블로트웨어 앱 정보를 사용합니다. <br/>
[UAD-NG](https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation)는 제조사별 블로트웨어 제거에 더 나은 구현을 제공합니다.
* UAD-Shizuku는 VirusTotal, Hybrid-Analysis, APKMirror와 제휴 관계가 없습니다. 이 프로그램을 통한 해당 서비스는 예고 없이 종료될 수 있습니다.

## 기능

* 블로트웨어 제거 : UAD-NG와 Shizuku(ADB 무선)를 사용한 앱 목록 및 블로트웨어 제거 기능
* 검사 : VirusTotal과 Hybrid-Analysis를 통한 앱 검사
* 설치 : [offa 목록](https://github.com/offa/android-foss?tab=readme-ov-file#-dialer), [fmhy 목록](https://fmhy.net/mobile#modded-apks)을 사용한 오픈소스 앱 목록

## 다운로드

| 아키텍처       | Windows        | MacOS         | Linux        | Android        | IOS         |
|:--------------|:--------------:|:-------------:|:------------:|:--------------:|--------------:|
| X86_64(AMD64) | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-x86_64-pc-windows-msvc.tar.gz) | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-x86_64-apple-darwin.tar.gz) | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-x86_64-unknown-linux-musl.tar.gz) | - | - |
| AARCH64(ARM64)| - | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-aarch64-apple-darwin.tar.gz) | [GUI](https://github.com/nikescar/uad-shizuku/releases/latest/download/uad-shizuku-aarch64-linux-android.tar.gz) | - | - |

[최신 릴리스](https://github.com/nikescar/UAD-Shizuku/releases)<br/>
<br/>
<br/>

## 사용법

* Android platform-tools(adb) 설치
* uad-shizuku 애플리케이션 실행
* 블로트웨어 제거, 검사, 앱 설치

## 설정

* 언어 : 한국어, 영어
* 폰트 : 기본 (NotoSansKr) 또는 시스템 폰트
* 텍스트 스타일 : 텍스트 렌더링 스타일 사용자 정의
* 화면 크기 : 데스크톱 (1024x768), 1080p (1920x1080)
* 색상 모드 : 라이트, 자동, 다크
* 대비 : 높음, 중간, 보통
* VirusTotal API 키 : VirusTotal 악성코드 검사 서비스용 API 키 (분당 4회 제한)
* VirusTotal 파일 업로드 허용 : 데이터베이스에 없는 경우 APK 파일을 VirusTotal에 업로드하여 분석
* HybridAnalysis API 키 : Hybrid Analysis 악성코드 검사 서비스용 API 키 (분당 200회 제한)
* Hybrid Analysis 파일 업로드 허용 : 데이터베이스에 없는 경우 APK 파일을 Hybrid Analysis에 업로드하여 분석
* Google Play 렌더러 : Google Play 스토어에서 앱 메타데이터 가져와서 표시
* F-Droid 렌더러 : 비시스템 앱에 대해 F-Droid 저장소에서 앱 메타데이터 가져와서 표시
* APKMirror 렌더러 : APKMirror에서 앱 메타데이터 가져와서 표시 (APKMirror에서의 앱 ID 검색은 정확하지 않을 수 있습니다. APKMirror의 앱 정보가 잘못되었을 수 있습니다.)
* APKMirror 자동 업로드 : 기기 버전이 APKMirror보다 최신인 경우 APK 자동 업로드
* APKMirror 이메일 : APKMirror 기여용 이메일 주소
* APKMirror 이름 : APKMirror 기여용 표시 이름
* 캐시 무효화 : 모든 캐시된 데이터 삭제 (앱 정보, 검사 결과)
* 로그 표시 : 선택 가능한 상세 수준으로 애플리케이션 로그 표시 (Error/Warn/Info/Debug/Trace)

## 블로트웨어 제거 탭

- 블로트웨어 카테고리별 앱 필터링 가능 (권장/고급/전문가/위험/알 수 없음)
- 여러 앱 선택 후 제거/비활성화/활성화 일괄 적용
- 제거 작업은 사용자 데이터를 삭제하고, 비활성화 작업은 사용자 데이터를 유지합니다

### 앱 상태 및 작업

* DEFAULT : 사용자가 설치했거나 사전 설치된 앱
* ENABLED : 사용자가 활성화했거나 사전 설치된 앱
* DISABLED : 비활성화된 시스템 앱
* DISABLED_USER : 비활성화된 사용자 앱

### 설치 이유 코드

이 코드들은 앱이 기기에 설치된 이유를 식별하는 데 도움이 되며, 특정 애플리케이션을 제거할지 유지할지 결정할 때 유용합니다.

* UNKNOWN : 사용자 설치
* SYSTEM : 사전 설치
* POLICY : MDM 정책에 의해 설치됨
* DEVICE_RESTORE : *
* DEVICE_SETUP : *
* USER_REQUESTED : *

## 검사 탭

VirusTotal과 HybridAnalysis로 앱 검사

## 앱 탭

FOSS 앱 목록을 가져와 직접 설치합니다. GitHub 및 F-Droid 자동 설치를 지원합니다.

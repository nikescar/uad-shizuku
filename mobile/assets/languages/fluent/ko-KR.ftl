app-title = UAD-Shizuku
app-description = Shizuku를 사용하는 유니버설 안드로이드 디블로터.
settings = 설정
exit = 종료
debloat = 디블로트
scan = 검사
apps = 앱
usage = 사용량
devices = 장치:
users = 사용자:
select-device = 장치 선택
all-users = 모든 사용자
refresh = 새로고침
logs = 애플리케이션 로그:
theme-mode = 테마 모드:
contrast-level = 대비 수준:
log-level = 로그 레벨:
ok = 확인

# Apps Tab
app-list = 앱 목록:
select-app-list = 앱 목록 선택
apps-found = ({ $count } 앱)
category = 카테고리
app-name = 앱 이름
links = 링크
install = 설치
installed = 설치됨

# Debloat Tab
all = 전체 ({ $count })
recommended = 권장 ({ $enabled }/{ $total })
advanced = 고급 ({ $enabled }/{ $total })
expert = 전문가 ({ $enabled }/{ $total })
unsafe = 위험 ({ $enabled }/{ $total })
unknown = 알 수 없음 ({ $enabled }/{ $total })
label-recommended = 권장
label-advanced = 고급
label-expert = 전문가
label-unsafe = 위험
label-unknown = 알 수 없음
show-only-enabled = 활성화된 항목만 표시
hide-system-app = 시스템 앱 숨기기
show-only-installable = 설치 가능한 항목만 표시
col-package-name = 패키지 이름
col-debloat-category = 디블로트 카테고리
col-runtime-permissions = 런타임 권한
col-enabled = 활성화 여부
col-install-reason = 설치 사유
col-tasks = 작업
no-packages-loaded = 패키지가 로드되지 않았습니다. 드롭다운에서 장치를 선택하세요.
deselect-all = 전체 선택 해제
selected-count = 선택됨: { $count }
uninstall-selected = 선택 항목 제거 ({ $count })
disable-selected = 선택 항목 비활성화 ({ $count })
enable-selected = 선택 항목 활성화 ({ $count })

# Scan Tab
virustotal-filter = VirusTotal 필터:
scanning-packages = 패키지 검사 중...
malicious = 악성 ({ $count })
suspicious = 의심 ({ $count })
safe = 안전 ({ $count })
no-specific-threat = 특정 위협 없음 ({ $count })
not-scanned = 미검사 ({ $count })
hybrid-analysis-filter = Hybrid-Analysis 필터:
col-izzy-risk = IzzyRisk
col-virustotal = VirusTotal
col-hybrid-analysis = HybridAnalysis
scan-not-initialized = 시작되지 않음
scan-not-scanned = 미검사
scan-scanning = 검사 중... ({ $scanned }/{ $total })
scan-error = 오류
scan-error-msg = 오류: { $message }
scan-skip = 건너뜀
scan-404 = 404
scan-malicious = 악성 { $count }/{ $total }
scan-suspicious = 의심 { $count }/{ $total }
scan-clean = 안전 { $count }/{ $total }
scan-no-results = 결과 없음
ha-malicious = 악성
ha-malicious-score = 악성 ({ $score })
ha-suspicious = 의심
ha-suspicious-score = 의심 ({ $score })
ha-whitelisted = 안전
ha-whitelisted-score = 안전 ({ $score })
ha-no-result = 결과 없음
ha-no-specific-threat = 특정 위협 없음
ha-no-specific-threat-score = 특정 위협 없음 ({ $score })
ha-skipped = 건너뜀
ha-rate-limited = 속도 제한
ha-submitted = 제출됨
ha-pending = 대기 중 ({ $jobid }...)
ha-pending-analysis = 분석 대기 중
ha-analysis-error = 분석 오류
ha-upload-error = 업로드 오류
ha-404 = 404 찾을 수 없음
ha-file-too-large = 파일 너무 큼: { $size }
ha-file-too-large-default = 파일 너무 큼 (>100 MB)
ha-pull-failed = 가져오기 실패: 파일을 찾을 수 없음
ha-temp-dir-error = 임시 디렉토리 오류
ha-wait-hours = { $text } (대기 { $hours }시간{ $mins }분)
ha-wait-mins = { $text } (대기 { $mins }분)
ha-wait-less-than-min = { $text } (대기 <1분)
refresh-scan = 이 패키지 다시 검사
izzyrisk-calculation = IzzyRisk 계산:
calculating-risk-scores = 위험 점수 계산 중...

# Usage Tab
usage-control = 사용량 제어
no-usage-stats = 사용량 통계가 없습니다. 사용량 통계를 보려면 장치를 선택하세요.
usage-statistics = 사용량 통계:

# Settings
display-size = 화면 크기:
color-mode = 색상 모드:
light-mode = ☀️ 라이트
auto-mode = 🌗 자동
dark-mode = 🌙 다크
contrast = 대비:
contrast-high = 높음
contrast-medium = 중간
contrast-normal = 보통
virustotal-api-key = VirusTotal(4/분) API 키:
get-api-key = API 키 발급
allow-virustotal-upload = VirusTotal 파일 업로드 허용
virustotal-upload-desc = (분석을 위해 VirusTotal에 없는 파일 업로드)
hybridanalysis-api-key = HybridAnalysis(200/분) API 키:
allow-hybridanalysis-upload = Hybrid Analysis 파일 업로드 허용
hybridanalysis-upload-desc = (검사를 위해 Hybrid Analysis에 없는 파일 업로드)
google-play-renderer = Google Play 렌더러
google-play-renderer-desc = (Google Play 스토어에서 앱 정보 가져오기 및 표시)
fdroid-renderer = F-Droid 렌더러
fdroid-renderer-desc = (비시스템 앱에 대해 F-Droid에서 앱 정보 가져오기 및 표시)
apkmirror-renderer = APKMirror 렌더러
apkmirror-renderer-desc = (APKMirror에서 앱 정보 가져오기 및 표시)
apkmirror-email = APKMirror 이메일:
email-hint = your@email.com
apkmirror-name = APKMirror 이름:
name-hint = 이름 (선택 사항)
apkmirror-auto-upload = APKMirror 자동 업로드
apkmirror-auto-upload-desc = (장치 버전이 APKMirror보다 최신인 경우 APK 자동 업로드)
invalidate-cache = 404 캐시 무효화
invalidate-cache-desc = (캐시된 앱 및 검사 결과 지우기)
show-logs = 로그 표시:
show = 표시
cancel = 취소
save = 저장
flush = 초기화

# Status & Errors
status-checking-api = API 확인 중
status-pulling-file = 파일 가져오는 중
status-uploading = 업로드 중
error-invalid-package-id = 유효하지 않은 패키지 ID (도메인 수준 2 미만)
error-timeout-rate-limit = 시간 제한 비율 도달
error-email-not-configured = 이메일이 설정되지 않음
error-rate-limit-reached = 속도 제한 도달 (429)
error-app-not-found = 앱을 찾을 수 없음

# GUI Descriptions
debloat-description = Universal Android Debloater 목록으로 안드로이드 애플리케이션 디블로트:
loading-packages = 패키지 세부 정보 로드 중...
scan-description = VirusTotal 및 HybridAnalysis로 바이러스 검사:
set-api-keys = 설정에서 VirusTotal 혹은 HybridAnalysis API 키를 설정해주세요.
apps-description = FOSS 애플리케이션 목록 :
usage-description = 애플리케이션 사용량 :
language = 언어:
font = 글꼴:
text-style = 텍스트 스타일:

# Disclaimer Dialog
disclaimer-title = ⚠ 경고
disclaimer-no-user-data = • 이 앱은 사용자 파일이나 사용자 데이터를 조작하지 않습니다.
disclaimer-uninstall-warning = • 앱 제거 시 해당 앱의 사용자 데이터도 함께 삭제됩니다. 앱 제거 시 주의하세요.

# About Dialog
about = 정보
about-description = 블로트웨어를 선택적으로 제거하고, 설치된 앱을 VirusTotal/HybridAnalysis로 검사하며, Obtainium을 통해 FOSS 앱을 설치합니다.
about-website = 웹사이트
about-credits = 크레딧

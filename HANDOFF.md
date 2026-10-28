# Jusage 인수인계

작성일: 2026-10-28
판정: **배포 보류 (HOLD)**. 대표님 승인 전에는 출시하지 않는다.

이 파일이 다음 작업의 기준이다. 다른 메모와 이 내용이 다르면 이 파일을 따른 뒤, 저장소와 Windows 상태를 다시 확인하고 고친다.

## 새 에이전트는 여기서 시작한다

1. 작업 폴더는 이 컴퓨터의 `/home/skkse12/Desktop/Projects/Core/tray-left` 이다.
2. 브랜치는 `jusage-v0.3` 이다. 브랜치 끝을 그대로 쓴다. 기능이 들어 있는 커밋은 `73f2b9d6d88c3ad730a9e5ba1047c29ec4798844` 이고, 이 문서는 그 바로 다음 커밋이다.
3. 이 파일을 먼저 읽는다.
4. 형제 폴더 `/home/skkse12/Desktop/Projects/Core/ai-agent-monitor` 는 다른 제품이다. Jusage 작업 폴더로 열지 않는다.

공개 저장소는 https://github.com/ju0o/tray-left 이다. 폴더 이름은 `tray-left` 그대로 둔다. 저장소 이름을 바꾸지 않는다.

초안은 https://github.com/ju0o/tray-left/pull/1 이다. 초안 상태이고, 기준 브랜치는 `main` 이다. 이 초안을 본선에 넣지 않는다.

## 지금 상태

받아 쓰는 파일은 여전히 0.2.0이다.

https://github.com/ju0o/tray-left/releases/tag/v0.2.0 의 `jusage.exe`

`main` 은 `3097ff4738ef141cb652e61d2af95976918488eb` 이다. 병합하지 않았고, GitHub Release 도 만들지 않았다.

0.3.0 소스만 `jusage-v0.3` 에 있다. 기능 추가는 멈춰 있다. 남은 일은 출시가 아니라, 아래의 미검증 항목이다.

## 하면 안 되는 일

- `main` 병합, 정식 Release, 태그, 공개 실행 파일 교체.
- 시계 옆에서 켜져 있는 프로그램 종료, 그 실행 파일 덮어쓰기, 탐색기 재시작.
- 점검 실행을 `--preview` 없이 Windows에서 켜기. 일반 실행은 설치 폴더로 자신을 복사하고 다시 켠다.
- 로그인 정보, 액세스 토큰, 쿠키, 계정 번호, 원문 응답을 로그나 문서나 그림에 남기기.
- 인증 파일 수정. WSL 쪽도 읽기만 한다.
- 속도, 예상 소진, 하루 기록, 최근 7일 계산을 지우는 일. 기본 창에서 숨긴 것이지 기능 삭제가 아니다.
- 다른 제품 이름, 다른 사람 이름, 다른 저작권 표기를 이 앱의 이름으로 쓰는 일. 저작권은 `Copyright (c) 2026 ju0o` 이다.
- 전역 git 설정 변경. 커밋할 때만 `git -c user.name=ju0o -c user.email=209601732+ju0o@users.noreply.github.com` 를 쓴다.

대표님 승인 없이 출시 판단만 바꾸고 GO라고 적지 않는다. 보지 않은 항목은 통과로 적지 않는다.

## 경로

| 역할 | 위치 |
| --- | --- |
| 이 컴퓨터의 소스. 여기서 작업한다. | `/home/skkse12/Desktop/Projects/Core/tray-left` |
| Windows에 설치되어 켜져 있는 프로그램 | `C:\Users\user\AppData\Local\Jusage` |
| 그 실행 파일 | `C:\Users\user\AppData\Local\Jusage\jusage.exe` |
| 점검용 소스 복사본 | `C:\Users\user\src\tray-left-qa` |
| 점검용 그림과 가짜 기록 | `C:\Users\user\src\jusage-preview` |
| 예전 옆 빌드. 실행하지 않는다. | `C:\Users\user\src\tray-left-v03` |

2026-10-28 확인 당시 설치 실행 파일 해시는 `D5B1BB0769B9433E48A271D1ED7ACB12D85F685131C8EF06D6CC29FB0F74F2B5`, 길이 1440256, 시각 `2026-10-27T18:46:34.7827300Z` 였다. 프로세스는 그 파일 하나였다. 번호는 다시 확인한다. 끄지 않는다.

설정 파일 `settings.txt` 와 `Jusage\data` 폴더는 그 시점에 없었다. 기록은 실행 파일 옆에 있다. 내용을 보고서에 옮기지 않는다. 시작 항목은 `Jusage` 만 있고 `TrayLeft` 는 없다.

Windows 작업은 SSH 호스트 `main` 으로 한다. 원격 기본 셸은 PowerShell이다. 경로는 따옴표로 감싸 역슬래시가 사라지지 않게 한다. `$pid` 와 `$ErrorActionPreference=Stop` 은 쓰지 않는다. 빌드 환경은 `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat` 와 `C:\Users\user\.cargo\bin` 이다.

SSH로 보이는 화면은 사용자 바탕 화면이 아니다. 2026-10-28에는 1024x768, DPI 96, 화면 1개로 나왔다. 이것을 사용자 모니터나 배율 검증으로 적지 않는다.

## 제품

Windows 전용 트레이 프로그램이다. Rust와 Win32만 쓴다. 이름은 Jusage, 실행 파일은 `jusage.exe`, 창 제목은 Jusage이다. 화면 문구는 한국어이다. Codex 구독 잔량만 보여 준다. 다른 AI는 범위 밖이다.

트레이에만 있고 작업 표시줄에는 올리지 않는다. 막대는 가로 막대이다. 가운데 고리는 넣지 않는다. 청록은 여유, 주황은 주의, 빨강은 거의 없음에만 쓴다. 초 단위 카운트다운, 20%와 5% 알림, 차트 라이브러리, 이 PC의 세션 파일 개수, 없는 과거 날짜는 넣지 않는다.

새 설치는 동의 없이 시작 프로그램을 켜지 않는다. 이미 있는 시작 항목은 첫 설정 파일에서 그대로 둔다. 기본 자동 새로고침은 쉬고 있을 때 1분에 한 번이다. 이 PC에서 Codex 작업이 있으면 15초 뒤 한 번 조회한다. 채워진 뒤 숫자가 그대로면 15초 간격으로 최대 다섯 번이다. 몇 초마다 조회하지 않는다.

## 0.3에서 이미 들어 있는 것

- 설정: Windows 또는 WSL, 대표 잔량 자동·첫 창·한 주, 시작 프로그램 유지·켜기·끄기, 밝기 시스템·어둡게·밝게.
- 사용자 에이전트 `jusage/0.3`. 버전 0.3.0.
- 조회는 `GET https://chatgpt.com/backend-api/wham/usage`, 제한 시간 12초. 인증 파일은 읽기만 한다.
- 기본 창 순서: 대표 잔여량, 각 창 잔여량, 채워지는 시각, 연결 상태와 신선도. 속도·예상·하루 기록·최근 7일은 `자세히` 안에 있다.
- 긴 날짜는 칸 안에서 두 줄이다. `10월 30일` 다음 줄에 시각이 온다.
- 어두운 충전 막대는 `#98A9C4`, 밝은 충전 막대는 `#536684` 이다. 상태색이 아니다.
- `--preview <폴더>` 와 `--isolated` 는 설치본과 다른 락, 창 클래스, 프로필을 쓴다. 레지스트리와 설치 폴더 복사를 하지 않는다.
- 미리보기 경로가 설치 폴더로 풀리면 폴더를 만들기 전에 멈춘다. `..`, 끝의 점, `C:Jusage` 같은 상대 경로도 풀어서 검사한다.

설치본 락은 `Local\JusageSingle`, 창 클래스는 `JusageWindow`, 프로필은 `Jusage` 이다. 점검용은 `Local\JusageIsolated`, `JusageIsolatedWindow`, `JusageIsolated` 이다.

색은 `src/look.rs` 에 있다.

- `#111522` 깊은 배경
- `#1E263B` 메인 팝업
- `#303D59` 정보 카드
- `#536684` 테두리와 보조 요소. 작은 본문에는 쓰지 않는다.
- `#98A9C4` 보조 글자. 어두운 화면의 충전 막대도 이 색이다.
- `#DCE3EF` 주요 글자

밝은 화면 글자는 `#111522`, 보조 글자는 `#3E4D68`, 판은 `#F4F7FB` 이다. 어두운 화면의 보조 글자색을 밝은 판 위에 그대로 쓰지 않는다.

주기 기록은 남은 양이 저장한 마감보다 크고, 그 주기를 열었던 값 이상일 때만 새로 시작한다.

## 2026-10-28에 실제로 한 확인

- 리눅스에서 `cargo test --offline` 42개 통과.
- `cargo check --offline --target x86_64-pc-windows-msvc` 통과.
- Windows 릴리스 빌드는 `C:\Users\user\src\tray-left-qa` 에서만 했고, 끝은 `jusage.exe --preview C:\Users\user\src\jusage-preview` 였다. 종료 코드 0.
- 그림으로 본 것: 어두운 기본, 밝은 기본, 두 창, 자세히, 설정, WSL 설정 줄, 오래됨, 오류, 150%로 키운 배치, 16px 아이콘을 4배로 본 것. 숫자는 가짜 58이다. 요금제 문구는 Pro 100, Plus 로 보였다.
- 기본 창 높이 458, 두 창 626, 자세히 약 1056. 그리는 동안 메모리 약 11MB, 최대 약 16MB, 0.4초 안쪽.
- 설치 폴더로 풀리는 미리보기 다섯 가지를 실행했다. 설치 폴더, 점이 붙은 그 폴더, 그 폴더 안의 `.`, `data\..`, 드라이브 상대 `C:Jusage`. 설정 파일과 `Jusage\data` 는 생기지 않았다. 시작 항목과 실행 파일 해시도 그대로였다.

## 통과라고 적으면 안 되는 것

- 사용자 바탕 화면에서 창을 열고 닫은 것.
- Windows 배율 자체를 100, 125, 150으로 바꾼 것. 150%는 배치 숫자만 키워 그렸다.
- 모니터 두 대로 창을 옮긴 것. 좌표 계산 시험만 있다.
- 설정을 눌러 저장한 것. 그림만 있다.
- 실제 Codex 로그인으로 잔량을 받은 것.
- 실제 WSL 배포판에서 읽은 것.
- 켜 둔 채의 CPU.
- 설치본을 0.3으로 바꾸는 업그레이드. 일부러 하지 않았다.
- 대표님 승인.

한 주 창에서 충전까지 남은 시간이 짧으면 충전 막대도 짧게 남는다. 색이 빠진 것이 아니다.

## Windows에서 다시 그릴 때

설치 폴더의 `jusage.exe` 를 덮는 스크립트를 쓰지 않는다. 예전 `C:\Users\user\src\jusage-v03-build.ps1` 는 그럴 수 있으니 실행하지 않는다.

안전한 순서는 이렇다.

1. 설치 실행 파일 해시, 프로세스, 시작 항목, `settings.txt` 유무, `Jusage\data` 유무를 적는다.
2. 소스는 `C:\Users\user\src\tray-left-qa` 에만 복사한다.
3. `C:\Users\user\src\jusage-qa-build.bat` 만 실행한다. 이 파일은 vcvars 뒤 `cargo build --release --offline` 을 하고, `--preview C:\Users\user\src\jusage-preview` 로만 끝난다.
4. 같은 항목을 다시 읽어 설치본이 그대로인지 확인한다.
5. 미리보기 없이 `jusage.exe` 를 켜지 않는다. 프로세스를 하나 더 남기지 않는다.

시험은 이 컴퓨터에서 저장소 폴더로 이동한 뒤 `cargo test --offline` 이다. 이 컴퓨터의 다른 폴더에는 `Cargo.toml` 이 없다.

## 코드

- `src/usage.rs` 계산, 조회, 요금제 이름. `prolite` 는 조회 때 `Pro 100` 이 된다.
- `src/look.rs` 색과 대비.
- `src/settings.rs` 설정, 실행 인자, 설치 폴더와 겹치는 경로 거부, 창 접기, 화면 밖 좌표.
- `src/win.rs` 창, 트레이, 그리기, 미리보기. `run` 은 미리보기 경로를 풀고 안전한 뒤에만 폴더를 만든다.
- `src/icon.rs` 트레이 숫자.
- `README.md`, `CHANGELOG.md` 사용자에게 보이는 설명.

지운 것처럼 보여도 남겨 둔 함수가 있다. `dial`, `paint_arc`, `tile_brief`, `pace_value`, `named_refill`, `refill_phrase` 를 정리한다는 이유로 삭제하지 않는다.

## 다음에 이어서 할 일

기능을 더 넣지 않는다. 대표님이 출시를 승인하기 전에는 0.2.0 파일을 바꾸지 않는다.

이어서 할 수 있는 일은 사용자 화면에서 직접 보는 검증뿐이다. 그 검증이 시계 옆 프로그램을 끄거나 바꿔야 하면 하지 말고 막힌 일로 보고한다.

보고는 한국어로, 본 것과 못 본 것을 먼저 쓴다. 배포 권고는 GO, HOLD, NO-GO 중 하나다. 지금 권고는 HOLD 이다.

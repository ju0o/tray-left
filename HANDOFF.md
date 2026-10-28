# Jusage 인수인계

작성일: 2026-10-28
판정: **배포 보류 (HOLD)**. 대표님 승인 전에는 출시하지 않는다.

이 파일이 다음 작업의 기준이다. 다른 메모와 이 내용이 다르면 이 파일을 따른 뒤, 저장소와 Windows 상태를 다시 확인하고 고친다.

## 새 에이전트는 여기서 시작한다

1. 작업 폴더는 이 컴퓨터의 `/home/skkse12/Desktop/Projects/Core/tray-left` 이다.
2. 브랜치는 `jusage-v0.3` 이다. 이번 QA 기준 커밋은 `73f2b9d6d88c3ad730a9e5ba1047c29ec4798844` 이다. 이번 동기화 시작 때 받은 로컬 후보는 `dc6ab4d88970a0d7711dcfbcb2865c62af412511`, 원격 PR HEAD는 `b2d7bb66c7340f64e365ab8b089a57e0c2522a28` 이다. 캡처 개인정보를 제외한 뒤의 최종 SHA는 `git rev-parse HEAD` 로 확인한다.
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
- 자세히 내용이 작업 영역보다 길면 팝업 높이를 제한하고 본문을 휠·방향키·PageUp/PageDown·Home/End로 스크롤한다. 닫기·설정과 접기·다시 확인·끝내기는 고정한다.
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

## 2026-10-28 이전 확인 기록

- GUI 실행 전 `cargo test --offline` 42개 통과, Windows 타깃 `cargo check` 통과.
- 오프스크린 미리보기로 테마, 두 창, 자세히, 오류·오래됨, 설정 등을 확인했다. 이 그림은 실제 Windows GUI 증거가 아니다.
- 설치 폴더로 풀리는 미리보기 경로 다섯 가지는 프로필과 시작 항목을 만들지 않았다.

## 2026-10-28 최종 GUI/E2E 재검증

- Windows Release 빌드는 `C:\Users\user\src\jusage-v03-qa-b2d7bb6-scroll` 에서 `--offline` 으로 성공했다. 실행 파일 SHA256은 `4F2E123560B3F49E4E15953B6577A1A2CCA33AB04BD5BA0D3792B1D9EB7986E2` 이다.
- `cargo test --offline` 43개 통과. `cargo fmt --all -- --check`, `cargo check --offline --target x86_64-pc-windows-msvc`, `git diff --check` 통과.
- `--preview <폴더>` 는 오프스크린 BMP 렌더링이다. 생성 이미지는 화면 배치 검토에만 썼고, 실제 창 표시·입력·DPI 검증 근거로 취급하지 않았다.
- 실제 Windows 대화형 세션 1에서 Release `--isolated` GUI를 열고 클릭했다. 단일 화면은 1920×1080, 작업 영역은 1920×1032, DPI는 96(100%)이었다. 스크롤은 끝까지, Home으로 되돌아오는 것까지 확인했다.
- 격리 GUI에서 실제 Windows와 Ubuntu WSL의 표시 결과가 각 격리 상태와 일치했다. 정확한 사용률·요금제·초기화 시각은 계정 정보라 공유 문서에서 뺐다. WSL 설정은 검증 뒤 Windows로 복구했다. 인증 파일은 존재 여부만 확인했고 내용을 읽거나 출력하지 않았다.
- 자세히 열기·접기, 휠, Page Down, End, Home, 설정 저장, 어둡게/밝게 전환, 재실행 후 다크 테마 복원, WSL↔Windows 전환, ESC, 바깥 클릭, 트레이 종료·재실행을 확인했다.
- 두 창, 긴 자세히 내용, 오류, 오래된 데이터 화면은 별도 임시 Windows Release fixture로 표시했다. 이 fixture는 네트워크 worker를 실행하지 않고 별도 QA 프로필을 썼다. 화면은 실제 GUI 캡처지만 사용량 값은 합성 데이터다.
- 공유 가능한 실제 GUI 캡처와 합성 fixture는 `qa-artifacts/windows-gui/` 에 있다. 실제 사용량·요금제·초기화 시각이 보이는 원본 캡처는 `qa-artifacts/private-local/` 에만 보관하고 커밋하지 않는다. 오프스크린 BMP도 PR 증거로 제출하지 않았다.
- 숨긴 `--isolated` 프로세스의 30초 유휴 측정은 CPU 0.078초, Working Set 24.54MB, Private Bytes 4.11MB였다.
- 현재 V0.2.0 설치본은 SHA256 `D5B1BB0769B9433E48A271D1ED7ACB12D85F685131C8EF06D6CC29FB0F74F2B5`, 크기 1,440,256 bytes, PID 26396이다. 시작 항목 `Jusage` 존재, `TrayLeft` 부재, 설치 프로필 `settings.txt` 와 `Jusage\data` 부재를 재확인했다. 설치본·기록·시작 항목은 변경하지 않았다.
- 공개 `v0.2.0` 태그는 `3097ff4738ef141cb652e61d2af95976918488eb` 이고 다운로드한 release asset은 SHA256 `5525dd81aff3d95813be20375bcb1f397efe513fc86c19a4a476e611a82e21d4`, 963,072 bytes다. 태그와 패키지 버전은 0.2.0이며, 설치본 PE의 FileVersion/ProductVersion도 0.2.0이다. 설치본에만 `.rsrc` 섹션이 있고 해시와 크기도 다르다. 태그에 재현 가능한 빌드 workflow가 없고 설치본의 빌드 기록도 없어 별도 산출물이라는 사실까지만 확인했다. 정확히 어느 소스/빌드 설정에서 달라졌는지는 **미확정**이다.

## 미검증과 출시 보류 사유

- 실제 Windows 125%·150%와 두 모니터 이동은 미검증이다. 확인 당시 디스플레이는 1920×1080 한 대였고, 설정은 바꾸지 않았다.
- 안전한 설치 QA 환경은 없다. Hyper-V VM 목록이 비어 있고, `JusageTest` 사용자가 없으며 Windows Sandbox는 비활성화돼 있다. 실행 중인 설치본 PID 26396은 그대로 뒀고, 업그레이드·시작 경로·잠금·중복 프로세스·롤백은 실행하지 않았다. 이 QA 항목은 **BLOCKED**다.
- 현재 설치본과 공개 V0.2.0 asset이 다르지만, 원인을 빌드 provenance까지 확정할 자료가 없다. 이 차이를 해소하고 격리 업그레이드와 실제 DPI/다중 모니터 검증을 마칠 때까지 출시 권고는 **HOLD**다.

한 주 창에서 충전까지 남은 시간이 짧으면 충전 막대도 짧게 남는다. 색이 빠진 것이 아니다.

## Windows에서 다시 그릴 때

설치 폴더의 `jusage.exe` 를 덮는 스크립트를 쓰지 않는다. 예전 `C:\Users\user\src\jusage-v03-build.ps1` 는 그럴 수 있으니 실행하지 않는다.

1. 설치 실행 파일 해시, 프로세스, 시작 항목, `settings.txt` 유무, `Jusage\data` 유무를 기록한다.
2. 새 Release 빌드는 격리 점검 복사본에서만 만든다. 마지막 복사본은 `C:\Users\user\src\jusage-v03-qa-b2d7bb6-scroll` 이다.
3. 실제 GUI 점검은 `jusage.exe --isolated` 로만 시작한다. 일반 실행은 설치 폴더 복사와 시작 항목 처리가 있을 수 있으므로 쓰지 않는다.
4. 종료 뒤 설치 파일 해시, 프로세스, 시작 항목, 설치 프로필을 다시 비교한다.
5. 합성 fixture는 별도 QA 프로필에서만 실행하고 실제 인증·사용량 증거와 구분한다.

## 코드

- `src/usage.rs` 계산, 조회, 요금제 이름. `prolite` 는 조회 때 `Pro 100` 이 된다.
- `src/look.rs` 색과 대비.
- `src/settings.rs` 설정, 실행 인자, 설치 폴더와 겹치는 경로 거부, 창 접기, 화면 밖 좌표, 팝업 스크롤 범위.
- `src/win.rs` 창, 트레이, 그리기, 미리보기, 작업 영역 높이 제한과 휠·키보드 스크롤.
- `src/icon.rs` 트레이 숫자.
- `README.md`, `CHANGELOG.md` 사용자에게 보이는 설명.

지운 것처럼 보여도 남겨 둔 함수가 있다. `dial`, `paint_arc`, `tile_brief`, `pace_value`, `named_refill`, `refill_phrase` 를 정리한다는 이유로 삭제하지 않는다.

## 다음에 이어서 할 일

기능을 더 넣지 않는다. 대표님이 출시를 승인하기 전에는 0.2.0 파일을 바꾸지 않는다.

다음 게이트는 재현 가능한 V0.2.0 설치 산출물을 확보한 뒤 별도 Windows 사용자/VM에서 업그레이드·롤백을 검증하는 것이다. 그 환경이 없으면 업그레이드를 실행하지 않는다. 실제 Windows 125%·150% 배율과 듀얼 모니터도 기존 설정을 바꾸지 않고 검증할 수 있을 때만 진행한다. 기존 설치본 교체, `main` 병합, Release는 하지 않는다.

보고는 한국어로, 본 것과 못 본 것을 먼저 쓴다. 배포 권고는 GO, HOLD, NO-GO 중 하나다. 지금 권고는 HOLD 이다.

## 2026-10-28 오픈소스 배포 준비 갱신

- 아이콘은 `76a67b5` 에 처음 추가됐지만 편집 원본이나 제작 기록이 없어 출처를 확정하지 못했다. 독립 제작 자산으로 교체하는 방안을 제안했고, 이번에는 기존 파일을 바꾸지 않았다.
- Windows MSVC 의존성 48개 패키지/버전의 라이선스 식과 원문 파일을 `THIRD_PARTY_NOTICES.md` 및 `THIRD_PARTY_LICENSES/` 에 기록했다. Jusage의 MIT LICENSE는 유지했다.
- `8b5c785b2c62da5353d814cfa2167f4d27075c22` 입력에서 Windows Release 재빌드를 수행했다. 자세한 도구 버전, 명령, SHA-256, 재현성 차이는 [`docs/RELEASE_PREPARATION.md`](docs/RELEASE_PREPARATION.md) 를 따른다.
- 기존 Windows GUI QA 바이너리와 독립 재빌드의 해시가 다르며 `/Brepro`를 사용한 두 독립 빌드도 서로 다르다. 바이트 단위 재현성은 **미통과**다.
- 별도 Windows 설치 QA 환경은 만들지 않았다. 업그레이드·롤백은 **BLOCKED**, 실제 125%·150% DPI와 듀얼 모니터는 **미검증**으로 유지한다.
- 출시 권고는 계속 **HOLD**다. `main` 병합, 운영 설치 교체, 공개 Release는 하지 않았다.

# Jusage V0.3 RC 준비 기록

상태: **RC_HOLD**. 저장소는 Draft PR로 유지한다. 이 문서는 실제 검증이 끝난 뒤 그 증거와 소스 커밋을 기록한다.

## 소스 변경 검토

초기 미커밋 변경 5개 파일을 보존한 채 검토했다.

| 파일 | 검토한 변경 |
| --- | --- |
| `CHANGELOG.md`, `README.md` | 불투명 배경, 본문 숫자색, 상세 화면 구획, 주기 초기화 기록, 짧은 표본 예측 안내 설명 |
| `src/settings.rs` | 긴 초기화 날짜·시간을 두 줄로 표시하는 경계 사례 |
| `src/usage.rs` | 한도 주기 초기화 전후를 일일 기록에서 분리하고, 빈 기록 안내와 짧은 표본 추정 문구를 표시 |
| `src/win.rs` | 불투명 Midnight Slate 면, 본문색 대표 숫자, 날짜 줄바꿈, 상세 간격, 조용한 새로고침 표시 |

초기화 사례는 전날 기록을 보존하면서 오늘 최저·시작값을 새 주기 값으로 바꾸는 회귀 테스트로 다룬다. 사용량 계산식을 바꾸지 않는다. 이 검토는 코드 검토이며 Windows GUI 수용 검수는 별도다.

## 아이콘

- 원본: [`assets/jusage.svg`](../assets/jusage.svg). 도형은 Jusage용으로 새로 만든 기하학적 J 모노그램과 진행 표시점이다.
- 생성기: [`tools/build_icon.py`](../tools/build_icon.py). Python 표준 라이브러리로 SVG, 투명 배경의 16·24·32·48·64·128·256px ICO 및 PNG 미리보기를 만든다.
- 재생성: `python tools/build_icon.py`.
- 제작 과정은 기존 아이콘 파일이나 타사 이미지·폰트·외부 다운로드를 사용하지 않는다. 트레이의 사용량 숫자는 기존 코드 생성 경로에 남아 있다.
- ICO 헤더·각 DIB 차원·크기를 생성기에서 자체 점검한다. PNG 미리보기는 오프스크린 자산 검토용이며 실제 Windows 트레이 캡처가 아니다.

## 라이선스와 패키지

- Jusage 코드: [`LICENSE`](../LICENSE), MIT, `Copyright (c) 2026 ju0o`.
- Windows MSVC Cargo.lock 그래프: 48개 패키지/버전(일반 의존성 45개, 추가 빌드 전용 3개). 이는 Cargo 그래프이며 런타임 DLL 수가 아니다.
- [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md)는 각 Cargo 라이선스 식과 출처를 적고, [`THIRD_PARTY_LICENSES/`](../THIRD_PARTY_LICENSES/)는 업스트림 라이선스·고지 파일 76개를 원문 그대로 보존한다.
- Jusage MIT와 각 업스트림 라이선스는 별도다. Windows RC 패키지는 `jusage.exe`, `LICENSE`, `THIRD_PARTY_NOTICES.md`, `THIRD_PARTY_LICENSES/`를 포함해야 한다.
- 이 인벤토리는 Cargo 메타데이터와 보존된 원문을 바탕으로 한 배포 준비 점검이며 법률 의견은 아니다.

## 빌드 추적성 및 재현성

최종 Windows 빌드 증거는 한 RC 소스 커밋에서 생성해야 한다. `qa/windows/run-rc.ps1`은 다음을 기록하는 수동 CI 절차다.

- 소스 커밋 SHA, Rust/Cargo와 MSVC 컴파일러·링커 버전
- `cargo test --locked` 및 `cargo build --release --locked --target x86_64-pc-windows-msvc`
- `/Brepro`가 적용된 별도 대상 폴더 두 곳의 Release SHA-256
- 오프스크린 미리보기로 실제 실행한 후보 실행 파일 SHA-256
- 패키지 ZIP SHA-256과 PE/COFF 헤더, 디버그 디렉터리, `.text`·`.rdata`·`.rsrc` 등의 섹션 해시 비교
- `.rdata`의 절대 경로 흔적은 전체 경로를 기록하지 않고 파일명·길이·해시만 남긴다. CodeView PDB 경로도 전체 경로를 남기지 않는다.

별도 대상 폴더 빌드가 바이트 단위로 다르면 CI는 진단 자료를 남기고 실패한다. 일치하지 않는 산출물을 같은 후보라고 부르지 않는다. PE 헤더, 디버그 경로, `.rdata`, `.rsrc` 차이는 한 차례 집중 비교해 기록하고, 해결되지 않으면 원인 미확정으로 남긴다.

**현재 새 RC 실행 파일의 SHA-256 및 실제 실행 파일 QA SHA-256은 아직 기록되지 않았다.** 예전 커밋의 GUI 검증 해시를 이 RC 증거로 재사용하지 않는다.

## Windows Actions 및 설치 시나리오

- [Windows RC QA workflow](../.github/workflows/windows-qa.yml)는 `windows-latest`와 수동 `workflow_dispatch`만 사용한다. 공개 V0.2.0 실행 파일의 SHA-256을 확인하고, 임시 `LOCALAPPDATA`·빈 `CODEX_HOME`·합성 설정/기록·러너 전용 시작 레지스트리를 사용한다.
- 후보의 설치 경로 self-copy, 설정/기록 보존, 시작 레지스트리, 중복 프로세스 방지, 종료 후 V0.2.0 파일 복원, 파일 잠금으로 인한 교체 거부 시나리오를 준비했다. 이는 합성 파일/레지스트리 시뮬레이션이다. 실제 제품 설치 관리자, 실행 중 바이너리 교체, 제품 자체의 롤백 처리, GUI 사용성은 검증하지 않는다.
- `--preview` 검증은 합성 데이터 오프스크린 렌더링이다. 실제 GUI 상호작용·DPI·듀얼 모니터는 이 워크플로의 PASS 범위가 아니다.
- 워크플로는 승인 없이 실행하지 않았다. GitHub 문서상 `workflow_dispatch` 실행 버튼을 사용하려면 워크플로 파일이 기본 브랜치에 있어야 한다. 현재 병합 금지로 인해 Draft 브랜치에서 이 Action을 실행할 수 없다. [GitHub 수동 워크플로 실행 안내](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)
- 업그레이드·롤백을 실제 운영 설치본에서 시험하지 않는다. 별도 VM/사용자 승인 환경도 사용하지 않았다.

## 미검증 범위와 출시 판정

- 새 RC의 Windows Release 실행 파일은 Actions를 실행하기 전까지 미검증이다.
- 실제 `--isolated` 트레이 표시·마우스/키보드 입력, 실제 125%·150% DPI, 듀얼 모니터 이동은 미검증이다.
- 실제 설치 관리자나 실행 중 업그레이드·실패 복구 절차는 존재하거나 검증된 것으로 주장하지 않는다. Actions 시나리오는 파일/레지스트리 합성 검증이다.
- V0.2.0 운영본, 사용자 설정·기록, 시작 항목은 이 RC QA에서 변경하지 않는다.

위 범위가 남아 있는 동안 권고는 **HOLD**다. Draft PR 외의 병합, 운영본 교체, 공개 Release를 하지 않는다.

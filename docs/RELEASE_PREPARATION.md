# Jusage V0.3 RC 준비 및 검증 기록

현재 판정: **HOLD**. Draft PR #1만 유지하며 `main` 병합, 공개 Release, 운영 설치 교체는 금지한다.

## 기준 소스와 검토

- Windows 검증 소스 커밋: `4191bf4c0c5a37b0b2175815d4472a223cc3f5b0`.
- 초기 미커밋 변경 5개 파일을 보존해 검토했다. Midnight Slate 면·본문 숫자·상세 레이아웃, 긴 날짜 줄바꿈, 한도 초기화 전후 기록 분리와 짧은 표본 예측 안내를 반영했고 계산식은 바꾸지 않았다.
- 추가된 회귀 사례는 긴 한국어/AM·PM 초기화 시각과 주기 초기화 전후 기록이다.

## 아이콘 출처 및 라이선스

- [`assets/jusage.svg`](../assets/jusage.svg)는 Jusage 전용 기하학적 J 모노그램과 진행 표시점으로 새로 작성했다. 기존 아이콘·타사 이미지·외부 폰트를 사용하지 않았다.
- [`tools/build_icon.py`](../tools/build_icon.py)는 Python 표준 라이브러리만으로 SVG, 투명 16·24·32·48·64·128·256px ICO와 PNG 미리보기를 만든다. `python tools/build_icon.py`로 재생성하며 ICO 헤더·이미지 크기를 자체 확인한다.
- 트레이 아이콘의 사용량 숫자 표시 경로는 기존 코드에 유지되어 있다. 아이콘 미리보기는 실제 Windows 트레이 캡처가 아니다.
- Jusage 코드는 [`LICENSE`](../LICENSE)의 MIT, `Copyright (c) 2026 ju0o`로 배포한다. Windows MSVC Cargo.lock 그래프의 48개 패키지(일반 45개, 빌드 전용 3개)에 대해 [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md)와 원문 라이선스 76개를 [`THIRD_PARTY_LICENSES/`](../THIRD_PARTY_LICENSES/)에 보존한다. 이 고지는 Jusage MIT와 별도이며 법률 의견이 아니다.
- 준비된 Windows 패키지 절차는 `jusage.exe`, `LICENSE`, `THIRD_PARTY_NOTICES.md`, `THIRD_PARTY_LICENSES/`를 묶는다. 최종 ZIP은 아직 생성·검증하지 않았다.

## Windows 빌드 및 재현성 결과

2026-10-28, 격리 임시 체크아웃에서 소스 커밋 `4191bf4c0c5a37b0b2175815d4472a223cc3f5b0`를 빌드했다. 사용자 설치 경로·설정·기록·시작 항목은 사용하지 않았다.

| 항목 | 결과 |
| --- | --- |
| Rust | `rustc 1.90.0 (1159e78c4 2025-09-14)`, LLVM 20.1.8 |
| Cargo | `cargo 1.90.0 (840b83a10 2025-07-30)` |
| Windows 환경 | Windows 11 Pro x64, build 26200; MSVC toolset 14.44.35207, linker 14.44.35226.0 |
| 단위 테스트 | Windows `cargo +1.90.0 test --locked --quiet`: 44 passed, 0 failed |
| Release 명령 | PowerShell: `$env:RUSTFLAGS = '-C link-arg=/Brepro'; cargo +1.90.0 build --release --locked --target x86_64-pc-windows-msvc --target-dir <temp>\a` (A), then repeat with `<temp>\b` (B) |
| 빌드 A SHA-256 | `b5a1df43aaaae30813200e6dadda4c993ac4a0b5fe55e1b5d09f7b842f3f3a99` |
| 빌드 B SHA-256 | `412aeb453cba88011e6e328ba6d33d8233d0b1557ebae0377c29383b7b3a87c2` |
| 실행 검증 SHA-256 | 빌드 A와 동일한 `b5a1df43aaaae30813200e6dadda4c993ac4a0b5fe55e1b5d09f7b842f3f3a99`로 `--preview` 실행 |
| 오프스크린 렌더 | 합성 BMP 19개 성공 보고; `mode=preview`, `copy_home=false`, `registry=false` 확인 |

독립 빌드 A/B는 **불일치**했다. 집중 PE 비교에서 `.text`는 동일했지만 COFF 타임스탬프(`0x573BA09F` / `0x7103DBE8`), `.rdata`(368,128바이트 중 64바이트 차이, 6 구간), `.rsrc`(373,760바이트 중 56바이트 차이)가 달랐다. CodeView 항목은 `jusage.pdb`를 가리켰고 두 빌드의 PDB 경로 해시는 같았지만 디버그 디렉터리 타임스탬프·식별 데이터는 일치하지 않았다. `/Brepro`가 적용된 빌드에서도 차이가 남았다. 이 한 차례 비교로 관찰된 차이는 기록했지만 `.rsrc` 변동의 생성 원인은 확정하지 못했다. 추가 반복 조사는 하지 않았다.

따라서 빌드 A는 실행 시험을 통과했어도 재현 가능한 배포 후보로 확정하지 않는다. A와 B 중 어느 파일도 공개 배포물로 지정하지 않는다.

검증은 비대화형 SSH에서 수행했다. `--preview` 프로세스가 끝났고 임시 빌드 경로를 사용하는 Jusage 프로세스가 0개임을 확인한 뒤 해당 임시 폴더를 삭제했다. GUI 창이나 사용자 설치 파일은 열거나 수정하지 않았다.

## 시각 증거

[`docs/evidence/v0.3-preview/`](evidence/v0.3-preview/)에는 Windows `--preview`에서 합성 데이터로 만든 PNG를 둔다. Midnight Slate 기본·두 사용량 카드·상세·설정, 라이트, 오래된 데이터, 오류 화면을 포함한다. 이 자료는 소스 SHA `4191bf4c0c5a37b0b2175815d4472a223cc3f5b0`의 디자인 검토용이며 실제 HWND/트레이 캡처나 실제 계정 자료가 아니다. 시각 검토에서 이 렌더 크기의 잘림·겹침은 보이지 않았다. 실제 DPI·모니터·입력 검증을 대체하지 않는다.

## Windows QA 및 설치 시나리오

- [`.github/workflows/windows-qa.yml`](../.github/workflows/windows-qa.yml)은 `push`를 `jusage-v0.3` 브랜치와 `.github/workflows/windows-qa.yml` 경로로 제한하고 `workflow_dispatch`를 유지한다. GitHub Actions 저장소 설정은 활성화 상태(`enabled=true`)이며 `allowed_actions=all`, SHA pinning-required는 꺼져 있다. 실제 사용하는 checkout/upload actions는 SHA로 고정되어 있다.
- Workflow `GITHUB_TOKEN`은 `permissions: contents: read`로 제한되어 있고, workflow/QA 스크립트는 GitHub Secret이나 실제 Codex 인증 변수를 참조하지 않는다. 로컬 `gh` 인증 토큰은 `repo`/`workflow` 권한이 있으나 설정 조회에만 사용했고 실행·수정에는 쓰지 않았다.
- Artifact 대상은 `qa-results.txt`, PE/설치 결과, 패키지와 라이선스 고지다. `--preview`는 합성 데이터를 쓰고 설치 fixture는 임시 경로·빈 `CODEX_HOME`·러너의 테스트용 레지스트리 값만 사용한다. 사용자 설정·기록·토큰 파일을 artifact에 포함하는 경로는 확인되지 않았다.
- Workflow는 고정 Rust 도구체인, Windows Release A/B 빌드, 단위 테스트, PE 비교, 합성 `--preview`, 공개 V0.2.0 바이너리 해시 확인, 임시 프로필 기반 업그레이드·설정/기록 보존·시작 레지스트리·중복 프로세스·롤백, 라이선스 포함 패키징을 준비한다. 자동 배포·Release·운영 PC 변경 단계는 없다.
- `qa-results.txt`는 Unit Test, Windows Release Build, Synthetic Preview, Installation Fixture, Rollback Fixture, Package ZIP, Binary Reproducibility를 개별 상태로 기록한다. 재현성이 FAIL이어도 다른 단계가 실행되면 상태·해시·패키지를 artifact에 올린 뒤 마지막 workflow gate는 실패하도록 유지한다.
- `workflow_dispatch`에는 기본 브랜치에 workflow가 있어야 한다는 제약이 있지만, push trigger는 해당 workflow 파일 변경이 대상 branch에 push될 때 동작한다. [수동 워크플로 실행 요건](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)
- 설치 fixture는 GitHub runner에서만 실행한다. 기존 사용자 설치·HKCU 시작 레지스트리를 사용하거나 변경하는 로컬 실행은 하지 않는다.

## 미검증 범위와 판정

- 배포 후보 파일과 테스트 대상 파일의 동일성은 A 단독 실행에 대해서만 확인했다. A/B 재현성이 실패했으므로 배포용 단일 RC는 **NOT_ESTABLISHED**.
- 각 Windows Actions 실행의 `qa-results.txt`, `build-manifest.txt`, package ZIP을 해당 run artifact에서 확인하고 이전 로컬 검증 결과와 구분한다.
- 실제 트레이 GUI 상호작용, 125%·150% Windows DPI, 듀얼 모니터, 실제 Codex 계정은 미검증. 이번 작업은 창이나 콘솔을 열지 않았다.
- 기존 V0.2.0 운영본·기록·설정·시작 항목은 변경하지 않았다.

재현성 실패와 남은 설치·GUI gate 때문에 V0.3 권고는 **HOLD**다. `main` 병합, 운영본 교체, 공개 Release는 하지 않는다.

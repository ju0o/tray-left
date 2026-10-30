# Jusage V0.3 RC 준비 및 검증 기록

현재 판정: **HOLD**. Draft PR #1만 유지하며 `main` 병합, 공개 Release, 운영 설치 교체는 금지한다.

## 최신 Windows CI 결과

검증한 소스 커밋은 `81b956b6b650b451aa12dddf4361b5d52b2bcea3`이다. [Run 40727390218](https://github.com/ju0o/tray-left/actions/runs/40727390218)은 모든 기능·설치·패키지 검사를 통과했지만 A/B 바이너리 재현성 gate가 FAIL이라 전체 상태는 실패이며 RC는 **HOLD**다. 이 소스 SHA의 push Run은 1회였다.

| 검사 | Run 40727390218 |
| --- | --- |
| Windows Unit Test | PASS, 47/47 |
| Release Build A/B | PASS; 서로 다른 바이너리 생성 |
| Preview 거부 코드 | PASS: 22, 36, 37; 금지 경로와 차단 파일 유지 |
| 정상 Preview | PASS: 장면 BMP 19개, 트레이 BMP 3개, `measure.txt` |
| 격리 Installation Fixture | PASS; 공개 V0.2.0 프로세스는 실행하지 않음 |
| Rollback Fixture | PASS; 이전 V0.2.0 바이너리와 설정·기록 해시 복원 확인 |
| Locked-file Guard | PASS; 열린 파일 교체 거부 후 기준 SHA 유지 |
| QA ZIP / 라이선스 | PASS; ZIP 내 실행 파일 및 76개 라이선스 파일 확인 |
| Binary Reproducibility | FAIL; 최종 workflow gate에서 HOLD 유지 |

이전 Run 40713632616은 종료 요청 뒤 프로세스 종료 여부와 파일 잠금 해제를 확인하지 않았고, 롤백 복사에서 Windows의 “다른 프로세스가 사용 중” 오류가 났다. 그 Run에는 잠금 소유 PID가 기록되지 않아 당시의 정확한 PID는 사후 확인할 수 없다. 이번 실행은 격리 설치 경로의 QA PID `2696`와 실행 파일 경로를 기록하고, 그 PID의 종료를 확인했다. 같은 경로의 다른 프로세스가 없는지 다시 조회한 뒤 읽기 전용 독점 잠금 probe가 첫 시도에 통과해 롤백이 성공했다. 따라서 수정된 안전 절차의 동작은 확인됐지만, 이전 Windows 핸들을 소유한 주체가 PID `2696`이었다고 단정하지 않는다.

fixture의 V0.2.0 baseline은 파일로만 준비했고 실행하지 않았다. 이 검사는 격리 경로의 self-copy·설정/기록 보존·롤백을 검증하며, 실행 중인 사용자 V0.2.0을 교체한 결과가 아니다. 사용자가 수행할 V0.3 수동 업데이트와 복구 절차는 [README](../README.md)에 있다. `settle_home()`은 자동 업데이터가 아니며, 복사 실패를 표시하지 않을 수 있다.

| 산출물 | SHA-256 |
| --- | --- |
| Release A / Preview 테스트 바이너리 | `2df20d2a002190999864024cce755fd6f195426be2eff1c264aaf5d1743a984a` |
| Release B | `0664923cdec77fe5ce24c4aaf6b1b35b7cd63d2813533f4379d6969a217dea47` |
| 패키지 staging 바이너리 / ZIP에서 추출한 바이너리 | `2df20d2a002190999864024cce755fd6f195426be2eff1c264aaf5d1743a984a` |
| QA ZIP | `4ef9d49b8959b76e61f45c31d8f16ddf41636f3bfaa8d72a09c947a03900f59b` |

PE 비교에서 `.text`, `.pdata`, `.reloc`은 같고 `.rdata`, `.rsrc`, COFF 타임스탬프와 디버그 정보가 달랐다. 재현성 불일치 원인은 아직 확정되지 않았다. [Run artifact](https://github.com/ju0o/tray-left/actions/runs/40727390218/artifacts/12833746606)에 개별 상태, A/B 해시, Preview 결과, 종료 PID와 잠금 진단, QA ZIP이 보존되어 있다.

이번 수정은 `qa/windows/verify-install-fixture.ps1`, `qa/windows/run-rc.ps1`, README의 수동 업데이트 절차와 이 기록에 한정했다. 사용자 운영본·기록·시작 항목, `main`, 공개 Release는 변경하지 않았다.

## CI Preview 침묵 실패 수정 후보

2026-10-28 기준, 이 수정 후보는 기준 커밋 `c5da0927428672636ff7c997397f6a80d2abbb63` 위에서 준비했다. 아래의 미실행 항목은 승인된 2차 CI 재시도 직전 상태를 기록한다.

- [실패 Run 40698329087](https://github.com/ju0o/tray-left/actions/runs/40698329087)은 Unit Test 44/44와 A/B Windows Release 빌드가 통과했지만 PE 비교상 재현성은 FAIL이었다. Preview 프로세스 종료 코드는 0이었지만 `measure.txt`가 없어 실패했고, 설치 fixture·롤백 fixture·ZIP 패키징은 `NOT_RUN`이었다.
- 해당 Run의 내부 실패 단계를 재구성할 로그가 없다. 이전 코드는 설치 프로필 경로 검사와 디렉터리 생성 실패 때 조용히 반환했고, BMP·`measure.txt` 및 합성 이력 쓰기 오류를 무시했다. 따라서 그 Run이 경로 거부, 디렉터리 생성, GDI/BMP 렌더 또는 파일 쓰기 중 어디서 멈췄는지는 **확정 불가**다.
- 수정 후보는 Preview 경로 검사, reparse 해석, 격리 프로필 설정, 디렉터리 생성, GDI 단계, BMP, 합성 이력, `measure.txt` 쓰기마다 비정상 종료 코드와 단계를 돌려준다. 설치 프로필 경로 차단은 유지하며, 거부 검사는 디렉터리 생성 전에 수행한다.
- `run-rc.ps1`은 Release A SHA를 A 빌드 직후, B SHA와 비교 결과를 B 빌드 직후 `build-evidence.txt`에 저장한다. 따라서 이후 Preview가 실패해도 소스 커밋, 러너·컴파일러 환경, 옵션, 앞선 빌드 해시가 artifact에 남는다. 전체 바이너리 해시 둘 다 없으면 재현성은 `NOT_EVALUATED`; 실제 해시가 다르면 `FAIL`을 유지한다.
- Preview 실패 뒤 시작하지 않은 설치·롤백·패키지 검증은 `NOT_RUN`으로 유지한다. 워크플로의 `push.paths`는 워크플로 파일, Cargo/build 설정, 소스, Windows QA, 자산 변경을 포함하도록 로컬에서 수정했다.

| 로컬 검증 | 결과 |
| --- | --- |
| `cargo test --locked` | 44 passed, 0 failed (Linux 실행) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --all-targets --locked` | PASS; 기존 경고 출력 있음 |
| Windows MSVC target `cargo check` | PASS (컴파일 검사만) |
| Windows 전용 테스트 cross-target compile check | PASS (실행 아님) |
| YAML parse, `git diff --check` | PASS |
| Windows Release link 및 Preview E2E | 미실행: 이 작업 환경에 MSVC linker와 Windows runtime이 없음 |
| PowerShell parser | 미실행: `pwsh` 미설치 |

실제 Windows Release 빌드, 정상·거부·쓰기 실패 Preview E2E, artifact 보존 여부는 승인된 Windows Actions 재시도에서 확인한다. 사용자 PC와 운영 Jusage는 이번 수정·검증에서 실행하거나 변경하지 않았다.

## 기준 소스와 검토

- Windows 검증 소스 커밋: `4191bf4c0c5a37b0b2175815d4472a223cc3f5b0`.
- 초기 미커밋 변경 5개 파일을 보존해 검토했다. Midnight Slate 면·본문 숫자·상세 레이아웃, 긴 날짜 줄바꿈, 한도 초기화 전후 기록 분리와 짧은 표본 예측 안내를 반영했고 계산식은 바꾸지 않았다.
- 추가된 회귀 사례는 긴 한국어/AM·PM 초기화 시각과 주기 초기화 전후 기록이다.

## 아이콘 출처 및 라이선스

- [`assets/jusage.svg`](../assets/jusage.svg)는 Jusage 전용 기하학적 J 모노그램과 진행 표시점으로 새로 작성했다. 기존 아이콘·타사 이미지·외부 폰트를 사용하지 않았다.
- [`tools/build_icon.py`](../tools/build_icon.py)는 Python 표준 라이브러리만으로 SVG, 투명 16·24·32·48·64·128·256px ICO와 PNG 미리보기를 만든다. `python tools/build_icon.py`로 재생성하며 ICO 헤더·이미지 크기를 자체 확인한다.
- 트레이 아이콘의 사용량 숫자 표시 경로는 기존 코드에 유지되어 있다. 아이콘 미리보기는 실제 Windows 트레이 캡처가 아니다.
- Jusage 코드는 [`LICENSE`](../LICENSE)의 MIT, `Copyright (c) 2026 ju0o`로 배포한다. Windows MSVC Cargo.lock 그래프의 48개 패키지(일반 45개, 빌드 전용 3개)에 대해 [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md)와 원문 라이선스 76개를 [`THIRD_PARTY_LICENSES/`](../THIRD_PARTY_LICENSES/)에 보존한다. 이 고지는 Jusage MIT와 별도이며 법률 의견이 아니다.
- 앞선 로컬 준비 시점에는 최종 ZIP을 만들지 않았다. 최신 Windows CI에서는 QA ZIP과 내부 LICENSE·고지 파일을 생성·검증했지만 공개 배포하지 않았다.

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

- [`.github/workflows/windows-qa.yml`](../.github/workflows/windows-qa.yml)의 재시도 후보는 `push`를 `jusage-v0.3` 브랜치와 워크플로·Cargo/build 설정·`src/**`·`qa/windows/**`·`assets/**` 경로로 제한하고 `workflow_dispatch`를 유지한다. GitHub Actions 저장소 설정은 확인된 시점에 활성(`enabled=true`)이었으며, `allowed_actions=all`, SHA pinning-required는 꺼져 있었다. 실제 사용하는 checkout/upload actions는 SHA로 고정되어 있다.
- Workflow `GITHUB_TOKEN`은 `permissions: contents: read`로 제한되어 있고, workflow/QA 스크립트는 GitHub Secret이나 실제 Codex 인증 변수를 참조하지 않는다. 로컬 `gh` 인증 토큰은 `repo`/`workflow` 권한이 있으나 설정 조회에만 사용했고 실행·수정에는 쓰지 않았다.
- Artifact 대상은 `qa-results.txt`, PE/설치 결과, 패키지와 라이선스 고지다. `--preview`는 합성 데이터를 쓰고 설치 fixture는 임시 경로·빈 `CODEX_HOME`·러너의 테스트용 레지스트리 값만 사용한다. 사용자 설정·기록·토큰 파일을 artifact에 포함하는 경로는 확인되지 않았다.
- Workflow는 고정 Rust 도구체인, Windows Release A/B 빌드, 단위 테스트, PE 비교, 합성 `--preview`, 공개 V0.2.0 바이너리 해시 확인, 임시 프로필 기반 업그레이드·설정/기록 보존·시작 레지스트리·중복 프로세스·롤백, 라이선스 포함 패키징을 준비한다. 자동 배포·Release·운영 PC 변경 단계는 없다.
- `qa-results.txt`는 Unit Test, Windows Release Build, Synthetic Preview, Installation Fixture, Rollback Fixture, Locked-file Guard, Package ZIP, Binary Reproducibility를 개별 상태로 기록한다. 미실행 QA는 `NOT_RUN`; 두 해시를 확보하지 못한 재현성은 `NOT_EVALUATED`다. 실제 해시 불일치 시 `FAIL`과 HOLD를 유지한다. 빌드 근거는 Preview 전에 `build-evidence.txt`에 저장한다.
- `workflow_dispatch`에는 기본 브랜치에 workflow가 있어야 한다는 제약이 있지만, push trigger는 지정 브랜치에서 설정된 경로가 바뀔 때 동작한다. [수동 워크플로 실행 요건](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)
- 설치 fixture는 GitHub runner에서만 실행한다. 기존 사용자 설치·HKCU 시작 레지스트리를 사용하거나 변경하는 로컬 실행은 하지 않는다.

## 미검증 범위와 판정

- Run 40727390218에서 Preview 테스트 실행 파일, 패키지 staging 파일, ZIP 추출 실행 파일의 SHA-256은 모두 `2df20d2a002190999864024cce755fd6f195426be2eff1c264aaf5d1743a984a`로 일치했다. 독립 Release B는 달라서 재현 가능한 단일 RC는 **NOT_ESTABLISHED**.
- 격리 Installation Fixture는 파일 baseline에 대한 self-copy와 상태 보존을 확인했으며, 실행 중인 실제 V0.2.0 설치본을 종료·교체하는 업그레이드 시험은 아니다. 수동 절차는 README를 따른다.
- 실제 트레이 GUI 상호작용, 125%·150% Windows DPI, 듀얼 모니터, 실제 Codex 계정은 이번 CI에서 미검증. 사용자 운영 V0.2.0·기록·설정·시작 항목은 변경하지 않았다.
- A/B 재현성 실패와 실제 GUI gate 때문에 V0.3 권고는 계속 **HOLD**다. `main` 병합, 운영본 교체, 공개 Release는 하지 않는다.

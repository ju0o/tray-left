# Jusage V0.3 RC 인수인계

상태: **RC_HOLD**. Draft PR #1은 열린 초안이며 `main` 병합, 공개 Release, 운영 설치 교체를 하지 않는다.

- 저장소: https://github.com/ju0o/tray-left
- Draft PR: https://github.com/ju0o/tray-left/pull/1
- 작업 브랜치: `jusage-v0.3`

## 진행 규칙

- V0.3 새 기능을 추가하지 않는다. 승인된 수정, 검증, 배포 준비만 한다.
- 기존 사용자 설치본, 기록, 설정, 시작 항목을 읽거나 바꾸는 검증은 하지 않는다. 설치 동작 QA는 Actions의 합성 임시 프로필 안에서만 준비한다.
- Windows Actions의 CI 재시도 후보는 `jusage-v0.3`에서 워크플로, Cargo/build 설정, `src/**`, `qa/windows/**`, `assets/**` 변경을 감지하며 `workflow_dispatch`도 제공한다. 아래 기록은 승인된 재시도 직전 기준을 나타낸다.
- `--preview` 결과는 오프스크린 합성 렌더링이다. 실제 트레이 GUI, 입력, DPI, 다중 모니터 검증으로 보고하지 않는다.
- 로그인 토큰이나 계정 자료를 로그·스크린샷·저장소·Actions에 넣지 않는다.
- 테스트를 위해 사용자가 쓰는 설치본을 종료하거나 실행 파일을 덮어쓰지 않는다.

## 최신 증거

- 재시도 전 로컬 변경은 기준 커밋 `c5da0927428672636ff7c997397f6a80d2abbb63` 위의 수정 후보였다. [실패 Run 40698329087](https://github.com/ju0o/tray-left/actions/runs/40698329087)에서는 Unit Test 44/44와 A/B Release 빌드가 통과했지만 PE 비교상 재현성은 FAIL이었다. Preview 프로세스는 종료 코드 0을 반환했지만 `measure.txt`가 없어 실패했고, 설치·롤백·ZIP QA는 실행되지 않았다.
- 이전 실행은 `build-evidence.txt`를 Preview와 패키징 이후에만 쓰도록 되어 있어 artifact에 Release A/B 전체 SHA-256이 남지 않았다. 현재 후보는 각 빌드 직후 SHA를 기록하며, Preview 실패 시 설치·롤백·패키지는 `NOT_RUN`, 두 해시가 없을 때 재현성은 `NOT_EVALUATED`로 남긴다.
- 실패 Run의 정확한 내부 단계는 당시 조기 `return`과 파일 쓰기 오류 무시 때문에 사후 확정할 수 없다. 현재 코드는 경로 검사, 디렉터리 생성, GDI 렌더, BMP·측정 파일·합성 기록 쓰기의 단계별 오류 코드를 반환한다. 설치 프로필 경로 거부 검증은 그대로 유지한다.
- 로컬 검증: `cargo test --locked` 44/44, `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked`, Windows MSVC 대상 `cargo check` 및 Windows 전용 테스트의 cross-target compile check, YAML parse, `git diff --check` 통과. 실제 Windows 링크·Preview E2E와 PowerShell parser 검사는 이 환경에서 실행하지 않았다.
- 위 기록은 Push와 Actions 재실행 전의 상태다. 승인된 2차 CI 실행 결과는 해당 Actions Run과 최종 보고에서 확인한다.
- 과거 별도 Windows 검증 소스: `4191bf4c0c5a37b0b2175815d4472a223cc3f5b0`; Windows 단위 테스트 44/44 통과.
- 그 검증의 Release A `b5a1df43aaaae30813200e6dadda4c993ac4a0b5fe55e1b5d09f7b842f3f3a99`는 `--preview`로 실행해 확인했다. 독립 Release B `412aeb453cba88011e6e328ba6d33d8233d0b1557ebae0377c29383b7b3a87c2`와 달라, 어느 쪽도 재현 가능한 배포 후보로 승인하지 않는다.
- 그 PE 비교에서 `.text`는 같고 COFF 타임스탬프, 디버그 식별 데이터, `.rdata`, `.rsrc`가 달랐다. 원인과 해결은 확정되지 않았다. 세부 값과 범위는 [`docs/RELEASE_PREPARATION.md`](docs/RELEASE_PREPARATION.md)에 있다.
- 시각 자료 [`docs/evidence/v0.3-preview/`](docs/evidence/v0.3-preview/)는 합성 데이터의 오프스크린 렌더다. 실제 Windows GUI/DPI/모니터 입력 검증은 아니다.
- GitHub Actions 저장소 설정은 활성화되어 있다. 현재 워크플로의 `GITHUB_TOKEN` 권한은 `contents: read`이며 외부 Secret/Codex 인증 참조는 없다.
- QA artifact는 `qa-results.txt`에서 Unit Test, Release Build, Synthetic Preview, Installation Fixture, Rollback Fixture, Package ZIP, Binary Reproducibility를 별도 기록한다. 빌드 커밋·환경·옵션·A/B SHA는 빌드 직후 별도 파일에 쓴다. 재현성은 A/B 해시가 모두 있을 때만 평가하고, 명시적 FAIL이면 workflow gate를 실패시킨다.
- GitHub-hosted Windows runner에서만 합성 설치 fixture와 package ZIP을 만들도록 구성했다. 운영 PC의 Jusage, 설정, 기록은 입력으로 사용하지 않는다.
- Windows Release/미리보기 동안 기존 운영 설치·기록·시작 항목은 수정하지 않았고 GUI나 콘솔 창을 열지 않았다.

## RC 준비 항목

- 소스와 테스트의 현재 상태 및 검증 결과: [`docs/RELEASE_PREPARATION.md`](docs/RELEASE_PREPARATION.md)
- 아이콘 생성 원본과 절차: [`assets/jusage.svg`](assets/jusage.svg), [`tools/build_icon.py`](tools/build_icon.py)
- Windows 수동 QA: [`.github/workflows/windows-qa.yml`](.github/workflows/windows-qa.yml)
- 패키지에는 Jusage `LICENSE`, `THIRD_PARTY_NOTICES.md`, `THIRD_PARTY_LICENSES/`가 포함되어야 한다.

다음 작업자는 먼저 `git status --short --branch`, `git rev-parse HEAD`, 원격 브랜치 SHA를 확인한다. 재현성 문제가 해결되고 적절한 격리 환경/허용된 Action 실행이 확보되기 전에는 RC를 승격하거나 배포하지 않는다.

# Jusage V0.3 RC 인수인계

상태: **RC_HOLD**. Draft PR #1은 열린 초안이며 `main` 병합, 공개 Release, 운영 설치 교체를 하지 않는다.

- 저장소: https://github.com/ju0o/tray-left
- Draft PR: https://github.com/ju0o/tray-left/pull/1
- 작업 브랜치: `jusage-v0.3`

## 진행 규칙

- V0.3 새 기능을 추가하지 않는다. 승인된 수정, 검증, 배포 준비만 한다.
- 기존 사용자 설치본, 기록, 설정, 시작 항목을 읽거나 바꾸는 검증은 하지 않는다. 설치 동작 QA는 Actions의 합성 임시 프로필 안에서만 준비한다.
- Windows Actions는 `workflow_dispatch` 전용이며 자동 실행하지 않는다. GitHub는 수동 실행을 위해 워크플로 파일이 기본 브랜치에 있어야 한다고 요구한다. `main` 병합이 금지된 동안에는 이 워크플로를 실행할 수 없다.
- `--preview` 결과는 오프스크린 합성 렌더링이다. 실제 트레이 GUI, 입력, DPI, 다중 모니터 검증으로 보고하지 않는다.
- 로그인 토큰이나 계정 자료를 로그·스크린샷·저장소·Actions에 넣지 않는다.
- 테스트를 위해 사용자가 쓰는 설치본을 종료하거나 실행 파일을 덮어쓰지 않는다.

## 최신 증거

- Windows 검증 소스: `4191bf4c0c5a37b0b2175815d4472a223cc3f5b0`; Windows 단위 테스트 44/44 통과.
- Release A `b5a1df43aaaae30813200e6dadda4c993ac4a0b5fe55e1b5d09f7b842f3f3a99`는 `--preview`로 실행해 확인했다. 독립 Release B는 `412aeb453cba88011e6e328ba6d33d8233d0b1557ebae0377c29383b7b3a87c2`로 달라, 어느 쪽도 재현 가능한 배포 후보로 승인하지 않는다.
- PE 비교에서 `.text`는 같고 COFF 타임스탬프, 디버그 식별 데이터, `.rdata`, `.rsrc`가 달랐다. 원인과 해결은 확정되지 않았다. 세부 값과 범위는 [`docs/RELEASE_PREPARATION.md`](docs/RELEASE_PREPARATION.md)에 있다.
- 시각 자료 [`docs/evidence/v0.3-preview/`](docs/evidence/v0.3-preview/)는 합성 데이터의 오프스크린 렌더다. 실제 Windows GUI/DPI/모니터 입력 검증은 아니다.
- Windows Actions는 준비만 했고 미실행이다. 현재 Draft 브랜치에서는 기본 브랜치 조건 때문에 수동 실행할 수 없다. 최종 패키지와 합성 설치 fixture도 미실행이다.
- Windows Release/미리보기 동안 기존 운영 설치·기록·시작 항목은 수정하지 않았고 GUI나 콘솔 창을 열지 않았다.

## RC 준비 항목

- 소스와 테스트의 현재 상태 및 검증 결과: [`docs/RELEASE_PREPARATION.md`](docs/RELEASE_PREPARATION.md)
- 아이콘 생성 원본과 절차: [`assets/jusage.svg`](assets/jusage.svg), [`tools/build_icon.py`](tools/build_icon.py)
- Windows 수동 QA: [`.github/workflows/windows-qa.yml`](.github/workflows/windows-qa.yml)
- 패키지에는 Jusage `LICENSE`, `THIRD_PARTY_NOTICES.md`, `THIRD_PARTY_LICENSES/`가 포함되어야 한다.

다음 작업자는 먼저 `git status --short --branch`, `git rev-parse HEAD`, 원격 브랜치 SHA를 확인한다. 재현성 문제가 해결되고 적절한 격리 환경/허용된 Action 실행이 확보되기 전에는 RC를 승격하거나 배포하지 않는다.

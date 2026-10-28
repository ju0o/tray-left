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

## RC 준비 항목

- 소스와 테스트의 현재 상태 및 검증 결과: [`docs/RELEASE_PREPARATION.md`](docs/RELEASE_PREPARATION.md)
- 아이콘 생성 원본과 절차: [`assets/jusage.svg`](assets/jusage.svg), [`tools/build_icon.py`](tools/build_icon.py)
- Windows 수동 QA: [`.github/workflows/windows-qa.yml`](.github/workflows/windows-qa.yml)
- 패키지에는 Jusage `LICENSE`, `THIRD_PARTY_NOTICES.md`, `THIRD_PARTY_LICENSES/`가 포함되어야 한다.

다음 작업자는 먼저 `git status --short --branch`, `git rev-parse HEAD`, 원격 브랜치 SHA와 위 출시 준비 기록을 다시 확인한다. 기록이 현재 저장소 상태와 다르면 증거를 새로 수집하고 문서를 갱신한다.

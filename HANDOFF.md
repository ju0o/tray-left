# Jusage V0.3 RC 인수인계

상태: **RC_HOLD**. Draft PR #1을 유지하며 `main` 병합, 공개 Release, 운영 설치 교체는 금지한다.

- 저장소: https://github.com/ju0o/tray-left
- Draft PR: https://github.com/ju0o/tray-left/pull/1
- 작업 브랜치: `jusage-v0.3`
- 검증 소스 커밋: `81b956b6b650b451aa12dddf4361b5d52b2bcea3`
- Windows CI: [Run 40727390218](https://github.com/ju0o/tray-left/actions/runs/40727390218)
- QA artifact: [jusage-v0.3.0-rc-qa](https://github.com/ju0o/tray-left/actions/runs/40727390218/artifacts/12833746606)

## 최신 QA

| 항목 | 결과 |
| --- | --- |
| Windows Unit Test | PASS, 47/47 |
| Release Build A/B | 빌드 PASS, 해시 불일치 |
| Preview 거부 3종 | PASS: 22, 36, 37 |
| 정상 Preview | PASS: 화면 BMP 19개, 트레이 BMP 3개, `measure.txt` |
| 격리 Installation Fixture | PASS; 실행 중 V0.2.0 프로세스 교체는 미검증 |
| Rollback Fixture | PASS |
| Locked-file Guard | PASS |
| QA ZIP 및 라이선스 | PASS, ZIP 안 타사 라이선스 76개 |
| Binary Reproducibility | FAIL; workflow가 RC를 HOLD로 유지 |

격리 fixture는 QA PID `2696`와 설치 경로를 기록해 해당 프로세스만 종료했고, 종료 재조회 및 읽기 전용 잠금 probe가 첫 시도에 통과한 뒤 V0.2.0 파일·합성 설정·기록을 복원했다. 이전 Run 40713632616에는 잠금 소유 PID가 기록되지 않아 그 Run의 실제 잠금 주체는 확정할 수 없다. 새 fixture는 `settle_home()`의 self-copy 흐름을 검증하지만 실행 중인 사용자 V0.2.0 설치본을 교체하지 않았다.

Release A, Preview 테스트, staging 패키지, ZIP 추출 실행 파일 SHA-256은 모두 `2df20d2a002190999864024cce755fd6f195426be2eff1c264aaf5d1743a984a`다. Release B SHA-256은 `0664923cdec77fe5ce24c4aaf6b1b35b7cd63d2813533f4379d6969a217dea47`로 A와 다르다. QA ZIP SHA-256은 `4ef9d49b8959b76e61f45c31d8f16ddf41636f3bfaa8d72a09c947a03900f59b`다. 상세 비교와 빌드 환경은 [`docs/RELEASE_PREPARATION.md`](docs/RELEASE_PREPARATION.md)의 최신 CI 섹션을 본다.

## 운영과 검증 경계

- 실제 수동 업데이트와 복구 절차는 [README](README.md)에 있다. `settle_home()`은 자동 업데이터가 아니며 복사 실패를 표시하지 않을 수 있다.
- Actions는 GitHub-hosted Windows runner에서 실행했고, 실제 Codex 인증값·사용자 기록은 사용하지 않았다. Workflow 토큰 권한은 `contents: read`다.
- 기존 사용자 PC의 Jusage V0.2.0, 설정, 기록, 시작 항목은 건드리지 않았다.
- 실제 트레이 GUI, 실제 Codex 계정, 125%·150% DPI, 듀얼 모니터 검증은 이번 Run에 포함되지 않았다.
- 이 SHA에서 Windows Actions는 1회 실행했다. 실패 후 자동 재실행하지 않았다.

## 다음 출시 gate

1. Release A/B 바이트 재현성 차이의 원인을 해결하거나 승인된 별도 기준으로 검토한다.
2. 실제 설치 업그레이드와 GUI/DPI/듀얼 모니터는 사용자 운영본과 분리된 환경에서 검증한다.
3. 새 CI 실행이 필요하면 별도 실행 승인을 받는다. `main` 병합, 운영본 교체, 공개 Release는 하지 않는다.

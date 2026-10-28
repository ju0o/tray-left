# Jusage V0.3 오픈소스 배포 준비 기록

기준일: 2026-10-28
판정: **HOLD** — 아이콘 출처, 바이트 단위 빌드 재현성, 격리 설치 QA, 실제 125%·150% DPI와 듀얼 모니터 검증이 남아 있다.

## 저장소와 실행 파일 기준

- 소스 기준 커밋: `8b5c785b2c62da5353d814cfa2167f4d27075c22` (`jusage-v0.3`, 기존 Draft PR #1의 소스 HEAD).
- 이 기록의 문서 커밋은 실행 파일 소스를 바꾸지 않는다. 빌드 산출물은 위 소스 커밋의 입력으로 구분한다.
- 2026-10-28 확인 당시 로컬 작업트리에는 `CHANGELOG.md`, `README.md`, `src/settings.rs`, `src/usage.rs`, `src/win.rs` 수정이 있었다. 이 수정들은 이 기록의 빌드 입력이나 문서 커밋에 포함하지 않았고, 덮어쓰지 않았다.
- 기존 사용 설치본과 공개 V0.2.0 파일은 서로 다른 비교 기준이다. HANDOFF에 기록된 값은 설치본 `D5B1BB0769B9433E48A271D1ED7ACB12D85F685131C8EF06D6CC29FB0F74F2B5` (1,440,256 bytes), 공개 `v0.2.0` asset `5525dd81aff3d95813be20375bcb1f397efe513fc86c19a4a476e611a82e21d4` (963,072 bytes)다. 이번 빌드 검증에서 두 파일을 변경하지 않았다.

## 아이콘 출처

- 배포 자산은 `assets/jusage.ico` 하나다. 현재 파일 SHA-256: `9b3669e89fdb618e4ab7c4d5f361ec263f178e0b090cce25e700d53f8574d6b8`.
- Git 이력상 `76a67b5e29a162ca198d6821f2aed960debc4850`에서 처음 추가됐다. 이 커밋에는 편집 가능한 원본, 제작 도구, 제작 기록, 출처 표기가 없다.
- 커밋 작성자가 ju0o라는 사실만으로 그림을 직접 제작했다고 확정할 수 없다. 따라서 현재 아이콘의 제작자와 이용 허락은 **확인되지 않음**이다.
- 배포 전 제안: 기존 이미지를 바탕으로 삼지 않는 독립 도형으로 아이콘을 새로 만들고, 편집 가능한 원본과 제작 과정을 함께 커밋한다. 원본과 바이너리의 대응 및 제3자 자산 미사용을 검토한 뒤 이 자산을 교체한다. 이번 준비 작업에서는 아이콘을 바꾸지 않았다.

## 라이선스 고지

- Jusage 코드의 프로젝트 라이선스는 [`LICENSE`](../LICENSE)의 MIT이며 저작권 표기는 `Copyright (c) 2026 ju0o`다. 해당 파일은 수정하지 않았다.
- Windows MSVC 대상 Cargo.lock 그래프는 48개 패키지/버전이다. `normal` edge 그래프 45개와 추가 빌드 전용 항목 3개(`serde`, `toml`, `winres`)를 포함한다. 프로시저 매크로와 빌드 도구도 그래프에 들어 있으므로 이 수는 실행 파일 DLL 개수가 아니다.
- 각 패키지의 Cargo 라이선스 식과 원문 고지는 [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md)에 정리했다. 로컬 Cargo 레지스트리에서 가져온 `LICENSE`, `COPYING`, `NOTICE` 파일 76개를 [`THIRD_PARTY_LICENSES/`](../THIRD_PARTY_LICENSES/)에 원문 그대로 보존했다. 이 파일들의 저작권·라이선스는 Jusage의 MIT 저작권으로 대체되지 않는다.
- 이 잠금 그래프에서 선언된 라이선스 식은 MIT, Apache-2.0, Unicode-3.0, Unlicense 계열이다. GPL 단독 식은 보이지 않았다. 이는 Cargo 메타데이터와 포함된 고지 파일을 기준으로 한 인벤토리이며 법률 의견은 아니다.
- 런타임에 포함되는 이미지 자산은 `assets/jusage.ico`다. 창 글꼴은 Windows 시스템 글꼴을 사용하고 트레이 숫자 그림은 코드로 만든다. 추적된 PNG 캡처는 QA 증거이며 앱 리소스가 아니다. 아이콘 출처 문제는 별도로 해결해야 한다.

## Windows Release 빌드 기록

### 입력 소스

Windows QA 소스 복사본에는 `.git` 디렉터리가 없었다. 대신 실행 파일 입력 전체를 SHA-256으로 비교했다. `Cargo.toml`, `Cargo.lock`, `build.rs`, `assets/jusage.ico`, 그리고 `src/*.rs`가 기준 커밋의 파일 내용과 모두 일치했다. 그러므로 아래 재빌드는 소스 내용 기준으로 `8b5c785b2c62da5353d814cfa2167f4d27075c22`를 사용했다. Git 메타데이터 자체는 그 QA 복사본에서 확인할 수 없었다.

### 빌드 환경

- Windows x64, target `x86_64-pc-windows-msvc`
- `rustc 1.90.0` (`1159e78c4`), `cargo 1.90.0`, LLVM `20.1.8`
- Visual Studio 2022 Build Tools MSVC `14.44.35207`; linker `14.44.35226.0`
- MSVC 환경은 `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat`로 설정
- 의존성은 Cargo.lock 고정 및 오프라인 캐시 사용

### 실행한 명령과 산출물

별도의 새 설치 환경을 만들지 않고, 기존 QA 소스 복사본에서 서로 다른 새 산출물 디렉터리로 빌드했다. 기존 QA Release 실행 파일과 설치본 경로는 덮어쓰지 않았다.

```powershell
$env:RUSTFLAGS = '-C link-arg=/Brepro'
$build = 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul && "C:\Users\user\.cargo\bin\cargo.exe" build --release --locked --offline --target x86_64-pc-windows-msvc --target-dir "<새-산출물-폴더>"'
& $env:ComSpec /d /c $build
```

각 빌드 성공 후 `Get-FileHash -Algorithm SHA256`으로 확인한 결과:

| 산출물 경로 | 크기 | SHA-256 | 의미 |
| --- | ---: | --- | --- |
| `C:\Users\user\src\jusage-v03-qa-b2d7bb6-scroll\target\release\jusage.exe` | 1,595,904 | `4F2E123560B3F49E4E15953B6577A1A2CCA33AB04BD5BA0D3792B1D9EB7986E2` | 이전 실제 `--isolated` GUI 검증 기록의 산출물. 정확한 당시 빌드 입력·명령은 확인되지 않음 |
| `C:\Users\user\src\jusage-v03-repro-target-20261028\x86_64-pc-windows-msvc\release\jusage.exe` | 1,595,904 | `878149CA34D7750484DAC2E894ECE2538FC7B8673A60210345729FE50D29B7E5` | 같은 현재 소스 입력의 기본 옵션 독립 재빌드 |
| `C:\Users\user\src\jusage-v03-brepro-a-20261028\x86_64-pc-windows-msvc\release\jusage.exe` | 1,595,904 | `3DC9998285599D8F10C97BE573231B40F01A9BD251808DE423163706548598A0` | 재현 옵션 검증 A |
| `C:\Users\user\src\jusage-v03-brepro-b-20261028\x86_64-pc-windows-msvc\release\jusage.exe` | 1,595,904 | `C030D4099588D991A06B84321679DF5B3137B56B14AD0D67303401B363B35813` | 재현 옵션 검증 B |

두 `/Brepro` 산출물도 SHA-256이 다르다. 비교한 두 파일은 `.text` 내용은 같지만 PE 헤더, `.rdata`, `.rsrc`가 달랐다. 원인은 확정하지 못했다. 따라서 **현재 바이트 단위 재현 빌드는 통과하지 못했으며**, 위 해시들은 서로 대체 가능한 값이 아니다. 기존 GUI QA 증거의 빌드와 새 재현 빌드가 동일 바이너리라고 주장하지 않는다. 릴리스 전에 변동 영역의 원인을 찾아 고정한 뒤 독립 빌드가 같은 해시를 내는지 다시 확인해야 한다.

기본 재빌드와 `/Brepro` 재빌드는 Release 컴파일을 완료했으며 `dead_code` 경고 13개가 출력됐다. 이번 작업에서는 테스트를 실행하지 않았다.

## 설치 업그레이드·롤백 QA 계획

기존 V0.2.0 운영 설치본은 시험에 사용하지 않는다. 별도 Windows VM 또는 전용 시험 사용자와 되돌릴 수 있는 스냅샷을 준비한 뒤 승인된 환경에서만 아래 순서를 실행한다.

1. 공개 V0.2.0 asset과 V0.3 후보를 별도 시험 프로필에 준비한다. 시작 항목, 프로세스, 실행 경로·해시, 설정과 기록의 존재·해시를 기록한다. 개인 기록 내용은 보고서에 복사하지 않는다.
2. V0.2.0이 실행 중인 상태에서 승인된 업그레이드 흐름을 적용한다. 파일 잠금 처리, 실행 파일 경로, 자동 시작 값, 프로세스 중복, 트레이 아이콘이 하나인지 확인한다.
3. V0.3 실행 후 설정과 기록 보존, 재실행 및 시작 경로를 확인한다. 두 버전 프로세스나 잔류 아이콘이 없어야 한다.
4. 스냅샷 롤백을 수행하고 V0.2.0 버전·파일 해시·설정·기록 및 단일 트레이 동작을 다시 확인한다.
5. 별도 디스플레이가 있는 승인된 시험 환경에서 125%·150% DPI, 듀얼 모니터 이동, 팝업 경계와 내부 스크롤을 검증한다. 현재 사용자 디스플레이 설정은 바꾸지 않는다.

별도 Windows VM/시험 사용자는 준비되지 않았고, 이 작업에서 새 환경이나 설치본은 만들지 않았다. 따라서 업그레이드·롤백 Gate는 **BLOCKED**, 실제 125%·150% DPI와 듀얼 모니터 Gate는 **미검증**이다.

## 출시 게이트

아이콘 제작 출처를 확정하거나 독립 자산으로 교체하고, 반복 Release 산출물의 해시가 일치하는 빌드 절차를 만들며, 격리 업그레이드·롤백과 실제 DPI/모니터 QA를 끝내기 전까지 권고는 **HOLD**다. `main` 병합과 공개 Release는 하지 않았다.

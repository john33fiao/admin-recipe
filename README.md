# 레시피 관리자 어드민

- 식당 "순창한상" 오픈 준비를 위해 사용합니다

셰프 1명이 레시피와 식당 운영 정보를 관리하는 웹 UI 기반 설치형 앱을 직접 개발합니다. 초기에는 로컬에서 사용하고 안정화 후 별도 서버로 이관합니다. 소스코드를 공개할 계획이며, 운영 데이터는 공개 코드와 분리합니다.

## 현재 상태

Rust 서버에 Nuxt UI를 내장한 첫 실행파일과 재료 이름·관리 단위 등록 → SQLite/JSON 저장 → 목록·상세 조회를 구현했습니다. **T-001은 진행 중**입니다. macOS ARM64에서 검증했으며 Windows·별도 Mac의 배포 수용 기준은 아직 남아 있습니다. 재고·원가·판매 등 전체 업무 기능은 미구현입니다.

- **1차:** 레시피·식재료 원가·발주·입고·판매·재고
- **2차:** 내일 메뉴 계획·작업시간
- **백로그:** 시설 점검·부분 입고·보관 위치
- **초기 운영:** 단일 PC 사용. 바이너리 하나와 옆의 `db/` 폴더 하나로 실행하며 SQLite·JSON 변경 기록·사진을 이 폴더에 보관

1차 목표는 마감에 메뉴 판매·폐기를 입력하고 확인한 품목의 현재고를 대조하는 것입니다. 부족분 자동 폐기·손실 원가 배분·현재 보유량·평균 원가·단위당 가격 조회는 확정한 요구사항이며 아직 구현하지 않았습니다.

현재 재료 등록마다 JSON을 기록하고 무기한 보관하며 비밀번호 잠금은 없습니다. 선택 상태 전체 복원·사진 보관은 후속 구현 범위입니다. 다른 PC·Mac의 호환 빌드에서 같은 데이터를 사용하고 사진까지 옮길 때는 `db/` 전체를 함께 가져가는 것이 확정한 목표입니다.

운영 예외 네 가지와 Rust 백엔드·Nuxt 프론트엔드를 확정했습니다. 바이너리가 로컬 서버와 웹브라우저를 열며 WebSocket heartbeat 연결이 일정 시간 끊기면 종료합니다. 앱 내부 업데이트는 제외하고 새 앱이 이전 DB를 계속 지원합니다. Electron은 현재 고려하지 않습니다. [기술 결정 기록](docs/decisions/ADR-005-rust-nuxt-local-server.md)과 [첫 구현 티켓](docs/tickets/T-001-portable-local-app.md)에 구현 결과·미검증 항목을 정리했습니다. 기존 화면 초안은 레이아웃 참고안입니다.

## 개발·실행

개발 환경은 Rust 1.98과 C 컴파일러/OS SDK, Node `^22.22.3 || ^24.15.0 || >=26.0.0`입니다. 버전은 Cargo.lock·frontend/package-lock.json으로 재현합니다. 빌드 도구는 개발 시 필요하며 배포 실행 시 Node 서버를 사용하지 않습니다.

```sh
npm --prefix frontend ci
node scripts/build.mjs
```

macOS 실행파일은 `target/release/sunchang-hansang`, Windows는 `target/release/sunchang-hansang.exe`입니다. 자신이 쓰기 가능한 폴더로 해당 OS의 실행파일을 복사한 뒤 실행하세요. 기본 브라우저가 열리고 실행파일 옆 `db/`에 데이터가 생성됩니다. 브라우저 자동 열기에 실패하면 터미널에 출력된 주소를 직접 엽니다. `--no-browser`로 자동 열기를 생략할 수 있습니다.

모든 화면의 연결이 끊기면 120초 유예 후 종료하며 진행 중인 저장을 기다립니다. 초기 접속 대기도 120초입니다. 같은 `db/`를 중복 실행하면 두 번째 앱은 오류로 종료합니다. 앱을 수동 교체하거나 데이터를 옮길 때는 먼저 정상 종료한 뒤 `db/` 전체를 보존하세요. 이전 앱이 새 DB를 읽는 역방향 호환은 보장하지 않습니다.

JSON 기록 오류가 표시되면 같은 화면의 **같은 요청 재시도**를 사용하세요. DB에는 이미 저장됐을 수 있습니다. 앱 재실행으로 포트가 바뀐 경우 목록을 먼저 확인하세요. JSON 변경 기록은 구현했지만 선택 시점 전체 복원과 사진은 아직 구현하지 않았습니다. 운영 데이터의 영속 저장소는 `db/`이며 Git에 넣지 않습니다.

## 검증

UI를 변경했다면 먼저 `node scripts/build.mjs`를 실행합니다. Cargo 빌드는 누락되거나 오래된 UI 자산을 거절합니다.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
npm --prefix frontend test
npm --prefix frontend run typecheck
node --test scripts/ui-manifest.test.mjs
cd frontend
npm exec -- playwright install chromium
npm run test:e2e
```

실행 결과와 수용 기준별 상태는 [T-001 검증 기록](docs/tickets/T-001-verification.md)에 있습니다. 브라우저 검증은 가상 데이터를 임시 한글·공백 경로에 만들고 release 실행파일을 사용합니다. 같은 Mac에서 PATH를 비운 실행과 시스템 라이브러리 검사는 통과했으나 **개발 도구가 없는 별도 Windows/Mac, 실제 절전, 다운로드 서명·공증과 OS 간 이동은 미검증**입니다.

## 개발 문서

의존성 감사에는 빌드 도구의 `node-forge` 관련 경고가 남아 있습니다. 현재 실행 경로 검토와 한계는 [검증 기록](docs/tickets/T-001-verification.md)에 함께 적었습니다.

- [요구사항과 미결정 항목](docs/REQUIREMENTS.md)
- [요구사항 일괄 재검사](docs/REQUIREMENTS-AUDIT.md)
- [진행 순서와 할 일](docs/TODO.md)
- [구조·기술 결정 현황](docs/ARCHITECTURE.md)
- [결정 기록](docs/decisions/README.md)
- [작업 티켓](docs/tickets/README.md)
- [참고 자료 정책](docs/refs/README.md)
- [문서 이력](docs/deps/INDEX.md)

실제 레시피 배합, 매입 단가, 매출, 거래처 정보, 계정 정보와 운영 자료 원본은 저장소의 공개 대상에 포함하지 않습니다. 예제와 테스트에는 별도로 만든 가상 데이터를 사용합니다.

## 라이선스

이 프로젝트의 소스코드와 직접 작성한 문서는 [MIT License](LICENSE)를 따릅니다. 상업적 이용·수정·재배포를 허용하며, 복제본 또는 상당 부분에는 원본 저장소 주소가 포함된 저작권 고지와 허가 고지를 유지해야 합니다. 수정본을 공개할 의무는 없습니다. 원본 저장소는 [john33fiao/admin-recipe](https://github.com/john33fiao/admin-recipe)입니다.

별도 라이선스가 표시된 제3자 자료는 해당 조건을 따릅니다. 선택 근거와 적용 범위는 [라이선스 결정 기록](docs/decisions/ADR-001-mit-license.md)에 정리했습니다.

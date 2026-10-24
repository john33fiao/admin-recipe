# T-005 검증 기록

- 작성일: 2026-10-24
- 관련: [티켓](T-005-business-history.md), [계획과 변경 전 검토](T-005-plan.md)
- 상태: 공통 기반·대표 가격 업무 READY — 실제 판매 통합은 T-009/T-021 대기

## 범위와 검토

대표 사전 가격 입력을 DB 초안으로 저장·수정·보관하고, 미리보기 이후 같은 상태/입력 버전에서 가격과 업무 확정을 원자적으로 저장한다. 정정은 이유와 원본/대체 연결을 보존한다. 실제 판매·입고·생산·실사와 해당 정정의 수량/가치/배분 계산은 후속 티켓이다.

독립 설계 검토에서 정정 이유, 원본부터 현재 유효 확정까지의 조회 범위를 보완했다. 미리보기는 원정정 대상 가격·현재 최신 사전 가격·새 가격을 구분한다. 입력 형식/종류/버전/길이는 초안부터 검사하되, 미완성 숫자 문자열은 보존하고 실제 참조·단위·수량/금액은 확정 전에 검증한다.

## 현재 실행 근거

| 명령/검토 | 관측한 결과 |
| --- | --- |
| `cargo test --locked --test business_storage` | RED — 새 모듈 import/API 부재, exit101. 기존 최신 UI 자산에서 실행했으며 UI guard 실패가 아님 |
| `cargo test --locked --test business_http` | RED — 컴파일 후 4개 실패, 목록/화면404 및 새 POST405. 기존 Host/Origin/session 거절은 먼저 통과 |
| DTO 독립 검토 | strict envelope/meta/payload/data·닫힌 종류/버전·원입력 보존·UUID/길이/제어문자·숫자 외형을 대조해 차단 결함 없음 |
| `node scripts/build.mjs` 첫 통합 | PASS — Nuxt 생성·자산 검증·Rust release·개인 경로 잔존 검사 |
| 첫 `cargo test --locked` | HTTP6·payload3·저장8 PASS. 이전 DB fixture 설치 2개는 outbox 참조를 먼저 삽입해 FK 오류 발생 |
| `cargo test --locked --test business_storage` | fixture 전체 설치를 한 transaction으로 묶고 참조를 commit 때 검사한 뒤 10 PASS. 제품 FK 검사를 완화하지 않음 |
| `npm --prefix frontend test`, `npm --prefix frontend run typecheck` | 최종 UI 입력 보완 후 단위17 PASS·typecheck exit0 |
| 최종 `node scripts/build.mjs` | PASS — 최종 테스트 대기 보완·Rust 정적 검사 보완 뒤 정상 생성·release·경로 검사 |
| `cargo test --locked` | 73 PASS: lib7·bin1·business HTTP6/payload3/storage10·catalog HTTP5/storage8·domain4·hot journal4·기존 HTTP8/lifecycle4/storage13 |
| 최종 `cargo test --locked --test business_storage` | 10 PASS — clippy가 지적한 중첩 조건문을 같은 의미의 let-chain으로 정리한 뒤 영향 검증 |
| `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings` | PASS — cache hardlink 대신 copy하는 환경 경고만 남음 |
| 기존 `test:e2e -- local-app.spec.ts ui-foundation.spec.ts catalog.spec.ts --fully-parallel --workers=6` | 17 PASS·4.2분. 실제 120초 정상 종료·재시작·재료/가격/공통 UI 회귀 |
| 신규 `test:e2e -- business.spec.ts --fully-parallel --workers=3` 및 영향4 재실행 | 최종 7개 시나리오 PASS. 최초4 PASS/3 FAIL의 라우트 대기를 보완한 뒤 실패3+같은 경로1을 재실행해 4 PASS·4.1분. 영향 없는 기존3 PASS와 함께 확인 |
| `node --test scripts/ui-manifest.test.mjs`, `node scripts/ui-manifest.mjs --check`, `node --test scripts/release-artifact.test.mjs` | 자체 시험1·현재 입력/자산 일치·경로 잔존 시험1 PASS |

저장 재검증의 기본 sandbox 실행은 Windows 임시 파일 발행 권한(os error5)으로 실패했다. 기존 기준과 같은 승인된 실행 권한으로 동일 명령을 실행해 통과했으며 환경 실패를 별도로 보존했다. frontend 변경 뒤 정상 표준 빌드를 다시 수행하고 Cargo·브라우저 검증을 실행했다. 구현된 초안 자료를 실제 판매나 재고 지원으로 계산하지 않는다.

실행 환경은 Windows 11 Home x64, Rust/Cargo1.99.0, Node24.19.0, npm12.2.1, Playwright1.64.1·Chromium156.0.8078.4(revision1248)다. 원시 로그·trace·스크린샷은 Git 제외 경로에 보존한다. 새 업무의 일반 UI 시나리오는 가상 재료/초안을 작성하고 기본 유예로 앱을 두 번 정상 종료·재시작한 뒤 0원 가격 확정과 이유를 담은 10원 정정을 확인했다. DB 주입이나 쓰기 API 직접 호출을 사용하지 않았다. 실패/응답 유실 검증의 네트워크 대체와 fixture 정리용 강제 종료는 정상 시연 근거와 구분한다.

## 수용 기준 대응

| 기준 | 공통 기반·대표 가격 업무의 근거 | 후속 통합 |
| --- | --- | --- |
| T-005-01 | 미완성 DB 초안·원입력·수정/보관 이력·Store 재시작과 가격 행 불변. 정상 UI 프로세스 재시작은 아래 브라우저 결과로 확인 | 실제 수량/가치 행 불변은 T-007 이후 연결 |
| T-005-02 | 초안은 미완성 값을 보존하고 참조·형식·단위 문제를 필드별 표시. 공통 엔진에 재고 부족/레시피0 차단을 넣지 않음 | **미검증:** 실제 판매0·재고 부족 허용·판매 자체 재고 불변은 T-009 |
| T-005-03 | 가격·업무 확정·사용 품목 버전·요청·시각·seq·JSON을 같은 transaction에 저장. 가격/확정/정정 edge/outbox 오류의 전체 rollback 검증 | 실제 수량/가치·소모/배분 결과는 T-007~011 |
| T-005-04 | 원응답 재생·다른 본문/대상/명령의 요청키 충돌·같은 값의 별개 두 업무를 저장/API 검증 | **미검증:** 정상 별개 판매2건은 T-009 |
| T-005-05 | 초안 생성/수정/보관/확정/정정과 요청 결과가 전체 JSON에 포함됨. 발행 실패 게이트·재시도·재시작·누락 파일 원문 복구 검증 | 후속 업무 kind·사진·전체 복원은 각 티켓 |
| T-005-06 | 원본부터 유효 확정까지의 정정 chain·원가격/현재가격/새가격 구분, 세션·Origin·Host·종료 경계를 검증 | 실제 재고 업무의 영향 재계산은 T-012 |

T-005는 후속 기능이 사용하는 공통 기반의 구현 범위와 판매 통합 확인을 분리한다. 판매 관련 미검증을 완료로 바꾸지 않고 T-009/T-021에서 다시 확인한다.

## 변경 파일과 독립 검토

`src/business.rs`와 `tests/business_payload.rs`는 닫힌 종류/버전·엄격한 중첩 입력·UUID·길이/제어문자·미완성 문자열 보존을 다룬다. `src/store/mutation.rs`는 기존 요청 원응답을 바꾸지 않고 공통 트랜잭션을 추출한다. `src/store/snapshot.rs`는 버전별 판독·현재 전체 기록을, `src/store/business.rs`는 초안/확정/가격 결과/정정 관계를 맡는다. 기존 store/catalog와 lib 연결, HTTP의 새 경로와 SPA 허용 범위를 함께 갱신했다.

저장·HTTP 테스트와 합성 v3 DB/v2-schema3 JSON fixture를 추가하고 기존 저장 테스트의 현재 schema 기대값만 갱신했다. Nuxt에는 업무 목록·상세·가격 초안 편집, 공유 전송/초안 상태, 단위/E2E 검증을 추가했다. 탐색·재료 상세 진입점·스타일과 README·ARCHITECTURE·TODO·티켓을 실제 상태에 맞춘다.

독립 리뷰는 이전 JSON 엄격 판독/원문 보존, recover와 원응답 재생의 순서, 가격·확정·정정·요청의 원자성, HTTP 보호, 닫힌 handler의 확장 경계를 확인했다. 추가 차단 결함은 없었다. source schema3의 outbox가 새 schema4를 포함한 경우도 거절하도록 상한을 명시했다. parser 오류 테스트는 DB outbox와 파일에 동일하게 손상된 원문을 넣어 파일 불일치 검사만으로 통과하지 않게 보완했다.

## UI 검토와 발견 문제

공유 요청이 끝난 후 재진입한 화면의 입력/기준이 남지 않도록 초안·기준·요청 상태를 같은 수명으로 관리한다. 다른 화면에서 확정한 경우 저장된 확정 내용과 이 탭의 미저장 입력을 분리해 표시한다. 저장 중 이동과 검토 조회의 경합은 실제 Vue effectScope 단위 검증으로 확인한다.

신규 재료 선택에는 활성 품목과 이미 선택된 보관 품목만 표시한다. 비동기 `현재 재료 정보 적용`은 요청한 ID·현재 선택·잠금·컴포넌트 생존을 다시 확인해 늦은 응답으로 다른 선택의 버전을 덮어쓰지 않는다. 기존 확인/보관 대상의 참조는 유지한다.

최초 브라우저 실행에서는 라우트 이동 완료를 기다리지 않은 테스트의 `goBack`·즉시 URL 검사로 실패했다. 화면/trace를 확인한 뒤 도착 화면 또는 URL을 기다리도록 테스트를 수정하고 영향4개를 재실행해 통과했다. 제품 소스 변경으로 실패를 숨기지 않았다. 1440×900·768×1024·390×844의 라이트/다크 6개 정정 미리보기 화면을 모두 직접 검토하고 대표 데스크톱/모바일 2개를 독립 검토했다. 입력·버튼·가격 전후·기록 링크·포커스와 가로 넘침 없음을 확인했다.

## 변경 후 RTD

- [Step 1] 목적 적합성 - PASS: 초안/확정/정정 공통 기반과 실제 가격 결과를 6개 기준에 대응했다. 실제 판매 조건은 미검증으로 남겨 후속 T-009에서 확인한다.
- [Step 2] 오류·중요 문제·보안 - PASS: 엄격 중첩 입력·종류/버전·참조·단위·정수 수치, 불변 요청 재시도·복구 게이트·CAS·HTTP 보호를 확인했다. 공개 후보에서 개인 경로와 실제 개발 자료 혼입이 없음을 검사했다.
- [Step 3] 회귀 - PASS: 기존 요청 외형/원응답·UUID·epoch/revision·옛 JSON 바이트와 기존17 브라우저 흐름을 보존했다. 판매 즉시 차감 등 대체된 운영 정책을 추가하지 않았다.
- [Step 4] 파일·함수 크기 - PASS: 실제 두 저장 경로가 공유하는 mutation/snapshot을 추출하고 DTO·업무 SQL·HTTP·UI 상태를 역할별로 분리했다. 무관한 전면 리팩터링은 없다.
- [Step 5] 재사용·통합 - PASS: 기존 가격 effect·수치/단위·Store 잠금/recover·세션·UI API/연결/테마를 재사용했다. 동적 handler 등록이나 새 프레임워크/의존성은 없다.
- [Step 6] 부작용 - PASS: 기존 백업 후 단일 migration tx로 schema4를 적용하며 전체 기록은 누적된다. 누적 기록 비용은 T-021에서 측정한다. 새 서비스·전역 설정·백그라운드 동기화는 없다.
- [Step 7] 전체 변경 재검토 - PASS: DTO/저장/HTTP 독립 리뷰와 UI의 terminal·비동기 조회 검토, diff·새 파일을 재확인했다. 연결 노트의 기존 변경을 보존하고 데이터 구조/후속 범위를 맞췄다.
- [Step 8] 불필요한 잔여물 - PASS: 원시 로그·trace·검증기·실제 자료는 제외 경로에 둔다. 미지원 업무를 저장/확정하는 fallback과 테스트 전용 제품 API가 없다.
- [Step 9] 품질 검증 - PASS: 정상 표준 빌드, Rust73+영향10·UI단위17·typecheck·브라우저24개 시나리오·fmt/clippy·자산 검사를 실행했다. UTF-8/LF·상대 링크·diff·후보 ignore 규칙을 확인했다.
- [Step 10] 사용자·개발자 흐름 - PASS: 일반 UI로 임시 저장→실제 재시작→편집→미리보기→0원 확정→재시작→정정/이력을 확인했다. 필드 오류/포커스·재시도·보관·3너비/2테마를 검증하고 실행 근거를 연결했다.
- [Step 11] 발견 문제의 원인 - PASS: fixture의 참조 삽입 순서, 테스트의 SPA 대기 누락, 편집 상태 수명/늦은 조회를 구분해 보완하고 관련 검증을 다시 실행했다. 파일 발행 권한 실패를 제품 성공으로 계산하지 않았다.
- [Step 12] READY 판단 - PASS: 공통 기반·대표 가격 업무는 READY다. 실제 판매/재고·정정 영향 계산, 전체 복원·배포·T-021 시연의 완료를 뜻하지 않는다. 판매 기준의 체크는 후속 통합까지 열어 둔다.
- [Step 13] 요청된 커밋·PR 준비 - PASS: 사용자의 명시적 승인에 따라 이번 공개 변경만 준비한다. 기존 staged가 없는지와 후보 ignore·unstaged/staged diff를 확인하고 로컬 자료/노트는 포함하지 않는다. push·PR·배포는 하지 않는다.

## 동기화와 남은 범위

연결 기획 노트에 2026-10-24·C-02·D-02·D-11~D-13과 함께 DB 초안/확정·가격 정정·schema4/JSON2·불변 원본/후속 경계를 추가했다. 양쪽을 재읽고 새 두 블록을 제거하면 기존 노트 바이트와 일치함을 확인했다. 기존 사용자 변경을 보존했으며 노트는 커밋하지 않는다.

다음은 T-006 발주다. 판매 통합은 T-009, 실제 재고 정정은 T-012, 전체 복원은 T-018, 호환/이동은 T-019, 실제 개발 자료의 격리 시연은 T-021에 남는다. T-001의 별도 깨끗한 Windows/Mac·실제 절전은 미검증이며 로컬 검증으로 대체하지 않는다.

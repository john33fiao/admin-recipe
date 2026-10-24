# T-006 검증 기록

- 작성일: 2026-10-24
- 관련: [티켓](T-006-purchase-orders.md), [계획과 RTD Before](T-006-plan.md)
- 상태: READY — T-006 수용 범위 구현·검증 완료, 실제 입고 연결은 T-007

## 착수와 실제 RED

T-005 공통 기반을 검증하고 커밋한 뒤 설계·독립 검토를 마쳤다. 일반 주문 수정/취소의 선택 메모, 가격 정정의 필수 이유, schema5 허용 상태·frozen schema4와 요청 재생 순서를 구별했다. 실제 입고 연결은 T-007이며 T-006은 가상 입고 결과의 종료 계약을 검증한다.

| 명령 | 관측한 결과 |
| --- | --- |
| `cargo test --locked --test business_payload` | 컴파일 성공, 4 PASS/1 FAIL. 기존 가격 canonical 원문은 동일하고 새 주문 action은 이전 가격 DTO에서 거절되어 실패 |
| `cargo test --locked --test purchase_order_storage` | 컴파일 성공, 1 FAIL. 실제 주문 초안의 action을 이전 DTO가 거절함 |
| `cargo test --locked --test purchase_order_http` | 컴파일 성공, 1 PASS/4 FAIL. 주문 GET/SPA404, 새 action POST400. 기존 접근 보호·엄격 입력/크기 거절 검증은 통과 |

모두 당시 최신 T-005 자산에서 실행했으며 UI guard·파일 권한 실패를 RED로 계산하지 않았다. UI 단위 검사에서는 주문 payload 복사가 가격 kind로 바뀌고 가격 전용 검증이 수정·취소에 없는 필드를 요구하는 두 실패를 관측했다. 닫힌 kind/action 분기와 공통 저장 상태 기계를 연결해 해결했다. 순수 입고 종료 함수의 개별 사례를 구현 전 실패로 실행했다고 주장하지 않으며, 최종 계약 검증에 포함했다.

## 구현과 수용 기준

주문은 공통 DB 초안·미리보기·확정 경로를 사용한다. 주문의 안정적인 ID 아래 최초·수정·취소 버전을 보존하며 업무 초안/확정과 주문 미입고/취소 상태를 구분한다. 수정은 같은 품목의 수량·호환 표시 단위·선택 정보를 변경하며 수정·취소 메모는 선택이다. 가격 정정의 필수 이유는 유지한다.

| 기준 | 구현·실제 검증 근거 | 완료 경계 |
| --- | --- | --- |
| T-006-01 | kg/L·자유 단위, 선택 거래처/날짜/금액, 미완성 DB 초안·재열기·확정. 빈 예상 금액은 null, `0`은 유효한 금액 | 주문 품목 교체·거래처 마스터는 범위 밖 |
| T-006-02 | 주문·수정·취소의 저장/API 검사에서 재료·가격·기존 자료 불변 확인. 공통 원자성 경로에 주문 효과만 적용 | 실제 보유 원장은 T-007에서 연결한 뒤 추가 회귀 |
| T-006-03 | 취소 버전과 확정·원응답을 남기고 주문 head만 변경. 새 UUID의 중복 취소·수정은 거절, 원요청 재시도는 최초 응답 재생 | 입고 후 반품과 일반 업무 정정은 별도 티켓 |
| T-006-04 | 실제 Store 주문으로 typed 가상 입고 결과를 계산. 10kg→9.8kg, 수정 12kg→11.8kg에서 차이 -200g, 최초/현재 주문 버전 보존. 함수 실행 전후 DB/seq/JSON 불변 | 실제 receipt·received 저장·입고 UI는 T-007 |
| T-006-05 | 오래된 주문 head·닫힌 상태 거절, 최초/수정/취소 이력 조회, 동일 요청 재생·본문 충돌 구분. 순수 계약에서 received의 재종료 거절 | 실제 입고 후 DB/API의 중복 종료는 T-007 |

schema5에 주문 원본·버전·head를 추가했다. 이전 schema4는 가격 전용 reader로 고정하고 T-005 실행파일이 가상 자료로 만든 DB/JSON fixture의 UUID·가격 요청 canonical 문자열·응답·모든 JSON 원문을 보존한다. schema4의 주문 kind, schema5의 근거 없는 received·잘못된 action/status, 미래 형식·누락 컬렉션은 거절한다. 오류 주입으로 효과·확정·outbox 실패의 전체 rollback과 DB commit 후 JSON 실패의 같은 요청 복구를 확인했다.

## 실행 명령과 결과

Windows x64, Node 24.19.0·npm 12.2.1, Rust/Cargo 1.99.0, Nuxt 4.6.2·Vue 3.5.44. 브라우저는 Playwright 1.64.1의 Chromium 156.0.8078.4이다. 모든 제품 검증은 가상 자료와 격리 DB를 사용한다.

| 명령 | 결과 |
| --- | --- |
| `node scripts/build.mjs` | PASS. UI 생성·현재 입력/산출물 기록·Rust release·실행파일 경로 검사 |
| `cargo test --locked` | PASS, 99개. 신규 순수 계약7·주문 HTTP6·주문 저장10과 공통 payload6, 기존 전체 회귀 포함 |
| `cargo fmt --check` | PASS |
| `cargo clippy --locked --all-targets -- -D warnings` | PASS |
| `npm --prefix frontend test` | PASS, 22개 |
| `npm --prefix frontend run typecheck` | PASS |
| `node --test scripts/ui-manifest.test.mjs` | PASS, 오래된 입력·누락/변형 자산 거절 1개 |
| `node scripts/ui-manifest.mjs --check` | PASS, 현재 자산 해시 |
| `node --test scripts/release-artifact.test.mjs` | PASS, 빌드 workspace/홈 경로 잔존 검사 1개 |
| `npm --prefix frontend run test:e2e -- local-app.spec.ts ui-foundation.spec.ts catalog.spec.ts business.spec.ts --fully-parallel --workers=6` | PASS, 24개·4.2분 |
| `npm --prefix frontend run test:e2e -- orders.spec.ts --fully-parallel --workers=3` | PASS, 5개·2.1분 |

frontend 변경 후 표준 빌드를 먼저 실행했으며 Cargo 자산 guard를 우회하지 않았다. 원시 로그·trace·DB·화면 캡처는 Git 제외 로컬 경로에 보존한다. 실행 토큰·개인 경로·실제 배합/가격은 공개 기록에 넣지 않는다.

브라우저 신규 사례는 정상 UI로 미완성 초안 저장→기본120초 종료→같은 DB 재시작→10kg 주문→12kg/예상0원 수정→빈 메모 취소를 실행했다. 이력 3개·주문량·가격 불변을 확인했으며 수정 완료 뒤 다시 수정 화면을 여는 흐름도 확인했다. 각 action의 응답 유실과 다른 탭의 주문 head 변경 후 원요청/입력 보존도 통과했다. 준비된 결과를 SQLite에 주입해 정상 흐름을 대신하지 않았다.

1440/768/390px와 라이트/다크 6개 화면을 직접 열어 긴 이름·거래처 줄바꿈·버튼·미리보기·가로 넘침을 확인했다. 통합 담당도 데스크톱 라이트와 모바일 다크를 다시 확인했다. 별도 390px 가상 DB 세션에서 Tab/화살표/Enter/Space로 주문 작성 진입·재료 선택·입력·DB 저장·초안 열기·미리보기·확정까지 실행했다. 날짜 오류 필드와 미리보기 영역의 포커스 이동도 확인했다.

## 변경 파일과 다음 작업

- 계약·API: [business.rs](../../src/business.rs), [purchase_order.rs](../../src/purchase_order.rs), [server.rs](../../src/server.rs), [lib.rs](../../src/lib.rs).
- 저장·호환: [store.rs](../../src/store.rs), [business 저장](../../src/store/business.rs), [주문 저장](../../src/store/purchase_order.rs), [snapshot](../../src/store/snapshot.rs), `tests/fixtures/schema-v4.sql`·`snapshot-v2-schema4.json`.
- 화면: `frontend/app/`의 주문 목록/상세·BusinessOrderEditor·주문 types/utils, 기존 business 종류 분기·공통 초안 상태·내비게이션/스타일.
- 검증: `tests/purchase_order*.rs`, `tests/business_payload.rs`와 이전 저장 회귀, frontend 주문 단위/E2E와 공통 상태 검사.
- 문서: 이 검증 기록·계획·티켓·색인, README·ARCHITECTURE·TODO와 연결 노트.

다음 순차 작업은 T-007 실제 입고·보유 평균 원가다. T-006의 가상 입고 종료를 실제 입고 저장·재고 계산 성공으로 처리하지 않는다. T-005의 실제 판매 통합은 T-009, 실제 실사·최종 시연은 T-010/T-021, 전체 복원·이동은 T-018/T-019에 남는다. T-001 별도 깨끗한 Windows/Mac·실제 절전 등 필수 미검증은 그대로 남으며 출시·goal 완료의 근거가 아니다.

## RTD After

- [Step 1] 목적 적합성 - PASS: 기준01~05를 실제 주문과 순수 입고 종료 계약에 대응했다. 실제 입고 구현·시연 완료와 구별했다.
- [Step 2] 오류·중요 문제·보안 - PASS: 닫힌 kind/action·문자열 수치·길이/날짜·참조·상태/자료 버전·16KiB·세션/Host/Origin을 검사했다. 공개 후보에 개인 경로·개발 시연 자료·인증 정보가 없다.
- [Step 3] 회귀 - PASS: 기존 가격 wire/pending·원응답·옛 JSON·UUID를 유지한다. Rust99와 기존 브라우저24가 통과했고 주문은 가격/재고를 바꾸지 않는다.
- [Step 4] 파일·함수 크기 - PASS: 주문 계약·저장·화면을 별도 모듈로 두고 공통 business에는 명시적 분기만 연결했다. 관련 없는 리팩터링은 추가하지 않았다.
- [Step 5] 재사용·통합 - PASS: 기존 mutation·recover·epoch/revision·도메인 수치·UI 초안 상태·디자인 토큰을 재사용한다.
- [Step 6] 부작용 - PASS: schema5 migration과 새 주문 이력/JSON이 저장량을 늘린다. 실패 rollback·백업·옛 파일 복구를 확인했다. 외부 전송·전역 설정·배포는 없다.
- [Step 7] 전체 변경 재검토 - PASS: diff와 새 파일을 통합·독립 검토했다. 가격 전용 frozen4, 원응답 재생 뒤 상태 검사, 같은 트랜잭션과 UI pending 보존에 차단 결함이 없었다.
- [Step 8] 불필요한 잔여물 - PASS: 가짜 입고 확정·재고 집계·거래처 마스터·범용 실행 엔진을 추가하지 않았다. 임시 실행 자료는 공개 후보에서 제외했다.
- [Step 9] 품질 검증 - PASS: 표의 실제 build/test/typecheck/fmt/clippy/자산 검사가 통과했다. 공개40파일의 UTF-8/LF·EOF·상대 링크·diff·ignore·민감 경로 검사를 완료했다.
- [Step 10] 사용자·개발자 흐름 - PASS: README·티켓·색인·TODO·구조를 실제 구현과 맞췄다. 정상재시작·충돌/재시도·6화면·키보드 흐름을 확인했다. D-01·D-05·D-08·D-11~D-13의 데이터 구조/구현 경계를 연결 노트와 같은 작업에서 맞추고 양쪽을 재읽었다. 기존 노트 바이트·사용자 변경을 보존했다.
- [Step 11] 발견 문제의 원인 - PASS: 가격 전용 payload 복사/검증이 주문을 잘못 처리한 UI RED를 kind/action 분기로 고쳤다. 새 주문 버전마다 새 수정 편집을 시작하면서 불확실한 기존 요청은 원 identity로 복구하도록 검증했다. 통합 검증 뒤 제품 추가 수정은 없었다.
- [Step 12] READY 판단 - PASS: T-006 수용 범위 READY. 실제 입고는 T-007, 최종 시연·필수 실환경은 T-021/T-001에 남는다. 출시·goal 완료를 뜻하지 않는다.
- [Step 13] 요청된 커밋·PR 준비 - PASS: 사용자의 명시적 커밋 승인을 적용한다. 기존 staged 변경 없음·이번 공개40경로·비무시 경로를 확인했다. 실제 staged diff로 최종 범위를 확인한 뒤 논리 단위로 커밋하며 로컬 전용 기록·연결 노트는 포함하지 않는다. push·PR·배포는 하지 않는다.

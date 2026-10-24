# T-004 검증 기록

- 작성일: 2026-10-24
- 관련: [티켓](T-004-recipes-and-costs.md), [계획과 RTD Before](T-004-plan.md)
- 상태: READY — T-004 수용 범위 구현·검증 완료, 실제 판매 통합은 T-009/T-021

## 착수와 실제 RED

T-007 커밋 `85ea42a`의 실제 입고·보유 원장, 현재 가격 선택과 기존 요청/JSON 호환 검증을 확인했다. 품목의 레시피 버전/행/head, 직접 구성의 정확 원가, 독립 복제, 검색/양방향 조회와 schema7/frozen6 계획을 독립 검토했다.

독립 검토에서 화면에 표시한 배합 버전과 원가 응답의 경합을 보완했다. UI는 표시한 version ID로 원가를 조회하고 응답 ID·활성 화면 generation을 검사한다. 새 가격 state가 편집 CAS 기준이나 진행 중 요청을 덮어쓰지 않는다. 가격 계산 실패도 저장한 배합의 조회·편집을 막지 않는다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test --locked --test recipe_storage` | 정상 컴파일·기존 fixture 열기 후 1 FAIL. 실제 schema6와 기대 schema7의 차이로 실패 |
| `cargo test --locked --test recipe_http` | 정상 컴파일 후 1 PASS/5 FAIL. 목록·사용처·SPA404, 새 레시피 POST405를 확인. 기존 접근/본문 보호는 PASS |
| `npm --prefix frontend test` | 기존/새31 PASS·새1 FAIL. 참고 생산량의 중첩 객체를 편집하면 원본도 바뀌는 실제 assertion 실패를 확인 |

새 HTTP 검사의 첫 실행에서 무세션 응답을403으로 기대한 테스트 오류를 기존 계약401로 바로잡고 다시 실행했다. 제품을 변경하지 않은 최종 RED는 위 결과다. 새 기능이 없음을 컴파일 오류로 대체하지 않았다. 이 검사는 frontend 편집 전에 기존 T-007 자산으로 실행했고, 이후 Cargo·브라우저 검증 전에는 정식 빌드로 자산을 갱신한다.

T-007 release의 정상 API로 별도 가상 DB에 가격/정정·발주/수정/입고·직접 입고/누적 보유와 미완성 초안18건을 만들었다. [schema6 fixture](../../tests/fixtures/schema-v6.sql)와 [최종 JSON](../../tests/fixtures/snapshot-v2-schema6.json)은 원요청/응답·18개 outbox/JSON 바이트가 일치하는 것을 확인한 뒤 캡처했다. 실제 운영 데이터·T-021 시연 자료를 쓰지 않았다. fixture 캡처는 최종 시연이 아니다.

## 구현과 수용 기준

기존 품목 UUID에 불변 레시피 버전·구성 행·현재 head를 연결했다. 새 품목과 최초 레시피 동시 생성, 기존 품목 연결, 수정과 독립 복제는 공통 mutation 안에서 수행하며 원응답·전체 JSON을 함께 기록한다. 원가 조회는 배합 조회/저장과 분리한다.

| 기준 | 구현·실제 검증 근거 | 완료 경계 |
| --- | --- | --- |
| T-004-01 | 종류/단계 강제 없는 직접 구성, 같은 재료 여러 행, 자기/다단계/0수량 순환 거절, 과거 버전 합집합만 순환인 정상 현재 그래프, 검색·현재 직접 사용처 중복 제거 | 실제 생산은 T-008 |
| T-004-02 | 기본0·적당량 메모·빈 구성, 원입력 수량/단위·3개분100g 분자/분모와 참고 생산량 보존. 이후 수정이 이전 버전/합성 판매 참조를 바꾸지 않음을 검사 | 실제 판매 저장/참조는 T-009/T-021에서 통합 |
| T-004-03 | 10원/g·3개분100g→333.333333원, 서로 다른 단가 분모·행별 1회 반올림·정확 합계·중간/최종 상한 검사. 직접 비축품 단가만 적용하고 하위 원가를 재합산하지 않음 | 참고 생산량으로 실제 재고/수율을 만들지 않음 |
| T-004-04 | 사전 가격과 실제 입고 보유 평균 연결, 출처와0원·가격 없음 구별. 0수량 missing은 행0, 양수 missing은 행/전체null·알려진 소계. 과거 배합도 현재 가격 평가로 명시 | 0재고 마지막 단가/미확인 가치의 순수 계약과 실제 입고 연결을 구분. 생산/실사 전이는 후속 통합 |
| T-004-05 | 판매가 없음/0원 각각 다른 불가 사유, 양수 판매가·완전0원은0%. exact micro원 정수 비율을 UI 표시에서만 소수2자리로 반올림 | 세금·할인·결제 제외 |
| T-004-06 | 현재 저장본만 새 owner/version/line UUID로 복제. 보관 이름 포함 충돌 번호·100자 한글/비BMP·접미 자리수 검사. 같은 요청 재생·복제 편집 후 원본 불변 | 원본 가격/재고/자식 품목은 복제하지 않음 |
| T-004-07 | 과거 구성·당시 품목 버전/판매가 보존, 최신 품목 정보의 명시적 적용, 과거 배합과 현재 가격 분리. schema6 원문18건·migration·변조 거절과 요청 재생 | 실제 판매 당시 원가/확정 소모 고정은 T-009/T-010/T-021 |

복제 JSON은 정확한 원본 내용·복제 시점 최신 source 버전과 일치해야 한다. owner/component 버전은 레시피 작성 당시 존재해야 하고, recipe seq는 snapshot 시점 이내이며 실제 recipe 명령과1:1로 대응해야 한다. DB·JSON의 고아 참조·중복·잘못된 이력/행 순서/환산·현재 순환을 거절한다. 앞선 schema6의 가격/주문/입고 reader는 고정하며 원문을 재직렬화하지 않는다.

## 실행 명령과 결과

Windows x64, Node24.19.0·npm12.2.1, Rust/Cargo1.99.0, Nuxt4.6.2·Vue3.5.44, Playwright1.64.1·Chromium156.0.8078.4에서 가상 데이터·격리 DB로 실행했다.

| 명령 | 실제 결과 |
| --- | --- |
| `node scripts/build.mjs` | 최종 PASS. UI 생성·자산 기록·Rust release·실행파일 개인경로 검사 |
| `cargo test --locked` | 최종168 PASS. 레시피 순수12·HTTP9·저장13과 기존 회귀 포함 |
| `cargo fmt --check` | 최종 PASS |
| `cargo clippy --locked --all-targets -- -D warnings` | 최종 PASS |
| `npm --prefix frontend test` | 37 PASS |
| `npm --prefix frontend run typecheck` | PASS |
| `node --test scripts/ui-manifest.test.mjs` | 오래된/누락·변형 자산 거절1 PASS |
| `node scripts/ui-manifest.mjs --check` | 최종 자산/입력 해시 PASS |
| `npm --prefix frontend run test:e2e -- local-app.spec.ts ui-foundation.spec.ts catalog.spec.ts business.spec.ts orders.spec.ts receipts.spec.ts --fully-parallel --workers=6` | 기존36 PASS, 6.3분 |
| `npm --prefix frontend run test:e2e -- recipes.spec.ts --fully-parallel --workers=3` | 첫 실행6 PASS/테스트 경합1 FAIL. 수정·최종 backend 빌드 후 신규7 PASS, 2.1분 |

신규 브라우저는 정상 UI로 구성 저장·기본120초 정상 종료·동일 DB 재시작, 정확 분수·미계산/0·원가 상한 오류에도 편집, 참고 생산량·독립 복제, 과거 품목 참조·보관, 생성/복제 응답 유실·JSON 실패 재생, 다른 탭409·413·순환 입력 보존과 늦은 조회/이동 중 저장을 검사했다. 초기 실패는 아래에 원인과 수정을 기록하며 최종7개 성공·정상 종료 코드를 확인했다.

1440/768/390px 라이트/다크6개 화면을 직접 열어 긴 이름·폼·원가·복제·이력과 가로 넘침을 확인했다. 통합 담당도1440 라이트·390 다크·키보드 결과 화면을 재검토했다. 별도 가상 DB에서 Tab/Arrow/Enter/Space로 작성·행 추가/삭제 포커스·메모 Enter 무전송·빈 수량 오류 포커스·0수량 저장·현재 저장본 복제를 실행해 PASS했다. 테스트만 수정했으며 화면 검토 뒤 UI 제품은 바꾸지 않았다.

## 발견 문제와 수정

- UI의 첫 얕은 복사가 참고 생산량을 원본과 공유했다. 실제 단위 RED 후 중첩 객체/행을 복사하고 구조 비교·immutable pending·복원 검사를 보완했다. 새 owner의 종류 선택도 기존 신규 catalog 허용 범위에 맞췄으며 기존 unclassified 참조·복제/거절 요청 복구는 유지했다.
- 독립 정적 검토에서 복제 출처 내용/당시 최신 head·미래 품목 버전 참조의 빈틈을 발견했다. 최소 validator와10종 변조 회귀를 추가했고 첫 통합에서 모두 거절·원문 불변을 확인했다. 이 발견을 실제 수정 전 행동 RED로 주장하지 않는다.
- 첫 전체 Rust는166 PASS/1 FAIL이었다. 기존 v1 migration 검사의 기대 schema6만 남아 실제7과 불일치했다. 기대값을 갱신하고 기존 저장13개 및 최종 전체168개를 재실행해 PASS했다.
- recipe seq·명령·stable 품목 연결은 별도 실제5모드 RED로 확인했다. 미래 seq, 잘못된 operation, 명령 누락/중복, stable 품목 누락을 모두 수용하던 경로를 최소 관계 검증으로 닫았다. 레시피 저장13·기존 저장13 GREEN과 최종 전체 검증을 통과했다.
- 첫 신규 브라우저의 늦은 조회 사례는 gate 해제 직후 handler를 제거해 `Route is already handled`로 실패했다. 응답 완료를 기다린 후 handler를 해제하도록 테스트 경계를 수정했다. 제품 입력 동작을 바꾸지 않았으며 정식 재빌드 후 전체7개를 재실행해 통과했다.

## 변경 파일과 후속 범위

- 계약/순수 함수: [recipe.rs](../../src/recipe.rs), [검사](../../tests/recipe.rs), lib 등록.
- 저장/호환: [레시피 저장](../../src/store/recipe.rs), [frozen6](../../src/store/snapshot/frozen6.rs), store/catalog/snapshot 연결·schema6 fixture2개와 저장/이전 migration 검사.
- HTTP/화면: server 레시피 경로·국소256KiB 한도, recipe types/utils/composable/editor/cost/uses·목록/신규/상세/과거 화면·기존 재료/탐색과 단위/E2E.
- 문서: 계획·티켓·검증 기록·색인·README·ARCHITECTURE·TODO와 연결 노트.

C-01·C-03·D-04·D-09의 구조/구현 경계를 연결 노트에 반영하고 양쪽을 재읽었다. 두 추가 블록을 제거하면 기존 노트 바이트가 일치하며 기존 사용자 변경·메타데이터·엑셀 분석을 보존했다. 노트·로그/trace·임시 DB·시연 개인 자료는 Git 제외 경로에 두고 커밋하지 않는다.

다음 순차 작업은 T-008 실제 생산과 원가 이월이다. T-005 실제 판매 조건은 T-009, 실사/보정은 T-010/T-011, 최종 격리 시연은 T-021에 남는다. T-001 별도 깨끗한 Windows/Mac·실제 절전 등 필수 미검증을 이 티켓으로 완료 처리하지 않는다.

## RTD After

- [Step 1] 목적 적합성 - PASS: 기준01~07을 직접 구성·정확 분수/원가·0/없음·복제·버전/참조·검색에 대응했다. 실제 생산/판매/실사 통합 경계를 구분했다.
- [Step 2] 오류·중요 문제·보안 - PASS: 닫힌 중첩 입력·UUID/문자열 수치/길이·단위/상한·참조/순환·Host/Origin/세션을 검사했다. 레시피 POST만256KiB, 기존16KiB 유지. 원문/요청 변조15종과 개인정보 경계를 확인했다.
- [Step 3] 회귀 - PASS: Rust168·UI37·기존 브라우저36·신규7이 통과했다. schema6 실제18개 요청/응답·기존 UUID·모든 JSON 바이트와 입고/보유 가격을 보존했다.
- [Step 4] 파일·함수 크기 - PASS: 순수 DTO/계산과 저장 모듈을 분리했다. 저장 파일은 레시피 CRUD/그래프/원가 읽기·schema/검증으로 역할을 한정한다. 큰 가상 fixture는 기존 전체 JSON 원문 호환을 재현하기 위한 자료다.
- [Step 5] 재사용·통합 - PASS: 공통 mutation/recover/CAS, Money/Quantity/Unit/가격 선택, catalog 내부 생성과 기존 디자인/연결 상태를 재사용했다. 새 업무 종류/메뉴 마스터/범용 폼 프레임워크를 추가하지 않았다.
- [Step 6] 부작용 - PASS: schema7의 세 관계와 전체 snapshot 기록량이 늘어난다. 이전 DB 원문·rollback·JSON 실패 복구를 검증했으며 최종 누적 기록 비용은 T-021에서 확인한다. 실제 재고 효과·외부 전송·전역 설정은 없다.
- [Step 7] 전체 변경 재검토 - PASS: 새 파일과 diff를 통합/독립 검토했다. 복제/시점/seq 참조의 발견 사항을 닫고 실제 거절·원문 불변을 확인했으며 차단 결함이 남지 않았다.
- [Step 8] 불필요한 잔여물 - PASS: 자동 생산·수율·과거 판매 재계산·가격/재고 복제를 넣지 않았다. 원시 로그/trace/임시 DB·개인 자료는 로컬 제외 경로에 보존한다.
- [Step 9] 품질 검증 - PASS: 표의 정식 빌드·전체 Rust·정적/포맷·UI 단위/타입·자산·브라우저를 실행했다. 공개40경로의 UTF-8/LF·EOF·상대 링크·diff·ignore·개인 자료 검사를 확인했다.
- [Step 10] 사용자·개발자 흐름 - PASS: README·구조·티켓·색인·TODO와 연결 노트를 맞췄다. 주요 편집/복제·원가 실패·재시작/재시도·다른 탭·늦은 조회, 6개 화면과 실제 키보드 흐름을 확인했다.
- [Step 11] 발견 문제의 원인 - PASS: 중첩 복사, 복제/품목 시점·seq 참조 부족, 이전 schema 테스트 기대값, 브라우저 handler 해제 경합을 구별해 기록했다. 최종 build/전체168/신규7로 관련 보완을 검증했다.
- [Step 12] READY 판단 - PASS: T-004 수용 범위 READY. 실제 판매와 생산/실사·전체 시연·필수 별도 장비/절전은 후속 작업으로 남는다. 전체 운영·배포 준비·goal 완료를 뜻하지 않는다.
- [Step 13] 요청된 커밋·PR 준비 - PASS: 사용자의 명시적 승인을 적용한다. 기존 staged 변경 없음과 이번 공개40개 비무시 경로를 확인했다. 실제 staged diff를 검토해 논리 단위로 커밋하며 로컬 기록/노트는 제외한다. push·PR·배포는 하지 않는다.

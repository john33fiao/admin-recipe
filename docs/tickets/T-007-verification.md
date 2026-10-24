# T-007 검증 기록

- 작성일: 2026-10-24
- 관련: [티켓](T-007-receipts.md), [계획과 RTD Before](T-007-plan.md)
- 상태: READY — T-007 수용 범위 구현·검증 완료, 실제 실사/보정은 T-010/T-011

## 착수와 실제 RED

T-006 커밋 `51c1d5c`와 검증 결과를 확인하고 설계·독립 검토를 마쳤다. 직접/주문 연결 입고, 현재 보유 평균과 마지막 유효 단가, 관측과 정산 기준의 구별, 이미 포함된 지연 입고 임시 보관, schema6/frozen5를 구현한다. 실제 실사 화면과 지연 입고 보정은 각각 T-010/T-011이다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test --locked --test business_payload` | 컴파일 성공, 6 PASS/1 FAIL. 이전 가격/주문 검사는 유지됐고 새 receipt kind가 미지원이라 실패 |
| `cargo test --locked --test receipt_storage` | 컴파일 성공, 1 FAIL. 기존 Store로 실제 입고 초안을 저장하려 할 때 receipt kind가 미지원이라 실패 |
| `cargo test --locked --test receipt_http` | 컴파일 성공, 1 PASS/4 FAIL. 입고/보유 GET·입고 SPA404, receipt 초안400을 확인. 기존 접근 보호·본문 크기 검사는 통과 |
| `npm --prefix frontend test` | 기존22 PASS/새4 FAIL. 중첩 source 편집의 dirty 누락, 원본과 객체 공유, 구조 비교·pending 복원 실패를 관측 |

backend RED는 frontend를 변경하기 전 최신 T-006 산출물에서 실행했다. 컴파일 실패·자산 guard·권한 실패를 제품 행동 RED로 계산하지 않았다. 이후 frontend 편집을 시작했으며 다음 Cargo·브라우저 검증은 정식 `node scripts/build.mjs`로 산출물을 갱신한 뒤 실행한다.

UI의 중첩 입력을 종류별로 복사하고 구조 비교하여 원본·전송 본문을 보존했다. 새 입고를 반복 작성할 때 이전 생성 응답의 revision이 재진입 시 최신 revision을 덮는 추가 RED를 확인하고 명시적 새 작성에서 최신 기준을 유지하도록 고쳤다.

## 구현과 수용 기준

직접/미입고 주문 연결 입고를 공통 DB 초안·미리보기·확정에 연결했다. 실제 수량·구입금액·입고일은 빈 입력에서 시작하며 주문의 예상값을 실측값으로 자동 확정하지 않는다. 입고·보유 수량/가치·실제 주문 종료·확정·요청 원응답·전체 JSON은 같은 트랜잭션이다.

| 기준 | 구현·실제 검증 근거 | 완료 경계 |
| --- | --- | --- |
| T-007-01 | 직접 입고와 실제 주문 10kg→수정12kg→실입고11.8kg. 최초/현재 주문량·실량·차이 -200g과 원버전/실제 입고 링크를 저장/API/브라우저에서 확인 | 1회 입고로 종료. 부분 입고는 제외 |
| T-007-02 | 과거 윤일 입력·자동 기록시각 분리, 고정 현지 날짜 공급원으로 오늘/미래/잘못된 달력 검사. UTC 날짜와 다른 현지 오늘을 계약 검사 | 미래 예정일은 발주. 실제 OS 시계 변경·절전 검증은 별도 |
| T-007-03 | 호환 단위 환산과 정확 Q/V 합계, 시작 기준의 배타적 seq로 중복 없음. 가상 관측10kg 이후 입고2kg에서 관측/시각은 그대로이고 정산 기준12kg·후속+2kg 표시 계약 확인 | 실제 관측 writer/실사 화면 연결은 T-010 |
| T-007-04 | 미입력 금액 차단·0원 확정, 현재 보유 평균과 마지막 유효 단가 갱신, 사전 가격 행 불변. 미확인·확인한0·가치 미확인·마지막/사전 가격 우선순위의 순수 계약 검증 | 현재 DB는 입고로 생긴 양수 보유. 0재고/실사 전이는 후속 업무에서 통합 |
| T-007-05 | 이미 포함/확인 필요 입력을 미완성 상태로 DB 저장·정상 종료/재시작. 미리보기·확정을 차단하고 입고/보유/주문 효과가 없음을 확인 | 포함량 보정 확정은 T-011 |
| T-007-06 | 가상 1kg·10,000원+2kg·40,000원→3,000g·50,000원. 정확 비율50,000/3,000원/g와 0원 추가, UI와 저장/API 일치 | 건별 단가의 단순 평균이나 표시 반올림 값을 계산에 재사용하지 않음 |

첫 입고는 입고 기준을 만들며 실제 관측을 생성하지 않는다. 기존 품목을 확인한0으로 이관하지 않고, 마지막 확인 수량/시각과 현재 정산 기준/이후 확정 변동을 분리한다. 입고 미리보기는 현재 Q/V에 한 건을 더하고 조회는 정확한 시작 기준 이후 사건을 재구성한다. 존재하지 않는 미래 holding ID를 미리 만들지 않는다.

schema6에 입고·불변 보유 버전/현재 연결·주문 종료 관계를 추가했다. frozen5는 이전 가격/주문만 허용한다. T-006 release의 정상 API로 가상 가격 정정과 주문 작성·수정·취소 총11건을 저장해 만든 fixture에서 이전 UUID·요청 canonical 문자열·원응답·모든 JSON 원문을 보존했다. 오류 주입에서 입고/보유/주문 종료/확정/outbox 전체 rollback과 DB commit 후 JSON 실패의 동일 요청 복구를 확인했다.

## 실행 명령과 결과

Windows x64, Node24.19.0·npm12.2.1, Rust/Cargo1.99.0, Nuxt4.6.2·Vue3.5.44, Playwright1.64.1·Chromium156.0.8078.4. 가상 자료와 격리 DB만 사용하며 운영 자료·T-021 시연 자료를 공개 테스트에 사용하지 않는다.

| 명령 | 실제 결과 |
| --- | --- |
| `node scripts/build.mjs` | PASS. 최종 UI 생성·현재 자산 기록·Rust release·실행파일 개인경로 검사 |
| `cargo test --locked` | 최종134 PASS. holding7·receipt7·receipt HTTP8·receipt 저장12·payload7과 기존 회귀 포함 |
| `cargo fmt --check` | 최종 PASS |
| `cargo clippy --locked --all-targets -- -D warnings` | 최종 PASS. 테스트 초기화 정리 후 영향 holding7개도 재실행 PASS |
| `npm --prefix frontend test` | 28 PASS |
| `npm --prefix frontend run typecheck` | PASS |
| `node --test scripts/ui-manifest.test.mjs` | PASS, 오래된 입력/누락·변형 자산 거절1개 |
| `node scripts/ui-manifest.mjs --check` | PASS |
| `npm --prefix frontend run test:e2e -- local-app.spec.ts ui-foundation.spec.ts catalog.spec.ts business.spec.ts orders.spec.ts --workers=6` | 29개 PASS, 14.2분. 정상 종료를 기다리는 기존 파일 내 사례는 순차 실행 |
| `npm --prefix frontend run test:e2e -- receipts.spec.ts --fully-parallel --workers=3` | 신규7개 PASS, 2.1분. 최종 backend 보완 빌드에서 같은7개를 재실행해 PASS |

frontend 변경 후 정식 빌드를 먼저 실행했고 Cargo 자산 guard를 우회하지 않았다. 원시 로그·trace·화면·임시 DB는 로컬 Git 제외 경로에 보존한다. 최초 통합의 중첩 JSON 관련3target 실패를 숨기지 않고 아래에 원인·수정을 기록한다. 실제 테스트를 하지 않은 정식 배포·별도 장비 검증을 PASS로 표시하지 않는다.

신규 브라우저는 미완성 직접 입고→DB 저장→기본120초 정상 종료→같은 DB 재시작→편집/미리보기/확정, 포함 여부 확인 초안의 재시작/차단, 주문 실제 입고 종료, 응답 유실/JSON 실패·다른 탭의 변경·전송 중 이동/재진입·늦은 대상 조회를 실행했다. 정상 UI/API에서 상태를 만들었고 기대값이나 DB 수정으로 입고 결과를 주입하지 않았다.

1440/768/390px 라이트/다크 미리보기6개와 390px 다크 보유 화면을 직접 열어 긴 이름·표시 단위·입력/버튼·가로 넘침을 확인했다. 통합 담당도 데스크톱 라이트와 모바일 보유 화면을 재검토했다. 별도 가상 DB에서 Tab/Enter/Space로 직접 입고 작성·재료/수량/단위/날짜/0원·메모 입력·임시 저장·미리보기 포커스·확정까지 실행했다.

## 발견 문제와 수정

- 주문 버전 테이블만 교체하는 migration은 기존 head FK의 deferred counter 때문에 COMMIT가 실패했다. heads와 versions를 함께 보존·교체해 FK를 끄지 않고 해결했다. 독립 SQLite 실험 뒤 실제 Rust schema5 fixture 검사도 통과했다.
- Serde의 tagged unit variant는 `deny_unknown_fields`만으로 빈 분기의 추가 필드를 거절하지 않았다. 입력 unreflected와 출력 direct의 실제 RED를 재현하고 빈 struct variant로 바꿔 같은 JSON 외형을 유지하면서 거절하도록 수정했다. payload·순수·HTTP 검사를 재실행해 통과했다.
- 독립 검토에서 schema6 JSON 입고의 외부 업무 확정/품목 버전 참조 검사가 부족한 점을 발견했다. 없는/다른 확정·잘못된 종류/입력 버전/seq/품목 버전·결과 중복/누락·확정 수량 불일치·잘못된 holding UUID의10개 실제 RED를 확인했다. schema6 전용 관계 검증으로 모두 거절하고 DB/JSON 원문 불변과 정상 저장12개를 확인했다. frozen5는 유지한다.
- 새 receipt 미리보기의 큰 enum은 주문 원본 필드만 Box로 보관해 JSON 외형을 유지하며 줄였다. 테스트 초기화 스타일도 정적 검사 기준에 맞췄다. UI 중첩 복사/비교·새 작성 revision 문제는 앞의 RED와 단위28개로 확인했다.

## 변경 파일과 후속 범위

- 계약/API: [holding.rs](../../src/holding.rs), [receipt.rs](../../src/receipt.rs), [business.rs](../../src/business.rs), [purchase_order.rs](../../src/purchase_order.rs), [server.rs](../../src/server.rs), [lib.rs](../../src/lib.rs).
- 저장/호환: [입고 저장](../../src/store/receipt.rs), [보유 저장](../../src/store/holding.rs), store/business/catalog/purchase_order/snapshot과 schema5 fixture2개.
- 화면: receipt types/utils·BusinessReceiptEditor·HoldingPanel·입고 목록/상세, 기존 업무/재료/주문 화면과 공통 초안의 중첩 입력 보존.
- 검증: receipt 순수/HTTP/저장·holding/payload·이전 저장 회귀, frontend receipt 단위/E2E와 공통 상태 검사.
- 문서: 계획·티켓·검증 기록·색인, README·ARCHITECTURE·TODO와 연결 노트.

C-02·D-02·D-04~D-05·D-08·D-11~D-13의 데이터 구조/구현 경계를 연결 노트에 반영하고 양쪽을 재읽었다. 새로 추가한 두 블록을 제외한 기존 노트 바이트와 사용자 변경을 보존했다. 노트는 커밋하지 않는다.

다음 순차 작업은 T-004 레시피·메뉴 복제와 원가 계산이다. T-005 실제 판매 통합은 T-009, 실제 실사/보정은 T-010/T-011, 일반 정정은 T-012, 전체 복원/이동은 T-018/T-019, 최종 격리 DB 시연은 T-021이다. T-001 별도 깨끗한 Windows/Mac·실제 절전 등 필수 검증도 남는다. 이 티켓 결과는 전체 운영·배포 준비·goal 완료의 근거가 아니다.

## RTD After

- [Step 1] 목적 적합성 - PASS: 기준01~06을 실제 직접/주문 입고·가중 Q/V·0원·날짜·차단 초안과 가상 관측 계약에 대응했다. 실제 실사/보정 완료와 구분했다.
- [Step 2] 오류·중요 문제·보안 - PASS: 닫힌 중첩 입력·버전/UUID·문자열 수치·참조/상한·현지 날짜·세션/Host/Origin·16KiB를 검사했다. 추가 필드와 고아/중복/불일치 JSON 참조를 실제 거절하며 공개 후보에 개인 자료가 없다.
- [Step 3] 회귀 - PASS: Rust134·UI28·기존 브라우저29와 신규7이 통과했다. 이전 가격/주문 wire·원응답·UUID·JSON bytes와 아카이브/재시도 동작을 보존했다.
- [Step 4] 파일·함수 크기 - PASS: receipt/holding 계약과 저장을 분리하고 기존 business/snapshot에는 종류 분기와 필요한 검증만 추가했다. 큰 미리보기 필드는 Box로 보관하며 외부 JSON은 같다. fixture의 크기는 이전 실제11건의 원문 보존 때문이다.
- [Step 5] 재사용·통합 - PASS: 공통 mutation/recover/CAS·도메인 수치/가격·초안 상태·디자인 토큰을 재사용한다. 실제 수량 효과를 별도 가짜 가격 기록으로 대체하지 않는다.
- [Step 6] 부작용 - PASS: schema6 migration과 입고/보유 이력이 저장량을 늘린다. 기존 관계 보존·백업·rollback·JSON 실패 복구를 검증했다. 실제 관측·지연 보정·외부 전송·전역 설정은 추가하지 않았다.
- [Step 7] 전체 변경 재검토 - PASS: 새 파일과 전체 diff를 통합/독립 검토했다. 발견한 JSON 참조 P2는10개 실제 RED→거절 GREEN과 정상 저장12개로 해결했고 차단 결함이 남지 않았다.
- [Step 8] 불필요한 잔여물 - PASS: 거래처 마스터·부분 입고·예상 현재고·가짜 관측/보정 확정을 넣지 않았다. 임시 실행 자료와 개인 자료는 로컬 제외 경로에 보존한다.
- [Step 9] 품질 검증 - PASS: 표의 build/Rust/단위/타입/fmt/clippy/자산/브라우저 검증을 실행했다. 공개52파일의 UTF-8/LF·EOF·상대 링크·diff·ignore·개인 경로 검사를 완료했다.
- [Step 10] 사용자·개발자 흐름 - PASS: README·티켓·색인·TODO·구조와 연결 노트의 구현/후속 경계를 맞췄다. 정상 재시작·응답 유실/JSON 장애·복수 탭·키보드와7개 화면을 확인했다. 기존 노트 바이트와 사용자 변경은 보존했다.
- [Step 11] 발견 문제의 원인 - PASS: migration FK counter, nested UI 복사/동등성·새 작성 revision, Serde 빈 분기의 추가 필드, snapshot 외부 참조 누락을 재현하고 고쳤다. 최종 build/전체Rust 후 영향 입고7개를 재실행했다.
- [Step 12] READY 판단 - PASS: T-007 수용 범위 READY. 실제 실사/보정·전체 시연·필수 별도 장비/절전은 각각 후속 티켓에 남는다. 배포 준비·goal 완료를 뜻하지 않는다.
- [Step 13] 요청된 커밋·PR 준비 - PASS: 사용자의 명시적 커밋 승인을 적용한다. 기존 staged 변경 없음과 이번 공개52경로·비무시 경로를 확인했다. 실제 staged diff를 확인해 논리 단위로 커밋하며 로컬 전용 기록·노트는 제외한다. push·PR·배포는 하지 않는다.

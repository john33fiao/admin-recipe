# T-011 검증 기록

- 작성일: 2026-10-25
- 판정: READY — 명시적인 포함 입고와 경계 안내 구현·검증 완료. 실제 관측 정정 writer는 T-012에서 연결한다.
- 기준: [티켓](T-011-late-receipt-reconciliation.md), [구현 계획과 Before4](T-011-plan.md), [계산 계약 §8](../DOMAIN-CONTRACT.md#8-지연-입고의-수량금액-보정)

## 범위와 환경

실사 포함량을 명시적으로 선택하여 수량 중복을 막고 당시 확인 가치와 실제 구입금액의 차이를 현재 품목별 가치·별도 보정으로 나누는 흐름을 구현했다. 정상 UI/API와 가상 자료를 사용했다. 개발 자료 검증이나 가상 fixture를 T-021의 지정 실제 시연으로 세지 않는다.

환경은 Windows x64 10.0.26300, Rust/Cargo1.99, MSVC19.41, Node24.19.0, npm12.2.1, Python3.12.14, Playwright1.64.1·Chromium156.0.8078.4(headless)다. T-001의 별도 깨끗한 Windows/Mac·실제 절전·복귀는 미검증으로 유지한다.

## 구현과 변경 파일

| 영역 | 파일·변경 |
| --- | --- |
| 닫힌 입력/결과 | `src/reconciliation.rs`, `receipt.rs`, `business.rs`, `lib.rs`: 기존 receipt/version1에 새 reconciled 태그, 후보·수량/가치·연결 DTO |
| 순수 계산 | `src/reconciliation/provenance.rs`, `value.rs`: 정확 출처 예약, 고정 수량의 Base/Proposed 가치 재생, 금액 합계·별도 보정 |
| 저장 | `src/store/reconciliation.rs`, `schema.rs`, `store.rs`, `store/receipt.rs`, `holding.rs`, `business.rs`: 여섯 관계·원자 확정·실제 R 주문 종료·물리 열 검증 |
| 기존 원장 연결 | `src/holding.rs`, `store/production.rs`, `stocktake.rs`, `waste.rs`: 실제 보정 원천, U/Au/dV 한 번 반영·마지막 관측 보존 |
| 호환 | `src/store/snapshot.rs`, `snapshot/frozen8.rs`, `frozen9.rs`, 새 `frozen10.rs`; schema10 SQL/JSON 가상 fixture |
| HTTP | `src/server.rs`: 품목별 포함 근거 조회. 기존 세션/Host/Origin/16KiB 경계 유지 |
| 화면 | `ReceiptInclusionEditor.vue`, `ReceiptReconciliationResult.vue`, 기존 receipt 편집/상세·holding·business 페이지, 타입/입력 복사·복원·CSS |
| 시험 | reconciliation 순수/저장/HTTP 네 파일, 이전 저장 시험8파일의 현재 스키마 기대, UI 단위2파일·신규 E2E1파일 |
| 문서/의존성 | README·ARCHITECTURE·DOMAIN-CONTRACT·티켓/계획/검증·색인/TODO, Cargo.toml/lock |

실제 수량·금액은 기존6자리 정수 계약을 유지한다. 출처 지분과 확인 가치의 정확한 반올림 전 잔여에 `num-rational`·`num-bigint`를 사용한다. 잠금 파일에 num-bigint0.4.8·num-integer0.1.47·num-rational0.4.2를 추가했고 기존 num-traits0.2.19를 직접 참조한다. 기존 패키지 버전은 변경하지 않았다. 네 패키지의 MIT OR Apache-2.0 라이선스를 메타데이터에서 확인했다.

## 변경 전 행동과 실패 보완

기존 schema10 release의 정상 HTTP API로 입고→생산→판매/개수 정정→일반/초기0 실사→직원식 및 빈 실사 행·미완성 초안을 만들었다. 정상 요청·원응답·전체 JSON23건을 바이트 단위로 재생했다. SQLite 직접 수정으로 fixture를 제조하지 않았다. 새 reconciled 초안은 기대201/실제400, 후보 조회는 기대200/실제404였으며 실패 뒤 상태는 불변이었다.

UI 대상22개는 기존17 PASS/신규5 assertion FAIL이었다. 중첩 선택의 공유 때문에 원본/dirty baseline/pending 재시도 본문이 함께 바뀌었고 새 태그 복원도 실패했다. selector를 관측 전체/특정 원천으로 구체화한 후 같은 실패를 재확인하고 typed 깊은 복사·엄격 복원으로22개 PASS를 얻었다. 컴파일 실패를 기능 RED로 세지 않았다.

첫 Rust 대상48개는45 PASS/3 FAIL이었다. 후속 특정 원천 예약을 무시한 과거 전체 선택, 행별 Bv 중복 반올림, 실제 보유 기록이 있는데 상세 ID가 null인 문제였다. 추가 회귀에서 혼합 초과/묶음의 Bv 잔여, 실행 중 물리 열과 JSON의 불일치, 유효0 선택의 거절도 확인했다. 다음과 같이 수정하고 최종54개 모두 PASS로 확인했다.

- 과거 선택은 각 원천의 후속 고정 예약까지 통과하는 정확 상한으로 나눈다. 두 원천이 각각0.5micro인 경우도 전체1micro를 선택할 수 있다. 기존 예약은 다시 배분하지 않는다.
- 같은 관측의 이전/신규 정확 기대 가치 합을 한 번 반올림하고 이미 확정한 Bv를 뺀 잔여를 배분한다. 한 행·여러 행·여러 확정, 직접 초과/전체 묶음의 합계를 보존한다.
- 실제 보유 효과가 있는 상세에는 그 ID를 반환하고 별도 보정만 있으면 null을 유지한다. 유효0행은 근거만 보존하고 예약/가짜 보유 행을 만들지 않는다.
- 다섯 JSON 관계의 실제 키/시각/seq/position/FK와 JSON을 조회·복구 진입에서도 대조한다. 변조 거절 뒤 원본 DB를 보존한다.

신규 E2E 첫 실행은7 PASS/1 FAIL이었다. Chromium 선택 상자에서 방향키로 적용한 뒤 불필요한 Enter가 팝업을 다시 열어 첫 Tab이 닫기에 사용됐다. 실제 trace/activeElement로 확인하고 테스트의 Enter만 제거했다. 키보드 보조 검사는 빈 포함량의 서버 그룹 오류를 행 오류로 기대한 것이 원인이었다. 실제 그룹 포커스와 확정 차단으로 기대를 맞췄다. 제품·수치·timeout을 바꾸지 않았고 재build 후 영향1개와 키보드 검사가 통과했다.

## 실제 실행 결과

| 명령·검사 | 결과 |
| --- | --- |
| schema10 정상 API fixture·원응답/JSON 재생 | 23건 PASS; schema11 이관 뒤 동일23건 PASS |
| `cargo test --locked --no-fail-fast --test reconciliation_provenance --test reconciliation_value --test reconciliation_storage --test reconciliation_http` | 54 PASS, exit0: 출처16·가치14·저장17·HTTP7 |
| `cargo test --locked --no-fail-fast` | 381개 중372 PASS/9 FAIL, exit101. 실패는 기존 current schema10 기대값이며 과거 fixture 형식 실패가 아님 |
| 아래 저장 회귀7대상 명령 | 82 PASS, exit0. 현재11 기대 및 schema6 보유 행의 신규 nullable2열을 반영, 원래 열/원문 비교 유지 |
| 아래 실사 이관1대상 명령 | 1 PASS, exit0. 앞선 시도는 실행 중 같은 exe의 Windows 잠금(LNK1104)으로 미실행; 장시간 검사 종료 뒤 재실행 |
| schema10 frozen reader 단위3개 | 전체 실행에서3 PASS. 새태그/새관계/중첩 보유 필드/미래헤더 거절 |
| `npm --prefix frontend test` | 66 PASS, exit0 |
| `npm --prefix frontend run typecheck` | exit0 |
| `cargo fmt --check` | 정렬/줄바꿈 보완 뒤 최종exit0 |
| `cargo clippy --locked --all-targets -- -D warnings` | exit0 |
| `node scripts/build.mjs` | frontend 확정 뒤 최종exit0. UI14경로·자산 기록·release 빌드·실행파일 경로 잔존 검사 PASS |
| `node --test scripts/ui-manifest.test.mjs scripts/release-artifact.test.mjs` | 최종2 PASS, exit0 |
| `node scripts/ui-manifest.mjs --check` | 최종exit0 |
| 기존 E2E10파일 | 65 PASS, exit0/6.9분. 실제120초 종료·동일DB재시작 유지 |
| 신규 `reconciliations.spec.ts` | 최초7 PASS/1 FAIL(2.1분), 테스트 기대 수정·표준build 뒤 영향1 PASS(3초), 각각8개 성공 근거 확보 |
| 실제 키보드·화면 | 키보드 작성/오류/수정/미리보기/확정 exit0, 화면13장 직접 검토 |
| 파일/공개 경계 | UTF-8/noBOM/LF·상대 링크·diff check·후보 ignore·개인자료/절대경로 점검 PASS |

스키마 기대값 보완 후 명령:

```text
cargo test --locked --no-fail-fast --test business_storage --test catalog_storage --test production_storage --test purchase_order_storage --test recipe_storage --test sale_storage --test storage
cargo test --locked --test stocktake_storage schema_nine_open_adds_observations_without_rewriting_sales_or_holding_references -- --exact
```

전체381개 각각 성공 근거를 확보했으며, 단일 전체 명령이 exit0이었다고 표현하지 않는다. 기존101건 판매를 정상 Store로 누적하는 실사 시험도 생략·중단 없이 PASS했고 해당 suite는632.50초였다. 누적 JSON 검사의 debug 비용은 기존 T-021 측정 과제로 유지한다. release에서 작업별 지연·재시작·저장량과 정확 유리수 이력의 장기 성능은 별도 측정한다.

브라우저 명령:

```text
npm --prefix frontend run test:e2e -- local-app.spec.ts ui-foundation.spec.ts catalog.spec.ts business.spec.ts orders.spec.ts receipts.spec.ts recipes.spec.ts productions.spec.ts sales.spec.ts stocktakes.spec.ts --fully-parallel --workers=6
npm --prefix frontend run test:e2e -- reconciliations.spec.ts --fully-parallel --workers=3
npm --prefix frontend run test:e2e -- reconciliations.spec.ts -g "candidate responses cannot" --workers=1
```

기존65·신규8 각각73개 성공 근거다. 신규는 구형 초안 재시작/명시 전환, 전부·일부 포함과 주문 실제R, 후속 소모/0원 음수 차액, 다른 단위/0% 생산, 전량 폐기/noholding, create/revise/confirm 응답 유실·409, submicro 묶음과 중복 거절, 늦은 후보 응답·키보드 행 동작을 확인했다. 사진은 확정 상세6장,2행 편집/미리보기6장(1440/768/390×두 테마),390 결과1장이다. Tab/방향키/Space/Enter로 조작하고 그룹 오류 포커스·메모 줄바꿈 비전송·미리보기 포커스·확정까지 확인했다.

기존 Nuxt cache-driver 경고와 Node DEP0190 경고는 남아 있다. 실제 build와 보안 경계 검사는 통과했으나 이를 모든 의존성 경고가 해결됐다는 의미로 보지 않는다.

## 수용 기준

| 기준 | 판정과 실행 근거 |
| --- | --- |
| 01 관측/포함량 명시 선택 | PASS: 전체/특정 원천·기준 단위·빈값/0·후보 실패·늦은 응답·초안 재진입·키보드 확인. 날짜 자동 추정 없음 |
| 02 포함분 수량 중복 없음 | PASS: 전부/일부 포함·I0·U0/noholding·주문 실제R 종료와 재실사. 수량은U만 추가 |
| 03 가치 차액 보존 | PASS: Ai/Au/Bv/D·현재 품목별dV·별도 보정·음수D·0원·최소 단위 잔여와 한/여러 행·확정 동치 |
| 04 후속 소모/생산/폐기 | PASS: 후속 실사·다단계/다른 단위/0%·원품목 재산출·부분/전량 폐기. 원판매/실사/생산 결과 불변 |
| 05 영구 연결·이중 정산 방지 | PASS: 교차/과거 예약·미세 몫 묶음·사용 후 한도 비반환·UUID/CAS·발행 복구·rollback·관계 변조 거절 |
| 06 경계의 결과/해결 절차 | PASS:0원·소모 뒤 별도 보정·유효 참조/현재 상태 재검토·입력 보존 안내. 실제 관측 정정 writer와의 통합은 아래 후속 항목 |

실제 관측 정정으로 참조 버전·포함 한도가 바뀌는 writer 통합은 T-012에서 수행한다. 현재는 유효 버전/품목 검사와 상태 변경 후 새 미리보기·입력 보존 절차를 제공한다. 원기록/연결을 직접 수정하거나 포함량을 자동 축소해 통과시키지 않는다. T-012에서는 옛 미리보기 거절·연결 유효 개정 대체·중복 가치 방지를 실제 정정 경로로 추가 검증해야 한다.

## RTD After13

[Step 1] 목적 적합성 - PASS: 여섯 기준의 실제 흐름과 후속 정정 writer 경계를 구분했다.

[Step 2] 오류·중요 문제·보안 - PASS: 닫힌 입력·길이/행/본문 한도·세션/Host/Origin·정확 참조·물리 열/JSON 대조·공개 자료 경계를 확인했다. 새 SQL 식별자는 코드의 고정 목록뿐이다.

[Step 3] 회귀 - PASS: 기존 원응답/JSON·FK·구형 초안·가격/생산/실사/판매를 보존하고 현재 스키마 기대만 갱신했다. 별도 백로그 문서 변경은 그대로 보존했다.

[Step 4] 파일·함수 크기 - PASS: 출처/가치 순수 계산과 DB 관계·migration·화면 선택/결과를 분리했다. 큰 저장 검증은 실제 관계와 재계산의 역할에 한정하며 무관한 리팩터링을 추가하지 않았다.

[Step 5] 재사용·통합 - PASS: receipt 업무·공통 초안/확정·CAS·요청 재생·outbox·고정소수/최대잔여를 재사용했다. 새 전체 업무 프레임워크나 별도 재고 원장을 만들지 않았다.

[Step 6] 부작용 - PASS: immutable snapshot·정확 지분의 저장/계산 비용을 남겼다. 외부 전송·서비스·전역 설정·자동 동기화는 추가하지 않았다.

[Step 7] 전체 변경 재검토 - PASS: 독립 리뷰가 출처 상한·Bv 잔여·물리 FK/seq·schema11/frozen10·원자 저장·화면 타입을 검토했다. 새 차단 결함은 남지 않았다.

[Step 8] 불필요한 잔여물 - PASS: 임시 DB·실행파일·trace·화면·진단/동기화 도구는 로컬 제외 경로에 보존했다. 미사용 호환 wrapper는 실제 호출 없음 확인 후 제거했다.

[Step 9] 품질 검증 - PASS: Rust381개 각각·UI66·E2E73개 각각·타입/포맷/Clippy·최종 표준build/자산·문서 검사를 실행했다. 최초 실패와 재검증을 구별했다.

[Step 10] 사용자·개발자 흐름 - PASS: 기존 입고 작성/보존/재시작에서 근거 선택·미리보기·확정·조회로 연결한다. 실제 ID로 원기록에 연결하며 가짜 보유 기록이나 관측ID URL을 만들지 않는다.

[Step 11] 발견 문제의 원인 - PASS: 원천 분할 방향·독립 반올림·상세 연결·live 물리 검증·0처리 결함을 실제 실패로 보완했다. 브라우저 실패는 기본 선택 상자/그룹 포커스의 시험 기대를 바로잡아 검증했다.

[Step 12] READY 판단 - PASS: T-011 범위 READY. T-012/T-013 통합, T-021 실제 자료 시연·장기 성능 및 T-001 외부 환경 검증은 완료 처리하지 않는다.

[Step 13] 요청된 커밋 준비 - PASS: 현재 대화의 명시적 승인 범위와 후보 제외 규칙을 확인한다. 실제 staged diff에 근거한 공개 변경만 한 논리 단위로 커밋하며 별도 문서 변경/로컬 자료는 포함하지 않는다.

## 동기화와 다음 작업

2026-10-25 연결 노트에 C-02·D-02~D-05·D-08·D-11~D-13의 구현 계약과 후속 경계를 추가하고 양쪽 내용을 재읽었다. 기존 노트의 메타데이터·분석·백로그·개인 변경은 추가 블록 제거 시 바이트가 일치함을 확인했다. 노트와 로컬 근거는 공개 커밋에 포함하지 않는다. 이 동기화에 남은 미반영 항목은 없다.

다음은 T-012 원기록 보존 취소/정정·판매 재배분·실사/포함 연결의 유효 개정과 영향 미리보기다. 실제 반품은 T-013에서 T-010의 남은 기준과 함께 확인한다. T-022 이후 백로그와 전체 원자료 이관은 이번 실행 범위에 추가하지 않는다. 필수 외부 환경 검증이 남아 있으므로 T-021·배포 준비·goal은 완료가 아니다.

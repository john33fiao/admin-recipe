# T-009 구현 계획과 RTD Before

- 작성일: 2026-10-25
- 상태: 판매 구현·검증 완료 — 실제 실사 후 배분 표시는 T-010 후속
- 기준: [티켓](T-009-sales-consumption.md), [수량·원가 계약](../DOMAIN-CONTRACT.md), [요구사항](../REQUIREMENTS.md), [ADR-007](../decisions/ADR-007-consumption-and-ticket-scope.md)
- 선행: T-004·T-008 구현·검증 완료. T-005 공통 초안/확정 기반을 재사용한다.

## 1. 목표와 범위

메뉴 한 건의 판매 개수 입력 → DB 초안 저장/재시작/수정 → 미리보기 → 확정 → 목록/상세 → 개수 오입력 정정을 완성한다. 자동 입력 시각과 당시 품목/레시피/판매가·원가 근거를 보존한다. 판매는 재고 수량·가치를 변경하지 않는다.

실제 실사 배분 writer는 T-010, 일반 정정과 이미 배분된 소모의 영향 처리는 T-012에서 완성한다. T-009는 직접 참조와 유효 판매 개수를 제공하는 typed 계약 및 합성 구간 검증까지 수행한다. POS·환불·결제·할인·세금·일자 마감·예상 재고·일괄 판매·전체 자료 이관은 추가하지 않는다.

## 2. 입력·공통 업무 계약

기존 `BusinessKind/Payload/Effect/Result`에 `sale`을 추가한다. POST는 기존 16KiB 공통 business 경로를 사용한다. 입력은 `kind: "sale", version: 1`이며 `DraftSaleV1`은 nullable `material_id/material_version_id/recipe_version_id`, 문자열 `count/note`를 갖는다. 개수 원문은 80자, 메모/정정 이유는 2000자이며 UUID·줄바꿈·제어문자 정규화는 기존 규칙을 따른다. 미선택·빈칸은 초안에 보존한다.

확정 개수는 ASCII 십진 정수 0~1,000,000,000이다. JSON 숫자·음수·소수·지수·상한 초과와 0을 구분한다. 확정 시 신규 판매의 선택 품목 버전과 recipe head가 각각 현재인지 확인한다. recipe 소유 stable 품목은 같아야 하지만 레시피 작성 당시 owner 버전과 현재 판매 owner 버전은 달라도 정상이다. 실제 레시피 없이 버전을 만들어내지 않으며 등록을 안내한다. 빈 구성·수량 0 레시피와 재고/재료 가격 없음은 판매 차단 사유가 아니다. 품목 종류에 새로운 판매 제한을 추가하지 않는다.

공통 epoch/revision·head CAS·UUID 요청 재생·DB 초안·pending 원문 보존을 사용한다. 미리보기 이후 기준이 바뀌면 입력을 보존하고 새 미리보기를 요구한다. 현재 가격 조회의 늦은 응답으로 기존 선택이나 pending 요청을 교체하지 않는다.

## 3. 판매 당시 계산 고정

판매가는 선택한 recipe version의 `selling_price`다. 판매금액은 checked 정수 연산으로 판매가×개수를 계산한다. 판매가가 null이면 0개여도 금액 null, 유효한 0원 또는 개수 0이면 금액 0이다. 합계는 알려진 금액 소계와 미계산 건수를 구분하며 현재 유효 개정만 한 번 집계한다.

목록의 누적 조회 합계는 개별 판매금액의 허용 범위와 구분한다. 각각 유효한 여러 판매의 합이 개별 Money 상한을 넘더라도 조회를 막지 않도록 checked i128 합산과 정확한 십진 문자열을 사용한다. 각 판매 입력/확정의 Money 상한은 유지한다.

레시피 원가는 기존 `cost_recipe`와 단가 resolver를 재사용한다. `Store::recipe_cost`의 현재 단가 계산을 작은 내부 helper로 분리하고 확정 트랜잭션에서 당시 결과를 고정한다. 과거 판매 조회에서 현재 원가 API를 다시 호출하지 않는다. 직접 행별 분자 수량/basis_count와 단가 출처·분자/분모, 1개분 행 원가/알려진 소계/전체 원가/원가율을 보존한다. 행마다 한 번 반올림한 기존 계산 계약을 따른다. 0수량·가격 없음은 원가 0과 missing 출처를 함께 보존하고 양수·가격 없음은 원가 null이다.

새 `SaleRecipeCost`는 당시 결과만 갖고 현재 state/price_mode를 포함하지 않는다. 중첩 `SalePriceSnapshot`은 holding_average/last_valid/preregistered/missing의 닫힌 출처와 source_id/amount/quantity/reason을 가진 엄격 타입이다. 판매 원가 행은 당시 component 품목명도 보존해 조회 시 현재 이름으로 바뀌지 않게 한다. 명칭은 불변 품목 버전에서 가져오며 별도 변경 가능한 중복 원장을 만들지 않는다.

## 4. 불변 원본·개정과 저장

schema8에서 schema9로 확장하고 JSON format2는 유지한다. 최소 관계는 다음과 같다.

| 관계 | 내용 |
| --- | --- |
| sales | 불변 판매 원본, 최초 confirmation/업무, 당시 owner/recipe·원가 요약, 최초 입력 시각/seq |
| sale_cost_rows | 원본 레시피의 모든 직접 행과 1:1 대응하는 당시 단가 출처·정확 비율·행 원가 |
| sale_versions | 개수·금액·이유·새 시각/seq, 같은 판매의 이전 version과 confirmation |
| sale_heads | stable sale별 현재 유효 version 하나 |

최초 원본/행/개정/head 또는 정정 version/head, 공통 confirmation/correction, 요청 원응답과 outbox는 같은 mutation transaction에서 저장한다. 입고/보유/생산 원장에는 쓰지 않는다. 직접 재료는 원레시피 행에서 DISTINCT로 도출하며 별도 중복 테이블을 만들지 않는다.

source schema8은 기존 네 업무 종류만 허용하는 frozen8 reader로 먼저 검증한다. schema3~7 reader·원요청·응답·JSON 바이트는 유지한다. migration은 기존 보유/레시피/생산 행을 재작성하지 않는다. 최신 writer와 과거 reader가 live sale variant를 공유해 과거 형식을 넓히지 않는다.

schema9 snapshot은 원본/버전/head/업무·정정 관계, UUID·canonical seq·RFC3339 시각, 원본보다 앞선 owner/recipe/가격 근거와 모든 직접 행을 검증한다. 당시 직전의 가격 선택 우선순위 및 보존된 산술을 검사하고 현재 head로 과거 결과를 덮어쓰지 않는다. 누락/미래 참조·중복 confirmation·관계 변조는 복원이나 쓰기 전에 거절한다.

## 5. 개수 오입력 정정

기존 `corrects_confirmation_id`로 현재 유효한 판매 confirmation을 지정한다. 같은 stable sale에 새 version을 추가하고 원본의 품목/recipe/판매가/단가·원가를 그대로 사용한다. 수정 가능 입력은 개수와 이유다. 미완성 사유는 초안에 남길 수 있으나 정정 확정에는 이유가 필요하다. 다른 판매/업무·이미 대체된 대상과 참조 변경을 거절한다.

정정 개수가 0이어도 원본은 남는다. 고객 환불·실물 반환·재고 회복은 만들지 않는다. 원래 계산 위치는 최초 판매 seq이고 정정의 새 seq로 구간을 옮기지 않는다. 상세에서 원본 시각과 각 개정 시각, 이전/새 개수·금액을 구분한다. 공통 가격 정정은 기존 동작을 보존하며 다른 업무 정정에 권한을 넓히지 않는다.

## 6. T-010 연결 계약

`allocation_inputs(material_id, after_seq, before_seq, as_of_revision)`은 원본 판매 seq가 열린 구간에 있고 당시 recipe가 해당 stable 품목을 직접 참조한 판매를 한 번씩 제공한다. 같은 품목 중복 행과 수량 0을 구분하며 하위 레시피를 재귀 전개하지 않는다. 개수는 as_of_revision 이하의 마지막 유효 sale version에서 얻는다. 구간의 원본 순번과 개정을 선택하는 시점은 별개다.

순수 `SaleConsumptionSource`는 sale/version/recipe ID, 원본 seq, 개수와 직접 품목 ID 목록을 갖는다. Store는 해당 시점의 입력을 선택하고 순수 함수는 구간/중복/직접 참조를 검증한다. 잔여 동률은 최초 seq와 stable ID로 결정한다. 합성 A3/B1의 300/100 배분, 전체 개수 0의 미배분, 정정 후 1:1과 C/K/Q/V 불변을 검사한다. 합성 결과를 실사 DB나 API에 적재하지 않는다.

T-009의 모든 판매는 소모 미확정이다. 단일 boolean consumed 대신 이후 품목별 배분 연결이 가능하도록 두며, 실사 전의 null을 0소모/0원가로 표시하지 않는다. T-010에서 실사 writer를 추가할 때 기존 정정 경로가 확정 배분과 불일치하지 않도록 배분 영향 처리를 원자 연결하거나 미완성 상태를 명시적으로 차단해야 한다. 실제 연결은 T-010/T-012의 검증 항목이다.

## 7. DTO·조회와 화면

`SaleOriginal`은 id/original_confirmation_id/business_record_id/material_id/material_version_id/material_name/recipe_version_id/recipe_cost/recorded_at/seq, `SaleVersion`은 id/sale_id/previous_version_id/confirmation_id/business_record_id/count/amount/note/recorded_at/seq를 갖는다. 수량·금액·seq는 문자열이다. `SaleProposal`은 미래 ID·시각 없이 당시 참조/이름/count/amount/recipe_cost를 제공한다.

`BusinessResult::Sale`은 sale_id/sale_version_id, `BusinessEffect::Sale`은 sale proposal/corrects_confirmation_id/previous version을 제공한다. `SaleList`는 items/known_revenue/missing_revenue_count/state, `SaleDetail`은 item/head/versions/consumption_status/state를 반환한다. 상태는 현재 `unconfirmed`이고 미구현 실사 상태를 반환하지 않는다. 건수 보조 필드는 JSON 정수, 업무 개수는 문자열이다.

GET `/api/sales` 및 `?material_id`, `/api/sales/{id}`, SPA `/sales`와 `/sales/{id}`를 추가한다. 입력은 `/business?kind=sale`과 기존 업무 상세를 사용한다. 메뉴 선택·개수·메모와 정정의 고정 참조를 구분한다. 목록/상세에서 ‘레시피 기준 원가(판매 당시)’와 ‘실사 소모 미확정’을 따로 표시하고 당시 레시피 링크를 제공한다. 늦은 참조 조회·화면 이탈/재진입·불확실 요청 복구는 공통 보호를 따른다. 기본·어두운 테마, 1440/768/390 화면과 실제 키보드를 검증한다.

## 8. 실행할 검증과 변경 범위

제품 변경 전에 정상 컴파일 상태에서 sale 입력/조회의 미지원 행동과 schema8→9 기대 실패를 기록한다. import나 컴파일 실패를 행동 RED로 집계하지 않는다. 이전 release의 정상 API로 가상 schema8 fixture를 만들고 원요청/원응답/JSON 보존을 확인한다.

| 수용 기준 | 검증 |
| --- | --- |
| 01 개수·자동 시각 | 0/상한/불법 개수, DB 초안 재시작/수정/확정, 자동 시각·seq |
| 02 재고 무변경 | 미확인·빈·부족 재고/0구성/없는 가격 판매, 전후 보유·입고·생산·사전가격 불변 |
| 03 당시 금액/원가 | null/0/양수 판매가, 정확 분수 원가, 이후 이름/recipe/가격/입고/생산 변경과 재시작 뒤 원본 불변 |
| 04 직접 참조 | 중복/0행/비재귀, 열린 구간과 as-of 개정 선택, 합성 배분 |
| 05 소모 상태 구분 | 실제 미확정 UI/API, 당시 레시피 원가와 실사 배분 값의 분리 |
| 06 정정 연결 | 3→1→0, 이유 필수·고정 참조·현재 대상 CAS, 원본 seq 유지·재시도·환불/재고 변화 없음 |

쓰기 단계별 rollback·JSON 발행실패 복구·동일 UUID 원응답·다른 원문 충돌, schema8 변환/동결/변조 거절, 세션·Host·Origin·16KiB 경계와 기존 업무 회귀를 검사한다. T-005의 실제 판매 초안 보존/수정 조건도 함께 검증한다.

예상 파일은 sale 순수/저장 모듈과 tests, business/server/lib 연결, recipe 현재 원가 helper, snapshot/frozen8·가상 fixture, 프런트엔드 sale 편집/조회/타입/공통 상태와 검증이다. README·ARCHITECTURE·TODO·색인·티켓/검증 기록을 실제 결과로 갱신하고 저장 설계는 연결 노트와 동기화한다.

실제 명령은 `npm --prefix frontend test`, `npm --prefix frontend run typecheck`, `node scripts/build.mjs`, `cargo test --locked`, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `npm --prefix frontend run test:e2e`, manifest 자체/현재 정합성·release 경로 검사다. frontend 변경 후 반드시 정식 build로 생성한 뒤 Cargo/브라우저를 실행한다. 변경 파일 재읽기·상대 링크·UTF-8/LF·diff·ignore도 확인한다. 사용 DB를 자동 다운그레이드하지 않으며 필요하면 변경 전 백업과 호환 버전으로 복원한다.

## RTD Before

[Step 1] 변경 계획 - PASS: 수용 기준 여섯 개를 단일 판매 입력/저장/조회·개수 정정과 실제 검증에 대응했다. 실제 배분/일반 정정의 후속 경계를 명시했다.

[Step 2] 계획 검토 - PASS: 기존 공개 변경은 T-008까지 검증·커밋되어 분리돼 있다. 불변 가격/recipe, null과0, 원본seq와 개정seq, strict/frozen reader, 공통 CAS·요청 복구·공개/비공개 경계를 확인했다.

[Step 3] 검토 재검토 - PASS: 설계가 달라질 가정은 한 업무에 메뉴 하나, 개수 정정은 당시 참조 고정, schema8→9의 세 가지다. 확정 운영 정책 안의 최소 기술 범위로 채택하며 새 정책 질문은 현재 없다. 독립 설계 검토 후 행동 RED를 수행한다.

[Step 4] 계획 단순화 - PASS: 공통 업무와 기존 원가/배분 수치 함수를 재사용한다. 재귀 재료 전개·가짜 실사 API·판매 전용 대용량 경로·범용 정정 프레임워크를 추가하지 않는다.

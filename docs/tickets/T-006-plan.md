# T-006 발주 내역과 미입고 주문량 구현 계획

- 상태: 구현·검증 완료 — 실행 근거와 RTD After는 T-006-verification.md
- 기준 커밋: `708afc7`
- 실제 운영 자료를 사용하지 않으며 Git 작업은 통합 담당자가 수행한다.

**목적:** 주문 초안→DB 재시작→확정→미입고 조회→같은 품목의 주문 수정·취소를 실제 저장/API/UI로 구현하고, 한 번의 실제 입고로 종료하는 계약은 typed 가상 결과로 검증한다.

**구조:** T-005의 business 초안·확정·공통 mutation과 JSON 기록을 재사용한다. 주문에는 안정적인 order ID와 별도 불변 버전/현재 head를 두며 주문 진행 상태를 business 확정 상태와 분리한다. 가격 JSON 외형과 schema 4 reader를 고정한 채 schema 5의 주문 관계만 추가한다.

**기술:** 기존 Rust·SQLite·Axum·Nuxt/Vue·TypeScript. 새 외부 의존성/프레임워크 없음.

**기준:** [T-006](T-006-purchase-orders.md), [T-007](T-007-receipts.md), [요구사항](../REQUIREMENTS.md)의 발주·입고, [DOMAIN](../DOMAIN-CONTRACT.md) §1~5·9, [ADR-007](../decisions/ADR-007-consumption-and-ticket-scope.md) P-05/P-06, [T-005 계획](T-005-plan.md).

2026-10-24 작성. T-005 공통 기반의 Rust73·브라우저24 시나리오 등 검증과 RTD를 마친 커밋을 기준으로 한다. T-005의 실제 판매 통합은 T-009에 남으며 주문 구현의 선행 저장 경로는 준비됐다. 이 계획의 작성은 T-006 제품 검증 성공을 뜻하지 않는다.

## 공통 제약

- Rust 1.98 이상, Node `^22.22.3 || ^24.15.0 || >=26.0.0`, 기존 lockfile·MIT/제3자 고지 유지.
- 실제 주문 수량은 양수. 수량은 기준 단위 `0`~`9,000,000,000`, 소수 최대 6자리이며 내부 최대 `9,000,000,000,000,000` micro 단위다.
- 예상 금액은 없음 또는 `0` 이상, 최대 `9,000,000,000,000`원·소수 최대 6자리다. 빈 값과 `"0"`을 구별한다. 수량·금액·seq는 JSON 십진 문자열이다.
- g/kg·mL/L·동일 자유 단위만 환산한다. 주문·수정·취소는 보유 수량·가치·사전 가격을 바꾸지 않는다.
- 미입고 수정은 **같은 품목**의 수량/호환 표시 단위·거래처명·예정일·예상 금액·메모만 지원한다. 품목 교체, 발주서·외부 전송·거래처 마스터·결제·부분 입고는 범위 밖이다.
- place/revise/cancel의 `note`는 모두 선택 메모다. 미입고 주문 수정은 T-012 일반 오입력 정정과 구분하며 새 필수 이유 제한을 만들지 않는다. T-005 사전 가격 정정의 필수 note는 그대로 유지한다.
- 한 번의 실제 입고로 종료하며 잔여량을 다음 미입고로 만들지 않는다. T-006에는 실제 입고 쓰기 API/버튼·재고 원장이 없다. T-007에서 실제 입고·종료를 같은 tx로 연결한다.
- 기존 공통 UUID 공간·최초 응답·recover 게이트·epoch/revision CAS·Host/Origin/세션·16KiB·종료 대기를 유지한다. 공개 자료와 모든 테스트에는 가상 데이터만 사용한다.
- frontend 파일 변경 뒤 Cargo/E2E 전 `node scripts/build.mjs`를 실행한다. 자산 guard 우회나 산출물 검증 기록 수동 갱신은 금지한다. 담당자는 커밋하지 않는다.

## 주요 검토점

1. 수정 초안을 열어 둔 사이 다른 탭이 같은 주문을 수정·취소하면 이전 head로 확정하지 않는다(P05/H03/U03).
2. 새 주문/수정/취소의 응답을 잃고 다시 접속해도 같은 UUID·본문을 재생하며 주문/이력을 한 번만 만든다(P06/U04).
3. 기존 가격 초안·확정·정정·pending 원문이 typed enum 확장 때문에 변형되지 않는다(D01/P09/U05).
4. 선택 금액 없음/0, 미완성 날짜·숫자, 단위 환산 상한과 다른 자유 단위를 조용히 보정하지 않는다(D02/P02/H02).
5. 가상 입고 종료가 수정 전 주문량을 기준으로 차이를 계산하거나 실제 재고 적용으로 보고되지 않는다(D03/D04).

## 1. RTD Before 4개

- [Step 1] 변경 계획 - PASS: 실제 주문 작성·수정·취소 및 조회를 구현하고, 입고 종료는 순수 계약으로 검증한다. 파일/DTO/실행 순서/수용 기준을 아래에 고정했다. schema 4→5 변경은 기존 일관된 백업과 단일 migration tx를 사용한다.
- [Step 2] 계획 검토 - PASS: 기존 catalog_price 외형·원응답·원 JSON, 미완성 초안, 엄격 입력, 실패 원자성, 숫자 문자열, 요청 재시도와 접근 보호를 보존한다. schema 4에는 새 kind를 허용하지 않는다. 실제 입고 미연결은 T-007로 명시한다.
- [Step 3] 검토 재검토 - PASS: 가정은 ① T-005 최종 schema 4/공통 helper가 유지됨 ② 주문 수정은 현재 open 버전의 같은 품목만 대상으로 함 ③ 실제 receipt가 아직 없어 종료는 합성 결과만 사용함이다. ①이 달라지면 기준 fixture/API를 갱신하고, ②·③은 현재 지시와 티켓 범위에 근거한다.
- [Step 4] 계획 단순화 - PASS: 공통 주문서 헤더/여러 품목 행, supplier 마스터, 새 쓰기 endpoint, 범용 handler registry, 예상 재고, 선행 receipt 테이블을 추가하지 않는다. 품목 1개당 주문 1건과 기존 business 명령으로 완료한다.

현재 새 운영 질문은 없다. 루트가 확인한 기술 선택: 주문 자체의 불변 버전/현재 head/confirmation 참조로 수정·취소를 연결하고 `purchase_order.corrects_confirmation_id=null`을 유지한다. 사전 메모의 주문 수정에 business correction edge를 직접 재사용한다는 안을 이 최소안으로 구체화했다. **미입고 주문 내용 수정(T-006)**과 **이미 발생한 실제 업무 오입력의 일반 정정(T-012)**은 다르다.

## 2. 닫힌 payload와 기존 가격 호환

`src/business.rs`의 공개 Rust 표현은 kind로 구별하는 닫힌 enum으로 바꾼다. wire는 계속 `{kind,version,data}`이며 `catalog_price`의 필드/널/문자열/순서는 직렬화 회귀로 고정한다. 각 variant의 version은 정확히 1이고 모든 envelope/variant/data/action에서 unknown field를 거절한다. 내부 tagged enum 또는 작은 엄격 wire DTO를 사용하되 `flatten + deny_unknown_fields`에 의존하지 않는다. `Value`를 영속 typed payload 대신 보관하지 않는다.

```text
BusinessPayload =
  { kind:"catalog_price", version:1, data:DraftPriceV1 }       // T-005 외형 그대로
| { kind:"purchase_order", version:1, data:DraftOrderActionV1 }

DraftOrderActionV1 =
  { action:"place", material_id:string|null, material_version_id:string|null,
    quantity:string, unit:string, supplier:string, expected_date:string,
    estimated_amount:string, note:string }
| { action:"revise", order_id:string, expected_order_version_id:string,
    material_version_id:string|null, quantity:string, unit:string, supplier:string,
    expected_date:string, estimated_amount:string, note:string }
| { action:"cancel", order_id:string, expected_order_version_id:string, note:string }
```

revise에는 바꿀 수 있는 material_id 필드가 없다. 현재 주문의 material_id를 사용하며 선택한 최신 material_version_id가 그 품목에 속하는지 검사한다. cancel은 품목/수량/가격 필드가 없고 현재 주문 내용을 보존한 상태 전이만 만든다. 기존 `CreateBusinessDraft/ReviseBusinessDraft/PreviewBusiness/ConfirmBusiness/ArchiveBusiness` envelope와 URL은 유지한다. purchase_order의 non-null `corrects_confirmation_id`는 구조적으로 읽힌 뒤 400 validation으로 거절한다.

초안은 미완성 숫자/날짜를 문자열 그대로 보관한다. UUID non-null 값은 문법 검사·정규화하고 없는 참조/오래된 주문 head/미완성 업무 값은 저장 초안의 필드 issue로 표시한다. 주문 record의 kind/action/대상 order_id는 첫 저장 뒤 바꾸지 않는다. revise/cancel의 expected_order_version_id는 최신 조회 후 사용자 재확인에 따라 새 draft revision에 갱신할 수 있다. place의 선택 품목은 미확정 초안 동안 바꿀 수 있다. 이런 변경은 다른 주문/명령으로 조용히 전환하는 경로가 아니다.

길이 제한은 quantity/estimated_amount 80자, unit 20자, supplier 200자, expected_date 32자, note 2000자다. note의 CRLF/CR은 LF로 정규화하며 줄바꿈 외 제어문자를 거절한다. 나머지 문자열은 제어문자를 거절한다. 실제 값 정규화는 확정 필드에서 수행하고 raw draft/history는 보존한다.

확정 검증: 현재 품목 버전·같은 단위 차원·양수/범위·선택 금액을 검사한다. supplier/expected_date/estimated_amount는 trim 후 빈 값이면 확정 DTO의 null이다. 날짜는 연도 0001~9999의 실제 달력 날짜이며 정확한 `YYYY-MM-DD` 형태만 허용한다. 날짜를 UTC timestamp/입고일로 바꾸거나 오늘 이후만 허용하지 않는다. revise.note/cancel.note는 빈 값도 유효하며 메모 유무와 무관하게 원본·버전·변경 전후를 보존한다. 품목 아카이브 자체를 새 운영 차단 조건으로 추가하지 않으며 참조 선택 UI는 기존 품목의 조회/선택 정책을 따른다.

`BusinessResult`와 `BusinessEffect`도 kind별 닫힌 enum으로 바꾼다. 기존 catalog_price 직렬화는 완전히 동일하다.

```text
OrderBusinessResult = { kind:"purchase_order", action:"place"|"revise"|"cancel",
  order_id, order_version_id, material_id, material_version_id }
OrderBusinessEffect = { kind:"purchase_order", action, order_id:string|null,
  before:OrderVersion|null, after:OrderProposal }
OrderProposal = { status:"open"|"cancelled", material_id, material_version_id,
  quantity, unit, input_quantity, input_unit, supplier:string|null,
  expected_date:string|null, estimated_amount:string|null, note:string }
```

preview는 UUID/seq를 미리 만들지 않는다. place의 order_id/before는 null이며 확정 때 생성한다. revise는 현재 주문→새 내용, cancel은 현재 주문→cancelled를 표시한다. 모든 수량/금액은 십진 문자열이다. BusinessSummary.material_name/issue_count, payload 복사·동일성·pending 복구·화면 kind 분기를 함께 확장한다. 가격 전용 `.data.material_id` 접근을 공통 경로에 남기지 않는다.

## 3. 실제 주문 DTO·조회·상태

```text
OrderStatus = "open" | "received" | "cancelled"
OrderFilter = { status:"open"|"received"|"cancelled"|"all" } // 기본 open, unknown 거절
OrderVersion = { id, order_id, previous_version_id:string|null, confirmation_id,
  business_record_id, action:"place"|"revise"|"cancel", status,
  material_id, material_version_id, material_name,
  quantity, unit, input_quantity, input_unit, supplier:string|null,
  expected_date:string|null, estimated_amount:string|null, note, recorded_at, seq }
OrderSummary = { id, head_id, status, material_id, material_version_id, material_name,
  quantity, unit, input_quantity, input_unit, supplier, expected_date,
  estimated_amount, created_at, updated_at }
OrderList = { items:OrderSummary[], state:CatalogState }
OrderDetail = { item:OrderSummary, head:OrderVersion, versions:OrderVersion[],
  state:CatalogState }
```

`quantity/unit`는 기준 단위, `input_quantity/input_unit`는 확정 당시 입력 단위다. raw 초안은 business revision으로 조회한다. order versions는 seq순이며 각 confirmation_id/business_record_id를 통해 업무 확정 상세를 연결한다. material_name은 해당 버전이 참조한 당시 품목 이름이다. business_record_id/material_name은 DB 중복 문자열 저장 없이 같은 Store 조회에서 관계로 만든 필드다.

T-006 writer가 만드는 실제 상태는 open/cancelled뿐이다. `received` 조회는 빈 배열이며 종료 생성 API는 없다. schema 5 reader/DB 검증은 received 행을 입고 연결 없는 정상 주문으로 묵인하지 않는다. T-007에서 실입고 참조와 received 저장 구조를 확장한다. 이 API 필터가 있다는 사실을 실제 입고 구현으로 보고하지 않는다.

| 동작 | 사전 조건 | 결과 |
| --- | --- | --- |
| place 확정 | 유효한 입력 | 새 order UUID, 최초 version, open |
| revise 확정 | 대상 open·현재 order head 일치·같은 품목 | 같은 order UUID, 이전 version 참조한 새 open version |
| cancel 확정 | 대상 open·현재 order head 일치 | 같은 order UUID, 기존 내용 복사한 cancelled version |
| business archive | 미확정 business draft | draft만 archived; 기존 실제 주문은 불변 |
| received/cancelled 대상 수정·취소 | 닫힌 상태 | 효과 없는 409 stale_state, 최신 주문 안내 |

이미 확정한 business record를 수정/아카이브하여 주문을 취소하지 않는다. 새 revise/cancel business draft→preview→confirm을 만든다. 각 business confirmation은 불변이며 OrderDetail 전체 이력이 주문의 진행 상태를 설명한다. 일반 가격 정정 chain은 그대로 유지한다.

Store 공개 추가 메서드는 아래 두 개이며 쓰기는 기존 business 7개 메서드만 확장한다.

```rust
pub fn order_list(&self, filter: OrderFilter) -> Result<OrderList, StoreError>;
pub fn order_detail(&self, id: &str) -> Result<Option<OrderDetail>, StoreError>;
```

`GET /api/orders?status=...`는 200 OrderList, `GET /api/orders/{id}`는 200/404 not_found다. 잘못된 UUID/필터는 400 validation이다. `/orders`, `/orders/<UUID>`의 GET/HEAD SPA만 추가 허용하며 unknown API/경로/자산은 기존 404를 유지한다. HTTP body/호출자 보호와 공통 오류 code는 변경하지 않는다.

## 4. schema 5·같은 tx·frozen schema 4

최소 관계는 `purchase_orders`, `purchase_order_versions`, `purchase_order_heads`다. orders는 ID·최초 confirmation·생성 시각/seq, versions는 위 확정 필드·직전 version·confirmation·seq, heads는 order ID→현재 version을 담는다. versions의 confirmation_id는 UNIQUE FK이며 `(order_id,id)` UNIQUE, previous/head는 같은 order의 복합 FK다. material_id/version은 기존 품목 버전 복합 FK, seq는 outbox FK다. orders의 최초 confirmation도 FK이며 business confirmation과 연결 시 필요한 지연 제약은 단일 tx 안에서 검증한다. 수량/금액은 기존 micro INTEGER CHECK, 예상금액 없음은 NULL이다. action/status의 허용 조합은 place/open, revise/open, cancel/cancelled다. received 행/가짜 receipt FK/재고·미입고 잔액 테이블을 선행 생성하지 않는다.

business confirmation 생성→kind별 실제 효과→결과 참조→공통 응답/outbox 순서는 모두 같은 mutate tx 안에서 수행한다. 필요한 경우 confirmation ID를 handler 호출 전에 만들지만 tx 밖에 노출하지 않는다. 기존 catalog_price 흐름의 원자성과 최초 응답 외형을 회귀로 고정한다. 일반 registry/동적 SQL 대신 닫힌 match와 작은 handler 함수를 쓴다.

새 `src/store/purchase_order.rs` 내부 인터페이스:

```text
inspect_order(connection, record, payload)
  -> Result<(Vec<BusinessIssue>, Option<OrderBusinessEffect>), StoreError>
apply_order(tx, payload, confirmation_id, seq, recorded_at)
  -> Result<OrderBusinessResult, StoreError>
order_snapshot_rows(connection) -> Result<OrderSnapshotRows, StoreError>
migrate_orders(transaction) -> Result<(), Error>
validate_order_schema(connection) -> Result<(), Error>
```

confirm은 전역 epoch/revision·business head·order head를 tx 안에서 다시 검사한다. place/revise는 현재 품목 참조도 검사한다. cancel은 현재 주문 버전의 품목/수량을 그대로 보존하며 최신 품목 버전을 다시 선택하도록 요구하지 않는다. 입력 검증과 preview가 계산한 결과를 같은 함수로 확인하되 미리보기 숫자를 신뢰해 그대로 SQL에 넣지 않는다. 같은 요청 재생은 상태 검사보다 먼저 원응답을 반환하므로 이미 취소된 주문의 취소 재시도도 성공 당시 UUID/seq 그대로다. 다른 UUID의 중복 취소는 stale_state다. 미완성 초안 저장은 order version을 만들지 않는다.

`SCHEMA_VERSION=5`, writer는 계속 format 2다. schema 4 reader에 현재 확장 BusinessPayload를 그대로 사용하지 말고 **catalog_price만 아는 frozen payload/record/revision/result DTO**를 둔다. schema 3도 그대로 유지한다. schema 5에는 기존 전체 상태와 `purchase_orders/purchase_order_versions/purchase_order_heads`가 필수다. 현재 typed business payload는 schema 5에서만 사용한다. schema 4에 purchase_order가 들어 있으면 알 수 있는 새 kind라도 거절한다. schema 5에서도 알 수 없는 action/version/field·컬렉션 누락을 거절한다.

4→5 migration은 source_schema=4로 옛 outbox 전체를 검증한 뒤 테이블만 추가하며 새 seq·UUID·업무/주문을 만들지 않는다. 백업·DDL·검증은 기존 단일 tx 경로를 사용한다. 구 요청 payload/response 문자열과 JSON bytes는 변형하지 않는다. 새 writer는 가격/주문/초안 어느 변경에도 주문 컬렉션을 포함한다. 전체 복원 엔진은 T-018이며 이번 reader는 임의 JSON을 DB에 적재하는 복원 API가 아니다.

fixture는 실제 T-005 형식의 가상 가격 초안→확정→정정과 원요청 응답을 포함한 `schema-v4.sql`, `snapshot-v2-schema4.json`을 추가한다. FK fixture는 참조 행을 모두 채우는 단일 tx로 만들고 commit 때 검증한다. 잘못된 JSON 테스트는 DB outbox와 파일을 **같은 손상 bytes**로 맞춘 뒤 before를 캡처하여 단순 파일 불일치가 parser 실패를 가리지 않게 한다.

## 5. exact typed 입고 결과와 순수 종료 계약

새 `src/purchase_order.rs`에서 기존 Quantity/Unit·UUID를 사용한다. HTTP JSON이나 SQLite를 받지 않는 내부 타입이며 생성자에서 양수·상한·단위 구조를 검증한다. 생성자는 DB 참조 존재를 보증한다고 명명하지 않는다. 실제 참조 존재/입고 확정 사실은 T-007 adapter 책임이다.

```text
ReceiptResult::new(receipt_id:Uuid, confirmation_id:Uuid, order_id:Uuid,
  expected_order_version_id:Uuid, material_id:Uuid, material_version_id:Uuid,
  quantity:Quantity, input_unit:Unit) -> Result<ReceiptResult, DomainError>

OrderForReceipt = { order_id:Uuid, state:OrderStatus, material_id:Uuid,
  initial_order_version_id:Uuid, initial_quantity:Quantity,
  current_order_version_id:Uuid, ordered_quantity:Quantity, canonical_unit:Unit }

close_order(order:&OrderForReceipt, receipt:&ReceiptResult)
  -> Result<OrderReceiptClosure, DomainError>

OrderReceiptClosure = { status:"received", order_id, initial_order_version_id,
  initial_ordered_quantity, ordered_version_id, ordered_quantity,
  receipt_id, receipt_confirmation_id, receipt_material_version_id,
  received_quantity, difference, unit }
```

ReceiptResult 필드는 private이고 검사된 getter만 제공한다. order 입력 수량은 기준 단위이며 OrderForReceipt 생성자도 일치하는 기준 Unit·양수 범위를 검사한다. received/cancelled 상태를 포함한 입력은 함수가 닫힌 주문을 거절하는 검증에 사용한다. input_unit을 기준으로 quantity를 정확히 환산한 뒤 품목·order ID·expected current version·차원·양수/상한을 검사한다. 다른 표시 단위 kg/g는 허용하고 g/mL, 다른 자유 단위는 거절한다. receipt의 material_version_id는 과거 주문 버전과 달라도 같은 품목·호환 단위면 가능하며 실제 소속 확인은 T-007의 DB adapter에서 수행한다.

`difference = received_quantity - ordered_quantity`는 검사된 정수 뺄셈으로 계산한다. 값 범위는 ±9,000,000,000,000,000 micro 안이며 Quantity의 음수 금지를 풀지 않는다. 별도 파생 차이 타입/함수에서 정확한 문자열을 만든다. JSON은 양수 `"200"`, 음수 `"-200"`, 0 `"0"`; UI 표시에서만 `+200`을 사용할 수 있다. 수량·차이·seq를 float/Number로 계산하지 않는다.

10kg→9.8kg은 ordered `"10000"`, received `"9800"`, difference `"-200"`, unit g, received다. 초과 10.2kg은 `"200"`이며 주문량으로 자르지 않는다. 10kg→수정12kg→11.8kg 입고는 **현재 확정 주문 12kg 대비 -200g**이며 최초10kg/최초 version도 별도 필드와 기존 이력으로 보존한다. 입고 부족을 잔여 주문/새 주문으로 만들지 않는다. 구입금액·예상금액은 이 함수에 전달하지 않고 재고 가치도 계산하지 않는다.

T-006 검증은 실제 임시 Store에서 만든 주문 head와 코드로 생성한 가상 ReceiptResult를 순수 함수에 전달한다. 결과를 production DB/API에 쓰지 않고 반환 구조·직렬화 계약을 검사한다. 한 번 받은 closure로 received 상태를 구성해 두 번째 close가 실패하는 것을 검증한다. 실제 동일 요청 재생과 SQL receipt FK·주문 head 갱신·입고 Q/V의 단일 tx는 T-007에서 검증한다. T-006 종료 테스트 통과를 실제 입고 저장 성공으로 표시하지 않는다.

## 6. 파일 소유·작업별 RED→GREEN

| 담당 | 소유 파일 | 입력/산출 |
| --- | --- | --- |
| 루트 공통 DTO | 수정 `src/business.rs`, `src/lib.rs`, `tests/business_payload.rs` | kind/result/effect와 기존 가격 외형 고정. 구조 검증 helper·모듈 등록과 공통 계약 통합 |
| A 저장 | 새 `src/store/purchase_order.rs`, `tests/purchase_order_storage.rs`, schema4 fixtures; 수정 store.rs/business.rs/snapshot.rs 및 필요한 기존 저장 회귀 | 주문 상태·공통 dispatch·schema5/frozen4·동일 tx. mutation helper 외형 변경은 최소화 |
| B HTTP·주문 계약 | 수정 `src/server.rs`; 새 `src/purchase_order.rs`, `tests/purchase_order.rs`, `tests/purchase_order_http.rs`, 필요한 기존 business HTTP 회귀 | GET 2경로·SPA와 주문 DTO/구조 검증·ReceiptResult/close_order. lib.rs/Store 편집 제외 |
| C UI | 새 `types/purchase-order.ts`, `utils/purchase-order.ts`, `components/BusinessOrderEditor.vue`, `pages/orders/index.vue`, `pages/orders/[id].vue`, 관련 unit/E2E; 수정 기존 business types/utils/composable/pages와 app.vue/main.css | 기존 가격 입력은 유지하고 주문 입력/수정/취소·목록/이력 연결 |
| 루트 통합 | 공개 T-006 계획/검증/티켓·구조/README/TODO와 필요한 연결 노트, 표준 build·전체 회귀 | 실제 근거에만 완료 체크. 실제 실행 근거와 변경 상태를 구분해 기록 |

### Task A: typed 계약·호환·순수 종료

- [x] D01 실패 테스트: 현재 가격 payload/result/effect/저장 요청의 JSON fixture가 새 enum에서도 동일; purchase_order place/revise/cancel 정상, unknown kind/action/version/필드·숫자 JSON·서로 다른 action 필드 혼합은 거절. 기존 코드가 새 주문 kind를 거절하는 행동 RED부터 기록한다.
- [x] D02 실패 테스트: 미완성 수량/날짜·선택 빈 값 저장 가능, 확정 필드 null/0 구분, 윤년2028-02-29 유효/2026-02-29 실패, 수정·취소 메모 빈 값 유효. 기존 가격 정정의 빈 note는 계속 issue다. 문자열 최대 길이+1/제어문자/수량 7자리 소수·단위 환산 상한 오류를 고정한다.
- [x] DTO/normalize·확정 필드 검사·kind dispatch 접근 함수를 구현한다. 이전 가격 payload 정규화 결과와 직렬화 순서를 바꾸지 않는다.
- [x] D03/D04 close_order 구현과 계약 테스트: 10kg→9.8/10/10.2kg, 10→12kg 수정→11.8kg, 1L→900mL, 동일 자유 단위, 0/다른 품목·order·version·차원·닫힌 상태·범위 초과 거절. 순수 함수 전후 Store 내용 불변을 확인한다.
- [x] `cargo test --locked --test business_payload --test purchase_order` PASS. 컴파일 오류만을 완료된 행동 RED로 보고하지 않는다. frontend가 변경됐으면 루트 표준 build 뒤 실행한다.

### Task B: 영속 주문 효과·schema5·원자성

- [x] P01~P03 저장 실패 테스트: 불완전 place 초안 저장→Store 재열기→10kg/선택정보없음 확정; 같은 order를12kg·예상0원으로 수정; 취소. 최초/수정/취소 version·confirmation·raw draft·원응답 모두 남고 catalog 사전 가격/holding은 전후 동일하다.
- [x] P04/P05: closed 수정/취소 실패, draft archive는 실제 주문 불변, 다른 UUID의 동일 내용 주문은 별개, preview 뒤 다른 탭 수정/취소 또는 품목 버전 갱신은 stale_state 효과0. kind/action/order 대상 바꾸는 draft revision은 validation이다.
- [x] P06/P07: place/revise/cancel의 같은 UUID 재시도는 최초 응답 전체 동일, URL/본문 변경 UUID 재사용은 request_conflict. 효과/confirmation/outbox 각각 오류 주입 시 order head/version/요청/seq/snapshot 전부 rollback. 발행 실패 시 같은 요청 복구 전 성공 금지·다른 UUID 쓰기 차단·재시작 뒤 원응답 재생.
- [x] 주문 테이블/handler/조회·business match를 구현하고 같은 tx에서 효과와 원응답을 생성한다. 단일 주문 조회는 WHERE id, head는 직접 join으로 가져와 전체 주문·전체 revision을 매번 읽지 않는다. 목록은 필요한 요약을 한정된 조회로 구성한다.
- [x] P08/P09/P10: schema4 DB/가격 draft·확정·정정·요청/JSON bytes 보존 migration, 옛 pending 재시도/파일 누락 원문 복구, 새 snapshot 주문 컬렉션 필수. schema4에 새 order kind 주입·schema5 누락/unknown action·future schema/format은 원본 불변 실패. B10과 같은 동일 손상 bytes 방식 사용.
- [x] `cargo test --locked --test purchase_order_storage --test business_storage --test catalog_storage --test storage --test hot_journal` PASS. T-005 fixture 제약 실패/Windows persist 권한 문제를 제품 결함과 구분한다.

### Task C: HTTP 연결

- [x] H01: GET orders 기본 open 배열/중첩 state, detail 이력, received 빈 목록, unknown status/UUID400, missing404, /orders와UUID GET/HEAD200·unknown404를 먼저 실패 확인한다.
- [x] H02/H03: 기존 business routes로 place/revise/cancel·preview/confirm, 문자열 수치/strict actions, foreign Host/Origin/no-session/POST Origin 없음/17KiB413. 잘못된 요청은 DB/파일 변화0. stale confirm과 request_conflict를 구별한다.
- [x] 두 GET handler와 SPA allowlist만 추가하고 같은 with_store/공통 guard를 사용한다. 새 주문 쓰기 endpoint·가상 receipt endpoint는 만들지 않는다.
- [x] `cargo test --locked --test purchase_order_http --test business_http --test catalog_http --test http` PASS. 서버/테스트 외 파일은 해당 담당이 바꾸지 않는다.

### Task D: 일반 UI 주문 흐름

- [x] 기존 가격 unit/pending 테스트에 새 union/restore fixture를 추가하고 주문 validation·payload/action 잠금·불확실 요청 복구의 행동 RED를 기록한다. 새 주문 작성 identity는 기존 가격 작성 identity와 분리하고 sessionStorage 키 충돌을 막는다.
- [x] 공통 useBusinessDraft의 `가격 확정` 등 가격 전용 성공/안내 문구와 validateBusinessDraft의 가격 필드 검사를 kind/action별로 분기한다. 주문은 `주문 확정`/`주문 수정`/`주문 취소`를 명확히 표시하고 기존 가격 문구는 유지한다. 공통 전송·잠금·재시도 상태 기계를 복제하지 않는다.
- [x] `/orders` 목록에서 주문 작성 진입→기존 business 영속 초안 저장→상세 preview/confirm→주문 상세 연결을 구현한다. `/business`에서 kind/주문 action을 구분하며 가격 흐름을 그대로 제공한다. record가 이미 생성된 뒤 같은 local payload가 새 주문으로 재등록되지 않도록 기존 created/completed 처리를 유지한다.
- [x] 주문 상세의 open만 수정/취소 진입을 제공한다. 수정은 같은 품목 고정·현재 값/변경 후 값·선택 메모, 취소는 수량 차감 없는 상태 변경·선택 메모를 보여준다. 이력에서 business record/당시 품목 버전을 연결한다. received/실입고 버튼과 가짜 재고 숫자는 표시하지 않는다.
- [x] latest 상태 확인과 비동기 재료 응답에서 request identity/head/material guard를 적용한다. 오래된 응답이 현재 선택·pending·입력·preview를 덮어쓰지 않아야 한다. 별도 탭 변경 후 입력 보존→명시적 재확인→새 preview/UUID이며 request_conflict는 자동 재키잉하지 않는다.
- [x] U01~U05 E2E: 일반 UI 주문 초안 저장→실제 프로세스 재시작→확정; 수정→취소→불변 이력; 두 탭 stale; 응답 유실/화면 이동 중 재진입·같은 요청; 기존 가격 작성/정정·catalog 회귀. 실제 주문량이 화면/GET/DB와 맞고 보유 수량으로 표시되지 않는지 확인한다.
- [x] U06: 입력 label·오류 focus·키보드·disabled 이유·라이트/다크·390/768/1440px 확인. 예상 금액 없음/0와 날짜 의미를 명확히 표현한다.
- [x] `npm --prefix frontend test`, `npm --prefix frontend run typecheck` PASS 후 UI 쓰기를 멈추고 루트 표준 build를 요청한다. build 뒤 `npm --prefix frontend run test:e2e -- e2e/orders.spec.ts`와 기존 회귀를 실행한다.

## 7. 통합 검증과 수용 기준 대응

- [x] backend 행동 RED를 먼저 확보하고 UI 편집을 시작한다. frontend 변경 중 Cargo guard 실패를 제품 RED로 세지 않는다. 각 담당 결과를 모아 표준 `node scripts/build.mjs`를 한 번 조율한 뒤 검증한다.
- [x] `cargo test --locked`, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`와 frontend unit/typecheck/E2E를 실행한다. `node --test scripts/ui-manifest.test.mjs`, `node scripts/ui-manifest.mjs --check`, `node --test scripts/release-artifact.test.mjs`는 현재 산출물을 기준으로 확인한다.
- [x] 실제 실행 명령·브라우저/OS·PASS/FAIL·미검증 경계를 T-006 검증 문서에 기록하고 RTD After 13개를 수행한다. T-001 외부 장비/절전, T-018 복원, T-019 이동, T-007 실제 입고를 이번 완료로 간주하지 않는다.

| 수용 기준 | 이 계획의 증거 | 남는 경계 |
| --- | --- | --- |
| T-006-01 | D02/P01/H02/U01: 호환 단위·선택 입력·DB 재시작·확정 | 거래처 마스터/품목교체 없음 |
| T-006-02 | P01~03/H02/U01~02: 실제 주문/수정/취소 후 기존 가격·holding 불변 | T-007 실제 보유 원장 연결 후 추가 회귀 |
| T-006-03 | P03~07/H02~03/U02~04: 취소는 주문 상태만 변경 | 취소≠입고 후 반품/일반 정정 |
| T-006-04 | D03/D04: typed 합성 receipt 결과와 정확한 종료/차이 반환 계약 | SQL receipt 결과·종료/재고 동시 tx·실입고 UI는 T-007 |
| T-006-05 | P04~07/H03/U03~04: 취소 중복/상태 방지·원 이력, D04 닫힌 주문 순수 거절 | 실제 입고 뒤 DB/API 중복 종료는 T-007에서 검증 |

자체 검토: 01~05 각각 검증 작업에 대응했다. 이전 가격 wire/옛 snapshot/재시도, 수정·취소 경쟁, 최초/현재 주문량 보존을 테스트에 연결했다. 미결정 운영 정책을 추가하지 않고 T-007/T-012 경계를 분리했다. 루트·독립 검토를 거쳐 실행 가능한 범위를 정했다. 새 운영 결정은 추가하지 않으며 완료 체크는 실제 실행 근거 이후에 한다.

## 8. 독립 검토 보완

T-005 완료 커밋과 현재 요구사항·계산 계약을 대조한 독립 검토에서 실행 차단 모순은 없었다. 다음 경계를 명시적으로 보완했다.

- 취소 버전은 품목/수량/거래처/예정일/예상 금액을 현재 버전에서 복사하고 `note`에는 해당 취소의 선택 메모를 기록한다. 이전 메모는 이전 주문 버전과 업무 입력에 보존한다. 일반 주문 수정·취소 메모는 선택이며 T-005 가격 정정의 필수 이유와 다르다.
- P10은 schema5 DB/JSON에 `received`, `cancel/open`, `place/cancelled`를 주입한 경우도 거절하는지 확인한다. 공개 상태 enum에 `received`가 있다고 해서 실제 입고 참조가 없는 행을 현재 reader가 허용하지 않는다. 새 주문 snapshot의 action/status 조합을 명시적으로 검증한다.
- kind/action/order 대상 불변 및 현재 상태 검사는 `mutate` 내부 적용 단계에서 수행한다. 이미 성공한 동일 요청의 원응답 재생보다 먼저 현재 주문 상태를 검사해 옛 재시도를 실패시키지 않는다.

## 9. 실행 결과

[검증 기록](T-006-verification.md)에 실제 실행 명령과 수용 기준별 결과를 기록했다. Rust99·UI 단위22·타입/포맷/정적 검사·정식 빌드·브라우저29개, 6화면과 키보드 흐름을 확인했다. 위 체크는 해당 작업의 최종 구현·검증 완료이며 모든 개별 테스트를 구현 전 RED로 관측했다는 뜻은 아니다. 실제 착수 RED는 입력·저장·HTTP의 미지원 주문과 UI 복사/검증 오류이며 순수 함수의 개별 수계산은 최종 GREEN 근거다. 실제 입고 연결은 T-007, 출시·실데이터 시연 완료 판단은 T-021에 남는다.

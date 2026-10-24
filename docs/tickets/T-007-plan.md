# T-007 실측 입고·보유 평균 원가 구현 계획

- 상태: T-006 완료 후 설계·독립 검토 완료, 구현 착수
- 기준 커밋: `51c1d5c`, schema5·JSON 형식2
- 기준: [T-007](T-007-receipts.md), [요구사항](../REQUIREMENTS.md) C-02/D-02·D-04·D-05, [계산 계약](../DOMAIN-CONTRACT.md) §2~5, [ADR-007](../decisions/ADR-007-consumption-and-ticket-scope.md), [T-006 검증](T-006-verification.md).

2026-10-24. T-006의 Rust99·UI단위22·브라우저29·정식 빌드·포맷/정적 검사·RTD와 커밋을 확인했다. 기존 변경을 보존하고 같은 작업 위치에서 이어간다. 기술 선택은 위임 범위이며 새 운영 정책을 추가하지 않는다. 제품 검증 성공은 실제 실행 이후에 기록한다.

목적: 일반 직접/주문 연결 입고를 DB 초안→재시작→미리보기→확정하고 실제 보유 Q/V와 원가 출처를 연결한다. 주문 연결 입고는 최초·현재 주문량·실측량을 보존하며 같은 tx에서 received로 종료한다. 이미 포함/확인 필요 입력은 영속 초안으로 보관하고 수량 중복 가산을 막는다.

범위 밖: 실제 실사 writer/UI(T-010), 지연 입고 보정(T-011), 일반 정정(T-012), 생산·폐기·반품, 부분 입고 추적, 거래처 마스터, 배송비/세금 배분, 전체 복원·이동.

## 1. RTD Before 4개

- [Step 1] 변경 계획 - PASS: 수용 기준01~06을 아래 DTO·schema6·Store/API·UI·D/P/H/U 검증에 대응한다. 기존 mutate/recover/CAS와 순수 close_order를 재사용하고 실제 receipt/holding/closure만 추가한다. migration은 기존 백업+단일 tx이며 실패 시 원 DB rollback을 검증한다.
- [Step 2] 계획 검토 - PASS: 현재 holding은 NotEstablished 하드코딩, 주문 CHECK는 received 금지, schema5는 live DTO를 참조한다는 실제 코드 차이를 확인했다. 이를 adapter/보존 migration/frozen5로 각각 해결한다. 과거일과 seq, 미확인과0, 빈금액과0원, 관측과정산기준을 구별하며 공개 데이터는 가상 자료뿐이다.
- [Step 3] 검토 재검토 - PASS: 가정은 ① T-006 최종 schema5/현재 wire가 유지됨 ② 정상 입고는 양수 수량·확인한 실제 금액이며 미기록분만 가산함 ③ 실제 관측 writer는 T-010까지 없다는 세 가지다. ①이 달라지면 fixture/reader를 갱신하며 ②·③은 정본 계약/티켓 범위다. 운영 판단을 새로 요청할 사항은 없다.
- [Step 4] 계획 단순화 - PASS: generic ledger API/동적 handler registry/새 mutation 프레임워크를 만들지 않는다. 닫힌 receipt kind와 작은 holding 순수 계산/저장 adapter로 한 실제 흐름만 구현한다. 합성 관측은 순수 계약 테스트이며 운영 DB에 테스트용 관측을 넣는 endpoint는 만들지 않는다.

## 2. 입력 DTO와 미완성 초안

`BusinessKind/Payload/Result/Effect`에 `receipt`를 추가한다. 기존 `{kind,version,data}` 필드 순서와 catalog_price/purchase_order 직렬화 원문을 유지한다. 각 envelope/variant/nested object는 unknown field를 거절하며 version은 정확히1이다. 기존 business create/revise/preview/confirm/archive URL·meta·원응답 형식은 유지한다. receipt의 non-null corrects_confirmation_id는 validation으로 거절하고 실제 일반 정정은 T-012에 둔다.

```text
BusinessPayload.Receipt = {kind:"receipt", version:1, data:DraftReceiptV1}
DraftReceiptV1 = {
  source: ReceiptSourceDraft,
  quantity:string, unit:string, amount:string, receipt_date:string, note:string,
  stock_effect: ReceiptStockEffectDraft
}
ReceiptSourceDraft =
  {kind:"direct", material_id:string|null, material_version_id:string|null}
| {kind:"order", order_id:string|null, expected_order_version_id:string|null,
   material_version_id:string|null}
ReceiptStockEffectDraft =
  {kind:"unreflected"}
| {kind:"needs_reconciliation", basis_id:string|null, included_quantity:string}
```

order source에 임의 material_id를 받지 않는다. 현재 order의 material을 고정하고 선택한 material_version이 같은 품목의 최신 버전인지 검사한다. null 참조는 미완성 저장을 허용하고 non-null UUID는 문법 검사·정규화한다. direct/order 선택 및 대상 변경은 미확정 초안의 명시적 편집으로 허용하되 pending 전송 본문은 바꾸지 않는다. business kind는 첫 저장 후 불변이며 source 변경을 자동 추정하지 않는다.

stock_effect는 사용자가 선택하는 '아직 재고에 반영되지 않은 입고'와 '이미 포함되었거나 포함 여부 확인 필요'의 두 경우다. 두 번째는 basis_id/included_quantity를 몰라도 빈 초안으로 저장하고 재시작 후 보존한다. 이 참조는 T-011용 입력 메모이며 현재 실사/시작 기준 존재를 확인한 FK라고 표시하지 않는다. 이미 포함량을 계산하거나 유효한 보정 수량으로 확정하지 않는다. 선택을 날짜 비교로 자동 전환하지 않는다. UI 초기값은 unreflected이고 그 의미를 입력 옆에서 설명한다.

quantity/amount/included_quantity는 최대80자, unit20자, receipt_date32자, note2000자. note만 CRLF/CR→LF 후 줄바꿈 허용; 다른 제어문자는 거절한다. 숫자/날짜 원문은 초안에 그대로 보존한다. 확정 시 수량>0, Money의0포함범위, 같은 단위 차원·canonical 환산, 정확한 실제 달력 날짜를 검사한다. 금액 빈값은 미완성이고 `"0"`은 실제0원이다. 주문 예상금액/사전가격을 실제 구입금액으로 자동 저장하지 않는다.

needs_reconciliation은 다른 필드가 완성돼도 `needs_reconciliation` issue·effect=null·can_confirm=false이며 receipt/holding/closure/confirmation을 만들지 않는다. 일반 과거 입고일은 허용하며 날짜만 과거라는 이유로 blocked 처리하지 않는다. 모든 confirm 검사는 mutate 내부 최신 상태에서 재수행한다.

## 3. 날짜 clock과 순수 보유 계산

`src/receipt.rs`에 `parse_receipt_date(text, today:NaiveDate)`를 둔다. trim 후 정확 YYYY-MM-DD, 연도0001~9999·실제 달력·`date<=today`만 허용한다. 빈 날짜는 확정 불가다. 미래 예정은 주문 expected_date 화면으로 안내한다. 현재 local day는 서버 PC의 `chrono::Local::now().date_naive()`를 사용하고 UTC recorded_at은 기존 mutate 값이다. 브라우저 시계로 확정 유효성을 결정하지 않는다.

Store는 작은 날짜 공급원 `fn() -> NaiveDate`를 보유한다. `Store::open`은 시스템 local day, 테스트용 `open_with_today(root, fn()->NaiveDate)`는 고정 함수를 사용한다. 새 chrono 의존성은 필요 없다. preview와 confirm 각각 한 번 날짜를 캡처해 같은 검사 함수에 전달한다. 날짜 경계가 지나면 confirm이 다시 검사하며 과거 요청 재생은 날짜 검사보다 먼저 실행한다. 테스트는 윤년/미래/빈값·local/UTC 날짜가 다른 고정시계를 검증한다.

`src/holding.rs`의 pure 계약은 SQLite·HTTP를 받지 않는다.

```text
HoldingInput {
  basis: Option<BasisReference>, observation: Option<HoldingObservation>,
  quantity: Option<Quantity>, value: Option<Money>,
  source_id: Option<String>, last_valid: Option<UnitPrice>
}
BasisReference {kind: Receipt | Observation, id:string|null, seq:string|null}
HoldingObservation {id:string, quantity:string, recorded_at:string, seq:string}
HoldingChange {kind:Receipt, event_id:string, quantity:string, value:string,
               recorded_at:string, seq:string}
project_receipt(before, positive_canonical_q, amount)
  -> Result<HoldingProjection, DomainError>
```

불변식: basis없음은 quantity/value없음이고 NotEstablished다. known은 Q/V가 모두 있으며 Q0/V>0은 거절한다. Q>0/V없음은 ValueUnknown이고 새 입고로 이전 가치를0으로 메우지 않으며 확정 불가 issue를 만든다. 첫 정상 입고는 after Q=q,V=a로 receipt baseline을 만들고 observation은null. known은 checked Q+q/V+a로 도메인 상한을 검사한다. receipt의 실제 UUID/seq가 아직 없는 preview는 신규 basis/source ID를 null로 표시하며 임의 UUID를 생성하지 않는다.

합성 observation 기준10kg 이후 receipt2kg의 조회 projection은 observation10kg·원시각 유지, confirmed change+2kg, 정산기준12kg이다. 첫 receipt baseline과 그 receipt는 같은 seq이며 이후 합산은 `seq > baseline.seq`만 사용한다. 정산기준을 새 실측 현재고로 표시하지 않는다.

`project_receipt`는 이미 계산된 현재 Q/V에 한 입고를 더하는 미리보기다. `project_holding`은 정확한 시작 기준과 이후 변동으로 조회 내용을 구성하며 현재 누적 Q/V를 시작 기준으로 다시 쓰지 않는다. 내부 계산은 문자열을 검증해 Quantity/Money/양의 i64 순서로 처리한다. 조회 DTO의 모든 seq는 문자열이다. 시작 기준과 같은 순서의 입고는 같은 ID·수량·가치일 때만 제외하고, 중복 ID·역순·다른 근거의 재사용은 거절한다. 첫 입고는 시작 근거에 한 번만 표시하고 후속 변동 목록에는 그 이후 입고만 포함한다.

정확 단가는 V/Q를 UnitPrice::new로 만들고 source_id는 holding version이다. 0원 비율도 유효하다. current Q>0이면 current 평균 우선, known Q0이면 last-valid→사전가격, NotEstablished는 사전가격 또는없음, ValueUnknown>0은 가격미확인이다. 마지막 유효 비율은 새 양수 holding version으로 갱신하며 나중에0이돼도 이전 참조를 보존하는 pure 계약을 검증한다. 이번 정상 writer는 소모/폐기를 구현하지 않는다.

## 4. 출력 DTO·Store/API

모든 수량/가치/차이/seq는 십진 문자열이다. source_id와 참조 UUID를 보존한다. 일반 새 receipt preview의 전후와 주문 종료를 한 화면에 표시하며 actual receipt ID는 확정 때만 생성한다.

```text
ReceiptRecord = {id, confirmation_id, business_record_id, material_id,
 material_version_id, material_name, quantity, unit, input_quantity,input_unit,
 amount,receipt_date,note,recorded_at,seq,source:ReceiptSource}
ReceiptSource = {kind:"direct"}
 | {kind:"order",order_id,ordered_version_id}
ReceiptList = {items:ReceiptRecord[],state:CatalogState}
ReceiptDetail = {item:ReceiptRecord,holding_version_id,
 order_closure:OrderReceiptClosure|null,state:CatalogState}

HoldingAmounts = {quantity:string,value:string|null,source_id:string|null}
HoldingBasisRef = {kind:"receipt"|"observation",id:string|null,seq:string|null}
HoldingDetail = {material_id,material_version_id,unit,
 basis:HoldingBasisRef|null,observation:HoldingObservation|null,
 settlement_basis:HoldingAmounts|null,last_valid_price:PriceBasis|null,
 confirmed_changes:HoldingChange[],price:ResolvedPrice,state:CatalogState}
PriceBasis = {amount,quantity,source_id}

BusinessResult.Receipt = {kind:"receipt",receipt_id,holding_version_id,
 material_id,material_version_id,order_id:string|null,order_version_id:string|null}
BusinessEffect.Receipt = {kind:"receipt",material_id,material_version_id,
 quantity,amount,unit,input_quantity,input_unit,receipt_date,
 before:HoldingAmounts|null,after:HoldingAmounts,
 basis_before:HoldingBasisRef|null,observation:HoldingObservation|null,
 order_before:OrderVersion|null,order_after:OrderReceiptPreview|null}
OrderReceiptPreview = {status:"received",order_id,initial_order_version_id,
 initial_ordered_quantity,ordered_version_id,ordered_quantity,
 received_quantity,difference,unit}
```

preview.after.source_id는 null, 기존 before.source_id는 실제 holding version이다. `OrderReceiptPreview`에는 아직 없는 receipt/confirmation UUID를 넣지 않는다. 실제 close_order 반환값은 ReceiptDetail.order_closure로 저장 결과를 조회한다. received order version에는 기존 주문량을 보존하고 실측 수량으로 바꿔 쓰지 않는다.

```rust
Store::receipt_list(&self, filter: ReceiptFilter) -> Result<ReceiptList,StoreError>;
Store::receipt_detail(&self, id:&str) -> Result<Option<ReceiptDetail>,StoreError>;
Store::holding_detail(&self, material_id:&str) -> Result<Option<HoldingDetail>,StoreError>;
```

ReceiptFilter는 선택 material_id 하나만 받으며 unknown 필드·잘못된 UUID를 거절한다. GET `/api/receipts`, `/api/receipts/{id}`, `/api/holdings/{material_id}`를 추가한다. 재료는 있으나 입고가 없으면 holding detail은 200과 미확인 근거, 재료 자체가 없으면 404다. 기존 GET materials/catalog 외형은 유지하고 `catalog::summary`의 price만 실제 adapter로 바뀐다. 새로운 POST endpoint는 없다. 읽기 200/없음 404·신규 mutation 201·validation/request_conflict/stale_state/unavailable 기존 code와 Host/Origin/세션/16KiB를 보존한다.

`/receipts`, `/receipts/<UUID>`와 재료 상세의 보유 근거 패널을 제공한다. 별도 holding 목록/대시보드·가짜 관측 입력 화면을 추가하지 않는다. 전체 receipt 목록과 같은 품목 조회는 SQL filter/order를 사용하며 snapshot 재파싱 또는 품목마다 전체 이력 조회를 추가하지 않는다.

## 5. schema6 저장과 실제 주문 종료

writer는 format2/schema6다. 최소 추가 관계는 receipts, holding_versions, holding_heads, order_receipt_closures다.

| 테이블 | 필수 값/제약 |
| --- | --- |
| receipts | id PK, confirmation UNIQUE FK, material/version 복합FK, source_kind direct/order, q>0/amount>=0 micro CHECK, canonical/input 단위·수량, receipt_date/note/time/seq FK. source order는 closure가 반드시 있어야 함 |
| holding_versions | id PK, material/version FK, previous version 복합FK, receipt_id UNIQUE FK, baseline_receipt_id FK 및 baseline_seq, Q>0/V>=0 micro CHECK, time/seq. `(material_id,id)` UNIQUE |
| holding_heads | material PK FK, current version 복합FK, last_valid version 복합FK. T007 정상writer는 둘이 새 양수 version을 가리킴 |
| order_receipt_closures | order_id UNIQUE FK, receipt_id UNIQUE FK, receipt confirmation FK, initial/ordered/received version 복합FK, canonical actual quantity·signed difference. 실제 receipt·종료버전·같은 material/seq를 의미 검증 |

관측 컬럼/테이블이나 0/0 holding 행을 migration에서 만들지 않는다. 실제 observation은 없고 pure 합성 조회에서만 제공한다. 현재 known-only DB의 V는 NOT NULL이며 ValueUnknown은 후속 adapter의 typed 계약 테스트에만 있다. receipt baseline은 첫 receipt/version의 정확한 근거를 보존한다. 새 snapshot에는 위 네 collection이 모두 필수다.

출력 OrderAction에 receive를 추가하되 `DraftOrderActionV1`는 Place/Revise/Cancel을 그대로 유지한다. 새 received order version은 이전 현재 주문 필드(품목·주문량·거래처·예정일·예상금액·메모)를 복사하고 action=receive,status=received, confirmation은 실제 receipt confirmation이다. 입고 메모는 receipt에 보존한다. order business 수정/취소 API로 receive를 만들 수 없다. `BusinessResult::PurchaseOrder`의 기존 action 외형은 동일하고 receipt 결과는 별도Receipt variant다.

기존 purchase_order_versions CHECK를 receive/received까지 확장하는 보존적 재작성은 migration tx 안에서 수행한다. 원 ID·previous/head·confirmation·seq·모든 문자열을 그대로 복사하며 FK defer→새 테이블→복사→원 이름 복구/index→foreign_key_check/의미검증 순서를 SQLite fixture로 검증한다. foreign_keys를 tx 안에서 껐다고 가정하지 않는다. received 행은 closure가 없거나 다른 receipt/material/ordered version이면 거절한다. 이후 주문 수정/취소/두 번째 입고는 stale_state다.

적용 순서: 기존 mutate의 원응답 조회/recover → 전역 CAS → business head 및 품목/order head/입력 검사 → confirmation ID 생성 → receipt ID와 결과 생성 → close_order 계산 → receipt/holding/received version/closure 저장 → business 결과 → 원응답/전체 snapshot → commit → recover. 같은 tx이며 중간 실패는 모든 변경을 취소한다. 동일 성공 UUID의 재생은 날짜·terminal 상태 검사보다 먼저 이루어진다. receipt를 material_prices에도 쓰지 않는다.

5→6 migration은 모든 outbox를 source_schema=5로 선검증하고 새 app이 아는 6이라도 원 DB에 섞인 기록은 거절한다. schema4 reader를 유지하고, schema5는 catalog_price/purchase_order payload·place/revise/cancel 주문 version만 허용하는 frozen DTO로 고정한다. 기존 mutation payload/response 문자열과 JSON bytes는 재직렬화하지 않는다. 실제 T-006 release의 가상 가격 정정/place/revise/cancel fixture를 캡처하여 ID·원요청/응답·원문 발행을 검증한다.

## 6. 파일 소유와 실행 순서

| 담당 | 소유/책임 |
| --- | --- |
| 루트 | 공개 계획/검증/티켓·README/ARCHITECTURE/TODO/연결노트, src/business.rs enum·src/lib.rs, src/holding.rs·tests/holding.rs 순수 보유/조회 계약, 통합 build·검사·최종 Git |
| A 저장 | src/store/receipt.rs·holding.rs, store.rs/business.rs/catalog.rs/purchase_order.rs/snapshot.rs, 저장테스트·schema5 fixture. 공통 mutation은 필요한 최소 변경만 |
| C 계약/API | src/receipt.rs·purchase_order.rs(순수/출력 receive), tests/receipt.rs, src/server.rs·tests/receipt_http.rs, A/root와 공개 DTO 시그니처 합의 |
| D UI | receipt types/utils/editor·목록/상세, business 공통kind/초안·재료상세 원가/holding 연결, frontend unit/E2E·실제화면 |

먼저 A가 기존 T-006 자산에서 입고 초안→확정 저장 테스트를 쓰고 미지원 kind의 행동 RED를 실행한다. C는 HTTP 미지원/404 행동 RED, 루트는 payload 구조 RED를 병렬 확인한다. RED 동안 새 DTO/lib와 frontend를 쓰지 않는다. 이후 공개 타입을 고정해 A/C/D가 병렬 구현한다. frontend 변경 중 Cargo는 실행하지 않고 D의 unit/typecheck 뒤 쓰기를 고정하여 루트 표준 build→전체 검증을 한다. stage/commit은 루트의 별도 승인 범위다.

## 7. 실행 가능한 검증

모든 자료는 가상이며 단일 PC 사용자/복수 탭을 대상으로 한다. 테스트는 구현 전에 기대 행동을 작성하고 가능하면 실제 미지원 경로의 RED를 확보한다. 기존 회귀를 새 schema에 맞추기 위해 v1/v4/v5 fixture 읽기 검증을 제거하지 않는다.

| ID | 시작→행동→관측 결과 |
| --- | --- |
| D01 | 기존 가격/주문 payload·canonical 원문→새 enum 왕복→완전히 동일. receipt nested unknown/version/source 조합 거절, null 참조/미완성 문자열 보존 |
| D02 | 고정 local today→과거/오늘/미래/윤년/빈 날짜→정확한 허용/거절. UTC 날짜와 다른 local day도 같은 공급원으로 검증 |
| D03 | NotEstablished→첫 1kg/10000원+둘째 2kg/40000원→3000g/50000원·정확 50000/3000, 0원 유효, 합계/환산 상한 오류 |
| D04 | 합성 관측 10kg→receipt 2kg→관측 10kg/원시각 불변·후속 +2kg·정산 12kg. 첫 baseline receipt의 배타 seq로 중복 가산 없음 |
| D05 | Known Q0/last-valid, NotEstablished/사전 가격, ValueUnknown>0, 양수 Q/V0→우선순위·없음과 0원 구별 |
| P01 | 신규 품목/가격/주문→holding 미확인·행 없음; 미완성 입고 draft→재시작→수정→확정→receipt/holding 한 번, 당시 품목/입력 단위 보존 |
| P02 | 직접 1kg/10000원+2kg/40000원→DB Q/V·catalog source/ratio와 snapshot 일치, 사전 가격 행 불변·입고 이력 조회 |
| P03 | 실제 주문 10kg→수정 12kg→실입고 11.8kg→received와 -200g·최초/현재/실량 각각 보존; 직접 입고에는 closure 없음 |
| P04 | 주문 수정/취소·다른 receipt 확정 후 낡은 preview→stale_state·효과 없음. 닫힌 주문 새 UUID 중복 입고 거절, 동일 UUID 원응답 재생 |
| P05 | needs_reconciliation 초안→재시작·편집·확정 시도→can_confirm=false·Q/V/receipt/closure/order/state 불변. 과거일 unreflected는 정상 |
| P06 | 수량 0/빈 금액/미래/차원/상한 실패→효과 없음. 실제 금액 0 성공·정확한 원가 출처 |
| P07 | receipt/holding/order close/outbox 각각 실패 주입→모든 변경 rollback. DB commit 후 JSON 실패→같은 UUID 복구·다른 쓰기 차단·중복 없음 |
| P08 | schema5 실제 fixture→migration→기존 가격/주문 request 재생·ID/canonical 원문/원응답/JSON bytes 불변·옛 file 누락 원문 복구 |
| P09 | schema5 DB+schema6 outbox/새 kind/action, schema6 누락 collection/received의 잘못된 FK/무근거 holding→거절·원 DB/동일 변조 파일 불변. parser 실패가 파일 불일치에 가려지지 않게 함 |
| H01 | Host/Origin/세션/16KiB/unknown nested→기존 code와 접근 보호 유지; GET 200/404/validation 필터 |
| H02 | 직접/주문 입고 API→preview 무변경·confirm 201·receipt/holding GET/cost 일치. 미확인 holding 200과 없는 재료 404 구별 |
| H03 | 낡은 CAS/주문 참조·동일 UUID 본문/URL 충돌·응답 재생→stale_state/request_conflict 구별, 고정 본문만 재전송 |
| U01 | 일반 UI 가상 직접 입고 미완성 저장→프로세스 종료/재시작→초안 편집→미리보기→확정→실량/금액/날짜/원가/정산 근거 표시·DB 일치 |
| U02 | 주문 상세 입고 작성→최초 10/현재 12/실량 11.8kg→미리보기 차이→확정→received 목록/불변 이력/실제 receipt 링크·원가 갱신 |
| U03 | 포함/확인 필요 선택·빈 포함량 저장→재시작→차단 이유 표시/확정 disabled·주문/holding 무변화 |
| U04 | 두 탭 stale·응답 유실·저장 중 화면 이동/재진입→기존 pending 원문/UUID 불변, 자동 새 UUID 없음·명시적 재확인 |
| U05 | 빈 금액과 0·관측 없음과 정산 기준·local 날짜 오류의 label/focus/키보드/disabled 이유, 390/768/1440 라이트/다크; 기존 가격 정정·catalog·주문 회귀 |

수용 기준 대응: 01=P01/P03/H02/U01/U02, 02=D02/P06/U01, 03=D03/D04/P02/H02, 04=D05/P02/P06/U05, 05=P05/U03, 06=D03/P02. 실제 관측 UI는 이 대응에 없으며 합성 계약 증거만 기록한다.

검증 명령: `npm --prefix frontend test`, `npm --prefix frontend run typecheck`→쓰기 고정→`node scripts/build.mjs`→`cargo test --locked`, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, frontend receipt/기존 전체 E2E, `node --test scripts/ui-manifest.test.mjs`, `node scripts/ui-manifest.mjs --check`, `node --test scripts/release-artifact.test.mjs`. 실제 OS/브라우저/명령/PASS·FAIL을 기록한다. 기본 sandbox의 Windows 발행 권한 오류는 제품 실패와 구별하며 기존 승인된 격리 테스트 권한으로 검증한다.

## 8. 완료 경계와 초안 자체 검토

구현 뒤 수용 기준 01~06과 실제 실행 증거를 대조하고 RTD After 13개를 수행한다. 정상 UI·DB·API가 같은 결과인지 확인하고 T-001 별도 장비/절전·T-010 실제 실사·T-011 보정·T-018/019 복원/이동을 완료로 표시하지 않는다. 이 파일은 실행계획 검토용이며 제품 완료가 아니다. 공개화/최종 소유 분담은 루트 독립 검토 후 확정한다.


## 9. 독립 검토 보완

- 중첩 receipt의 `source`·`stock_effect`는 종류별 깊은 복사와 구조 비교를 사용한다. 기존 가격/주문은 wire·pending 키·원문을 유지한다. 중첩 필드만 편집해도 dirty가 되고 미리보기가 무효화되며, baseline·전송된 pending 본문은 변하지 않아야 한다. 재진입/복원·응답 유실 회귀를 검증한다.
- 순수 계산의 seq는 검사한 양의 i64로 사용할 수 있지만 API observation/change/basis view의 seq는 십진 문자열이다. 내부 타입을 그대로 Serialize하여 숫자 JSON을 보내지 않는다.
- HoldingInput의 basis/수량/가치/관측 불가능 조합을 validator로 거절한다. 합성 조회에서 관측 이전 사건·같은 baseline receipt를 다시 더하지 않으며 참조와 배타적 seq 경계를 검사한다.
- 관측 UI가 없는 현재 화면에서 basis UUID 입력을 요구하지 않는다. 포함/확인 필요 선택과 메모로 DB 초안을 보관하고, typed nullable 근거/미완성 포함량은 보존만 한다. 실제 관측 선택·포함량 검증·보정 확정은 T-011이다.
- received 주문 버전의 confirmation은 실제 receipt 종류다. 주문 상세/업무 결과 조회의 기존 가격·주문 전용 분기, receive 표시와 receipt 링크를 함께 확장해 실제 입고 기록으로 이동할 수 있어야 한다. 입력 DraftOrderAction은 여전히 Place/Revise/Cancel뿐이다.

이 보완은 현재 확정 요구를 구현할 기술 경계이며 새 사용자 정책 결정을 요구하지 않는다.

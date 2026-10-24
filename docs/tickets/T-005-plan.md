# T-005 업무 임시 저장·확정 구현 계획

- 작성일: 2026-10-24
- 상태: 공통 기반·대표 가격 업무 구현/검증 완료. 실제 판매 통합은 T-009/T-021에서 확인
- 대표 업무: 기존 사전 가격의 DB 초안→재시작→미리보기→확정·정정 연결
- 실제 운영 자료는 사용하지 않으며 스테이징·커밋은 통합 담당자가 실제 diff로 수행한다.
- 기준: [T-005](T-005-business-history.md), [계산 계약](../DOMAIN-CONTRACT.md), [T-003](T-003-verification.md), [T-012](T-012-corrections-and-versions.md)

## 1. 범위와 RTD Before

검토 근거: `T-005-business-history.md`, `T-012-corrections-and-versions.md`, T-006~T-009, ADR-007, DOMAIN-CONTRACT, 현재 `catalog_mutate`·가격 등록·snapshot/recover·마이그레이션, T-003 DTO 및 UI. 적용 스킬은 프로젝트 rtd-before/after와 pm의 test-scenarios다.

- [Step 1] 변경 계획 - PASS: 사전 가격 입력의 DB 초안 저장→재시작→편집→미리보기→확정→정정 연결을 구현한다. 실제 가격 버전이 결과물이다. 재고 원장·판매·실사·일반 정정 재계산·전체 복원은 후속 티켓이다. 아래 파일/DTO/검증과 실패 시 변경 전 DB 백업 보존을 정했다.
- [Step 2] 계획 검토 - PASS: 기존 catalog 요청 ID·원응답·v1/v2 JSON 원문·복구 게이트·Origin/세션·저장 중 종료를 보존한다. 새 초안도 영속 변경이므로 JSON을 남긴다. 아직 미지원 업무는 명시적으로 거절한다. 초안은 가격/수량/가치를 변경하지 않는다.
- [Step 3] 검토 재검토 - PASS: 가정은 ① T-003의 단일 writer와 공통 revision이 유지됨, ② 대표 사전 가격도 기존 가격 우선순위를 그대로 사용함, ③ 업무별 실제 효과는 후속 handler가 같은 SQL 트랜잭션에 연결함이다. 현재 코드·확정 정책과 맞는다. 저장 공통성 검증을 판매/재고 실구현 완료로 잘못 해석하지 않도록 §8에 수용 기준을 나눴다.
- [Step 4] 계획 단순화 - PASS: 일반 이벤트 엔진·외부 큐·handler 동적 등록·빈 미래 테이블을 만들지 않는다. 현재 helper의 추출과 닫힌 enum 한 variant, 실제 가격 effect 한 개만 구현한다. 스냅샷은 format 2/schema 4이며 format 3은 만들지 않는다.

이 계획은 추가 운영 결정을 만들지 않는다. T-005-06의 가격 정정 연결은 원기록과 최신 연결을 실제 검증하는 대표 사례이며 입고/판매/실사의 영향 계산 규칙을 대신하지 않는다.

## 2. 데이터와 상태 전이

모든 새 ID는 서버 UUID, sequence/revision과 유효 수량/금액은 JSON 문자열이다. 초안의 미완성 수량/금액은 계산값이 아니라 원입력 문자열이다. 임시 입력과 유효 정수 수치 타입을 구분한다.

| schema 4 관계 | 필드/불변 조건 |
| --- | --- |
| `business_records` | id, kind=`catalog_price`, created_at, created_sequence, `corrects_confirmation_id` nullable FK. 업무 종류/정정 대상은 생성 후 불변 |
| `business_revisions` | id, record_id, previous_revision_id nullable, status=`draft/confirmed/archived`, payload_version=1, payload_json, recorded_at, sequence. 추가만 가능; `(record_id,id)` unique·같은 record의 이전 revision FK |
| `business_heads` | record_id PK, current_revision_id; `(record_id,current_revision_id)` 복합 FK. 갱신 가능한 작은 projection |
| `business_confirmations` | id, record_id unique, revision_id unique, request_id unique, recorded_at, sequence. 확정 revision·record 소유관계 FK; 요청 FK는 동일 tx 마지막에 요청을 삽입하므로 deferred로 연결 |
| `business_catalog_price_results` | confirmation_id PK/FK, price_id unique FK→material_prices. 당시 품목 버전·수량·금액·입력 단위는 불변 price 행으로 연결. 후속 업무의 결과 테이블을 미리 만들지 않음 |
| `business_corrections` | target_confirmation_id PK/FK, replacement_confirmation_id unique FK. 확정 때만 추가하고 같은 종류·대상 품목 검증. 한 대상에 확정 대체 연결은 한 개 |

초안 생성→draft revision, 편집→새 draft revision, 아카이브→archived revision, 확정→confirmed revision+confirmation+실제 price 결과를 함께 만든다. archived/confirmed는 초안 편집·재확정할 수 없다. 같은 요청 재생은 이 상태 검사보다 앞서 원응답을 반환한다. 원문을 수정하거나 삭제하지 않는다.

정정은 현재 유효한 사전 가격 confirmation을 대상으로 **새 record**를 만들고 입력을 고친 뒤 확정한다. 초안 생성만으로 대체 연결을 확정하지 않는다. 확정 때 새 가격 버전과 `business_corrections`를 같은 tx에 추가한다. 원래 record/confirmation/가격은 남으며 조회는 원본부터 대체 연결을 따라 현재 유효 confirmation을 반환한다. 이 조회는 해당 정정 체인의 유효 상태이며 품목 전체의 최신 가격 선택은 기존 resolver 규칙 그대로다. 다른 독립 가격 등록을 지우거나 과거 원가를 재계산하지 않는다.

정정 대상 품목 변경은 거절한다. 이미 다른 정정이 확정된 옛 대상을 다시 대체하려 하면 stale_state이며 입력을 보존한다. 최신 대상에 새 정정으로 이어갈 수 있다. 복수 정정 초안은 가능하지만 한 대상의 확정 대체는 한 번이다. 이전 revision/정정 대상이 기존 행이어야 하므로 새 UUID 삽입만으로 순환 연결을 만들 수 없다. 기존 confirmed 기록의 단순 아카이브를 취소로 취급하지 않는다.

DOMAIN-CONTRACT §9의 정정 이유는 `DraftPriceV1.note`에 보존한다. 일반 가격 초안에서는 선택 메모, 정정 초안에서는 화면 레이블을 `정정 이유`로 표시한다. 정정 초안의 이유가 비어 있어도 임시 저장은 가능하지만 `note` 필드 issue를 반환하고 preview/confirm은 이유가 채워지기 전 실패한다. 정정 확정 revision의 note를 불변으로 남긴다.

## 3. DTO와 API 계약

새 HTTP 입력은 중첩 envelope를 사용해 `flatten`과 `deny_unknown_fields`의 충돌을 피한다. **모든 새 입력 envelope·payload·하위 data는 unknown field를 거절**한다. 기존 catalog API 외형은 바꾸지 않는다. DTO는 `src/business.rs` 한 곳에서 정의한다.

```text
BusinessPayload = { kind: "catalog_price", version: 1, data: DraftPriceV1 }
BusinessMeta = { request_id, epoch, expected_revision } // 기존 MutationMeta와 같은 외형, unknown field 거절
DraftPriceV1 = {
  material_id: string|null, material_version_id: string|null,
  quantity: string, unit: string, amount: string, note: string
}
BusinessStatus = "draft" | "confirmed" | "archived"
BusinessIssue = { field: string, code: string, message: string }
BusinessRevision = { id, record_id, previous_revision_id: string|null,
  status, payload: BusinessPayload, recorded_at, seq }
BusinessConfirmation = { id, record_id, revision_id, request_id, recorded_at, seq,
  result: { kind: "catalog_price", price_id, material_id, material_version_id } }
BusinessSummary = { id, kind, head_id, status, created_at, updated_at,
  corrects_confirmation_id: string|null, confirmation_id: string|null,
  effective_confirmation_id: string|null, issue_count: number, material_name: string|null }
BusinessDetail = { item: BusinessSummary, head: BusinessRevision,
  revisions: BusinessRevision[], confirmations: BusinessConfirmation[],
  corrections: { target_confirmation_id, replacement_confirmation_id }[],
  issues: BusinessIssue[], state: CatalogState }
BusinessList = { items: BusinessSummary[], state: CatalogState }
BusinessWrite = { item: BusinessSummary, head: BusinessRevision,
  confirmation: BusinessConfirmation|null, issues: BusinessIssue[] }
BusinessMutation = { result: BusinessWrite, state: CatalogState }
BusinessPreview = { record_id, head_id, state: CatalogState, can_confirm: boolean,
  issues: BusinessIssue[], effect: { kind: "catalog_price", material_id,
  material_version_id, quantity, amount, unit, corrects_confirmation_id,
  previous_price: MaterialPriceVersion|null, corrected_price: MaterialPriceVersion|null }|null }
```

`DraftPriceV1`의 빈 문자열/`1.` 같은 미완성 숫자는 초안에 그대로 보관한다. 수량·금액 각 80자, 단위 20자, note 2000자 이내·note 줄바꿈 외 제어문자 금지. null ID는 미선택이고 non-null은 UUID 구조를 검사한다. 없는 참조·미완성 수치·다른 차원·오래된 품목 버전은 필드 issues로 표시하여 저장된 초안을 다시 고칠 수 있게 한다. 확정에는 실제 품목·현재 품목 버전 일치, 양수 수량, 0 이상 금액, 같은 차원, DOMAIN 상한/정밀도가 필요하다. 종류/버전/자료형/길이가 잘못된 구조는 초안 저장부터 400으로 거절한다. 가격 0과 빈 금액은 다르다.

`BusinessDetail.item/head/revisions`는 요청 record의 상태/이력이다. `confirmations/corrections`는 그 record와 연결된 원본→현재 유효 확정 chain 전체를 seq순으로 반환한다. 각 confirmation의 record_id로 해당 상세를 열 수 있고 effective_confirmation_id가 이 배열 안에서 해석된다. 아직 미확정 정정 초안이면 대상의 기존 chain을 반환하되 초안 자체를 유효 확정으로 넣지 않는다. 조회 중 손상된 순환/깨진 연결을 만나면 명확한 오류를 반환해 무한 순회나 임의 연결 생략을 하지 않는다.

목록의 issue_count는 현재 draft의 입력 문제 개수이며 confirmed/archived는 0이다. 업무 수량이 아닌 UI 메타데이터 개수여서 JSON 정수로 표현한다. material_name은 payload의 참조 버전에서 읽은 이름이고 참조를 해석하지 못하면 null이다. UI는 상세마다 별도 API를 호출하지 않고 이 필드로 품목과 `입력 필요`를 표시한다.

| API | 요청 | 응답 |
| --- | --- | --- |
| `GET /api/business` | status=`draft/confirmed/archived/all` 기본 draft, kind 생략 또는 catalog_price | 200 BusinessList |
| `GET /api/business/{id}` | UUID | 200 BusinessDetail / 404 |
| `POST /api/business/drafts` | `{meta: BusinessMeta, payload, corrects_confirmation_id: string|null}` | 201 BusinessMutation |
| `POST /api/business/{id}/draft-revisions` | `{meta, expected_head_id, payload}` | 201 BusinessMutation |
| `POST /api/business/{id}/preview` | `{expected_head_id}`; 상태 변경 없음 | 200 BusinessPreview / head 불일치 409 |
| `POST /api/business/{id}/confirm` | `{meta, expected_head_id}` | 201 BusinessMutation |
| `POST /api/business/{id}/archive` | `{meta, expected_head_id}` | 201 BusinessMutation |

Store 메서드는 `business_list(BusinessFilter)`, `business_detail(&str)`, `business_create(CreateBusinessDraft)`, `business_revise(&str, ReviseBusinessDraft)`, `business_preview(&str, PreviewBusiness)`, `business_confirm(&str, ConfirmBusiness)`, `business_archive(&str, ArchiveBusiness)`로 고정한다. 읽기는 위 DTO 또는 Option<Detail>, 쓰기는 BusinessMutation, 모두 Result<_,StoreError>다. DTO의 kind/version은 닫힌 enum/custom deserializer로 정확히 검증한다. 숫자 JSON 금액·unknown kind/version·추가 `inventory_delta`·임의 `confirmed`는 실패한다.

미리보기는 Store mutex 아래에서 head+현재 전역 상태+품목 버전을 함께 읽는다. confirm의 meta.epoch/expected_revision과 expected_head_id는 **표시한 미리보기의 state/head를 그대로 복사**한다. 확정 본문으로 payload를 재전송하지 않고 저장된 head를 읽는다. 같은 tx에서 이 tuple과 참조를 다시 검증하므로 미리보기 이후 어떤 영속 변경도 stale_state가 된다. 별도 토큰 저장·만료 정책은 추가하지 않는다.

가격 미리보기의 previous_price는 현재 최신 사전 가격이고 corrected_price는 원정정 대상의 가격이다. 독립 가격 등록이 사이에 있으면 둘은 다를 수 있으므로 별도로 표시한다. 새 가격 확정으로 과거 가격/원가가 재계산된다고 설명하지 않는다. 10원 원확정 뒤 독립20원 등록→원확정의30원 정정은 대상10원·현재20원·새30원을 보여주고 기존10/20 이력을 모두 보존하는 검증을 추가한다.

공통 error code는 기존 validation/request_conflict/stale_state/unavailable 그대로다. 불확실 요청은 meta·대상·전체 본문·UUID를 고정해 재시도한다. stale_state만 최신 조회→입력 보존→사용자 재확인 후 새 UUID를 허용한다. request_conflict에 자동 새 UUID를 붙이지 않는다. 전체 목록·상세·쓰기 metadata는 원자적으로 같은 Store 호출에서 읽는다.

## 4. 기존 저장 경로의 최소 추출과 호환

`catalog_mutate`를 공통 `mutate`로 추출하되 기존 catalog가 저장하는 원응답의 JSON 외형·canonical payload를 바꾸지 않는다. 결과 포장만 callback/작은 공통 함수로 분리해 catalog는 기존 평탄 응답, business는 명시적 result/state 응답을 저장한다. legacy/new/business 요청은 하나의 UUID 공간이다. operation+대상 record ID+정규화된 전체 요청을 비교한다.

최소 내부 시그니처는 `mutate<R: Serialize + DeserializeOwned>(request_id, operation, canonical_payload, meta, apply)`이며 `apply(&Transaction, sequence, recorded_at, &new_state) -> Result<R, StoreError>`가 최종 응답 DTO를 반환한다. helper가 응답을 그대로 저장·재생하고, 기존 catalog 호출은 같은 `CatalogMutation<T>`를 구성한다. business 호출은 `BusinessMutation`을 구성한다. 새 공통 응답 envelope 때문에 기존 원응답의 외형을 바꾸지 않는다.

순서는 기존과 같다: 요청 조회/동일 여부→동일 요청이면 recover 성공 뒤 원응답→다른 새 요청도 recover→tx→epoch/revision/head CAS→outbox seq 예약→업무 적용→state revision 갱신→원응답/요청/전체 snapshot→commit→발행 성공 후 응답. 가격 effect는 `apply_catalog_price(tx, validated, seq, time)`를 호출하여 **같은 transaction**에 넣는다. `catalog_add_price()`를 중첩 호출하지 않는다. 테스트 전용 원자성 hook이 필요하면 테스트 코드에만 두며 HTTP에는 노출하지 않는다.

schema 3→4는 기존 백업·transaction·검증 경로에서 추가 관계만 만든다. UUID·epoch·revision·기존 sequence·응답·JSON을 보존하고 이전 업무를 임의 생성하지 않는다. 마이그레이션 자체가 새 업무 seq를 만들지 않는다. 원 DB가 미래 schema거나 구 outbox가 손상/미지원이면 전체 migration rollback과 원본 불변을 검증한다.

새 writer는 **format 2/schema 4**다. header의 `(format_version,schema_version)`으로 v1/schema1~2, v2/schema3, v2/schema4를 먼저 선택한 뒤 각각 고정된 엄격한 DTO로 파싱한다. schema3에는 업무 컬렉션이 없는 것이 정의된 옛 형식이고 schema4에는 아래 컬렉션이 필수다. `serde(default)`로 누락/미래 필드를 빈 자료로 만들지 않는다. v2/schema3 읽기를 현재 SCHEMA_VERSION 비교 하나에 묶지 않는다. 새 format3 도입은 없다.

schema4 state는 기존 catalog state 전체 + business_records/revisions/heads/confirmations/catalog_price_results/corrections를 포함한다. 저장된 업무 payload는 snapshot에서 닫힌 typed 객체로 표현하고 읽을 때도 검증한다. 기존 mutation_requests의 원본문/원응답 문자열은 그대로 남긴다. catalog 변경도 새 업무 컬렉션을 포함한 전체 snapshot을 만든다. 모든 버전은 원본 bytes 그대로 발행·누락 복구하며 재직렬화해 옛 파일을 교체하지 않는다. 전체 복원 엔진은 T-018이다.

## 5. 파일 소유와 병렬 작업

| 담당 | 소유 파일 | 선행/공유 계약 |
| --- | --- | --- |
| A 저장 | 새 `src/store/business.rs`, `src/store/mutation.rs`, `src/store/snapshot.rs`; 수정 `src/store.rs`, `src/store/catalog.rs`; 새 `tests/business_storage.rs`, `tests/fixtures/schema-v3.sql`, `snapshot-v2-schema3.json`; 기존 storage/catalog/hot_journal 기대값 | 루트 DTO/Store 시그니처 확정 후 연결. 기존 DTO는 필요한 가시성만 변경. 가격 SQL과 snapshot 추출은 같은 소유자가 수행 |
| B HTTP | `src/server.rs`, 새 `tests/business_http.rs`, 기존 HTTP 회귀 | A DTO 연결. `/business`와 하위 상세의 SPA 직접 접속 허용도 반영. Host/Origin/session/16KiB/with_store 유지 |
| C UI | 새 `frontend/app/types/business.ts`, `utils/business.ts`, `composables/useBusinessDraft.ts`, `pages/business/index.vue`, `pages/business/[id].vue`, 필요 입력 component; 관련 unit/E2E | 기존 `utils/api.ts`, pending draft 규칙 재사용. app.vue/main.css/material 상세의 가격 초안 진입 링크도 UI 담당 소유 |
| 루트 통합 | 새 `src/business.rs` DTO·엄격 payload/초안 구조 검증과 단위 테스트, `src/lib.rs` 등록, 공개 문서/티켓·검증 기록, 표준 build·통합 회귀 | 각 담당은 commit하지 않음. root가 새 DTO를 독립 검토한 후 병렬 구현 착수 |

모듈 분리는 실제 두 업무(catalog/business)가 공유하는 트랜잭션·snapshot에 한정한다. trait registry나 범용 SQL repository를 추가하지 않는다. DTO를 먼저 고정하면 HTTP/UI와 저장 구현을 병렬로 진행할 수 있다. frontend가 변경 중일 때 Cargo build guard 우회는 하지 않으며 backend RED를 먼저 실행하고 UI 편집 시작 신호를 맞춘다.

## 6. 테스트 RED→GREEN과 실제 UI 흐름

저장 담당은 최신 T-003 자산 상태에서 아래 실패 테스트를 먼저 작성해 `cargo test --locked --test business_storage`의 행동/API RED를 관측한다. 미구현 import 실패와 행동 실패를 구별해 기록한다. UI 작업으로 guard가 먼저 실패하면 이를 RED로 계산하지 않고 정상 build 뒤 다시 확인한다. 공통 추출만 먼저 하면 기존 catalog 테스트를 그대로 통과시킨 후 새 행위를 구현한다.

| ID | 사전 상태·동작 | 기대 결과/검증 책임 |
| --- | --- | --- |
| B01 | 가격 없는 가상 재료, quantity 빈값/amount 0 초안→Store 종료·재개→편집 1kg/0원 | 미완성 입력/ID/revisions 보존; 확정 전 가격 행0·재고값 없음; preview valid; 확정 후 1000g/0원 가격1+confirmation1 같은 seq·request |
| B02 | preview 후 다른 탭 품목 편집 또는 가격 등록→옛 confirm | 409 stale_state, 새 가격/확정 없음; 최신 품목 버전으로 초안을 고쳐 다시 preview 후 확정 가능 |
| B03 | confirm 응답 유실/JSON 발행 실패→동일 UUID 재시도; 후속 변경 뒤 다시 재시도 | 복구 전 성공 금지·다른 UUID 변경 금지; 복구 후 최초 응답 전체/price/event/seq 동일, 파일 누락은 원bytes로 재생성 |
| B04 | 가격 삽입 후 confirmation 또는 outbox 오류 주입 | draft head/가격/confirmation/정정 edge/요청/revision/outbox 모두 rollback. 부분 확정 없음 |
| B05 | 같은 head 두 다른 UUID confirm; 같은 가격의 독립 두 draft confirm | 전자는 효과1, 후자는 효과2. 같은 UUID 다른 body/operation/대상/legacy 명령 충돌. 값을 기준으로 업무를 합치지 않음 |
| B06 | unknown kind/version/필드, 숫자 JSON amount, 잘못된 UUID/길이; 빈값·0수량·다른 차원 | 구조 오류400·저장0. 미완성 값은 draft issues로 저장; confirm은 실패하며 원입력 보존. 0원+양수량은 성공 |
| B07 | draft archive·재시작; confirmed edit/archive 시도 | 원revision+archived head JSON 보존·기본 목록 제외. confirmed 변경은 거절하고 원기록 유지 |
| B08 | 1kg/10원 확정→정정 draft 1kg/20원→확정→다시 30원 정정 | 원10/20/30 가격·각 confirmation 불변, 유효 chain은30. 미확정 정정은 유효값에 영향0. 다른 품목/옛 target 재대체는 실패; 동일 정정 요청은 한 번 |
| B09 | v3 DB+v2/schema3 원문 fixture→v4 열기→draft→catalog 수정 | 기존 ID/epoch/응답/JSON 바이트 보존, 신규 snapshot schema4에 업무 포함. v1·v2/schema3 파일 누락 복구도 원문 그대로 |
| B10 | schema4 필수 컬렉션 누락·unknown business kind/version·미래schema/format·migration 실패 | 명확한 오류, 원본 DB/파일 불변. 미래 입력을 빈 업무로 읽지 않음 |
| H01 | 각 write·preview에 wrong Origin/session/Host와 16KiB 초과 | 기존 경계 유지. preview도 session/Origin 보호, 성공 시 seq/JSON 증가0 |
| H02 | API DTO 직렬화·status/kind 필터·404·오류 code | 위 외형 일치, decimal/seq 문자열, unknown query 거절, 세 code별 UI 판단 가능 |
| H03 | 저장 중 종료 요청·세션 heartbeat 종료 | 수락한 저장을 기다림; 재시작 뒤 완전 확정 또는 미확정만 있고 중간 상태 없음. 기존 lifecycle/socket 테스트 재사용 |

**정상 UI 실증 U01**: 역할은 단일 사용자, 격리한 가상 db에서 정식 release 실행파일과 일반 UI만 사용한다. `가상 초안 재료`를 등록하고 상세의 `가격 초안 작성`으로 진입한다. 수량을 비우고 금액 `0`으로 임시 저장한다. 업무 목록에 입력 필요 상태가 보이고 품목 상세의 가격은 여전히 없음이어야 한다. 브라우저/앱을 정상 종료하고 **같은 db로 프로세스를 실제 재시작**한다. 업무 목록→같은 초안을 열어 `1 kg`, `0원`으로 편집·저장한다. 미리보기의 `1000 g 기준 0원`을 보고 확정한다. 확정 상세의 가격 결과·원본 초안/수정 이력과 품목 상세의 사전 가격0원을 확인한다. 새로고침·한 번 더 프로세스 재시작해 같은 ID/이력을 확인한다. 데이터 만들기/확정에 API 직호출·DB주입을 쓰지 않는다. 테스트 harness의 프로세스 실행/종료와 결과 JSON 읽기는 관측만 한다.

**정상 UI 실증 U02**: U01 확정에서 정정 초안을 열고 금액10원으로 고친다. 미리보기에서 이전0→새10원·원기록 보존을 확인해 확정한다. 원확정→정정 연결과 최신 유효 확정·가격10원을 확인한다. 이 화면에서 과거 재고/판매 원가를 계산했다고 표시하지 않는다.

**UI 회귀 U03**: 별도 탭의 수정 뒤 stale preview, 응답 유실 뒤 재접속/동일요청, request_conflict 자동 재키잉 금지, 저장 중 화면 이동·성공 후 오래된 local draft 부활 금지, 아카이브·빈 목록·필드 오류 focus·키보드·light/dark·390/768/1440px. DB의 저장 초안과 전송 중 local pending을 별개로 표시하고 DB에 저장된 이후에는 local 복사본을 새로운 초안으로 재등록하지 않는다. 기존 T-003 직접 가격 등록 흐름도 계속 동작한다.

실행 순서: 새 저장/HTTP/unit RED→구현→관련 GREEN→`npm --prefix frontend test`/`run typecheck`→`node scripts/build.mjs`→Cargo test/fmt/clippy→`npm --prefix frontend run test:e2e`(신규 business+기존 흐름)→UI manifest/release-artifact 검사. frontend 최종 수정 뒤 Cargo/E2E 전에는 반드시 정상 build를 다시 만든다. 테스트 이름·실패 원인·명령·exit code·실제 OS/브라우저를 로컬 원시 로그와 공개 가능한 검증 요약으로 구분한다.

## 7. 후속 handler 확장 지점

`validate_draft(payload)`는 저장 가능한 구조와 issues, `preview_business(connection, head)`는 실제 입력 검증/사용 버전/효과, `apply_business(tx, validated, confirmation_id, seq, time)`는 같은 tx의 결과 참조를 반환한다. 첫 구현은 enum match의 CatalogPriceV1 한 분기이며 default 성공 분기는 없다. unknown은 오류다. pure helper와 SQL helper를 구분하며 확정은 preview와 같은 검증 함수를 다시 호출한다.

| 티켓 | 추가해야 할 실제 연결 |
| --- | --- |
| T-006 | 주문 payload·주문 결과 관계/상태 handler, 확정/취소가 보유량을 만들지 않는 검증. 주문량·실입고량·차이는 기존 계약대로 |
| T-007 | 직접/주문 입고 payload, 수량/가치·마지막단가·주문종료를 공통 tx에 연결. 지연 입고 임시 보관은 확정 효과와 분리 |
| T-008 | 생산 투입/결과물/가치 배분 handler와 보존 검증 |
| T-009 | 실제 판매 payload·당시 레시피/가격 basis snapshot. 0레시피·재고 부족 허용, 판매 자체 재고 불변, 정상 별개 판매2건 검증 |
| T-010~011 | 품목별 실사/폐기/지연 입고 preview+CAS, 실제 원장/배분·금액 보정. 공통 보정 참조와 도메인 수량/가치 보정은 같은 뜻이 아님 |
| T-012 | 위 업무의 정정 payload와 원본/영향 목록, 같은 preview revision CAS, 불변 원본의 replay와 새로운 보정/배분 결과. 단순 가격 정정 handler를 재고 업무에 재사용하지 않음 |

새 업무를 추가할 때 typed payload/초안 검증/확정 검증/실제 effect/결과 참조/snapshot reader·writer/회귀 테스트를 함께 추가한다. 지원하지 않은 업무를 이름만 받아 저장·확정하는 경로는 없다. 후속 스키마는 이전 snapshot을 명시적으로 읽으며 미래 구조를 조용히 버리지 않는다.

## 8. 수용 기준·완료 판정과 한계

| 기준 | T-005 자체 증거 | 후속 통합 |
| --- | --- | --- |
| 01 | B01/U01: 실제 DB 초안·재시작·가격/재고 무효과 | T-007 이후 실제 수량/가치 행 불변 추가 |
| 02 | 공통 engine에 재고 부족/레시피0 차단 없음; 잘못된 구조/단위 이유 표시. 판매 variant는 아직 미지원 | T-009 실제 판매0·재고부족·재고불변을 검증. 이 부분을 T-005에서 실판매 통과로 표시하지 않음 |
| 03 | B01/B04: 실제 가격 효과·확정·당시 버전·시각·요청·JSON 원자성 | T-007~011의 실제 수량/가치/배분 행까지 추가 |
| 04 | B03/B05: 원응답 재생·동일키 다른본문 거절·독립 동일내용 업무2건 | T-009 정상 판매2건과 교차 확인 |
| 05 | B01/B03/B07~10: 저장/수정/아카이브/확정/정정 전체 JSON, 복구 | 후속 kind와 사진·전체복원은 각 티켓 |
| 06 | B07/B08/H01/H03/U02: 원본·정정 chain·유효상태·종료/세션 | T-012 실제 영향 미리보기·수량/가치/소모 보정 |

제품 착수 전 루트와 독립 리뷰어가 DTO/이전 JSON/정정 chain/검증 경계를 확인한다. 완료 시 실제 결과만 T-005 검증 문서에 대응하고 RTD After 13개를 수행한다. 이 계획 파일의 작성은 제품 검증 성공이나 티켓 완료를 뜻하지 않는다. T-001 별도 장비·절전, T-018 전체 복원, T-019 실제 이동의 외부 한계는 유지한다.

## 9. 착수 근거와 독립 검토 보완

T-003 공개 커밋 뒤 Before 4개와 독립 설계 검토를 완료했다. 정정의 원가격/현재 사전가격/새 가격, 필수 정정 이유, 원본부터 유효 확정까지의 상세 체인과 목록 입력 문제 개수를 계약에 보완했다. 새 운영 질문은 발견하지 않았다.

저장 테스트는 새 business 모듈/API 없음으로 import/API RED(exit101)를 관측했고 HTTP 테스트 4개는 컴파일 후 새 목록·화면 404와 POST 405로 행동 RED(exit1)를 관측했다. UI guard나 파일 권한 실패가 아니었다. 이후 DTO/구조 검증·저장·HTTP·UI 구현을 분담하며 GREEN은 최종 UI 산출물을 정상 빌드한 뒤 확인한다.

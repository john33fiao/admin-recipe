# T-003 재료·상태·단위와 사전 가격 구현 계획

> 실행 담당자는 `superpowers:subagent-driven-development` 또는 `superpowers:executing-plans`의 작업별 구현·검증 순서를 적용한다. 아래 체크는 계획의 실행 상태이며 이 파일 작성만으로 구현 완료를 뜻하지 않는다. 사용자가 승인한 스테이징·커밋의 최종 수행은 통합 담당자가 맡으며 개별 구현 담당자의 파일 소유 범위에는 포함하지 않는다.

- 작성일: 2026-10-24
- 상태: 구현·통합 검증 완료. 실제 실행과 계획의 차이는 검증 기록에 명시
- 목표: 기존 재료 UUID를 보존하면서 종류·설명·단위 표시·검색·버전/아카이브·사전 가격 입력과 조회를 제공한다.
- 구조: 기존 Store의 단일 작성자·트랜잭션·outbox와 HTTP 세션/종료 경계를 재사용한다. 숫자/단위/가격 선택은 순수 함수로 분리하고 품목 SQL·DTO·Nuxt 입력을 연결한다.
- 기술: 현재 Rust 1.98+, Axum·rusqlite·serde, Nuxt/Vue/TypeScript·Node 테스트·Playwright. 새 DB/프런트엔드 프레임워크를 도입하지 않는다.
- 기준: [T-003](T-003-materials-and-units.md), [계산 계약](../DOMAIN-CONTRACT.md), [T-020 계획](T-020-plan.md), [T-005](T-005-business-history.md)

## 제약과 현재 코드 근거

- 실제 구현은 `src/store.rs`의 `Material/CreateMaterial`, `list/get/create`, schema v2, JSON v1에 한정된다. 이름/단위와 UUID만 있으며 종류·설명·수량·가격·수정 이력은 없다.
- `creation_requests`는 request UUID별 원래 이름/단위/재료 ID를 보존한다. `create()`는 같은 요청의 다른 내용을 거절하고 복구 후 원래 재료 ID를 반환한다. 신규 요청은 재료·요청·전체 snapshot 원문을 한 트랜잭션에 저장한다.
- `recover()`는 모든 보존 원문과 JSON을 대조하고 누락 파일을 다시 발행한다. `migrate()`는 백업 뒤 한 트랜잭션에서 변환·검증한다. 미래 DB와 hot journal 보호·writer lock·종료 직렬화를 유지한다.
- `src/server.rs`는 `/api/materials`·`/{id}`와 16KiB 입력 제한을 제공한다. `frontend/app/composables/useMaterialRegistration.ts`는 불확실한 요청 UUID/본문을 sessionStorage에 보관한다. T-020의 전송 전 초안·포커스·상태 처리와 합친다.
- `tests/storage.rs`는 v1→v2 fixture·재시도·JSON 실패·불변 원문·원자성을 검증한다. 현재 신규 snapshot을 v1/schema2로 고정한 검사는 새 출력 기대값으로 바꾸되 별도 v1 원문 fixture와 호환 검사를 보존한다. 기대값 수정만으로 과거 회귀를 지우지 않는다.
- 수량/금액은 계약의 6자리 고정소수 정수, API/JSON 숫자는 십진 문자열이다. 수량 상한 9,000,000,000 기준 단위, 금액 상한 9,000,000,000,000원이다. g/kg·mL/L만 환산하며 자유 단위는 같은 문자끼리만 계산한다.
- 실제 재고/입고·생산·판매·실사·전체 복원은 이 티켓에서 만들지 않는다. 사전 가격 저장이 실제 수량/보유 가치를 만들면 실패다. 실제 운영 자료·비공개 경로·링크는 공개 테스트/문서에 넣지 않는다.

## 변경 전 검토

[Step 1] 변경 계획 - PASS: T-003-01~06을 품목 버전/가격·검색·수정/아카이브·호환 구현으로 나누고 아래 API/소유 파일/검증을 고정한다. 이번 계획 작성의 변경 파일은 이 문서뿐이다. 제품 구현은 T-020 이후 진행하며 스키마 변경은 백업/롤백 테스트로 되돌릴 경계를 검증한다.

[Step 2] 계획 검토 - PASS: 기존 DB·JSON·UUID 재시도·일괄 실패·새로고침 입력 보존을 확인했다. 실제 재고 원장은 없으므로 T-003-03의 보유 평균/마지막 단가를 순수 resolver 검증과 T-007의 실제 연결 검증으로 나눈다. 현재 정책·MIT·공개 경계를 유지하며 T-005 업무 초안/이벤트 테이블을 앞당겨 만들지 않는다.

[Step 3] 검토 재검토 - PASS: (1) v2에는 종류·수량·가격 증거가 없음, (2) 사전 가격의 0원과 없음은 다름, (3) 요청 재시도는 현재 품목을 다시 생성하거나 편집해서는 안 됨을 확인했다. 따라서 종류 미지정/재고 미확인·불변 가격 버전·원본 요청 결과를 설계한다. 이 전제가 바뀌면 마이그레이션과 API 검증을 다시 검토한다.

[Step 4] 계획 단순화 - PASS: 기존 URL/Store 흐름을 확장하고 순수 수치·단위·가격 선택과 품목 SQL만 분리한다. 재고 이벤트 저장소·일반 업무 초안·분산 큐·전체 복원 엔진·고급 필터는 추가하지 않는다. 새 타입·helper는 실제 두 개 이상의 품목 변경 경로에서 필요한 범위로 한정한다.

## 검토 중점

1. 이름이 같아도 ID가 다른 두 품목, 한글·공백·`%`/`_`를 포함한 검색이 구분되어야 한다. 작업 C/UI의 검색 테스트로 확인한다.
2. 사전 가격 0원·가격 없음·미확인 보유 가치를 구분하고 `Q=0,V>0`을 만들지 않는다. 작업 A의 resolver·작업 B 저장 테스트로 확인한다.
3. 같은 요청 재전송과 409 이후 새 입력을 구분하며 JSON 실패/새로고침으로 가격 버전을 중복 추가하지 않는다. 작업 B/C/UI의 실패 시나리오로 확인한다.
4. kg 표시 변경 뒤 이전 g 가격/버전과 자유 단위가 변형되지 않는다. 작업 A/B의 단위·버전 테스트로 확인한다.
5. v1 JSON이 남아 있는 v2 DB 변환·재시작·파일 누락 복구·미래 버전 거절이 동시에 성립해야 한다. 작업 B의 원문/fixture·실패 테스트로 확인한다.

## 1. 최소 데이터 확장

### 품목과 가격

기존 `materials`를 품목 ID·최초 이름/단위·생성 시각의 원본으로 보존하고 다음 관계를 schema v3에 추가한다. 새 조회는 현재 버전을 조인하며 과거 행을 덮어쓰지 않는다.

| 관계 | 저장할 필드 |
| --- | --- |
| `material_versions` | version UUID, material UUID, 이름, 종류, 설명, 표시 단위, 차원/기준 단위, archived, 생성 시각, 출처, optional seq(이전 자료 변환만) |
| `material_heads` | material UUID → current version UUID |
| `material_prices` | price UUID, material UUID/당시 version UUID, 기준 수량 정수, 금액 정수, 원입력 수량/단위, 생성 시각, seq |
| `mutation_requests` | request UUID, 명령 종류, 정규화 본문, 불변 응답 원문/참조, epoch, seq |
| `catalog_state` | 한 행의 epoch·revision. 모든 성공한 영속 변경에서 revision 증가 |

종류는 `raw`(원물), `prepared`(손질재료), `stocked`(비축품), `menu`(판매메뉴)다. v2 자료는 종류 증거가 없으므로 마이그레이션 전용 `unclassified`(종류 미지정)를 둔다. 새 UI 등록은 원물을 기본 선택하되 네 종류 중 변경할 수 있고, 이전 자료의 종류 수정은 새 버전으로 기록한다. 같은 이름·종류도 UUID가 다르면 별개다. 원물→손질→비축의 실제 투입/결과 연결은 T-008에서 추가한다.

이름 1~100 Unicode 문자·관리 단위 1~20 문자·제어문자 금지는 유지한다. 설명은 0~2000 Unicode 문자로 제한하고 줄바꿈은 허용하되 NUL 등 본문이 아닌 제어문자는 거절한다. 종류·설명·같은 차원 표시 단위 수정과 아카이브는 새 `material_versions` 행으로 기록한다. 다른 차원 변경 요청은 400과 새 품목 등록 안내로 끝내며 기존 값을 바꾸지 않는다.

가격은 금액 0 이상, 기준 수량 양수여야 한다. kg 입력은 g, L 입력은 mL 기준 수량으로 환산하며 원입력 수량/단위도 보존한다. 최신 가격은 서버 확정 seq로 선택하고 가격 수정은 새 행 추가다. 실제 재고 필드를 0으로 채우는 테이블이나 가짜 입고 행을 만들지 않는다.

### 순차 마이그레이션과 재시도

1. 현재 v1 합성 fixture를 유지하고 **실제 T-001 구조의 v2 fixture**를 추가한다. 가상 재료 두 개, 같은 이름·서로 다른 단위, 요청 UUID, 실제 JSON v1 원문과 outbox를 포함한다. UUID·시각·원문은 상수여야 한다.
2. v1→v2→v3를 기존 마이그레이션 트랜잭션에서 수행한다. v2 자료마다 UUID를 유지하고 최초 버전/head를 만든다. 종류 미지정·설명 빈 값·아카이브 false·가격 없음·재고 기준 미확인을 적용한다. 단위 문자열은 보존한다. 대응하는 기존 생성 snapshot 순번을 확인할 수 있으면 그 순번을 참조하고, 없으면 `legacy_migration` 출처와 `seq: null`로 구분한다. 새 업무 순번을 꾸며 만들거나 양의 실제 seq 대신 0을 넣지 않는다.
3. 기존 `creation_requests`를 삭제하지 않는다. 기존 request UUID와 명령 종류/정규화 이름·단위를 대조하는 호환 조회를 유지한다. 신규 `mutation_requests`와 같은 UUID 공간을 사용하여 서로 다른 명령에 같은 UUID를 쓸 수 없게 한다. 옛 UUID에 새 kind/description 등 입력이 추가되면 이름/단위가 같아도 같은 요청으로 인정하지 않고 409를 반환한다.
4. 공통 helper는 먼저 요청 중복/본문을 대조한다. 이미 처리한 동일 요청이면 **기존 `recover()`가 성공한 뒤에만** 보존한 응답/seq를 반환한다. 새 요청도 기존 `recover()`가 성공한 뒤 epoch/revision 확인→업무 변경→불변 응답·snapshot 원문 기록→commit→발행을 실행한다. 미해결 JSON 기록이 있으면 후속 다른 UUID의 변경을 막는다. **이미 처리한 요청 조회는 오래된 revision 검사보다 먼저** 수행하지만 저장 성공 판정의 발행/복구 게이트를 건너뛰지 않는다.
5. 기존 요청 호환 응답은 원본 등록 시 이름/단위/ID·생성 시각을 보존한다. 현재 화면 조회는 head를 사용한다. 이름 수정 후 옛 등록 요청을 재시도해도 최신 이름을 새 원본 등록 결과처럼 덮어쓰지 않는다.
6. 새 요청의 stale epoch/revision/expected_version은 `StoreError::StaleState(String)`과 HTTP 409 `stale_state`다. 미처리 요청은 기록하지 않고 입력/미리보기를 갱신하게 한다. 기존 `StoreError::Conflict(String)`은 요청 ID/본문 충돌인 `request_conflict` 의미로 유지한다. 성공 응답에는 적용된 revision을 담아 같은 화면의 후속 변경이 이를 사용한다.

마이그레이션은 기존 `backup_before_migration`, `check_existing_version`, `validate_schema`, `recover` 경로를 확장한다. fixture의 hot journal·미래 스키마를 v3 기준으로 다시 확인한다. 테스트가 미래 버전을 숫자 3으로 가정했다면 새 현재 버전과 구분되는 fixture로 바꾸되 원본 바이트 불변 검사는 유지한다.

### snapshot v2의 최소 범위

형식 v2/schema v3의 `state`는 `materials` 원본, `material_versions`, `material_heads`, `material_prices`, 현재 epoch/revision을 포함한다. 원본 요청 재실행 방지 자료는 DB에 계속 보존하고, JSON만으로 구조화 상태를 복원할 때 필요한 요청 참조/결과도 함께 포함한다. outbox 원문 자체를 snapshot에 재귀적으로 넣지 않는다.

T-005가 없는 지금 업무·초안·배분 테이블을 빈 껍데기로 만들지 않는다. 후속 schema에서 실제 관계를 추가하고 옛 snapshot을 명시적으로 변환한다. `format_version`과 `schema_version`을 별도로 분기하고 지원하지 않는 새 스키마/업무 필드를 조용히 버리지 않는다.

v1 decoder는 기존 `sequence` 숫자·`materials` 배열을 그대로 읽고 원문을 보존한다. v2 writer는 `sequence`/수치 필드를 계약의 문자열로 쓴다. v1/v2 모두 기존 immutable 파일 발행·누락 복구·불일치 거절을 통과해야 한다. 마이그레이션만으로 과거 snapshot을 v2로 재작성하지 않는다. T-018이 구현할 전체 복원 버튼/엔진은 이 작업에 넣지 않는다.

## 2. 순수 수치·단위·가격 인터페이스

작업 A는 아래 타입/함수를 공개하고 저장·UI 담당자가 같은 이름으로 사용한다. 타입 내부의 정수 필드는 private으로 두고 파서를 통과한 값만 만든다.

| 경로 | 인터페이스/책임 |
| --- | --- |
| `src/domain/mod.rs` | `Quantity`, `Money`, `Unit`, `UnitPrice`, `HoldingBasis`, `ResolvedPrice`, `DomainError` 재내보내기 |
| `src/domain/number.rs` | `Quantity::parse(&str) -> Result<Quantity, DomainError>`, `Money::parse(&str) -> Result<Money, DomainError>`, 각각 `from_micros(i64) -> Result<Self, DomainError>`, `micros() -> i64`, `decimal() -> String`; 문자열 Serialize/Deserialize; 검사된 `round_ratio(i128, i128) -> Result<i128, DomainError>` |
| `src/domain/unit.rs` | `Unit::parse(&str) -> Result<Unit, DomainError>`, `dimension() -> &str`, `canonical() -> &str`, `same_dimension(&Unit) -> bool`, `to_canonical(Quantity) -> Result<Quantity, DomainError>` |
| `src/domain/price.rs` | `resolve_price(HoldingBasis, Option<UnitPrice>, Option<UnitPrice>) -> ResolvedPrice`; 마지막 인자는 사전 가격. `UnitPrice::new(amount: Money, quantity: Quantity, source_id: String) -> Result<UnitPrice, DomainError>`는 양수 quantity를 보장하며 필드는 private |

`HoldingBasis`는 `NotEstablished`, `Known { quantity: Quantity, value: Money, source_id: String }`, `ValueUnknown { quantity: Quantity }`를 구분한다. `ResolvedPrice`는 Serialize 가능한 `{ source, source_id, amount: Option<String>, quantity: Option<String>, reason: Option<String> }`이며 source=`holding_average | last_valid | preregistered | missing`, source_id는 원본 ID 또는 없음이다. amount/quantity는 원/기준 단위의 정확한 비율을 표현한다. 보유량 양수·금액0은 유효한 holding_average다. 양수 수량/미확인 가치는 missing이며 사전 가격으로 몰래 대체하지 않는다. 보유량0은 마지막 단가→사전 가격, 아직 보유 기준 없음은 사전 가격→missing이다. 검증된 생성자 외 방법으로 0 분모의 UnitPrice를 만들지 않는다.

T-003의 실제 저장 adapter는 `HoldingBasis::NotEstablished`, 마지막 단가 `None`, 등록 가격만 넘긴다. `holding_average`/`last_valid` 분기는 가상 인수의 순수 테스트로 검증한다. 이후 T-007이 실제 수량/가치/마지막 단가를 제공해 같은 함수를 재사용한다. T-003-03 기록에는 **resolver 전체 분기 검증과 실제 화면의 사전 가격/없음 표시 완료**, **실제 보유 원장 연결 미검증·T-007 담당**을 따로 적는다.

## 3. API·UI 경계

경로와 기존 응답의 외형을 유지한다. `GET /api/materials`는 배열 그대로, 기존 상세/등록 응답은 최상위 `id/name/unit/created_at`을 유지하고 새 필드만 추가한다. 배열을 envelope나 `{ materials }`로 감싸지 않는다. 별도 `GET /api/state`가 `CatalogState = { epoch: string, revision: string }`를 반환한다. 상세와 쓰기 응답에는 같은 metadata를 추가할 수 있다. 기존 T-001 HTTP/E2E의 배열·ID/단위 접근을 보존한다.

공통 변경 입력은 `MutationMeta = { request_id: string, epoch: string, expected_revision: string }`다. 새 등록은 `/api/state`, 수정/가격/아카이브는 그 품목 상세를 읽을 때 원자적으로 받은 metadata를 사용한다. 목록과 별도 state 읽기를 하나의 원자적 스냅샷처럼 취급하지 않는다. 전송 후 불확실한 입력은 metadata까지 그대로 재시도한다. `stale_state`로 미처리였음이 확인되면 새 상태를 읽고 사용자 입력을 보존한 채 사용자가 재확인할 때 새 요청 ID로 전송할 수 있다. `request_conflict`를 오래된 화면 상태로 오인해 새 UUID로 자동 재전송하지 않는다.

오류 응답은 기존 `error` 문자열을 유지하고 안정된 `code`를 추가한다. HTTP 담당 C가 다음 매핑을 노출하며 UI는 메시지 문구나 409만으로 원인을 추정하지 않는다.

| Store 오류 | HTTP 상태 | code | UI 처리 |
| --- | --- | --- | --- |
| `Validation(String)` | 400 | `validation` | 입력 오류를 표시하고 값 보존 |
| `Conflict(String)` | 409 | `request_conflict` | 동일 UUID의 다른 명령/본문 충돌을 알림. 자동 새 UUID 재전송 금지 |
| `StaleState(String)` | 409 | `stale_state` | 미저장 상태로 최신 자료를 다시 읽고 사용자의 재확인 뒤 새 UUID 허용 |
| `Unavailable(String)` | 503 | `unavailable` | DB 저장 여부가 불확실할 수 있으므로 같은 UUID/본문 재시도 유지 |

| HTTP | 요청 | 응답 |
| --- | --- | --- |
| `GET /api/state` | 없음 | `CatalogState` |
| `GET /api/materials?query=...&archive=active` | query 기본 빈값, archive=`active | archived | all` 기본 active | `MaterialSummary[]` |
| `GET /api/materials/{id}` | UUID | `MaterialDetail` |
| `POST /api/materials` | meta + `{ name, unit, kind, description }` | 생성한 `MaterialSummary`에 적용 epoch/revision 추가 |
| `POST /api/materials/{id}/versions` | meta + `{ name, unit, kind, description, expected_version_id }` | 새 `MaterialSummary`에 적용 epoch/revision 추가 |
| `POST /api/materials/{id}/archive` | meta + `{ expected_version_id }` | archived=true인 새 `MaterialSummary`에 적용 epoch/revision 추가 |
| `POST /api/materials/{id}/prices` | meta + `{ quantity: string, unit: string, amount: string, expected_version_id }` | `MaterialPriceVersion`에 적용 epoch/revision 추가 |

`MaterialSummary`는 `{ id, version_id, name, unit, canonical_unit, dimension, kind, description, archived, created_at, updated_at, price }`다. `unit`은 표시 단위, `dimension`은 `mass | volume | custom`이고 custom의 식별은 canonical_unit 원문이다. `price`는 위 `ResolvedPrice`의 JSON DTO다. 재고 필드를 넣어 수량0처럼 보이게 하지 않는다.

`MaterialDetail`은 최상위 `MaterialSummary` 필드를 그대로 두고 `versions`, `prices`, `epoch`, `revision`을 추가한다. 버전에는 ID·변경 시각/seq·출처·이름/종류/설명/단위/아카이브를, 가격 버전에는 ID·당시 품목 버전·canonical quantity/amount 문자열·원입력 수량/단위·시각/seq를 담는다. 이력은 seq 오름차순이며 seq가 없는 이전 자료의 최초 버전은 시작 위치에 두고 생성 시각·ID로 정렬한다.

검색은 바깥 공백을 trim한 문자열의 이름 부분 일치다. 빈 문자열은 선택한 archive 모드 전체를 반환하고 `%`·`_`를 와일드카드로 해석하지 않는다. 같은 이름은 합치지 않으며 `created_at, id`의 안정된 순서를 유지한다. 별칭·고급 필터·전문 검색 인덱스는 넣지 않는다.

기존 `Store::create(CreateMaterial)`와 HTTP의 `{ request_id, name, unit }` 등록 입력도 보존한다. 이 좁은 옛 형식은 기존 정규화/요청 비교를 적용하고 종류 미지정·빈 설명으로 새 등록 내부 함수에 위임하여 버전·snapshot을 빠뜨리지 않게 한다. 새 UI 형식은 metadata를 필수로 검사하고 종류/설명/가격 필드가 있는데 metadata가 없으면 옛 형식으로 fallback하지 않는다. 수정/가격/아카이브에는 옛 형식의 예외가 없다. 같은 UUID 공간에서 서로 다른 명령 재사용은 항상 409다. T-018에서는 복원 전 화면의 옛 요청도 재적용하지 않는 세션/epoch 경계를 통합 검증한다.

UI는 기존 목록 입력에 종류·설명을 추가하고 검색/아카이브 조회를 연결한다. 상세에는 현재 정보 편집, 사전 가격 등록, 가격/정보 이력, 아카이브 동작을 둔다. 0원과 가격 없음·종류 미지정을 명확히 표시한다. 같은 이름 후보에는 종류·단위와 ID 일부를 함께 보여준다. 단위 차원이 달라지는 편집은 기존 입력을 보존하며 새 품목 등록을 안내한다. 아직 없는 입고/재고·레시피 버튼은 추가하지 않는다.

## 4. 작업 분할과 파일 소유

| 작업/담당 경계 | 생성·수정 파일 | 선행/공유 경계 |
| --- | --- | --- |
| A 수치/단위/가격 | 새 `src/domain/mod.rs`, `number.rs`, `unit.rs`, `price.rs`, `tests/domain.rs` | 독립. `src/lib.rs` 등록은 통합 담당자 |
| B 저장/호환 | 새 `src/catalog.rs`(DTO/검증), `src/store/catalog.rs`(품목 SQL); 수정 `src/store.rs`; 새 `tests/catalog_storage.rs`, `tests/fixtures/schema-v2.sql`, `tests/fixtures/snapshot-v1.json`; 기존 저장/hot-journal 테스트 | A 인터페이스. Store 소유자는 한 명; snapshot·마이그레이션·요청 helper 변경도 같은 담당자 |
| C HTTP 통합 | 수정 `src/server.rs`, `src/lib.rs`, 새 `tests/catalog_http.rs`, 기존 `tests/http.rs` 요청 helper | B가 공개한 Store 메서드·DTO 사용. 세션/lifecycle 로직 재설계 없음 |
| D UI | 수정 `frontend/app/types/material.ts`, `utils/material.ts`, `composables/useMaterialRegistration.ts`, 목록/상세 페이지; 필요 시 새 `composables/useMaterialEditor.ts`, `components/MaterialPriceForm.vue`; 기존 단위 테스트·새 `frontend/e2e/catalog.spec.ts` | T-020 완료본·위 API 타입. 전역 CSS/탐색은 통합 담당자와 한 번에 반영 |
| E 통합/기록 | 관련 README·구조·T-003 상태/검증 기록, 빌드·전체 회귀 | A~D 완료 후. T-005·T-007의 미완료 범위를 완료로 바꾸지 않음 |

B가 제공할 Store 인터페이스는 `catalog_state()`, `catalog_list(CatalogFilter)`, `catalog_detail(&str)`, `catalog_create(CreateCatalogMaterial)`, `catalog_revise(&str, ReviseCatalogMaterial)`, `catalog_archive(&str, ArchiveCatalogMaterial)`, `catalog_add_price(&str, CreateCatalogPrice)`다. 읽기는 각각 `Result<CatalogState, StoreError>`, `Result<Vec<MaterialSummary>, StoreError>`, `Result<Option<MaterialDetail>, StoreError>`다. 쓰기는 `Result<CatalogMutation<T>, StoreError>`를 반환한다. `CatalogMutation<T>`는 value와 적용 metadata를 저장하며 HTTP 직렬화는 value 필드를 최상위로 펼쳐 기존 외형을 보존한다. 새 UUID는 서버에서 만든다. `CatalogFilter`와 변경 DTO는 `src/catalog.rs`에 한 번 정의하고 HTTP/UI 계약과 이름을 맞춘다.

새 `src/store/catalog.rs`는 `Store`의 하위 모듈로 기존 connection/root 경계를 사용한다. snapshot/outbox를 직접 우회하는 두 번째 저장 경로를 만들지 않는다. 기존 `store.rs`에서 실제 공통으로 쓰는 `record_snapshot`·`lookup_request`·`record_request` 정도만 추출하고 T-005의 일반 업무 엔진을 미리 구현하지 않는다. 병렬 편집은 A와 D의 독립 파일에 한정하고 Store/server/lib 통합은 순서대로 한다.

## 5. 구현·검증 순서

### A. 수치·단위·가격 resolver

- [x] `tests/domain.rs`에 `decimal_boundaries_and_zero_are_exact`, `unit_dimensions_do_not_cross`, `price_priority_keeps_zero_and_missing_distinct`를 먼저 작성한다. `"0.000001"`→정수1, `"0.0000001"`/지수/음수/상한 초과 거절, 1.25kg→1250g, g→mL 거절을 고정한다.
- [x] `cargo test --locked --test domain`으로 실패를 확인한다. UI 산출물이 오래됐으면 먼저 정상 빌드로 갱신하고 누락된 모듈/함수에 의한 실패와 빌드 환경 실패를 구분한다.
- [x] §2 인터페이스를 구현한다. resolver는 보유 100g/0원→holding 0, 보유0+마지막10원/g→last_valid, 미설정+사전0원/100g→preregistered 0, 모두없음→missing을 반환한다.
- [x] 같은 테스트를 통과시키고 `cargo fmt --check`를 확인한다. 순수 계산 단계에서 실제 재고가 저장됐다고 보고하지 않는다.

### B. schema v3·품목 버전·가격·snapshot

- [x] `tests/catalog_storage.rs`에 `v2_migration_preserves_ids_requests_and_v1_bytes`, `revisions_and_price_history_survive_restart`, `archive_and_dimension_edit_preserve_old_versions`, `price_does_not_create_holdings`, `same_request_replays_original_response_after_revision`, `legacy_key_rejects_new_payload_fields`, `snapshot_failure_and_retry_append_once`를 추가한다.
- [x] v2 fixture의 원래 재료 UUID/이름/단위/JSON 바이트, 신규 0원 가격 한 행, 수정 후 이전 이름/단위와 현재 head, 아카이브 기본 제외·상세 조회, g→mL 거절시 변화0을 기대값으로 고정한다. 옛 UUID에 동일 이름/단위와 새 종류/설명을 붙이면 409다. JSON 발행 실패 뒤 조회에는 DB 결과가 있을 수 있으나 동일 요청 복구 후 버전/가격/snapshot은 한 번이어야 한다. snapshot 부재→재시도는 원문과 같은 파일 생성 후에만 성공하며, 계속 발행 실패하면 성공 응답을 금지하고 다른 UUID의 변경도 막는다.
- 실행 제약: 관련 테스트의 최초 실행은 UI guard로 차단되어 구현 전 기능 RED는 미관측이다. §1의 마이그레이션·공통 요청/기록 helper와 품목 메서드를 구현한 뒤 outbox 강제 실패에서 품목 버전/가격/요청 전체 롤백을 실제 검증했다.
- [x] 현재 `tests/storage.rs`, `tests/hot_journal.rs`와 새 저장 테스트를 통과시킨다. v1/v2 원문 누락 복구·변조 거절·future schema 무변경·실패 rollback을 함께 확인한다.

### C. API

- [x] `tests/catalog_http.rs`에 `catalog_writes_require_origin_session_and_revision`, `search_treats_wildcards_as_text_and_filters_archive`, `price_and_revision_roundtrip_use_decimal_strings`, `stale_revision_has_distinct_code_without_changes`, `request_conflict_code_does_not_allow_automatic_rekey`를 추가한다.
- [x] 같은 이름 두 재료, 이름 `가상 %_ 재료`, query 바깥 공백/빈 결과, 옛 형식 등록/재시도·신규 형식의 metadata 누락 거절, 숫자 JSON을 금액 필드에 보낸 입력 거절을 확인한다. 기존 목록 배열·상세/등록 최상위 ID·단위 외형도 검사한다.
- [x] 라우트와 JSON DTO를 연결하고 현재 HTTP/소켓 회귀를 통과시킨다. 기존 16KiB 제한·Host/Origin/세션·오류 상태 코드는 유지한다.

### D. UI·실제 흐름

- [x] 프런트엔드 숫자 문자열·metadata 포함 pending 복구·같은 요청 불변·오류 코드별 입력 보존 테스트를 작성한다. `stale_state`에서 최신 자료 재조회/사용자 재확인 후에만 새 UUID를 만들고, `request_conflict`에서는 자동 재키잉/재전송하지 않으며, `unavailable`에서는 같은 UUID를 유지하는지 확인한다. T-020의 전송 전 초안/연결/포커스 동작을 재사용한다.
- [x] 목록 등록/검색, 상세 수정/가격/이력/아카이브를 연결한다. `MaterialPriceForm`은 가격 기본 수량을 임의 보유량으로 보이지 않게 표시하고 0원·없음 출처를 구분한다.
- [x] `catalog.spec.ts`에 재료등록→0원 가격→가격 수정→이름/kg 표시 수정→이력→아카이브→검색 모드→정상 재시작 흐름을 작성한다. JSON 실패/응답 유실→새로고침→같은 요청 재시도도 한 번 실행한다. 기존 E2E에서 신규 snapshot의 조회 경로만 `state.materials`로 맞추며 ID/단위·JSON 보존 assertions는 유지하고 v1 원문 호환은 별도 fixture로 검증한다.
- [x] 라이트/다크·390px/768px/1440px에서 주요 화면, 키보드 첫 오류/완료 포커스, 긴 이름/설명, 로딩/빈 검색/네트워크 오류·재시도를 확인한다. 이 API/자료는 모두 가상이다.

### E. 통합과 수용 기준

- [x] `node scripts/build.mjs`로 최신 UI·manifest·release를 만든다.
- [x] `cargo fmt --check`, `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, `npm --prefix frontend test`, `npm --prefix frontend run typecheck`를 실행한다.
- [x] `node --test scripts/ui-manifest.test.mjs`, `node scripts/ui-manifest.mjs --check`, `node --test scripts/release-artifact.test.mjs`, `npm --prefix frontend run test:e2e`를 실행한다. 실패가 없다면 같은 검사를 불필요하게 반복하지 않는다.
- [x] T-003-01/02/04/05/06을 저장/API/UI 증거와 대응한다. T-003-03은 resolver 분기와 실제 사전 가격 화면 증거, T-007에서 해야 할 실제 보유 평균/마지막 단가 연결을 구분한다.
- [x] README·구조·티켓에 실제 구현 상태와 스키마/JSON 형식을 맞추고 프로젝트 RTD After 13개를 수행한다. T-001 별도 장비/절전·T-018 전체 복원·T-019 전체 이동을 완료로 표시하지 않는다.

## 6. 계획 자체 점검과 인계

T-003-01은 품목 종류/ID·목록과 상세, 02는 가격 버전/0원·수량 미생성, 03은 resolver/후속 T-007, 04는 버전/아카이브, 05는 같은 차원 표시 변경/다른 차원 거절, 06은 검색/재시작 테스트로 대응한다. 위 검토 중점의 다섯 실패 유형은 각각 소유 작업의 테스트에 연결했다. 계획의 DTO·메서드 이름은 섹션 간 동일하다.

선택한 구조는 기존 Store·snapshot을 확장하는 안이다. 별도 업무 저장소/일반 이벤트 엔진을 먼저 만드는 안은 T-005와 중복되고 이전 요청/복구 경로를 둘로 나누므로 채택하지 않는다. 계획의 파일 분리는 숫자·단위·가격/품목 SQL/HTTP/UI의 실제 독립 작업에 한정한다.

이 파일 작성 단계에서는 제품 코드·DB·snapshot fixture를 변경하지 않았고 테스트 실행 성공을 주장하지 않는다. 실행은 T-020 완료와 이 계획의 기술 검토 후 위 순서로 진행한다. 사용자의 실행 위임에 따라 추가 운영 인터뷰나 커밋 승인 요청을 만들지 않는다.

## 7. 실행 완료와 계획 차이

실제 결과는 [T-003 검증 기록](T-003-verification.md)에 대응한다. A의 모듈 누락 RED와 UI 두 경합 RED를 확인했으며 B의 최초 실행은 UI guard로 차단되어 기능 RED를 관측하지 못했다. 위 B 체크는 실패 실행 시도·제약 기록 및 이후 8개 저장 GREEN을 포함한 완료 의미이고, 관측하지 않은 RED를 통과로 바꾸지 않는다. 계획의 테스트명은 구현에서 같은 책임별 시나리오로 묶었다. C의 lib 등록은 통합 담당자가 했고 가격/수정 폼은 MaterialEditor 하나로 합쳤다. 최종 Rust54·UI단위11·브라우저17 및 표준 명령의 실제 결과를 확인했다.

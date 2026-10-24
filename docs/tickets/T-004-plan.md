# T-004 레시피·메뉴 복제와 원가 구현 계획

- 작성일: 2026-10-24
- 기준: [티켓](T-004-recipes-and-costs.md), [요구사항](../REQUIREMENTS.md), [계산 계약](../DOMAIN-CONTRACT.md), [ADR-007](../decisions/ADR-007-consumption-and-ticket-scope.md)
- 선행: T-002 계약·T-020 UI·T-003 품목·T-005 공통 저장·T-006 발주·T-007 실제 입고/보유 검증 완료. T-005의 실제 판매 조건은 T-009에 남는다.
- 상태: 실행 완료 — 설계·독립 검토와 행동 RED 후 구현·관련 검증·RTD After를 완료했다. 실제 근거는 [검증 기록](T-004-verification.md)을 따른다.

## 1. 범위와 RTD Before

기존 품목에 불변 레시피 버전과 직접 구성 행을 연결한다. 메뉴 기준 개수·비축 참고 생산량·조리 설명/공정/참고 시간·판매가, 현재 가격으로 계산한 식재료비·원가율, 이름 검색·양방향 참조·독립 복제를 구현한다. 미저장 입력/불확실 요청과 저장된 버전을 구별한다.

실제 생산·판매·재고 차감·실사 배분·일반 정정·사진·실제 작업시간·세금/할인/결제는 제외한다. 과거 판매 참조는 합성 계약으로 검사하며 실제 판매 완료는 T-009/T-021에서 확인한다. 운영 자료·최종 시연 자료는 공개 코드/fixture에 쓰지 않는다.

- [Step 1] 변경 계획 - PASS: 기준01~07을 아래 DTO·저장·원가·UI와 검증 행에 연결했다. 기존 코드와 공개 자료만으로 수행할 수 있으며 변경 전 실행 파일/가상 fixture·migration 백업으로 이전 원문을 비교한다.
- [Step 2] 계획 검토 - PASS: 기존 품목 UUID·단위·가격 adapter·mutation/recover를 재사용한다. 0/없음·정확 분자/분모·참조/순환·복제 독립성·과거 원문·공개 경계와 후속 판매를 구분했다.
- [Step 3] 검토 재검토 - PASS: 설계를 바꿀 가정은 세 가지다. 기존 품목이 owner라는 계약, 원가 조회가 보유를 바꾸지 않는 계약, 선행 writer가 schema6이라는 실제 상태다. 두 계약은 DOMAIN에서 확인했고 schema6은 T-007 커밋/실행 파일과 fixture로 재확인한다. 미결정 운영 정책을 확정하지 않는다.
- [Step 4] 계획 단순화 - PASS: recipe 전용 세 관계와 작은 편집기를 추가한다. 새 business kind·확정 단계·메뉴 마스터·범용 그래프/폼 프레임워크를 만들지 않고 공정/참고 시간은 텍스트로 보관한다.

독립 검토에서 아래 기술 계약과 기존 코드가 맞는지 확인한 후 미지원 행동 RED→제품 구현→관련 검증→After13 순으로 진행한다. 명시적인 사용자 위임 범위의 구현 선택이므로 별도 착수 승인을 요구하지 않는다.

## 2. identity·기준량·입력 계약

레시피 identity는 기존 `material_id`, 버전과 구성 행은 새 UUID다. 별도 recipe kind를 만들지 않는다. 소유/구성 품목에 새 종류 제한을 추가하지 않으며 기존 unclassified도 참조할 수 있다. 신규 품목 등록의 허용 kind는 기존 catalog와 같다. 이름/종류/단위/아카이브는 catalog, 조리 내용/판매가/구성은 recipe version이 정본이다.

`basis_count`는1~1,000,000,000 정수 문자열이며 기본1이다. 3개분100g은 분자100/분모3으로 보존한다. 선택적 `reference_output`은 양수 수량/소유 품목과 호환 단위이며 `basis_count=1`일 때만 사용한다. 있으면 기준 배치 원가와 참고 생산량을 표시한다. 실제 생산량·수율·보유 단가·재고를 생성하지 않고 품목 종류만으로 기준을 바꾸지 않는다.

입력과 중첩 객체는 unknown field를 거절한다. UUID·수량·금액·순번은 문자열이다. 수량은 명시적인0을 허용하며 빈칸을 서버가0으로 바꾸지 않는다. `selling_price=null`과 `"0"`을 구별한다. 빈 행 배열·빈 조리 설명도 저장 가능하다. 같은 품목 여러 행과0수량 행/메모를 순서대로 보존한다.

```text
RecipeReferenceInput { quantity: String, unit: String }
RecipeLineInput { material_id, material_version_id, quantity, unit, note: String }
RecipeDataInput {
  basis_count: String, reference_output: RecipeReferenceInput|null,
  selling_price: String|null, cooking_description, process, reference_time: String,
  lines: RecipeLineInput[]
}
RecipeOwnerInput (tag="mode") =
  Existing { material_id, expected_material_version_id: String }
  | New { name, unit, kind, description: String }
CreateRecipe { meta: BusinessMeta, owner: RecipeOwnerInput, data: RecipeDataInput }
ReviseRecipe {
  meta: BusinessMeta, expected_material_version_id, expected_recipe_version_id: String,
  data: RecipeDataInput
}
CopyRecipe { meta: BusinessMeta, expected_material_version_id, source_recipe_version_id: String }
```

`BusinessMeta`의 기존 request_id/epoch/expected_revision 외형만 재사용한다. 신규 owner.kind 문자열은 기존 catalog의 신규 등록 규칙으로 검사한다. 새 recipe API가 기존 catalog/business 입력 모양을 바꾸지 않는다. 원입력의 수량/단위를 보존하고 줄바꿈을 LF로 정규화한다. 제어문자/잘못된 단위·숫자·길이는 저장 전에 거절한다.

기술 한도는 구성100행, 행 메모200자, 조리 설명2000자, 공정4000자, 참고 시간200자다. 이름은 기존100 Unicode scalar·단위20자·품목 설명2000자를 유지한다. 레시피 POST에만256KiB body 한도를 적용해 여러 구성 행을 수용하고 나머지 API의16KiB는 유지한다. 초과413에서도 입력/pending을 보존해 편집 후 새 요청할 수 있게 한다. 한도는 UI/서버/검증에서 일치시킨다.

## 3. 저장·조회 DTO와 API

```text
RecipeReferenceOutput { quantity, unit, input_quantity, input_unit: String }
RecipeLine {
  id, position, material_id, material_version_id, material_name: String,
  quantity, unit, input_quantity, input_unit, note: String
}
RecipeVersion {
  id, material_id, material_version_id, material_name, material_unit: String,
  previous_version_id, copied_from_version_id: String|null,
  basis_count: String, reference_output: RecipeReferenceOutput|null,
  selling_price: String|null, cooking_description, process, reference_time: String,
  lines: RecipeLine[], recorded_at, seq: String
}
RecipeVersionSummary { id, material_version_id, previous_version_id, copied_from_version_id, basis_count, recorded_at, seq }
RecipeSummary { item: MaterialSummary, recipe_version_id, updated_at: String }
RecipeList / RecipeUses { items: RecipeSummary[], state: CatalogState }
RecipeDetail { item: MaterialSummary, head: RecipeVersion, versions: RecipeVersionSummary[], state: CatalogState }
RecipeMutation { item: MaterialSummary, version: RecipeVersion, state: CatalogState }
```

version의 material_name/unit과 구성 이름은 해당 불변 품목 버전에서 읽는다. 상세의 item은 현재 catalog 정보다. 두 의미를 UI에서 구별한다. 현재 가격 계산은 별도 GET으로 분리하여 가격 오류/상한 초과가 배합 조회·편집까지 막지 않게 한다. 원가 응답이 실패하면 부분 값을 완전한0원으로 표시하지 않는다.

| Store 메서드 | HTTP | 결과 |
| --- | --- | --- |
| recipe_list(&self, RecipeFilter) | GET /api/recipes | RecipeList |
| recipe_detail(&self, material_id) | GET /api/recipes/{id} | Option<RecipeDetail> |
| recipe_version(&self, material_id, version_id) | GET /api/recipes/{id}/versions/{version} | Option<RecipeVersion> |
| recipe_cost(&self, material_id, version_id:Option<&str>) | GET /api/recipes/{id}/cost, GET /api/recipes/{id}/versions/{version}/cost | Option<RecipeCost> |
| recipe_uses(&self, material_id) | GET /api/materials/{id}/recipe-uses | Option<RecipeUses> |
| recipe_create(&mut self, CreateRecipe) | POST /api/recipes | RecipeMutation,201 |
| recipe_revise(&mut self, material_id, ReviseRecipe) | POST /api/recipes/{id}/versions | RecipeMutation,201 |
| recipe_copy(&mut self, material_id, CopyRecipe) | POST /api/recipes/{id}/copies | RecipeMutation,201 |

각 메서드는 기존 `Result<...,StoreError>`다. 없는 품목/레시피/버전 Option은404, 존재하지만 사용처 없는 품목은 빈 목록이다. RecipeFilter는 query/archive의 기존 의미를 사용한다. API/SPA guard·세션·Host/Origin·오류code는 유지한다. SPA는 `/recipes`, `/recipes/new`, UUID 상세와 두 UUID가 있는 과거 버전 경로만 명시적으로 허용한다.

### 원자 쓰기와 참조

공통 mutate의 정규화·동일 request 재생·recover·epoch/revision CAS·단일 transaction·원응답/outbox·commit/recover를 사용한다. operation과 URL도 canonical 입력에 포함한다. 같은 요청의 재생은 현재 가격·이름·head를 다시 평가하지 않는다.

신규 품목+최초 레시피는 catalog 내부 생성 helper와 한 transaction이다. public catalog_create를 중첩 호출하지 않는다. Existing은 현재 owner 버전과 recipe 부재를 확인한다. 수정은 expected owner와 recipe head를 확인한 뒤 새 버전/행을 추가한다. 구성 참조는 실제 품목/버전 소속·단위 차원을 확인하되, 이미 선택해 보존한 과거 품목 버전을 일반 편집에서 강제로 최신으로 바꾸지 않는다. UI는 현재 정보 적용을 명시적으로 제공한다. 새로운 선택 목록은 활성 품목을 기본으로 하고 기존 보관 참조는 유지한다.

복제는 현재 source recipe head와 expected 현재 owner 버전을 검사한다. UI는 현재 저장본을 대상으로 하며 미저장 편집을 복제한 것처럼 표시하지 않는다. 새 owner UUID·recipe version·모든 line UUID를 생성하고 구성/메모/기준량/판매가/조리 내용을 깊게 복사한다. 원본 재고/가격/입고와 자식 품목은 복제하지 않는다. copied_from_version_id로 정확한 원본을 남기며 previous_version_id는 같은 owner의 수정 연결이다.

복제 이름은 원이름 뒤` (2)`부터 아카이브를 포함한 현재 이름 전체와 exact 비교하여 빈 번호를 고른다. 기존100자 한도를 넘으면 접미 문자 공간만큼 원이름 앞부분을 Unicode scalar 경계에서 줄인다. 원본 이름은 바꾸지 않고 최종 복제 이름을 응답/화면에 표시한다. 일반 중복 이름을 금지하는 UNIQUE 규칙은 추가하지 않는다.

순환은 현재 head의 품목→직접 구성 그래프에 후보를 대입해 검사한다. 자기/다단계·0수량·나중에 붙인 recipe의 순환도 거절한다. 모든 과거 버전의 합집합으로 정상 현재 구성을 거절하지 않는다. 재귀 깊이에 의존하지 않는 순회로 검사하고 사용처는 현재 직접 부모를 중복 없이 반환한다.

## 4. 정확한 원가와 가격 경계

순수 `cost_recipe(&RecipeCostInput)->Result<RecipeCostValue,DomainError>`는 DB/시각/UUID 생성 없이 계산한다. 입력은 basis_count:u32, 각 행의 Quantity 분자와 Option<UnitPrice>, Option<Money> 판매가다. 출력은 같은 순서의 Option<Money> 행 원가, Money 알려진 소계, Option<Money> 전체 원가, 선택 정확 비율과 불가 사유다.

`row = round_ratio(price_amount_micros × numerator_quantity_micros, price_quantity_micros × basis_count)`를 checked i128로 한 번 계산하고 행별 Money 후 합한다. 중간 표시 반올림·분모 연쇄 곱을 쓰지 않는다. 상한을 넘는 계산 요청은 명시 오류이며 recipe 원문은 계속 조회/편집 가능하다.

| 직접 행/판매가 | 결과 |
| --- | --- |
| 수량0·가격 없음 | 행0원, price missing 표시는 유지 |
| 양수·유효0원 | 행0원, 유효 출처 보존 |
| 양수·가격 없음/미확인 가치 | 행cost=null, 알려진 소계와 미계산 행·전체null |
| 판매가null/0 | 각각 구별한 계산 불가 사유, 나눗셈 없음 |
| 양수 판매가·완전0원 | 유효0% |

Store는 기존 holding/last-valid/preregistered resolver로 직접 행의 가격만 얻는다. 비축품의 하위 recipe 원가를 더하거나 가격 없음의 대체 단가로 쓰지 않는다. 실제0재고 전이는 T-008 이후 통합이며 last-valid/ValueUnknown은 순수 합성 입력과 구분한다.

`RecipeCost`는 recipe_version_id/basis_count, `price_mode:"current"`, rows(line_id/material_id/material_version_id/numerator_quantity/basis_count/unit/price:ResolvedPrice/cost), missing_price_line_ids, known_subtotal/total_cost/selling_price, cost_ratio{numerator,denominator} 또는 null, ratio_unavailable_reason, state를 담는다. 숫자/비율 항은 문자열이며 cost_ratio의 항은 각각 micro원 정수의 십진 문자열이다. 사유는 missing_selling_price/zero_selling_price/missing_cost로 구분하고 이 순서로 판정한다. UI 원가율은 exact 비율에100을 곱해 소수2자리까지만 표시하고 계산에 재사용하지 않는다.

원가 조회는 같은 Store 읽기 경계의 가격과 state를 반환한다. 과거 배합도 현재 가격 평가임을 명시하며 당시 판매 원가로 표시하지 않는다. 가격 조회 때문에 recipe version을 만들지 않는다. T-009는 당시 레시피/판매가·각 적용 가격/행·총원가를 판매 확정에 고정한다.

독립 검토 보완: UI는 상세에 표시한 version ID를 고정한 `/versions/{version}/cost`를 호출하고 응답 recipe_version_id·활성 화면/generation을 비교한다. R1 배합 옆에 다른 탭의 R2 원가가 표시되지 않아야 한다. 가격 평가 state가 더 최신이어도 편집 CAS basis나 pending을 자동 덮어쓰지 않는다. 별도 현재 `/cost`는 현재 head 조회가 필요한 호출자의 API이며 이 화면 경합을 우회하는 수단이 아니다.

## 5. schema7·이전 원문 호환

새 관계는 recipe_versions, recipe_lines, recipe_heads 세 개다. version에 owner/당시 owner version·previous/copy source·basis/reference/판매가/조리/seq/time, line에 version/position·구성 품목/version·canonical 수량/단위와 원입력/메모를 둔다. FK·owner/version 복합 FK·head/previous 소속·행 position 유일성과 수량/금액 상한을 제약한다. reference_output은 수량/단위/원입력이 모두 null 또는 모두 존재이며 basis_count1을 검사한다.

schema6→7은 기존 테이블을 다시 쓰지 않고 새 관계/인덱스를 같은 migration transaction에 추가한다. 이전 outbox를 source_schema6 기준으로 먼저 검증한다. DB와 schema7 JSON 모두 UUID·수치·참조 소속·이력/head·행 순서·환산/기준량·copy source·현재 그래프를 검증한다. T-007 receipt 외부 참조 검사를 schema7에서도 유지한다.

writer는 format2/schema7이며 새 세 컬렉션을 필수로 포함한다. schema6는 가격/주문/입고의 종류·입력/결과·주문 action·receipt source가 고정된 reader로 보존한다. 공통 관계 검사 재사용을 위해 작은 borrowed context를 분리할 수 있으나 원문을 재직렬화하지 않는다. 이전 UUID·canonical request·원응답·outbox/JSON bytes는 그대로다.

T-007 최종 실행 파일의 정상 API로 가상 가격 정정·주문/수정/실입고·직접 입고/누적 holding·미완성/포함확인 초안을 만든 schema6 fixture를 캡처한다. 미래 형식·새 컬렉션을 과거 reader에 넣은 경우와 손상 관계는 원 DB/JSON을 바꾸지 않고 거절한다.

## 6. UI·소유 분담·검증

목록→새/기존 품목 연결→구성 편집→저장→현재 원가→독립 복제/과거 버전·양방향 조회의 작은 흐름이다. 수량 기본0과 메모를 보존하고 행 stable key·깊은 복사·동등성·immutable pending·명시적 충돌 검토·늦은 행 조회 guard를 사용한다. 원가 영역은 저장한 구성의 평가이며 미저장 편집의 결과로 표시하지 않는다. 가격 없음/0·정확100÷3g·기준 생산량과 실제 생산을 구별한다.

| 담당 | 범위 |
| --- | --- |
| 루트 | 공개 계획/기록/문서/연결 노트, src/recipe.rs DTO·순수 계산/정규화/복제 이름·tests/recipe.rs·lib, 통합 검증/Git |
| 저장 | store/recipe.rs·store/catalog/snapshot 연결·schema7·저장 tests·schema6 fixture |
| HTTP/독립 검토 | server.rs 라우트/국소 body 한도·tests/recipe_http.rs, 설계/구현 독립 검토 |
| UI | recipe types/helpers/editor/composable/pages/cost/uses·탐색·단위/E2E·화면/키보드 |

먼저 이전 T-007 자산에서 새 route의404·미지원 기능 RED를 실행한다. 없는 import/컴파일 오류를 행동 RED로 주장하지 않는다. 첫 REST RED 뒤 DTO/인터페이스를 고정해 병렬 구현하고 frontend 변경 후에는 정식 build 전 Cargo/브라우저를 실행하지 않는다.

| 검증 | 수용 기준과 기대 |
| --- | --- |
| 직접/0/기준량 | 원물·손질·비축 직접 선택,0·메모·빈 구성,3개분100g/참고 생산량·원문/재시작 보존 (01·02) |
| 원가 | 333.333333원 정확 행·서로 다른 가격 분모·합계·상한,0/없음/양수미계산·원가율 사유·직접 비축만 합산 (03~05) |
| 가격/버전 | 사전→실제입고 평균→가격 변경, 조회로recipe/재고 변경 없음, 과거 구성·판매가/합성 판매 참조 불변 (04·07) |
| 그래프/참조 | 자기/2단계/긴/0행/나중연결 순환 거절, 과거 합집합만 순환인 정상 현재graph, 중복 없는 양방향 조회 (01) |
| 복제 | (2)/(3)/아카이브 충돌·100자 한글/비BMP·접미 자리수, 새UUID·원본/복제 편집 분리·samekey 재생 (06) |
| 저장/장애 | 각쓰기 단계/outbox 실패 전체rollback·JSON 복구·stale/source conflict·원응답/UUID 재생·oldwire (공통) |
| 호환 | 실제schema6 fixture의 모든oldbytes/응답 보존, 미래/손상/새컬렉션/외부참조/line/copy/graph 변조 거절·원문 불변 (07) |
| HTTP/UI | guard·256KiB/기존16KiB·404/413/code, 정상저장/재시작·원가실패에도편집·복제/참조·pending/경합/늦은조회·6화면/키보드 (전체) |

명령은 frontend 단위/타입→`node scripts/build.mjs`→`cargo test --locked`·fmt·clippy, 신규/기존 Playwright E2E, manifest 자체/정합성·release 경로 검사다. 독립 E2E는 격리 DB와 `--fully-parallel`을 사용해 정상 종료 대기를 병렬로 검증한다. 실제 실행 결과만 기록하며 새 파일/상대 링크/UTF-8·LF/EOF/ignore/diff를 확인한다. 최종 After13과 요청된 공개 커밋 뒤 T-008을 이어간다. T-001 필수 실환경·T-021 시연/goal 완료는 별도다.

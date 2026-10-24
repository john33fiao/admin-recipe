# T-008 실제 생산과 원가 이월 구현 계획

- 작성일: 2026-10-24
- 기준: [티켓](T-008-production.md), [요구사항](../REQUIREMENTS.md), [계산 계약](../DOMAIN-CONTRACT.md), [ADR-007](../decisions/ADR-007-consumption-and-ticket-scope.md)
- 선행: T-004 `a919110`·T-007 `85ea42a`의 구현/검증과 현재 schema7. T-005 실제 판매 조건은 T-009에 남는다.
- 상태: 설계·독립 검토 중. 아래 계약을 검토한 뒤 기존 제품의 행동 RED부터 실행한다.

## 1. 범위와 RTD Before

실제 여러 재료 투입과 실제 여러 결과 수량, 사용자 배분율로 보존한 원가, 한 번의 공통 업무 확정·조회와 품목별 보유 변화가 범위다. 원물·손질분·비축품의 단계/종류를 강제하지 않으며 재손질과 같은 품목의 투입/결과를 지원한다.

레시피/참고 생산량으로 실측 수량을 채우지 않는다. 자동 수율·생산 계획·실제 작업시간·별도 생산일·사진·판매 차감·실사/정정 writer는 제외한다. 실제 실사와 기존 사건 정정은 T-010/T-012에 남기며 T-008-06은 가상 보정 결과와 실제 누락 입고 후 생산 재검증으로 확인한다.

- [Step 1] 변경 계획 - PASS: 기준01~06을 실제 투입/결과·배분·원자 확정·보유/마지막 단가·충돌 보존에 연결했다. 순수 계산·production DTO/저장·holding 확장·공통 business/HTTP/UI와 이전 schema7 fixture를 변경 대상으로 한다. 이전 release·migration 백업·원문 비교와 아래 검증으로 되돌림 경계를 확인한다.
- [Step 2] 계획 검토 - PASS: 입력 전 Q/V, 같은 품목 중복 행, 원가 합계·0원·전량 잔여, 미확인 가치/기록 충돌, ID/단위/순서·기존 receipt 원문·공개 경계를 확인했다. 현 PC의 가상 자료/격리 DB로 검증하며 실환경 완료를 주장하지 않는다.
- [Step 3] 검토 재검토 - PASS: 가정은 세 가지다. 생산은 기존 공통 DB 초안/확정 경로를 확장한다. 보유는 현재 receipt 전용이어서 생산 원천과0수량을 명시적으로 확장해야 한다. 판매는 수량 변동을 만들지 않으므로 생산 부족을 예상 판매 소모로 판정하지 않는다. 실제 코드와 DOMAIN/ADR-007에서 확인했다.
- [Step 4] 계획 단순화 - PASS: production 원행/품목 효과와 기존 holding의 닫힌 원천 확장만 추가한다. 범용 사건 엔진·새 메뉴 마스터·가짜 입고·미래 보정 선택 UI·추정 수율을 만들지 않는다. 계산 근거는 실제 채택한 before holding version과 기존 FK로 추적한다.

## 2. 입력·본문 크기·공통 업무

`BusinessKind::Production`, `BusinessPayload`의 `kind:"production",version:1` 분기를 추가한다. 과거 가격·주문·입고의 JSON 외형은 유지한다. DB 내부 input_version과 API의 version 키를 혼동하지 않는다.

```text
DraftProductionInput { material_id:String|null, material_version_id:String|null, quantity:String, unit:String }
DraftProductionOutput { material_id:String|null, material_version_id:String|null, quantity:String, unit:String, allocation_percent:String }
DraftProductionV1 { inputs:DraftProductionInput[], outputs:DraftProductionOutput[], note:String }
```

입력/중첩 행은 unknown field·숫자 JSON 값을 거절한다. 양쪽 최대100행, 수량/배분율 원문80자·단위20자·전체 메모2000자다. UUID는 선택 시 정규화하고 메모는 LF로 맞춘다. 미선택·빈칸·미완성 숫자·배분 합계 오류는 DB 초안에 보존한다. 확정 시 입력/결과가 각각 한 행 이상, 모든 행 양수 수량·유효 참조/단위, 배분율0~100/소수6자리·합100이어야 한다. 남은 빈 행은 완성/삭제하도록 안내하고 확정하지 않으며 자동0이나 유령 결과로 만들지 않는다. 품목 버전이 바뀌면 기존 업무와 같은 명시적 참조 검토를 요구한다.

생산용 POST alias 두 곳만256KiB를 허용한다. `POST /api/production-drafts`와 `POST /api/production-drafts/{business_id}/revisions`는 기존 Create/ReviseBusinessDraft envelope·Store 명령을 사용하고 payload.kind가 production인지 검사한다. 기존 공통 draft 경로의16KiB는 유지한다. 같은 논리 명령/normalized payload의 요청 ID는 공통/alias 어느 경로로 재시도해도 같은 원응답을 반환한다. 새 별도 저장 상태 기계를 만들지 않는다.

미리보기·확정·보관은 기존 `/api/business/{id}` 경로를 쓴다. 생산 정정은 이 티켓에 추가하지 않는다. 원생산 확정 뒤의 품목/레시피 수정으로 과거 생산 원가를 재계산하지 않는다.

## 3. 순수 계산·중복 행·원가 보존

DB/시각/UUID 생성 없이 각 품목의 입력 전 `HoldingInput`과 기준 단위로 환산한 실제 행을 계산한다. 수량/금액은 기존 Quantity/Money, 중간 곱/합계는 checked i128이다. 결과는 행별 투입 가치·결과 배분 가치, 품목별 before/after와 투입 원가 합계다.

1. 동일 투입 품목의 수량을 합하고 **생산 전 Q/V**와 비교한다. 기준 없음·가치 미확인·실제 투입 초과는 `needs_reconciliation`로 반환한다. 사전 가격이나 이번 생산 결과를 미리 더해 부족을 해소하지 않는다.
2. 품목별 차감 원가는 `round(V×q/Q)`를 한 번 계산한다. q==Q이면 V 전부다. 같은 품목을 여러 입력 행으로 쪼개도 총원가가 바뀌지 않는다.
3. 그 차감 가치를 해당 입력 행의 실제 수량 비율로 최대잔여 배분한다. 행별 가치 합계는 품목별 차감액과 같다. 모든 품목의 차감액을 검사해 총 투입 원가를 구한다.
4. 총원가를 결과 행의 배분율로 최대잔여 배분한다. 주 결과100/나머지0은 새 행의 기본값이며 사용자가 바꿀 수 있다. 전체0원·0% 결과도 유효하고 각 결과 수량은 양수다.
5. 결과의 같은 품목도 합쳐 품목별 `afterQ=beforeQ-inputQ+outputQ`, `afterV=beforeV-inputValue+outputValue`를 계산한다. 같은 품목이 양쪽에 있어도 투입을 먼저 입력 전 기준에서 검사한다. net0인 효과도 실제 입출력 근거를 보존한다. 품목당 보유 버전은 하나만 만든다.

재사용하는 작은 함수 `allocate_largest_remainder(total:i64,weights:&[i64])`는 비음수 총량/가중치·양수 가중치 합, checked i128, 내림값과 분수 잔여 순 배분을 보장한다. 동률은 배열의 원래 행 순서이며 같은 사건의 seq가 같고 position이 유일하므로 미래 UUID를 만들지 않는다. 금액/수량 범위와 전체0가중치의 업무별 의미는 호출자가 검사한다.

미리보기는 실제 존재하는 before.source_id·basis/observation만 참조한다. after.source_id는 null이며 확정 시 실제 holding UUID를 생성한다. 미리보기로 미래 생산/보유 ID를 발급하지 않는다.

## 4. 보유 기준·0재고·정산 경계

처음 생산한 결과는 production effect를 시작 기준으로 만든다. 기존 품목을 확인한0으로 이관하거나 관측을 생성하지 않는다. 기존 기준/관측이 있으면 보존하고 생산 투입/결과만 후속 확정 변동에 포함한다.

- 최종Q>0이면 새Q/V가 유효한 마지막 단가다. V0도 유효한0원이다.
- 전량 투입 후Q0이면 V도0이고 직전 current 양수 Q/V·source의 정확한 마지막 단가를 유지한다. 과거 last_valid 필드가 없는 유효 합성 입력에서도 current 근거를 보존하며0분모 단가를 만들지 않는다.
- 가치 미확인인 양수 보유에 결과를 더할 때도 기존 가치를0으로 가정하지 않는다.
- holding 조회는 receipt 또는 production의 닫힌 변동 종류를 사용한다. production 한 effect에 투입/결과 양쪽 값을 담고 같은 seq의 두 사건으로 쪼개지 않는다. 시작 effect는 후속 변화로 다시 더하지 않는다.
- production basis의 id는 effect UUID이며 생산 record UUID와 다르다. 상세 링크에는 실제 production_id를 쓰고, ID 연결 정보가 없는 기준은 근거 ID만 표시한다. 추정 URL을 만들지 않는다.

생산 투입은 알려진 확정 변동이다. 후속 실사에서 판매에 배분할 미기록 소모에 다시 포함하지 않는다. 실제 판매/실사 통합은 T-009/T-010에서 검증한다.

기록 충돌은 원입력·명시적 실제량을 보존한다. 누락 입고 등 근거가 바뀌면 새 state/head에서 다시 미리보기한다. effect의 before holding version, basis/observation과 해당 holding의 receipt/production FK가 채택한 근거를 연결한다. 별도 가짜 보정 ID를 받지 않는다. 합성 보정 기준 후 생산의 단 한 번 적용과 실제 누락 입고 후 새 기준 채택을 구분해 검증한다.

## 5. 조회·효과 DTO

```text
ProductionInputRow {
  position,material_id,material_version_id,material_name,quantity,unit,input_quantity,input_unit,value:String
}
ProductionOutputRow { [같은 필드], allocation_percent:String }
ProductionMaterialEffect {
  material_id,material_version_id,unit:String,
  before:HoldingAmounts|null, after:HoldingAmounts,
  basis_before:HoldingBasisRef|null, observation:HoldingObservation|null,
  input_quantity,input_value,output_quantity,output_value:String
}
ProductionPreview { inputs,outputs,materials:ProductionMaterialEffect[],total_input_value:String }
BusinessEffect::Production { [ProductionPreview 필드] }
BusinessResult::Production { production_id:String }
ProductionRecord { id,confirmation_id,business_record_id,note,recorded_at,seq,total_input_value:String,inputs,outputs }
ProductionMaterialResult { id:String, [ProductionMaterialEffect 필드] }
ProductionDetail { item:ProductionRecord,materials:ProductionMaterialResult[],state:CatalogState }
ProductionSummary {
  id,confirmation_id,business_record_id,first_output_name,total_input_value,recorded_at,seq:String,
  input_count,output_count:usize
}
ProductionList { items:ProductionSummary[],state:CatalogState }
```

원행은 parent production UUID와 canonical position으로 불변 식별한다. 별도 row UUID는 필요 없다. 확정 효과는 독립 UUID를 가지고 actual after.source_id와 before.source_id를 제공한다. 현재 명칭 대신 해당 품목 버전 이름과 원입력을 보존한다. 배분율 원입력은 확정 입력 revision에 남고 결과에는 canonical 배분율을 둔다. 목록의 행 개수만 표시용 JSON 정수이며 업무 수량/금액·순번은 문자열을 유지한다.

Store `production_list(ProductionFilter{material_id:Option<String>})`와 `production_detail(id)`를 GET `/api/productions`·`/api/productions/{id}`에 연결한다. 없는 상세는404, 품목 필터는 입력 또는 결과에 해당 품목이 있는 생산을 중복 없이 반환한다. SPA는 목록과 UUID 상세만 허용한다. 목록은 요약만 반환하고 상세에서 원행/근거를 제공한다.

## 6. schema8·원자성·과거 원문

production records/inputs/outputs/material effects의 네 관계와 holding의 닫힌 원천 확장이 필요하다. 원행은 production+position, effect는 production+material 유일성을 가진다. receipt_id 또는 production_effect_id 중 정확히 하나, baseline receipt 또는 baseline production effect 중 정확히 하나를 실제 FK로 연결한다. holding quantity는0이 가능하되0Q이면0V를 강제한다. last-valid는 유효 양수 버전에 연결한다.

효과의 before는 새 holding.previous_version_id로 실제 근거를 조회해 도출하고 불필요한 순환 FK를 만들지 않는다. 기존 heads와 버전 관계를 함께 보존·교체하며 FK를 끄지 않는다. 이전 snapshot을 source_schema7로 먼저 검증한 뒤 migration transaction을 적용한다.

새 writer는 format2/schema8이다. schema7은 가격/주문/입고·receipt-only holding·레시피만 허용하는 닫힌 reader로 고정한다. 이전 UUID·request canonical 문자열·원응답·JSON 바이트를 재작성하지 않는다. T-004 release의 정상 API로 가상 가격/주문/입고·레시피/수정/복제와 미완성 입력을 만들고 실제 schema7 fixture를 캡처한다.

확정은 공통 CAS·요청 재생·recover를 사용해 생산·원행·효과·각 품목 holding·business 결과·원응답/outbox를 한 transaction에 저장한다. DB와 JSON 모두 품목/버전 소속·시점, confirmation/record/revision 종류·seq, 원행/효과 합계·배분 합계, before→after, source/baseline/last-valid, 기존 receipt/recipe 외부 관계를 검사한다. 범위 초과·잘못된 참조/배분·중복은 전체를 거절한다.

## 7. UI·검증·분담

생산 목록→업무 초안의 실제 투입/결과→DB 임시 저장→미리보기→한 번 확정→생산/품목 상세 흐름이다. 수량은 빈칸에서 시작하고 첫 결과 배분율100/추가 결과0만 초기값으로 둔다. 삭제/재진입/재시도에서 사용자의 배분율을 자동 재작성하지 않는다. 배열과 각 행을 깊게 복사·구조 비교하고 pending을 고정한다. 늦은 행 조회·다른 탭·이동 중 저장·JSON 실패·413 재검토를 보존한다.

| 담당 | 범위 |
| --- | --- |
| 루트 | 공개 계획/기록·요구/연결 노트, 배분 helper·business/lib·순수 holding 조정, 통합/Git |
| 저장 | 생산/holding 저장·schema8/frozen7·business 저장 연결·fixture·저장/변조/원자성 |
| 순수/HTTP | production DTO·정규화/계산·순수 검사, server alias/조회·HTTP·독립 검토 |
| UI | 생산 editor/조회·공통 draft 배열/preview·holding 분기·단위/E2E·화면/키보드 |

독립 검토 후 기존 T-004 제품에서 production 미지원/새 GET404·alias405의 행동 RED를 실행한다. 순수 합계/전량·최대잔여·중복/동일 품목·원가0·상한·미확인/기록초과, 실제3kg/30000→2kg/30000와100:0·70:30·최소 금액 잔여, 가상 보정 기준 후1회 적용을 검사한다. 실제 DB는 receipt→production→receipt→recipe cost와 last-valid를 연결하고 조회 불변·재시도/각 단계 rollback·JSON 복구·schema7 원문·미래/변조 거절을 확인한다.

관련 명령은 UI 단위/타입→`node scripts/build.mjs`→Cargo test/fmt/clippy, manifest/실행파일 검사, 신규와 기존 Playwright(`--fully-parallel`), 3너비×2테마·키보드다. frontend 소스/설정/테스트 변경 뒤 반드시 정식 빌드를 먼저 갱신한다. 실제 결과와 수용 기준·변경 파일·After13을 기록하고 검증한 공개 변경을 커밋한 뒤 T-009를 이어간다. T-001 필수 실환경·T-021 실제 시연·goal 완료는 별도다.

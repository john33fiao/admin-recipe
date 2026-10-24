# T-010 구현 계획과 RTD Before

- 작성일: 2026-10-25
- 상태: 독립 설계 검토·변경 전 행동 검증 준비
- 기준: [티켓](T-010-stocktake-and-waste.md), [계산 계약](../DOMAIN-CONTRACT.md), [요구사항](../REQUIREMENTS.md), [ADR-007](../decisions/ADR-007-consumption-and-ticket-scope.md)
- 선행: T-008 생산과 T-009 판매 구현·검증 완료. T-009-05의 실제 실사 후 배분 표시를 이번 티켓에서 연결한다.

## 1. 범위와 흐름

지정 품목의 명시 폐기/직원식과 선택·일괄 실사 입력 → DB 초안 → 미리보기 → 확정 → 관측/소모/배분·보유 이력을 구현한다. 현재 보유 원장은 이미 확정 입고·생산·폐기 효과를 한 번 반영한 정산 기준이다. 판매의 예상량을 차감하거나 실사 때 과거 변동을 다시 더하지 않는다.

T-011의 이미 실사에 포함된 입고 보정, T-012의 관측/폐기/판매 배분 정정, T-013 반품 writer는 앞당기지 않는다. 기존 지연 입고 초안은 계속 입력을 보존하고 명시 확인 없이 일반 입고로 확정하지 않는다. 예상 현재고·자동 폐기·영업일 마감·판매 직접 선택·로트·전체 자료 이관은 추가하지 않는다.

## 2. 입력과 확인 계약

공통 업무에 `stocktake`와 `waste`, payload version1을 추가한다. 두 업무 모두 기존 UUID 요청 재생·DB 초안·preview/CAS·confirm/archive를 사용한다. 실사 최대100행, 폐기 한 품목을 지원한다. 실제 판매 후보는100건으로 자르지 않는다. 수량/금액/seq는 정확한 문자열이다.

```text
DraftStocktakeV1 { rows, note }
DraftStocktakeRowV1 {
  material_id?, material_version_id?, quantity, unit,
  initial_value?: StocktakeValueDraft,
  excess?: { reviewed, reason, value?: StocktakeValueDraft }
}
StocktakeValueDraft = { kind: "existing_price" }
                   | { kind: "manual_amount", amount }
DraftWasteV1 {
  material_id?, material_version_id?, quantity, unit,
  reason: "discard" | "staff_meal", note
}
```

닫힌 중첩 타입·UUID·제어문자/줄바꿈 정규화와 원문 길이 한도는 기존 규칙을 따른다. 수량/금액80자, 단위20자, 전체 메모/초과 확인 이유2000자다. 빈 배열과 미완성 문자열은 초안으로 저장한다. 일반 HTTP 본문16KiB는 유지하고 실사 초안 생성/수정 전용 alias만256KiB를 허용한다.

- 수량이 공백뿐인 실사 행은 미확인이다. 초안은 보존하고 관측/holding/시각/배분 효과는 만들지 않는다. `"0"`은 확인한 실제 수량0이다. `abc`·음수·상한/정밀도 초과를 빈칸처럼 생략하지 않는다.
- 값이 있는 행은 현재 품목 버전·단위·수치가 모두 유효해야 한다. 확인한 동일 품목이 batch에 중복되거나 유효 확인 행이0개이면 확정하지 않는다. 한 행 오류면 전체 효과가 없다.
- 초기 기준이 없으면 실제 M과 가치를 확인한다. 사용자가 기존 단가 적용 또는 총가치 직접 입력을 선택하며 사전 단가만으로 초기 재고0을 만들지 않는다. 실제 M=0은 V=0만 허용하고 UI가0원 근거를 표시해 함께 확인받는다. 별도의0 타이핑은 강제하지 않는다.
- 기준이 있는 정상 실사는 초기/초과 확인이 필요 없다. 상태 변경으로 기존 확인 값의 분기가 달라지면 원입력을 보존하고 다시 검토하도록 한다.
- 초과는 누락 입고/생산/오입력 확인 `reviewed=true`와 이유가 필요하다. 적용 단가가 있으면 기존 단가를 사용하고, 없을 때만 초과분 금액을 확인한다. 정상0원과 금액 없음은 구별한다.
- 폐기/직원식은 실제 양수 수량을 입력한다. Q/V 미확인 또는 q>Q는 `needs_reconciliation`으로 입력을 보존한다. 일반 판매에 이 수량 제한을 추가하지 않는다.

## 3. 순수 계산과 배분

`src/stocktake.rs`·`src/waste.rs`는 DTO·정규화·계산을 맡고 SQL·파일·시계·ID 발급을 포함하지 않는다. 기존 Quantity/Money/UnitPrice·checked 정수 비율·최대잔여 배분을 재사용한다. 계산 오류는 draft issue로 표시하고 실제 IO/무결성 오류를 입력 오류로 숨기지 않는다.

| 상태 | 계산과 보존 |
| --- | --- |
| 기준 없음 | `Initial` 결과. 실제 M·확인 V로 시작 관측을 만들며 이전 소모 C/K는 null, 이전 판매에0 배분을 만들지 않음 |
| 기준 있음, M≤Q | C=Q-M, K=round(V×C/Q). 전량이면 K=V, Q=M=0이면 C/K0. 새 Q=M, V=V-K |
| 기준 있음, M>Q | C/K0, E=M-Q. 기존 단가 또는 확인 금액으로 초과 가치를 더하고 출처/이유를 보존. 음수 소모·가짜 입고 없음 |
| 부분 명시 폐기 0<q<Q | 새 Q=Q-q, V 유지, 별도 전량손실0 |
| 전량 명시 폐기 q=Q | 새 Q/V0, 이전 V를 별도 손실로 종료. 다음 입고에 이월하지 않음 |

관측이 없어도 실제 입고/생산의 시작 기준이 있으면 첫 실사에서 정상 정산한다. 순수 HoldingInput의 V=None은 재확인 필요이며 사전 가격으로 과거 보유 가치를 추정하지 않는다. 현재 영속 writer가 만들지 않는 가치 미확인 원장을 이번 티켓에서 새로 도입하지 않는다.

정산 품목별 후보는 `(직전 유효 관측 또는 실제 시작 기준 seq, 이번 확정 seq)`의 열린 구간이다. 최초 판매 seq와 as-of revision 이하의 유효 개수 개정을 별도로 선택한다. 당시 레시피가 해당 품목을 직접 참조하면 수량0도 포함하고 중복 품목은 한 번만, 하위 레시피는 재귀 전개하지 않는다.

개수를 가중치로 C/K를 각각 최대잔여 배분한다. 동률은 원본 seq→stable sale UUID다. `Σ배분+미배분=총 C/K`를 각각 검증한다. 대상 없음 또는 총개수0이면 C/K 전부 미배분으로 남긴다. 총개수0의 후보 관계는 보존하되 배분 수량/가치는 null로 표시한다. 양수 가중치 합 안의0개 후보는 유효한 배분0이다. 레시피 사용량·판매가·당시 원가를 가중치로 사용하지 않는다.

## 4. 저장·관측 기준과 schema10

format2/schema10을 작성하며 기존 schema9와 원요청·원응답·JSON 바이트를 보존한다. 새 관계는 다음과 같다.

| 관계 | 내용 |
| --- | --- |
| stocktake_batches | 한 업무 confirmation·전체 메모와 batch ID |
| stocktake_observations | stable 관측 ID·최초 batch/위치·품목 |
| stocktake_versions / stocktake_heads | 불변 관측 version·현재 유효 version, 이전 관측/시작 기준·before holding·실측 M·전후 Q/V·초기/정산/초과 근거 |
| consumption_versions / consumption_heads | 관측별 확정 C/K·조회 시점·미배분·현재 배분 version |
| consumption_rows | 당시 후보 sale/version/recipe·원본seq·가중치·순서·배분 수량/가치 |
| wastes | 명시 폐기/직원식 원본·confirmation·품목 버전·실제량·before holding·전량손실 |

T-010은 관측/배분의 최초 version만 작성한다. 이력/유효 참조를 구분할 최소 관계를 두고 일반 정정 writer는 T-012에서 연결한다. 관측 상세에는 stable ID와 immutable version ID를 함께 제공한다. HoldingObservation.id 및 Observation basis.id는 불변 관측 version을 가리킨다.

holding version의 원천은 receipt/production effect/observation version/waste 중 정확히 하나다. 관측은 그 시점 M/afterV로 새 기준을 만들며 이후 입고/생산/폐기는 같은 기준을 유지한다. 최신 관측 자체를 이후 변동에 다시 더하지 않는다. 첫 확인0을 위해 holding head의 last_valid를 nullable로 바꾸고 가짜 양수 근거나0분모 단가를 만들지 않는다. 양수 수량의0원은 유효한 마지막 단가다.

current_input·holding_detail·holding snapshot validator를 함께 바꾼다. 생산 snapshot의 observation=None 가정도 실제 관측을 읽는 adapter로 연결한다. 판매 당시 가격은 원본 판매보다 앞선 해당 holding을 사용하며 이전 판매 원가를 바꾸지 않는다. 실사 confirm은 같은 transaction의 내부 후보 helper를 쓰고 public Store를 다시 잠그지 않는다. batch마다 전체 판매를 반복100번 읽는 구조는 피한다.

schema9의 sale_cost_rows가 holding_versions를 FK로 참조하므로 holding_versions/heads 재구성에는 sale_cost_rows도 포함한다. 이전 행을 그대로 복사하고 child→parent 순서로 교체하며 transaction/foreign_key_check로 검증한다. foreign_keys 비활성화로 우회하지 않는다. source9 outbox는 frozen9의 기존 다섯 종류·입고/생산 holding·판매 형식으로 선검증한다. 신규 필드/종류가 옛 형식에 섞이면 거절하고 이전 reader 및 원문은 유지한다.

한 batch의 모든 관측·holding·배분/미배분·head·confirmation·요청·outbox가 한 transaction이다. 실제 seq는 preview revision+1이고 preview가 미래 UUID/시각을 생성하지 않는다. 다른 저장이 끼면 입력을 유지하고409로 새 검토를 요구한다. 같은 요청 성공의 원응답 재생은 현재 상태/정정 차단 검사보다 앞선 기존 순서를 유지한다.

## 5. 조회·화면과 정정 경계

실사/폐기 입력은 `/business?kind=stocktake|waste`를 사용한다. GET `/api/stocktakes[/{id}]`, `/api/wastes[/{id}]`와 대응 SPA 목록/상세를 추가하며 material_id 필터를 지원한다. 실사 전용 POST `/api/stocktake-drafts`와 `/{id}/revisions`만256KiB이고 preview/confirm/archive와 폐기는 기존 공통 경로다.

실사 편집은 확인 행의 현재 기준·마지막 관측·이후 변동, 초기 가치/초과 확인과 행별 오류를 보여준다. 미확인 빈칸과 확인0, 현재 정산 기준과 실제 관측 수량/시각을 구분한다. 미리보기는 수량/가치 전후·소모/초과·판매 후보와 미배분을 보여주며 입력을 수정하면 이전 미리보기를 무효화한다. nested 초안·pending 요청은 깊은 복사와 기존 UUID 원문 복원을 사용한다.

판매 상세에는 직접 품목별 `unconfirmed/allocated/unallocated`와 관측/정산 근거·배분 수량/가치를 별도로 제공한다. 전체 상태는 `unconfirmed/partially_settled/settled/not_applicable`로 확인 범위를 표현한다. 빈 레시피나 시작 이전 판매를 실제0 소모로 만들지 않는다. 당시 레시피 원가와 실사 배분 원가를 합치지 않는다. 기존 holdings API와 재료 화면에는 최신 관측 및 그 뒤 확정 변동을 연결한다.

이미 현재 정산의 후보로 연결된 판매 개수 정정은 T-012의 원자 재배분 연결 전 `requires_allocation_correction` issue로 확정을 막는다. C/K0·개수0·총가중치0·일부 품목만 정산한 경우도 포함한다. 정정 초안/원문/이력 조회는 보존하고 아무 품목도 정산하지 않은 판매 정정은 기존대로 허용한다. 이는 후속 영향 계산을 구현하기 전의 기술 경계이며 확정한 정정 정책을 철회하지 않는다.

기존 테마/탐색/연결 상태를 재사용하고1440/768/390의 두 테마·키보드·오류 포커스·재시도/재시작을 검증한다. T-014의 통합 작업 화면을 이번 티켓에서 미리 만들지는 않는다.

## 6. 검증·변경 범위와 복구

| 수용 기준 | 실제로 확인할 가상 사례 |
| --- | --- |
| 01 선택 실사 | 다른 기준의 두 품목·빈행·명시0, 중복/불법 한 행의 전체 차단, 미확인 head/시각 불변 |
| 02 확정 변동 | 입고→생산/폐기→실사→입고→재실사, 이전 소모·생산 투입 중복 없음. 반품 writer는 T-013에서 같은 계약 연결 |
| 03 개수 배분 | A3/B1·총400g→300/100, 직접0행/중복/비재귀, 원본 구간/as-of, 수량/가치 잔여 합 보존 |
| 04 폐기/미배분 | 10kg→폐기1kg→실사6kg에서 소모3kg, 판매 없음/전부0은 미배분 |
| 05 폐기 가치 | 부분V유지·전량손실V·0원·다음 입고 이월 없음, q>Q/미확인은 입력 보존 |
| 06 소모 가치 | 평균 K차감·전량 잔여 전부·최소단위 반올림·Q=M=0·C0의 실제 새 관측 |
| 07 초과/미확인 | 확인/이유·현재/마지막/사전 단가 또는 가격 없음 수동0/양수, 빈행 관측 불변 |
| 08 첫 실사/통합 | receipt/production 기준 첫 정산, 기준 없는 M/V확인·첫0·이전 판매 미확인, 생산 부족 초안→실제 관측→old preview409→새 preview/생산 한 번 |

변경 전 정상 컴파일 상태에서 미지원 조회/업무 종류와 schema9→10의 행동 RED를 기록한다. 현재 release의 정상 API로 가상 schema9 fixture를 캡처하고 판매/holding FK·개수 정정·원요청/응답/JSON 바이트를 보존한다. import/컴파일 오류를 기능 RED로 계산하지 않는다.

쓰기 단계별 rollback·발행 실패 gate/복구·동일 요청 재생·다른 본문 충돌·batch 마지막 행 실패·동시 탭 CAS, 관측/holding/판매 version/배분 합·시각/요청 연결 변조, frozen9의 신규 종류/필드 거절과 migration 실패를 검증한다. API는 세션·Host·Origin·종료·본문 한도를 유지한다. 테스트와 공개 문서는 가상 자료만 사용한다.

예상 변경은 stocktake/waste 순수·저장·테스트, holding/production/sale adapter, business/server/lib/schema10/snapshot/frozen9, Nuxt 편집/조회/판매 배분/보유 표시와 검증이다. README·ARCHITECTURE·TODO·관련 티켓·연결 노트를 함께 갱신한다. 정상 명령은 RTD의 frontend unit/typecheck→`node scripts/build.mjs`→Cargo 전체/fmt/clippy·브라우저·자산 검사다. frontend 파일 변경 뒤 반드시 표준 빌드를 갱신한다. 문서 재읽기·상대 링크·UTF-8/LF·diff·ignore도 확인한다.

변경 전 DB 백업과 원래 JSON을 보존한다. schema10을 이전 앱으로 억지 다운그레이드하지 않고 필요 시 변경 전 백업과 호환 앱으로 복구한다. 누적 기록 비용·실제 격리 시연·별도 OS/실제 절전은 T-021 및 T-001에 남는다.

## RTD Before

[Step 1] 변경 계획 - PASS: 여덟 수용 기준을 명시 폐기·품목별 실사·소모/배분·실제 생산 연결과 실행 가능한 검사로 대응했다. 일반 정정/지연 입고/반품 후속 범위를 구분했다.

[Step 2] 계획 검토 - PASS: 기존 공개 변경은 판매까지 검증·커밋되어 분리됐다. 빈칸/0·초기 미확인/관측·수량/가치·부분폐기·가중치0·FK 교체·엄격 이전 reader·공개 경계를 검토했다.

[Step 3] 검토 재검토 - PASS: 설계가 달라질 가정은 batch당 확인 품목1행, 첫 시작의 가치 확인, 정산 후 판매 정정의 T-012 연결 전 차단이다. 기존 운영 결정의 최소 기술 구현이며 새 정책 질문은 현재 없다. 담당자 독립 검토 후 행동 RED를 실행한다.

[Step 4] 계획 단순화 - PASS: 기존 공통 업무/원자 저장·정수 계산·판매 후보·UI 상태를 재사용한다. 새 원장 프레임워크·가짜 관측·예상 재고·전체 원자료 이관을 추가하지 않는다.
